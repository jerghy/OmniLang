//! AST -> IR lowering for logic constructs (functions, structs, enums, type aliases,
//! imports, exports).

use super::*;
use crate::parser::ast::*;

pub fn lower_function(func: &FunctionDef) -> FunctionIR {
    FunctionIR {
        name: func.name.clone(),
        visibility: match func.visibility {
            Visibility::Public => "public".into(),
            Visibility::Private => "private".into(),
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
        body: lower_block(&func.body),
    }
}

pub fn lower_struct(s: &StructDef) -> StructIR {
    StructIR {
        name: s.name.clone(),
        visibility: match s.visibility {
            Visibility::Public => "public".into(),
            Visibility::Private => "private".into(),
        },
        fields: s
            .fields
            .iter()
            .map(|f| FieldIR {
                name: f.name.clone(),
                r#type: lower_type(&f.type_ann),
                visibility: "public".into(),
            })
            .collect(),
    }
}

pub fn lower_enum(tu: &TaggedUnionDef) -> EnumIR {
    EnumIR {
        name: tu.name.clone(),
        visibility: "public".into(),
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
            Visibility::Public => "public".into(),
            Visibility::Private => "private".into(),
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
            kind: "named".into(),
        },
        ImportStmt::Namespace(ns) => ImportIR {
            path: ns.path.clone(),
            names: vec![],
            alias: Some(ns.alias.clone()),
            kind: "namespace".into(),
        },
        ImportStmt::TypeOnly(t) => ImportIR {
            path: t.path.clone(),
            names: t.names.clone(),
            alias: None,
            kind: "type_only".into(),
        },
        ImportStmt::Lib(name) => ImportIR {
            path: name.clone(),
            names: vec![],
            alias: None,
            kind: "lib".into(),
        },
    }
}

pub fn lower_export(exp: &ExportFrom) -> ExportIR {
    ExportIR {
        names: exp.names.clone(),
        path: Some(exp.path.clone()),
    }
}
