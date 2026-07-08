//! IR nodes for logic constructs: functions, structs, enums, type aliases, imports, exports.

use super::*;
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;
pub use super::visibility::Visibility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionIR {
    pub name: String,
    pub visibility: Visibility,
    pub params: Vec<ParamIR>,
    pub return_type: Option<TypeIR>,
    pub body: BlockIR,
    #[serde(default)]
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructIR {
    pub name: String,
    pub visibility: Visibility,
    pub fields: Vec<FieldIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldIR {
    pub name: String,
    pub r#type: TypeIR,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumIR {
    pub name: String,
    pub visibility: Visibility,
    pub variants: Vec<VariantIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub payload: Option<TypeIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasIR {
    pub name: String,
    pub visibility: Visibility,
    pub target: TypeIR,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    Named,
    Namespace,
    TypeOnly,
    Lib,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIR {
    pub path: String,
    pub names: Vec<String>,
    pub alias: Option<String>,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportIR {
    pub names: Vec<String>,
    pub path: Option<String>,
}

/// A single module in the IR — corresponds to one .omni source file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleIR {
    pub name: String,
    #[serde(default)]
    pub functions: FxHashMap<String, FunctionIR>,
    #[serde(default)]
    pub structs: FxHashMap<String, StructIR>,
    #[serde(default)]
    pub enums: FxHashMap<String, EnumIR>,
    #[serde(default)]
    pub type_aliases: FxHashMap<String, TypeAliasIR>,
    #[serde(default)]
    pub components: FxHashMap<String, super::ComponentIR>,
    #[serde(default)]
    pub stores: FxHashMap<String, super::StoreIR>,
    #[serde(default)]
    pub themes: Vec<serde_json::Value>,
    #[serde(default)]
    pub imports: Vec<ImportIR>,
    #[serde(default)]
    pub exports: Vec<ExportIR>,
    #[serde(default)]
    pub motions: Vec<super::MotionIntent>,
    #[serde(default)]
    pub timelines: Vec<super::TimelineIntent>,
    #[serde(default)]
    pub designs: Vec<super::DesignIR>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_visibility_public() {
        let v = Visibility::Public;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"public\"");
        let back: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Visibility::Public);
    }

    #[test]
    fn serde_roundtrip_visibility_private() {
        let v = Visibility::Private;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"private\"");
        let back: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Visibility::Private);
    }

    #[test]
    fn serde_roundtrip_import_kind_named() {
        let v = ImportKind::Named;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"named\"");
        let back: ImportKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ImportKind::Named);
    }

    #[test]
    fn serde_roundtrip_import_kind_namespace() {
        let v = ImportKind::Namespace;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"namespace\"");
        let back: ImportKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ImportKind::Namespace);
    }

    #[test]
    fn serde_roundtrip_import_kind_type_only() {
        let v = ImportKind::TypeOnly;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"type_only\"");
        let back: ImportKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ImportKind::TypeOnly);
    }

    #[test]
    fn serde_roundtrip_import_kind_lib() {
        let v = ImportKind::Lib;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"lib\"");
        let back: ImportKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ImportKind::Lib);
    }
}
