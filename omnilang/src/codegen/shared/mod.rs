//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr`, `FormatStmt`, `FormatType` traits and their
//! language-family implementations, plus the new `FormatBundle` trait and
//! family-specific default generation logic.

pub mod bundle;
pub mod expr;
pub(crate) mod cache;
pub(crate) mod generator;
pub(crate) mod html;
pub(crate) mod inline_handler;
pub(crate) mod stmt;
pub(crate) mod types;
pub(crate) mod visitor;

// Family-specific default generation helpers.
// Each module is gated to compile only when the corresponding features are enabled.
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-omniview"
))]
pub(crate) mod js_defaults;
#[cfg(any(feature = "codegen-dart", feature = "codegen-flutter"))]
pub(crate) mod dart_defaults;
#[cfg(any(feature = "codegen-rust", feature = "codegen-dioxus"))]
pub(crate) mod rust_defaults;
