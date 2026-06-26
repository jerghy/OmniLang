//! Semantic analysis — multi-pass: collect, resolve, type-check, deps, validate.

pub mod cycles;
pub mod lints;
pub mod symbols;

use std::collections::{HashMap, HashSet};

use crate::error::Diagnostic;
use crate::lexer::Span;
use crate::parser::ast::*;
use symbols::*;

/// Result of semantic analysis — always returned (diagnostics may contain errors/warnings).
#[derive(Debug)]
pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub dependencies: HashMap<String, Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Run all semantic analysis passes on the parsed program.
/// Always returns a `SemanticResult` (never early-exits) — diagnostics are accumulated
/// in the result for downstream processing.
pub fn analyze(program: &Program) -> SemanticResult {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

pub struct SemanticAnalyzer {
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    dependencies: HashMap<String, Vec<String>>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Declare a symbol, emitting a warning diagnostic on duplicate declaration.
    fn declare_or_warn(&mut self, symbol: Symbol) {
        let declared_at = symbol.declared_at;
        if let Err(msg) = self.symbols.declare(symbol) {
            self.diagnostics.push(Diagnostic {
                severity: crate::error::Severity::Warning,
                code: "E0002".into(),
                message: msg,
                line: 0,
                col: declared_at as u32,
                end_line: 0,
                end_col: declared_at as u32,
                suggestion: None,
            });
        }
    }

    pub fn analyze(&mut self, program: &Program) -> SemanticResult {
        // Pass 1: Collect symbols from all modules
        for (_filepath, module) in &program.modules {
            self.collect_module_symbols(module);
        }

        // Pass 2: Resolve types per module
        for (_filepath, module) in &program.modules {
            self.resolve_module_types(module);
        }

        // Pass 3: Collect dependencies per module
        for (_filepath, module) in &program.modules {
            self.collect_module_dependencies(module);
        }

        // Pass 4: Validate per module
        for (_filepath, module) in &program.modules {
            self.validate_module(module);
        }

        // Always return what we have — diagnostics accumulated in self.diagnostics
        SemanticResult {
            symbols: std::mem::take(&mut self.symbols),
            dependencies: std::mem::take(&mut self.dependencies),
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    // ── Pass 1: Collect symbols ────────────────────────

    fn collect_module_symbols(&mut self, module: &Module) {
        for comp in &module.components {
            self.symbols.push_scope(&format!("component:{}", comp.name));

            for prop in &comp.props {
                self.declare_or_warn(Symbol {
                    name: prop.name.clone(),
                    kind: SymbolKind::Prop,
                    type_ann: Some(type_to_string(&prop.type_ann)),
                    mutable: false,
                    declared_at: prop.type_ann.span().start,
                });
            }

            for sig in &comp.signals {
                self.declare_or_warn(Symbol {
                    name: sig.name.clone(),
                    kind: SymbolKind::Signal,
                    type_ann: Some(type_to_string(&sig.type_ann)),
                    mutable: true,
                    declared_at: sig.type_ann.span().start,
                });
            }

            for c in &comp.computeds {
                self.declare_or_warn(Symbol {
                    name: c.name.clone(),
                    kind: SymbolKind::Computed,
                    type_ann: Some(type_to_string(&c.type_ann)),
                    mutable: false,
                    declared_at: c.type_ann.span().start,
                });
            }

            for ev in &comp.events {
                self.declare_or_warn(Symbol {
                    name: ev.name.clone(),
                    kind: SymbolKind::Event,
                    type_ann: Some(type_to_string(&ev.type_ann)),
                    mutable: false,
                    declared_at: ev.type_ann.span().start,
                });
            }

            for r in &comp.refs {
                self.declare_or_warn(Symbol {
                    name: r.name.clone(),
                    kind: SymbolKind::Ref,
                    type_ann: Some(type_to_string(&r.type_ann)),
                    mutable: false,
                    declared_at: r.type_ann.span().start,
                });
            }

            // Functions, actions, getters
            for f in &comp.fns {
                self.collect_fn_symbols(f);
            }
            for a in &comp.actions {
                self.collect_action_symbols(a);
            }
            for g in &comp.getters {
                self.declare_or_warn(Symbol {
                    name: g.name.clone(),
                    kind: SymbolKind::Getter,
                    type_ann: Some(type_to_string(&g.type_ann)),
                    mutable: false,
                    declared_at: g.type_ann.span().start,
                });
            }

            // Lifecycle hooks
            for lc in &comp.lifecycles {
                self.collect_block_symbols(&lc.body);
            }

            // Add store references from @store declarations to component scope
            for store_ref in &comp.store_refs {
                self.declare_or_warn(Symbol {
                    name: store_ref.clone(),
                    kind: SymbolKind::Store,
                    type_ann: None,
                    mutable: false,
                    declared_at: 0,
                });
            }

            self.symbols.pop_scope(); // component scope
        }

        // Stores
        for store in &module.stores {
            self.symbols.push_scope(&format!("store:{}", store.name));

            for sig in &store.signals {
                self.declare_or_warn(Symbol {
                    name: sig.name.clone(),
                    kind: SymbolKind::Signal,
                    type_ann: Some(type_to_string(&sig.type_ann)),
                    mutable: true,
                    declared_at: sig.type_ann.span().start,
                });
            }

            for g in &store.getters {
                self.declare_or_warn(Symbol {
                    name: g.name.clone(),
                    kind: SymbolKind::Getter,
                    type_ann: Some(type_to_string(&g.type_ann)),
                    mutable: false,
                    declared_at: g.type_ann.span().start,
                });
            }

            for a in &store.actions {
                self.collect_action_symbols(a);
            }

            self.symbols.pop_scope();
        }

        // Top-level functions
        for f in &module.functions {
            self.collect_fn_symbols(f);
        }

        // Structs
        for s in &module.structs {
            self.declare_or_warn(Symbol {
                name: s.name.clone(),
                kind: SymbolKind::Unknown,
                type_ann: None,
                mutable: false,
                declared_at: s.span.start,
            });
        }

        // Tagged unions — register variant names as symbols so bare variant
        // expressions like `Loading` (which parse as VarRef) resolve correctly
        for tu in &module.tagged_unions {
            for variant in &tu.variants {
                self.declare_or_warn(Symbol {
                    name: variant.name.clone(),
                    kind: SymbolKind::Unknown,
                    type_ann: Some(tu.name.clone()),
                    mutable: false,
                    declared_at: 0,
                });
            }
        }

        // Imports — declare imported names as symbols so they can be resolved.
        // Cross-file validation (validate_cross_file) checks these against pub symbols.
        for import in &module.imports {
            match import {
                ImportStmt::Named(ni) => {
                    for name in &ni.names {
                        self.declare_or_warn(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Unknown,
                            type_ann: None,
                            mutable: false,
                            declared_at: ni.span.start,
                        });
                    }
                }
                ImportStmt::TypeOnly(ti) => {
                    for name in &ti.names {
                        self.declare_or_warn(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Unknown,
                            type_ann: None,
                            mutable: false,
                            declared_at: ti.span.start,
                        });
                    }
                }
                ImportStmt::Namespace(nsi) => {
                    self.declare_or_warn(Symbol {
                        name: nsi.alias.clone(),
                        kind: SymbolKind::Unknown,
                        type_ann: None,
                        mutable: false,
                        declared_at: nsi.span.start,
                    });
                }
                ImportStmt::Lib(_) => {
                    // "import omniview" — marker import, no symbols to declare
                }
            }
        }
    }

    fn collect_fn_symbols(&mut self, f: &FunctionDef) {
        self.declare_or_warn(Symbol {
            name: f.name.clone(),
            kind: SymbolKind::Function,
            type_ann: None,
            mutable: false,
            declared_at: f.span.start,
        });
        self.symbols.push_scope(&format!("fn:{}", f.name));
        for p in &f.params {
            self.declare_or_warn(Symbol {
                name: p.name.clone(),
                kind: SymbolKind::Param,
                type_ann: p.type_ann.as_ref().map(type_to_string),
                mutable: false,
                declared_at: 0,
            });
        }
        self.collect_block_symbols(&f.body);
        self.symbols.pop_scope();
    }

    fn collect_action_symbols(&mut self, a: &ActionDef) {
        self.declare_or_warn(Symbol {
            name: a.name.clone(),
            kind: SymbolKind::Action,
            type_ann: None,
            mutable: false,
            declared_at: 0,
        });
        self.symbols.push_scope(&format!("action:{}", a.name));
        for p in &a.params {
            self.declare_or_warn(Symbol {
                name: p.name.clone(),
                kind: SymbolKind::Param,
                type_ann: p.type_ann.as_ref().map(type_to_string),
                mutable: false,
                declared_at: 0,
            });
        }
        self.collect_block_symbols(&a.body);
        self.symbols.pop_scope();
    }

    fn collect_block_symbols(&mut self, block: &Block) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::VarDecl {
                    name,
                    type_ann,
                    mutable,
                    ..
                } => {
                    self.declare_or_warn(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: type_ann.as_ref().map(type_to_string),
                        mutable: *mutable,
                        declared_at: 0,
                    });
                }
                Stmt::ForIn { var_name, body, .. } => {
                    self.declare_or_warn(Symbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: None,
                        mutable: false,
                        declared_at: 0,
                    });
                    self.collect_block_symbols(body);
                }
                Stmt::If {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    self.collect_block_symbols(then_body);
                    for (_, body) in else_ifs {
                        self.collect_block_symbols(body);
                    }
                    if let Some(body) = else_body {
                        self.collect_block_symbols(body);
                    }
                }
                Stmt::While { body, .. } => {
                    self.collect_block_symbols(body);
                }
                _ => {}
            }
        }
    }

    // ── Pass 2: Resolve references ─────────────────────

    fn resolve_module_types(&mut self, module: &Module) {
        // Walk all expressions and check VarRef names resolve.
        // Push the same scope hierarchy as collect_symbols so resolution works.
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
            Stmt::VarDecl { init, .. } => {
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
                if !self.symbols.contains(name) {
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
                self.resolve_block_refs(body);
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

    // ── View tree resolution ────────────────────────────

    /// Recursively resolve all expressions within a ViewNode tree.
    fn resolve_view_node_refs(&mut self, node: &ViewNode) {
        match node {
            ViewNode::Interpolation(expr) => {
                self.resolve_expr_refs(expr);
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

    // ── Pass 3 (renumbered): Collect dependencies ──────

    fn collect_module_dependencies(&mut self, module: &Module) {
        for comp in &module.components {
            let mut deps = Vec::new();
            for c in &comp.computeds {
                deps.push(c.name.clone());
            }
            self.dependencies.insert(comp.name.clone(), deps);
        }
    }

    // ── Pass 4: Validate ───────────────────────────────

    fn validate_module(&mut self, module: &Module) {
        for comp in &module.components {
            self.validate_control_flow_in_component(comp);
            self.validate_bind_targets(comp);
        }
        for func in &module.functions {
            self.validate_function_body(&func.body, func.is_async);
        }
        for store in &module.stores {
            self.validate_store(store);
        }
        self.validate_motions_in_module(module);
    }

    /// break/continue must be inside while/for loops; await must be in async fn
    fn validate_control_flow_in_component(&mut self, comp: &ComponentDef) {
        for action in &comp.actions {
            self.validate_function_body(&action.body, action.is_async);
        }
        for func in &comp.fns {
            self.validate_function_body(&func.body, func.is_async);
        }
        for computed in &comp.computeds {
            self.validate_expr_await(&computed.body, false);
        }
        for lifecycle in &comp.lifecycles {
            let mut depth = 0u32;
            self.validate_block(&lifecycle.body, false, &mut depth);
        }
    }

    fn validate_function_body(&mut self, body: &Block, is_async: bool) {
        let mut loop_depth = 0u32;
        self.validate_block(body, is_async, &mut loop_depth);
    }

    fn validate_block(&mut self, block: &Block, is_async: bool, loop_depth: &mut u32) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Break(span) => {
                    if *loop_depth == 0 {
                        self.diagnostics.push(Diagnostic::error(
                            "E0003",
                            "break used outside of a loop",
                            span,
                        ));
                    }
                }
                Stmt::Continue(span) => {
                    if *loop_depth == 0 {
                        self.diagnostics.push(Diagnostic::error(
                            "E0003",
                            "continue used outside of a loop",
                            span,
                        ));
                    }
                }
                Stmt::While {
                    condition, body, ..
                } => {
                    self.validate_expr_await(condition, is_async);
                    *loop_depth += 1;
                    self.validate_block(body, is_async, loop_depth);
                    *loop_depth -= 1;
                }
                Stmt::ForIn { iterable, body, .. } => {
                    self.validate_expr_await(iterable, is_async);
                    *loop_depth += 1;
                    self.validate_block(body, is_async, loop_depth);
                    *loop_depth -= 1;
                }
                Stmt::If {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    self.validate_expr_await(condition, is_async);
                    self.validate_block(then_body, is_async, loop_depth);
                    for (elif_cond, elif_body) in else_ifs {
                        self.validate_expr_await(elif_cond, is_async);
                        self.validate_block(elif_body, is_async, loop_depth);
                    }
                    if let Some(else_b) = else_body {
                        self.validate_block(else_b, is_async, loop_depth);
                    }
                }
                // Check await in all other statement types
                Stmt::VarDecl { init, .. } => {
                    if let Some(e) = init {
                        self.validate_expr_await(e, is_async);
                    }
                }
                Stmt::ExprStmt { expr, .. } => {
                    self.validate_expr_await(expr, is_async);
                }
                Stmt::Return { value, .. } => {
                    if let Some(e) = value {
                        self.validate_expr_await(e, is_async);
                    }
                }
                Stmt::Assign { value, .. } => {
                    self.validate_expr_await(value, is_async);
                }
                Stmt::Print { arg, .. } => {
                    self.validate_expr_await(arg, is_async);
                }
                Stmt::EventEmit { args, .. } => {
                    for arg in args {
                        self.validate_expr_await(arg, is_async);
                    }
                }
                Stmt::Throw { expr, .. } => {
                    self.validate_expr_await(expr, is_async);
                }
                Stmt::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    self.validate_block(try_body, is_async, loop_depth);
                    if let Some(ref b) = catch_body {
                        self.validate_block(b, is_async, loop_depth);
                    }
                    if let Some(ref b) = finally_body {
                        self.validate_block(b, is_async, loop_depth);
                    }
                }
            }
        }
        // Check trailing expression for await-in-non-async
        if let Some(ref expr) = block.trailing_expr {
            self.validate_expr_await(expr, is_async);
        }
    }

    /// Recursively check for await in non-async context within expressions.
    fn validate_expr_await(&mut self, expr: &Expr, is_async: bool) {
        match expr {
            Expr::AwaitExpr { span, .. } if !is_async => {
                self.diagnostics.push(Diagnostic::error(
                    "E0004",
                    "await used outside async function",
                    span,
                ));
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_expr_await(left, is_async);
                self.validate_expr_await(right, is_async);
            }
            Expr::UnaryOp { operand, .. } => {
                self.validate_expr_await(operand, is_async);
            }
            Expr::FuncCall { func, args, .. } => {
                self.validate_expr_await(func, is_async);
                for arg in args {
                    self.validate_expr_await(arg, is_async);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                self.validate_expr_await(object, is_async);
                for arg in args {
                    self.validate_expr_await(arg, is_async);
                }
            }
            Expr::FieldAccess { object, .. } => {
                self.validate_expr_await(object, is_async);
            }
            Expr::IndexGet { object, index, .. } => {
                self.validate_expr_await(object, is_async);
                self.validate_expr_await(index, is_async);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    self.validate_expr_await(e, is_async);
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.validate_expr_await(k, is_async);
                    self.validate_expr_await(v, is_async);
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    self.validate_expr_await(val, is_async);
                }
            }
            Expr::TemplateString { parts, .. } => {
                for part in parts {
                    if let TemplatePart::Expr(e) = part {
                        self.validate_expr_await(e, is_async);
                    }
                }
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.validate_expr_await(condition, is_async);
                self.validate_expr_await(then_expr, is_async);
                self.validate_expr_await(else_expr, is_async);
            }
            Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.validate_expr_await(condition, is_async);
                self.validate_expr_await(then_expr, is_async);
                self.validate_expr_await(else_expr, is_async);
            }
            Expr::Block { body, .. } => {
                let mut depth = 0u32;
                self.validate_block(body, is_async, &mut depth);
            }
            Expr::MatchExpr { matched, arms, .. } => {
                self.validate_expr_await(matched, is_async);
                for arm in arms {
                    let mut depth = 0u32;
                    self.validate_block(&arm.body, is_async, &mut depth);
                }
            }
            Expr::AwaitExpr { operand, .. } => {
                // Check nested awaits (only the outer one is caught by the top rule)
                self.validate_expr_await(operand, is_async);
            }
            Expr::ClosureExpr {
                params: _, body, ..
            } => {
                // Closures inherit the async context from their enclosing scope
                self.validate_expr_await(body, is_async);
            }
            // Literal, VarRef, StoreFieldAccess, NotYetImplemented have no sub-expressions
            _ => {}
        }
    }

    /// bind: must target a signal, not a store action or fn
    fn validate_bind_targets(&mut self, comp: &ComponentDef) {
        let signal_names: HashSet<&str> = comp.signals.iter().map(|s| s.name.as_str()).collect();

        fn walk_elements(
            nodes: &[ViewNode],
            signal_names: &HashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            for node in nodes {
                match node {
                    ViewNode::Element(el) => {
                        for prop in &el.props {
                            if let ElementProp::Bind {
                                name: _,
                                signal_path,
                            } = prop
                            {
                                // signal_path is e.g. "name" or "store.field"
                                let root = signal_path.split('.').next().unwrap_or(signal_path);
                                if !signal_names.contains(root) && !signal_path.contains('.') {
                                    // Only flag if it doesn't look like a store reference
                                    diagnostics.push(Diagnostic::warning(
                                        "E0005",
                                        &format!(
                                            "bind target '{}' is not a signal in this component",
                                            signal_path
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
                        }
                        walk_elements(&el.children, signal_names, diagnostics);
                    }
                    ViewNode::IfDirective(if_dir) => {
                        walk_elements(&if_dir.body, signal_names, diagnostics);
                        for elif in &if_dir.else_ifs {
                            walk_elements(&elif.1, signal_names, diagnostics);
                        }
                        if let Some(ref else_body) = if_dir.else_body {
                            walk_elements(else_body, signal_names, diagnostics);
                        }
                    }
                    ViewNode::ForDirective(for_dir) => {
                        walk_elements(&for_dir.body, signal_names, diagnostics);
                        if let Some(ref empty_body) = for_dir.empty_body {
                            walk_elements(empty_body, signal_names, diagnostics);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(ref view) = comp.view {
            walk_elements(&view.children, &signal_names, &mut self.diagnostics);
        }
    }

    /// Validate store actions/getters for await and control flow
    fn validate_store(&mut self, store: &StoreDef) {
        for action in &store.actions {
            self.validate_function_body(&action.body, action.is_async);
        }
        for getter in &store.getters {
            self.validate_expr_await(&getter.body, false);
        }
    }

    // ── Motion Validation ─────────────────────────────────────

    /// Validate all motion definitions and timeline triggers in a module.
    fn validate_motions_in_module(&mut self, module: &Module) {
        // Collect all module-level motion/timeline names for reference resolution
        let module_motion_names: HashSet<&str> = module
            .motions
            .iter()
            .map(|m| m.name.as_str())
            .chain(module.timelines.iter().map(|t| t.name.as_str()))
            .collect();

        // Validate module-level motion definitions
        for motion in &module.motions {
            self.validate_motion_def(motion, &module_motion_names);
        }

        // Validate module-level timeline definitions
        let timeline_pairs: Vec<(&str, Vec<&str>)> = module
            .timelines
            .iter()
            .map(|tl| (tl.name.as_str(), collect_timeline_refs(tl)))
            .collect();
        for timeline in &module.timelines {
            self.validate_timeline_def(timeline, &module_motion_names);
        }
        self.detect_timeline_cycles(&timeline_pairs);

        // Validate component-level motions and timelines
        for comp in &module.components {
            let comp_motion_names: HashSet<&str> = module_motion_names
                .iter()
                .copied()
                .chain(comp.motions.iter().map(|m| m.name.as_str()))
                .chain(comp.timelines.iter().map(|t| t.name.as_str()))
                .collect();

            for motion in &comp.motions {
                self.validate_motion_def(motion, &comp_motion_names);
            }

            let comp_timeline_pairs: Vec<(&str, Vec<&str>)> = comp
                .timelines
                .iter()
                .map(|tl| (tl.name.as_str(), collect_timeline_refs(tl)))
                .collect();
            for timeline in &comp.timelines {
                self.validate_timeline_def(timeline, &comp_motion_names);
            }
            self.detect_timeline_cycles(&comp_timeline_pairs);

            // Validate motion triggers in view
            if let Some(ref view) = comp.view {
                // Collect element ids for @timeline validation
                let element_ids = collect_element_ids(&view.children);
                self.validate_view_motion_triggers(
                    &view.children,
                    &comp_motion_names,
                    &element_ids,
                );
            }
        }
    }

    /// Validate a single motion definition (expanded with all spec §20.1 checks).
    fn validate_motion_def(&mut self, motion: &MotionDef, all_motion_names: &HashSet<&str>) {
        match &motion.body {
            MotionBody::Keyframe(kf) => {
                // Check: from must be paired with to
                let has_from = kf
                    .stops
                    .iter()
                    .any(|s| matches!(s.position, StopPosition::From));
                let has_to = kf
                    .stops
                    .iter()
                    .any(|s| matches!(s.position, StopPosition::To));
                if has_from && !has_to {
                    self.diagnostics.push(Diagnostic::error(
                        "E0006",
                        &format!("@motion '{}': 'from' must be paired with 'to'", motion.name),
                        &motion.span,
                    ));
                }

                // Check: non-interpolatable properties
                for stop in &kf.stops {
                    for (prop, expr) in &stop.properties {
                        if is_non_interpolatable(prop) {
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!("@motion '{}': '{}' is not interpolatable and cannot be animated", motion.name, prop),
                                &motion.span,
                            ));
                        }
                        // Check: keyword values in animation (e.g. width: auto→300px)
                        if let Expr::VarRef(name, _) = expr {
                            if is_keyword_value(name) {
                                self.diagnostics.push(Diagnostic::error(
                                    "E0006",
                                    &format!("@motion '{}': keyword value '{}' cannot be used in animation for property '{}'", motion.name, name, prop),
                                    &motion.span,
                                ));
                            }
                        }
                    }
                }

                // Check: type mismatch across stops
                self.validate_keyframe_type_mismatch(motion, kf);

                // Check: valid easing name
                if let Some(ref e) = kf.ease {
                    self.validate_easing_name(e, &motion.span);
                }
                for stop in &kf.stops {
                    if let Some(ref e) = stop.ease {
                        self.validate_easing_name(e, &motion.span);
                    }
                }

                // Check: clipPath with different shape functions
                self.validate_clip_path_shapes(motion, kf);

                // Check: boxShadow with different value structures
                self.validate_box_shadow_structure(motion, kf);

                // Warning: d attribute path structure mismatch
                self.warn_path_structure_mismatch(motion, kf);

                // Validate playback property conflicts
                self.validate_playback_conflicts(motion, &kf.playback);
            }
            MotionBody::Timeline(tl) => {
                // Check: set is allowed in timeline; stagger is allowed in timeline
                // Check: valid easing names in timeline items
                for item in &tl.items {
                    if let TimelineItemKind::Direction(dir) = &item.kind {
                        if let Some(ref e) = dir.ease {
                            self.validate_easing_name(e, &motion.span);
                        }
                    }
                }
                for item in &tl.items {
                    match &item.kind {
                        TimelineItemKind::Direction(dir) => {
                            for (prop, expr) in &dir.properties {
                                if is_non_interpolatable(prop) {
                                    self.diagnostics.push(Diagnostic::error(
                                        "E0006",
                                        &format!("@motion '{}': '{}' is not interpolatable and cannot be animated", motion.name, prop),
                                        &motion.span,
                                    ));
                                }
                                if let Expr::VarRef(name, _) = expr {
                                    if is_keyword_value(name) {
                                        self.diagnostics.push(Diagnostic::error(
                                            "E0006",
                                            &format!("@motion '{}': keyword value '{}' cannot be used in animation for property '{}'", motion.name, name, prop),
                                            &motion.span,
                                        ));
                                    }
                                }
                            }
                        }
                        TimelineItemKind::MotionRef(ref_name) => {
                            if !all_motion_names.contains(ref_name.as_str()) {
                                self.diagnostics.push(Diagnostic::error(
                                    "E0006",
                                    &format!(
                                        "@motion '{}': references undefined motion '{}'",
                                        motion.name, ref_name
                                    ),
                                    &motion.span,
                                ));
                            }
                            // Check circular self-reference
                            if ref_name == &motion.name {
                                self.diagnostics.push(Diagnostic::error(
                                    "E0006",
                                    &format!(
                                        "@motion '{}': circular reference to itself",
                                        motion.name
                                    ),
                                    &motion.span,
                                ));
                            }
                        }
                        TimelineItemKind::Stagger(stagger) => {
                            if let Expr::VarRef(ref_name, _) = stagger.motion.as_ref() {
                                if !all_motion_names.contains(ref_name.as_str()) {
                                    self.diagnostics.push(Diagnostic::error(
                                        "E0006",
                                        &format!(
                                            "@motion '{}': references undefined motion '{}'",
                                            motion.name, ref_name
                                        ),
                                        &motion.span,
                                    ));
                                }
                                if ref_name == &motion.name {
                                    self.diagnostics.push(Diagnostic::error(
                                        "E0006",
                                        &format!("@motion '{}': circular reference to itself via stagger", motion.name),
                                        &motion.span,
                                    ));
                                }
                            }
                        }
                        TimelineItemKind::SetBlock(_) | TimelineItemKind::Label(_, _)
                        | TimelineItemKind::StartPos(_) => {}
                    }
                }

                // Validate playback property conflicts
                self.validate_playback_conflicts(motion, &tl.playback);
            }
            MotionBody::CallbackOnly(cb) => {
                // Check: $velocity only valid in gesture contexts
                for spec in &cb.callbacks {
                    let expr = match spec {
                        CallbackSpec::OnStart(e)
                        | CallbackSpec::OnUpdate(e)
                        | CallbackSpec::OnComplete(e)
                        | CallbackSpec::OnRepeat(e)
                        | CallbackSpec::OnInterrupt(e) => e,
                    };
                    if contains_velocity_ref(expr) {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            &format!("@motion '{}': '$velocity' is only valid in @drag-horizontal/@drag-vertical callbacks, not in standalone motion definitions", motion.name),
                            &motion.span,
                        ));
                        break;
                    }
                }
            }
        }

        // Emit warnings for all motion types
        self.emit_motion_warnings(motion);
    }

    /// Validate playback property conflicts (§20.1).
    fn validate_playback_conflicts(&mut self, motion: &MotionDef, playback: &[PlaybackProp]) {
        let mut has_loop_infinite = false;
        let mut has_fill_both = false;

        // Note: driven-by is checked at element level, not motion level.
        // But we check loop + fill conflicts here.

        for prop in playback {
            match prop {
                PlaybackProp::Loop(lv) => {
                    has_loop_infinite = matches!(lv, LoopValue::Infinite);
                }
                PlaybackProp::Fill(fv) => {
                    has_fill_both = matches!(fv, FillValue::Both);
                }
                PlaybackProp::Direction(dv) => {
                    // Warning: direction: alternate + loop: 1
                    if let Some(lv) = playback.iter().find_map(|p| {
                        if let PlaybackProp::Loop(lv) = p {
                            Some(lv)
                        } else {
                            None
                        }
                    }) {
                        if matches!(
                            dv,
                            DirectionValue::Alternate | DirectionValue::AlternateReverse
                        ) && matches!(lv, LoopValue::Count(1))
                        {
                            self.diagnostics.push(Diagnostic::warning(
                                "W0001",
                                &format!("@motion '{}': direction 'alternate' with loop: 1 — animation plays forward then reverse; is this intended?", motion.name),
                                &motion.span,
                            ));
                        }
                    }
                }
                PlaybackProp::Paused(_) => {
                    // No conflict, just a state flag
                }
                PlaybackProp::When(_) => {
                    self.diagnostics.push(Diagnostic::error(
                        "E0006",
                        &format!("@motion '{}': 'when' is only valid on @enter/@leave element triggers, not on @motion definitions", motion.name),
                        &motion.span,
                    ));
                }
                PlaybackProp::DelayChildren(_) | PlaybackProp::StaggerChildren(_) => {
                    // These ARE valid on @motion definitions — they configure
                    // how the motion distributes animations among child elements.
                    // The runtime processes them via playback_to_config.
                }
                _ => {}
            }
        }

        // Check fill: both + loop: infinite
        if has_fill_both && has_loop_infinite {
            self.diagnostics.push(Diagnostic::warning(
                "W0001",
                &format!(
                    "@motion '{}': fill 'both' has no meaning with loop 'infinite'",
                    motion.name
                ),
                &motion.span,
            ));
        }
    }

    /// Validate that an easing expression uses a recognized name.
    fn validate_easing_name(&mut self, e: &EasingExpr, span: &Span) {
        if let EasingExpr::Named(name) = e {
            if !is_valid_easing_name(name) {
                self.diagnostics.push(Diagnostic::error(
                    "E0006",
                    &format!("Unknown easing name '{}'", name),
                    span,
                ));
            }
        }
        if let EasingExpr::Back { direction, .. } = e {
            if let Some(ref dir_name) = direction {
                if !is_valid_easing_name(dir_name) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0006",
                        &format!("Unknown easing name '{}'", dir_name),
                        span,
                    ));
                }
            }
        }
        if let EasingExpr::Elastic { direction, .. } = e {
            if let Some(ref dir_name) = direction {
                if !is_valid_easing_name(dir_name) {
                    self.diagnostics.push(Diagnostic::error(
                        "E0006",
                        &format!("Unknown easing name '{}'", dir_name),
                        span,
                    ));
                }
            }
        }
    }

    /// Validate element-level animation conflicts.
    fn validate_element_motion_props(&mut self, el: &Element, all_motion_names: &HashSet<&str>) {
        let mut has_driven_by = false;
        let mut motion_count: u32 = 0;

        for prop in &el.props {
            match prop {
                ElementProp::DrivenBy(_) => {
                    has_driven_by = true;
                }
                ElementProp::MotionTrigger(trigger) => {
                    motion_count += 1;
                    // Check driven-by + loop conflict: can't fully check here
                    // since loop is in the motion def; validate when resolving
                    if let Expr::VarRef(name, span) = &trigger.motion_expr {
                        if !all_motion_names.contains(name.as_str()) {
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!(
                                    "Undefined motion '{}' referenced in element trigger",
                                    name
                                ),
                                span,
                            ));
                        }
                    }
                }
                ElementProp::When(_)
                | ElementProp::DelayChildren(_)
                | ElementProp::StaggerChildren(_) => {
                    // These are only valid on @enter/@leave triggers
                    // Check: if there's no @enter or @leave trigger, warn
                    let has_enter_leave = el.props.iter().any(|p| {
                        if let ElementProp::MotionTrigger(mt) = p {
                            matches!(mt.trigger, TriggerKind::Enter | TriggerKind::Leave)
                        } else {
                            false
                        }
                    });
                    if !has_enter_leave {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            "when:/delayChildren:/staggerChildren: are only valid on @enter/@leave triggers",
                            &el.span,
                        ));
                    }
                }
                ElementProp::StatesMap(states) => {
                    // Validate states: block
                    self.validate_states_block(el, states);
                }
                _ => {}
            }
        }

        // Warning: >10 animations on a single element (performance risk)
        if motion_count > 10 {
            self.diagnostics.push(Diagnostic::warning(
                "W0001",
                &format!(
                    "Element has {} motion triggers — performance risk (>10 recommended max)",
                    motion_count
                ),
                &el.span,
            ));
        }

        // Warning: driven-by + loop conflict (if element has both)
        if has_driven_by {
            for prop in &el.props {
                if let ElementProp::MotionTrigger(trigger) = prop {
                    if let Expr::VarRef(_name, _) = &trigger.motion_expr {
                        // If the referenced motion has loop, that's a conflict
                        // This requires cross-referencing with motion defs
                        // For now, mark as warning
                    }
                }
            }
        }
    }

    /// Validate states: block (§20.1).
    fn validate_states_block(&mut self, el: &Element, states: &[(String, StateDef)]) {
        let has_reserved = states
            .iter()
            .any(|(name, _)| matches!(name.as_str(), "idle" | "hover" | "pressed"));
        let has_custom = !has_reserved;
        let has_on_signal = el
            .props
            .iter()
            .any(|p| matches!(p, ElementProp::AnimateSignal(_)));

        if has_reserved && has_custom && has_on_signal {
            self.diagnostics.push(Diagnostic::error(
                "E0006",
                "states: cannot mix reserved names (idle/hover/pressed) with custom names and on: signal binding",
                &el.span,
            ));
        }

        // Custom states require on: signal
        if !has_reserved && !has_on_signal {
            self.diagnostics.push(Diagnostic::error(
                "E0006",
                "states: with custom state names requires on: signal binding to drive state selection",
                &el.span,
            ));
        }

        // Reserved names with on: present is an error
        if has_reserved && has_on_signal {
            self.diagnostics.push(Diagnostic::error(
                "E0006",
                "states: reserved names (idle/hover/pressed) auto-bind to mouse events; on: is only valid with custom state names",
                &el.span,
            ));
        }

        // Check for duplicate state names
        let mut seen: HashSet<&str> = HashSet::new();
        for (name, _) in states {
            if !seen.insert(name.as_str()) {
                self.diagnostics.push(Diagnostic::error(
                    "E0006",
                    &format!("Duplicate state name '{}' in states: block", name),
                    &el.span,
                ));
            }
        }
    }

    /// Validate @if mode string (§20.1).
    fn validate_if_mode(&mut self, mode: &str, span: &Span) {
        if mode != "sync" && mode != "wait" {
            self.diagnostics.push(Diagnostic::error(
                "E0006",
                &format!("@if mode '{}' is invalid; expected 'sync' or 'wait'", mode),
                span,
            ));
        }
    }

    /// Check for type mismatches across keyframe stops for the same property.
    fn validate_keyframe_type_mismatch(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        let mut prop_types: HashMap<&str, &str> = HashMap::new();
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                let tag = match expr {
                    Expr::Literal(lit, _) => match lit {
                        Literal::Int(_) | Literal::Float(_) => "number",
                        Literal::String(_) => "string",
                        Literal::Bool(_) => "bool",
                        Literal::Null => "null",
                    },
                    _ => "unknown",
                };
                if let Some(prev_tag) = prop_types.get(prop.as_str()) {
                    if *prev_tag != tag && *prev_tag != "unknown" && tag != "unknown" {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            &format!(
                                "@motion '{}': type mismatch for property '{}' ({} vs {})",
                                motion.name, prop, prev_tag, tag
                            ),
                            &motion.span,
                        ));
                    }
                } else {
                    prop_types.insert(prop.as_str(), tag);
                }
            }
        }
    }

    /// Validate clipPath shape consistency across keyframe stops.
    fn validate_clip_path_shapes(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                if prop == "clipPath" || prop == "clip-path" {
                    if let Expr::Literal(Literal::String(val), _) = expr {
                        // Extract shape function name (e.g., "circle", "inset", "polygon")
                        let shape = val.trim_start().split('(').next().unwrap_or("");
                        // Record and compare across stops
                        for other_stop in &kf.stops {
                            for (other_prop, other_expr) in &other_stop.properties {
                                if other_prop == prop {
                                    if let Expr::Literal(Literal::String(other_val), _) = other_expr
                                    {
                                        let other_shape =
                                            other_val.trim_start().split('(').next().unwrap_or("");
                                        if shape != other_shape
                                            && !shape.is_empty()
                                            && !other_shape.is_empty()
                                        {
                                            self.diagnostics.push(Diagnostic::error(
                                                "E0006",
                                                &format!("@motion '{}': clipPath shape mismatch — '{}()' and '{}()' are different shape functions", motion.name, shape, other_shape),
                                                &motion.span,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate boxShadow value structure consistency.
    fn validate_box_shadow_structure(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        let mut first_box_shadow: Option<&str> = None;
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                if prop == "boxShadow" || prop == "box-shadow" {
                    if let Expr::Literal(Literal::String(val), _) = expr {
                        if let Some(first) = first_box_shadow {
                            let has_rgba1 = first.contains("rgba");
                            let has_rgba2 = val.contains("rgba");
                            if has_rgba1 != has_rgba2 {
                                self.diagnostics.push(Diagnostic::error(
                                    "E0006",
                                    &format!("@motion '{}': boxShadow value structure mismatch — one uses rgba() and the other does not", motion.name),
                                    &motion.span,
                                ));
                            }
                        } else {
                            first_box_shadow = Some(val);
                        }
                    }
                }
            }
        }
    }

    /// Warning: SVG path 'd' attribute structure mismatch.
    fn warn_path_structure_mismatch(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        let mut first_path: Option<&str> = None;
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                if prop == "d" {
                    if let Expr::Literal(Literal::String(val), _) = expr {
                        if let Some(first) = first_path {
                            let cmds1: Vec<char> =
                                first.chars().filter(|c| c.is_alphabetic()).collect();
                            let cmds2: Vec<char> =
                                val.chars().filter(|c| c.is_alphabetic()).collect();
                            if cmds1 != cmds2 {
                                self.diagnostics.push(Diagnostic::warning(
                                    "W0001",
                                    &format!("@motion '{}': SVG path 'd' attribute command structure differs between keyframe stops — interpolation may not work as expected", motion.name),
                                    &motion.span,
                                ));
                            }
                        } else {
                            first_path = Some(val);
                        }
                    }
                }
            }
        }
    }

    // ── Motion Warnings (§20.2) ─────────────────────────

    /// Emit warnings for motion-specific issues.
    fn emit_motion_warnings(&mut self, motion: &MotionDef) {
        self.check_over_duration_warnings(motion);
        self.check_opacity_range_warnings(motion);
    }

    fn check_over_duration_warnings(&mut self, motion: &MotionDef) {
        let over_exprs: Vec<&Expr> = match &motion.body {
            MotionBody::Keyframe(kf) => vec![&kf.over],
            MotionBody::Timeline(tl) => tl
                .items
                .iter()
                .filter_map(|item| match &item.kind {
                    TimelineItemKind::Direction(dir) => Some(&dir.over),
                    _ => None,
                })
                .collect(),
            MotionBody::CallbackOnly(cb) => vec![&cb.over],
        };

        for expr in &over_exprs {
            if let Expr::Literal(Literal::String(val), _) = expr {
                if val == "0ms" || val == "0s" {
                    self.diagnostics.push(Diagnostic::warning(
                        "W0001",
                        &format!("@motion '{}': over: 0ms — consider using 'set' instead for instant assignment", motion.name),
                        &motion.span,
                    ));
                }
                if let Some(ms) = parse_duration_ms(val) {
                    if ms > 3_600_000.0 {
                        self.diagnostics.push(Diagnostic::warning(
                            "W0001",
                            &format!(
                                "@motion '{}': over > 3600s ({}) — suspiciously long duration",
                                motion.name, val
                            ),
                            &motion.span,
                        ));
                    }
                }
            }
        }
    }

    fn check_opacity_range_warnings(&mut self, motion: &MotionDef) {
        let props: Vec<&(String, Expr)> = match &motion.body {
            MotionBody::Keyframe(kf) => kf.stops.iter().flat_map(|s| &s.properties).collect(),
            MotionBody::Timeline(tl) => tl
                .items
                .iter()
                .filter_map(|item| match &item.kind {
                    TimelineItemKind::Direction(dir) => Some(&dir.properties),
                    TimelineItemKind::SetBlock(props) => Some(props),
                    _ => None,
                })
                .flat_map(|p| p.iter())
                .collect(),
            _ => vec![],
        };

        for (prop, expr) in &props {
            if prop == "opacity" {
                if let Expr::Literal(lit, _) = expr {
                    match lit {
                        Literal::Float(f) if *f < 0.0 || *f > 1.0 => {
                            self.diagnostics.push(Diagnostic::warning(
                                "W0001",
                                &format!(
                                    "@motion '{}': opacity value {} is outside [0, 1] range",
                                    motion.name, f
                                ),
                                &motion.span,
                            ));
                        }
                        Literal::Int(i) if *i < 0 || *i > 1 => {
                            self.diagnostics.push(Diagnostic::warning(
                                "W0001",
                                &format!(
                                    "@motion '{}': opacity value {} is outside [0, 1] range",
                                    motion.name, i
                                ),
                                &motion.span,
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // ── View Tree Motion Validation ─────────────────────

    /// Validate a single timeline definition (undefined refs, non-interpolatable props).
    fn validate_timeline_def(&mut self, timeline: &TimelineDef, all_motion_names: &HashSet<&str>) {
        for item in &timeline.items {
            match &item.kind {
                TimelineItemKind::MotionRef(ref_name) => {
                    if !all_motion_names.contains(ref_name.as_str()) {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            &format!(
                                "@timeline '{}': references undefined motion '{}'",
                                timeline.name, ref_name
                            ),
                            &timeline.span,
                        ));
                    }
                }
                TimelineItemKind::Direction(dir) => {
                    for (prop, _) in &dir.properties {
                        if is_non_interpolatable(prop) {
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!("@timeline '{}': '{}' is not interpolatable and cannot be animated", timeline.name, prop),
                                &timeline.span,
                            ));
                        }
                    }
                }
                TimelineItemKind::Stagger(stagger) => {
                    if let Expr::VarRef(ref_name, _) = stagger.motion.as_ref() {
                        if !all_motion_names.contains(ref_name.as_str()) {
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!(
                                    "@timeline '{}': references undefined motion '{}'",
                                    timeline.name, ref_name
                                ),
                                &timeline.span,
                            ));
                        }
                    }
                }
                TimelineItemKind::SetBlock(_) | TimelineItemKind::Label(_, _) | TimelineItemKind::StartPos(_) => {}
            }
        }
    }

    /// Detect circular motion references in timeline definitions.
    fn detect_timeline_cycles(&mut self, refs: &[(&str, Vec<&str>)]) {
        let graph: HashMap<&str, &[&str]> = refs
            .iter()
            .map(|(name, deps)| (*name, deps.as_slice()))
            .collect();

        let mut reported: HashSet<&str> = HashSet::new();

        for &(start_name, _) in refs {
            if reported.contains(start_name) {
                continue;
            }
            // DFS from start, following only timeline nodes
            let mut visited: HashSet<&str> = HashSet::new();
            let mut stack: Vec<&str> = vec![start_name];
            visited.insert(start_name);

            while let Some(current) = stack.pop() {
                if let Some(neighbors) = graph.get(current) {
                    for &next in *neighbors {
                        if !graph.contains_key(next) {
                            // Only follow timeline nodes for cycle detection
                            continue;
                        }
                        if next == start_name {
                            reported.insert(start_name);
                            self.diagnostics.push(Diagnostic::error(
                                "E0006",
                                &format!(
                                    "Timeline '{}' contains a circular motion reference",
                                    start_name
                                ),
                                &Span {
                                    start: 0,
                                    end: 0,
                                    line: 0,
                                    col: 0,
                                },
                            ));
                            stack.clear();
                            break;
                        }
                        if !visited.contains(next) {
                            visited.insert(next);
                            stack.push(next);
                        }
                    }
                }
            }
        }
    }

    /// Validate motion triggers in view tree (expanded with @for key checking, element ID collection).
    fn validate_view_motion_triggers(
        &mut self,
        nodes: &[ViewNode],
        all_motion_names: &HashSet<&str>,
        element_ids: &HashSet<String>,
    ) {
        for node in nodes {
            match node {
                ViewNode::Element(el) => {
                    self.validate_element_motion_props(el, all_motion_names);

                    // Validate @if mode on conditional branches within element
                    self.validate_view_motion_triggers(&el.children, all_motion_names, element_ids);
                }
                ViewNode::IfDirective(if_dir) => {
                    // Validate @if mode string if present
                    if let Some(ref mode) = if_dir.transition_mode {
                        self.validate_if_mode(mode, &if_dir.span);
                    }
                    self.validate_view_motion_triggers(&if_dir.body, all_motion_names, element_ids);
                    for (_, body) in &if_dir.else_ifs {
                        self.validate_view_motion_triggers(body, all_motion_names, element_ids);
                    }
                    if let Some(ref else_body) = if_dir.else_body {
                        self.validate_view_motion_triggers(
                            else_body,
                            all_motion_names,
                            element_ids,
                        );
                    }
                }
                ViewNode::ForDirective(for_dir) => {
                    // Check: @for with enter/leave/stagger triggers but no key={}
                    let has_anim = has_enter_leave_animation(&for_dir.body);
                    let has_key = for_dir.key_expr.is_some();
                    if has_anim && !has_key {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            "@for with @enter/@leave/stagger animation requires key={} for identity tracking",
                            &for_dir.span,
                        ));
                    }
                    self.validate_view_motion_triggers(
                        &for_dir.body,
                        all_motion_names,
                        element_ids,
                    );
                    if let Some(ref empty_body) = for_dir.empty_body {
                        self.validate_view_motion_triggers(
                            empty_body,
                            all_motion_names,
                            element_ids,
                        );
                    }
                }
                ViewNode::Text(_) | ViewNode::Interpolation(_) | ViewNode::SlotDirective { .. } => {}
            }
        }
    }
}

// ── Motion Validation Helpers (standalone) ──────────────

/// Collect all element `id` attributes from a view tree (for @timeline validation).
fn collect_element_ids(nodes: &[ViewNode]) -> HashSet<String> {
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
fn is_valid_easing_name(name: &str) -> bool {
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
fn is_keyword_value(name: &str) -> bool {
    matches!(
        name,
        "auto" | "none" | "normal" | "inherit" | "initial" | "unset" | "revert"
    )
}

/// Check if a view node tree contains any @enter or @leave animation triggers.
fn has_enter_leave_animation(nodes: &[ViewNode]) -> bool {
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
fn contains_velocity_ref(expr: &Expr) -> bool {
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

fn contains_velocity_stmt(stmt: &Stmt) -> bool {
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
fn parse_duration_ms(val: &str) -> Option<f64> {
    let val = val.trim();
    if let Some(ms_part) = val.strip_suffix("ms") {
        ms_part.parse::<f64>().ok()
    } else if let Some(s_part) = val.strip_suffix('s') {
        s_part.parse::<f64>().ok().map(|s| s * 1000.0)
    } else {
        None
    }
}

// ── Helpers ────────────────────────────────────────────

/// Convert a TypeAnnotation to a human-readable string.
fn type_to_string(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Named {
            name, type_args, ..
        } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                let args: Vec<_> = type_args.iter().map(type_to_string).collect();
                format!("{}<{}>", name, args.join(", "))
            }
        }
        TypeAnnotation::Optional { base, .. } => format!("{}?", type_to_string(base)),
        TypeAnnotation::Union { types, .. } => {
            let t: Vec<_> = types.iter().map(type_to_string).collect();
            t.join(" | ")
        }
        TypeAnnotation::Fn {
            params,
            return_type,
            ..
        } => {
            let p: Vec<_> = params.iter().map(type_to_string).collect();
            format!("fn({}) -> {}", p.join(", "), type_to_string(return_type))
        }
    }
}

/// Check if a CSS property name is non-interpolatable (cannot be animated).
fn is_non_interpolatable(prop: &str) -> bool {
    matches!(
        prop,
        "display"
            | "position"
            | "overflow"
            | "flex-direction"
            | "flexDirection"
            | "grid-template"
            | "gridTemplate"
            | "visibility"
            | "content"
            | "cursor"
            | "pointer-events"
            | "pointerEvents"
            | "user-select"
            | "userSelect"
            | "font-family"
            | "fontFamily"
            | "text-decoration"
            | "textDecoration"
            | "white-space"
            | "whiteSpace"
    )
}

/// Collect all motion/timeline names referenced by a timeline definition.
fn collect_timeline_refs(timeline: &TimelineDef) -> Vec<&str> {
    timeline
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            TimelineItemKind::MotionRef(name) => Some(name.as_str()),
            TimelineItemKind::Stagger(stagger) => {
                if let Expr::VarRef(name, _) = stagger.motion.as_ref() {
                    Some(name.as_str())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

/// Extension trait to get span from TypeAnnotation.
impl TypeAnnotation {
    pub fn span(&self) -> Span {
        match self {
            TypeAnnotation::Named { span, .. } => span.clone(),
            TypeAnnotation::Optional { span, .. } => span.clone(),
            TypeAnnotation::Union { span, .. } => span.clone(),
            TypeAnnotation::Fn { span, .. } => span.clone(),
        }
    }
}
