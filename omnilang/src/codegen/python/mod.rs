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

pub mod expr;
pub mod stmt;

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
            for alias in module.type_aliases.values() {
                generate_type_alias(&mut w, alias);
            }

            // Separate logic constructs from components/stores with a blank line
            if !module.type_aliases.is_empty() {
                w.emit("");
            }

            // Generate structs as @dataclass classes
            for struct_ir in module.structs.values() {
                generate_struct(&mut w, struct_ir);
                w.emit("");
            }

            // Generate enums as Enum classes
            for enum_ir in module.enums.values() {
                generate_enum(&mut w, enum_ir);
                w.emit("");
            }

            // Generate standalone functions
            let mut has_main = false;
            for (fname, func) in &module.functions {
                stmt::generate_function(&mut w, func, true);
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
                stmt::generate_store(&mut w, store_name, store);
                w.emit("");
            }

            // Generate components from this module
            for (comp_name, comp) in &module.components {
                stmt::generate_component(&mut w, comp_name, comp);
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
    let target = expr::format_type_ir(&alias.target);
    w.emit(&format!("{} = {}", alias.name, target));
}

/// Generate a struct as a `@dataclass` class.
fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR) {
    w.emit("@dataclass");
    w.emit(&format!("class {}:", struct_ir.name));
    w.indent();
    for field in &struct_ir.fields {
        let type_str = expr::format_type_ir(&field.r#type);
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
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                format!("'{}'", other) // forward reference
            } else {
                "Any".into()
            }
        }
    }
}
