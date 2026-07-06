//! AST -> IR lowering for logic constructs (functions, structs, enums, type aliases,
//! imports, exports).

use super::*;
use crate::parser::ast::*;

// Disambiguate Visibility between parser AST and IR
use crate::ir::logic::Visibility as IRVisibility;
use crate::parser::ast::Visibility as ParserVisibility;

pub fn lower_function(func: &FunctionDef, variant_map: &HashMap<String, String>) -> FunctionIR {
    FunctionIR {
        name: func.name.clone(),
        visibility: match func.visibility {
            ParserVisibility::Public => IRVisibility::Public,
            ParserVisibility::Private => IRVisibility::Private,
        },
        params: func
            .params
            .iter()
            .map(|p| ParamIR {
                name: p.name.clone(),
                r#type: p.type_ann.as_ref().map(lower_type),
            })
            .collect(),
        return_type: func.return_type.as_ref().map(lower_type),
        body: lower_block(&func.body, variant_map, None),
        is_async: func.is_async,
    }
}

pub fn lower_struct(s: &StructDef) -> StructIR {
    StructIR {
        name: s.name.clone(),
        visibility: match s.visibility {
            ParserVisibility::Public => IRVisibility::Public,
            ParserVisibility::Private => IRVisibility::Private,
        },
        fields: s
            .fields
            .iter()
            .map(|f| FieldIR {
                name: f.name.clone(),
                r#type: lower_type(&f.type_ann),
                visibility: IRVisibility::Public,
            })
            .collect(),
    }
}

pub fn lower_enum(tu: &TaggedUnionDef) -> EnumIR {
    EnumIR {
        name: tu.name.clone(),
        visibility: IRVisibility::Public,
        variants: tu
            .variants
            .iter()
            .map(|v| VariantIR {
                name: v.name.clone(),
                payload: v.fields.first().map(|f| lower_type(&f.type_ann)),
            })
            .collect(),
    }
}

pub fn lower_type_alias(ta: &TypeAlias) -> TypeAliasIR {
    TypeAliasIR {
        name: ta.name.clone(),
        visibility: match ta.visibility {
            ParserVisibility::Public => IRVisibility::Public,
            ParserVisibility::Private => IRVisibility::Private,
        },
        target: lower_type(&ta.type_ann),
    }
}

pub fn lower_import(imp: &ImportStmt) -> ImportIR {
    match imp {
        ImportStmt::Named(n) => ImportIR {
            path: n.path.clone(),
            names: n.names.clone(),
            alias: None,
            kind: ImportKind::Named,
        },
        ImportStmt::Namespace(ns) => ImportIR {
            path: ns.path.clone(),
            names: vec![],
            alias: Some(ns.alias.clone()),
            kind: ImportKind::Namespace,
        },
        ImportStmt::TypeOnly(t) => ImportIR {
            path: t.path.clone(),
            names: t.names.clone(),
            alias: None,
            kind: ImportKind::TypeOnly,
        },
        ImportStmt::Lib(name) => ImportIR {
            path: name.clone(),
            names: vec![],
            alias: None,
            kind: ImportKind::Lib,
        },
    }
}

pub fn lower_export(exp: &ExportFrom) -> ExportIR {
    ExportIR {
        names: exp.names.clone(),
        path: Some(exp.path.clone()),
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::tokenize;
    use crate::parser;
    use std::collections::HashMap;

    #[test]
    fn test_lower_fn_main_to_ir() {
        let source = "fn main() { return 42; }";
        let tokens = tokenize(source);
        let program = parser::parse(&tokens, source).unwrap();
        let semantics = crate::semantic::SemanticResult {
            symbols: crate::semantic::symbols::SymbolTable::new(),
            dependencies: HashMap::new(),
            diagnostics: Vec::new(),
        };
        let ir = crate::ir::lower(&program, &semantics);
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
        let program = parser::parse(&tokens, source).unwrap();
        let semantics = crate::semantic::SemanticResult {
            symbols: crate::semantic::symbols::SymbolTable::new(),
            dependencies: HashMap::new(),
            diagnostics: Vec::new(),
        };
        let ir = crate::ir::lower(&program, &semantics);
        let main_module = ir.modules.get("input.omni").unwrap();
        assert!(main_module.structs.contains_key("Point"));
    }
}
