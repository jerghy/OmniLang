//! Rust codegen: IR -> Rust.
//!
//! Generates Rust source files from Intent IR. Each component becomes
//! a struct with an impl block, each function becomes a method, and
//! stores become standalone structs with impl blocks.

use std::collections::HashMap;

use crate::ir::*;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};

pub struct RustCodegen;

impl Codegen for RustCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Generate stores (structs with impl blocks)
            for (store_name, store) in &ir.stores {
                generate_store(&mut w, store_name, store);
                w.emit("");
            }

            // Generate components as structs with impl blocks
            for (comp_name, comp) in &ir.components {
                generate_component(&mut w, comp_name, comp);
                w.emit("");
            }

            let out_path = source_path.replace(".omni", ".rs");
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
    let has_fields = !comp.states.is_empty() || !comp.props.is_empty();
    let has_methods = has_fields || !comp.fns.is_empty() || !comp.computeds.is_empty();

    // Emit struct definition
    if has_fields {
        w.emit(&format!("struct {} {{", name));
        w.indent();
        for (prop_name, prop_type) in &comp.props {
            w.emit(&format!("{}: {},", prop_name, rs_type(&prop_type.kind, &prop_type.type_args)));
        }
        for state in &comp.states {
            w.emit(&format!("{}: {},", state.name, rs_type(&state.r#type.kind, &state.r#type.type_args)));
        }
        w.dedent();
        w.emit("}");
    } else {
        w.emit(&format!("struct {};", name));
    }

    if !has_methods {
        return;
    }

    // Emit impl block
    w.emit(&format!("impl {} {{", name));
    w.indent();

    // Constructor (new) if there are fields
    if has_fields {
        w.emit("fn new() -> Self {");
        w.indent();
        if comp.states.is_empty() && comp.props.is_empty() {
            w.emit(&format!("{}", name));
        } else {
            w.emit(&format!("Self {{"));
            w.indent();
            for prop_name in comp.props.keys() {
                w.emit(&format!("{}: None,  // prop", prop_name));
            }
            for state in &comp.states {
                let rs_init = format_expr_ir(&state.initial_value);
                w.emit(&format!("{}: {},", state.name, rs_init));
            }
            w.dedent();
            w.emit("}");
        }
        w.dedent();
        w.emit("}");
        w.emit("");
    }

    // Computed properties as methods
    for computed in &comp.computeds {
        let body = format_expr_ir(&computed.body);
        let ret_type = rs_type(&computed.r#type.kind, &computed.r#type.type_args);
        w.emit(&format!("fn {}(&self) -> {} {{", computed.name, ret_type));
        w.indent();
        w.emit(&format!("{}", body));
        w.dedent();
        w.emit("}");
        w.emit("");
    }

    // Functions (methods)
    for func in &comp.fns {
        generate_function(w, func);
        w.emit("");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    if store.signals.is_empty() {
        w.emit(&format!("struct {};", name));
        w.emit("");
        w.emit(&format!("impl {} {{", name));
        w.indent();
        w.emit("fn new() -> Self {");
        w.indent();
        w.emit(&format!("{}", name));
        w.dedent();
        w.emit("}");
        w.dedent();
        w.emit("}");
        return;
    }

    // Struct with fields
    w.emit(&format!("struct {} {{", name));
    w.indent();
    for (signal_name, signal_type) in &store.signals {
        w.emit(&format!("{}: {},", signal_name, rs_type(&signal_type.kind, &signal_type.type_args)));
    }
    w.dedent();
    w.emit("}");

    // Impl block
    w.emit(&format!("impl {} {{", name));
    w.indent();

    // Constructor
    w.emit("fn new() -> Self {");
    w.indent();
    w.emit("Self {");
    w.indent();
    for (signal_name, _signal_type) in &store.signals {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(format_json_value)
            .unwrap_or_else(|| "None".into());
        w.emit(&format!("{}: {},", signal_name, default));
    }
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");

    // Getters as methods (no return type — TODO: infer from expression)
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        let body = format_expr_ir(getter_expr);
        w.emit(&format!("fn {}(&self) {{", getter_name));
        w.indent();
        w.emit(&format!("{}", body));
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
    // Build parameter list — prepend &self for method
    let mut param_strs = vec!["&self".to_string()];
    for p in &func.params {
        let mut s = p.name.clone();
        if let Some(ref ty) = p.r#type {
            let ann = rs_type(&ty.kind, &ty.type_args);
            s.push_str(&format!(": {}", ann));
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", rs_type(&rt.kind, &rt.type_args)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");

    w.emit(&format!("fn {}({}){} {{", func.name, all_params, ret_ann));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — valid Rust syntax
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
        StatementIR::LetStmtIR { name, value, r#type } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "None".into());
            let type_ann = r#type
                .as_ref()
                .map(|t| format!(": {}", rs_type(&t.kind, &t.type_args)))
                .unwrap_or_default();
            w.emit(&format!("let {}{} = {};", name, type_ann, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_body,
        } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("if {} {{", cond));
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
                .unwrap_or_else(|| "()".into());
            w.emit(&format!("return {};", rhs));
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
                        w.emit(&format!("self.{} = {};", target, amount));
                    }
                    "increment" => {
                        w.emit(&format!("self.{} += {};", target, amount));
                    }
                    "decrement" => {
                        w.emit(&format!("self.{} -= {};", target, amount));
                    }
                    other => {
                        w.emit(&format!("// TODO: mutation operation '{}' on '{}'", other, target));
                    }
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = format_expr_ir(expr);
            // Skip null expressions
            if line != "None" {
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
            let rs_op = binary_op_to_rust(op);
            let lhs = format_expr_ir(left);
            let rhs = format_expr_ir(right);
            format!("{} {} {}", lhs, rs_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let rs_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", rs_op, format_expr_ir(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond);
            let t = format_expr_ir(then_expr);
            let e = format_expr_ir(else_expr);
            format!("if {} {{ {} }} else {{ {} }}", c, t, e)
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
    }
}

/// Format a JSON value as a Rust literal.
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".into(),
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
            let escaped = s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\".to_string()", escaped)
        }
        other => other.to_string(),
    }
}

/// Map OMNI binary operator name to Rust operator.
fn binary_op_to_rust(op: &str) -> &str {
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

/// Map OMNI type to Rust type annotation string.
pub fn rs_type(kind: &str, type_args: &[TypeIR]) -> String {
    match kind {
        "int" | "i32" => "i32".into(),
        "i64" => "i64".into(),
        "u32" => "u32".into(),
        "u64" => "u64".into(),
        "float" | "f64" => "f64".into(),
        "f32" => "f32".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "String".into(),
        "void" | "()" => "()".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("Vec<{}>", rs_type(&inner.kind, &inner.type_args))
            } else {
                "Vec<()>".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "HashMap<{}, {}>",
                    rs_type(&type_args[0].kind, &type_args[0].type_args),
                    rs_type(&type_args[1].kind, &type_args[1].type_args)
                )
            } else {
                "HashMap<String, ()>".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("Option<{}>", rs_type(&inner.kind, &inner.type_args))
            } else {
                "Option<()>".into()
            }
        }
        other => {
            // If it looks like a user-defined type, use as-is
            if other.chars().next().map_or(false, |c| c.is_uppercase()) {
                other.into()
            } else {
                "()".into()
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Unit tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── rs_type tests ──────────────────────────────────────

    #[test]
    fn test_rs_type_primitives() {
        assert_eq!(rs_type("int", &[]), "i32");
        assert_eq!(rs_type("i32", &[]), "i32");
        assert_eq!(rs_type("i64", &[]), "i64");
        assert_eq!(rs_type("f64", &[]), "f64");
        assert_eq!(rs_type("bool", &[]), "bool");
        assert_eq!(rs_type("string", &[]), "String");
        assert_eq!(rs_type("void", &[]), "()");
    }

    #[test]
    fn test_rs_type_generics() {
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

        assert_eq!(rs_type("Vec", &[i32_arg.clone()]), "Vec<i32>");
        assert_eq!(
            rs_type("HashMap", &[string_arg.clone(), i32_arg.clone()]),
            "HashMap<String, i32>"
        );
    }

    #[test]
    fn test_rs_type_option() {
        let i32_arg = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(rs_type("Option", &[i32_arg]), "Option<i32>");
    }

    #[test]
    fn test_rs_type_user_defined() {
        assert_eq!(rs_type("MyType", &[]), "MyType");
        assert_eq!(rs_type("unknown", &[]), "()");
    }

    // ── format_json_value tests ────────────────────────────

    #[test]
    fn test_format_json_value() {
        assert_eq!(format_json_value(&serde_json::Value::Null), "None");
        assert_eq!(format_json_value(&serde_json::Value::Bool(true)), "true");
        assert_eq!(format_json_value(&serde_json::Value::Bool(false)), "false");
        assert_eq!(format_json_value(&serde_json::json!(42)), "42");
        assert_eq!(
            format_json_value(&serde_json::json!("hello")),
            "\"hello\".to_string()"
        );
        assert_eq!(
            format_json_value(&serde_json::json!("he\"llo")),
            "\"he\\\"llo\".to_string()"
        );
    }

    // ── binary_op_to_rust tests ────────────────────────────

    #[test]
    fn test_binary_op_to_rust() {
        assert_eq!(binary_op_to_rust("add"), "+");
        assert_eq!(binary_op_to_rust("eq"), "==");
        assert_eq!(binary_op_to_rust("and"), "&&");
        assert_eq!(binary_op_to_rust("or"), "||");
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
            left: Box::new(ExprIR::IdentifierExpr {
                name: "a".into(),
            }),
            right: Box::new(ExprIR::IdentifierExpr {
                name: "b".into(),
            }),
        };
        assert_eq!(format_expr_ir(&expr), "a + b");
    }

    #[test]
    fn test_format_expr_ir_ternary() {
        let expr = ExprIR::TernaryIR {
            cond: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
            }),
            then_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            else_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            }),
        };
        assert_eq!(format_expr_ir(&expr), "if x { 1 } else { 0 }");
    }

    #[test]
    fn test_format_expr_ir_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "obj".into(),
            }),
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
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
            }),
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
            name: "do_thing".into(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn do_thing(&self) {"));
    }

    #[test]
    fn test_generate_function_with_return_type() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "get_value".into(),
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
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn get_value(&self) -> i32 {"));
    }

    #[test]
    fn test_generate_function_with_params() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "greet".into(),
            params: vec![
                ParamIR {
                    name: "name".into(),
                    r#type: Some(TypeIR {
                        kind: "string".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                },
            ],
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
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn greet(&self, name: String) -> String {"));
    }

    // ── generate_component tests ──────────────────────────

    #[test]
    fn test_generate_component_empty() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            view: None,
            styles: vec![],
        };
        generate_component(&mut w, "App", &comp);
        let out = w.into_string();
        assert!(out.contains("struct App;"));
    }

    #[test]
    fn test_generate_component_with_function() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "App".into(),
            generic_params: vec![],
            props: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![FunctionIR {
                name: "greet".into(),
                params: vec![],
                return_type: None,
                body: BlockIR {
                    statements: vec![],
                    execution_mode: "client-only".into(),
                },
            }],
            store_refs: vec![],
            refs: vec![],
            view: None,
            styles: vec![],
        };
        generate_component(&mut w, "App", &comp);
        let out = w.into_string();
        assert!(out.contains("struct App;"));
        assert!(out.contains("impl App {"));
        assert!(out.contains("fn greet(&self) {"));
    }

    #[test]
    fn test_generate_component_with_state() {
        let mut w = CodeWriter::new();
        let comp = ComponentIR {
            name: "Counter".into(),
            generic_params: vec![],
            props: HashMap::new(),
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
            view: None,
            styles: vec![],
        };
        generate_component(&mut w, "Counter", &comp);
        let out = w.into_string();
        assert!(out.contains("struct Counter {"));
        assert!(out.contains("count: i32,"));
        assert!(out.contains("impl Counter {"));
        assert!(out.contains("fn new() -> Self {"));
        assert!(out.contains("count: 0,"));
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
        assert!(out.contains("struct AppStore {"));
        assert!(out.contains("count: i32,"));
        assert!(out.contains("name: String,"));
        assert!(out.contains("fn new() -> Self {"));
        assert!(out.contains("count: 0,"));
        assert!(out.contains("name: \"world\".to_string(),"));
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
        assert!(out.contains("struct EmptyStore;"));
        assert!(out.contains("fn new() -> Self"));
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
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_generate_statement_let_with_type() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "name".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            }),
            r#type: Some(TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "let name: String = \"hello\".to_string();\n");
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
    fn test_generate_statement_return_void() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR {
            value: None,
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "return ();\n");
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
        assert_eq!(w.into_string(), "self.count += 1;\n");
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
        assert_eq!(w.into_string(), "self.name = \"hello\".to_string();\n");
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
        assert_eq!(w.into_string(), "self.count -= 1;\n");
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
        assert!(out.contains("if cond {"));
        assert!(out.contains("let x = 1;"));
        assert!(out.contains("} else {"));
        assert!(out.contains("let x = 2;"));
        assert!(out.contains("}"));
    }

    // ── Full pipeline through trait ────────────────────────

    #[test]
    fn test_full_pipeline_through_trait() {
        let codegen = RustCodegen;
        let ir = IRProgram {
            version: "0.1".into(),
            components: HashMap::from([(
                "App".into(),
                ComponentIR {
                    name: "App".into(),
                    generic_params: vec![],
                    props: HashMap::new(),
                    states: vec![],
                    computeds: vec![],
                    effects: vec![],
                    lifecycles: vec![],
                    fns: vec![FunctionIR {
                        name: "run".into(),
                        params: vec![],
                        return_type: None,
                        body: BlockIR {
                            statements: vec![],
                            execution_mode: "client-only".into(),
                        },
                    }],
                    store_refs: vec![],
                    refs: vec![],
                    view: None,
                    styles: vec![],
                },
            )]),
            stores: HashMap::new(),
            source_map: HashMap::new(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].path, "main.rs");
        let content = &output.files[0].content;
        assert!(content.contains("struct App;"));
        assert!(content.contains("impl App {"));
        assert!(content.contains("fn run(&self) {"));
    }

    #[test]
    fn test_full_pipeline_with_store() {
        let codegen = RustCodegen;
        let ir = IRProgram {
            version: "0.1".into(),
            components: HashMap::new(),
            stores: HashMap::from([(
                "AppStore".into(),
                StoreIR {
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
                    ]),
                    signal_defaults: HashMap::from([
                        ("count".into(), serde_json::json!(0)),
                    ]),
                    getters: HashMap::new(),
                    actions: vec![],
                },
            )]),
            source_map: HashMap::new(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        let content = &output.files[0].content;
        assert!(content.contains("struct AppStore {"));
        assert!(content.contains("count: i32,"));
        assert!(content.contains("fn new() -> Self"));
        assert!(content.contains("count: 0,"));
    }
}
