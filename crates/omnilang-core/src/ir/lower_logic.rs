//! AST -> IR lowering for logic constructs (functions, structs, enums, type aliases,
//! imports, exports).

use super::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;

pub fn lower_function(
    func: &FunctionDef,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> FunctionIR {
    FunctionIR {
        name: func.name.clone(),
        visibility: func.visibility,
        params: func
            .params
            .iter()
            .map(|p| ParamIR {
                name: p.name.clone(),
                r#type: p.type_ann.as_ref().map(lower_type),
            })
            .collect(),
        return_type: func.return_type.as_ref().map(lower_type),
        body: lower_block(&func.body, variant_map, None, semantics),
        is_async: func.is_async,
        source_span: None,
    }
}

pub fn lower_struct(s: &StructDef) -> StructIR {
    StructIR {
        name: s.name.clone(),
        visibility: s.visibility,
        fields: s
            .fields
            .iter()
            .map(|f| FieldIR {
                name: f.name.clone(),
                r#type: lower_type(&f.type_ann),
                visibility: Visibility::Public,
                source_span: None,
            })
            .collect(),
        source_span: None,
    }
}

pub fn lower_enum(tu: &TaggedUnionDef) -> EnumIR {
    EnumIR {
        name: tu.name.clone(),
        visibility: Visibility::Public,
        variants: tu
            .variants
            .iter()
            .map(|v| VariantIR {
                name: v.name.clone(),
                fields: v
                    .fields
                    .iter()
                    .map(|f| FieldIR {
                        name: f.name.clone(),
                        r#type: lower_type(&f.type_ann),
                        visibility: Visibility::Public,
                        source_span: None,
                    })
                    .collect(),
                source_span: None,
            })
            .collect(),
        source_span: None,
    }
}

pub fn lower_type_alias(ta: &TypeAlias) -> TypeAliasIR {
    TypeAliasIR {
        name: ta.name.clone(),
        visibility: ta.visibility,
        target: lower_type(&ta.type_ann),
        source_span: None,
    }
}

pub fn lower_import(imp: &ImportStmt) -> ImportIR {
    match imp {
        ImportStmt::Named(n) => ImportIR {
            path: n.path.clone(),
            names: n.names.clone(),
            alias: None,
            kind: ImportKind::Named,
            source_span: None,
        },
        ImportStmt::Namespace(ns) => ImportIR {
            path: ns.path.clone(),
            names: vec![],
            alias: Some(ns.alias.clone()),
            kind: ImportKind::Namespace,
            source_span: None,
        },
        ImportStmt::TypeOnly(t) => ImportIR {
            path: t.path.clone(),
            names: t.names.clone(),
            alias: None,
            kind: ImportKind::TypeOnly,
            source_span: None,
        },
        ImportStmt::Lib(name) => ImportIR {
            path: name.clone(),
            names: vec![],
            alias: None,
            kind: ImportKind::Lib,
            source_span: None,
        },
    }
}

pub fn lower_export(exp: &ExportFrom) -> ExportIR {
    ExportIR {
        names: exp.names.clone(),
        path: Some(exp.path.clone()),
        source_span: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::tokenize;
    use crate::parser;
    use rustc_hash::FxHashMap;
    #[test]
    fn test_lower_fn_main_to_ir() {
        let source = "fn main() { return 42; }";
        let tokens = tokenize(source);
        let (program, _) = parser::parse(tokens, source);
        let mut semantics = crate::semantic::SemanticResult {
            symbols: crate::semantic::symbols::SymbolTable::new(),
            dependencies: FxHashMap::default(),
            diagnostics: Vec::new(),
            event_param_name: None,
        };
        let ir = crate::ir::lower(&program, &mut semantics);
        let main_module = ir.modules.get("input.omni").unwrap();
        assert!(main_module.functions.contains_key("main"));
        let main_fn = &main_module.functions["main"];
        assert_eq!(main_fn.name, "main");
        assert_eq!(main_fn.visibility, crate::ir::logic::Visibility::Private);
    }

    #[test]
    fn test_lower_struct_to_ir() {
        let source = "struct Point { x: i32, y: i32 }";
        let tokens = tokenize(source);
        let (program, _) = parser::parse(tokens, source);
        let mut semantics = crate::semantic::SemanticResult {
            symbols: crate::semantic::symbols::SymbolTable::new(),
            dependencies: FxHashMap::default(),
            diagnostics: Vec::new(),
            event_param_name: None,
        };
        let ir = crate::ir::lower(&program, &mut semantics);
        let main_module = ir.modules.get("input.omni").unwrap();
        assert!(main_module.structs.contains_key("Point"));
    }
}
