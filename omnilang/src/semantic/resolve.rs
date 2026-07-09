//! Pass 2 (merged): Resolve references, validate control flow inline.
//!
//! Walks all expressions in every module, component, store, function, and view
//! tree to verify that every [`VarRef`](crate::parser::ast::Expr::VarRef) resolves
//! to a declared symbol.  Also handles inline handler scopes and the `resolve_*`
//! sub-methods for directives.
//!
//! Validation of `break`/`continue` scoping and `await`-in-non-async is performed
//! inline during the traversal using a [`ValidationContext`] — avoiding a separate
//! pass.  Remaining validations (bind targets, store action refs, prop assignment,
//! store field access, motions) are called after the resolve walk for each
//! component / module.

use super::helpers::type_to_string;
use super::symbols::{Symbol, SymbolKind};
use super::validate_inline::{check_control_flow, check_expr_await, ValidationContext};
use super::SemanticAnalyzer;
use crate::error::Diagnostic;
use crate::error::DiagnosticKind;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Walk all expressions in a module and check VarRef names resolve.
    /// Pushes the same scope hierarchy as `collect_symbols` so resolution works.
    ///
    /// Control-flow validation (break/continue/await) runs inline during the walk.
    /// View-tree validations (bind targets, store action refs, prop assignments)
    /// are called after the component's resolve walk finishes.
    pub(super) fn resolve_and_validate_module(&mut self, module: &Module) {
        for comp in &module.components {
            // Find the component scope by name
            let comp_scope_name = format!("component:{}", comp.name);
            if let Some(scope_idx) = self.find_scope(&comp_scope_name) {
                self.symbols.set_current_scope(scope_idx);
            }

            let mut vctx = ValidationContext::new();

            // Resolve prop default values
            for prop in &comp.props {
                if let Some(ref default) = prop.default {
                    self.resolve_expr_refs(default, &mut vctx);
                }
            }

            // Resolve signal initializers
            for sig in &comp.signals {
                self.resolve_expr_refs(&sig.init, &mut vctx);
            }

            // Resolve computed bodies
            for c in &comp.computeds {
                self.resolve_expr_refs(&c.body, &mut vctx);
            }

            for f in &comp.fns {
                let fn_scope_name = format!("fn:{}", f.name);
                if let Some(fn_idx) = self.find_scope(&fn_scope_name) {
                    self.symbols.set_current_scope(fn_idx);
                }
                let mut fn_vctx = ValidationContext {
                    is_async: f.is_async,
                    loop_depth: 0,
                };
                self.resolve_block_refs(&f.body, &mut fn_vctx);
                self.symbols
                    .set_current_scope(self.symbols.current_scope().parent.unwrap_or(0));
            }
            for a in &comp.actions {
                let action_scope_name = format!("action:{}", a.name);
                if let Some(a_idx) = self.find_scope(&action_scope_name) {
                    self.symbols.set_current_scope(a_idx);
                }
                let mut fn_vctx = ValidationContext {
                    is_async: a.is_async,
                    loop_depth: 0,
                };
                self.resolve_block_refs(&a.body, &mut fn_vctx);
                if let Some(parent) = self.symbols.current_scope().parent {
                    self.symbols.set_current_scope(parent);
                }
            }

            // Resolve lifecycle hook bodies
            for lc in &comp.lifecycles {
                let mut lc_vctx = ValidationContext::new();
                self.resolve_block_refs(&lc.body, &mut lc_vctx);
            }

            // Resolve view block (interpolations, directives, element props)
            if let Some(ref view) = comp.view {
                let mut view_vctx = ValidationContext::new();
                for child in &view.children {
                    self.resolve_view_node_refs(child, &mut view_vctx);
                }
            }

            self.symbols.set_current_scope(0); // back to global

            // Remaining validations (view-tree walks / block-scoped checks)
            self.validate_bind_targets(comp);
            self.validate_store_action_refs(comp, module);
            self.validate_store_field_access(comp);
            self.validate_prop_assignment_in_component(comp);
        }
        for f in &module.functions {
            let fn_scope_name = format!("fn:{}", f.name);
            if let Some(fn_idx) = self.find_scope(&fn_scope_name) {
                self.symbols.set_current_scope(fn_idx);
            }
            let mut fn_vctx = ValidationContext {
                is_async: f.is_async,
                loop_depth: 0,
            };
            self.resolve_block_refs(&f.body, &mut fn_vctx);
            self.symbols.set_current_scope(0);
        }

        // Resolve store expressions (signal inits, getter bodies, action bodies)
        for store in &module.stores {
            let store_scope_name = format!("store:{}", store.name);
            if let Some(scope_idx) = self.find_scope(&store_scope_name) {
                self.symbols.set_current_scope(scope_idx);
            }

            let mut vctx = ValidationContext::new();

            for sig in &store.signals {
                self.resolve_expr_refs(&sig.init, &mut vctx);
            }

            for g in &store.getters {
                self.resolve_expr_refs(&g.body, &mut vctx);
            }

            for a in &store.actions {
                let action_scope_name = format!("action:{}", a.name);
                if let Some(a_idx) = self.find_scope(&action_scope_name) {
                    self.symbols.set_current_scope(a_idx);
                }
                let mut fn_vctx = ValidationContext {
                    is_async: a.is_async,
                    loop_depth: 0,
                };
                self.resolve_block_refs(&a.body, &mut fn_vctx);
                if let Some(parent) = self.symbols.current_scope().parent {
                    self.symbols.set_current_scope(parent);
                }
            }

            self.symbols.set_current_scope(0);
        }

        // Motion validation (still a standalone call, not inlined)
        self.validate_motions_in_module(module);
    }

    /// Find a scope by name (hash lookup, O(1) average).
    fn find_scope(&self, name: &str) -> Option<usize> {
        self.symbols.find_scope_by_name(name)
    }

    fn resolve_block_refs(&mut self, block: &Block, vctx: &mut ValidationContext) {
        for stmt in &block.stmts {
            self.resolve_stmt_refs(stmt, vctx);
        }
        if let Some(ref expr) = block.trailing_expr {
            self.resolve_expr_refs(expr, vctx);
        }
    }

    fn resolve_stmt_refs(&mut self, stmt: &Stmt, vctx: &mut ValidationContext) {
        // Inline validation: break/continue must be inside a loop
        if let Some(diag) = check_control_flow(stmt, vctx) {
            self.diagnostics.push(diag);
        }

        match stmt {
            Stmt::VarDecl {
                name,
                init,
                type_ann,
                mutable,
                ..
            } => {
                // Declare the variable so it's available in enclosing scope
                self.declare_or_warn(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    type_ann: type_ann.as_ref().map(type_to_string),
                    mutable: *mutable,
                    declared_at: 0,
                });
                if let Some(e) = init {
                    self.resolve_expr_refs(e, vctx);
                }
            }
            Stmt::ExprStmt { expr, .. } => self.resolve_expr_refs(expr, vctx),
            Stmt::Return { value: Some(e), .. } => {
                self.resolve_expr_refs(e, vctx);
            }
            Stmt::Return { .. } => {}
            Stmt::Assign { value, .. } => self.resolve_expr_refs(value, vctx),
            Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                self.resolve_expr_refs(condition, vctx);
                self.resolve_block_refs(then_body, vctx);
                for (cond, body) in else_ifs {
                    self.resolve_expr_refs(cond, vctx);
                    self.resolve_block_refs(body, vctx);
                }
                if let Some(body) = else_body {
                    self.resolve_block_refs(body, vctx);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                vctx.enter_loop();
                self.resolve_expr_refs(condition, vctx);
                self.resolve_block_refs(body, vctx);
                vctx.exit_loop();
            }
            Stmt::ForIn { iterable, body, .. } => {
                vctx.enter_loop();
                self.resolve_expr_refs(iterable, vctx);
                self.resolve_block_refs(body, vctx);
                vctx.exit_loop();
            }
            Stmt::Print { arg, .. } => self.resolve_expr_refs(arg, vctx),
            // Break / Continue are handled by check_control_flow above
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::EventEmit { args, .. } => {
                for arg in args {
                    self.resolve_expr_refs(arg, vctx);
                }
            }
            Stmt::Throw { expr, .. } => {
                self.resolve_expr_refs(expr, vctx);
            }
            Stmt::TryCatch {
                try_body,
                catch_var,
                catch_body,
                finally_body,
                ..
            } => {
                if let Some(ref var) = catch_var {
                    self.declare_or_warn(Symbol {
                        name: var.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: None,
                        mutable: false,
                        declared_at: 0,
                    });
                }
                self.resolve_block_refs(try_body, vctx);
                if let Some(ref b) = catch_body {
                    self.resolve_block_refs(b, vctx);
                }
                if let Some(ref b) = finally_body {
                    self.resolve_block_refs(b, vctx);
                }
            }
            Stmt::Error { .. } => {}
        }
    }

    fn resolve_expr_refs(&mut self, expr: &Expr, vctx: &mut ValidationContext) {
        // Inline validation: await must be in async context
        if let Some(diag) = check_expr_await(expr, vctx.is_async) {
            self.diagnostics.push(diag);
        }

        match expr {
            Expr::VarRef(name, span) => {
                if !self.symbols.contains(name) && !self.resolving_interpolation {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::E0001,
                        &format!("Undeclared variable '{}'", name),
                        span,
                    ));
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.resolve_expr_refs(left, vctx);
                self.resolve_expr_refs(right, vctx);
            }
            Expr::UnaryOp { operand, .. } => self.resolve_expr_refs(operand, vctx),
            Expr::FuncCall { func, args, .. } => {
                self.resolve_expr_refs(func, vctx);
                for a in args {
                    self.resolve_expr_refs(a, vctx);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                self.resolve_expr_refs(object, vctx);
                for a in args {
                    self.resolve_expr_refs(a, vctx);
                }
            }
            Expr::FieldAccess { object, .. } => self.resolve_expr_refs(object, vctx),
            Expr::OptionalChain { object, .. } => self.resolve_expr_refs(object, vctx),
            Expr::IndexGet { object, index, .. } => {
                self.resolve_expr_refs(object, vctx);
                self.resolve_expr_refs(index, vctx);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    self.resolve_expr_refs(e, vctx);
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    self.resolve_expr_refs(val, vctx);
                }
            }
            Expr::TemplateString { parts, .. } => {
                for part in parts {
                    if let TemplatePart::Expr(e) = part {
                        self.resolve_expr_refs(e, vctx);
                    }
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.resolve_expr_refs(k, vctx);
                    self.resolve_expr_refs(v, vctx);
                }
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.resolve_expr_refs(condition, vctx);
                self.resolve_expr_refs(then_expr, vctx);
                self.resolve_expr_refs(else_expr, vctx);
            }
            Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.resolve_expr_refs(condition, vctx);
                self.resolve_expr_refs(then_expr, vctx);
                self.resolve_expr_refs(else_expr, vctx);
            }
            Expr::Block { body, .. } => {
                self.symbols.push_scope("block_expr");
                self.resolve_block_refs(body, vctx);
                self.symbols.pop_scope();
            }
            Expr::MatchExpr { matched, arms, .. } => {
                self.resolve_expr_refs(matched, vctx);
                for arm in arms {
                    // Each arm introduces a new scope for pattern bindings
                    self.symbols.push_scope("match_arm");
                    if let MatchPatternKind::Variant { bindings, .. } = &arm.pattern.kind {
                        for binding in bindings {
                            self.declare_or_warn(Symbol {
                                name: binding.clone(),
                                kind: SymbolKind::Variable,
                                type_ann: None,
                                mutable: false,
                                declared_at: 0,
                            });
                        }
                    }
                    self.resolve_block_refs(&arm.body, vctx);
                    self.symbols.pop_scope();
                }
            }
            Expr::AwaitExpr { operand, .. } => {
                // check_expr_await at the top of this function handles the diagnostic.
                // Here we just recurse into the operand.
                self.resolve_expr_refs(operand, vctx);
            }
            Expr::ClosureExpr { params, body, .. } => {
                // params are available inside the closure body
                for p in params {
                    self.declare_or_warn(Symbol {
                        name: p.clone(),
                        kind: SymbolKind::Param,
                        type_ann: None,
                        mutable: false,
                        declared_at: 0,
                    });
                }
                self.resolve_expr_refs(body, vctx);
            }
            Expr::StoreFieldAccess { .. } => {
                // store.field — resolved at codegen time, skip here
            }
            _ => {}
        }
    }
}

// ── View tree resolution ────────────────────────────

impl SemanticAnalyzer {
    /// Recursively resolve all expressions within a ViewNode tree.
    pub(super) fn resolve_view_node_refs(&mut self, node: &ViewNode, vctx: &mut ValidationContext) {
        match node {
            ViewNode::Interpolation(expr) => {
                self.resolving_interpolation = true;
                self.resolve_expr_refs(expr, vctx);
                self.resolving_interpolation = false;
            }
            ViewNode::Element(element) => {
                self.resolve_element_refs(element, vctx);
            }
            ViewNode::IfDirective(dir) => {
                self.resolve_if_directive_refs(dir, vctx);
            }
            ViewNode::ForDirective(dir) => {
                self.resolve_for_directive_refs(dir, vctx);
            }
            ViewNode::SlotDirective { .. } | ViewNode::Text(_) => {}
        }
    }

    fn resolve_element_refs(&mut self, element: &Element, vctx: &mut ValidationContext) {
        for prop in &element.props {
            match prop {
                ElementProp::Static { value, .. } => {
                    self.resolve_expr_refs(value, vctx);
                }
                ElementProp::Style { entries, .. } => {
                    for (_, expr) in entries {
                        self.resolve_expr_refs(expr, vctx);
                    }
                }
                ElementProp::Class { value } => {
                    if let ClassValue::ClassMap(entries) = value {
                        for (_, expr) in entries {
                            self.resolve_expr_refs(expr, vctx);
                        }
                    }
                }
                ElementProp::Event { handler, .. } => {
                    self.resolve_event_handler_refs(handler, vctx);
                }
                ElementProp::Bind { .. }
                | ElementProp::TestId(_)
                | ElementProp::Ref(_)
                | ElementProp::MotionTrigger(_)
                | ElementProp::StatesMap(_)
                | ElementProp::AnimateSignal(_)
                | ElementProp::PausedOverride(_)
                | ElementProp::HoverReverse(_)
                | ElementProp::DrivenBy(_)
                | ElementProp::InViewOnce(_)
                | ElementProp::InViewRepeat(_)
                | ElementProp::InViewThreshold(_)
                | ElementProp::When(_)
                | ElementProp::DelayChildren(_)
                | ElementProp::StaggerChildren(_)
                | ElementProp::ScrollStart(_)
                | ElementProp::ScrollEnd(_)
                | ElementProp::ScrollScrub(_)
                | ElementProp::DragConstraints(_)
                | ElementProp::DragElastic(_)
                | ElementProp::DragScrub(_)
                | ElementProp::DragMomentum(_)
                | ElementProp::DragThreshold(_)
                | ElementProp::DragVelocityMax(_)
                | ElementProp::HeroTag(_)
                | ElementProp::HeroTransition(_)
                | ElementProp::Variant { .. } => {}
            }
        }
        for child in &element.children {
            self.resolve_view_node_refs(child, vctx);
        }
    }

    fn resolve_event_handler_refs(&mut self, handler: &EventHandler, vctx: &mut ValidationContext) {
        match handler {
            EventHandler::InlineFn { body, params, .. } => {
                // Push a temporary scope for inline fn params
                self.symbols.push_scope("inline_handler");
                for p in params {
                    self.declare_or_warn(Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: p.type_ann.as_ref().map(type_to_string),
                        mutable: false,
                        declared_at: 0,
                    });
                }
                self.resolve_block_refs(body, vctx);
                self.symbols.pop_scope();
            }
            EventHandler::FnRef(_) | EventHandler::StoreAction { .. } => {}
        }
    }

    fn resolve_if_directive_refs(&mut self, dir: &IfDirective, vctx: &mut ValidationContext) {
        self.resolve_expr_refs(&dir.condition, vctx);
        for child in &dir.body {
            self.resolve_view_node_refs(child, vctx);
        }
        for (condition, body) in &dir.else_ifs {
            self.resolve_expr_refs(condition, vctx);
            for child in body {
                self.resolve_view_node_refs(child, vctx);
            }
        }
        if let Some(ref else_body) = dir.else_body {
            for child in else_body {
                self.resolve_view_node_refs(child, vctx);
            }
        }
    }

    fn resolve_for_directive_refs(&mut self, dir: &ForDirective, vctx: &mut ValidationContext) {
        self.resolve_expr_refs(&dir.iterable, vctx);
        // Push a temporary scope for the loop variable
        self.symbols.push_scope("for_loop");
        self.declare_or_warn(Symbol {
            name: dir.item_name.clone(),
            kind: SymbolKind::Variable,
            type_ann: None,
            mutable: false,
            declared_at: 0,
        });
        // Resolve key_expr inside the loop scope (may reference loop variable)
        if let Some(ref key_expr) = dir.key_expr {
            self.resolve_expr_refs(key_expr, vctx);
        }
        for child in &dir.body {
            self.resolve_view_node_refs(child, vctx);
        }
        if let Some(ref empty_body) = dir.empty_body {
            for child in empty_body {
                self.resolve_view_node_refs(child, vctx);
            }
        }
        self.symbols.pop_scope();
    }
}
