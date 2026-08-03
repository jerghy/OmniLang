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
    DartFormatExpr {
        event_param_name: None,
    }
    .format_expr(expr)
}

/// Map OMNI type to Dart type annotation string.
pub fn dart_type(kind: &str, type_args: &[TypeIR]) -> String {
    DartTypeFormatter.format_type(kind, type_args)
}

/// Format a TypeIR into a Dart type string via the existing `dart_type()`.
pub(crate) fn format_type_ir_dart(ty: &TypeIR) -> String {
    DartTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args)
}

/// Format a TypeIR as a Dart type, honoring the `is_nullable` flag
/// (`String?` annotations previously lost their `?` — the signal became a
/// non-nullable field and `user = null` failed to type-check).
#[must_use]
pub fn dart_type_ir(ty: &TypeIR) -> String {
    let base = dart_type(ty.kind.as_str(), &ty.type_args);
    if ty.is_nullable && !base.ends_with('?') {
        format!("{base}?")
    } else {
        base
    }
}

// ═══════════════════════════════════════════════════════════
// Logic construct generation (moved to visitor.rs)
// ═══════════════════════════════════════════════════════════
