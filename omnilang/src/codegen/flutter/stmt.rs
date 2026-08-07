//! Statement and function generation for Flutter codegen.

use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashSet;
use std::fmt::Write;

// ═══════════════════════════════════════════════════════════════
// Statement generation (Flutter/Dart)
// ═══════════════════════════════════════════════════════════════

/// Shared Dart-family `let` statement formatting.
///
/// Task 5.1: `is_mut` selects the keyword — mutable → `var`, immutable →
/// `final` (previously always `var`, losing `final` for immutable bindings).
fn format_dart_let_stmt<F: FormatExpr>(
    expr: &F,
    w: &mut CodeWriter,
    name: &str,
    value: &Option<ExprIR>,
    type_ann: &Option<TypeIR>,
    is_mut: bool,
) {
    let rhs = value
        .as_ref()
        .map(|v| expr.format_expr(v))
        .unwrap_or_else(|| "null".into());
    if let Some(ref ty) = type_ann {
        let ann = super::super::dart::dart_type(ty.kind.as_str(), &ty.type_args);
        w.emit_fmt(format_args!("{ann} {name} = {rhs};"));
    } else {
        let kw = if is_mut { "var" } else { "final" };
        w.emit_fmt(format_args!("{kw} {name} = {rhs};"));
    }
}

/// Component-mode Flutter statement formatter — wraps mutations in `setState`.
pub(crate) struct FlutterStmtFormatter<F: FormatExpr> {
    pub(crate) expr: F,
    /// Phase 3 (ADR-071 decision 2): D1/D2 signal names — mutations write
    /// `_name.value` directly, bypassing the setState wrapper (zero
    /// whole-tree rebuild). Filled from the shared classification at
    /// construction sites with component context; empty elsewhere.
    pub(crate) direct_write_signals: FxHashSet<String>,
}

impl<F: FormatExpr> FormatStmt for FlutterStmtFormatter<F> {
    type ExprFormatter = F;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "var"
    }

    fn for_in_uses_of(&self) -> bool {
        false
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
        is_mut: bool,
    ) {
        format_dart_let_stmt(&self.expr, w, name, value, type_ann, is_mut);
    }

    /// Component-local mutations target the plain field name.
    fn mutation_target_expr(&self, target: &str) -> String {
        target.to_string()
    }

    /// Component-local mutations are wrapped in setState.
    fn mutation_stmt_prefix(&self) -> Option<&str> {
        Some("setState(() {")
    }

    fn mutation_stmt_suffix(&self) -> Option<&str> {
        Some("});")
    }

    /// The language's `list.push(x)` maps to Dart's `List.add` (Task 5.8).
    fn mutation_push_method(&self) -> &'static str {
        "add"
    }

    /// The language's `list.pop()` maps to Dart's `List.removeLast`.
    fn mutation_pop_method(&self) -> &'static str {
        "removeLast"
    }

    /// Event emits (`onSwitch("overview")`) forward to the widget's callback
    /// field — `widget.onSwitch?.call("overview")` (Task 5.8: bare calls
    /// produced `undefined method 'onSwitch'` on the State class).
    fn format_event_emit_stmt(&self, w: &mut CodeWriter, event_name: &str, args: &[ExprIR]) {
        let arg_str = if args.is_empty() {
            String::from("")
        } else {
            let formatted: Vec<String> = args
                .iter()
                .map(|a| self.expr_fmt().format_expr(a))
                .collect();
            formatted.join(", ")
        };
        w.emit_fmt(format_args!("widget.{event_name}?.call({arg_str});"));
    }

    /// Mutations whose value expression contains `await` cannot be wrapped in
    /// the synchronous `setState(() { ... })` closure — hoist the await out:
    /// `var __v = await expr; setState(() { target = __v; });` (Task 5.8:
    /// StdLibShowcase's `sleepResult = await demo_sleep_and_return();`
    /// produced `await_in_wrong_context` inside the closure).
    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        // Phase 3 (ADR-071 decision 2): D1/D2 signals write the ValueNotifier
        // directly — `_name.value = ...`, NO setState, no whole-tree build.
        // Every mutation form must use the `.value` target: the State field
        // was RENAMED `_name` (bare `name += 1` would be an undeclared
        // identifier). Runs BEFORE the await-hoisting branch — a direct write
        // has no setState closure, so an await-bearing RHS needs no hoist.
        if let Some(target) = mutation_target_name(intent) {
            if self.direct_write_signals.contains(target) {
                let semi = if self.needs_semicolon() { ";" } else { "" };
                match intent {
                    MutationIntent::DirectAssign { value, .. } => {
                        let amount = value
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "0".into());
                        w.emit_fmt(format_args!("_{}.value = {amount}{semi}", target));
                    }
                    MutationIntent::Increment { amount, .. } => {
                        let amt = amount
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "1".into());
                        w.emit_fmt(format_args!("_{}.value += {amt}{semi}", target));
                    }
                    MutationIntent::Decrement { amount, .. } => {
                        let amt = amount
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "1".into());
                        w.emit_fmt(format_args!("_{}.value -= {amt}{semi}", target));
                    }
                    MutationIntent::Multiply { amount, .. } => {
                        let amt = amount
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "2".into());
                        w.emit_fmt(format_args!("_{}.value *= {amt}{semi}", target));
                    }
                    MutationIntent::Divide { amount, .. } => {
                        let amt = amount
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "2".into());
                        w.emit_fmt(format_args!("_{}.value /= {amt}{semi}", target));
                    }
                    MutationIntent::Modulo { amount, .. } => {
                        let amt = amount
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "0".into());
                        w.emit_fmt(format_args!("_{}.value %= {amt}{semi}", target));
                    }
                    MutationIntent::Push { item, .. } => {
                        let it = item
                            .as_ref()
                            .map(|v| self.expr_fmt().format_expr(v))
                            .unwrap_or_else(|| "null".into());
                        // Reassign the notifier cell (spread copy) so
                        // listeners are notified — in-place `add` would not.
                        w.emit_fmt(format_args!(
                            "_{}.value = [..._{}.value, {}]{semi}",
                            target, target, it
                        ));
                    }
                    MutationIntent::Pop { .. } => {
                        w.emit_fmt(format_args!(
                            "_{}.value = _{}.value.sublist(0, _{}.value.length - 1){semi}",
                            target, target, target
                        ));
                    }
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
                return;
            }
        }
        let value_has_await = match intent {
            MutationIntent::DirectAssign { value, .. } => {
                value.as_deref().is_some_and(expr_emits_await)
            }
            MutationIntent::Push { item, .. } => item.as_deref().is_some_and(expr_emits_await),
            _ => false,
        };
        if value_has_await {
            // DirectAssign: hoist the await before the setState closure.
            if let MutationIntent::DirectAssign {
                target,
                value: Some(v),
                ..
            } = intent
            {
                let formatted = self.expr_fmt().format_expr(v);
                // Unique temp per TARGET — two await-bearing mutations
                // in one body previously both emitted `var __v`
                // (duplicate_definition, Task 5.8).
                let tmp = format!("__{}", target.name);
                w.emit_fmt(format_args!("var {tmp} = {formatted};"));
                w.emit_fmt(format_args!("setState(() {{ {} = {tmp}; }});", target.name));
                return;
            }
            // Unhoistable shape (compound ops with await in the amount) —
            // plain assignment outside setState (the enclosing method is
            // async; avoids the invalid `await` inside the closure).
            let semi = if self.needs_semicolon() { ";" } else { "" };
            match intent {
                MutationIntent::Increment { target, amount } => {
                    let amt = amount
                        .as_ref()
                        .map(|v| self.expr_fmt().format_expr(v))
                        .unwrap_or_else(|| "1".into());
                    w.emit_fmt(format_args!("{} += {amt}{semi}", target.name));
                }
                MutationIntent::Decrement { target, amount } => {
                    let amt = amount
                        .as_ref()
                        .map(|v| self.expr_fmt().format_expr(v))
                        .unwrap_or_else(|| "1".into());
                    w.emit_fmt(format_args!("{} -= {amt}{semi}", target.name));
                }
                MutationIntent::Push { target, item } => {
                    let it = item
                        .as_ref()
                        .map(|v| self.expr_fmt().format_expr(v))
                        .unwrap_or_else(|| "null".into());
                    w.emit_fmt(format_args!("{}.add({it}){semi}", target.name));
                }
                MutationIntent::DirectAssign { target, value, .. } => {
                    let v = value
                        .as_ref()
                        .map(|v| self.expr_fmt().format_expr(v))
                        .unwrap_or_else(|| "null".into());
                    w.emit_fmt(format_args!("{} = {v}{semi}", target.name));
                }
                _ => {}
            }
        }
        // Standard path — emitted inline (a trait-ufcs call would
        // dispatch back to this override: infinite recursion). Mirrors the
        // shared default: setState-wrapped mutation via the language hooks.
        let semi = if self.needs_semicolon() { ";" } else { "" };
        if let Some(prefix) = self.mutation_stmt_prefix() {
            w.emit(prefix);
            w.indent();
        }
        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amount = value
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} = {amount}{semi}"));
            }
            MutationIntent::Increment { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} += {amt}{semi}"));
            }
            MutationIntent::Decrement { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} -= {amt}{semi}"));
            }
            MutationIntent::Multiply { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!("{ref_t} *= {amt}{semi}"));
            }
            MutationIntent::Divide { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!("{ref_t} /= {amt}{semi}"));
            }
            MutationIntent::Modulo { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} %= {amt}{semi}"));
            }
            MutationIntent::Push { target, item } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let it = item
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!(
                    "{ref_t}.{}({it}){semi}",
                    self.mutation_push_method()
                ));
            }
            MutationIntent::Pop { target } => {
                let ref_t = self.mutation_target_expr(&target.name);
                w.emit_fmt(format_args!(
                    "{ref_t}.{}(){semi}",
                    self.mutation_pop_method()
                ));
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
        if let Some(suffix) = self.mutation_stmt_suffix() {
            w.dedent();
            w.emit(suffix);
        }
    }
}

/// Does this statement block contain an `await` (explicit `AwaitExpr` or a
/// stdlib call whose Flutter emission embeds `await` — storage/http)? Drives
/// async method signatures and the mount-body async wrapper (Task 5.8).
pub(crate) fn stmt_block_emits_await(statements: &[StatementIR]) -> bool {
    statements.iter().any(stmt_emits_await)
}

fn stmt_emits_await(stmt: &StatementIR) -> bool {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => value.as_ref().is_some_and(expr_emits_await),
        StatementIR::ExprStmtIR { expr } => expr_emits_await(expr),
        StatementIR::ReturnStmtIR { value } => value.as_ref().is_some_and(expr_emits_await),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            expr_emits_await(condition)
                || then_body.statements.iter().any(stmt_emits_await)
                || else_ifs
                    .iter()
                    .any(|(c, b)| expr_emits_await(c) || b.statements.iter().any(stmt_emits_await))
                || else_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_emits_await))
        }
        StatementIR::WhileStmtIR { condition, body } => {
            expr_emits_await(condition) || body.statements.iter().any(stmt_emits_await)
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            expr_emits_await(iterable) || body.statements.iter().any(stmt_emits_await)
        }
        StatementIR::MutationStmt { intent } => intent.as_ref().is_some_and(|i| match i {
            MutationIntent::DirectAssign { value, .. } => {
                value.as_deref().is_some_and(expr_emits_await)
            }
            MutationIntent::Push { item, .. } => item.as_deref().is_some_and(expr_emits_await),
            _ => false,
        }),
        StatementIR::EventEmitStmtIR { args, .. } => args.iter().any(expr_emits_await),
        StatementIR::BlockStmtIR { body } => body.statements.iter().any(stmt_emits_await),
        StatementIR::ThrowStmtIR { expr } => expr_emits_await(expr),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body.statements.iter().any(stmt_emits_await)
                || catch_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_emits_await))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(stmt_emits_await))
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Recursively check whether an expression's Flutter emission requires an
/// async context.
pub(crate) fn expr_emits_await(expr: &ExprIR) -> bool {
    match expr {
        ExprIR::AwaitExpr { .. } => true,
        ExprIR::CallExpr { name, args } => {
            stdlib_flutter_emits_await(name) || args.iter().any(expr_emits_await)
        }
        ExprIR::FieldAccessExpr { object, .. } => expr_emits_await(object),
        ExprIR::OptionalChainExpr { object, .. } => expr_emits_await(object),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_emits_await(left) || expr_emits_await(right)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_emits_await(operand),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => expr_emits_await(cond) || expr_emits_await(then_expr) || expr_emits_await(else_expr),
        ExprIR::ArrayExpr { elements } => elements.iter().any(expr_emits_await),
        ExprIR::SpreadExpr { operand } => expr_emits_await(operand),
        ExprIR::StructExpr { fields, .. } => fields.iter().any(|(_, v)| expr_emits_await(v)),
        ExprIR::StoreFieldAccessExpr { call_args, .. } => call_args.iter().any(expr_emits_await),
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_emits_await(object) || args.iter().any(expr_emits_await)
        }
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(stmt_emits_await) || expr_emits_await(tail)
        }
        ExprIR::IndexExpr { object, index } => expr_emits_await(object) || expr_emits_await(index),
        ExprIR::ClosureExpr { body, .. } => expr_emits_await(body),
        ExprIR::VariantLiteral { fields, .. } => fields.iter().any(|(_, v)| expr_emits_await(v)),
        ExprIR::MatchExpr { matched, arms } => {
            expr_emits_await(matched)
                || arms
                    .iter()
                    .any(|a| a.body.statements.iter().any(stmt_emits_await))
        }
        ExprIR::MapExpr { entries } => entries
            .iter()
            .any(|(k, v)| expr_emits_await(k) || expr_emits_await(v)),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => expr_emits_await(subject) || expr_emits_await(predicate),
        _ => false,
    }
}

/// Does this stdlib call's Flutter emission embed `await` (storage_get/set —
/// SharedPreferences, http_*)? The registry's `is_async` flag is per-fn, not
/// per-target, so the emitted form itself is the source of truth.
fn stdlib_flutter_emits_await(name: &str) -> bool {
    let Some(f) = crate::stdlib::registry().iter().find(|f| f.name == name) else {
        return false;
    };
    if !f
        .available_targets
        .contains(&crate::codegen::Target::Flutter)
    {
        return false;
    }
    let dummies: Vec<String> = (0..f.min_args.max(1)).map(|_| "x".into()).collect();
    crate::codegen::try_format_stdlib_call(name, &dummies, crate::codegen::Target::Flutter)
        .is_some_and(|code| code.contains("await"))
}

/// Store-action-mode Flutter statement formatter (Riverpod 3.0 Notifier
/// methods). Mutations use the `state = state.copyWith(...)` pattern with no
/// setState wrapping; bare signal/getter identifiers are rewritten to
/// `state.<name>` by `StoreActionExprFormatter`.
pub(crate) struct StoreActionStmtFormatter<'a> {
    pub(crate) expr: super::expr::StoreActionExprFormatter<'a>,
}

impl<'a> FormatStmt for StoreActionStmtFormatter<'a> {
    type ExprFormatter = super::expr::StoreActionExprFormatter<'a>;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "var"
    }

    fn for_in_uses_of(&self) -> bool {
        false
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
        is_mut: bool,
    ) {
        format_dart_let_stmt(&self.expr, w, name, value, type_ann, is_mut);
    }

    /// Riverpod 3.0 store mutations: `state = state.copyWith(field: value)`.
    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let fmt_expr = |e: &ExprIR| self.expr.format_expr(e);
        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let v = value
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: {});",
                    target.name, v
                ));
            }
            MutationIntent::Increment { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "1".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} + {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Decrement { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "1".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} - {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Multiply { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} * {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Divide { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} / {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Modulo { target, amount } => {
                let amt = amount
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "2".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{} % {});",
                    target.name, target.name, amt
                ));
            }
            MutationIntent::Push { target, item } => {
                let it = item
                    .as_deref()
                    .map(fmt_expr)
                    .unwrap_or_else(|| "null".into());
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: [...state.{}]..add({}));",
                    target.name, target.name, it
                ));
            }
            MutationIntent::Pop { target } => {
                w.emit_fmt(format_args!(
                    "state = state.copyWith({}: state.{}.sublist(0, state.{}.length - 1));",
                    target.name, target.name, target.name
                ));
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
}

/// The mutation's target signal name — shared by the D1/D2 short-circuit
/// (every MutationIntent variant carries `target: SignalRefIR`).
fn mutation_target_name(intent: &MutationIntent) -> Option<&str> {
    Some(match intent {
        MutationIntent::DirectAssign { target, .. }
        | MutationIntent::Increment { target, .. }
        | MutationIntent::Decrement { target, .. }
        | MutationIntent::Multiply { target, .. }
        | MutationIntent::Divide { target, .. }
        | MutationIntent::Modulo { target, .. }
        | MutationIntent::Push { target, .. }
        | MutationIntent::Pop { target } => &target.name,
        #[allow(unreachable_patterns)]
        _ => return None,
    })
}

/// Setless statement emission — the D1/D2-free path for tests (setState
/// wrapping, bare signal names). Production component emission goes through
/// `generate_statement_flutter_dw`.
#[cfg(test)]
pub(crate) fn generate_statement_flutter(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = FlutterStmtFormatter {
        expr: super::r#ref::FlutterFormatExpr::new(None),
        direct_write_signals: FxHashSet::default(),
    };
    fmt.format_stmt(w, stmt);
}

/// Phase 3 (ADR-071 decision 2): statement emission carrying the component's
/// D1/D2 signal set — mutations on those signals write `_name.value`
/// directly (no setState), and identifiers resolve through the `.value`
/// form. Used for every statement emitted inside a component context
/// (State methods, @mount/@unmount, effect bodies, computed fn-form bodies,
/// inline handlers).
pub(crate) fn generate_statement_flutter_dw(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    direct_write_signals: &FxHashSet<String>,
) {
    let fmt = FlutterStmtFormatter {
        expr: super::r#ref::FlutterFormatExpr::new_with_direct_write(None, direct_write_signals),
        direct_write_signals: direct_write_signals.clone(),
    };
    fmt.format_stmt(w, stmt);
}

/// Statement formatter for TOP-LEVEL library functions (inlined cross-file
/// fns, Task 5.8): mutations are plain assignments — no `setState` wrapper
/// exists at module scope. Everything else matches the component formatter
/// (Dart method table, event emits, await hoisting).
struct PlainStmtFormatter<F: FormatExpr> {
    expr: F,
}

impl<F: FormatExpr> FormatStmt for PlainStmtFormatter<F> {
    type ExprFormatter = F;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "var"
    }

    fn for_in_uses_of(&self) -> bool {
        false
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
        is_mut: bool,
    ) {
        format_dart_let_stmt(&self.expr, w, name, value, type_ann, is_mut);
    }

    fn mutation_target_expr(&self, target: &str) -> String {
        target.to_string()
    }

    fn mutation_push_method(&self) -> &'static str {
        "add"
    }

    fn mutation_pop_method(&self) -> &'static str {
        "removeLast"
    }

    fn format_event_emit_stmt(&self, w: &mut CodeWriter, event_name: &str, args: &[ExprIR]) {
        let arg_str = if args.is_empty() {
            String::from("")
        } else {
            let formatted: Vec<String> = args
                .iter()
                .map(|a| self.expr_fmt().format_expr(a))
                .collect();
            formatted.join(", ")
        };
        w.emit_fmt(format_args!("widget.{event_name}?.call({arg_str});"));
    }
}

/// Generate a cross-file library function at TOP LEVEL (not a State method):
/// Dart-typed params/returns, async when the body needs `await`, plain
/// mutations (no setState at module scope).
pub(crate) fn generate_lib_fn_flutter(w: &mut CodeWriter, func: &FunctionIR) {
    // Task 5.8a: re-annotate IdentifierExpr type_ann from the fn's params
    // (the IR's annotation is never populated) so the shared Dart method
    // table shapes get/contains/concat from static types — e.g.
    // error_handling.omni's `map.contains(key)` on a Map<String,String>
    // param (unnecessary_type_check without the annotation).
    let type_scope = super::type_annotate::build_lib_fn_type_scope(func);
    let body_stmts: Vec<StatementIR> = func
        .body
        .statements
        .iter()
        .map(|s| super::type_annotate::annotate_stmt_types(s, &type_scope))
        .collect();
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = super::super::dart::dart_type_ir(ty);
            let _ = write!(s, "{} ", ann);
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    let is_async = func.is_async || stmt_block_emits_await(&func.body.statements);
    let return_prefix = if is_async {
        match func.return_type {
            Some(ref rt) if !matches!(rt.kind, TypeKind::Void) => {
                format!("Future<{}> ", super::super::dart::dart_type_ir(rt))
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| !matches!(rt.kind, TypeKind::Void))
            .map(|rt| format!("{} ", super::super::dart::dart_type_ir(rt)))
            .unwrap_or_default()
    };
    let async_kw = if is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit_fmt(format_args!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();
    let fmt = PlainStmtFormatter {
        expr: super::r#ref::FlutterFormatExpr::new(None),
    };
    // Tail-expression return (Task 5.8): a non-void fn whose LAST statement
    // is a bare expression (the language's tail `match result { ... }` form)
    // must RETURN it — the match IIFE was previously discarded
    // (body_might_complete_normally in lib/api.omni's error_message).
    let has_non_void_return = func
        .return_type
        .as_ref()
        .is_some_and(|rt| !matches!(rt.kind, TypeKind::Void));
    let is_tail_expr = has_non_void_return
        && matches!(
            func.body.statements.last(),
            Some(StatementIR::ExprStmtIR { .. })
        );
    let tail_count = if is_tail_expr {
        body_stmts.len() - 1
    } else {
        body_stmts.len()
    };
    for stmt in &body_stmts[..tail_count] {
        fmt.format_stmt(w, stmt);
    }
    if is_tail_expr {
        if let Some(StatementIR::ExprStmtIR { expr }) = body_stmts.last() {
            w.emit_fmt(format_args!(
                "return {};",
                super::r#ref::FlutterFormatExpr::new(None).format_expr(expr)
            ));
        }
    }
    w.dedent();
    w.emit("}");
}

/// Generate statements inside a store action body (Riverpod 3.0 Notifier
/// method — `state = state.copyWith(...)` mutations with bare signal
/// identifiers rewritten to `state.<name>`). `param_names` are the action's
/// params: they shadow same-named store signals (fn_locals pattern, OmniView
/// store.rs), so identifiers matching them stay bare.
pub(crate) fn generate_store_action_stmt(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    store: &StoreIR,
    param_names: &FxHashSet<String>,
) {
    let store_names: FxHashSet<String> = store
        .signals
        .keys()
        .chain(store.getters.keys())
        .cloned()
        .collect();
    let fmt = StoreActionStmtFormatter {
        expr: super::expr::StoreActionExprFormatter {
            inner: DartFormatExpr {
                event_param_name: None,
            },
            store_names: &store_names,
            param_names,
        },
    };
    fmt.format_stmt(w, stmt);
}

/// Generate a function as a method on the State class (test-only entry —
/// production call sites use `generate_function_flutter_with`).
#[cfg(test)]
pub(crate) fn generate_function_flutter(w: &mut CodeWriter, func: &FunctionIR) {
    generate_function_flutter_with(w, func, &func.body.statements, &FxHashSet::default());
}

/// Generate a function as a method on the State class, emitting the given
/// (already rewritten) statements. Task 5.8: the method is emitted `async`
/// when the body needs it — stdlib calls whose Flutter emission embeds
/// `await` (storage_get/set, http_*) would otherwise produce
/// `await_in_wrong_context` (UserSettings' storage_get in `@mount`).
/// Phase 3: `direct_write_signals` carries the component's D1/D2 set — the
/// method's mutations write `_name.value` directly, no setState.
pub(crate) fn generate_function_flutter_with(
    w: &mut CodeWriter,
    func: &FunctionIR,
    statements: &[StatementIR],
    direct_write_signals: &FxHashSet<String>,
) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = super::super::dart::dart_type_ir(ty);
            let _ = write!(s, "{} ", ann);
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    let is_async = func.is_async || stmt_block_emits_await(statements);

    // Determine return type
    // Wrap in Future<T> for async functions
    let return_prefix = if is_async {
        match func.return_type {
            Some(ref rt) if !matches!(rt.kind, TypeKind::Void) => {
                format!("Future<{}> ", super::super::dart::dart_type_ir(rt))
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| !matches!(rt.kind, TypeKind::Void))
            .map(|rt| format!("{} ", super::super::dart::dart_type_ir(rt)))
            .unwrap_or_default()
    };

    let async_kw = if is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit_fmt(format_args!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();

    let has_non_void_return = func
        .return_type
        .as_ref()
        .is_some_and(|rt| !matches!(rt.kind, TypeKind::Void));
    // Task 5.8: a NON-VOID fn whose LAST statement is a bare expression
    // (tail-expression semantics — the language's `match result { ... }`
    // tail form) must RETURN it — the emitted match IIFE was discarded
    // (body_might_complete_normally in error_message).
    let is_tail_expr =
        has_non_void_return && matches!(statements.last(), Some(StatementIR::ExprStmtIR { .. }));
    let tail_count = if is_tail_expr {
        statements.len() - 1
    } else {
        statements.len()
    };
    for stmt in &statements[..tail_count] {
        generate_statement_flutter_dw(w, stmt, direct_write_signals);
    }
    if is_tail_expr {
        if let Some(StatementIR::ExprStmtIR { expr }) = statements.last() {
            w.emit_fmt(format_args!(
                "return {};",
                generate_statement_flutter_expr(expr, direct_write_signals)
            ));
        }
    }

    w.dedent();
    w.emit("}");
}

/// Format a single expression with the component statement formatter's
/// expression pipeline. Phase 3: carries the D1/D2 set so tail-expression
/// identifiers resolve through `_name.value`.
fn generate_statement_flutter_expr(
    expr: &ExprIR,
    direct_write_signals: &FxHashSet<String>,
) -> String {
    super::r#ref::FlutterFormatExpr::new_with_direct_write(None, direct_write_signals)
        .format_expr(expr)
}

/// Drop the statements that follow an unsupported pointer-capture ref-op:
/// its Flutter emission is a `throw UnsupportedError(...)` (design D3 /
/// spec §10.3 — capturePointer/releasePointer hard-error), so later
/// statements in the same block are unreachable — keeping them flagged
/// `dead_code` in dart analyze (Task 5.8a, ElementRefDemo's onPtrDown /
/// onPtrUp). The throw statement itself is kept (the honest degradation).
pub(crate) fn truncate_after_pointer_capture(stmts: &mut Vec<StatementIR>) {
    let pos = stmts.iter().position(|s| match s {
        StatementIR::ExprStmtIR { expr } => matches!(
            expr,
            ExprIR::RefOpExpr {
                op: RefOpIR::CapturePointer | RefOpIR::ReleasePointer,
                ..
            }
        ),
        _ => false,
    });
    if let Some(p) = pos {
        stmts.truncate(p + 1);
    }
}
