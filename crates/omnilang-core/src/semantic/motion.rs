//! Standalone motion validation helper functions.
//!
//! These are free functions (not methods on `SemanticAnalyzer`) used by
//! [`validate_motion`](super::validate_motion) and other passes.

use crate::parser::ast::*;

/// Check if a string is a valid easing function name.
///
/// Delegates to the shared [`NAMED_EASINGS`](crate::codegen::shared::easing::NAMED_EASINGS) table.
pub(super) fn is_valid_easing_name(name: &str) -> bool {
    crate::codegen::shared::easing::resolve_easing(name).is_some()
}

/// Check if a value name is a non-interpolatable CSS keyword.
pub(super) fn is_keyword_value(name: &str) -> bool {
    matches!(
        name,
        "auto" | "none" | "normal" | "inherit" | "initial" | "unset" | "revert"
    )
}

/// Check if a view node tree contains any @enter animation triggers.
pub(super) fn has_enter_leave_animation(nodes: &[ViewNode]) -> bool {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                for prop in &el.props {
                    if let ElementProp::MotionTrigger(mt) = prop {
                        if matches!(mt.trigger, TriggerKind::Enter) {
                            return true;
                        }
                    }
                }
                if has_enter_leave_animation(&el.children) {
                    return true;
                }
            }
            ViewNode::IfDirective(if_dir) => {
                if has_enter_leave_animation(&if_dir.body) {
                    return true;
                }
                for (_, body) in &if_dir.else_ifs {
                    if has_enter_leave_animation(body) {
                        return true;
                    }
                }
                if let Some(ref else_body) = if_dir.else_body {
                    if has_enter_leave_animation(else_body) {
                        return true;
                    }
                }
            }
            ViewNode::ForDirective(for_dir) => {
                if has_enter_leave_animation(&for_dir.body) {
                    return true;
                }
                if let Some(ref empty_body) = for_dir.empty_body {
                    if has_enter_leave_animation(empty_body) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Recursively check if an expression references $velocity.
pub(super) fn contains_velocity_ref(expr: &Expr) -> bool {
    match expr {
        Expr::VarRef(name, _) => name == "$velocity",
        Expr::BinaryOp { left, right, .. } => {
            contains_velocity_ref(left) || contains_velocity_ref(right)
        }
        Expr::UnaryOp { operand, .. } => contains_velocity_ref(operand),
        Expr::FuncCall { func, args, .. } => {
            contains_velocity_ref(func) || args.iter().any(contains_velocity_ref)
        }
        Expr::MethodCall { object, args, .. } => {
            contains_velocity_ref(object) || args.iter().any(contains_velocity_ref)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            contains_velocity_ref(object)
        }
        Expr::IndexGet { object, index, .. } => {
            contains_velocity_ref(object) || contains_velocity_ref(index)
        }
        Expr::ArrayLiteral { elements, .. } => elements.iter().any(contains_velocity_ref),
        Expr::MapLiteral { entries, .. } => entries
            .iter()
            .any(|(k, v)| contains_velocity_ref(k) || contains_velocity_ref(v)),
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        }
        | Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            contains_velocity_ref(condition)
                || contains_velocity_ref(then_expr)
                || contains_velocity_ref(else_expr)
        }
        Expr::TemplateString { parts, .. } => parts.iter().any(|p| {
            if let TemplatePart::Expr(e) = p {
                contains_velocity_ref(e)
            } else {
                false
            }
        }),
        Expr::ClosureExpr { body, .. } => contains_velocity_ref(body),
        Expr::AwaitExpr { operand, .. } => contains_velocity_ref(operand),
        Expr::MatchExpr { matched, arms, .. } => {
            contains_velocity_ref(matched)
                || arms
                    .iter()
                    .any(|arm| arm.body.stmts.iter().any(contains_velocity_stmt))
        }
        Expr::SpreadExpr { operand, .. } => contains_velocity_ref(operand),
        Expr::Block { .. }
        | Expr::Literal(..)
        | Expr::NotYetImplemented { .. }
        | Expr::IsCheck { .. }
        | Expr::StoreFieldAccess { .. }
        | Expr::StructLiteral { .. }
        | Expr::Error { .. } => false,
    }
}

pub(super) fn contains_velocity_stmt(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::ExprStmt { expr, .. } => contains_velocity_ref(expr),
        Stmt::Return { value, .. } => value.as_ref().is_some_and(contains_velocity_ref),
        Stmt::VarDecl { init, .. } => init.as_ref().is_some_and(contains_velocity_ref),
        Stmt::Assign { value, .. } => contains_velocity_ref(value),
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            contains_velocity_ref(condition)
                || then_body.stmts.iter().any(contains_velocity_stmt)
                || else_ifs.iter().any(|(c, b)| {
                    contains_velocity_ref(c) || b.stmts.iter().any(contains_velocity_stmt)
                })
                || else_body
                    .as_ref()
                    .is_some_and(|b| b.stmts.iter().any(contains_velocity_stmt))
        }
        Stmt::While {
            condition, body, ..
        } => contains_velocity_ref(condition) || body.stmts.iter().any(contains_velocity_stmt),
        Stmt::ForIn { iterable, body, .. } => {
            contains_velocity_ref(iterable) || body.stmts.iter().any(contains_velocity_stmt)
        }
        Stmt::Print { arg, .. } => contains_velocity_ref(arg),
        Stmt::EventEmit { args, .. } => args.iter().any(contains_velocity_ref),
        Stmt::Throw { expr, .. } => contains_velocity_ref(expr),
        Stmt::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body.stmts.iter().any(contains_velocity_stmt)
                || catch_body
                    .as_ref()
                    .is_some_and(|b| b.stmts.iter().any(contains_velocity_stmt))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| b.stmts.iter().any(contains_velocity_stmt))
        }
        Stmt::Break(_) | Stmt::Continue(_) => false,
        Stmt::Error { .. } => false,
    }
}

/// Parse a duration string (e.g. "300ms", "1.5s") to milliseconds.
/// Returns `Err` with a description if the value cannot be parsed.
pub(super) fn parse_duration_ms(val: &str) -> Result<f64, String> {
    let val = val.trim();
    if let Some(ms_part) = val.strip_suffix("ms") {
        ms_part
            .parse::<f64>()
            .map_err(|_| format!("invalid number in duration: '{}'", val))
    } else if let Some(s_part) = val.strip_suffix('s') {
        s_part
            .parse::<f64>()
            .map(|s| s * 1000.0)
            .map_err(|_| format!("invalid number in duration: '{}'", val))
    } else {
        Err(format!(
            "invalid duration format: '{}' (expected e.g. '300ms' or '1.5s')",
            val
        ))
    }
}
