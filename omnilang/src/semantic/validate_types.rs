//! Pass 3 (dependencies) and Pass 4 (type validation) for semantic analysis.
//!
//! Validates control flow correctness (`break`/`continue`/`await` scoping),
//! bind targets, store action references, store field access, and prop
//! assignment prohibition.  Motion-specific validation lives in the sibling
//! [`validate_motion`] module.

use std::collections::{HashMap, HashSet};

use super::SemanticAnalyzer;
use crate::error::Diagnostic;
use crate::lexer::Span;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    // ── Pass 3: Collect dependencies ──────

    /// Collect per-component dependency lists (currently just computed names).
    pub(super) fn collect_module_dependencies(&mut self, module: &Module) {
        for comp in &module.components {
            let mut deps = Vec::new();
            for c in &comp.computeds {
                deps.push(c.name.clone());
            }
            self.dependencies.insert(comp.name.clone(), deps);
        }
    }

    // ── Pass 4: Validate ───────────────────────────────

    /// Run all validation rules for a module.
    pub(super) fn validate_module(&mut self, module: &Module) {
        for comp in &module.components {
            self.validate_control_flow_in_component(comp);
            self.validate_bind_targets(comp);
            self.validate_store_action_refs(comp, module);
            self.validate_store_field_access(comp);
            self.validate_prop_assignment_in_component(comp);
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
}

// ── Bind target validation ──────────────────────────

impl SemanticAnalyzer {
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
}

// ── Store action references ─────────────────────────

impl SemanticAnalyzer {
    /// Validate that EventHandler::StoreAction references point to real stores and actions.
    /// Triggers: E0007
    fn validate_store_action_refs(&mut self, comp: &ComponentDef, module: &Module) {
        // Build a map: store_name -> set of action names
        let mut store_actions: HashMap<&str, HashSet<&str>> = HashMap::new();
        for store in &module.stores {
            let actions: HashSet<&str> = store.actions.iter().map(|a| a.name.as_str()).collect();
            store_actions.insert(store.name.as_str(), actions);
        }

        let store_refs: HashSet<&str> = comp.store_refs.iter().map(|s| s.as_str()).collect();

        fn walk_nodes(
            nodes: &[ViewNode],
            store_actions: &HashMap<&str, HashSet<&str>>,
            store_refs: &HashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            for node in nodes {
                match node {
                    ViewNode::Element(el) => {
                        for prop in &el.props {
                            if let ElementProp::Event { handler, .. } = prop {
                                if let EventHandler::StoreAction { store, action } = handler {
                                    // Check @store declaration
                                    if !store_refs.contains(store.as_str()) {
                                        diagnostics.push(Diagnostic::error(
                                            "E0007",
                                            &format!(
                                                "Store '{}' is not declared with @store in this \
                                                 component — add '@store {};' to the component body",
                                                store, store
                                            ),
                                            &el.span,
                                        ));
                                    }
                                    // Check action exists
                                    if let Some(actions) = store_actions.get(store.as_str()) {
                                        if !actions.contains(action.as_str()) {
                                            diagnostics.push(Diagnostic::error(
                                                "E0007",
                                                &format!(
                                                    "Store '{}' has no action '{}'",
                                                    store, action
                                                ),
                                                &el.span,
                                            ));
                                        }
                                    } else {
                                        // Store not found at all
                                        diagnostics.push(Diagnostic::error(
                                            "E0007",
                                            &format!("Store '{}' not found", store),
                                            &el.span,
                                        ));
                                    }
                                }
                            }
                        }
                        walk_nodes(&el.children, store_actions, store_refs, diagnostics);
                    }
                    ViewNode::IfDirective(if_dir) => {
                        walk_nodes(&if_dir.body, store_actions, store_refs, diagnostics);
                        for (_, body) in &if_dir.else_ifs {
                            walk_nodes(body, store_actions, store_refs, diagnostics);
                        }
                        if let Some(ref else_body) = if_dir.else_body {
                            walk_nodes(else_body, store_actions, store_refs, diagnostics);
                        }
                    }
                    ViewNode::ForDirective(for_dir) => {
                        walk_nodes(&for_dir.body, store_actions, store_refs, diagnostics);
                        if let Some(ref empty_body) = for_dir.empty_body {
                            walk_nodes(empty_body, store_actions, store_refs, diagnostics);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(ref view) = comp.view {
            walk_nodes(
                &view.children,
                &store_actions,
                &store_refs,
                &mut self.diagnostics,
            );
        }
    }
}

// ── Store field access ──────────────────────────────

impl SemanticAnalyzer {
    /// Validate that store fields are only accessed with @store declaration.
    /// Triggers: E0008
    fn validate_store_field_access(&mut self, comp: &ComponentDef) {
        let store_refs: HashSet<&str> = comp.store_refs.iter().map(|s| s.as_str()).collect();

        // Collect all known store names from the module
        // (we check against store_refs since we're inside validate_module)

        // Walk function bodies looking for StoreFieldAccess or FieldAccess on store names
        fn check_expr_for_undeclared_store(
            expr: &Expr,
            store_refs: &HashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            match expr {
                Expr::StoreFieldAccess { store_alias, .. } => {
                    if !store_refs.contains(store_alias.as_str()) {
                        diagnostics.push(Diagnostic::error(
                            "E0008",
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
            store_refs: &HashSet<&str>,
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
            store_refs: &HashSet<&str>,
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
                            ElementProp::Class { value } => {
                                if let ClassValue::ClassMap(entries) = value {
                                    for (_, expr) in entries {
                                        check_expr_for_undeclared_store(
                                            expr,
                                            store_refs,
                                            diagnostics,
                                        );
                                    }
                                }
                            }
                            ElementProp::Event { handler, .. } => {
                                if let EventHandler::InlineFn { body, .. } = handler {
                                    for stmt in &body.stmts {
                                        check_stmt_for_undeclared_store(
                                            stmt,
                                            store_refs,
                                            diagnostics,
                                        );
                                    }
                                    if let Some(ref trailing) = body.trailing_expr {
                                        check_expr_for_undeclared_store(
                                            trailing,
                                            store_refs,
                                            diagnostics,
                                        );
                                    }
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

// ── Prop assignment ─────────────────────────────────

impl SemanticAnalyzer {
    /// Validate that component props are not assigned to within the component.
    fn validate_prop_assignment_in_component(&mut self, comp: &ComponentDef) {
        let prop_names: HashSet<&str> = comp.props.iter().map(|p| p.name.as_str()).collect();

        if prop_names.is_empty() {
            return;
        }

        fn check_block(
            block: &Block,
            prop_names: &HashSet<&str>,
            diagnostics: &mut Vec<Diagnostic>,
        ) {
            for stmt in &block.stmts {
                if let Stmt::Assign {
                    target: AssignTarget::Name(name),
                    span,
                    ..
                } = stmt
                {
                    if prop_names.contains(name.as_str()) {
                        diagnostics.push(Diagnostic::error(
                            "E0009",
                            &format!(
                                "Cannot assign to prop '{}' — component props are read-only. \
                                 Use a local signal if you need mutable state.",
                                name
                            ),
                            span,
                        ));
                    }
                }
                // Recurse into nested blocks
                match stmt {
                    Stmt::If {
                        then_body,
                        else_ifs,
                        else_body,
                        ..
                    } => {
                        check_block(then_body, prop_names, diagnostics);
                        for (_, body) in else_ifs {
                            check_block(body, prop_names, diagnostics);
                        }
                        if let Some(ref body) = else_body {
                            check_block(body, prop_names, diagnostics);
                        }
                    }
                    Stmt::While { body, .. } | Stmt::ForIn { body, .. } => {
                        check_block(body, prop_names, diagnostics);
                    }
                    Stmt::TryCatch {
                        try_body,
                        catch_body,
                        finally_body,
                        ..
                    } => {
                        check_block(try_body, prop_names, diagnostics);
                        if let Some(ref body) = catch_body {
                            check_block(body, prop_names, diagnostics);
                        }
                        if let Some(ref body) = finally_body {
                            check_block(body, prop_names, diagnostics);
                        }
                    }
                    _ => {}
                }
            }
        }

        for func in &comp.fns {
            check_block(&func.body, &prop_names, &mut self.diagnostics);
        }
        for action in &comp.actions {
            check_block(&action.body, &prop_names, &mut self.diagnostics);
        }
        for lc in &comp.lifecycles {
            check_block(&lc.body, &prop_names, &mut self.diagnostics);
        }
        // Also check view inline event handlers
        if let Some(ref view) = comp.view {
            fn check_view_nodes(
                nodes: &[ViewNode],
                prop_names: &HashSet<&str>,
                diagnostics: &mut Vec<Diagnostic>,
            ) {
                for node in nodes {
                    match node {
                        ViewNode::Element(el) => {
                            for prop in &el.props {
                                if let ElementProp::Event { handler, .. } = prop {
                                    if let EventHandler::InlineFn { body, .. } = handler {
                                        check_block(body, prop_names, diagnostics);
                                    }
                                }
                            }
                            check_view_nodes(&el.children, prop_names, diagnostics);
                        }
                        ViewNode::IfDirective(if_dir) => {
                            check_view_nodes(&if_dir.body, prop_names, diagnostics);
                            for (_, body) in &if_dir.else_ifs {
                                check_view_nodes(body, prop_names, diagnostics);
                            }
                            if let Some(ref else_body) = if_dir.else_body {
                                check_view_nodes(else_body, prop_names, diagnostics);
                            }
                        }
                        ViewNode::ForDirective(for_dir) => {
                            check_view_nodes(&for_dir.body, prop_names, diagnostics);
                            if let Some(ref empty_body) = for_dir.empty_body {
                                check_view_nodes(empty_body, prop_names, diagnostics);
                            }
                        }
                        _ => {}
                    }
                }
            }
            check_view_nodes(&view.children, &prop_names, &mut self.diagnostics);
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
}
