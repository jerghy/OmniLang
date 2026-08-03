// ═══════════════════════════════════════════════════════════
// Store slice subscription (Task 3.3, perf HIGH-1/H2)
//
// Components subscribe to the WHOLE Zustand store today
// (`const TodoStore = useTodoStore();`) while the store file's own
// per-field selector hooks go unused — every store write re-renders every
// subscriber and memo deps on `[TodoStore]` recompute constantly.
//
// This module decides, per store, between two subscription modes:
//
//  1. Slice mode (default): one `const items = useTodoStore((s) => s.items);`
//     per referenced signal/getter/action; all `TodoStore.field` reads and
//     `TodoStore.action()` calls are rewritten to the slice locals; memo and
//     effect deps reference the slice locals.
//
//  2. Whole-store mode (fallback): the pre-T3.3 behavior, kept when any
//     referenced field name would be captured by an inner scope or collide
//     with a component-level declaration — a bare name would resolve to the
//     wrong binding, so the qualified `TodoStore.field` form (which is
//     unambiguous) is preserved for that store.
//
// The store file (useXxx.ts) emits the store object only — the standalone
// per-field selector hooks (`useCount`, `useLiveAccent`, ...) were dead
// code: no consumer imports them (components AND the design-tokens hook
// subscribe inline with `useStore((s) => s.x)`, identical semantics, no
// import changes, no cross-store hook-name collisions) — removed by
// Task 2.7 (perf M2).
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Referenced fields of one store, classified by kind (emission order:
/// signals, getters, actions — matching the store file's hook order).
#[derive(Debug, Default, Clone)]
pub(crate) struct SliceFields {
    pub signals: Vec<String>,
    pub getters: Vec<String>,
    pub actions: Vec<String>,
}

impl SliceFields {
    fn is_empty(&self) -> bool {
        self.signals.is_empty() && self.getters.is_empty() && self.actions.is_empty()
    }
}

/// Collect store → referenced-field map from all component bodies (view,
/// computeds, effects, lifecycles, fns, inline handlers). Covers every IR
/// form a store reference takes:
///   - `FieldAccessExpr { Ident(store), field }` — reads (property syntax)
///   - `MethodCallExpr { Ident(store), method, .. }` — action calls
///   - `StoreFieldAccessExpr { store_alias, field_name, .. }` — the
///     semantic-normalized form (hand-built IR / fixture paths)
///   - `EventHandlerIR::StoreAction { r#ref: "Store.action" }` — view
///     handler shorthand
///   - `el.bindings` names — `bind:value={Store.field}` read side
///
/// Also returns the set of BARE store-alias identifier references (`let s =
/// TodoStore;`) — a store referenced as a value (not via a field) must keep
/// the whole-store object, since slice locals only cover individual fields
/// (review LOW-3).
///
/// Library functions are intentionally NOT scanned (review LOW-4): they are
/// emitted at module scope where the component's slice locals (and the
/// whole-store const) are out of scope, so store references inside them
/// cannot be satisfied by either subscription form — T3.3 keeps their
/// pre-existing (qualified) output untouched.
pub(super) fn collect_component_store_field_refs(
    comp: &ComponentIR,
    view: Option<&ViewTree>,
) -> (FxHashMap<String, FxHashSet<String>>, FxHashSet<String>) {
    let mut acc: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let mut bare: FxHashSet<String> = FxHashSet::default();

    if let Some(view) = view {
        collect_view_store_field_refs(view, &mut acc, &mut bare);
    }
    for computed in &comp.computeds {
        collect_expr_store_field_refs(&computed.body, &mut acc, &mut bare);
    }
    for effect in &comp.effects {
        for stmt in &effect.body.statements {
            collect_stmt_store_field_refs(stmt, &mut acc, &mut bare);
        }
    }
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_stmt_store_field_refs(stmt, &mut acc, &mut bare);
        }
    }
    for fun in &comp.fns {
        for stmt in &fun.body.statements {
            collect_stmt_store_field_refs(stmt, &mut acc, &mut bare);
        }
    }
    (acc, bare)
}

fn collect_view_store_field_refs(
    node: &ViewTree,
    acc: &mut FxHashMap<String, FxHashSet<String>>,
    bare: &mut FxHashSet<String>,
) {
    match node {
        ViewTree::Element(el) => {
            for val in el.props.values() {
                collect_expr_store_field_refs(val, acc, bare);
            }
            // bind:value read side — `Store.field` names are emitted
            // verbatim as JSX values (view.rs) and must be sliced too.
            for sig_ref in el.bindings.values() {
                if let Some((store, field)) = sig_ref.name.split_once('.') {
                    acc.entry(store.to_string())
                        .or_default()
                        .insert(field.to_string());
                }
            }
            for binding in el.class_map_bindings.values() {
                collect_expr_store_field_refs(binding, acc, bare);
            }
            for binding_expr in el.semantic_bindings.values() {
                collect_expr_store_field_refs(binding_expr, acc, bare);
            }
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        if let Some((store, field)) = r#ref.rsplit_once('.') {
                            acc.entry(store.to_string())
                                .or_default()
                                .insert(field.to_string());
                        }
                    }
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        // `@onClick={Store.action}` shorthand lowering to FnRef —
                        // the ref may be `Store.action` when the store hook is
                        // the only resolution (defensive; StoreAction is the
                        // canonical form).
                        if let Some((store, field)) = r#ref.rsplit_once('.') {
                            if acc.contains_key(store) {
                                acc.entry(store.to_string())
                                    .or_default()
                                    .insert(field.to_string());
                            }
                        }
                    }
                    EventHandlerIR::InlineFn { body, .. } => {
                        for stmt in &body.statements {
                            collect_stmt_store_field_refs(stmt, acc, bare);
                        }
                    }
                }
            }
            for child in &el.children {
                collect_view_store_field_refs(child, acc, bare);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_expr_store_field_refs(&branch.condition, acc, bare);
                if let Some(ref body) = branch.body {
                    collect_view_store_field_refs(body, acc, bare);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_store_field_refs(fb, acc, bare);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_expr_store_field_refs(&list.iterable, acc, bare);
            if let Some(ref body) = list.body {
                collect_view_store_field_refs(body, acc, bare);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_store_field_refs(empty_body, acc, bare);
            }
        }
        ViewTree::Interpolation(expr) => collect_expr_store_field_refs(expr, acc, bare),
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_view_store_field_refs(child, acc, bare);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_store_field_refs(child, acc, bare);
            }
        }
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Names declared anywhere in the component (or in inner scopes of its
/// bodies) that would CAPTURE a bare store-field slice local. When any used
/// field name collides, the store keeps the whole-store object so reads stay
/// qualified and unambiguous.
pub(super) fn collect_shadowing_names(
    comp: &ComponentIR,
    view: Option<&ViewTree>,
) -> FxHashSet<String> {
    let mut acc: FxHashSet<String> = FxHashSet::default();
    // Component-level declarations.
    for s in &comp.signals {
        acc.insert(s.name.clone());
    }
    for p in comp.props.keys() {
        acc.insert(p.clone());
    }
    for f in &comp.fns {
        acc.insert(f.name.clone());
    }
    for c in &comp.computeds {
        acc.insert(c.name.clone());
    }
    for r in &comp.refs {
        acc.insert(r.name.clone());
    }
    for e in &comp.events {
        acc.insert(e.name.clone());
    }
    // Slot props (`children` + named) become destructured props.
    for s in &comp.slots {
        acc.insert(s.name.clone().unwrap_or_else(|| "children".to_string()));
    }
    if let Some(view) = view {
        collect_view_shadowing_names(view, &mut acc);
    }
    // Fn params and inner-scope declarations. Library functions are NOT
    // scanned (review LOW-4): they are emitted at module scope, outside the
    // component, so their params/bodies cannot capture the component's
    // slice locals.
    for fun in &comp.fns {
        for p in &fun.params {
            acc.insert(p.name.clone());
        }
        for stmt in &fun.body.statements {
            collect_stmt_shadowing_names(stmt, &mut acc);
        }
    }
    for effect in &comp.effects {
        for stmt in &effect.body.statements {
            collect_stmt_shadowing_names(stmt, &mut acc);
        }
    }
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_stmt_shadowing_names(stmt, &mut acc);
        }
    }
    for computed in &comp.computeds {
        collect_expr_shadowing_names(&computed.body, &mut acc);
    }
    acc
}

fn collect_view_shadowing_names(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, params, .. } = handler {
                    for p in params {
                        acc.insert(p.clone());
                    }
                    for stmt in &body.statements {
                        collect_stmt_shadowing_names(stmt, acc);
                    }
                }
            }
            for val in el.props.values() {
                collect_expr_shadowing_names(val, acc);
            }
            for binding in el.class_map_bindings.values() {
                collect_expr_shadowing_names(binding, acc);
            }
            for binding_expr in el.semantic_bindings.values() {
                collect_expr_shadowing_names(binding_expr, acc);
            }
            for child in &el.children {
                collect_view_shadowing_names(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_expr_shadowing_names(&branch.condition, acc);
                if let Some(ref body) = branch.body {
                    collect_view_shadowing_names(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_shadowing_names(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_expr_shadowing_names(&list.iterable, acc);
            acc.insert(list.item_binding.name.clone());
            if let Some(ref body) = list.body {
                collect_view_shadowing_names(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_shadowing_names(empty_body, acc);
            }
        }
        ViewTree::Interpolation(expr) => collect_expr_shadowing_names(expr, acc),
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_view_shadowing_names(child, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_shadowing_names(child, acc);
            }
        }
        ViewTree::Text(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

fn collect_stmt_shadowing_names(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            acc.insert(name.clone());
            if let Some(ref v) = value {
                collect_expr_shadowing_names(v, acc);
            }
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
            ..
        } => {
            acc.insert(var_name.clone());
            collect_expr_shadowing_names(iterable, acc);
            for s in &body.statements {
                collect_stmt_shadowing_names(s, acc);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_expr_shadowing_names(condition, acc);
            for s in &then_body.statements {
                collect_stmt_shadowing_names(s, acc);
            }
            for (cond, body) in else_ifs {
                collect_expr_shadowing_names(cond, acc);
                for s in &body.statements {
                    collect_stmt_shadowing_names(s, acc);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_stmt_shadowing_names(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_expr_shadowing_names(condition, acc);
            for s in &body.statements {
                collect_stmt_shadowing_names(s, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_shadowing_names(s, acc);
            }
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_shadowing_names(s, acc);
            }
            if let Some(ref cv) = catch_var {
                acc.insert(cv.clone());
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_stmt_shadowing_names(s, acc);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_stmt_shadowing_names(s, acc);
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => collect_expr_shadowing_names(expr, acc),
        StatementIR::ReturnStmtIR { value } => {
            if let Some(ref v) = value {
                collect_expr_shadowing_names(v, acc);
            }
        }
        StatementIR::ThrowStmtIR { expr } => collect_expr_shadowing_names(expr, acc),
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                collect_mutation_shadowing_names(intent, acc);
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                collect_expr_shadowing_names(arg, acc);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

fn collect_mutation_shadowing_names(intent: &MutationIntent, acc: &mut FxHashSet<String>) {
    let expr = match intent {
        MutationIntent::DirectAssign { value, .. } => value.as_deref(),
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => amount.as_deref(),
        MutationIntent::Push { item, .. } => item.as_deref(),
        MutationIntent::Pop { .. } => None,
        #[allow(unreachable_patterns)]
        _ => None,
    };
    if let Some(e) = expr {
        collect_expr_shadowing_names(e, acc);
    }
}

fn collect_expr_shadowing_names(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    match expr {
        ExprIR::ClosureExpr { params, body } => {
            for p in params {
                acc.insert(p.clone());
            }
            collect_expr_shadowing_names(body, acc);
        }
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                collect_stmt_shadowing_names(stmt, acc);
            }
            collect_expr_shadowing_names(tail, acc);
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_expr_shadowing_names(matched, acc);
            for arm in arms {
                if let MatchPatternKindIR::Variant { bindings, .. } = &arm.pattern.kind {
                    for b in bindings {
                        acc.insert(b.clone());
                    }
                }
                for stmt in &arm.body.statements {
                    collect_stmt_shadowing_names(stmt, acc);
                }
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_expr_shadowing_names(left, acc);
            collect_expr_shadowing_names(right, acc);
        }
        ExprIR::UnaryOpExpr { operand, .. } => collect_expr_shadowing_names(operand, acc),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr_shadowing_names(cond, acc);
            collect_expr_shadowing_names(then_expr, acc);
            collect_expr_shadowing_names(else_expr, acc);
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                collect_expr_shadowing_names(arg, acc);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            collect_expr_shadowing_names(object, acc);
            for arg in args {
                collect_expr_shadowing_names(arg, acc);
            }
        }
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            collect_expr_shadowing_names(object, acc)
        }
        ExprIR::IndexExpr { object, index } => {
            collect_expr_shadowing_names(object, acc);
            collect_expr_shadowing_names(index, acc);
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                collect_expr_shadowing_names(e, acc);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_expr_shadowing_names(k, acc);
                collect_expr_shadowing_names(v, acc);
            }
        }
        ExprIR::StructExpr { fields, .. } | ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                collect_expr_shadowing_names(v, acc);
            }
        }
        ExprIR::SpreadExpr { operand } => collect_expr_shadowing_names(operand, acc),
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                collect_expr_shadowing_names(arg, acc);
            }
        }
        ExprIR::AwaitExpr { operand } => collect_expr_shadowing_names(operand, acc),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_expr_shadowing_names(subject, acc);
            collect_expr_shadowing_names(predicate, acc);
        }
        ExprIR::RefOpExpr { op, .. } => collect_ref_op_shadowing_names(op, acc),
        ExprIR::IdentifierExpr { .. } | ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

fn collect_ref_op_shadowing_names(op: &RefOpIR, acc: &mut FxHashSet<String>) {
    match op {
        RefOpIR::ScrollTo { x, y } => {
            if let Some(x) = x {
                collect_expr_shadowing_names(x, acc);
            }
            if let Some(y) = y {
                collect_expr_shadowing_names(y, acc);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                collect_expr_shadowing_names(dx, acc);
            }
            if let Some(dy) = dy {
                collect_expr_shadowing_names(dy, acc);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            collect_expr_shadowing_names(start, acc);
            collect_expr_shadowing_names(end, acc);
        }
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value
        | RefOpIR::CapturePointer
        | RefOpIR::ReleasePointer => {}
        // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Collect store → referenced-field map from an expression tree. Field and
/// method receivers are consumed by their parent arms (never recursed into),
/// so any IdentifierExpr reached by the walker is a BARE reference (review
/// LOW-3).
///
/// `pub(crate)`: also used by the design-tokens hook emitter
/// (react/design.rs) to slice its dynamic-token expressions (Task 2.7).
pub(crate) fn collect_expr_store_field_refs(
    expr: &ExprIR,
    acc: &mut FxHashMap<String, FxHashSet<String>>,
    bare: &mut FxHashSet<String>,
) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => match object.as_ref() {
            ExprIR::IdentifierExpr { name, .. } => {
                acc.entry(name.clone()).or_default().insert(field.clone());
            }
            _ => collect_expr_store_field_refs(object, acc, bare),
        },
        ExprIR::OptionalChainExpr { object, field } => {
            // `TodoStore?.items` reads the store field like a plain access —
            // record it so the store slices (review NEW-1; this arm also
            // prevents the unreachable! panic on ANY `?.` in a body).
            match object.as_ref() {
                ExprIR::IdentifierExpr { name, .. } => {
                    acc.entry(name.clone()).or_default().insert(field.clone());
                }
                _ => collect_expr_store_field_refs(object, acc, bare),
            }
        }
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            match object.as_ref() {
                ExprIR::IdentifierExpr { name, .. } => {
                    acc.entry(name.clone()).or_default().insert(method.clone());
                }
                _ => collect_expr_store_field_refs(object, acc, bare),
            }
            for arg in args {
                collect_expr_store_field_refs(arg, acc, bare);
            }
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            acc.entry(store_alias.clone())
                .or_default()
                .insert(field_name.clone());
            for arg in call_args {
                collect_expr_store_field_refs(arg, acc, bare);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_expr_store_field_refs(left, acc, bare);
            collect_expr_store_field_refs(right, acc, bare);
        }
        ExprIR::UnaryOpExpr { operand, .. } => collect_expr_store_field_refs(operand, acc, bare),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr_store_field_refs(cond, acc, bare);
            collect_expr_store_field_refs(then_expr, acc, bare);
            collect_expr_store_field_refs(else_expr, acc, bare);
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                collect_expr_store_field_refs(arg, acc, bare);
            }
        }
        ExprIR::IndexExpr { object, index } => {
            collect_expr_store_field_refs(object, acc, bare);
            collect_expr_store_field_refs(index, acc, bare);
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                collect_expr_store_field_refs(e, acc, bare);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_expr_store_field_refs(k, acc, bare);
                collect_expr_store_field_refs(v, acc, bare);
            }
        }
        ExprIR::ClosureExpr { body, .. } => collect_expr_store_field_refs(body, acc, bare),
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                collect_stmt_store_field_refs(stmt, acc, bare);
            }
            collect_expr_store_field_refs(tail, acc, bare);
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_expr_store_field_refs(matched, acc, bare);
            for arm in arms {
                for stmt in &arm.body.statements {
                    collect_stmt_store_field_refs(stmt, acc, bare);
                }
            }
        }
        ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                collect_expr_store_field_refs(v, acc, bare);
            }
        }
        ExprIR::SpreadExpr { operand } => collect_expr_store_field_refs(operand, acc, bare),
        ExprIR::AwaitExpr { operand } => collect_expr_store_field_refs(operand, acc, bare),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_expr_store_field_refs(subject, acc, bare);
            collect_expr_store_field_refs(predicate, acc, bare);
        }
        ExprIR::RefOpExpr { op, .. } => collect_ref_op_store_field_refs(op, acc, bare),
        ExprIR::IdentifierExpr { name, .. } => {
            bare.insert(name.clone());
        }
        ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

fn collect_ref_op_store_field_refs(
    op: &RefOpIR,
    acc: &mut FxHashMap<String, FxHashSet<String>>,
    bare: &mut FxHashSet<String>,
) {
    match op {
        RefOpIR::ScrollTo { x, y } => {
            if let Some(x) = x {
                collect_expr_store_field_refs(x, acc, bare);
            }
            if let Some(y) = y {
                collect_expr_store_field_refs(y, acc, bare);
            }
        }
        RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                collect_expr_store_field_refs(dx, acc, bare);
            }
            if let Some(dy) = dy {
                collect_expr_store_field_refs(dy, acc, bare);
            }
        }
        RefOpIR::SetSelection { start, end } => {
            collect_expr_store_field_refs(start, acc, bare);
            collect_expr_store_field_refs(end, acc, bare);
        }
        RefOpIR::Focus
        | RefOpIR::Blur
        | RefOpIR::Measure { .. }
        | RefOpIR::Select
        | RefOpIR::Value
        | RefOpIR::CapturePointer
        | RefOpIR::ReleasePointer => {}
        // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

fn collect_stmt_store_field_refs(
    stmt: &StatementIR,
    acc: &mut FxHashMap<String, FxHashSet<String>>,
    bare: &mut FxHashSet<String>,
) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => collect_expr_store_field_refs(expr, acc, bare),
        StatementIR::LetStmtIR { value, .. } => {
            if let Some(ref v) = value {
                collect_expr_store_field_refs(v, acc, bare);
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            if let Some(ref v) = value {
                collect_expr_store_field_refs(v, acc, bare);
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_expr_store_field_refs(condition, acc, bare);
            for s in &then_body.statements {
                collect_stmt_store_field_refs(s, acc, bare);
            }
            for (cond, body) in else_ifs {
                collect_expr_store_field_refs(cond, acc, bare);
                for s in &body.statements {
                    collect_stmt_store_field_refs(s, acc, bare);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_stmt_store_field_refs(s, acc, bare);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_expr_store_field_refs(condition, acc, bare);
            for s in &body.statements {
                collect_stmt_store_field_refs(s, acc, bare);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_expr_store_field_refs(iterable, acc, bare);
            for s in &body.statements {
                collect_stmt_store_field_refs(s, acc, bare);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_store_field_refs(s, acc, bare);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                collect_mutation_store_field_refs(intent, acc, bare);
            }
        }
        StatementIR::ThrowStmtIR { expr } => collect_expr_store_field_refs(expr, acc, bare),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_store_field_refs(s, acc, bare);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_stmt_store_field_refs(s, acc, bare);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_stmt_store_field_refs(s, acc, bare);
                }
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                collect_expr_store_field_refs(arg, acc, bare);
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

fn collect_mutation_store_field_refs(
    intent: &MutationIntent,
    acc: &mut FxHashMap<String, FxHashSet<String>>,
    bare: &mut FxHashSet<String>,
) {
    let expr = match intent {
        MutationIntent::DirectAssign { value, .. } => value.as_deref(),
        MutationIntent::Increment { amount, .. }
        | MutationIntent::Decrement { amount, .. }
        | MutationIntent::Multiply { amount, .. }
        | MutationIntent::Divide { amount, .. }
        | MutationIntent::Modulo { amount, .. } => amount.as_deref(),
        MutationIntent::Push { item, .. } => item.as_deref(),
        MutationIntent::Pop { .. } => None,
        #[allow(unreachable_patterns)]
        _ => None,
    };
    if let Some(e) = expr {
        collect_expr_store_field_refs(e, acc, bare);
    }
}

/// Stores referenced from motion value expressions (keyframe stops, `over`,
/// `delay`, in-view thresholds, scroll scrub, drag constraints). Motion
/// value formatting does not carry the slice renames (they are module-level
/// shared defs), so such stores keep the whole-store object — the qualified
/// `Store.field` reads stay valid.
pub(super) fn motion_whole_store_stores(
    view: Option<&ViewTree>,
    ir: &IRProgram,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> FxHashSet<String> {
    let Some(view) = view else {
        return FxHashSet::default();
    };
    let mut motion_bodies: FxHashMap<String, MotionBodyIR> = FxHashMap::default();
    for m in ir.modules.values().flat_map(|m| &m.motions) {
        motion_bodies.insert(m.name.clone(), m.body.clone());
    }
    let mut acc: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let mut bare: FxHashSet<String> = FxHashSet::default();
    let scan_body = |body: &MotionBodyIR,
                     acc: &mut FxHashMap<String, FxHashSet<String>>,
                     bare: &mut FxHashSet<String>| {
        collect_expr_store_field_refs(&body.over, acc, bare);
        if let Some(ref delay) = body.playback.delay {
            collect_expr_store_field_refs(delay, acc, bare);
        }
        for stop in &body.stops {
            for val in stop.properties.values() {
                collect_expr_store_field_refs(val, acc, bare);
            }
        }
    };
    super::super::motion::walk_motion_elements(view, component_map, |el, _idx| {
        let behavior = el.motion_behavior();
        if let Some(behavior) = behavior {
            for trigger in &behavior.triggers {
                if let Some(body) = super::super::motion::resolve_def(trigger, &motion_bodies) {
                    scan_body(body, &mut acc, &mut bare);
                }
                // @scroll / @drag-* control dimensions (Task 5.1) can
                // reference store fields — slices must keep them in the
                // subscribed set.
                match &trigger.kind {
                    TriggerKind::Scroll { start, end, scrub } => {
                        if let Some(e) = scrub {
                            collect_expr_store_field_refs(e, &mut acc, &mut bare);
                        }
                        if let Some(e) = start {
                            collect_expr_store_field_refs(e, &mut acc, &mut bare);
                        }
                        if let Some(e) = end {
                            collect_expr_store_field_refs(e, &mut acc, &mut bare);
                        }
                    }
                    TriggerKind::DragHorizontal {
                        constraints,
                        elastic,
                        threshold,
                        velocity_max,
                        ..
                    }
                    | TriggerKind::DragVertical {
                        constraints,
                        elastic,
                        threshold,
                        velocity_max,
                        ..
                    } => {
                        if let Some((min, max)) = constraints {
                            collect_expr_store_field_refs(min, &mut acc, &mut bare);
                            collect_expr_store_field_refs(max, &mut acc, &mut bare);
                        }
                        if let Some(e) = elastic {
                            collect_expr_store_field_refs(e, &mut acc, &mut bare);
                        }
                        if let Some(e) = threshold {
                            collect_expr_store_field_refs(e, &mut acc, &mut bare);
                        }
                        if let Some(e) = velocity_max {
                            collect_expr_store_field_refs(e, &mut acc, &mut bare);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(e) = &behavior.in_view_threshold {
                collect_expr_store_field_refs(e, &mut acc, &mut bare);
            }
        }
    });
    acc.into_keys().collect()
}

/// Decide the subscription mode per store and build the rename map.
///
/// Returns `(store_field_renames, slice_fields_by_store)`. Stores absent
/// from `slice_fields_by_store` keep the whole-store object.
///
/// `pub(crate)`: also used by the design-tokens hook emitter
/// (react/design.rs) with an empty (default) component — the component is
/// only consulted for its effect deps, of which the hook file has none
/// (Task 2.7).
pub(crate) fn build_slice_plan(
    comp: &ComponentIR,
    store_field_refs: &FxHashMap<String, FxHashSet<String>>,
    shadowing_names: &FxHashSet<String>,
    bare_store_refs: &FxHashSet<String>,
    motion_stores: &FxHashSet<String>,
    all_stores: &FxHashMap<&str, &StoreIR>,
) -> (FxHashMap<String, String>, FxHashMap<String, SliceFields>) {
    let mut renames: FxHashMap<String, String> = FxHashMap::default();
    let mut slices: FxHashMap<String, SliceFields> = FxHashMap::default();
    // Slice local names claimed by previously planned stores — two stores
    // with the same field name would emit duplicate `const items = ...`
    // declarations (TS2451), so the second store falls back to whole-store.
    let mut claimed_slice_names: FxHashSet<String> = FxHashSet::default();

    for (store_ref, fields) in store_field_refs {
        let Some(store) = all_stores.get(store_ref.as_str()) else {
            // Inline store (no module store file) or unknown — no slice
            // selectors possible; keep whatever the caller does (nothing
            // today). Renames stay empty so reads keep their qualified form.
            continue;
        };
        let mut slice = SliceFields::default();
        let mut unknown = false;
        for field in fields {
            if store.signals.contains_key(field.as_str()) {
                slice.signals.push(field.clone());
            } else if store.getters.contains_key(field.as_str()) {
                slice.getters.push(field.clone());
            } else if store.actions.iter().any(|a| a.name == *field) {
                slice.actions.push(field.clone());
            } else {
                // Referenced field is not a declared store member — cannot
                // slice it; the whole-store object keeps the reference valid.
                unknown = true;
                break;
            }
            // A bare slice local would be captured by an inner scope or
            // collide with a component declaration — fall back.
            if shadowing_names.contains(field.as_str()) {
                unknown = true;
                break;
            }
        }
        // Effect deps reference bare store signal names (`@effect(items)`) —
        // a dep that belongs to this store counts as a used field.
        for effect in &comp.effects {
            for dep in &effect.deps {
                if (store.signals.contains_key(dep.as_str())
                    || store.getters.contains_key(dep.as_str())
                    || store.actions.iter().any(|a| a.name == *dep))
                    && !slice.signals.contains(dep)
                    && !slice.getters.contains(dep)
                    && !slice.actions.contains(dep)
                    && !unknown
                {
                    if store.signals.contains_key(dep.as_str()) {
                        slice.signals.push(dep.clone());
                    } else if store.getters.contains_key(dep.as_str()) {
                        slice.getters.push(dep.clone());
                    } else {
                        slice.actions.push(dep.clone());
                    }
                    if shadowing_names.contains(dep.as_str()) {
                        unknown = true;
                        break;
                    }
                }
            }
            if unknown {
                break;
            }
        }
        // A BARE store-alias reference (`let s = TodoStore;` — review
        // LOW-3) needs the whole-store object; slice locals only cover
        // individual fields.
        if unknown
            || slice.is_empty()
            || bare_store_refs.contains(store_ref.as_str())
            || motion_stores.contains(store_ref.as_str())
        {
            continue;
        }
        // A field name already claimed by another store's slice would
        // produce a duplicate local — fall back for this store.
        let collides_with_other_store = slice
            .signals
            .iter()
            .chain(slice.getters.iter())
            .chain(slice.actions.iter())
            .any(|f| claimed_slice_names.contains(f.as_str()));
        if collides_with_other_store {
            continue;
        }
        for field in slice
            .signals
            .iter()
            .chain(slice.getters.iter())
            .chain(slice.actions.iter())
        {
            claimed_slice_names.insert(field.clone());
            renames.insert(format!("{}.{}", store_ref, field), field.clone());
        }
        slices.insert(store_ref.clone(), slice);
    }
    (renames, slices)
}

/// Emit one slice-selector subscription per referenced field:
/// `const items = useTodoStore((s) => s.items);`
///
/// `pub(crate)`: also used by the design-tokens hook emitter
/// (react/design.rs) — the hook subscribes per field like a component
/// (Task 2.7).
pub(crate) fn emit_store_slices(
    w: &mut CodeWriter,
    store_ref: &str,
    _store: &StoreIR,
    fields: &SliceFields,
) {
    for sig in &fields.signals {
        w.emit_fmt(format_args!(
            "const {} = use{}((s) => s.{});",
            sig, store_ref, sig
        ));
    }
    for getter in &fields.getters {
        w.emit_fmt(format_args!(
            "const {} = use{}((s) => s.{});",
            getter, store_ref, getter
        ));
    }
    for action in &fields.actions {
        w.emit_fmt(format_args!(
            "const {} = use{}((s) => s.{});",
            action, store_ref, action
        ));
    }
}
