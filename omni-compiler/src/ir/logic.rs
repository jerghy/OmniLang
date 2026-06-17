//! IR nodes for logic constructs: functions, structs, enums, type aliases, imports, exports.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionIR {
    pub name: String,
    pub visibility: String,  // "public" | "private"
    pub params: Vec<ParamIR>,
    pub return_type: Option<TypeIR>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructIR {
    pub name: String,
    pub visibility: String,
    pub fields: Vec<FieldIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldIR {
    pub name: String,
    pub r#type: TypeIR,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumIR {
    pub name: String,
    pub visibility: String,
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
    pub visibility: String,
    pub target: TypeIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIR {
    pub path: String,
    pub names: Vec<String>,
    pub alias: Option<String>,
    pub kind: String,  // "named" | "namespace" | "type_only" | "lib"
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
}
