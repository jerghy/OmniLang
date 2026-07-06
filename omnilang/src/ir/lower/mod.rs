//! AST → IR lowering: walks the parsed AST and produces IR types.
//!
//! This module is the entry point for the lowering pipeline.
//! Sub-modules handle specific domains: components, views, expressions,
//! statements, and motion/animation definitions.

mod component;
mod expr;
mod motion;
mod stmt;
mod view;

pub(crate) use component::*;
#[allow(unused_imports)]
pub(super) use motion::*;
pub(crate) use stmt::*;

use super::logic::*;
use super::types::*;
use super::types_motion::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use std::collections::HashMap;

/// Lower AST -> IR.
pub fn lower(program: &Program, _semantics: &SemanticResult) -> IRProgram {
    let mut modules = HashMap::new();

    // Build variant → union name lookup for tagged union construction
    let mut variant_map: HashMap<String, String> = HashMap::new();
    for module in program.modules.values() {
        for tu in &module.tagged_unions {
            for variant in &tu.variants {
                variant_map.insert(variant.name.clone(), tu.name.clone());
            }
        }
    }

    for (module_name, module) in &program.modules {
        let mut components = HashMap::new();
        let mut stores = HashMap::new();
        let mut functions = HashMap::new();
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        let mut type_aliases = HashMap::new();
        let mut imports: Vec<ImportIR> = Vec::new();
        let mut exports: Vec<ExportIR> = Vec::new();

        for comp in &module.components {
            let ir = lower_component(comp, &variant_map);
            components.insert(comp.name.clone(), ir);
        }

        for store in &module.stores {
            let ir = lower_store(store, &variant_map);
            stores.insert(store.name.clone(), ir);
        }

        for func in &module.functions {
            let ir = super::lower_logic::lower_function(func, &variant_map);
            functions.insert(func.name.clone(), ir);
        }

        for s in &module.structs {
            let ir = super::lower_logic::lower_struct(s);
            structs.insert(s.name.clone(), ir);
        }

        for tu in &module.tagged_unions {
            let ir = super::lower_logic::lower_enum(tu);
            enums.insert(tu.name.clone(), ir);
        }

        for ta in &module.type_aliases {
            let ir = super::lower_logic::lower_type_alias(ta);
            type_aliases.insert(ta.name.clone(), ir);
        }

        for imp in &module.imports {
            imports.push(super::lower_logic::lower_import(imp));
        }

        for exp in &module.exports {
            exports.push(super::lower_logic::lower_export(exp));
        }

        let motions: Vec<MotionIntent> = module
            .motions
            .iter()
            .map(|m| lower_motion_def(m, &variant_map))
            .collect();

        let timelines: Vec<TimelineIntent> = module
            .timelines
            .iter()
            .map(|t| lower_timeline_def(t, &variant_map))
            .collect();

        let module_ir = ModuleIR {
            name: module_name.clone(),
            functions,
            structs,
            enums,
            type_aliases,
            components,
            stores,
            themes: Vec::new(),
            imports,
            exports,
            motions,
            timelines,
        };

        modules.insert(module_name.clone(), module_ir);
    }

    // Collect stdlib imports: any "import { ... } from \"std/<module>\""
    let mut stdlib_imports: Vec<String> = Vec::new();
    for module in program.modules.values() {
        for imp in &module.imports {
            if let ImportStmt::Named(ni) = imp {
                if ni.path.starts_with("std/") {
                    let module_name = ni
                        .path
                        .strip_prefix("std/")
                        .expect("path starts with \"std/\" as checked above")
                        .to_string();
                    if !stdlib_imports.contains(&module_name) {
                        stdlib_imports.push(module_name);
                    }
                }
            }
        }
    }

    IRProgram {
        version: "1.0".to_string(),
        modules,
        source_map: HashMap::new(),
        stdlib_imports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::analyze;

    fn lower_source(source: &str) -> IRProgram {
        let tokens = tokenize(source);
        let program = parse(&tokens, source).expect("parse should succeed");
        let semantics = analyze(&program);
        lower(&program, &semantics)
    }

    #[test]
    fn lower_simple_function() {
        let ir = lower_source("fn main() -> i32 { return 42; }");
        assert_eq!(ir.version, "1.0");
        let module = ir.modules.get("input.omni").expect("should have input.omni module");
        assert!(
            module.functions.contains_key("main"),
            "should contain 'main' function, got keys: {:?}",
            module.functions.keys()
        );
    }

    #[test]
    fn lower_struct_and_function() {
        let source = "struct Point { x: i32, y: i32 }
                       fn origin() -> Point { return Point { x: 0, y: 0 }; }";
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");
        assert!(
            module.structs.contains_key("Point"),
            "should contain 'Point' struct"
        );
        assert!(
            module.functions.contains_key("origin"),
            "should contain 'origin' function"
        );
    }

    #[test]
    fn lower_import_std() {
        let source = "import { sqrt } from \"std/math\"; fn main() { print(sqrt(16)); }";
        let ir = lower_source(source);
        assert!(
            !ir.stdlib_imports.is_empty(),
            "should detect stdlib imports"
        );
        assert!(
            ir.stdlib_imports.contains(&"math".to_string()),
            "should contain 'math' stdlib import"
        );
    }
}
