//! Python codegen: IR -> Python.
//!
//! Generates Python source files from Intent IR. Each module becomes
//! a .py file with:
//! - Type aliases (e.g., `UserId = int`)
//! - Structs as `@dataclass` classes
//! - Enums as `Enum` classes
//! - Standalone functions (`def` without `self`)
//! - Stores as standalone classes
//! - Components as classes with methods

use std::collections::HashMap;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::ir::*;

pub struct PythonCodegen;

impl Codegen for PythonCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_key, module) in &ir.modules {
            // Strip .omni suffix if present to derive module name
            let module_name = module_key.strip_suffix(".omni").unwrap_or(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit(&format!("# Generated from {}", source_path));
            w.emit("");

            // Generate imports needed for logic constructs
            generate_module_imports(&mut w, module);

            // Generate type aliases (e.g., UserId = int)
            for (_name, alias) in &module.type_aliases {
                generate_type_alias(&mut w, alias);
            }

            // Separate logic constructs from components/stores with a blank line
            if !module.type_aliases.is_empty() {
                w.emit("");
            }

            // Generate structs as @dataclass classes
            for (_name, struct_ir) in &module.structs {
                generate_struct(&mut w, struct_ir);
                w.emit("");
            }

            // Generate enums as Enum classes
            for (_name, enum_ir) in &module.enums {
                generate_enum(&mut w, enum_ir);
                w.emit("");
            }

            // Generate standalone functions
            let mut has_main = false;
            for (fname, func) in &module.functions {
                generate_function(&mut w, func, true);
                w.emit("");
                if fname == "main" {
                    has_main = true;
                }
            }

            // Entry point: if there's a main(), call it
            if has_main {
                w.emit("if __name__ == \"__main__\":");
                w.indent();
                w.emit("main()");
                w.dedent();
                w.emit("");
            }

            // Generate stores from this module
            for (store_name, store) in &module.stores {
                generate_store(&mut w, store_name, store);
                w.emit("");
            }

            // Generate components from this module
            for (comp_name, comp) in &module.components {
                generate_component(&mut w, comp_name, comp);
                w.emit("");
            }

            let out_path = format!("{}.py", module_name);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

// ═══════════════════════════════════════════════════════════
// Module-level helpers
// ═══════════════════════════════════════════════════════════

/// Generate import statements needed by logic constructs.
fn generate_module_imports(w: &mut CodeWriter, module: &ModuleIR) {
    let mut imports: Vec<&str> = Vec::new();
    if !module.structs.is_empty() {
        imports.push("from dataclasses import dataclass");
    }
    if !module.enums.is_empty() {
        imports.push("from enum import Enum");
    }
    for imp in &imports {
        w.emit(imp);
    }
    if !imports.is_empty() {
        w.emit("");
    }
}

/// Generate a type alias: `UserId = int`
fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR) {
    let target = format_type_ir(&alias.target);
    w.emit(&format!("{} = {}", alias.name, target));
}

/// Generate a struct as a `@dataclass` class.
fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR) {
    w.emit("@dataclass");
    w.emit(&format!("class {}:", struct_ir.name));
    w.indent();
    for field in &struct_ir.fields {
        let type_str = format_type_ir(&field.r#type);
        w.emit(&format!("{}: {}", field.name, type_str));
    }
    if struct_ir.fields.is_empty() {
        w.emit("pass");
    }
    w.dedent();
}

/// Generate an enum as an `Enum` class.
fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR) {
    w.emit(&format!("class {}(Enum):", enum_ir.name));
    w.indent();
    if enum_ir.variants.is_empty() {
        w.emit("pass");
    } else {
        for (i, variant) in enum_ir.variants.iter().enumerate() {
            w.emit(&format!("{} = {}", variant.name, i));
        }
    }
    w.dedent();
}

// ═══════════════════════════════════════════════════════════
// Component generation
// ═══════════════════════════════════════════════════════════

fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    w.emit(&format!("class {}:", name));
    w.indent();

    // Constructor with signals as instance variables
    if !comp.states.is_empty() || !comp.props.is_empty() {
        w.emit("def __init__(self):");
        w.indent();
        for prop in comp.props.keys() {
            w.emit(&format!("self.{} = None  # prop", prop));
        }
        for state in &comp.states {
            let py_init = format_expr_ir(&state.initial_value);
            w.emit(&format!("self.{} = {}", state.name, py_init));
        }
        w.dedent();
        w.emit("");
    }

    // Computed properties as @property methods
    for computed in &comp.computeds {
        let body = format_expr_ir(&computed.body);
        // Use @cached_property or simple @property for now
        w.emit("@property");
        w.emit(&format!("def {}(self):", computed.name));
        w.indent();
        w.emit(&format!("return {}", body));
        w.dedent();
        w.emit("");
    }

    // Functions (methods)
    for func in &comp.fns {
        generate_function(w, func, false);
        w.emit("");
    }

    w.dedent();
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    w.emit(&format!("class {}:", name));
    w.indent();

    // Constructor with signal defaults
    w.emit("def __init__(self):");
    w.indent();
    for (signal_name, _signal_type) in &store.signals {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(format_json_value)
            .unwrap_or_else(|| "None".into());
        w.emit(&format!("self.{} = {}", signal_name, default));
    }
    w.dedent();

    // Getters as properties
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        w.emit("@property");
        w.emit(&format!("def {}(self):", getter_name));
        w.indent();
        w.emit(&format!("return {}", format_expr_ir(getter_expr)));
        w.dedent();
    }

    w.dedent();
}

// ═══════════════════════════════════════════════════════════
// Function generation
// ═══════════════════════════════════════════════════════════

fn generate_function(w: &mut CodeWriter, func: &FunctionIR, is_standalone: bool) {
    // Build parameter list
    let mut param_strs = Vec::new();
    if !is_standalone {
        param_strs.push("self".to_string());
    }
    for p in &func.params {
        let mut s = p.name.clone();
        // Optionally include type annotation
        if let Some(ref ty) = p.r#type {
            s.push_str(&format!(": {}", format_type_ir(ty)));
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", format_type_ir(rt)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");

    w.emit(&format!("def {}({}){}:", func.name, all_params, ret_ann));
    w.indent();

    if func.body.statements.is_empty() {
        w.emit("pass");
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt);
        }
    }

    w.dedent();
}

// ═══════════════════════════════════════════════════════════
// Statement generation
// ═══════════════════════════════════════════════════════════

fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "None".into());
            let type_hint = r#type
                .as_ref()
                .map(|t| format!(": {}", format_type_ir(t)))
                .unwrap_or_default();
            w.emit(&format!("{}{} = {}", name, type_hint, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("if {}:", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_ir(elif_cond);
                w.emit(&format!("elif {}:", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement(w, s);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("else:");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement(w, s);
                    }
                    w.dedent();
                }
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "None".into());
            w.emit(&format!("return {}", rhs));
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(mut_intent) = intent {
                let target = &mut_intent.target.name;
                let amount = mut_intent
                    .amount
                    .as_ref()
                    .map(format_expr_ir)
                    .unwrap_or_else(|| "0".into());
                match mut_intent.operation.as_str() {
                    "direct_assign" => {
                        w.emit(&format!("{} = {}", target, amount));
                    }
                    "increment" => {
                        w.emit(&format!("{} += {}", target, amount));
                    }
                    "decrement" => {
                        w.emit(&format!("{} -= {}", target, amount));
                    }
                    other => {
                        w.emit(&format!(
                            "# TODO: mutation operation '{}' on {}",
                            other, target
                        ));
                    }
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = format_expr_ir(expr);
            // Skip null expressions
            if line != "None" {
                w.emit(&line);
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(format_expr_ir).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("# emit event: {}{}", event_name, arg_str));
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                generate_statement(w, s);
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("while {}:", cond));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = format_expr_ir(iterable);
            w.emit(&format!("for {} in {}:", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
        }
        StatementIR::BreakStmtIR => {
            w.emit("break");
        }
        StatementIR::ContinueStmtIR => {
            w.emit("continue");
        }
        _ => {}
    }
}

fn format_block_as_python_expr(block: &BlockIR) -> String {
    if let Some(last) = block.statements.last() {
        match last {
            StatementIR::ExprStmtIR { expr } => format_expr_ir(expr),
            StatementIR::ReturnStmtIR { value } => value
                .as_ref()
                .map(|e| format_expr_ir(e))
                .unwrap_or_else(|| "None".into()),
            _ => "None".into(),
        }
    } else {
        "None".into()
    }
}

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

fn format_expr_ir(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let py_op = binary_op_to_python(op);
            let lhs = format_expr_ir(left);
            let rhs = format_expr_ir(right);
            format!("{} {} {}", lhs, py_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let py_op = match op.as_str() {
                "not" => "not ",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", py_op, format_expr_ir(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond);
            let t = format_expr_ir(then_expr);
            let e = format_expr_ir(else_expr);
            format!("{} if {} else {}", t, c, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(format_expr_ir).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements.iter().map(format_expr_ir).collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { type_name, fields } => {
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(name, val)| format!("{}={}", name, format_expr_ir(val)))
                .collect();
            format!("{}({})", type_name, field_strs.join(", "))
        }
        ExprIR::MethodCallExpr { is_optional,
            object,
            method,
            args,
        } => {
            let obj = format_expr_ir(object);
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir).collect();
            match method.as_str() {
                "len" | "length" => format!("len({})", obj),
                "is_empty" => format!("len({}) == 0", obj),
                "push" => format!("{}.append({})", obj, arg_strs.join(", ")),
                "pop" => format!("{}.pop()", obj),
                "contains" => format!("({} in {})", arg_strs.join(", "), obj),
                _ => format!("{}.{}({})", obj, method, arg_strs.join(", ")),
            }
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_ir(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            format!("{}[{}]", format_expr_ir(object), format_expr_ir(index))
        }
        ExprIR::ClosureExpr { params, body } => {
            let body_str = format_expr_ir(body);
            format!("lambda {}: {}", params.join(", "), body_str)
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir(operand))
        }
        ExprIR::VariantLiteral {
            variant, fields, ..
        } => {
            let mut entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v)))
                .collect();
            entries.insert(0, format!("\"_variant\": \"{}\"", variant));
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let mut parts = Vec::new();
            for (i, arm) in arms.iter().enumerate() {
                let body = format_block_as_python_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, .. } => {
                        let kw = if i == 0 { "if" } else { "elif" };
                        parts.push(format!("({body}) {kw} {m}.get(\"_variant\") == \"{name}\""));
                    }
                    MatchPatternKindIR::Wildcard => {
                        parts.push(format!("({body})"));
                    }
                    _ => {}
                }
            }
            if parts.is_empty() {
                "None".into()
            } else {
                parts.join(" else ")
            }
        }
        _ => "None".into(),
    }
}

/// Format a JSON value as a Python literal.
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".into(),
        serde_json::Value::Bool(b) => {
            if *b {
                "True".into()
            } else {
                "False".into()
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
            // Escape special characters for Python string literals
            let escaped = s
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("'{}'", escaped)
        }
        // Arrays and objects: use repr
        other => other.to_string(),
    }
}

/// Map OMNI binary operator name to Python operator.
fn binary_op_to_python(op: &str) -> &str {
    match op {
        "add" => "+",
        "sub" => "-",
        "mul" => "*",
        "div" => "/",
        "mod" => "%",
        "eq" => "==",
        "neq" => "!=",
        "lt" => "<",
        "gt" => ">",
        "lteq" => "<=",
        "gteq" => ">=",
        "and" => "and",
        "or" => "or",
        "assign" => "=",
        "addassign" => "+=",
        "subassign" => "-=",
        "mulassign" => "*=",
        "divassign" => "/=",
        "modassign" => "%=",
        _ => op,
    }
}

/// Format a TypeIR as a Python type annotation string.
fn format_type_ir(ty: &TypeIR) -> String {
    match ty.kind.as_str() {
        "i32" | "i64" | "int" | "u32" | "u64" => "int".into(),
        "f64" | "f32" | "float" => "float".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "str".into(),
        "void" | "()" => "None".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = ty.type_args.first() {
                format!("list[{}]", format_type_ir(inner))
            } else {
                "list".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if ty.type_args.len() >= 2 {
                let args: Vec<String> = ty.type_args.iter().map(format_type_ir).collect();
                format!("dict[{}]", args.join(", "))
            } else {
                "dict".into()
            }
        }
        "Option" => {
            if let Some(inner) = ty.type_args.first() {
                format!("Optional[{}]", format_type_ir(inner))
            } else {
                "Optional[Any]".into()
            }
        }
        other => {
            // If it looks like a user-defined type, use as-is
            if other.chars().next().map_or(false, |c| c.is_uppercase()) {
                format!("'{}'", other) // forward reference
            } else {
                "Any".into()
            }
        }
    }
}

/// Map OMNI type to Python type annotation string.
pub fn py_type(kind: &str, type_args: &[TypeIR]) -> String {
    match kind {
        "int" | "i32" | "i64" | "u32" | "u64" => "int".into(),
        "float" | "f64" | "f32" => "float".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "str".into(),
        "void" | "()" => "None".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("list[{}]", py_type(&inner.kind, &inner.type_args))
            } else {
                "list".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "dict[{}]",
                    type_args
                        .iter()
                        .map(|t| py_type(&t.kind, &t.type_args))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                "dict".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("Optional[{}]", py_type(&inner.kind, &inner.type_args))
            } else {
                "Optional[Any]".into()
            }
        }
        other => {
            // If it looks like a user-defined type, use as-is
            if other.chars().next().map_or(false, |c| c.is_uppercase()) {
                format!("'{}'", other) // forward reference
            } else {
                "Any".into()
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Unit tests
// ═══════════════════════════════════════════════════════════
