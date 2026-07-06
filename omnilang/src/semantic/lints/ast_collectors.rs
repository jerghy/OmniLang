//! AST name reference collectors for lint rules.
//!
//! Extracted from `helpers.rs` to keep that file below 800 lines.

use std::collections::HashSet;

use crate::parser::ast::*;

// ── AST name reference collectors ────────────────────

pub(super) fn collect_used_names_in_block<'a>(block: &'a Block, used: &mut HashSet<&'a str>) {
    for stmt in &block.stmts {
        match stmt {
            crate::parser::ast::Stmt::VarDecl { init, .. } => {
                if let Some(ref e) = init {
                    collect_used_names_in_expr(e, used);
                }
            }
            crate::parser::ast::Stmt::Assign { value, .. } => {
                collect_used_names_in_expr(value, used);
            }
            Stmt::ExprStmt { expr, .. } => {
                collect_used_names_in_expr(expr, used);
            }
            crate::parser::ast::Stmt::Return { value: Some(e), .. } => {
                collect_used_names_in_expr(e, used);
            }
            crate::parser::ast::Stmt::Return { .. } => {}
            crate::parser::ast::Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_used_names_in_expr(condition, used);
                collect_used_names_in_block(then_body, used);
                for (cond, body) in else_ifs {
                    collect_used_names_in_expr(cond, used);
                    collect_used_names_in_block(body, used);
                }
                if let Some(ref body) = else_body {
                    collect_used_names_in_block(body, used);
                }
            }
            crate::parser::ast::Stmt::While {
                condition, body, ..
            } => {
                collect_used_names_in_expr(condition, used);
                collect_used_names_in_block(body, used);
            }
            crate::parser::ast::Stmt::ForIn { iterable, body, .. } => {
                collect_used_names_in_expr(iterable, used);
                collect_used_names_in_block(body, used);
            }
            crate::parser::ast::Stmt::Print { arg, .. } => {
                collect_used_names_in_expr(arg, used);
            }
            crate::parser::ast::Stmt::Break(_) | crate::parser::ast::Stmt::Continue(_) => {}
            crate::parser::ast::Stmt::EventEmit { args, .. } => {
                for arg in args {
                    collect_used_names_in_expr(arg, used);
                }
            }
            crate::parser::ast::Stmt::Throw { expr, .. } => {
                collect_used_names_in_expr(expr, used);
            }
            crate::parser::ast::Stmt::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_used_names_in_block(try_body, used);
                if let Some(ref b) = catch_body {
                    collect_used_names_in_block(b, used);
                }
                if let Some(ref b) = finally_body {
                    collect_used_names_in_block(b, used);
                }
            }
            crate::parser::ast::Stmt::Error { .. } => {}
        }
    }
    if let Some(ref expr) = block.trailing_expr {
        collect_used_names_in_expr(expr, used);
    }
}

pub(super) fn collect_used_names_in_expr<'a>(expr: &'a Expr, used: &mut HashSet<&'a str>) {
    match expr {
        Expr::VarRef(name, _) => {
            used.insert(name.as_str());
        }
        Expr::Literal(..) => {}
        Expr::BinaryOp { left, right, .. } => {
            collect_used_names_in_expr(left, used);
            collect_used_names_in_expr(right, used);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_used_names_in_expr(operand, used);
        }
        Expr::FuncCall { func, args, .. } => {
            collect_used_names_in_expr(func, used);
            for a in args {
                collect_used_names_in_expr(a, used);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_used_names_in_expr(object, used);
            for a in args {
                collect_used_names_in_expr(a, used);
            }
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            collect_used_names_in_expr(object, used);
        }
        Expr::IndexGet { object, index, .. } => {
            collect_used_names_in_expr(object, used);
            collect_used_names_in_expr(index, used);
        }
        Expr::ArrayLiteral { elements, .. } => {
            for e in elements {
                collect_used_names_in_expr(e, used);
            }
        }
        Expr::MapLiteral { entries, .. } => {
            for (k, v) in entries {
                collect_used_names_in_expr(k, used);
                collect_used_names_in_expr(v, used);
            }
        }
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
            collect_used_names_in_expr(condition, used);
            collect_used_names_in_expr(then_expr, used);
            collect_used_names_in_expr(else_expr, used);
        }
        Expr::TemplateString { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    collect_used_names_in_expr(e, used);
                }
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, val) in fields {
                collect_used_names_in_expr(val, used);
            }
        }
        Expr::StoreFieldAccess { .. } => {}
        Expr::ClosureExpr { body, .. } => {
            collect_used_names_in_expr(body, used);
        }
        Expr::AwaitExpr { operand, .. } => {
            collect_used_names_in_expr(operand, used);
        }
        Expr::MatchExpr { matched, arms, .. } => {
            collect_used_names_in_expr(matched, used);
            for arm in arms {
                collect_used_names_in_block(&arm.body, used);
            }
        }
        Expr::Block { .. } => {}
        Expr::NotYetImplemented { .. } => {}
        Expr::Error { .. } => {}
    }
}

pub(super) fn collect_used_names_in_view<'a>(view: &'a ViewBlock, used: &mut HashSet<&'a str>) {
    for node in &view.children {
        collect_used_names_in_view_node(node, used);
    }
}

pub(super) fn collect_used_names_in_view_node<'a>(node: &'a ViewNode, used: &mut HashSet<&'a str>) {
    match node {
        ViewNode::Element(el) => {
            for prop in &el.props {
                match prop {
                    ElementProp::Static { value, .. } => {
                        collect_used_names_in_expr(value, used);
                    }
                    ElementProp::Bind { signal_path, .. } => {
                        if let Some(dot) = signal_path.find('.') {
                            used.insert(&signal_path[..dot]);
                        } else {
                            used.insert(signal_path.as_str());
                        }
                    }
                    ElementProp::Event { handler, .. } => match handler {
                        EventHandler::InlineFn { body, .. } => {
                            collect_used_names_in_block(body, used);
                        }
                        EventHandler::FnRef(name) => {
                            used.insert(name.as_str());
                        }
                        EventHandler::StoreAction { store, action, .. } => {
                            used.insert(store.as_str());
                            used.insert(action.as_str());
                        }
                    },
                    _ => {}
                }
            }
            for child in &el.children {
                collect_used_names_in_view_node(child, used);
            }
        }
        ViewNode::Text(_) => {}
        ViewNode::Interpolation(expr) => {
            collect_used_names_in_expr(expr, used);
        }
        ViewNode::IfDirective(if_dir) => {
            collect_used_names_in_expr(&if_dir.condition, used);
            for node in &if_dir.body {
                collect_used_names_in_view_node(node, used);
            }
            for (cond, body) in &if_dir.else_ifs {
                collect_used_names_in_expr(cond, used);
                for node in body {
                    collect_used_names_in_view_node(node, used);
                }
            }
            if let Some(ref body) = if_dir.else_body {
                for node in body {
                    collect_used_names_in_view_node(node, used);
                }
            }
        }
        ViewNode::ForDirective(for_dir) => {
            collect_used_names_in_expr(&for_dir.iterable, used);
            for node in &for_dir.body {
                collect_used_names_in_view_node(node, used);
            }
            if let Some(ref empty_body) = for_dir.empty_body {
                for node in empty_body {
                    collect_used_names_in_view_node(node, used);
                }
            }
        }
        ViewNode::SlotDirective { .. } => {}
    }
}
