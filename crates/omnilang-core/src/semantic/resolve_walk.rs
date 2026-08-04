//! Pass 2 (merged): module-level resolve orchestration — walks every module,
//! component, store, function, and design block, dispatching into the
//! expression walk ([`super::resolve_expr`]) and view-tree walk
//! ([`super::resolve_view`]) and running the post-walk validations.

use std::fmt::Write as _;

use super::resolve::check_token_type_compatibility;
use super::symbols::SymbolKind;
use super::validate_inline::ValidationContext;
use super::SemanticAnalyzer;
use crate::error::suggest_similar_name;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Walk all expressions in a module and check VarRef names resolve.
    /// Pushes the same scope hierarchy as `collect_symbols` so resolution works.
    ///
    /// Control-flow validation (break/continue/await) runs inline during the walk.
    /// View-tree validations (bind targets, store action refs, prop assignments)
    /// are called after the component's resolve walk finishes.
    pub(super) fn resolve_and_validate_module(&mut self, module: &Module) {
        // Component tags known in this module — element tags matching one of
        // these are component instantiations (slot-forwarding detection).
        // Imported names are included so cross-file component references
        // (`import { Modal } from "./Modal.omni"` + `<Modal>{slot}</Modal>`)
        // don't escape the check (deferral #21).
        self.module_component_names = module.components.iter().map(|c| c.name.clone()).collect();
        for import in &module.imports {
            if let ImportStmt::Named(ni) = import {
                self.module_component_names.extend(ni.names.iter().cloned());
            }
        }
        self.inside_component_children = None;
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

        // Resolve variable references in design token values
        for design in &module.designs {
            for token in &design.tokens {
                match &token.value {
                    TokenValue::Variable(name) => {
                        if !self.symbols.contains(name) {
                            let all_names = self.symbols.all_visible_names();
                            let known_names: Vec<&str> =
                                all_names.iter().map(|s| s.as_str()).collect();
                            let suggestion = suggest_similar_name(name, &known_names);
                            let mut msg = format!(
                                "Undefined variable '{}' in design token '{}'",
                                name, token.name
                            );
                            if let Some(sug) = suggestion {
                                let _ = write!(msg, ". Did you mean '{}'?", sug);
                            }
                            msg.push_str(
                                ". Variable references in design token values must use \
                                 compile-time constants (PascalCase or UPPER_SNAKE_CASE). \
                                 Lowercase names are treated as CSS values.",
                            );
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0001,
                                &msg,
                                &Span {
                                    start: 0,
                                    end: 0,
                                    line: 0,
                                    col: 0,
                                },
                            ));
                        } else if let Some(sym) = self.symbols.resolve(name) {
                            if sym.kind != SymbolKind::Const
                                // Allow Unknown — cross-file imported symbols may not have their
                                // final kind resolved yet at design token validation time.
                                && sym.kind != SymbolKind::Unknown
                            {
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0001,
                                    &format!(
                                        "'{}' is a {} in design token '{}', not a \
                                         compile-time constant. Only const declarations \
                                         can be used as direct variable references. \
                                         For runtime values, use dotted expression \
                                         syntax: `token {}: {} = {}.{};`",
                                        name,
                                        format!("{:?}", sym.kind).to_lowercase(),
                                        token.name,
                                        token.name,
                                        format!("{:?}", token.token_type).to_lowercase(),
                                        name,
                                        name,
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
                        // Warn for cross-file imports whose kind hasn't been resolved yet
                        if let Some(sym) = self.symbols.resolve(name) {
                            if sym.kind == SymbolKind::Unknown {
                                self.diagnostics.push(Diagnostic::warning(
                                    DiagnosticKind::E0001,
                                    &format!(
                                        "Symbol '{}' referenced in design token '{}' was imported \
                                         from another file and its kind could not be verified at \
                                         compile time. Ensure '{}' is declared as a const in its \
                                         source file.",
                                        name, token.name, name
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
                    TokenValue::Expression(expr) => {
                        let mut vctx = ValidationContext::new();
                        self.resolve_expr_refs(expr, &mut vctx);

                        // Phase 2: Check expression type is compatible with token type
                        check_token_type_compatibility(
                            expr,
                            &token.token_type,
                            &token.name,
                            &self.symbols,
                            self.symbols.current_scope_index(),
                            &mut self.diagnostics,
                        );

                        // Phase 2 restriction: dynamic tokens cannot have theme overrides
                        if !token.theme_values.is_empty() {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0001,
                                &format!(
                                    "Dynamic token '{}' cannot have theme overrides. \
                                     Use a static value for theme-aware tokens, or handle \
                                     theme switching in the store signal.",
                                    token.name
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
                    TokenValue::Literal(_) => {}
                }
                // Also check theme_values
                for (theme_name, theme_val) in &token.theme_values {
                    match theme_val {
                        TokenValue::Variable(name) => {
                            if !self.symbols.contains(name) {
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0001,
                                    &format!(
                                        "Undefined variable '{}' in theme '{}' of design token '{}'",
                                        name, theme_name, token.name
                                    ),
                                    &Span {
                                        start: 0,
                                        end: 0,
                                        line: 0,
                                        col: 0,
                                    },
                                ));
                            } else if let Some(sym) = self.symbols.resolve(name) {
                                if sym.kind != SymbolKind::Const
                                    // Allow Unknown — cross-file imported symbols may not have
                                    // their final kind resolved yet at design token validation
                                    // time.
                                    && sym.kind != SymbolKind::Unknown
                                {
                                    self.diagnostics.push(Diagnostic::error(
                                        DiagnosticKind::E0001,
                                        &format!(
                                            "'{}' is a {} in theme '{}' of design token '{}', \
                                             not a compile-time constant. Theme override values \
                                             must use const declarations or static literals.",
                                            name,
                                            format!("{:?}", sym.kind).to_lowercase(),
                                            theme_name,
                                            token.name,
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
                        TokenValue::Expression(expr) => {
                            // Phase 2 restriction: dynamic expressions are not supported in theme
                            // overrides because neither CSS [data-theme] selectors nor JS codegen
                            // has a path for per-theme dynamic values.
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0001,
                                &format!(
                                    "Theme value '{}' for token '{}' cannot be a dynamic expression. \
                                     Theme overrides must use static values (literals or compile-time constants).",
                                    theme_name, token.name
                                ),
                                &Span { start: 0, end: 0, line: 0, col: 0 },
                            ));
                            // Still resolve refs so other diagnostics (e.g., undefined variables)
                            // within the expression are also emitted.
                            let mut vctx = ValidationContext::new();
                            self.resolve_expr_refs(expr, &mut vctx);
                        }
                        TokenValue::Literal(_) => {}
                    }
                }
            }
        }

        // Motion validation (still a standalone call, not inlined)
        self.validate_motions_in_module(module);
    }

    /// Find a scope by name (hash lookup, O(1) average).
    fn find_scope(&self, name: &str) -> Option<usize> {
        self.symbols.find_scope_by_name(name)
    }

    /// Walk up the scope chain to find the enclosing function name (e.g. "main").
    pub(super) fn find_enclosing_fn_name(&self) -> Option<String> {
        let mut scope_idx = self.symbols.current_scope_index();
        loop {
            let cur_scope = self.symbols.get_scope(scope_idx)?;
            if cur_scope.name.starts_with("fn:") {
                return Some(cur_scope.name[3..].to_string());
            }
            scope_idx = cur_scope.parent?;
        }
    }
}
