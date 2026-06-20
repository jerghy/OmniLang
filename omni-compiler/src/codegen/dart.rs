//! Dart codegen: IR -> Dart.
//!
//! Generates Dart source files from Intent IR. Each component becomes
//! a class, each function becomes a method, and stores become standalone
//! classes.

use std::collections::HashMap;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::ir::*;

pub struct DartCodegen;

impl Codegen for DartCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_key, module) in &ir.modules {
            let module_name = super::sanitize_module_name(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Generate type aliases
            for alias in module.type_aliases.values() {
                generate_type_alias(&mut w, alias);
            }
            if !module.type_aliases.is_empty() {
                w.emit("");
            }

            // Generate structs as classes
            for struct_ir in module.structs.values() {
                generate_struct(&mut w, struct_ir);
                w.emit("");
            }

            // Generate enums
            for enum_ir in module.enums.values() {
                generate_enum(&mut w, enum_ir);
                w.emit("");
            }

            // Generate standalone functions
            for func in module.functions.values() {
                generate_standalone_function(&mut w, func);
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

            let out_path = format!("{}.dart", module_name);
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

fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    w.emit(&format!("class {} {{", name));
    w.indent();

    if comp.states.is_empty()
        && comp.props.is_empty()
        && comp.fns.is_empty()
        && comp.computeds.is_empty()
    {
        // Empty class — just emit default constructor
        w.emit(&format!("{}();", name));
        w.dedent();
        w.emit("}");
        return;
    }

    // Field declarations for props (initialized to null)
    for (prop_name, prop_type) in &comp.props {
        let ann = dart_type(&prop_type.kind, &prop_type.type_args);
        w.emit(&format!("{} {};", ann, prop_name));
    }

    // Field declarations + initialization for states
    for state in &comp.states {
        let ann = dart_type(&state.r#type.kind, &state.r#type.type_args);
        let init = format_expr_ir(&state.initial_value);
        // Use `late` for fields initialized in constructor
        w.emit(&format!("{} {} = {};", ann, state.name, init));
    }

    // Constructor with state/prop initialization
    if !comp.states.is_empty() || !comp.props.is_empty() {
        w.emit("");
        w.emit(&format!("{}() {{", name));
        w.indent();
        for prop in comp.props.keys() {
            w.emit(&format!("this.{} = null;  // prop", prop));
        }
        // States are already initialized inline, but include as comments for clarity
        for state in &comp.states {
            let init = format_expr_ir(&state.initial_value);
            w.emit(&format!("this.{} = {};", state.name, init));
        }
        w.dedent();
        w.emit("}");
    }

    // Computed properties as methods
    for computed in &comp.computeds {
        w.emit("");
        let body = format_expr_ir(&computed.body);
        let ret_type = dart_type(&computed.r#type.kind, &computed.r#type.type_args);
        w.emit(&format!("{} {}() {{", ret_type, computed.name));
        w.indent();
        w.emit(&format!("return {};", body));
        w.dedent();
        w.emit("}");
    }

    // Functions (methods)
    for func in &comp.fns {
        w.emit("");
        generate_function(w, func);
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    w.emit(&format!("class {} {{", name));
    w.indent();

    if store.signals.is_empty() && store.getters.is_empty() {
        w.emit(&format!("{}();", name));
        w.dedent();
        w.emit("}");
        return;
    }

    // Field declarations with defaults for signals
    for (signal_name, signal_type) in &store.signals {
        let ann = dart_type(&signal_type.kind, &signal_type.type_args);
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(format_json_value)
            .unwrap_or_else(|| "null".into());
        w.emit(&format!("{} {} = {};", ann, signal_name, default));
    }

    // Default constructor
    if !store.signals.is_empty() {
        w.emit("");
        w.emit(&format!("{}() {{", name));
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
    }

    // Getters
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        w.emit(&format!("get {} {{", getter_name));
        w.indent();
        w.emit(&format!("return {};", format_expr_ir(getter_expr)));
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Function generation
// ═══════════════════════════════════════════════════════════

fn generate_function(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = dart_type(&ty.kind, &ty.type_args);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type (skip void — implicit in Dart)
    // Wrap in Future<T> for async functions
    let return_prefix = if func.is_async {
        match func.return_type {
            Some(ref rt) if rt.kind != "void" && rt.kind != "()" => {
                format!("Future<{}> ", dart_type(&rt.kind, &rt.type_args))
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| rt.kind != "void" && rt.kind != "()")
            .map(|rt| format!("{} ", dart_type(&rt.kind, &rt.type_args)))
            .unwrap_or_default()
    };

    let async_kw = if func.is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit(&format!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — valid Dart syntax
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt);
        }
    }

    w.dedent();
    w.emit("}");
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
                .unwrap_or_else(|| "null".into());
            if let Some(ref ty) = r#type {
                let ann = dart_type(&ty.kind, &ty.type_args);
                w.emit(&format!("{} {} = {};", ann, name, rhs));
            } else {
                w.emit(&format!("var {} = {};", name, rhs));
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_body,
        } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement(w, s);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
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
                .map(format_expr_ir)
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
                other => {
                    w.emit(&format!(
                        "// TODO: mutation operation '{}' on '{}'",
                        other, target
                    ));
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = format_expr_ir(expr);
            // Skip null expressions
            if line != "null" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(format_expr_ir).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("while ({}) {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = format_expr_ir(iterable);
            w.emit(&format!("for (var {} in {}) {{", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
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

fn format_expr_ir(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let dart_op = binary_op_to_dart(op);
            let lhs = format_expr_ir(left);
            let rhs = format_expr_ir(right);
            format!("{} {} {}", lhs, dart_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let dart_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", dart_op, format_expr_ir(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond);
            let t = format_expr_ir(then_expr);
            let e = format_expr_ir(else_expr);
            format!("{} ? {} : {}", c, t, e)
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
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir(v)))
                .collect();
            format!("{}.{}({})", union_type, variant, entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir(object);
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_ir(a)).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr { tail, .. } => format_expr_ir(tail),
        ExprIR::IndexExpr { .. } => "null".to_string(),
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir(operand))
        }
                ExprIR::MatchExpr { .. } => "null".into(),
        ExprIR::ClosureExpr { .. } => "null".to_string(),
    }
}

/// Format a JSON value as a Dart literal.
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
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("'{}'", escaped)
        }
        other => other.to_string(),
    }
}

/// Map OMNI binary operator name to Dart operator.
fn binary_op_to_dart(op: &str) -> &str {
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

/// Map OMNI type to Dart type annotation string.
pub fn dart_type(kind: &str, type_args: &[TypeIR]) -> String {
    match kind {
        "int" | "i32" | "i64" | "u32" | "u64" => "int".into(),
        "float" | "f64" | "f32" => "double".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "String".into(),
        "void" | "()" => "void".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("List<{}>", dart_type(&inner.kind, &inner.type_args))
            } else {
                "List".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "Map<{}, {}>",
                    dart_type(&type_args[0].kind, &type_args[0].type_args),
                    dart_type(&type_args[1].kind, &type_args[1].type_args)
                )
            } else {
                "Map".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("{}?", dart_type(&inner.kind, &inner.type_args))
            } else {
                "Object?".into()
            }
        }
        other => {
            // If it looks like a user-defined type (uppercase), use as-is
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                other.into()
            } else {
                "dynamic".into()
            }
        }
    }
}

/// Format a TypeIR into a Dart type string via the existing `dart_type()`.
fn format_type_ir_dart(ty: &TypeIR) -> String {
    dart_type(&ty.kind, &ty.type_args)
}

// ═══════════════════════════════════════════════════════════
// Logic construct generation (structs, enums, type aliases, standalone functions)
// ═══════════════════════════════════════════════════════════

fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR) {
    w.emit(&format!("class {} {{", struct_ir.name));
    w.indent();

    // Field declarations (final)
    for field in &struct_ir.fields {
        let type_str = format_type_ir_dart(&field.r#type);
        w.emit(&format!("final {} {};", type_str, field.name));
    }

    // Constructor with named params
    if !struct_ir.fields.is_empty() {
        w.emit("");
        let mut param_strs = Vec::new();
        for field in &struct_ir.fields {
            param_strs.push(format!("required this.{}", field.name));
        }
        let all_params = param_strs.join(", ");
        w.emit(&format!("{}({{ {} }});", struct_ir.name, all_params));
    }

    if struct_ir.fields.is_empty() {
        w.emit(&format!("{}();", struct_ir.name));
    }

    w.dedent();
    w.emit("}");
}

fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR) {
    w.emit(&format!("enum {} {{", enum_ir.name));
    w.indent();
    if !enum_ir.variants.is_empty() {
        let names: Vec<String> = enum_ir.variants.iter().map(|v| v.name.clone()).collect();
        w.emit(&names.join(", ").to_string());
    }
    w.dedent();
    w.emit("}");
}

fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR) {
    let target = format_type_ir_dart(&alias.target);
    w.emit(&format!("typedef {} = {};", alias.name, target));
}

fn generate_standalone_function(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = format_type_ir_dart(ty);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type (skip void — implicit in Dart)
    let return_prefix = func
        .return_type
        .as_ref()
        .filter(|rt| rt.kind != "void" && rt.kind != "()")
        .map(|rt| format!("{} ", format_type_ir_dart(rt)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");
    w.emit(&format!(
        "{}{}({}) {{",
        return_prefix, func.name, all_params
    ));
    w.indent();

    if !func.body.statements.is_empty() {
        for stmt in &func.body.statements {
            generate_statement(w, stmt);
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

    // ── dart_type tests ────────────────────────────────────

    #[test]
    fn test_dart_type_primitives() {
        assert_eq!(dart_type("int", &[]), "int");
        assert_eq!(dart_type("i32", &[]), "int");
        assert_eq!(dart_type("i64", &[]), "int");
        assert_eq!(dart_type("f64", &[]), "double");
        assert_eq!(dart_type("bool", &[]), "bool");
        assert_eq!(dart_type("string", &[]), "String");
        assert_eq!(dart_type("void", &[]), "void");
    }

    #[test]
    fn test_dart_type_generics() {
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
            dart_type("Vec", std::slice::from_ref(&i32_arg)),
            "List<int>"
        );
        assert_eq!(
            dart_type("HashMap", &[string_arg.clone(), i32_arg.clone()]),
            "Map<String, int>"
        );
    }

    #[test]
    fn test_dart_type_option() {
        let i32_arg = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(dart_type("Option", &[i32_arg]), "int?");
    }

    #[test]
    fn test_dart_type_user_defined() {
        assert_eq!(dart_type("MyType", &[]), "MyType");
        assert_eq!(dart_type("unknown", &[]), "dynamic");
    }

    // ── format_json_value tests ────────────────────────────

    #[test]
    fn test_format_json_value() {
        assert_eq!(format_json_value(&serde_json::Value::Null), "null");
        assert_eq!(format_json_value(&serde_json::Value::Bool(true)), "true");
        assert_eq!(format_json_value(&serde_json::Value::Bool(false)), "false");
        assert_eq!(format_json_value(&serde_json::json!(42)), "42");
        assert_eq!(format_json_value(&serde_json::json!("hello")), "'hello'");
        assert_eq!(
            format_json_value(&serde_json::json!("he'llo")),
            "'he\\'llo'"
        );
    }

    // ── binary_op_to_dart tests ────────────────────────────

    #[test]
    fn test_binary_op_to_dart() {
        assert_eq!(binary_op_to_dart("add"), "+");
        assert_eq!(binary_op_to_dart("eq"), "==");
        assert_eq!(binary_op_to_dart("and"), "&&");
        assert_eq!(binary_op_to_dart("or"), "||");
    }

    // ── format_expr_ir tests ─────────────────────────────

    #[test]
    fn test_format_expr_ir_literal() {
        let expr = ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        };
        assert_eq!(format_expr_ir(&expr), "42");
    }

    #[test]
    fn test_format_expr_ir_identifier() {
        let expr = ExprIR::IdentifierExpr {
            name: "count".into(),
        };
        assert_eq!(format_expr_ir(&expr), "count");
    }

    #[test]
    fn test_format_expr_ir_binary() {
        let expr = ExprIR::BinaryOpExpr {
            op: "add".into(),
            left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
            right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
        };
        assert_eq!(format_expr_ir(&expr), "a + b");
    }

    #[test]
    fn test_format_expr_ir_ternary() {
        let expr = ExprIR::TernaryIR {
            cond: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
            then_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            else_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            }),
        };
        assert_eq!(format_expr_ir(&expr), "x ? 1 : 0");
    }

    #[test]
    fn test_format_expr_ir_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
            field: "prop".into(),
        };
        assert_eq!(format_expr_ir(&expr), "obj.prop");
    }

    #[test]
    fn test_format_expr_ir_unary_not() {
        let expr = ExprIR::UnaryOpExpr {
            op: "not".into(),
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "flag".into(),
            }),
        };
        assert_eq!(format_expr_ir(&expr), "!flag");
    }

    #[test]
    fn test_format_expr_ir_unary_neg() {
        let expr = ExprIR::UnaryOpExpr {
            op: "neg".into(),
            operand: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
        };
        assert_eq!(format_expr_ir(&expr), "-x");
    }

    #[test]
    fn test_format_expr_ir_store_field_access() {
        let expr = ExprIR::StoreFieldAccessExpr {
            store_alias: "store".into(),
            field_name: "getCount".into(),
            call_args: vec![],
        };
        assert_eq!(format_expr_ir(&expr), "store.getCount");
    }

    // ── generate_function tests ───────────────────────────

    #[test]
    fn test_generate_function_empty() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "doSomething".into(),
            visibility: "public".into(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("doSomething() {"));
    }

    #[test]
    fn test_generate_function_with_return_type() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "getValue".into(),
            visibility: "public".into(),
            params: vec![],
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
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("int getValue() {"));
    }

    #[test]
    fn test_generate_function_with_void_return() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "doThing".into(),
            visibility: "public".into(),
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
        generate_function(&mut w, &func);
        let out = w.into_string();
        // void return type is implicit — no annotation emitted
        assert!(out.contains("doThing() {"));
    }

    #[test]
    fn test_generate_function_with_params() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "greet".into(),
            visibility: "public".into(),
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
            is_async: false,
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        // Note: dart_type("string") = "String", so the param type annotation looks like "name  String "
        assert!(out.contains("String greet("));
    }

    // ── generate_component tests ──────────────────────────

    #[test]
    fn test_generate_component_empty() {
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
        };
        generate_component(&mut w, "App", &comp);
        let out = w.into_string();
        assert!(out.contains("class App {"));
        assert!(out.contains("App();"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_generate_component_with_function() {
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
                visibility: "public".into(),
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
        };
        generate_component(&mut w, "App", &comp);
        let out = w.into_string();
        assert!(out.contains("class App {"));
        assert!(out.contains("greet() {"));
    }

    #[test]
    fn test_generate_component_with_state() {
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
        };
        generate_component(&mut w, "Counter", &comp);
        let out = w.into_string();
        assert!(out.contains("class Counter {"));
        assert!(out.contains("int count = 0;"));
    }

    // ── generate_store tests ──────────────────────────────

    #[test]
    fn test_generate_store() {
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
        generate_store(&mut w, "AppStore", &store);
        let out = w.into_string();
        assert!(out.contains("class AppStore {"));
        assert!(out.contains("int count = 0;"));
        assert!(out.contains("String name = 'world';"));
    }

    #[test]
    fn test_generate_store_empty() {
        let mut w = CodeWriter::new();
        let store = StoreIR {
            name: "EmptyStore".into(),
            signals: HashMap::new(),
            signal_defaults: HashMap::new(),
            getters: HashMap::new(),
            actions: vec![],
        };
        generate_store(&mut w, "EmptyStore", &store);
        let out = w.into_string();
        assert!(out.contains("class EmptyStore {"));
        assert!(out.contains("EmptyStore();"));
    }

    // ── generate_statement tests ──────────────────────────

    #[test]
    fn test_generate_statement_let() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            }),
            r#type: None,
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "var x = 42;\n");
    }

    #[test]
    fn test_generate_statement_return() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR {
            value: Some(ExprIR::IdentifierExpr {
                name: "result".into(),
            }),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "return result;\n");
    }

    #[test]
    fn test_generate_statement_mutation_increment() {
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
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "this.count += 1;\n");
    }

    #[test]
    fn test_generate_statement_mutation_assign() {
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
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "this.name = 'hello';\n");
    }

    #[test]
    fn test_generate_statement_mutation_decrement() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::MutationStmt {
            intent: Some(MutationIntent {
                target: SignalRefIR {
                    owner: None,
                    name: "count".into(),
                },
                operation: "decrement".into(),
                amount: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                item: None,
            }),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "this.count -= 1;\n");
    }

    #[test]
    fn test_generate_statement_if() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::IfStmtIR {
            condition: ExprIR::IdentifierExpr {
                name: "cond".into(),
            },
            then_body: BlockIR {
                statements: vec![StatementIR::LetStmtIR {
                    name: "x".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    }),
                    r#type: None,
                }],
                execution_mode: "client-only".into(),
            },
            else_body: Some(BlockIR {
                statements: vec![StatementIR::LetStmtIR {
                    name: "x".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    }),
                    r#type: None,
                }],
                execution_mode: "client-only".into(),
            }),
        };
        generate_statement(&mut w, &stmt);
        let out = w.into_string();
        assert!(out.contains("if (cond) {"));
        assert!(out.contains("var x = 1;"));
        assert!(out.contains("} else {"));
        assert!(out.contains("var x = 2;"));
        assert!(out.contains("}"));
    }

    // ── Full pipeline through trait ────────────────────────

    #[test]
    fn test_full_pipeline_through_trait() {
        let codegen = DartCodegen;
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
                                visibility: "public".into(),
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
        assert_eq!(output.files[0].path, "main.dart");
        let content = &output.files[0].content;
        assert!(content.contains("class App {"));
        assert!(content.contains("run() {"));
    }

    #[test]
    fn test_full_pipeline_with_store() {
        let codegen = DartCodegen;
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::new(),
                    stores: HashMap::from([(
                        "AppStore".into(),
                        StoreIR {
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
                            signal_defaults: HashMap::from([(
                                "count".into(),
                                serde_json::json!(0),
                            )]),
                            getters: HashMap::new(),
                            actions: vec![],
                        },
                    )]),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        let content = &output.files[0].content;
        assert!(content.contains("class AppStore {"));
        assert!(content.contains("int count = 0;"));
    }

    // ── Logic construct tests (structs, enums, type aliases, standalone functions) ──

    #[test]
    fn test_dart_generate_struct() {
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
        generate_struct(&mut w, &struct_ir);
        let out = w.into_string();
        assert!(out.contains("class Point {"));
        assert!(out.contains("final int x;"));
        assert!(out.contains("final int y;"));
        assert!(out.contains("Point({ required this.x, required this.y });"));
    }

    #[test]
    fn test_dart_generate_empty_struct() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Empty".into(),
            visibility: "public".to_string(),
            fields: vec![],
        };
        generate_struct(&mut w, &struct_ir);
        let out = w.into_string();
        assert!(out.contains("class Empty {"));
        assert!(out.contains("Empty();"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_dart_generate_enum() {
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
        generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("enum Color {"));
        assert!(out.contains("Red, Green, Blue"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_dart_generate_empty_enum() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Empty".into(),
            visibility: "public".to_string(),
            variants: vec![],
        };
        generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("enum Empty {"));
    }

    #[test]
    fn test_dart_generate_type_alias() {
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
        generate_type_alias(&mut w, &alias);
        assert_eq!(w.into_string(), "typedef UserId = int;\n");
    }

    #[test]
    fn test_dart_generate_standalone_function() {
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
        generate_standalone_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("main() {"));
    }

    #[test]
    fn test_dart_generate_standalone_function_with_return_type() {
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
        generate_standalone_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("int add(int a) {"));
    }

    #[test]
    fn test_dart_format_type_ir_dart() {
        let ty = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(format_type_ir_dart(&ty), "int");

        let str_ty = TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(format_type_ir_dart(&str_ty), "String");

        let vec_ty = TypeIR {
            kind: "Vec".into(),
            type_args: vec![ty],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(format_type_ir_dart(&vec_ty), "List<int>");
    }

    #[test]
    fn test_generate_async_function_dart() {
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
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(
            out.contains("Future<String> fetchData() async {"),
            "Dart async function should wrap return in Future. Output:\n{}",
            out
        );
    }

    #[test]
    fn test_generate_async_function_dart_void() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "doAsync".into(),
            visibility: "private".to_string(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: true,
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(
            out.contains("Future<void> doAsync() async {"),
            "Dart async void function should have Future<void>. Output:\n{}",
            out
        );
    }

    #[test]
    fn test_format_await_expr_dart() {
        let expr = ExprIR::AwaitExpr {
            operand: Box::new(ExprIR::CallExpr {
                name: "fetchData".into(),
                args: vec![],
            }),
        };
        let out = format_expr_ir(&expr);
        assert!(
            out.starts_with("await "),
            "Dart await should be prefix 'await'. Output: {}",
            out
        );
        assert!(
            out.contains("fetchData()"),
            "Dart await should contain the awaited call. Output: {}",
            out
        );
    }

    #[test]
    fn test_format_variant_literal_dart() {
        let expr = ExprIR::VariantLiteral {
            union_type: "HttpResult".into(),
            variant: "ok".into(),
            fields: vec![
                ("value".into(), ExprIR::LiteralExpr {
                    value: serde_json::json!(42),
                }),
            ],
        };
        let out = format_expr_ir(&expr);
        assert!(
            out.contains("HttpResult.ok"),
            "Dart variant should use dot syntax. Output: {}",
            out
        );
        assert!(
            out.contains("value: 42"),
            "Dart variant should contain fields. Output: {}",
            out
        );
    }
}
