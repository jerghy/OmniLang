//! Expression, statement, and type formatting for Dart codegen.
//!
//! Functions for formatting IR expressions, types, and logic constructs
//! (structs, enums, type aliases, standalone functions) as Dart source code.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::shared::types::{DartTypeFormatter, FormatType};
use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

pub(crate) fn format_expr_ir(expr: &ExprIR) -> String {
    DartFormatExpr.format_expr(expr)
}

/// Map OMNI type to Dart type annotation string.
pub fn dart_type(kind: &str, type_args: &[TypeIR]) -> String {
    DartTypeFormatter.format_type(kind, type_args)
}

/// Format a TypeIR into a Dart type string via the existing `dart_type()`.
pub(crate) fn format_type_ir_dart(ty: &TypeIR) -> String {
    DartTypeFormatter.format_type(&ty.kind, &ty.type_args)
}

// ═══════════════════════════════════════════════════════════
// Logic construct generation (moved to visitor.rs)
// ═══════════════════════════════════════════════════════════
