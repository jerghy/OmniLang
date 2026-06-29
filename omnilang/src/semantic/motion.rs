//! Standalone motion validation helper functions.
//!
//! These are free functions (not methods on `SemanticAnalyzer`) used by
//! [`validate_motion`](super::validate_motion) and other passes.

use std::collections::HashSet;

use crate::parser::ast::*;

/// Collect all element `id` attributes from a view tree (for @timeline validation).
pub(super) fn collect_element_ids(nodes: &[ViewNode]) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                for prop in &el.props {
                    if let ElementProp::Static { name, value } = prop {
                        if name == "id" {
                            if let Expr::Literal(Literal::String(id_val), _) = value {
                                ids.insert(id_val.clone());
                            }
                        }
                    }
                }
                ids.extend(collect_element_ids(&el.children));
            }
            ViewNode::IfDirective(if_dir) => {
                ids.extend(collect_element_ids(&if_dir.body));
                for (_, body) in &if_dir.else_ifs {
                    ids.extend(collect_element_ids(body));
                }
                if let Some(ref else_body) = if_dir.else_body {
                    ids.extend(collect_element_ids(else_body));
                }
            }
            ViewNode::ForDirective(for_dir) => {
                ids.extend(collect_element_ids(&for_dir.body));
                if let Some(ref empty_body) = for_dir.empty_body {
                    ids.extend(collect_element_ids(empty_body));
                }
            }
            _ => {}
        }
    }
    ids
}

/// Check if a string is a valid easing function name.
pub(super) fn is_valid_easing_name(name: &str) -> bool {
    matches!(
        name,
        "linear"
            | "easeIn"
            | "easeOut"
            | "easeInOut"
            | "power1In"
            | "power1Out"
            | "power1InOut"
            | "power2In"
            | "power2Out"
            | "power2InOut"
            | "power3In"
            | "power3Out"
            | "power3InOut"
            | "power4In"
            | "power4Out"
            | "power4InOut"
            | "backIn"
            | "backOut"
            | "backInOut"
            | "bounceIn"
            | "bounceOut"
            | "bounceInOut"
            | "elasticIn"
            | "elasticOut"
            | "elasticInOut"
    )
}

/// Check if a value name is a non-interpolatable CSS keyword.
pub(super) fn is_keyword_value(name: &str) -> bool {
    matches!(
        name,
        "auto" | "none" | "normal" | "inherit" | "initial" | "unset" | "revert"
    )
}

/// Check if a view node tree contains any @enter or @leave animation triggers.
pub(super) fn has_enter_leave_animation(nodes: &[ViewNode]) -> bool {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                for prop in &el.props {
                    if let ElementProp::MotionTrigger(mt) = prop {
                        if matches!(mt.trigger, TriggerKind::Enter | TriggerKind::Leave) {
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
            contains_velocity_ref(func) || args.iter().any(|a| contains_velocity_ref(a))
        }
        Expr::MethodCall { object, args, .. } => {
            contains_velocity_ref(object) || args.iter().any(|a| contains_velocity_ref(a))
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            contains_velocity_ref(object)
        }
        Expr::IndexGet { object, index, .. } => {
            contains_velocity_ref(object) || contains_velocity_ref(index)
        }
        Expr::ArrayLiteral { elements, .. } => elements.iter().any(|e| contains_velocity_ref(e)),
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
                    .any(|arm| arm.body.stmts.iter().any(|s| contains_velocity_stmt(s)))
        }
        Expr::Block { .. }
        | Expr::Literal(..)
        | Expr::NotYetImplemented { .. }
        | Expr::StoreFieldAccess { .. }
        | Expr::StructLiteral { .. } => false,
    }
}

pub(super) fn contains_velocity_stmt(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::ExprStmt { expr, .. } => contains_velocity_ref(expr),
        Stmt::Return { value, .. } => value.as_ref().map_or(false, |e| contains_velocity_ref(e)),
        Stmt::VarDecl { init, .. } => init.as_ref().map_or(false, |e| contains_velocity_ref(e)),
        Stmt::Assign { value, .. } => contains_velocity_ref(value),
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            contains_velocity_ref(condition)
                || then_body.stmts.iter().any(|s| contains_velocity_stmt(s))
                || else_ifs.iter().any(|(c, b)| {
                    contains_velocity_ref(c) || b.stmts.iter().any(|s| contains_velocity_stmt(s))
                })
                || else_body
                    .as_ref()
                    .map_or(false, |b| b.stmts.iter().any(|s| contains_velocity_stmt(s)))
        }
        Stmt::While {
            condition, body, ..
        } => {
            contains_velocity_ref(condition) || body.stmts.iter().any(|s| contains_velocity_stmt(s))
        }
        Stmt::ForIn { iterable, body, .. } => {
            contains_velocity_ref(iterable) || body.stmts.iter().any(|s| contains_velocity_stmt(s))
        }
        Stmt::Print { arg, .. } => contains_velocity_ref(arg),
        Stmt::EventEmit { args, .. } => args.iter().any(|a| contains_velocity_ref(a)),
        Stmt::Throw { expr, .. } => contains_velocity_ref(expr),
        Stmt::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body.stmts.iter().any(|s| contains_velocity_stmt(s))
                || catch_body
                    .as_ref()
                    .map_or(false, |b| b.stmts.iter().any(|s| contains_velocity_stmt(s)))
                || finally_body
                    .as_ref()
                    .map_or(false, |b| b.stmts.iter().any(|s| contains_velocity_stmt(s)))
        }
        Stmt::Break(_) | Stmt::Continue(_) => false,
    }
}

/// Parse a duration string (e.g. "300ms", "1.5s") to milliseconds.
pub(super) fn parse_duration_ms(val: &str) -> Option<f64> {
    let val = val.trim();
    if let Some(ms_part) = val.strip_suffix("ms") {
        ms_part.parse::<f64>().ok()
    } else if let Some(s_part) = val.strip_suffix('s') {
        s_part.parse::<f64>().ok().map(|s| s * 1000.0)
    } else {
        None
    }
}
