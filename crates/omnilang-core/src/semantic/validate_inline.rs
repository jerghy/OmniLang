//! Validation helpers called inline during the resolve pass (Phase 5.5 merge).
//!
//! Replaces the separate control-flow validation traversal that was formerly
//! Pass 4 in `validate_types.rs`.  These functions are called inline from
//! `resolve.rs` during the existing AST walk — no second traversal needed.

use crate::error::{Diagnostic, DiagnosticKind};
use crate::parser::ast::*;

/// Tracks loop depth and async context for inline validation during resolve pass.
pub(crate) struct ValidationContext {
    pub is_async: bool,
    pub loop_depth: u32,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self {
            is_async: false,
            loop_depth: 0,
        }
    }

    pub fn enter_loop(&mut self) {
        self.loop_depth += 1;
    }

    pub fn exit_loop(&mut self) {
        self.loop_depth = self.loop_depth.saturating_sub(1);
    }
}

/// Check whether a statement is a `break` or `continue` outside of a loop.
///
/// Returns `Some(diagnostic)` when the statement is an invalid `break` / `continue`:
/// `E0003` — "break/continue used outside of a loop".
pub(crate) fn check_control_flow(stmt: &Stmt, vctx: &ValidationContext) -> Option<Diagnostic> {
    match stmt {
        Stmt::Break(span) if vctx.loop_depth == 0 => Some(Diagnostic::error(
            DiagnosticKind::E0003,
            "break used outside of a loop",
            span,
        )),
        Stmt::Continue(span) if vctx.loop_depth == 0 => Some(Diagnostic::error(
            DiagnosticKind::E0003,
            "continue used outside of a loop",
            span,
        )),
        _ => None,
    }
}

/// Check whether an `await` expression appears outside an async context.
///
/// Returns `Some(diagnostic)` when `expr` is `Expr::AwaitExpr` but `is_async`
/// is `false`: `E0004` — "await used outside async function".
pub(crate) fn check_expr_await(expr: &Expr, is_async: bool) -> Option<Diagnostic> {
    match expr {
        Expr::AwaitExpr { span, .. } if !is_async => Some(Diagnostic::error(
            DiagnosticKind::E0004,
            "await used outside async function",
            span,
        )),
        _ => None,
    }
}
