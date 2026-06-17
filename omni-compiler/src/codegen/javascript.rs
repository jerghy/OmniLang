//! JavaScript/TypeScript codegen: IR -> JS/TS.
//!
//! Generates JS/TS source files from Intent IR. Each component becomes
//! a class, each function becomes a method, and stores become standalone
//! classes. In JS mode (ts_mode: false), no type annotations are emitted.
//! In TS mode (ts_mode: true), type annotations are added.

use std::collections::HashMap;

use crate::ir::*;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};

pub struct JavaScriptCodegen {
    pub ts_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(ts_mode: bool) -> Self {
        Self { ts_mode }
    }
}

impl Codegen for JavaScriptCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        let ext = if self.ts_mode { ".ts" } else { ".js" };

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Generate stores as standalone classes
            for (store_name, store) in ir.all_stores() {
                generate_store(&mut w, store_name, store, self.ts_mode);
                w.emit("");
            }

            // Generate components as classes
            for (comp_name, comp) in ir.all_components() {
                generate_component(&mut w, comp_name, comp, self.ts_mode);
                w.emit("");
            }

            let out_path = source_path.replace(".omni", ext);
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
            if ann.is_empty() { String::new() } else { format!(": {}", ann) }
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
        for (getter_name, _getter_expr) in &store.getters {
            // Infer type for getter — use `any` for TS mode
            w.emit(&format!("get {}(): any;", getter_name));
        }
    }

    // Constructor with signal defaults
    w.emit("constructor() {");
    w.indent();
    for (signal_name, _signal_type) in &store.signals {
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
        let mut s = p.name.clone();
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
    w.emit(&format!("{}({}){} {{", func.name, all_params, ret_ann));
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
        StatementIR::LetStmtIR { name, value, r#type } => {
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
                        if ann.is_empty() { String::new() } else { format!(": {}", ann) }
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            w.emit(&format!("{} {}{} = {};", kw, name, type_hint, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_body,
        } => {
            let cond = format_expr_ir(condition, ts_mode);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s, ts_mode);
            }
            w.dedent();
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
        StatementIR::MutationStmt { intent } => {
            if let Some(mut_intent) = intent {
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
                    other => {
                        w.emit(&format!("// TODO: mutation operation '{}' on {}", other, target));
                    }
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
                let formatted: Vec<String> = args.iter().map(|a| format_expr_ir(a, ts_mode)).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

fn format_expr_ir(expr: &ExprIR, ts_mode: bool) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object, ts_mode), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let js_op = binary_op_to_js(op);
            let lhs = format_expr_ir(left, ts_mode);
            let rhs = format_expr_ir(right, ts_mode);
            format!("{} {} {}", lhs, js_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let js_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", js_op, format_expr_ir(operand, ts_mode))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond, ts_mode);
            let t = format_expr_ir(then_expr, ts_mode);
            let e = format_expr_ir(else_expr, ts_mode);
            format!("{} ? {} : {}", c, t, e)
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(|a| format_expr_ir(a, ts_mode)).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
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
            let escaped = s.replace('\\', "\\\\")
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
            if other.chars().next().map_or(false, |c| c.is_uppercase()) {
                other.into()
            } else {
                "any".into()
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

        assert_eq!(js_type("Vec", &[i32_arg.clone()], true), "number[]");
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
            left: Box::new(ExprIR::IdentifierExpr {
                name: "a".into(),
            }),
            right: Box::new(ExprIR::IdentifierExpr {
                name: "b".into(),
            }),
        };
        assert_eq!(format_expr_ir(&expr, false), "a + b");
    }

    #[test]
    fn test_format_expr_ternary() {
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
        assert_eq!(format_expr_ir(&expr, false), "x ? 1 : 0");
    }

    #[test]
    fn test_format_expr_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "obj".into(),
            }),
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
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
            }),
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
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
        };
        generate_function(&mut w, &func, false);
        let out = w.into_string();
        assert!(out.contains("greet(name) {"));
        assert!(!out.contains(": string"));
    }

    // ── generate_component tests ──────────────────────────

    #[test]
    fn test_js_empty_component() {
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
            }],
            store_refs: vec![],
            refs: vec![],
            view: None,
            styles: vec![],
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
                            }],
                            store_refs: vec![],
                            refs: vec![],
                            view: None,
                            styles: vec![],
                        },
                    )]),
                    stores: HashMap::new(),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
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
                            }],
                            store_refs: vec![],
                            refs: vec![],
                            view: None,
                            styles: vec![],
                        },
                    )]),
                    stores: HashMap::new(),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
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
}
