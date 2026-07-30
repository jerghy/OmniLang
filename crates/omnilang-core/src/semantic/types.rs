//! Semantic type representation for type inference.
//! Independent from parser TypeAnnotation (AST) and codegen TypeIR.
//! Used internally during semantic analysis resolve pass.

use crate::parser::ast::TypeAnnotation;
use std::fmt;

/// Internal type representation for semantic type checking.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticType {
    Int,
    Float,
    String,
    Bool,
    Void,
    Null,
    /// User-defined named type (struct, type alias)
    Named(String),
    Array(Box<SemanticType>),
    Map(Box<SemanticType>, Box<SemanticType>),
    Optional(Box<SemanticType>),
    Union(Vec<SemanticType>),
    Fn(Vec<SemanticType>, Box<SemanticType>),
    /// Fallback — inference failed or unknown type
    Any,
}

impl SemanticType {
    /// Convert a parser TypeAnnotation into a SemanticType.
    /// Maps built-in type names (i32, String, bool, etc.) to their SemanticType variants.
    /// Composite types (Array<T>, Map<K,V>, Optional<T>, Union, Fn) are recursively converted.
    pub fn from_type_annotation(ta: &TypeAnnotation) -> Self {
        match ta {
            TypeAnnotation::Named {
                name, type_args, ..
            } => match name.as_str() {
                "i8" | "i16" | "i32" | "i64" | "int" | "Int" | "integer" | "isize" | "usize"
                | "u8" | "u16" | "u32" | "u64" => SemanticType::Int,
                "f64" | "f32" | "float" | "Float" | "num" | "number" => SemanticType::Float,
                "String" | "str" | "string" => SemanticType::String,
                "bool" | "Bool" | "boolean" => SemanticType::Bool,
                "void" | "Void" | "()" | "unit" => SemanticType::Void,
                "auto" | "Auto" | "var" | "Var" => SemanticType::Any,
                "null" | "Null" | "nil" | "None" => SemanticType::Null,
                "Array" if type_args.len() == 1 => {
                    SemanticType::Array(Box::new(Self::from_type_annotation(&type_args[0])))
                }
                "Map" if type_args.len() == 2 => SemanticType::Map(
                    Box::new(Self::from_type_annotation(&type_args[0])),
                    Box::new(Self::from_type_annotation(&type_args[1])),
                ),
                _ => SemanticType::Named(name.clone()),
            },
            TypeAnnotation::Optional { base, .. } => {
                SemanticType::Optional(Box::new(Self::from_type_annotation(base)))
            }
            TypeAnnotation::Union { types, .. } => {
                SemanticType::Union(types.iter().map(Self::from_type_annotation).collect())
            }
            TypeAnnotation::Fn {
                params,
                return_type,
                ..
            } => SemanticType::Fn(
                params.iter().map(Self::from_type_annotation).collect(),
                Box::new(Self::from_type_annotation(return_type)),
            ),
        }
    }

    /// Parse a type from a string representation (for Symbol type_ann lookups).
    /// Handles simple built-in names; complex types fall back to Any.
    pub fn parse_type_string(s: &str) -> Self {
        match s {
            "i8" | "i16" | "i32" | "i64" | "int" | "Int" | "integer" | "isize" | "usize" | "u8"
            | "u16" | "u32" | "u64" => SemanticType::Int,
            "f64" | "f32" | "float" | "Float" | "num" | "number" => SemanticType::Float,
            "String" | "str" | "string" => SemanticType::String,
            "bool" | "Bool" | "boolean" => SemanticType::Bool,
            "void" | "Void" | "()" | "unit" => SemanticType::Void,
            "null" | "Null" | "nil" | "None" => SemanticType::Null,
            _ => SemanticType::Named(s.to_string()),
        }
    }
}

impl fmt::Display for SemanticType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticType::Int => write!(f, "Int"),
            SemanticType::Float => write!(f, "Float"),
            SemanticType::String => write!(f, "String"),
            SemanticType::Bool => write!(f, "Bool"),
            SemanticType::Void => write!(f, "Void"),
            SemanticType::Null => write!(f, "Null"),
            SemanticType::Named(name) => write!(f, "{}", name),
            SemanticType::Array(inner) => write!(f, "Array<{}>", inner),
            SemanticType::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            SemanticType::Optional(inner) => write!(f, "{}?", inner),
            SemanticType::Union(types) => {
                let names: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", names.join(" | "))
            }
            SemanticType::Fn(params, ret) => {
                let p: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                write!(f, "fn({}) -> {}", p.join(", "), ret)
            }
            SemanticType::Any => write!(f, "<unknown>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_int() {
        assert_eq!(SemanticType::Int.to_string(), "Int");
    }

    #[test]
    fn display_optional_string() {
        let t = SemanticType::Optional(Box::new(SemanticType::String));
        assert_eq!(t.to_string(), "String?");
    }

    #[test]
    fn display_array_int() {
        let t = SemanticType::Array(Box::new(SemanticType::Int));
        assert_eq!(t.to_string(), "Array<Int>");
    }

    #[test]
    fn display_union() {
        let t = SemanticType::Union(vec![SemanticType::Int, SemanticType::String]);
        assert_eq!(t.to_string(), "Int | String");
    }

    #[test]
    fn parse_type_string_i32() {
        assert_eq!(SemanticType::parse_type_string("i32"), SemanticType::Int);
    }

    #[test]
    fn parse_type_string_unknown() {
        assert_eq!(
            SemanticType::parse_type_string("User"),
            SemanticType::Named("User".to_string())
        );
    }
}
