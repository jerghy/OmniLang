//! Remaining standalone validations (bind targets, store actions, prop assignment).
//!
//! Control-flow validation (`break`/`continue`/`await`) was merged inline into the
//! resolve pass (see `validate_inline.rs` and `resolve.rs`).  Motion validation
//! lives in the sibling [`validate_motion`] module.

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use super::walk_view::walk_view_nodes;
use super::SemanticAnalyzer;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;

// ── Bind target validation ──────────────────────────

impl SemanticAnalyzer {
    /// bind: must target a signal, not a store action or fn
    pub(super) fn validate_bind_targets(&mut self, comp: &ComponentDef) {
        let signal_names: FxHashSet<&str> = comp.signals.iter().map(|s| s.name.as_str()).collect();

        if let Some(ref view) = comp.view {
            walk_view_nodes(&view.children, &mut |el| {
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
                            self.diagnostics.push(Diagnostic::warning(
                                DiagnosticKind::E0005,
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
            });
        }
    }
}

// ── Store action references ─────────────────────────

impl SemanticAnalyzer {
    /// Validate that EventHandler::StoreAction references point to real stores and actions.
    /// Triggers: E0007
    pub(super) fn validate_store_action_refs(&mut self, comp: &ComponentDef, module: &Module) {
        // Build a map: store_name -> set of action names
        let mut store_actions: FxHashMap<&str, FxHashSet<&str>> = FxHashMap::default();
        for store in &module.stores {
            let actions: FxHashSet<&str> = store.actions.iter().map(|a| a.name.as_str()).collect();
            store_actions.insert(store.name.as_str(), actions);
        }

        let store_refs: FxHashSet<&str> = comp.store_refs.iter().map(|s| s.as_str()).collect();

        if let Some(ref view) = comp.view {
            walk_view_nodes(&view.children, &mut |el| {
                for prop in &el.props {
                    if let ElementProp::Event {
                        handler: EventHandler::StoreAction { store, action },
                        ..
                    } = prop
                    {
                        // Check @store declaration
                        if !store_refs.contains(store.as_str()) {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0007,
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
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0007,
                                    &format!("Store '{}' has no action '{}'", store, action),
                                    &el.span,
                                ));
                            }
                        } else {
                            // Store not found at all
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0007,
                                &format!("Store '{}' not found", store),
                                &el.span,
                            ));
                        }
                    }
                }
            });
        }
    }
}

// ── Prop assignment ─────────────────────────────────

impl SemanticAnalyzer {
    /// Validate that component props are not assigned to within the component.
    pub(super) fn validate_prop_assignment_in_component(&mut self, comp: &ComponentDef) {
        let prop_names: FxHashSet<&str> = comp.props.iter().map(|p| p.name.as_str()).collect();

        if prop_names.is_empty() {
            return;
        }

        fn check_block(
            block: &Block,
            prop_names: &FxHashSet<&str>,
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
                            DiagnosticKind::E0009,
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
            walk_view_nodes(&view.children, &mut |el| {
                for prop in &el.props {
                    if let ElementProp::Event {
                        handler: EventHandler::InlineFn { body, .. },
                        ..
                    } = prop
                    {
                        check_block(body, &prop_names, &mut self.diagnostics);
                    }
                }
            });
        }
    }
}
