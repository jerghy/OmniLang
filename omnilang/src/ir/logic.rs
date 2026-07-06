//! IR nodes for logic constructs: functions, structs, enums, type aliases, imports, exports.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIR {
    pub path: String,
    pub names: Vec<String>,
    pub alias: Option<String>,
    pub kind: String, // "named" | "namespace" | "type_only" | "lib"
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
    pub functions: HashMap<String, FunctionIR>,
    #[serde(default)]
    pub structs: HashMap<String, StructIR>,
    #[serde(default)]
    pub enums: HashMap<String, EnumIR>,
    #[serde(default)]
    pub type_aliases: HashMap<String, TypeAliasIR>,
    #[serde(default)]
    pub components: HashMap<String, super::ComponentIR>,
    #[serde(default)]
    pub stores: HashMap<String, super::StoreIR>,
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
}
