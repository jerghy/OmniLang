//! Expression, statement, and type formatting for Rust codegen.
//!
//! Functions for formatting IR expressions, types, and logic constructs
//! (structs, enums, type aliases, standalone functions) as Rust source code.

use crate::codegen::shared::expr::{FormatExpr, RustFormatExpr};
use crate::codegen::shared::types::{FormatType, RustTypeFormatter};
use crate::ir::*;

pub(crate) fn format_expr_ir(expr: &ExprIR) -> String {
    RustFormatExpr.format_expr(expr)
}

/// Map OMNI type to Rust type annotation string.
pub fn rs_type(kind: &str, type_args: &[TypeIR]) -> String {
    RustTypeFormatter.format_type(kind, type_args)
}

// ═══════════════════════════════════════════════════════════
// Logic construct generation (moved to visitor.rs)
// ═══════════════════════════════════════════════════════════

pub(crate) fn format_type_ir_rust(ty: &TypeIR) -> String {
    RustTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args)
}
