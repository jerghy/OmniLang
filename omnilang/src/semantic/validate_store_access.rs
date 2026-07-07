//! Store field access validation for Pass 4.
//!
//! Validates that store fields are only accessed when the component has
//! declared the store with `@store`.  Extracted from `validate_types` to
//! keep that file below 800 lines.

use rustc_hash::FxHashSet;
use super::SemanticAnalyzer;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Validate that store fields are only accessed with @store declaration.
    /// Triggers: E0008
    pub(super) fn validate_store_field_access(&mut self, comp: &ComponentDef) {
        let store_refs: FxHashSet<&str> = comp.store_refs.iter().map(|s| s.as_str()).collect();

        // Collect all known store names from the module
        // (we check against store_refs since we're inside validate_module)

        // Walk function bodies looking for StoreFieldAccess or FieldAccess on store names
        fn check_expr_for_undeclared_store(
            expr: &Expr,
            store_refs: &FxHashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            match expr {
                Expr::StoreFieldAccess { store_alias, .. } => {
                    if !store_refs.contains(store_alias.as_str()) {
                        diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0008,
                            &format!(
                                "Accessing store '{}' without @store declaration — \
                                 add '@store {};' to the component body",
                                store_alias, store_alias
                            ),
                            &Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ));
                    }
                }
                Expr::FieldAccess { object, .. } => {
                    if let Expr::VarRef(_name, _) = object.as_ref() {
                        // Check if this is a store name access
                        // Only flag if the name matches a known pattern
                        // We can't know all store names here, so we check
                        // against store_refs (if it's in store_refs, it's fine;
                        // if we can't tell, we're lenient)
                    }
                    check_expr_for_undeclared_store(object, store_refs, diagnostics);
                }
                Expr::FuncCall { func, args, .. } => {
                    check_expr_for_undeclared_store(func, store_refs, diagnostics);
                    for a in args {
                        check_expr_for_undeclared_store(a, store_refs, diagnostics);
                    }
                }
                Expr::MethodCall { object, args, .. } => {
                    check_expr_for_undeclared_store(object, store_refs, diagnostics);
                    for a in args {
                        check_expr_for_undeclared_store(a, store_refs, diagnostics);
                    }
                }
                Expr::BinaryOp { left, right, .. } => {
                    check_expr_for_undeclared_store(left, store_refs, diagnostics);
                    check_expr_for_undeclared_store(right, store_refs, diagnostics);
                }
                Expr::UnaryOp { operand, .. } => {
                    check_expr_for_undeclared_store(operand, store_refs, diagnostics);
                }
                Expr::ArrayLiteral { elements, .. } => {
                    for e in elements {
                        check_expr_for_undeclared_store(e, store_refs, diagnostics);
                    }
                }
                Expr::MapLiteral { entries, .. } => {
                    for (k, v) in entries {
                        check_expr_for_undeclared_store(k, store_refs, diagnostics);
                        check_expr_for_undeclared_store(v, store_refs, diagnostics);
                    }
                }
                Expr::StructLiteral { fields, .. } => {
                    for (_, v) in fields {
                        check_expr_for_undeclared_store(v, store_refs, diagnostics);
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
                    check_expr_for_undeclared_store(condition, store_refs, diagnostics);
                    check_expr_for_undeclared_store(then_expr, store_refs, diagnostics);
                    check_expr_for_undeclared_store(else_expr, store_refs, diagnostics);
                }
                Expr::TemplateString { parts, .. } => {
                    for part in parts {
                        if let TemplatePart::Expr(e) = part {
                            check_expr_for_undeclared_store(e, store_refs, diagnostics);
                        }
                    }
                }
                Expr::Block { body, .. } => {
                    for stmt in &body.stmts {
                        check_stmt_for_undeclared_store(stmt, store_refs, diagnostics);
                    }
                    if let Some(ref trailing) = body.trailing_expr {
                        check_expr_for_undeclared_store(trailing, store_refs, diagnostics);
                    }
                }
                Expr::IndexGet { object, index, .. } => {
                    check_expr_for_undeclared_store(object, store_refs, diagnostics);
                    check_expr_for_undeclared_store(index, store_refs, diagnostics);
                }
                Expr::ClosureExpr { body, .. } => {
                    check_expr_for_undeclared_store(body, store_refs, diagnostics);
                }
                Expr::AwaitExpr { operand, .. } => {
                    check_expr_for_undeclared_store(operand, store_refs, diagnostics);
                }
                Expr::MatchExpr { matched, arms, .. } => {
                    check_expr_for_undeclared_store(matched, store_refs, diagnostics);
                    for arm in arms {
                        for stmt in &arm.body.stmts {
                            check_stmt_for_undeclared_store(stmt, store_refs, diagnostics);
                        }
                        if let Some(ref trailing) = arm.body.trailing_expr {
                            check_expr_for_undeclared_store(trailing, store_refs, diagnostics);
                        }
                    }
                }
                _ => {}
            }
        }

        fn check_stmt_for_undeclared_store(
            stmt: &Stmt,
            store_refs: &FxHashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            match stmt {
                Stmt::ExprStmt { expr, .. }
                | Stmt::Print { arg: expr, .. }
                | Stmt::Throw { expr, .. } => {
                    check_expr_for_undeclared_store(expr, store_refs, diagnostics);
                }
                Stmt::Return { value: Some(e), .. } => {
                    check_expr_for_undeclared_store(e, store_refs, diagnostics);
                }
                Stmt::VarDecl { init: Some(e), .. } => {
                    check_expr_for_undeclared_store(e, store_refs, diagnostics);
                }
                Stmt::Assign { value, .. } => {
                    check_expr_for_undeclared_store(value, store_refs, diagnostics);
                }
                Stmt::If {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    check_expr_for_undeclared_store(condition, store_refs, diagnostics);
                    for s in &then_body.stmts {
                        check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                    }
                    for (cond, body) in else_ifs {
                        check_expr_for_undeclared_store(cond, store_refs, diagnostics);
                        for s in &body.stmts {
                            check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                        }
                    }
                    if let Some(ref body) = else_body {
                        for s in &body.stmts {
                            check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                        }
                    }
                }
                Stmt::While {
                    condition, body, ..
                }
                | Stmt::ForIn {
                    iterable: condition,
                    body,
                    ..
                } => {
                    check_expr_for_undeclared_store(condition, store_refs, diagnostics);
                    for s in &body.stmts {
                        check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                    }
                }
                Stmt::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    for s in &try_body.stmts {
                        check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                    }
                    if let Some(ref body) = catch_body {
                        for s in &body.stmts {
                            check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                        }
                    }
                    if let Some(ref body) = finally_body {
                        for s in &body.stmts {
                            check_stmt_for_undeclared_store(s, store_refs, diagnostics);
                        }
                    }
                }
                _ => {}
            }
        }

        // Walk all component code
        for func in &comp.fns {
            for stmt in &func.body.stmts {
                check_stmt_for_undeclared_store(stmt, &store_refs, &mut self.diagnostics);
            }
            if let Some(ref trailing) = func.body.trailing_expr {
                check_expr_for_undeclared_store(trailing, &store_refs, &mut self.diagnostics);
            }
        }
        for action in &comp.actions {
            for stmt in &action.body.stmts {
                check_stmt_for_undeclared_store(stmt, &store_refs, &mut self.diagnostics);
            }
            if let Some(ref trailing) = action.body.trailing_expr {
                check_expr_for_undeclared_store(trailing, &store_refs, &mut self.diagnostics);
            }
        }
        for lc in &comp.lifecycles {
            for stmt in &lc.body.stmts {
                check_stmt_for_undeclared_store(stmt, &store_refs, &mut self.diagnostics);
            }
            if let Some(ref trailing) = lc.body.trailing_expr {
                check_expr_for_undeclared_store(trailing, &store_refs, &mut self.diagnostics);
            }
        }
        for computed in &comp.computeds {
            check_expr_for_undeclared_store(&computed.body, &store_refs, &mut self.diagnostics);
        }
        for sig in &comp.signals {
            check_expr_for_undeclared_store(&sig.init, &store_refs, &mut self.diagnostics);
        }
        // Walk view tree
        if let Some(ref view) = comp.view {
            for child in &view.children {
                check_view_node_for_undeclared_store(child, &store_refs, &mut self.diagnostics);
            }
        }

        fn check_view_node_for_undeclared_store(
            node: &ViewNode,
            store_refs: &FxHashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            match node {
                ViewNode::Element(el) => {
                    for prop in &el.props {
                        match prop {
                            ElementProp::Static { value, .. }
                            | ElementProp::AnimateSignal(value)
                            | ElementProp::PausedOverride(value)
                            | ElementProp::DrivenBy(value)
                            | ElementProp::InViewOnce(value)
                            | ElementProp::InViewRepeat(value)
                            | ElementProp::InViewThreshold(value)
                            | ElementProp::ScrollScrub(value)
                            | ElementProp::DragElastic(value)
                            | ElementProp::DragScrub(value)
                            | ElementProp::DragMomentum(value)
                            | ElementProp::DragThreshold(value)
                            | ElementProp::DragVelocityMax(value)
                            | ElementProp::HeroTransition(value) => {
                                check_expr_for_undeclared_store(value, store_refs, diagnostics);
                            }
                            ElementProp::Style { entries, .. } => {
                                for (_, expr) in entries {
                                    check_expr_for_undeclared_store(expr, store_refs, diagnostics);
                                }
                            }
                            ElementProp::Class {
                                value: ClassValue::ClassMap(entries),
                            } => {
                                for (_, expr) in entries {
                                    check_expr_for_undeclared_store(expr, store_refs, diagnostics);
                                }
                            }
                            ElementProp::Event {
                                handler: EventHandler::InlineFn { body, .. },
                                ..
                            } => {
                                for stmt in &body.stmts {
                                    check_stmt_for_undeclared_store(stmt, store_refs, diagnostics);
                                }
                                if let Some(ref trailing) = body.trailing_expr {
                                    check_expr_for_undeclared_store(
                                        trailing,
                                        store_refs,
                                        diagnostics,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    for child in &el.children {
                        check_view_node_for_undeclared_store(child, store_refs, diagnostics);
                    }
                }
                ViewNode::Interpolation(expr) => {
                    check_expr_for_undeclared_store(expr, store_refs, diagnostics);
                }
                ViewNode::IfDirective(if_dir) => {
                    check_expr_for_undeclared_store(&if_dir.condition, store_refs, diagnostics);
                    for child in &if_dir.body {
                        check_view_node_for_undeclared_store(child, store_refs, diagnostics);
                    }
                    for (cond, body) in &if_dir.else_ifs {
                        check_expr_for_undeclared_store(cond, store_refs, diagnostics);
                        for child in body {
                            check_view_node_for_undeclared_store(child, store_refs, diagnostics);
                        }
                    }
                    if let Some(ref else_body) = if_dir.else_body {
                        for child in else_body {
                            check_view_node_for_undeclared_store(child, store_refs, diagnostics);
                        }
                    }
                }
                ViewNode::ForDirective(for_dir) => {
                    check_expr_for_undeclared_store(&for_dir.iterable, store_refs, diagnostics);
                    for child in &for_dir.body {
                        check_view_node_for_undeclared_store(child, store_refs, diagnostics);
                    }
                    if let Some(ref empty_body) = for_dir.empty_body {
                        for child in empty_body {
                            check_view_node_for_undeclared_store(child, store_refs, diagnostics);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
