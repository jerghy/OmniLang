//! Python codegen: IR -> Python.
//!
//! Generates Python source files from Intent IR. Each component becomes
//! a Python class, each function becomes a method, and stores become
//! standalone classes.

use std::collections::HashMap;

use crate::ir::*;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};

pub struct PythonCodegen;

impl Codegen for PythonCodegen {
    fn generate(&self, ir: &IRProgram, source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (source_path, _source_text) in source_files {
            let mut w = CodeWriter::new();
            w.emit(&format!("# Generated from {}", source_path));
            w.emit("");

            // Generate stores (standalone classes, no framework)
            for (store_name, store) in &ir.stores {
                generate_store(&mut w, store_name, store);
                w.emit("");
            }

            // Generate components as Python classes
            for (comp_name, comp) in &ir.components {
                generate_component(&mut w, comp_name, comp);
                w.emit("");
            }

            let out_path = source_path.replace(".omni", ".py");
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
        generate_function(w, func);
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

fn generate_function(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list
    let mut param_strs = vec!["self".to_string()];
    for p in &func.params {
        let mut s = p.name.clone();
        // Optionally include type annotation
        if let Some(ref ty) = p.r#type {
            s.push_str(&format!(": {}", py_type(&ty.kind, &ty.type_args)));
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", py_type(&rt.kind, &rt.type_args)))
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
        StatementIR::LetStmtIR { name, value, r#type } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "None".into());
            let type_hint = r#type
                .as_ref()
                .map(|t| format!(": {}", py_type(&t.kind, &t.type_args)))
                .unwrap_or_default();
            w.emit(&format!("{}{} = {}", name, type_hint, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_body,
        } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("if {}:", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s);
            }
            w.dedent();
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
                        w.emit(&format!("# TODO: mutation operation '{}' on {}", other, target));
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
            // Escape single quotes (use double quotes for Python strings)
            let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
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
                format!("'{}'", other)  // forward reference
            } else {
                "Any".into()
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

    #[test]
    fn test_py_type_primitives() {
        assert_eq!(py_type("int", &[]), "int");
        assert_eq!(py_type("f64", &[]), "float");
        assert_eq!(py_type("bool", &[]), "bool");
        assert_eq!(py_type("string", &[]), "str");
        assert_eq!(py_type("void", &[]), "None");
    }

    #[test]
    fn test_py_type_generics() {
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

        assert_eq!(py_type("Vec", &[i32_arg.clone()]), "list[int]");
        assert_eq!(
            py_type("HashMap", &[string_arg.clone(), i32_arg.clone()]),
            "dict[str, int]"
        );
    }

    #[test]
    fn test_py_type_option() {
        let i32_arg = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        };
        assert_eq!(py_type("Option", &[i32_arg]), "Optional[int]");
    }

    #[test]
    fn test_py_type_user_defined() {
        assert_eq!(py_type("MyType", &[]), "'MyType'");
    }

    #[test]
    fn test_format_json_value() {
        assert_eq!(format_json_value(&serde_json::Value::Null), "None");
        assert_eq!(format_json_value(&serde_json::Value::Bool(true)), "True");
        assert_eq!(format_json_value(&serde_json::Value::Bool(false)), "False");
        assert_eq!(
            format_json_value(&serde_json::json!(42)),
            "42"
        );
        assert_eq!(
            format_json_value(&serde_json::json!("hello")),
            "'hello'"
        );
    }

    #[test]
    fn test_binary_ops() {
        assert_eq!(binary_op_to_python("add"), "+");
        assert_eq!(binary_op_to_python("eq"), "==");
        assert_eq!(binary_op_to_python("and"), "and");
    }

    #[test]
    fn test_format_expr_literal() {
        let expr = ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        };
        assert_eq!(format_expr_ir(&expr), "42");
    }

    #[test]
    fn test_format_expr_identifier() {
        let expr = ExprIR::IdentifierExpr {
            name: "count".into(),
        };
        assert_eq!(format_expr_ir(&expr), "count");
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
        assert_eq!(format_expr_ir(&expr), "a + b");
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
        assert_eq!(format_expr_ir(&expr), "1 if x else 0");
    }

    #[test]
    fn test_format_expr_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "obj".into(),
            }),
            field: "prop".into(),
        };
        assert_eq!(format_expr_ir(&expr), "obj.prop");
    }

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
        assert!(out.contains("def do_thing(self):"));
        assert!(out.contains("pass"));
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
        assert!(out.contains("class App:"));
        assert!(out.contains("def greet(self):"));
        assert!(out.contains("pass"));
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
        assert!(out.contains("class Counter:"));
        assert!(out.contains("def __init__(self):"));
        assert!(out.contains("self.count = 0"));
    }

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
        assert!(out.contains("class AppStore:"));
        assert!(out.contains("def __init__(self):"));
        assert!(out.contains("self.count = 0"));
        assert!(out.contains("self.name = 'world'"));
    }

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
        assert_eq!(w.into_string(), "x = 42\n");
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
        assert_eq!(w.into_string(), "return result\n");
    }

    #[test]
    fn test_generate_statement_mutation() {
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
        assert_eq!(w.into_string(), "count += 1\n");
    }

    #[test]
    fn test_full_pipeline_through_trait() {
        let codegen = PythonCodegen;
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
        assert_eq!(output.files[0].path, "main.py");
        let content = &output.files[0].content;
        assert!(content.contains("class App:"));
        assert!(content.contains("def run(self):"));
    }
}
