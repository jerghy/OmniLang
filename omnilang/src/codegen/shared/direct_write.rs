//! Signal-domain classification for the direct-write channel (ADR-071
//! decision 2, Phase 1 — shared pass, Task 12).
//!
//! Classifies component signals consumed by dynamic `style: { expr }`
//! bindings on `@ref`'d elements (the "property always equals expr"
//! semantic) into storage domains:
//!
//! * D1 (direct-write): signal's only output reads are inside active
//!   entries → stored in a ref cell, mutations inject a direct DOM write.
//! * D2 (write-only): signal has no output reads (only mutation-RHS
//!   self-reads) → stored in a ref cell, no apply needed.
//! * B2 (shared): signal has render-path reads (text/class/prop/...)
//!   → stays on the render-path state channel + a deps-driven effect
//!   direct write.
//!
//! Computed signals and store fields NEVER enter D1/D2 (they must keep
//! their render-path derivation); entries reading them are B2.
//!
//! Read-domain boundary (fix rounds F2/F1): output-read classification
//! counts (a) view-tree reads, (b) inline event-handler body reads,
//! (c) effect/lifecycle/computed-getter body reads, and (d) active-entry
//! value reads (subtracted for D1). Reads inside *called-fn* bodies —
//! including mutation-RHS self-reads — never participate in D1/D2
//! determination: D1/D2 names resolve to `{name}Ref.current` there
//! (always the live value; Task 3 emission contract), so the render path
//! never consumes them and the semantics are equivalent. Effect/lifecycle/
//! computed bodies DO participate (fix round F1): their dep-array emitters
//! emit bare signal names, which would be undeclared (TS2304) for a
//! ref-stored D1/D2 signal — counting the reads sends such signals back to
//! B2 (render-path state), where the bare name is declared.
//!
//! This module is the TARGET-INDEPENDENT analysis half of the direct-write
//! channel: it contains no emission code and no backend conventions
//! (React's `useRef`/`useEffect`/JSX or Flutter's widget tree) — the
//! backend-specific emission lives in the React target
//! (`react/component/direct_write.rs`). The collector helpers below are
//! self-contained mirrors of the equivalent React-target collectors
//! (`react/component/state.rs` view-read/mutation collectors,
//! `react/component/mod.rs` stmt/expr identifier collectors,
//! `react/component/function.rs` called-local-fn collection) so the
//! classification result is bit-identical to the pre-Phase-3 React pass.

use crate::ir::walk::walk_expr;
use crate::ir::{
    BinaryOpIR, ComponentIR, EventHandlerIR, ExprIR, MutationIntent, StatementIR, ViewTree,
};
use rustc_hash::FxHashSet;
use serde_json::Value;

/// One activated style entry: `{ref}Ref.current.style.{prop} = value`.
#[derive(Debug, Clone)]
pub(crate) struct DirectWriteEntry {
    pub ref_name: String,
    pub prop: String,
    pub value: Value,
    pub read_signals: FxHashSet<String>,
}

/// Classification result consumed by the backend emission passes.
#[derive(Debug, Default)]
pub(crate) struct DirectWriteSets {
    /// D1 signals: ref-cell storage + mutation-site apply injection.
    pub d1: FxHashSet<String>,
    /// D2 signals: ref-cell storage, no apply.
    pub d2: FxHashSet<String>,
    /// Activated (D1) entries — order follows view traversal.
    pub entries: Vec<DirectWriteEntry>,
    /// B2 entries: deps-driven effect direct writes.
    pub b2_entries: Vec<DirectWriteEntry>,
}

/// True when a style value is dynamic (contains $signal/$ident/$field/
/// $call/$concat meta-objects) — mirrors the shared/style.rs dispatch.
fn is_dynamic_style_value(val: &Value) -> bool {
    match val {
        Value::Object(obj) => {
            obj.contains_key("$signal")
                || obj.contains_key("$ident")
                || obj.contains_key("$field")
                || obj.contains_key("$call")
                || obj.contains_key("$concat")
        }
        _ => false,
    }
}

/// Collect every identifier name referenced by a style value JSON tree
/// ($signal / $ident leaf nodes).
fn collect_value_reads(val: &Value, acc: &mut FxHashSet<String>) {
    match val {
        Value::Object(obj) => {
            if let Some(sig) = obj.get("$signal").and_then(|v| v.as_str()) {
                acc.insert(sig.to_string());
                return;
            }
            if let Some(ident) = obj.get("$ident").and_then(|v| v.as_str()) {
                acc.insert(ident.to_string());
                return;
            }
            for v in obj.values() {
                collect_value_reads(v, acc);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_value_reads(v, acc);
            }
        }
        _ => {}
    }
}

/// Walk the view tree collecting elements (ref binding + dynamic style
/// entries). `in_for` guards activation condition 3 (@for + ref inactive).
fn collect_active_entries(node: &ViewTree, in_for: bool, out: &mut Vec<DirectWriteEntry>) {
    match node {
        ViewTree::Element(el) => {
            // ElementIR has no for_binding field — elements inside @for are
            // reached via the ListRendering arm below with in_for=true.
            let nested_for = in_for;
            if let Some(ref_name) = &el.ref_binding {
                if let Some(obj) = el.inline_style.as_ref().and_then(|v| v.as_object()) {
                    for (prop, val) in obj {
                        if !nested_for && is_dynamic_style_value(val) {
                            let mut reads = FxHashSet::default();
                            collect_value_reads(val, &mut reads);
                            out.push(DirectWriteEntry {
                                ref_name: ref_name.clone(),
                                prop: prop.clone(),
                                value: val.clone(),
                                read_signals: reads,
                            });
                        }
                    }
                }
            }
            for child in &el.children {
                collect_active_entries(child, nested_for, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for b in &cond.branches {
                if let Some(body) = &b.body {
                    collect_active_entries(body, in_for, out);
                }
            }
            if let Some(fallback) = &cond.fallback {
                collect_active_entries(fallback, in_for, out);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(body) = &list.body {
                collect_active_entries(body, true, out);
            }
            if let Some(empty_body) = &list.empty_body {
                collect_active_entries(empty_body, true, out);
            }
        }
        // Multi-element roots and slot fallbacks (Task 4 gap fix): lowering
        // wraps multi-node view roots in ViewTree::Fragment — the collector
        // must descend or a multi-sibling view silently deactivates the
        // whole direct-write channel (entries=[] → style stays on the render
        // path; the Drag fixture's single-div root masked this).
        ViewTree::Fragment(children) => {
            for child in children {
                collect_active_entries(child, in_for, out);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_active_entries(child, in_for, out);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// Identifier collection (mirrors react/component/mod.rs)
// ═══════════════════════════════════════════════════════════

/// Collect identifier + call names from an expression tree (the shared IR
/// walker descends blocks/match arms — including statements inside them,
/// which is how mutation-RHS self-reads join the set). Mirrors the React
/// target's `collect_expr_identifiers` (walk-based) + emit-name scanning.
fn collect_expr_identifiers(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    walk_expr(expr, &mut |e| match e {
        ExprIR::IdentifierExpr { name, .. } => {
            acc.insert(name.clone());
        }
        ExprIR::CallExpr { name, .. } => {
            acc.insert(name.clone());
        }
        _ => {}
    });
    collect_expr_emit_names(expr, acc);
}

/// Collect `EventEmitStmtIR` event names from expression-level blocks —
/// the shared walker cannot surface statement-level data, so this targeted
/// scanner (Task 5.5 review Important-1) walks every `BlockExpr` /
/// `MatchExpr` arm body in the expression tree — the only places statements
/// live inside expressions.
fn collect_expr_emit_names(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    walk_expr(expr, &mut |e| match e {
        ExprIR::BlockExpr { statements, .. } => {
            collect_block_emit_names(statements, acc);
        }
        ExprIR::MatchExpr { arms, .. } => {
            for arm in arms {
                collect_block_emit_names(&arm.body.statements, acc);
            }
        }
        _ => {}
    });
}

/// Collect emit event names from a statement list, descending through
/// nested statement bodies (the statement-level mirror of
/// `collect_stmt_identifiers`' traversal — only the emit surface).
fn collect_block_emit_names(statements: &[StatementIR], acc: &mut FxHashSet<String>) {
    for stmt in statements {
        collect_stmt_emit_names(stmt, acc);
    }
}

/// Collect the event name of an `emit` statement, recursing into nested
/// statement bodies.
fn collect_stmt_emit_names(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::EventEmitStmtIR { event_name, .. } => {
            acc.insert(event_name.clone());
        }
        StatementIR::IfStmtIR {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            collect_block_emit_names(&then_body.statements, acc);
            for (_, body) in else_ifs {
                collect_block_emit_names(&body.statements, acc);
            }
            if let Some(body) = else_body {
                collect_block_emit_names(&body.statements, acc);
            }
        }
        StatementIR::WhileStmtIR { body, .. } | StatementIR::ForInStmtIR { body, .. } => {
            collect_block_emit_names(&body.statements, acc)
        }
        StatementIR::BlockStmtIR { body } => collect_block_emit_names(&body.statements, acc),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            collect_block_emit_names(&try_body.statements, acc);
            if let Some(body) = catch_body {
                collect_block_emit_names(&body.statements, acc);
            }
            if let Some(body) = finally_body {
                collect_block_emit_names(&body.statements, acc);
            }
        }
        _ => {}
    }
}

/// Collect identifier references from a StatementIR (mirror of the React
/// target's `collect_stmt_identifiers` — including `emit onX(...)`
/// event-name reads).
fn collect_stmt_identifiers(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => {
            collect_expr_identifiers(expr, acc);
        }
        StatementIR::LetStmtIR { name: _, value, .. } => {
            if let Some(ref v) = value {
                collect_expr_identifiers(v, acc);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(ref v) = value {
                collect_expr_identifiers(v, acc);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_expr_identifiers(condition, acc);
            for s in &then_body.statements {
                collect_stmt_identifiers(s, acc);
            }
            for (cond, body) in else_ifs {
                collect_expr_identifiers(cond, acc);
                for s in &body.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_expr_identifiers(condition, acc);
            for s in &body.statements {
                collect_stmt_identifiers(s, acc);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_expr_identifiers(iterable, acc);
            for s in &body.statements {
                collect_stmt_identifiers(s, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_identifiers(s, acc);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                collect_mutation_identifiers(intent, acc);
            }
        }
        StatementIR::ThrowStmtIR { expr } => {
            collect_expr_identifiers(expr, acc);
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_identifiers(s, acc);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            // `emit onX(...)` emits the event prop name + argument
            // expressions verbatim — both must count as reads or the
            // generated call references an undeclared name (TS2304).
            acc.insert(event_name.clone());
            for arg in args {
                collect_expr_identifiers(arg, acc);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Identifier reads carried by a mutation's value/amount/item expressions
/// (mirror of the React target's `collect_mutation_identifiers`).
fn collect_mutation_identifiers(intent: &MutationIntent, acc: &mut FxHashSet<String>) {
    match intent {
        MutationIntent::DirectAssign { value, .. } => {
            if let Some(ref v) = value {
                collect_expr_identifiers(v, acc);
            }
        }
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => {
            if let Some(ref v) = amount {
                collect_expr_identifiers(v, acc);
            }
        }
        MutationIntent::Push { item, .. } => {
            if let Some(ref v) = item {
                collect_expr_identifiers(v, acc);
            }
        }
        MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// View read collection (mirrors react/component/state.rs)
// ═══════════════════════════════════════════════════════════

/// Collect signal names that are read (not just mutated) from the view tree
/// (mirror of the React target's `collect_view_read_signals`).
fn collect_view_read_signals(node: &ViewTree) -> FxHashSet<String> {
    let mut acc = FxHashSet::default();
    collect_view_read_signals_inner(node, &mut acc);
    acc
}

fn collect_view_read_signals_inner(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for val in el.props.values() {
                collect_expr_identifiers(val, acc);
            }
            for sig_ref in el.bindings.values() {
                acc.insert(sig_ref.name.clone());
                // A qualified binding target (`bind:value={Store.sig}`) must
                // also feed the store-ref root so the `useStore()` hook is
                // generated and the read (`value={Store.sig}`) resolves.
                if let Some((root, _)) = sig_ref.name.split_once('.') {
                    acc.insert(root.to_string());
                }
            }
            // Event-handler FnRefs emit the ref name as a value
            // (`@onPress={onClose}` → `onClick={onClose}`) — an event prop
            // referenced ONLY that way must still be declared, or the
            // generated code references an undeclared name (TS2304).
            for handler in el.events.values() {
                if let EventHandlerIR::FnRef { r#ref, .. } = handler {
                    acc.insert(r#ref.clone());
                }
            }
            // Class map bindings (class: { active: isOpen }) emit their
            // conditions inside the rendered class template — a signal read
            // ONLY there would otherwise be emitted undeclared.
            for binding in el.class_map_bindings.values() {
                collect_expr_identifiers(binding, acc);
            }
            // Semantic ref bindings (focus={sig}, value={Store.sig}, ...)
            // read their bound signals inside generated effects — they must
            // count toward the storage decisions or a store-backed binding
            // silently loses its store hook.
            for binding_expr in el.semantic_bindings.values() {
                collect_expr_identifiers(binding_expr, acc);
            }
            // a11y block expressions (hidden/alt — design doc 域 10) emit
            // inside the rendered attributes — a signal read ONLY there
            // must still count as a render-path read.
            if let Some(ref a11y) = el.a11y {
                if let Some(ref expr) = a11y.hidden {
                    collect_expr_identifiers(expr, acc);
                }
                if let Some(ref expr) = a11y.alt {
                    collect_expr_identifiers(expr, acc);
                }
            }
            // Inline style bindings (style: { width: sig, ... }) read their
            // signals inside the emitted style object — collect them so
            // a signal referenced only there still counts as a render-path
            // read (active-entry reads are subtracted afterwards).
            if let Some(ref style_json) = el.inline_style {
                collect_inline_style_signals(style_json, acc);
            }
            for child in &el.children {
                collect_view_read_signals_inner(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_expr_identifiers(&branch.condition, acc);
                if let Some(ref body) = branch.body {
                    collect_view_read_signals_inner(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_read_signals_inner(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_expr_identifiers(&list.iterable, acc);
            if let Some(ref body) = list.body {
                collect_view_read_signals_inner(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_read_signals_inner(empty_body, acc);
            }
        }
        ViewTree::Interpolation(expr) => {
            collect_expr_identifiers(expr, acc);
        }
        ViewTree::Slot { fallback, .. } => {
            // Fallback content renders inside the component — signals it
            // reads must count toward the storage decisions.
            for child in fallback {
                collect_view_read_signals_inner(child, acc);
            }
        }
        ViewTree::Text(_) => {}
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_read_signals_inner(child, acc);
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect signal names referenced from an inline-style value
/// (`style: { prop: expr }` lowers to JSON with `{"$signal": "name"}`
/// markers plus `$concat` / `$call` / `$signal_concat` nesting).
fn collect_inline_style_signals(val: &Value, acc: &mut FxHashSet<String>) {
    match val {
        Value::Object(obj) => {
            if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
                acc.insert(sig_name.to_string());
            }
            for v in obj.values() {
                collect_inline_style_signals(v, acc);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_inline_style_signals(item, acc);
            }
        }
        _ => {}
    }
}

/// Walk view tree to collect identifier reads from inline event handler
/// bodies (mirror of the React target's `collect_view_inline_handler_reads`).
fn collect_view_inline_handler_reads(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for stmt in &body.statements {
                        collect_stmt_identifiers(stmt, acc);
                    }
                }
            }
            for child in &el.children {
                collect_view_inline_handler_reads(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_inline_handler_reads(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_inline_handler_reads(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_inline_handler_reads(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_inline_handler_reads(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_inline_handler_reads(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_view_inline_handler_reads(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// Mutation collection (mirrors react/component/state.rs)
// ═══════════════════════════════════════════════════════════

/// Collect names of all signals that are mutated (assigned to) anywhere in
/// the component — mirror of the React target's `collect_signal_mutations`
/// (called-fn filter: only fns reachable from view/effects/lifecycles/
/// computeds are scanned; the filter keeps the D1/D2 decision in lockstep
/// with the per-target state-hook decisions that consume the same set).
fn collect_signal_mutations(
    comp: &ComponentIR,
    called_fns: &FxHashSet<String>,
) -> FxHashSet<String> {
    let mut acc = FxHashSet::default();

    // Scan component fns — only those actually called from view/effects/lifecycles
    for fun in &comp.fns {
        if !called_fns.contains(&fun.name) {
            continue;
        }
        for stmt in &fun.body.statements {
            collect_mutation_targets_from_stmt(stmt, &mut acc);
        }
    }
    // Scan effects
    for effect in &comp.effects {
        for stmt in &effect.body.statements {
            collect_mutation_targets_from_stmt(stmt, &mut acc);
        }
    }
    // Scan lifecycles
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_mutation_targets_from_stmt(stmt, &mut acc);
        }
    }
    // Scan computeds (may contain mutations via let/assign)
    for computed in &comp.computeds {
        if let ExprIR::BlockExpr { statements, .. } = &computed.body {
            for stmt in statements {
                collect_mutation_targets_from_stmt(stmt, &mut acc);
            }
        }
    }
    // Scan view inline handlers
    if let Some(ref view) = comp.view {
        collect_view_mutation_targets(view, &mut acc);
    }

    acc
}

/// Collect mutation target names from a statement (the signal being
/// assigned to).
fn collect_mutation_targets_from_stmt(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                collect_mutation_targets_from_intent(intent, acc);
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            // Check for direct assignments in expr position
            collect_mutation_targets_from_expr(expr, acc);
        }
        StatementIR::LetStmtIR { name: _, value, .. } => {
            if let Some(ref v) = value {
                collect_mutation_targets_from_expr(v, acc);
            }
        }
        StatementIR::IfStmtIR {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            for s in &then_body.statements {
                collect_mutation_targets_from_stmt(s, acc);
            }
            for (_, body) in else_ifs {
                for s in &body.statements {
                    collect_mutation_targets_from_stmt(s, acc);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_mutation_targets_from_stmt(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { body, .. } => {
            for s in &body.statements {
                collect_mutation_targets_from_stmt(s, acc);
            }
        }
        StatementIR::ForInStmtIR { body, .. } => {
            for s in &body.statements {
                collect_mutation_targets_from_stmt(s, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_mutation_targets_from_stmt(s, acc);
            }
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_mutation_targets_from_stmt(s, acc);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_mutation_targets_from_stmt(s, acc);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_mutation_targets_from_stmt(s, acc);
                }
            }
        }
        StatementIR::ReturnStmtIR { .. }
        | StatementIR::BreakStmtIR
        | StatementIR::ContinueStmtIR
        | StatementIR::ThrowStmtIR { .. }
        | StatementIR::EventEmitStmtIR { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Extract mutation target from a MutationIntent.
fn collect_mutation_targets_from_intent(intent: &MutationIntent, acc: &mut FxHashSet<String>) {
    let target = match intent {
        MutationIntent::DirectAssign { target, .. }
        | MutationIntent::Increment { target, .. }
        | MutationIntent::Decrement { target, .. }
        | MutationIntent::Multiply { target, .. }
        | MutationIntent::Divide { target, .. }
        | MutationIntent::Modulo { target, .. }
        | MutationIntent::Push { target, .. }
        | MutationIntent::Pop { target } => target,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    };
    acc.insert(target.name.clone());
}

/// Check an expression for mutation targets (e.g., assignment expressions).
fn collect_mutation_targets_from_expr(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    match expr {
        ExprIR::BinaryOpExpr { left, op, .. } if *op == BinaryOpIR::Assign => {
            if let ExprIR::IdentifierExpr { name, .. } = left.as_ref() {
                acc.insert(name.clone());
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                collect_mutation_targets_from_stmt(stmt, acc);
            }
            collect_mutation_targets_from_expr(tail, acc);
        }
        _ => {}
    }
}

/// Walk view tree to collect mutation targets from inline event handlers
/// and two-way bindings (mirror of the React target's
/// `collect_view_mutation_targets`).
fn collect_view_mutation_targets(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::InlineFn { body, .. } => {
                        for stmt in &body.statements {
                            collect_mutation_targets_from_stmt(stmt, acc);
                        }
                    }
                    EventHandlerIR::FnRef { .. } | EventHandlerIR::StoreAction { .. } => {}
                }
            }
            // Two-way bindings (bind:value={x}) emit `setX(...)` into the
            // element's onChange — a signal mutated ONLY that way (never in
            // a fn/handler) must still count as mutated, or it is emitted
            // as a plain const and the generated setter call is TS2304.
            for sig_ref in el.bindings.values() {
                acc.insert(sig_ref.name.clone());
            }
            for child in &el.children {
                collect_view_mutation_targets(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_mutation_targets(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_mutation_targets(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_mutation_targets(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_mutation_targets(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_mutation_targets(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_view_mutation_targets(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// Called-local-fn collection (mirrors react/component/function.rs)
// ═══════════════════════════════════════════════════════════

/// Names of component fns actually called from the execution surface
/// (view handlers, effects, lifecycles, computeds) with transitive closure
/// through fn bodies and view inline handlers. Mirrors the React target's
/// `collect_called_local_fns` — the mutation scan only counts mutations in
/// called fns (an uncalled fn's body never runs; counting its mutations
/// would desync the classification from the state-hook decisions).
fn collect_called_local_fns(comp: &ComponentIR) -> FxHashSet<String> {
    let mut acc = FxHashSet::default();

    // From view event handlers and props
    if let Some(ref view) = comp.view {
        collect_called_fns_from_view(view, &mut acc);
    }
    // From effects
    for effect in &comp.effects {
        for stmt in &effect.body.statements {
            collect_called_fns_from_stmt(stmt, &mut acc);
        }
    }
    // From lifecycles
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_called_fns_from_stmt(stmt, &mut acc);
        }
    }
    // From computeds
    for computed in &comp.computeds {
        collect_called_fns_from_expr(&computed.body, &mut acc);
    }

    // Transitive closure: scan bodies of called fns for calls to other
    // local fns
    let mut prev_len = 0;
    while acc.len() > prev_len {
        prev_len = acc.len();
        for fun in &comp.fns {
            if acc.contains(&fun.name) {
                for stmt in &fun.body.statements {
                    collect_called_fns_from_stmt(stmt, &mut acc);
                }
            }
        }
        // Also scan fn bodies from view inline handlers for transitive calls
        if let Some(ref view) = comp.view {
            collect_called_fns_from_view_transitively(view, &mut acc);
        }
    }

    acc
}

fn collect_called_fns_from_view_transitively(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for stmt in &body.statements {
                        collect_called_fns_from_stmt(stmt, acc);
                    }
                }
            }
            for child in &el.children {
                collect_called_fns_from_view_transitively(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_called_fns_from_view_transitively(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_called_fns_from_view_transitively(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_called_fns_from_view_transitively(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_called_fns_from_view_transitively(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_called_fns_from_view_transitively(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_called_fns_from_view_transitively(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

fn collect_called_fns_from_view(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        acc.insert(r#ref.clone());
                    }
                    EventHandlerIR::InlineFn { body, .. } => {
                        for stmt in &body.statements {
                            collect_called_fns_from_stmt(stmt, acc);
                        }
                    }
                    EventHandlerIR::StoreAction { .. } => {}
                }
            }
            // Also scan class_map_bindings for function calls (e.g. isActive())
            for binding in el.class_map_bindings.values() {
                collect_called_fns_from_expr(binding, acc);
            }
            // @drag/@dragEnd handlers (ADR-073) — the synthesized pointer
            // wiring calls them by bare name; without this the emitted code
            // references an undeclared const (TS2304).
            if let Some(drag) = &el.drag_binding {
                if let EventHandlerIR::FnRef { r#ref, .. } = &drag.on_drag {
                    acc.insert(r#ref.clone());
                }
                if let Some(EventHandlerIR::FnRef { r#ref, .. }) = &drag.on_drag_end {
                    acc.insert(r#ref.clone());
                }
            }
            // C1: prop values may CALL component fns (src={getTitle()}) or
            // reference them by VALUE (transform={shout}) — the emitted code
            // references both forms, so both must be collected. Identifier
            // names that are not component fns (signals, loop items, props)
            // are filtered by the consumer (mutation scan matches against
            // comp.fns), so the collection never over-emits.
            for prop in el.props.values() {
                collect_called_fns_from_expr(prop, acc);
                collect_expr_identifiers(prop, acc);
            }
            // Semantic ref bindings (focus={expr}, value={expr}, ...) emit
            // their expressions inside generated effects — calls there are
            // referenced by the emitted code and need declarations.
            for binding_expr in el.semantic_bindings.values() {
                collect_called_fns_from_expr(binding_expr, acc);
            }
            // C1: inline styles lower to JSON with `$call` markers rendered
            // as `fnName(args)` by format_style_value_expr — the called
            // function must be declared (getScoreColor(row.score)).
            if let Some(ref style_json) = el.inline_style {
                collect_called_fns_from_style_json(style_json, acc);
            }
            // el.bindings are SignalRefIR (name-only references) — they can
            // never carry function calls, so nothing to collect here (the
            // mutation side is handled by collect_signal_mutations).
            for child in &el.children {
                collect_called_fns_from_view(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_called_fns_from_expr(&branch.condition, acc);
                if let Some(ref body) = branch.body {
                    collect_called_fns_from_view(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_called_fns_from_view(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_called_fns_from_expr(&list.iterable, acc);
            if let Some(ref body) = list.body {
                collect_called_fns_from_view(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_called_fns_from_view(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_called_fns_from_view(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_called_fns_from_view(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

fn collect_called_fns_from_style_json(val: &Value, acc: &mut FxHashSet<String>) {
    match val {
        Value::Object(obj) => {
            if let Some(call_obj) = obj.get("$call").and_then(|v| v.as_object()) {
                if let Some(fn_name) = call_obj.get("name").and_then(|v| v.as_str()) {
                    acc.insert(fn_name.to_string());
                }
            }
            for v in obj.values() {
                collect_called_fns_from_style_json(v, acc);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_called_fns_from_style_json(item, acc);
            }
        }
        _ => {}
    }
}

/// Collect call names from a statement (mirror of the React target's
/// `collect_called_fns_from_stmt` — including the DirectAssign-only mutation
/// arm, kept verbatim so the called set is bit-identical).
fn collect_called_fns_from_stmt(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => {
            collect_called_fns_from_expr(expr, acc);
        }
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(ref v) = value {
                collect_called_fns_from_expr(v, acc);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(ref v) = value {
                collect_called_fns_from_expr(v, acc);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_called_fns_from_expr(condition, acc);
            for s in &then_body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
            for (cond, body) in else_ifs {
                collect_called_fns_from_expr(cond, acc);
                for s in &body.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_called_fns_from_expr(condition, acc);
            for s in &body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_called_fns_from_expr(iterable, acc);
            for s in &body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(MutationIntent::DirectAssign {
                value: Some(ref v), ..
            }) = intent
            {
                collect_called_fns_from_expr(v, acc);
            }
        }
        StatementIR::ThrowStmtIR { expr } => {
            collect_called_fns_from_expr(expr, acc);
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_called_fns_from_stmt(s, acc);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_called_fns_from_stmt(s, acc);
                }
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            // `emit onX(fnCall())` emits the argument expressions verbatim.
            for arg in args {
                collect_called_fns_from_expr(arg, acc);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Collect call names from an expression. Built on the shared IR walker
/// (`walk_expr` descends every sub-expression, including block/match arm
/// statements) — equivalent to the React target's manual walker; the only
/// intentional difference is that `RefOpExpr` scroll/selection argument
/// expressions are walked here (the React target skips ref ops entirely),
/// an over-collection that only affects uncalled-fn pruning, never
/// classification of emitted code.
fn collect_called_fns_from_expr(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    walk_expr(expr, &mut |e| {
        if let ExprIR::CallExpr { name, .. } = e {
            acc.insert(name.clone());
        }
    });
}

// ═══════════════════════════════════════════════════════════
// Classification
// ═══════════════════════════════════════════════════════════

/// Output reads of a signal OUTSIDE active entry values — the set of
/// identifiers referenced by render output (text interpolation, class
/// bindings, props, non-active style entries) plus inline event-handler
/// body reads. Reads inside called-fn/effect bodies (incl. mutation-RHS
/// self-reads) are intentionally NOT collected — read-domain boundary,
/// see module doc (fix round F2).
///
/// Reuses the shared view collectors; the active-entry reads are
/// subtracted afterwards.
fn collect_output_reads(
    comp: &ComponentIR,
    active_entries: &[DirectWriteEntry],
) -> FxHashSet<String> {
    let mut reads = FxHashSet::default();
    if let Some(view) = &comp.view {
        reads.extend(collect_view_read_signals(view));
        collect_view_inline_handler_reads(view, &mut reads);
    }
    for entry in active_entries {
        for s in &entry.read_signals {
            reads.remove(s);
        }
    }
    // Fix round F1: effect + lifecycle body reads (stmt identifiers, incl.
    // mutation-RHS reads) and computed getter body reads join the output
    // set — the dep-array emitters (effect.rs deps, effect.rs unmount_deps,
    // computed.rs getter_expr_contains_ident, mod.rs useCallback wrapper
    // deps) emit these reads as BARE names; a D1/D2 signal (stored as
    // `nameRef`) would leave the bare name undeclared (TS2304). Counting
    // them returns the signal to B2 (render-path state), where the bare
    // name is declared. Collected AFTER the active-entry subtraction so a
    // read in BOTH an effect body and an entry still counts as an output
    // read — entry-only reads are what make D1 legal, not effect reads.
    // The fn-body read-domain boundary (incl. mutation-RHS self-reads)
    // stays: fn bodies emit no dep arrays.
    for effect in &comp.effects {
        for stmt in &effect.body.statements {
            collect_stmt_identifiers(stmt, &mut reads);
        }
    }
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_stmt_identifiers(stmt, &mut reads);
        }
    }
    for computed in &comp.computeds {
        collect_expr_identifiers(&computed.body, &mut reads);
    }
    reads
}

/// Classify component signals into D1 / D2; everything else stays on the
/// existing render-path state. Only `comp.signals` (not computeds) qualify.
pub(crate) fn classify_direct_write(comp: &ComponentIR) -> DirectWriteSets {
    let mut sets = DirectWriteSets::default();
    if let Some(view) = &comp.view {
        collect_active_entries(view, false, &mut sets.entries);
    }
    if sets.entries.is_empty() {
        return sets;
    }

    // Signal names appearing in any active entry value.
    let mut entry_consumed: FxHashSet<String> = FxHashSet::default();
    for e in &sets.entries {
        entry_consumed.extend(e.read_signals.iter().cloned());
    }

    // Output reads outside active entries (view + handlers + fns).
    let output_reads = collect_output_reads(comp, &sets.entries);

    let signal_names: FxHashSet<String> = comp.signals.iter().map(|s| s.name.clone()).collect();
    let called_fns = collect_called_local_fns(comp);
    let mutated_signals = collect_signal_mutations(comp, &called_fns);

    for sig in &comp.signals {
        if !signal_names.contains(&sig.name) {
            continue;
        }
        if !mutated_signals.contains(&sig.name) {
            continue;
        }
        if entry_consumed.contains(&sig.name) && !output_reads.contains(&sig.name) {
            sets.d1.insert(sig.name.clone());
        } else if !entry_consumed.contains(&sig.name) && !output_reads.contains(&sig.name) {
            sets.d2.insert(sig.name.clone());
        }
    }

    // B2 entries: any entry that fails to read ONLY D1 signals. Empty
    // read_signals (pure-literal $concat / zero-arg $call) is NOT "all D1":
    // such an entry consumes no D1 signal, so the apply channel would never
    // cover it and the style prop would silently never take effect — it
    // must stay on the deps-driven effect path (fix round F1). A non-D1
    // read (computed, store field, prop, or a plain signal left on the
    // render-path state) also keeps the entry on the effect path. A read
    // can never be D2 here (D2 requires absence from every entry's read
    // set).
    for e in sets.entries.iter() {
        let all_d1 =
            !e.read_signals.is_empty() && e.read_signals.iter().all(|s| sets.d1.contains(s));
        if !all_d1 {
            sets.b2_entries.push(e.clone());
        }
    }
    sets
}
