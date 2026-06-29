//! Pass 2: Resolve references and resolve the view tree.
//!
//! Walks all expressions in every module, component, store, function, and view
//! tree to verify that every [`VarRef`](crate::parser::ast::Expr::VarRef) resolves
//! to a declared symbol.  Also handles inline handler scopes and the `resolve_*`
//! sub-methods for directives.

use super::helpers::type_to_string;
use super::symbols::{Symbol, SymbolKind};
use super::SemanticAnalyzer;
use crate::error::Diagnostic;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Walk all expressions in a module and check VarRef names resolve.
    /// Pushes the same scope hierarchy as `collect_symbols` so resolution works.
    pub(super) fn resolve_module_types(&mut self, module: &Module) {
        for comp in &module.components {
            // Find the component scope by name
            let comp_scope_name = format!("component:{}", comp.name);
            if let Some(scope_idx) = self.find_scope(&comp_scope_name) {
                self.symbols.current_scope = scope_idx;
            }

            // Resolve prop default values
            for prop in &comp.props {
                if let Some(ref default) = prop.default {
                    self.resolve_expr_refs(default);
                }
            }

            // Resolve signal initializers
            for sig in &comp.signals {
                self.resolve_expr_refs(&sig.init);
            }

            // Resolve computed bodies
            for c in &comp.computeds {
                self.resolve_expr_refs(&c.body);
            }

            for f in &comp.fns {
                let fn_scope_name = format!("fn:{}", f.name);
                if let Some(fn_idx) = self.find_scope(&fn_scope_name) {
                    self.symbols.current_scope = fn_idx;
                }
                self.resolve_block_refs(&f.body);
                self.symbols.current_scope = self.symbols.scopes[self.symbols.current_scope]
                    .parent
                    .unwrap_or(0);
            }
            for a in &comp.actions {
                let action_scope_name = format!("action:{}", a.name);
                if let Some(a_idx) = self.find_scope(&action_scope_name) {
                    self.symbols.current_scope = a_idx;
                }
                self.resolve_block_refs(&a.body);
                if let Some(parent) = self.symbols.scopes[self.symbols.current_scope].parent {
                    self.symbols.current_scope = parent;
                }
            }

            // Resolve lifecycle hook bodies
            for lc in &comp.lifecycles {
                self.resolve_block_refs(&lc.body);
            }

            // Resolve view block (interpolations, directives, element props)
            if let Some(ref view) = comp.view {
                for child in &view.children {
                    self.resolve_view_node_refs(child);
                }
            }

            self.symbols.current_scope = 0; // back to global
        }
        for f in &module.functions {
            let fn_scope_name = format!("fn:{}", f.name);
            if let Some(fn_idx) = self.find_scope(&fn_scope_name) {
                self.symbols.current_scope = fn_idx;
            }
            self.resolve_block_refs(&f.body);
            self.symbols.current_scope = 0;
        }

        // Resolve store expressions (signal inits, getter bodies, action bodies)
        for store in &module.stores {
            let store_scope_name = format!("store:{}", store.name);
            if let Some(scope_idx) = self.find_scope(&store_scope_name) {
                self.symbols.current_scope = scope_idx;
            }

            for sig in &store.signals {
                self.resolve_expr_refs(&sig.init);
            }

            for g in &store.getters {
                self.resolve_expr_refs(&g.body);
            }

            for a in &store.actions {
                let action_scope_name = format!("action:{}", a.name);
                if let Some(a_idx) = self.find_scope(&action_scope_name) {
                    self.symbols.current_scope = a_idx;
                }
                self.resolve_block_refs(&a.body);
                if let Some(parent) = self.symbols.scopes[self.symbols.current_scope].parent {
                    self.symbols.current_scope = parent;
                }
            }

            self.symbols.current_scope = 0;
        }
    }

    /// Find a scope by name (linear scan — sufficient for our scale).
    fn find_scope(&self, name: &str) -> Option<usize> {
        self.symbols.scopes.iter().position(|s| s.name == name)
    }

    fn resolve_block_refs(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.resolve_stmt_refs(stmt);
        }
        if let Some(ref expr) = block.trailing_expr {
            self.resolve_expr_refs(expr);
        }
    }

    fn resolve_stmt_refs(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name,
                init,
                type_ann,
                mutable,
                ..
            } => {
                // Declare the variable so it's available in enclosing scope
                let _ = self.symbols.declare(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    type_ann: type_ann.as_ref().map(type_to_string),
                    mutable: *mutable,
                    declared_at: 0,
                });
                if let Some(e) = init {
                    self.resolve_expr_refs(e);
                }
            }
            Stmt::ExprStmt { expr, .. } => self.resolve_expr_refs(expr),
            Stmt::Return { value: Some(e), .. } => {
                self.resolve_expr_refs(e);
            }
            Stmt::Return { .. } => {}
            Stmt::Assign { value, .. } => self.resolve_expr_refs(value),
            Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                self.resolve_expr_refs(condition);
                self.resolve_block_refs(then_body);
                for (cond, body) in else_ifs {
                    self.resolve_expr_refs(cond);
                    self.resolve_block_refs(body);
                }
                if let Some(body) = else_body {
                    self.resolve_block_refs(body);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.resolve_expr_refs(condition);
                self.resolve_block_refs(body);
            }
            Stmt::ForIn { iterable, body, .. } => {
                self.resolve_expr_refs(iterable);
                self.resolve_block_refs(body);
            }
            Stmt::Print { arg, .. } => self.resolve_expr_refs(arg),
            _ => {}
        }
    }

    fn resolve_expr_refs(&mut self, expr: &Expr) {
        match expr {
            Expr::VarRef(name, span) => {
                if !self.symbols.contains(name) && !self.resolving_interpolation {
                    self.diagnostics.push(Diagnostic::error(
                        "E0001",
                        &format!("Undeclared variable '{}'", name),
                        span,
                    ));
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.resolve_expr_refs(left);
                self.resolve_expr_refs(right);
            }
            Expr::UnaryOp { operand, .. } => self.resolve_expr_refs(operand),
            Expr::FuncCall { func, args, .. } => {
                self.resolve_expr_refs(func);
                for a in args {
                    self.resolve_expr_refs(a);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                self.resolve_expr_refs(object);
                for a in args {
                    self.resolve_expr_refs(a);
                }
            }
            Expr::FieldAccess { object, .. } => self.resolve_expr_refs(object),
            Expr::IndexGet { object, index, .. } => {
                self.resolve_expr_refs(object);
                self.resolve_expr_refs(index);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    self.resolve_expr_refs(e);
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    self.resolve_expr_refs(val);
                }
            }
            Expr::TemplateString { parts, .. } => {
                for part in parts {
                    if let TemplatePart::Expr(e) = part {
                        self.resolve_expr_refs(e);
                    }
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.resolve_expr_refs(k);
                    self.resolve_expr_refs(v);
                }
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.resolve_expr_refs(condition);
                self.resolve_expr_refs(then_expr);
                self.resolve_expr_refs(else_expr);
            }
            Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.resolve_expr_refs(condition);
                self.resolve_expr_refs(then_expr);
                self.resolve_expr_refs(else_expr);
            }
            Expr::Block { body, .. } => {
                self.symbols.push_scope("block_expr");
                self.resolve_block_refs(body);
                self.symbols.pop_scope();
            }
            Expr::MatchExpr { matched, arms, .. } => {
                self.resolve_expr_refs(matched);
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
                    self.resolve_block_refs(&arm.body);
                    self.symbols.pop_scope();
                }
            }
            Expr::AwaitExpr { operand, .. } => {
                self.resolve_expr_refs(operand);
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
                self.resolve_expr_refs(body);
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
    pub(super) fn resolve_view_node_refs(&mut self, node: &ViewNode) {
        match node {
            ViewNode::Interpolation(expr) => {
                self.resolving_interpolation = true;
                self.resolve_expr_refs(expr);
                self.resolving_interpolation = false;
            }
            ViewNode::Element(element) => {
                self.resolve_element_refs(element);
            }
            ViewNode::IfDirective(dir) => {
                self.resolve_if_directive_refs(dir);
            }
            ViewNode::ForDirective(dir) => {
                self.resolve_for_directive_refs(dir);
            }
            ViewNode::SlotDirective { .. } | ViewNode::Text(_) => {}
        }
    }

    fn resolve_element_refs(&mut self, element: &Element) {
        for prop in &element.props {
            match prop {
                ElementProp::Static { value, .. } => {
                    self.resolve_expr_refs(value);
                }
                ElementProp::Style { entries, .. } => {
                    for (_, expr) in entries {
                        self.resolve_expr_refs(expr);
                    }
                }
                ElementProp::Class { value } => {
                    if let ClassValue::ClassMap(entries) = value {
                        for (_, expr) in entries {
                            self.resolve_expr_refs(expr);
                        }
                    }
                }
                ElementProp::Event { handler, .. } => {
                    self.resolve_event_handler_refs(handler);
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
                | ElementProp::HeroTransition(_) => {}
            }
        }
        for child in &element.children {
            self.resolve_view_node_refs(child);
        }
    }

    fn resolve_event_handler_refs(&mut self, handler: &EventHandler) {
        match handler {
            EventHandler::InlineFn { body, params, .. } => {
                // Push a temporary scope for inline fn params
                self.symbols.push_scope("inline_handler");
                for p in params {
                    let _ = self.symbols.declare(Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: p.type_ann.as_ref().map(type_to_string),
                        mutable: false,
                        declared_at: 0,
                    });
                }
                self.resolve_block_refs(body);
                self.symbols.pop_scope();
            }
            EventHandler::FnRef(_) | EventHandler::StoreAction { .. } => {}
        }
    }

    fn resolve_if_directive_refs(&mut self, dir: &IfDirective) {
        self.resolve_expr_refs(&dir.condition);
        for child in &dir.body {
            self.resolve_view_node_refs(child);
        }
        for (condition, body) in &dir.else_ifs {
            self.resolve_expr_refs(condition);
            for child in body {
                self.resolve_view_node_refs(child);
            }
        }
        if let Some(ref else_body) = dir.else_body {
            for child in else_body {
                self.resolve_view_node_refs(child);
            }
        }
    }

    fn resolve_for_directive_refs(&mut self, dir: &ForDirective) {
        self.resolve_expr_refs(&dir.iterable);
        // Push a temporary scope for the loop variable
        self.symbols.push_scope("for_loop");
        let _ = self.symbols.declare(Symbol {
            name: dir.item_name.clone(),
            kind: SymbolKind::Variable,
            type_ann: None,
            mutable: false,
            declared_at: 0,
        });
        // Resolve key_expr inside the loop scope (may reference loop variable)
        if let Some(ref key_expr) = dir.key_expr {
            self.resolve_expr_refs(key_expr);
        }
        for child in &dir.body {
            self.resolve_view_node_refs(child);
        }
        if let Some(ref empty_body) = dir.empty_body {
            for child in empty_body {
                self.resolve_view_node_refs(child);
            }
        }
        self.symbols.pop_scope();
    }
}
