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

use crate::ir::*;
use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};

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
        StatementIR::LetStmtIR { name, value, r#type } => {
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
    use crate::ir::ModuleIR;

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
        assert!(out.contains("def do_thing(self):"));
        assert!(out.contains("pass"));
    }

    #[test]
    fn test_generate_standalone_function() {
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
        };
        generate_function(&mut w, &func, true);
        let out = w.into_string();
        assert!(out.contains("def main():"));
        assert!(out.contains("pass"));
        // Standalone functions must NOT have self
        assert!(!out.contains("self"));
    }

    #[test]
    fn test_generate_standalone_function_with_params() {
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
        };
        generate_function(&mut w, &func, true);
        let out = w.into_string();
        assert!(out.contains("def add(a: int) -> int:"));
        assert!(!out.contains("self"));
    }

    #[test]
    fn test_generate_struct() {
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
        assert!(out.contains("@dataclass"));
        assert!(out.contains("class Point:"));
        assert!(out.contains("x: int"));
        assert!(out.contains("y: int"));
    }

    #[test]
    fn test_generate_enum() {
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
        assert!(out.contains("class Color(Enum):"));
        assert!(out.contains("Red = 0"));
        assert!(out.contains("Green = 1"));
        assert!(out.contains("Blue = 2"));
    }

    #[test]
    fn test_generate_type_alias() {
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
        let out = w.into_string();
        assert_eq!(out, "UserId = int\n");
    }

    #[test]
    fn test_generate_module_imports_structs() {
        let mut w = CodeWriter::new();
        let mut structs = HashMap::new();
        structs.insert(
            "Point".into(),
            StructIR {
                name: "Point".into(),
                visibility: "public".to_string(),
                fields: vec![],
            },
        );
        let module = ModuleIR {
            structs,
            ..Default::default()
        };
        generate_module_imports(&mut w, &module);
        let out = w.into_string();
        assert!(out.contains("from dataclasses import dataclass"));
    }

    #[test]
    fn test_generate_module_imports_enums() {
        let mut w = CodeWriter::new();
        let mut enums = HashMap::new();
        enums.insert(
            "Color".into(),
            EnumIR {
                name: "Color".into(),
                visibility: "public".to_string(),
                variants: vec![],
            },
        );
        let module = ModuleIR {
            enums,
            ..Default::default()
        };
        generate_module_imports(&mut w, &module);
        let out = w.into_string();
        assert!(out.contains("from enum import Enum"));
    }

    #[test]
    fn test_generate_empty_struct() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Empty".into(),
            visibility: "public".to_string(),
            fields: vec![],
        };
        generate_struct(&mut w, &struct_ir);
        let out = w.into_string();
        assert!(out.contains("@dataclass"));
        assert!(out.contains("class Empty:"));
        assert!(out.contains("pass"));
    }

    #[test]
    fn test_generate_empty_enum() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "EmptyEnum".into(),
            visibility: "public".to_string(),
            variants: vec![],
        };
        generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("class EmptyEnum(Enum):"));
        assert!(out.contains("pass"));
    }

    #[test]
    fn test_format_type_ir() {
        assert_eq!(format_type_ir(&TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }), "int");
        assert_eq!(format_type_ir(&TypeIR { kind: "f64".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }), "float");
        assert_eq!(format_type_ir(&TypeIR { kind: "string".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }), "str");
        assert_eq!(format_type_ir(&TypeIR { kind: "bool".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false }), "bool");
    }

    #[test]
    fn test_format_type_ir_vec() {
        let inner = TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false };
        assert_eq!(format_type_ir(&TypeIR { kind: "Vec".into(), type_args: vec![inner], is_nullable: false, needs_clone: false, needs_partial_eq: false }), "list[int]");
    }

    #[test]
    fn test_format_type_ir_option() {
        let inner = TypeIR { kind: "i32".into(), type_args: vec![], is_nullable: false, needs_clone: false, needs_partial_eq: false };
        assert_eq!(format_type_ir(&TypeIR { kind: "Option".into(), type_args: vec![inner], is_nullable: false, needs_clone: false, needs_partial_eq: false }), "Optional[int]");
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
        let mut components = HashMap::new();
        components.insert(
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
        );
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components,
                    ..Default::default()
                },
            )]),
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
