//! JavaScript/TypeScript codegen: IR -> JS/TS.
//!
//! Generates JS/TS source files from Intent IR. Each component becomes
//! a class, each function becomes a method, and stores become standalone
//! classes. In JS mode (ts_mode: false), no type annotations are emitted.
//! In TS mode (ts_mode: true), type annotations are added.

use std::collections::HashMap;

use super::{try_format_stdlib_call, CodeWriter, Codegen, CompileOutput, CompiledFile, Target};
use crate::ir::*;

pub struct JavaScriptCodegen {
    pub ts_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(ts_mode: bool) -> Self {
        Self { ts_mode }
    }
}

impl Codegen for JavaScriptCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        let ext = if self.ts_mode { ".ts" } else { ".js" };

        for (module_key, module) in &ir.modules {
            let module_name = super::sanitize_module_name(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Generate type aliases
            for alias in module.type_aliases.values() {
                generate_type_alias(&mut w, alias, self.ts_mode);
            }
            if !module.structs.is_empty()
                || !module.enums.is_empty()
                || !module.functions.is_empty()
            {
                w.emit("");
            }

            // Generate structs as classes
            for struct_ir in module.structs.values() {
                generate_struct(&mut w, struct_ir, self.ts_mode);
                w.emit("");
            }

            // Generate enums as frozen objects
            for enum_ir in module.enums.values() {
                generate_enum(&mut w, enum_ir, self.ts_mode);
                w.emit("");
            }

            // Generate standalone functions
            let mut has_main = false;
            for (fname, func) in &module.functions {
                generate_standalone_function(&mut w, func, self.ts_mode);
                w.emit("");
                if fname == "main" {
                    has_main = true;
                }
            }

            // Entry point: if there's a main(), call it
            if has_main {
                w.emit("main();");
                w.emit("");
            }

            // Generate stores from this module
            for (store_name, store) in &module.stores {
                generate_store(&mut w, store_name, store, self.ts_mode);
                w.emit("");
            }

            // Generate components from this module
            for (comp_name, comp) in &module.components {
                generate_component(&mut w, comp_name, comp, self.ts_mode);
                w.emit("");
            }

            let out_path = format!("{}{}", module_name, ext);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

// ═══════════════════════════════════════════════════════════
// Component generation
// ═══════════════════════════════════════════════════════════

fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR, ts_mode: bool) {
    w.emit(&format!("class {} {{", name));
    w.indent();

    // Constructor with state initialization
    if !comp.states.is_empty() || !comp.props.is_empty() {
        // TS: emit field declarations before constructor
        if ts_mode {
            for (prop_name, prop_type) in &comp.props {
                let ann = js_type(&prop_type.kind, &prop_type.type_args, true);
                w.emit(&format!("{}: {};", prop_name, ann));
            }
            for state in &comp.states {
                let ann = js_type(&state.r#type.kind, &state.r#type.type_args, true);
                w.emit(&format!("{}: {};", state.name, ann));
            }
        }

        w.emit("constructor() {");
        w.indent();
        for prop in comp.props.keys() {
            w.emit(&format!("this.{} = null;  // prop", prop));
        }
        for state in &comp.states {
            let js_init = format_expr_ir(&state.initial_value, ts_mode);
            w.emit(&format!("this.{} = {};", state.name, js_init));
        }
        w.dedent();
        w.emit("}");
    }

    // Computed properties as getters
    for computed in &comp.computeds {
        let body = format_expr_ir(&computed.body, ts_mode);
        let ret_ann = if ts_mode {
            let ann = js_type(&computed.r#type.kind, &computed.r#type.type_args, true);
            if ann.is_empty() {
                String::new()
            } else {
                format!(": {}", ann)
            }
        } else {
            String::new()
        };
        w.emit(&format!("get {}(){}{{", computed.name, ret_ann));
        w.indent();
        w.emit(&format!("return {};", body));
        w.dedent();
        w.emit("}");
    }

    // Functions (methods)
    for func in &comp.fns {
        generate_function(w, func, ts_mode);
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR, ts_mode: bool) {
    w.emit(&format!("class {} {{", name));
    w.indent();

    // TS: field declarations
    if ts_mode {
        for (signal_name, signal_type) in &store.signals {
            let ann = js_type(&signal_type.kind, &signal_type.type_args, true);
            w.emit(&format!("{}: {};", signal_name, ann));
        }
        for getter_name in store.getters.keys() {
            // Infer type for getter — use `any` for TS mode
            w.emit(&format!("get {}(): any;", getter_name));
        }
    }

    // Constructor with signal defaults
    w.emit("constructor() {");
    w.indent();
    for signal_name in store.signals.keys() {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(format_json_value)
            .unwrap_or_else(|| "null".into());
        w.emit(&format!("this.{} = {};", signal_name, default));
    }
    w.dedent();
    w.emit("}");

    // Getters as get accessors
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        let ret_ann = if ts_mode {
            ": any".into()
        } else {
            String::new()
        };
        w.emit(&format!("get {}(){}{{", getter_name, ret_ann));
        w.indent();
        w.emit(&format!("return {};", format_expr_ir(getter_expr, ts_mode)));
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Function generation
// ═══════════════════════════════════════════════════════════

fn generate_function(w: &mut CodeWriter, func: &FunctionIR, ts_mode: bool) {
    // Build parameter list — no `self` equivalent in JS classes
    let mut param_strs = Vec::new();
    for p in &func.params {
        let safe_name = super::sanitize_js_ident(&p.name);
        let mut s = safe_name;
        if ts_mode {
            if let Some(ref ty) = p.r#type {
                let ann = js_type(&ty.kind, &ty.type_args, true);
                if !ann.is_empty() {
                    s.push_str(&format!(": {}", ann));
                }
            }
        }
        param_strs.push(s);
    }

    // Determine return type annotation (TS only)
    let ret_ann = if ts_mode {
        func.return_type
            .as_ref()
            .map(|rt| {
                let ann = js_type(&rt.kind, &rt.type_args, true);
                if ann.is_empty() {
                    String::new()
                } else {
                    format!(": {}", ann)
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let all_params = param_strs.join(", ");
    let async_prefix = if func.is_async { "async " } else { "" };
    w.emit(&format!(
        "{}{}({}){} {{",
        async_prefix, func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — JS doesn't need `pass`, body can be empty
        // but some linters prefer a comment or semicolon
        w.emit(";");
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt, ts_mode);
        }
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Statement generation
// ═══════════════════════════════════════════════════════════

fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR, ts_mode: bool) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let safe_name = super::sanitize_js_ident(name);
            let rhs = value
                .as_ref()
                .map(|v| format_expr_ir(v, ts_mode))
                .unwrap_or_else(|| "null".into());
            let kw = "let";
            let type_hint = if ts_mode {
                r#type
                    .as_ref()
                    .map(|t| {
                        let ann = js_type(&t.kind, &t.type_args, true);
                        if ann.is_empty() {
                            String::new()
                        } else {
                            format!(": {}", ann)
                        }
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            w.emit(&format!("{} {}{} = {};", kw, safe_name, type_hint, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = format_expr_ir(condition, ts_mode);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s, ts_mode);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_ir(elif_cond, ts_mode);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement(w, s, ts_mode);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement(w, s, ts_mode);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(|v| format_expr_ir(v, ts_mode))
                .unwrap_or_else(|| "null".into());
            w.emit(&format!("return {};", rhs));
        }
        StatementIR::MutationStmt { intent: None } => {}
        StatementIR::MutationStmt {
            intent: Some(mut_intent),
        } => {
            let target = &mut_intent.target.name;
            let amount = mut_intent
                .amount
                .as_ref()
                .map(|v| format_expr_ir(v, ts_mode))
                .unwrap_or_else(|| "0".into());
            match mut_intent.operation.as_str() {
                "direct_assign" => {
                    w.emit(&format!("this.{} = {};", target, amount));
                }
                "increment" => {
                    w.emit(&format!("this.{} += {};", target, amount));
                }
                "decrement" => {
                    w.emit(&format!("this.{} -= {};", target, amount));
                }
                "multiply" => {
                    w.emit(&format!("this.{} *= {};", target, amount));
                }
                "divide" => {
                    w.emit(&format!("this.{} /= {};", target, amount));
                }
                "modulo" => {
                    w.emit(&format!("this.{} %= {};", target, amount));
                }
                "push" => {
                    let item = mut_intent
                        .item
                        .as_ref()
                        .map(|v| format_expr_ir(v, ts_mode))
                        .unwrap_or_else(|| "undefined".into());
                    w.emit(&format!("this.{}.push({});", target, item));
                }
                "pop" => {
                    w.emit(&format!("this.{}.pop();", target));
                }
                other => {
                    w.emit(&format!(
                        "// TODO: mutation operation '{}' on {}",
                        other, target
                    ));
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = format_expr_ir(expr, ts_mode);
            // Skip null expressions
            if line != "null" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> =
                    args.iter().map(|a| format_expr_ir(a, ts_mode)).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir(condition, ts_mode);
            w.emit(&format!("while ({}) {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s, ts_mode);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = format_expr_ir(iterable, ts_mode);
            w.emit(&format!("for (const {} of {}) {{", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s, ts_mode);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::BreakStmtIR => {
            w.emit("break;");
        }
        StatementIR::ContinueStmtIR => {
            w.emit("continue;");
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

fn format_expr_ir(expr: &ExprIR, _ts_mode: bool) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value),
        ExprIR::IdentifierExpr { name } => super::sanitize_js_ident(name),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object, _ts_mode), field)
        }
        ExprIR::OptionalChainExpr { object, field } => {
            format!("{}?.{}", format_expr_ir(object, _ts_mode), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let js_op = binary_op_to_js(op);
            let lhs = format_expr_ir(left, _ts_mode);
            let rhs = format_expr_ir(right, _ts_mode);
            format!("{} {} {}", lhs, js_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let js_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                "await" => "await ",
                _ => op,
            };
            format!("{}{}", js_op, format_expr_ir(operand, _ts_mode))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond, _ts_mode);
            let t = format_expr_ir(then_expr, _ts_mode);
            let e = format_expr_ir(else_expr, _ts_mode);
            format!("{} ? {} : {}", c, t, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_ir(a, _ts_mode)).collect();
            // Check stdlib intrinsics
            let target = if _ts_mode {
                Target::TypeScript
            } else {
                Target::JavaScript
            };
            if let Some(native) = try_format_stdlib_call(name, &arg_strs, target) {
                return native;
            }
            match name.as_str() {
                "println" => format!("console.log({})", arg_strs.join(", ")),
                "print" => format!("process.stdout.write(String({}))", arg_strs.join(", ")),
                _ => format!("{}({})", name, arg_strs.join(", ")),
            }
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args
                .iter()
                .map(|a| format_expr_ir(a, _ts_mode))
                .collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements
                .iter()
                .map(|e| format_expr_ir(e, _ts_mode))
                .collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v, _ts_mode)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            variant, fields, ..
        } => {
            let mut entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v, _ts_mode)))
                .collect();
            entries.insert(0, format!("\"_variant\": \"{}\"", variant));
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir(object, _ts_mode);
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_ir(a, _ts_mode)).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr { statements, tail } => {
            // Emit as IIFE with preceding statements
            let stmts: Vec<String> = statements
                .iter()
                .map(|s| format!("  {};", format_stmt_ir(s, _ts_mode)))
                .collect();
            let tail_str = format_expr_ir(tail, _ts_mode);
            format!(
                "(() => {{\n{}\n  return {};\n}})()",
                stmts.join("\n"),
                tail_str
            )
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir(object, false);
            let idx = format_expr_ir(index, false);
            format!("({obj})[{idx}]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir(operand, _ts_mode))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let matched_str = format_expr_ir(matched, _ts_mode);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body_expr = format_block_as_js_expr(&arm.body, _ts_mode);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!(
                                "  if (_v._variant === \"{}\") {{ return {}; }}",
                                name, body_expr
                            )
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("  return {};", body_expr)
                        }
                        MatchPatternKindIR::Literal { value } => {
                            let v = format_json_value(value);
                            format!("  if (_v._variant === {}) {{ return {}; }}", v, body_expr)
                        }
                    }
                })
                .collect();
            format!(
                "(() => {{\n  const _v = {};\n{}\n}})()",
                matched_str,
                cases.join("\n  ")
            )
        }
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_ir(body, false);
            format!("(({p}) => {{ return {b}; }})")
        }
    }
}

/// Format a block's trailing expression as a JS value (for match arm bodies).
fn format_block_as_js_expr(block: &BlockIR, ts_mode: bool) -> String {
    if let Some(last) = block.statements.last() {
        match last {
            StatementIR::ExprStmtIR { expr } => format_expr_ir(expr, ts_mode),
            StatementIR::ReturnStmtIR { value } => value
                .as_ref()
                .map(|e| format_expr_ir(e, ts_mode))
                .unwrap_or_else(|| "undefined".into()),
            _ => "undefined".into(),
        }
    } else {
        "undefined".into()
    }
}

/// Format a single statement IR node as a JS expression string (for block exprs).
fn format_stmt_ir(stmt: &StatementIR, ts_mode: bool) -> String {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            let safe_name = super::sanitize_js_ident(name);
            let val = value
                .as_ref()
                .map(|e| format_expr_ir(e, ts_mode))
                .unwrap_or_else(|| "undefined".into());
            format!("let {} = {}", safe_name, val)
        }
        StatementIR::ExprStmtIR { expr } => format_expr_ir(expr, ts_mode),
        _ => "/* stmt */".into(),
    }
}

/// Format a JSON value as a JavaScript literal.
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                f.to_string()
            } else {
                n.to_string()
            }
        }
        serde_json::Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\"", escaped)
        }
        other => other.to_string(),
    }
}

/// Map OMNI binary operator name to JavaScript operator.
fn binary_op_to_js(op: &str) -> &str {
    match op {
        "add" => "+",
        "sub" => "-",
        "mul" => "*",
        "div" => "/",
        "mod" => "%",
        "eq" => "===",
        "neq" => "!==",
        "lt" => "<",
        "gt" => ">",
        "lteq" => "<=",
        "gteq" => ">=",
        "and" => "&&",
        "or" => "||",
        "assign" => "=",
        "addassign" => "+=",
        "subassign" => "-=",
        "mulassign" => "*=",
        "divassign" => "/=",
        "modassign" => "%=",
        _ => op,
    }
}

/// Map OMNI type to JS/TS type annotation string.
pub fn js_type(kind: &str, type_args: &[TypeIR], ts_mode: bool) -> String {
    if !ts_mode {
        return String::new();
    }
    match kind {
        "int" | "i32" | "i64" | "u32" | "u64" | "float" | "f64" | "f32" => "number".into(),
        "bool" => "boolean".into(),
        "string" | "String" | "str" => "string".into(),
        "void" | "()" => "void".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("{}[]", js_type(&inner.kind, &inner.type_args, true))
            } else {
                "any[]".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "Record<{}, {}>",
                    js_type(&type_args[0].kind, &type_args[0].type_args, true),
                    js_type(&type_args[1].kind, &type_args[1].type_args, true)
                )
            } else {
                "Record<string, any>".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("{} | null", js_type(&inner.kind, &inner.type_args, true))
            } else {
                "any | null".into()
            }
        }
        other => {
            // If it looks like a user-defined type (starts with uppercase), use as-is
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                other.into()
            } else {
                "any".into()
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Type alias, struct, enum, and standalone function generation
// ═══════════════════════════════════════════════════════════

fn format_type_ir_js(ty: &TypeIR, ts_mode: bool) -> String {
    js_type(&ty.kind, &ty.type_args, ts_mode)
}

fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR, ts_mode: bool) {
    let export_prefix = if struct_ir.visibility == "public" {
        "export "
    } else {
        ""
    };
    w.emit(&format!("{}class {} {{", export_prefix, struct_ir.name));
    w.indent();

    // Build constructor params with optional TS type annotations
    let mut param_strs = Vec::new();
    for field in &struct_ir.fields {
        let mut s = field.name.clone();
        if ts_mode {
            let ann = format_type_ir_js(&field.r#type, true);
            s.push_str(&format!(": {}", ann));
        }
        param_strs.push(s);
    }
    let all_params = param_strs.join(", ");

    if !struct_ir.fields.is_empty() {
        w.emit(&format!("constructor({}) {{", all_params));
        w.indent();
        for field in &struct_ir.fields {
            w.emit(&format!("this.{} = {};", field.name, field.name));
        }
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR, _ts_mode: bool) {
    let export_prefix = if enum_ir.visibility == "public" {
        "export "
    } else {
        ""
    };
    w.emit(&format!(
        "{}const {} = Object.freeze({{",
        export_prefix, enum_ir.name
    ));
    w.indent();
    for (i, variant) in enum_ir.variants.iter().enumerate() {
        if variant.payload.is_some() {
            w.emit(&format!("{}: \"{}\",", variant.name, variant.name));
        } else {
            w.emit(&format!("{}: {},", variant.name, i));
        }
    }
    w.dedent();
    w.emit("});");
}

fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR, ts_mode: bool) {
    if !ts_mode {
        return; // Skip in JS mode
    }
    let target = format_type_ir_js(&alias.target, true);
    let export_prefix = if alias.visibility == "public" {
        "export "
    } else {
        ""
    };
    w.emit(&format!(
        "{}type {} = {};",
        export_prefix, alias.name, target
    ));
}

fn generate_standalone_function(w: &mut CodeWriter, func: &FunctionIR, ts_mode: bool) {
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = p.name.clone();
        if ts_mode {
            if let Some(ref ty) = p.r#type {
                let ann = format_type_ir_js(ty, true);
                if !ann.is_empty() {
                    s.push_str(&format!(": {}", ann));
                }
            }
        }
        param_strs.push(s);
    }

    let ret_ann = if ts_mode {
        func.return_type
            .as_ref()
            .map(|rt| {
                let ann = format_type_ir_js(rt, true);
                if ann.is_empty() {
                    String::new()
                } else {
                    format!(": {}", ann)
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let all_params = param_strs.join(", ");

    let async_kw = if func.is_async { "async " } else { "" };
    let prefix = if func.visibility == "public" {
        format!("export {}function ", async_kw)
    } else {
        format!("{}function ", async_kw)
    };

    w.emit(&format!(
        "{}{}({}){} {{",
        prefix, func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        w.emit(";");
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt, ts_mode);
        }
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Unit tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── js_type tests ──────────────────────────────────────

    #[test]
    fn test_js_type_no_ts() {
        assert_eq!(js_type("int", &[], false), "");
        assert_eq!(js_type("string", &[], false), "");
        assert_eq!(js_type("bool", &[], false), "");
        assert_eq!(js_type("void", &[], false), "");
    }

    #[test]
    fn test_ts_type_primitives() {
        assert_eq!(js_type("int", &[], true), "number");
        assert_eq!(js_type("i32", &[], true), "number");
        assert_eq!(js_type("f64", &[], true), "number");
        assert_eq!(js_type("bool", &[], true), "boolean");
        assert_eq!(js_type("string", &[], true), "string");
        assert_eq!(js_type("void", &[], true), "void");
    }

    #[test]
    fn test_ts_type_generics() {
        let i32_arg = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        let string_arg = TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };

        assert_eq!(
            js_type("Vec", std::slice::from_ref(&i32_arg), true),
            "number[]"
        );
        assert_eq!(
            js_type("HashMap", &[string_arg.clone(), i32_arg.clone()], true),
            "Record<string, number>"
        );
    }

    #[test]
    fn test_ts_type_option() {
        let i32_arg = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(js_type("Option", &[i32_arg], true), "number | null");
    }

    #[test]
    fn test_ts_type_user_defined() {
        assert_eq!(js_type("MyType", &[], true), "MyType");
        assert_eq!(js_type("unknown", &[], true), "any");
    }

    // ── format_json_value tests ────────────────────────────

    #[test]
    fn test_format_json_value() {
        assert_eq!(format_json_value(&serde_json::Value::Null), "null");
        assert_eq!(format_json_value(&serde_json::Value::Bool(true)), "true");
        assert_eq!(format_json_value(&serde_json::Value::Bool(false)), "false");
        assert_eq!(format_json_value(&serde_json::json!(42)), "42");
        assert_eq!(format_json_value(&serde_json::json!("hello")), "\"hello\"");
        assert_eq!(
            format_json_value(&serde_json::json!("he\"llo")),
            "\"he\\\"llo\""
        );
    }

    // ── binary_op_to_js tests ──────────────────────────────

    #[test]
    fn test_binary_ops() {
        assert_eq!(binary_op_to_js("add"), "+");
        assert_eq!(binary_op_to_js("eq"), "===");
        assert_eq!(binary_op_to_js("and"), "&&");
        assert_eq!(binary_op_to_js("or"), "||");
    }

    // ── format_expr_ir tests ─────────────────────────────

    #[test]
    fn test_format_expr_literal() {
        let expr = ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        };
        assert_eq!(format_expr_ir(&expr, false), "42");
    }

    #[test]
    fn test_format_expr_identifier() {
        let expr = ExprIR::IdentifierExpr {
            name: "count".into(),
        };
        assert_eq!(format_expr_ir(&expr, false), "count");
    }

    #[test]
    fn test_format_expr_binary() {
        let expr = ExprIR::BinaryOpExpr {
            op: "add".into(),
            left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
            right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
        };
        assert_eq!(format_expr_ir(&expr, false), "a + b");
    }

    #[test]
    fn test_format_expr_ternary() {
        let expr = ExprIR::TernaryIR {
            cond: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
            then_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            else_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            }),
        };
        assert_eq!(format_expr_ir(&expr, false), "x ? 1 : 0");
    }

    #[test]
    fn test_format_expr_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
            field: "prop".into(),
        };
        assert_eq!(format_expr_ir(&expr, false), "obj.prop");
    }

    #[test]
    fn test_format_expr_unary_not() {
        let expr = ExprIR::UnaryOpExpr {
            op: "not".into(),
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "flag".into(),
            }),
        };
        assert_eq!(format_expr_ir(&expr, false), "!flag");
    }

    #[test]
    fn test_format_expr_unary_neg() {
        let expr = ExprIR::UnaryOpExpr {
            op: "neg".into(),
            operand: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
        };
        assert_eq!(format_expr_ir(&expr, false), "-x");
    }

    // ── generate_function tests ───────────────────────────

    #[test]
    fn test_generate_function_empty_js() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "doThing".into(),
            visibility: "private".to_string(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_function(&mut w, &func, false);
        let out = w.into_string();
        assert!(out.contains("doThing() {"));
    }

    #[test]
    fn test_generate_function_empty_ts() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "doThing".into(),
            visibility: "private".to_string(),
            params: vec![],
            return_type: Some(TypeIR {
                kind: "void".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_function(&mut w, &func, true);
        let out = w.into_string();
        assert!(out.contains("doThing(): void {"));
    }

    #[test]
    fn test_generate_function_with_params_ts() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "greet".into(),
            visibility: "private".to_string(),
            params: vec![ParamIR {
                name: "name".into(),
                r#type: Some(TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                }),
            }],
            return_type: Some(TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_function(&mut w, &func, true);
        let out = w.into_string();
        assert!(out.contains("greet(name: string): string {"));
    }

    #[test]
    fn test_generate_function_with_params_js() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "greet".into(),
            visibility: "private".to_string(),
            params: vec![ParamIR {
                name: "name".into(),
                r#type: Some(TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                }),
            }],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_function(&mut w, &func, false);
        let out = w.into_string();
        assert!(out.contains("greet(name) {"));
        assert!(!out.contains(": string"));
    }

    #[test]
    fn test_generate_async_function_js() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "fetchData".into(),
            visibility: "private".to_string(),
            params: vec![],
            return_type: Some(TypeIR {
                kind: "String".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: true,
        };
        generate_function(&mut w, &func, false);
        let out = w.into_string();
        assert!(
            out.contains("async fetchData()"),
            "JS async function should have 'async' prefix. Output:\n{}",
            out
        );
    }

    #[test]
    fn test_generate_async_function_ts() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "fetchData".into(),
            visibility: "private".to_string(),
            params: vec![],
            return_type: Some(TypeIR {
                kind: "String".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: true,
        };
        generate_function(&mut w, &func, true);
        let out = w.into_string();
        assert!(
            out.contains("async fetchData()"),
            "TS async function should have 'async' prefix. Output:\n{}",
            out
        );
    }

    #[test]
    fn test_format_await_expr_js() {
        let expr = ExprIR::AwaitExpr {
            operand: Box::new(ExprIR::CallExpr {
                name: "fetch".into(),
                args: vec![ExprIR::LiteralExpr {
                    value: serde_json::json!("/api/data"),
                }],
            }),
        };
        let out = format_expr_ir(&expr, false);
        assert!(
            out.starts_with("await "),
            "JS await should be prefix 'await'. Output: {}",
            out
        );
        assert!(
            out.contains("fetch"),
            "await expr should contain the awaited call. Output: {}",
            out
        );
    }

    #[test]
    fn test_js_empty_component() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            timelines: vec![],
        };
        generate_component(&mut w, "App", &comp, false);
        let out = w.into_string();
        assert!(out.contains("class App {"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_ts_empty_component() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            timelines: vec![],
        };
        generate_component(&mut w, "App", &comp, true);
        let out = w.into_string();
        assert!(out.contains("class App {"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_js_component_with_function() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![FunctionIR {
                name: "greet".into(),
                visibility: "private".to_string(),
                params: vec![],
                return_type: None,
                body: BlockIR {
                    statements: vec![],
                    execution_mode: "client-only".into(),
                },
                is_async: false,
            }],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            timelines: vec![],
        };
        generate_component(&mut w, "App", &comp, false);
        let out = w.into_string();
        assert!(out.contains("class App {"));
        assert!(out.contains("greet() {"));
    }

    #[test]
    fn test_js_component_with_state() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "Counter".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![StateIntent {
                name: "count".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!(0),
                },
                mutability: "mutable".into(),
                update_semantics: "setter-call".into(),
                accessor_style: "direct".into(),
                is_copy: false,
            }],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            timelines: vec![],
        };
        generate_component(&mut w, "Counter", &comp, false);
        let out = w.into_string();
        assert!(out.contains("class Counter {"));
        assert!(out.contains("constructor() {"));
        assert!(out.contains("this.count = 0;"));
    }

    #[test]
    fn test_ts_component_with_state() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "Counter".into(),
            generic_params: vec![],
            props: HashMap::new(),
            prop_defaults: HashMap::new(),
            states: vec![StateIntent {
                name: "count".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!(0),
                },
                mutability: "mutable".into(),
                update_semantics: "setter-call".into(),
                accessor_style: "direct".into(),
                is_copy: false,
            }],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
            motions: vec![],
            timelines: vec![],
        };
        generate_component(&mut w, "Counter", &comp, true);
        let out = w.into_string();
        assert!(out.contains("class Counter {"));
        assert!(out.contains("count: number;"));
        assert!(out.contains("constructor() {"));
        assert!(out.contains("this.count = 0;"));
    }

    // ── generate_store tests ──────────────────────────────

    #[test]
    fn test_js_generate_store() {
        let mut w = CodeWriter::new();
        let store = StoreIR {
            name: "AppStore".into(),
            signals: HashMap::from([
                (
                    "count".into(),
                    TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                ),
                (
                    "name".into(),
                    TypeIR {
                        kind: "string".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                ),
            ]),
            signal_defaults: HashMap::from([
                ("count".into(), serde_json::json!(0)),
                ("name".into(), serde_json::json!("world")),
            ]),
            getters: HashMap::new(),
            actions: vec![],
        };
        generate_store(&mut w, "AppStore", &store, false);
        let out = w.into_string();
        assert!(out.contains("class AppStore {"));
        assert!(out.contains("constructor() {"));
        assert!(out.contains("this.count = 0;"));
        assert!(out.contains("this.name = \"world\";"));
    }

    #[test]
    fn test_ts_generate_store() {
        let mut w = CodeWriter::new();
        let store = StoreIR {
            name: "AppStore".into(),
            signals: HashMap::from([(
                "count".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                },
            )]),
            signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
            getters: HashMap::new(),
            actions: vec![],
        };
        generate_store(&mut w, "AppStore", &store, true);
        let out = w.into_string();
        assert!(out.contains("class AppStore {"));
        assert!(out.contains("count: number;"));
        assert!(out.contains("constructor() {"));
        assert!(out.contains("this.count = 0;"));
    }

    // ── generate_statement tests ──────────────────────────

    #[test]
    fn test_js_generate_statement_let() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            }),
            r#type: None,
        };
        generate_statement(&mut w, &stmt, false);
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_js_generate_statement_return() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR {
            value: Some(ExprIR::IdentifierExpr {
                name: "result".into(),
            }),
        };
        generate_statement(&mut w, &stmt, false);
        assert_eq!(w.into_string(), "return result;\n");
    }

    #[test]
    fn test_js_generate_statement_mutation() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::MutationStmt {
            intent: Some(MutationIntent {
                target: SignalRefIR {
                    owner: None,
                    name: "count".into(),
                },
                operation: "increment".into(),
                amount: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                item: None,
            }),
        };
        generate_statement(&mut w, &stmt, false);
        assert_eq!(w.into_string(), "this.count += 1;\n");
    }

    #[test]
    fn test_js_generate_statement_mutation_assign() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::MutationStmt {
            intent: Some(MutationIntent {
                target: SignalRefIR {
                    owner: None,
                    name: "name".into(),
                },
                operation: "direct_assign".into(),
                amount: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!("hello"),
                }),
                item: None,
            }),
        };
        generate_statement(&mut w, &stmt, false);
        assert_eq!(w.into_string(), "this.name = \"hello\";\n");
    }

    // ── Full pipeline through trait ────────────────────────

    #[test]
    fn test_js_full_pipeline_through_trait() {
        let codegen = JavaScriptCodegen::new(false);
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::from([(
                        "App".into(),
                        ComponentIR {
                            name: "App".into(),
                            generic_params: vec![],
                            props: HashMap::new(),
                            prop_defaults: HashMap::new(),
                            states: vec![],
                            computeds: vec![],
                            effects: vec![],
                            lifecycles: vec![],
                            fns: vec![FunctionIR {
                                name: "run".into(),
                                visibility: "private".to_string(),
                                params: vec![],
                                return_type: None,
                                body: BlockIR {
                                    statements: vec![],
                                    execution_mode: "client-only".into(),
                                },
                                is_async: false,
                            }],
                            store_refs: vec![],
                            refs: vec![],
                            event_names: vec![],
                            view: None,
                            styles: vec![],
                            motions: vec![],
                            timelines: vec![],
                        },
                    )]),
                    stores: HashMap::new(),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].path, "main.js");
        let content = &output.files[0].content;
        assert!(content.contains("class App {"));
        assert!(content.contains("run() {"));
    }

    #[test]
    fn test_ts_full_pipeline_through_trait() {
        let codegen = JavaScriptCodegen::new(true);
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::from([(
                        "App".into(),
                        ComponentIR {
                            name: "App".into(),
                            generic_params: vec![],
                            props: HashMap::new(),
                            prop_defaults: HashMap::new(),
                            states: vec![],
                            computeds: vec![],
                            effects: vec![],
                            lifecycles: vec![],
                            fns: vec![FunctionIR {
                                name: "run".into(),
                                visibility: "private".to_string(),
                                params: vec![],
                                return_type: Some(TypeIR {
                                    kind: "void".into(),
                                    type_args: vec![],
                                    is_nullable: false,
                                    needs_clone: false,
                                    needs_partial_eq: false,
                                }),
                                body: BlockIR {
                                    statements: vec![],
                                    execution_mode: "client-only".into(),
                                },
                                is_async: false,
                            }],
                            store_refs: vec![],
                            refs: vec![],
                            event_names: vec![],
                            view: None,
                            styles: vec![],
                            motions: vec![],
                            timelines: vec![],
                        },
                    )]),
                    stores: HashMap::new(),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].path, "main.ts");
        let content = &output.files[0].content;
        assert!(content.contains("class App {"));
        assert!(content.contains("run(): void {"));
    }

    // ── Logic construct tests (standalone functions, structs, enums, type aliases) ──

    #[test]
    fn test_js_generate_struct() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Point".into(),
            visibility: "public".to_string(),
            fields: vec![
                FieldIR {
                    name: "x".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                    visibility: "public".into(),
                },
                FieldIR {
                    name: "y".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                    visibility: "public".into(),
                },
            ],
        };
        generate_struct(&mut w, &struct_ir, false);
        let out = w.into_string();
        assert!(out.contains("class Point {"));
        assert!(out.contains("constructor(x, y) {"));
        assert!(out.contains("this.x = x;"));
        assert!(out.contains("this.y = y;"));
    }

    #[test]
    fn test_ts_generate_struct() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Point".into(),
            visibility: "public".to_string(),
            fields: vec![
                FieldIR {
                    name: "x".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                    visibility: "public".into(),
                },
                FieldIR {
                    name: "y".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                    visibility: "public".into(),
                },
            ],
        };
        generate_struct(&mut w, &struct_ir, true);
        let out = w.into_string();
        assert!(out.contains("class Point {"));
        assert!(out.contains("constructor(x: number, y: number) {"));
    }

    #[test]
    fn test_js_generate_empty_struct() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Empty".into(),
            visibility: "public".to_string(),
            fields: vec![],
        };
        generate_struct(&mut w, &struct_ir, false);
        let out = w.into_string();
        assert!(out.contains("class Empty {"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_js_generate_enum() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Color".into(),
            visibility: "public".to_string(),
            variants: vec![
                VariantIR {
                    name: "Red".into(),
                    payload: None,
                },
                VariantIR {
                    name: "Green".into(),
                    payload: None,
                },
                VariantIR {
                    name: "Blue".into(),
                    payload: None,
                },
            ],
        };
        generate_enum(&mut w, &enum_ir, false);
        let out = w.into_string();
        assert!(out.contains("const Color = Object.freeze({"));
        assert!(out.contains("Red: 0,"));
        assert!(out.contains("Green: 1,"));
        assert!(out.contains("Blue: 2,"));
        assert!(out.contains("});"));
    }

    #[test]
    fn test_js_generate_empty_enum() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Empty".into(),
            visibility: "public".to_string(),
            variants: vec![],
        };
        generate_enum(&mut w, &enum_ir, false);
        let out = w.into_string();
        assert!(out.contains("const Empty = Object.freeze({"));
        assert!(out.contains("});"));
    }

    #[test]
    fn test_ts_generate_type_alias() {
        let mut w = CodeWriter::new();
        let alias = TypeAliasIR {
            name: "UserId".into(),
            visibility: "public".to_string(),
            target: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        };
        generate_type_alias(&mut w, &alias, true);
        assert_eq!(w.into_string(), "export type UserId = number;\n");
    }

    #[test]
    fn test_js_skip_type_alias() {
        let mut w = CodeWriter::new();
        let alias = TypeAliasIR {
            name: "UserId".into(),
            visibility: "public".to_string(),
            target: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        };
        generate_type_alias(&mut w, &alias, false);
        // CodeWriter always appends trailing newline, even when empty
        assert_eq!(w.into_string(), "\n");
    }

    #[test]
    fn test_js_generate_standalone_function() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "main".into(),
            visibility: "public".to_string(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_standalone_function(&mut w, &func, false);
        let out = w.into_string();
        assert!(out.contains("function main() {"));
        assert!(out.contains(";"));
    }

    #[test]
    fn test_ts_generate_standalone_function() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "add".into(),
            visibility: "public".to_string(),
            params: vec![ParamIR {
                name: "a".into(),
                r#type: Some(TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                }),
            }],
            return_type: Some(TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_standalone_function(&mut w, &func, true);
        let out = w.into_string();
        assert!(out.contains("function add(a: number): number {"));
    }

    #[test]
    fn test_ts_generate_standalone_function_private() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "helper".into(),
            visibility: "private".to_string(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_standalone_function(&mut w, &func, false);
        let out = w.into_string();
        assert!(out.contains("function helper() {"));
        assert!(!out.contains("export"));
    }

    #[test]
    fn test_format_type_ir_js() {
        let ty = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(format_type_ir_js(&ty, false), "");
        assert_eq!(format_type_ir_js(&ty, true), "number");

        let str_ty = TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(format_type_ir_js(&str_ty, true), "string");

        let vec_ty = TypeIR {
            kind: "Vec".into(),
            type_args: vec![ty.clone()],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(format_type_ir_js(&vec_ty, true), "number[]");
    }

    #[test]
    fn test_js_generate_struct_private() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Internal".into(),
            visibility: "private".to_string(),
            fields: vec![],
        };
        generate_struct(&mut w, &struct_ir, false);
        let out = w.into_string();
        assert!(!out.contains("export"));
    }

    #[test]
    fn test_format_variant_literal_js() {
        let expr = ExprIR::VariantLiteral {
            union_type: "HttpResult".into(),
            variant: "Ok".into(),
            fields: vec![(
                "value".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(42),
                },
            )],
        };
        let out = format_expr_ir(&expr, false);
        assert!(
            out.contains("\"_variant\": \"Ok\""),
            "Variant literal should contain _variant marker. Output: {}",
            out
        );
        assert!(
            out.contains("\"value\": 42"),
            "Variant literal should contain fields. Output: {}",
            out
        );
    }

    // ── Match expression tests ─────────────────────────

    #[test]
    fn test_match_expr_single_variant() {
        let expr = ExprIR::MatchExpr {
            matched: Box::new(ExprIR::IdentifierExpr {
                name: "result".into(),
            }),
            arms: vec![MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Ok".into(),
                        bindings: vec!["value".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::IdentifierExpr {
                            name: "value".into(),
                        }),
                    }],
                    execution_mode: "client-only".into(),
                },
            }],
        };
        let out = format_expr_ir(&expr, false);
        assert!(
            out.contains("_v._variant"),
            "Match should check _variant. Output: {}",
            out
        );
        assert!(
            out.contains("\"Ok\""),
            "Match should check for Ok variant. Output: {}",
            out
        );
        assert!(
            out.contains("return value"),
            "Match arm should return value. Output: {}",
            out
        );
    }

    #[test]
    fn test_match_expr_with_wildcard() {
        let expr = ExprIR::MatchExpr {
            matched: Box::new(ExprIR::IdentifierExpr { name: "r".into() }),
            arms: vec![
                MatchArmIR {
                    pattern: MatchPatternIR {
                        kind: MatchPatternKindIR::Variant {
                            name: "Ok".into(),
                            bindings: vec!["v".into()],
                        },
                    },
                    body: BlockIR {
                        statements: vec![StatementIR::ReturnStmtIR {
                            value: Some(ExprIR::IdentifierExpr { name: "v".into() }),
                        }],
                        execution_mode: "client-only".into(),
                    },
                },
                MatchArmIR {
                    pattern: MatchPatternIR {
                        kind: MatchPatternKindIR::Wildcard,
                    },
                    body: BlockIR {
                        statements: vec![StatementIR::ReturnStmtIR {
                            value: Some(ExprIR::LiteralExpr {
                                value: serde_json::json!(-1),
                            }),
                        }],
                        execution_mode: "client-only".into(),
                    },
                },
            ],
        };
        let out = format_expr_ir(&expr, false);
        assert!(
            out.contains("return -1"),
            "Wildcard arm should return -1. Output: {}",
            out
        );
        // Wildcard arm should not be guarded by an if-check
        let ok_count = out.matches("\"Ok\"").count();
        assert_eq!(
            ok_count, 1,
            "Ok should appear exactly once. Output: {}",
            out
        );
    }

    #[test]
    fn test_match_expr_multiple_variants() {
        let expr = ExprIR::MatchExpr {
            matched: Box::new(ExprIR::IdentifierExpr { name: "r".into() }),
            arms: vec![
                MatchArmIR {
                    pattern: MatchPatternIR {
                        kind: MatchPatternKindIR::Variant {
                            name: "Ok".into(),
                            bindings: vec![],
                        },
                    },
                    body: BlockIR {
                        statements: vec![StatementIR::ReturnStmtIR {
                            value: Some(ExprIR::LiteralExpr {
                                value: serde_json::json!("success"),
                            }),
                        }],
                        execution_mode: "client-only".into(),
                    },
                },
                MatchArmIR {
                    pattern: MatchPatternIR {
                        kind: MatchPatternKindIR::Variant {
                            name: "Err".into(),
                            bindings: vec![],
                        },
                    },
                    body: BlockIR {
                        statements: vec![StatementIR::ReturnStmtIR {
                            value: Some(ExprIR::LiteralExpr {
                                value: serde_json::json!("failure"),
                            }),
                        }],
                        execution_mode: "client-only".into(),
                    },
                },
            ],
        };
        let out = format_expr_ir(&expr, false);
        assert!(out.contains("\"Ok\""), "Should check Ok. Output: {}", out);
        assert!(out.contains("\"Err\""), "Should check Err. Output: {}", out);
        assert!(
            out.contains("\"success\""),
            "Should return success. Output: {}",
            out
        );
        assert!(
            out.contains("\"failure\""),
            "Should return failure. Output: {}",
            out
        );
    }

    #[test]
    fn test_format_block_as_js_expr_with_return() {
        let block = BlockIR {
            statements: vec![
                StatementIR::LetStmtIR {
                    name: "x".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    }),
                    r#type: None,
                },
                StatementIR::ReturnStmtIR {
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(42),
                    }),
                },
            ],
            execution_mode: "client-only".into(),
        };
        let out = format_block_as_js_expr(&block, false);
        assert_eq!(out, "42", "Should extract return value. Got: {}", out);
    }

    #[test]
    fn test_format_block_as_js_expr_with_expr_stmt() {
        let block = BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::IdentifierExpr {
                    name: "result".into(),
                },
            }],
            execution_mode: "client-only".into(),
        };
        let out = format_block_as_js_expr(&block, false);
        assert_eq!(out, "result", "Should extract expr stmt. Got: {}", out);
    }
}
