//! FormatType trait — eliminates type-mapping duplication across 8 backends.
//!
//! Provides a single `format_type` dispatch method with four concrete family
//! implementations:
//!
//! | Struct | Family | Key characteristics |
//! |--------|--------|-------------------|
//! | `JsTypeFormatter` | JS (React, Vue, JavaScript) | `number`, `boolean`, `string`, `void` |
//! | `DartTypeFormatter` | Dart (Dart, Flutter) | `int`, `double`, `String`, `void` |
//! | `RustTypeFormatter` | Rust (Rust, Dioxus) | `i32`, `f64`, `String`, `()` |
//! | `PythonTypeFormatter` | Python | `int`, `float`, `str`, `None` |

#[cfg_attr(
    not(any(
        feature = "codegen-javascript",
        feature = "codegen-react",
        feature = "codegen-vue",
        feature = "codegen-dart",
        feature = "codegen-flutter",
        feature = "codegen-rust",
        feature = "codegen-dioxus",
        feature = "codegen-python"
    )),
    allow(unused_imports)
)]
use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Trait definition
// ═══════════════════════════════════════════════════════════

/// Trait for mapping OmniLang types to a target language type representation.
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
pub(crate) trait FormatType {
    /// Map an OmniLang `kind` + `type_args` to the target language type string.
    fn format_type(&self, kind: &str, type_args: &[TypeIR]) -> String;
}

// ═══════════════════════════════════════════════════════════
// JS family: JavaScript, TypeScript, React, Vue, OmniView
// ═══════════════════════════════════════════════════════════

/// JS-family type formatter.
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue"
))]
pub(crate) struct JsTypeFormatter;

#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue"
))]
impl FormatType for JsTypeFormatter {
    fn format_type(&self, kind: &str, type_args: &[TypeIR]) -> String {
        match kind {
            "int" | "i32" | "i64" | "u32" | "u64" | "float" | "f64" | "f32" => "number".into(),
            "bool" => "boolean".into(),
            "string" | "String" | "str" => "string".into(),
            "void" | "()" => "void".into(),
            "Vec" | "Array" | "List" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "{}[]",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "any[]".into()
                }
            }
            "FxHashMap" | "HashMap" | "Map" | "Dict" => {
                if type_args.len() >= 2 {
                    format!(
                        "Record<{}, {}>",
                        self.format_type(type_args[0].kind.as_str(), &type_args[0].type_args),
                        self.format_type(type_args[1].kind.as_str(), &type_args[1].type_args)
                    )
                } else {
                    "Record<string, any>".into()
                }
            }
            "Option" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "{} | null",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "any | null".into()
                }
            }
            other => {
                // If it looks like a user-defined type (starts with uppercase), use as-is
                if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                    other.into()
                } else {
                    "any".into()
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Dart family: Dart, Flutter
// ═══════════════════════════════════════════════════════════

/// Dart-family type formatter.
#[cfg(any(feature = "codegen-dart", feature = "codegen-flutter"))]
pub(crate) struct DartTypeFormatter;

#[cfg(any(feature = "codegen-dart", feature = "codegen-flutter"))]
impl FormatType for DartTypeFormatter {
    fn format_type(&self, kind: &str, type_args: &[TypeIR]) -> String {
        match kind {
            "int" | "i32" | "i64" | "u32" | "u64" => "int".into(),
            "float" | "f64" | "f32" => "double".into(),
            "bool" => "bool".into(),
            "string" | "String" | "str" => "String".into(),
            "void" | "()" => "void".into(),
            "Vec" | "Array" | "List" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "List<{}>",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "List".into()
                }
            }
            "FxHashMap" | "HashMap" | "Map" | "Dict" => {
                if type_args.len() >= 2 {
                    format!(
                        "Map<{}, {}>",
                        self.format_type(type_args[0].kind.as_str(), &type_args[0].type_args),
                        self.format_type(type_args[1].kind.as_str(), &type_args[1].type_args)
                    )
                } else {
                    "Map".into()
                }
            }
            "Option" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "{}?",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "Object?".into()
                }
            }
            other => {
                // If it looks like a user-defined type (uppercase), use as-is
                if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                    other.into()
                } else {
                    "dynamic".into()
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Rust family: Rust, Dioxus
// ═══════════════════════════════════════════════════════════

/// Rust-family type formatter.
#[cfg(any(feature = "codegen-rust", feature = "codegen-dioxus"))]
pub(crate) struct RustTypeFormatter;

#[cfg(any(feature = "codegen-rust", feature = "codegen-dioxus"))]
impl FormatType for RustTypeFormatter {
    fn format_type(&self, kind: &str, type_args: &[TypeIR]) -> String {
        match kind {
            "int" | "i32" => "i32".into(),
            "i64" => "i64".into(),
            "u32" => "u32".into(),
            "u64" => "u64".into(),
            "float" | "f64" => "f64".into(),
            "f32" => "f32".into(),
            "bool" => "bool".into(),
            "string" | "String" | "str" => "String".into(),
            "void" | "()" => "()".into(),
            "Vec" | "Array" | "List" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "Vec<{}>",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "Vec<()>".into()
                }
            }
            "FxHashMap" | "HashMap" | "Map" | "Dict" => {
                if type_args.len() >= 2 {
                    format!(
                        "FxHashMap<{}, {}>",
                        self.format_type(type_args[0].kind.as_str(), &type_args[0].type_args),
                        self.format_type(type_args[1].kind.as_str(), &type_args[1].type_args)
                    )
                } else {
                    "FxHashMap<String, ()>".into()
                }
            }
            "Option" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "Option<{}>",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "Option<()>".into()
                }
            }
            other => {
                // If it looks like a user-defined type, use as-is
                if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                    other.into()
                } else {
                    "()".into()
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Python family: Python
// ═══════════════════════════════════════════════════════════

/// Python type formatter.
#[cfg(feature = "codegen-python")]
pub(crate) struct PythonTypeFormatter;

#[cfg(feature = "codegen-python")]
impl FormatType for PythonTypeFormatter {
    fn format_type(&self, kind: &str, type_args: &[TypeIR]) -> String {
        match kind {
            "int" | "i32" | "i64" | "u32" | "u64" => "int".into(),
            "float" | "f64" | "f32" => "float".into(),
            "bool" => "bool".into(),
            "string" | "String" | "str" => "str".into(),
            "void" | "()" => "None".into(),
            "Vec" | "Array" | "List" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "list[{}]",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
                } else {
                    "list".into()
                }
            }
            "FxHashMap" | "HashMap" | "Map" | "Dict" => {
                if type_args.len() >= 2 {
                    format!(
                        "dict[{}]",
                        type_args
                            .iter()
                            .map(|t| self.format_type(t.kind.as_str(), &t.type_args))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    "dict".into()
                }
            }
            "Option" => {
                if let Some(inner) = type_args.first() {
                    format!(
                        "Optional[{}]",
                        self.format_type(inner.kind.as_str(), &inner.type_args)
                    )
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
}
