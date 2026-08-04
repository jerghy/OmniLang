//! Pass 2 (merged): statement + expression walking — resolves every
//! `VarRef`, runs inline control-flow validation (break/continue/await)
//! and the bottom-up type checks (binary ops, assignments, calls,
//! returns) from [`super::validate_types`].

use super::helpers::type_to_string;
use super::symbols::{Symbol, SymbolKind};
use super::types::SemanticType;
use super::validate_inline::{check_control_flow, check_expr_await, ValidationContext};
use super::validate_types::{
    check_assignment, check_binary_op, check_call, check_method_name, check_return, infer_expr_type,
};
use super::SemanticAnalyzer;
use crate::error::suggest_similar_name;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    pub(super) fn resolve_block_refs(&mut self, block: &Block, vctx: &mut ValidationContext) {
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

    pub(super) fn resolve_expr_refs(&mut self, expr: &Expr, vctx: &mut ValidationContext) {
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
            Expr::MethodCall {
                object,
                method,
                args,
                span,
            } => {
                self.resolve_expr_refs(object, vctx);
                for a in args {
                    self.resolve_expr_refs(a, vctx);
                }
                // Stdlib method-name validation (Task 4.10, ai LOW): a
                // method unknown on the receiver's concrete type
                // (String/i32/... ) warns with the nearest stdlib suggestion
                // — previously these compiled clean ("No issues found") and
                // only failed at runtime (s.toUpper() → TypeError).
                // Receivers with their own method surface (signals, refs,
                // stores, event params) and Any-typed receivers are skipped
                // — no cascade.
                let receiver_has_own_surface = matches!(
                    object.as_ref(),
                    Expr::VarRef(name, _)
                        if matches!(
                            self.symbols.resolve(name).map(|s| s.kind.clone()),
                            Some(
                                SymbolKind::Signal
                                    | SymbolKind::Ref
                                    | SymbolKind::Store
                                    | SymbolKind::Event
                            )
                        )
                );
                if !receiver_has_own_surface {
                    let scope = self.symbols.current_scope_index();
                    let receiver_type = infer_expr_type(object, &self.symbols, scope);
                    if let Some(msg) = check_method_name(&receiver_type, method) {
                        self.diagnostics.push(Diagnostic::warning(
                            DiagnosticKind::E0006,
                            &msg,
                            span,
                        ));
                    }
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
