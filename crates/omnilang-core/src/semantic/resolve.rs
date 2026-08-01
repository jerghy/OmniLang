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

use std::fmt::Write as _;

use super::helpers::type_to_string;
use super::symbols::{Symbol, SymbolKind, SymbolTable};
use super::types::SemanticType;
use super::validate_inline::{check_control_flow, check_expr_await, ValidationContext};
use super::validate_types::{
    check_assignment, check_binary_op, check_call, check_return, infer_expr_type,
};
use super::SemanticAnalyzer;
use crate::error::suggest_similar_name;
use crate::error::Diagnostic;
use crate::error::DiagnosticKind;
use crate::lexer::Span;
use crate::parser::ast::*;

/// Check that an expression's inferred type is compatible with the design token type.
///
/// Compatibility rules (Phase 2):
/// - `String` / `Any` (unknown) → always compatible (all token types accept CSS string values)
/// - `Int` / `Float` → compatible with Duration, Opacity, ZIndex, Size, LineHeight, FontWeight
/// - Everything else → type mismatch diagnostic
fn check_token_type_compatibility(
    expr: &Expr,
    token_type: &TokenType,
    token_name: &str,
    symbols: &SymbolTable,
    scope: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let inferred = infer_token_expr_type(expr, symbols, scope);

    // String and Any are always compatible — CSS custom properties accept any string
    if matches!(inferred, SemanticType::String | SemanticType::Any) {
        return;
    }

    // Numeric types are compatible with numeric-accepting token types
    if matches!(inferred, SemanticType::Int | SemanticType::Float) {
        let numeric_ok = matches!(
            token_type,
            TokenType::Duration
                | TokenType::Opacity
                | TokenType::ZIndex
                | TokenType::Size
                | TokenType::LineHeight
                | TokenType::FontWeight
        );
        if numeric_ok {
            return;
        }
    }

    // Type mismatch — emit diagnostic
    let type_name = match inferred {
        SemanticType::Int => "int",
        SemanticType::Float => "float",
        SemanticType::Bool => "bool",
        SemanticType::Void => "void",
        SemanticType::Null => "null",
        SemanticType::Named(ref n) => n.as_str(),
        SemanticType::Array(_) => "array",
        SemanticType::Map(..) => "map",
        SemanticType::Optional(_) => "optional",
        SemanticType::Union(_) => "union",
        SemanticType::Fn(..) => "function",
        SemanticType::Any | SemanticType::String => unreachable!(),
    };

    let token_type_str = token_type_name(token_type);

    diagnostics.push(Diagnostic::error(
        DiagnosticKind::E0010,
        &format!(
            "Type mismatch in design token '{}': expression has type '{}' \
             but token type '{}' expects a string value. \
             Wrap the value in a string or use a compatible signal type.",
            token_name, type_name, token_type_str
        ),
        &Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        },
    ));
}

/// Extended type inference for design token expressions.
///
/// Unlike `infer_expr_type()`, this resolves `FieldAccess` expressions
/// (e.g. `ThemeStore.primaryColor`) by looking up the store signal's type
/// rather than returning `Any`.
fn infer_token_expr_type(expr: &Expr, symbols: &SymbolTable, scope: usize) -> SemanticType {
    match expr {
        Expr::FieldAccess { object, field, .. } => {
            // Resolve object → store name → signal type
            if let Expr::VarRef(store_name, _) = object.as_ref() {
                // Look up the store signal: store scope is "store:<StoreName>"
                let store_scope_name = format!("store:{}", store_name);
                if let Some(store_scope_idx) = symbols.find_scope_by_name(&store_scope_name) {
                    if let Some(scope) = symbols.get_scope(store_scope_idx) {
                        if let Some(sig_symbol) = scope.symbols.get(field) {
                            if let Some(type_str) = &sig_symbol.type_ann {
                                return SemanticType::parse_type_string(type_str);
                            }
                        }
                    }
                }
            }
            SemanticType::Any
        }
        _ => infer_expr_type(expr, symbols, scope),
    }
}

/// Human-readable name for a design token type (for diagnostic messages).
fn token_type_name(tt: &TokenType) -> &'static str {
    match tt {
        TokenType::Color => "color",
        TokenType::Size => "size",
        TokenType::Radius => "radius",
        TokenType::FontFamily => "fontFamily",
        TokenType::FontSize => "fontSize",
        TokenType::FontWeight => "fontWeight",
        TokenType::LineHeight => "lineHeight",
        TokenType::LetterSpacing => "letterSpacing",
        TokenType::Shadow => "shadow",
        TokenType::Duration => "duration",
        TokenType::Easing => "easing",
        TokenType::Opacity => "opacity",
        TokenType::ZIndex => "zIndex",
    }
}

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
        self.module_component_names = module
            .components
            .iter()
            .map(|c| c.name.clone())
            .collect();
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
    fn find_enclosing_fn_name(&self) -> Option<String> {
        let mut scope_idx = self.symbols.current_scope_index();
        loop {
            let cur_scope = self.symbols.get_scope(scope_idx)?;
            if cur_scope.name.starts_with("fn:") {
                return Some(cur_scope.name[3..].to_string());
            }
            scope_idx = cur_scope.parent?;
        }
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
                span,
            } => {
                // Declare the variable only if it hasn't already been collected
                // by Pass 1 (collect_block_symbols). This avoids duplicate warnings
                // for named scopes (fn:name, action:name) while still allowing
                // declaration in dynamic scopes (block_expr, inline_handler, closure)
                // where collect can't reliably match scope indices across passes.
                if !self.symbols.is_in_current_scope(name) {
                    self.declare_or_warn(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: type_ann.as_ref().map(type_to_string),
                        mutable: *mutable,
                        declared_at: 0,
                    });
                }
                if let Some(e) = init {
                    self.resolve_expr_refs(e, vctx);
                    // Type-check: initializer must match declared type
                    if let Some(ref ta) = type_ann {
                        let scope = self.symbols.current_scope_index();
                        let expected = SemanticType::from_type_annotation(ta);
                        let actual = infer_expr_type(e, &self.symbols, scope);
                        if let Err(msg) = check_assignment(&expected, &actual) {
                            self.diagnostics.push(
                                Diagnostic::error(DiagnosticKind::E0010, &msg, span)
                                    .with_suggestion(&format!(
                                        "expected {}, got {}",
                                        expected, actual
                                    )),
                            );
                        }
                    }
                }
            }
            Stmt::ExprStmt { expr, .. } => self.resolve_expr_refs(expr, vctx),
            Stmt::Return {
                value: Some(e),
                span,
            } => {
                self.resolve_expr_refs(e, vctx);
                // Type-check: return value must match function return type
                if let Some(fn_name) = self.find_enclosing_fn_name() {
                    let scope = self.symbols.current_scope_index();
                    let ret_type_str = self
                        .symbols
                        .resolve(&fn_name)
                        .and_then(|s| s.type_ann.clone());
                    if let Some(ref ret_type_str) = ret_type_str {
                        let expected = SemanticType::parse_type_string(ret_type_str);
                        let actual = infer_expr_type(e, &self.symbols, scope);
                        if let Err(msg) = check_return(&expected, &actual) {
                            self.diagnostics.push(
                                Diagnostic::error(DiagnosticKind::E0010, &msg, span)
                                    .with_suggestion(&format!(
                                        "expected {}, got {}",
                                        expected, actual
                                    )),
                            );
                        }
                    }
                }
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
                // Declare catch_var only if not already collected by Pass 1
                if let Some(ref var) = catch_var {
                    if !self.symbols.is_in_current_scope(var) {
                        self.declare_or_warn(Symbol {
                            name: var.clone(),
                            kind: SymbolKind::Variable,
                            type_ann: None,
                            mutable: false,
                            declared_at: 0,
                        });
                    }
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
                    let all_names = self.symbols.all_visible_names();
                    let known_names: Vec<&str> = all_names.iter().map(|s| s.as_str()).collect();
                    let suggestion = suggest_similar_name(name, &known_names)
                        .map(|s| format!("did you mean '{}'?", s));
                    let mut diag = Diagnostic::error(
                        DiagnosticKind::E0001,
                        &format!("Undeclared variable '{}'", name),
                        span,
                    );
                    if let Some(ref sug) = suggestion {
                        diag = diag.with_suggestion(sug);
                    }
                    self.diagnostics.push(diag);
                }
                // Slot forwarding (ADR-053 D5): a `{slot}` interpolation
                // inside the children of a component instantiation would be
                // materialized into a ViewTree::Slot and treated by codegen
                // as a projection INTO that component — the forwarded
                // content never reaches the child. Warn (not error —
                // forwarding may become supported later).
                if self.resolving_interpolation {
                    if let Some(ref comp_tag) = self.inside_component_children {
                        if let Some(sym) = self.symbols.resolve(name) {
                            if sym.kind == SymbolKind::Slot {
                                self.diagnostics.push(Diagnostic::warning(
                                    DiagnosticKind::E0006,
                                    &format!(
                                        "slot '{}' is referenced inside the children of \
                                         component '{}' — slot forwarding to a child \
                                         component is not supported yet; the content will \
                                         not be passed to '{}'",
                                        name, comp_tag, comp_tag
                                    ),
                                    span,
                                ));
                            }
                        }
                    }
                }
            }
            Expr::BinaryOp {
                left,
                right,
                op,
                span,
            } => {
                self.resolve_expr_refs(left, vctx);
                self.resolve_expr_refs(right, vctx);
                // Type-check: binary operand types must match
                let scope = self.symbols.current_scope_index();
                let lhs = infer_expr_type(left, &self.symbols, scope);
                let rhs = infer_expr_type(right, &self.symbols, scope);
                if let Err(msg) = check_binary_op(op, &lhs, &rhs) {
                    self.diagnostics.push(
                        Diagnostic::error(DiagnosticKind::E0010, &msg, span).with_suggestion(
                            &format!("expected compatible types, got {} and {}", lhs, rhs),
                        ),
                    );
                }
            }
            Expr::UnaryOp { operand, .. } => self.resolve_expr_refs(operand, vctx),
            Expr::FuncCall { func, args, span } => {
                self.resolve_expr_refs(func, vctx);
                for a in args {
                    self.resolve_expr_refs(a, vctx);
                }

                // Validate function argument types against parameter types
                if let Expr::VarRef(fn_name, _) = func.as_ref() {
                    if let Some(fn_sym) = self.symbols.resolve(fn_name) {
                        if fn_sym.kind == SymbolKind::Function || fn_sym.kind == SymbolKind::Action
                        {
                            // Collect parameter types from the fn:<name> scope
                            let fn_scope_name = format!(
                                "{}:{}",
                                if fn_sym.kind == SymbolKind::Action {
                                    "action"
                                } else {
                                    "fn"
                                },
                                fn_name
                            );
                            if let Some(scope_idx) = self.symbols.find_scope_by_name(&fn_scope_name)
                            {
                                let mut param_symbols: Vec<&Symbol> = self
                                    .symbols
                                    .get_scope(scope_idx)
                                    .map(|scope| {
                                        scope
                                            .symbols
                                            .values()
                                            .filter(|s| s.kind == SymbolKind::Param)
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                // Sort by declared_at to preserve parameter declaration order
                                param_symbols.sort_by_key(|s| s.declared_at);
                                let param_types: Vec<SemanticType> = param_symbols
                                    .iter()
                                    .map(|s| {
                                        s.type_ann
                                            .as_ref()
                                            .map(|t| SemanticType::parse_type_string(t))
                                            .unwrap_or(SemanticType::Any)
                                    })
                                    .collect();

                                // Infer argument types
                                let scope = self.symbols.current_scope_index();
                                let arg_types: Vec<SemanticType> = args
                                    .iter()
                                    .map(|a| infer_expr_type(a, &self.symbols, scope))
                                    .collect();

                                // Get declared return type from function symbol
                                let return_type = fn_sym
                                    .type_ann
                                    .as_ref()
                                    .map(|t| SemanticType::parse_type_string(t))
                                    .unwrap_or(SemanticType::Void);

                                if let Err(msg) =
                                    check_call(fn_name, &param_types, &arg_types, &return_type)
                                {
                                    let arg_types_str = arg_types
                                        .iter()
                                        .map(|t| t.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let param_types_str = param_types
                                        .iter()
                                        .map(|t| t.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    self.diagnostics.push(
                                        Diagnostic::error(DiagnosticKind::E0010, &msg, span)
                                            .with_suggestion(&format!(
                                                "expected [{}], got [{}]",
                                                param_types_str, arg_types_str
                                            )),
                                    );
                                }
                            }
                        }
                    }
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
                // Push a proper closure scope so params don't leak into the
                // enclosing function scope (matching collect pass behavior)
                self.symbols.push_scope("closure");
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
                self.symbols.pop_scope();
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
                // The driving signal expression is resolved like any other
                // prop expression (ref-existence validation is Phase 2.1).
                ElementProp::RefBinding { signal, .. } => {
                    self.resolve_expr_refs(signal, vctx);
                }
                // Named slot projection content (ADR-053 D5) is a view
                // subtree — resolve references inside it like children.
                ElementProp::SlotContent { content, .. } => {
                    for node in content {
                        self.resolve_view_node_refs(node, vctx);
                    }
                }
                ElementProp::Bind { .. }
                | ElementProp::TestId(_)
                | ElementProp::Ref(_)
                | ElementProp::MotionTrigger(_)
                | ElementProp::Variant { .. } => {}
            }
        }
        // Children of a component instantiation are consumer-side
        // projections — a `{slot}` interpolation among them is an attempt
        // to forward this component's slot into the child component, which
        // codegen cannot do today (the materialized slot would be treated
        // as the child's own projection). Track the position so
        // resolve_expr_refs can warn.
        let is_component = self
            .module_component_names
            .contains(element.tag.as_str());
        let saved = self.inside_component_children.take();
        if is_component {
            self.inside_component_children = Some(element.tag.clone());
        }
        for child in &element.children {
            self.resolve_view_node_refs(child, vctx);
        }
        self.inside_component_children = saved;
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
