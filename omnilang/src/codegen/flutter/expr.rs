//! Expression formatting for Flutter codegen.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::ir::*;

// ═══════════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════════

pub(crate) fn format_expr_ir_flutter(expr: &ExprIR) -> String {
    DartFormatExpr { event_param_name: None }.format_expr(expr)
}
