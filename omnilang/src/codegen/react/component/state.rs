// ═══════════════════════════════════════════════════════════
// State codegen: signal → useState / setter codegen
// ═══════════════════════════════════════════════════════════

use crate::codegen::capitalize_first;
use crate::codegen::react::expr;
use crate::codegen::react::ReactCodegen;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::collect_expr_identifiers;
use super::collect_stmt_identifiers;

impl ReactCodegen {
    /// Generate useState hooks for component state signals.
    /// Only generates setter if signal is mutated; non-mutated signals use plain const.
    /// Skips states never read in view/computeds/effects/lifecycles/called-fns.
    /// Task 2.11: the read set is the VALUE-CONSUMED set — a signal whose
    /// only reads are its own mutation RHS (which emit as functional
    /// updaters that never reference the state value) is write-only dead
    /// state and is skipped here too (its mutations are pruned by the
    /// statement emitter, so no setter references remain).
    /// `hoisted_signals` (perf M3) were already emitted as module-scope consts
    /// before the component function — they are skipped here entirely.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gen_state_hooks(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        consumed_signals: &FxHashSet<String>,
        mutated_signals: &FxHashSet<String>,
        pruned_signals: &FxHashSet<String>,
        hoisted_signals: &FxHashSet<String>,
        fn_names: &FxHashSet<String>,
        store_field_renames: &FxHashMap<String, String>,
        user_types: &FxHashSet<String>,
    ) {
        for state in &comp.signals {
            // v4 H2: 有可达 mutation 的信号必须保留 state hook——
            // mutation 发射（含 fn 体）引用 setter。仅当读与写都缺失才跳过。
            let mutated = mutated_signals.contains(&state.name);
            // 修正（brief 偏差）：pruned 信号（值未被消费且 mutation 被
            // 发射侧剪枝——Task 2.11 DCE 契约）没有存活的 setter 引用，
            // 必须仍被跳过，否则发射悬空的 `const [, setX]`（unused 变量 +
            // 无意义 hook）。写专用但未 pruned 的信号（variant）命中
            // `mutated && !pruned`，保留 hook。
            let pruned = pruned_signals.contains(&state.name);
            if !consumed_signals.contains(&state.name) && (!mutated || pruned) {
                continue;
            }
            // Hoisted to module scope (perf M3) — in-function references
            // resolve to the module const via scope hoisting unchanged.
            if hoisted_signals.contains(&state.name) {
                continue;
            }
            // A signal initializer may read a store field
            // (`signal x: i32 = TodoStore.count;`) — the emitted initializer
            // must reference the slice local (review LOW-1).
            let init_val = expr::format_expr_ir_tsx_renamed(
                &state.initial_value,
                &FxHashSet::default(),
                store_field_renames,
            );
            if mutated_signals.contains(&state.name) {
                let setter_base = format!("set{}", capitalize_first(&state.name));
                let setter = if fn_names.contains(&setter_base) {
                    format!("__{}", setter_base)
                } else {
                    setter_base
                };
                // TS batch: type the useState generic from the IR signal type
                // (`useState<string[]>([])` instead of `useState([])` which
                // infers `never[]` and cascades into TS2322/TS2345 on every
                // setter call). Only when every name in the type resolves in
                // the generated file — user structs are now declared inline
                // by types.rs (deferral #49), so `useState<RowItem[]>([])`
                // resolves; `any` would defeat a literal initializer's
                // better inference.
                let type_ann = typed_state_annotation(&state.r#type, user_types);
                // v4 H2: 写专用信号（值从未被消费）——省略值名（无
                // unused 变量），`const [, setX] = useState(...)`。
                if consumed_signals.contains(&state.name) {
                    if type_ann.is_empty() {
                        w.emit_fmt(format_args!(
                            "const [{}, {}] = useState({});",
                            state.name, setter, init_val
                        ));
                    } else {
                        w.emit_fmt(format_args!(
                            "const [{}, {}] = useState<{}>({});",
                            state.name, setter, type_ann, init_val
                        ));
                    }
                } else {
                    if type_ann.is_empty() {
                        w.emit_fmt(format_args!(
                            "const [, {}] = useState({});",
                            setter, init_val
                        ));
                    } else {
                        w.emit_fmt(format_args!(
                            "const [, {}] = useState<{}>({});",
                            setter, type_ann, init_val
                        ));
                    }
                }
            } else {
                w.emit_fmt(format_args!("const {} = {};", state.name, init_val));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// perf M3: never-mutated literal signals → module-scope consts
// ═══════════════════════════════════════════════════════════

/// Whether an expression is a pure literal subtree — scalar literals
/// (string/number/bool/null), array literals, and object literals whose
/// keys and values are all themselves pure literals. Anything referencing
/// runtime state (identifiers, store fields, calls, spreads, ...)
/// disqualifies the subtree — conservative by design: only values that are
/// compile-time constants may leave the component function.
fn expr_is_pure_literal(expr: &ExprIR) -> bool {
    match expr {
        ExprIR::LiteralExpr { value } => {
            value.is_null() || value.is_boolean() || value.is_number() || value.is_string()
        }
        ExprIR::ArrayExpr { elements } => elements.iter().all(expr_is_pure_literal),
        ExprIR::MapExpr { entries } => entries
            .iter()
            .all(|(k, v)| expr_is_pure_literal(k) && expr_is_pure_literal(v)),
        _ => false,
    }
}

/// Collect the signals eligible for module-scope hoisting (perf M3):
/// read in the component, never mutated, initialized with a pure literal,
/// and not colliding with an existing module-scope binding in the generated
/// file (the component fn name, imported names, lib fns, memo aliases and
/// internal module tables — a duplicate `const` at module scope is a
/// redeclaration SyntaxError).
///
/// The never-mutated requirement keeps the hoist decision identical to the
/// useState decision (a mutated signal must stay useState — it can never
/// hoist); the pure-literal requirement guarantees the const carries no
/// store/prop/runtime dependencies (a store-field initializer must keep
/// referencing the slice local, a prop initializer the destructured prop).
pub(super) fn collect_hoistable_literal_signals(
    comp: &ComponentIR,
    read_signals: &FxHashSet<String>,
    mutated_signals: &FxHashSet<String>,
    module_reserved_names: &FxHashSet<String>,
) -> FxHashSet<String> {
    comp.signals
        .iter()
        .filter(|s| {
            read_signals.contains(&s.name)
                && !mutated_signals.contains(&s.name)
                && !module_reserved_names.contains(&s.name)
                && expr_is_pure_literal(&s.initial_value)
        })
        .map(|s| s.name.clone())
        .collect()
}

/// Emit module-scope consts for hoisted literal signals (perf M3). Called
/// before the component function declaration — references inside the
/// component resolve to these consts via scope hoisting unchanged.
pub(super) fn gen_hoisted_signal_consts(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    hoisted_signals: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
) {
    let mut emitted = false;
    for state in &comp.signals {
        if !hoisted_signals.contains(&state.name) {
            continue;
        }
        // Pure-literal initializers carry no identifiers, so the rename map
        // cannot apply — passed for signature symmetry with the in-function
        // emission (which the module const replaces).
        let init_val = expr::format_expr_ir_tsx_renamed(
            &state.initial_value,
            &FxHashSet::default(),
            store_field_renames,
        );
        w.emit_fmt(format_args!("const {} = {};", state.name, init_val));
        emitted = true;
    }
    if emitted {
        w.emit("");
    }
}

// ═══════════════════════════════════════════════════════════
// Signal mutation & usage tracking
// ═══════════════════════════════════════════════════════════

/// The TS type argument for `useState<T>`, or empty when the type must not
/// be emitted. A name resolves only when it exists in the generated file:
/// builtins always, user-declared types (structs/unions/aliases — `user_types`,
/// deferral #49) since types.rs declares them inline into every file that
/// references them. `any` is left untyped: a literal initializer (`[]`, `""`,
/// `0`) infers a strictly better type than `any`.
fn typed_state_annotation(ty: &TypeIR, user_types: &FxHashSet<String>) -> String {
    if !js_type_resolves(ty, user_types) {
        return String::new();
    }
    JsTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args)
}

/// Whether a TypeIR only references names that exist in the generated file:
/// builtin kinds plus container wrappers whose element types also resolve,
/// plus user-declared types (declared inline by types.rs).
fn js_type_resolves(ty: &TypeIR, user_types: &FxHashSet<String>) -> bool {
    match &ty.kind {
        TypeKind::String | TypeKind::Int | TypeKind::Float | TypeKind::Bool | TypeKind::Void => {
            true
        }
        TypeKind::Any => false,
        TypeKind::Named { name } => match name.as_str() {
            "Array" | "Vec" | "List" | "Map" | "HashMap" | "FxHashMap" | "Dict" | "Option"
            | "Record" => ty.type_args.iter().all(|t| js_type_resolves(t, user_types)),
            _ => user_types.contains(name),
        },
    }
}

/// Collect names of all signals that are mutated (assigned to) anywhere in the component.
pub(super) fn collect_signal_mutations(
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

/// Collect mutation target names from a statement (the signal being assigned to).
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

/// Walk view tree to collect mutation targets from inline event handlers.
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

/// Walk view tree to collect identifier reads from inline event handler bodies.
/// This catches event props like `onClose(...)` called inside handlers.
pub(super) fn collect_view_inline_handler_reads(node: &ViewTree, acc: &mut FxHashSet<String>) {
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

/// Task 2.11: inline-handler walker for the VALUE-CONSUMPTION analysis —
/// identical to [`collect_view_inline_handler_reads`] but walks handler
/// statements with the consumed walker (`collect_consumed_stmt_identifiers`
/// in component/mod.rs), which drops self-mutation phantom reads (`count =
/// count + 1` emits `setCount((c) => c + 1)`, so the RHS `count` never
/// appears in generated code).
pub(super) fn collect_view_inline_handler_consumed_reads(
    node: &ViewTree,
    acc: &mut FxHashSet<String>,
) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for stmt in &body.statements {
                        super::collect_consumed_stmt_identifiers(stmt, acc);
                    }
                }
            }
            for child in &el.children {
                collect_view_inline_handler_consumed_reads(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_inline_handler_consumed_reads(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_inline_handler_consumed_reads(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_inline_handler_consumed_reads(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_inline_handler_consumed_reads(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_inline_handler_consumed_reads(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_view_inline_handler_consumed_reads(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect signal names that are read (directly or via stores) in the component.
/// Scans view, effects, lifecycles, computeds, and function bodies.
/// Also collects store ref names from StoreAction and inline handlers.
pub(super) fn collect_used_store_signals(comp: &ComponentIR) -> FxHashSet<String> {
    let mut acc = FxHashSet::default();

    // From view (including event handlers that reference store refs)
    if let Some(ref view) = comp.view {
        collect_view_read_signals_inner(view, &mut acc);
        collect_view_store_refs_from_handlers(view, &mut acc);
    }
    // From effects
    for effect in &comp.effects {
        // Dep-array names are emitted verbatim — a store signal referenced
        // only as an effect dep still needs its store hook import.
        for dep in &effect.deps {
            acc.insert(dep.clone());
        }
        for stmt in &effect.body.statements {
            collect_expr_ids_from_stmt(stmt, &mut acc);
        }
    }
    // From lifecycles
    for lc in &comp.lifecycles {
        for stmt in &lc.body.statements {
            collect_expr_ids_from_stmt(stmt, &mut acc);
        }
    }
    // From computeds
    for computed in &comp.computeds {
        collect_expr_identifiers(&computed.body, &mut acc);
    }
    // From component fns (also scan the function bodies themselves)
    for fun in &comp.fns {
        for stmt in &fun.body.statements {
            collect_expr_ids_from_stmt(stmt, &mut acc);
        }
    }

    acc
}

fn collect_expr_ids_from_stmt(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::ExprStmtIR { expr } => {
            collect_expr_identifiers(expr, acc);
        }
        StatementIR::LetStmtIR { value, .. } => {
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
                collect_expr_ids_from_stmt(s, acc);
            }
            for (cond, body) in else_ifs {
                collect_expr_identifiers(cond, acc);
                for s in &body.statements {
                    collect_expr_ids_from_stmt(s, acc);
                }
            }
            if let Some(ref eb) = else_body {
                for s in &eb.statements {
                    collect_expr_ids_from_stmt(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_expr_identifiers(condition, acc);
            for s in &body.statements {
                collect_expr_ids_from_stmt(s, acc);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_expr_identifiers(iterable, acc);
            for s in &body.statements {
                collect_expr_ids_from_stmt(s, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_expr_ids_from_stmt(s, acc);
            }
        }
        StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                match intent {
                    MutationIntent::DirectAssign { target, value, .. } => {
                        acc.insert(target.name.clone());
                        if let Some(ref v) = value {
                            collect_expr_identifiers(v, acc);
                        }
                    }
                    MutationIntent::Increment { target, amount, .. }
                    | MutationIntent::Decrement { target, amount, .. }
                    | MutationIntent::Multiply { target, amount, .. }
                    | MutationIntent::Divide { target, amount, .. }
                    | MutationIntent::Modulo { target, amount, .. } => {
                        acc.insert(target.name.clone());
                        if let Some(ref v) = amount {
                            collect_expr_identifiers(v, acc);
                        }
                    }
                    MutationIntent::Push { target, item, .. } => {
                        acc.insert(target.name.clone());
                        if let Some(ref v) = item {
                            collect_expr_identifiers(v, acc);
                        }
                    }
                    MutationIntent::Pop { target } => {
                        acc.insert(target.name.clone());
                    }
                    #[allow(unreachable_patterns)]
                    _ => unreachable!(
                        "Unknown MutationIntent variant — update codegen for new variant"
                    ),
                }
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
                collect_expr_ids_from_stmt(s, acc);
            }
            if let Some(ref cb) = catch_body {
                for s in &cb.statements {
                    collect_expr_ids_from_stmt(s, acc);
                }
            }
            if let Some(ref fb) = finally_body {
                for s in &fb.statements {
                    collect_expr_ids_from_stmt(s, acc);
                }
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
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

/// Collect signal names that are read (not just mutated) from the view tree.
pub(super) fn collect_view_read_signals(node: &ViewTree) -> FxHashSet<String> {
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
            // Event-handler FnRefs emit the ref name as a JSX value
            // (`@onPress={onClose}` → `onClick={onClose}`) — an event prop
            // referenced ONLY that way must still be destructured, or the
            // generated JSX references an undeclared name (TS2304).
            for handler in el.events.values() {
                if let EventHandlerIR::FnRef { r#ref, .. } = handler {
                    acc.insert(r#ref.clone());
                }
            }
            // Class map bindings (class: { active: isOpen }) emit their
            // conditions inside the JSX className template — a signal read
            // ONLY there (e.g. `submitted`) would otherwise be emitted
            // undeclared (the generated `setSubmitted(...)` / `${submitted}`
            // both throw TS2304).
            for binding in el.class_map_bindings.values() {
                collect_expr_identifiers(binding, acc);
            }
            // Semantic ref bindings (focus={sig}, value={Store.sig}, ...)
            // read their bound signals inside generated effects — they must
            // count toward useState / store hook decisions or a store-backed
            // binding silently loses its `useStore()` import.
            for binding_expr in el.semantic_bindings.values() {
                collect_expr_identifiers(binding_expr, acc);
            }
            // a11y block expressions (hidden/alt — design doc 域 10) emit
            // inside the JSX attributes (`aria-hidden={expr}`) — a signal
            // read ONLY there must still produce a useState/store hook.
            if let Some(ref a11y) = el.a11y {
                if let Some(ref expr) = a11y.hidden {
                    collect_expr_identifiers(expr, acc);
                }
                if let Some(ref expr) = a11y.alt {
                    collect_expr_identifiers(expr, acc);
                }
            }
            // Inline style bindings (style: { width: sig, ... }) read their
            // signals inside the emitted JSX style object — collect them so
            // a signal referenced only there still gets a useState hook (it
            // would otherwise be emitted undeclared, throwing a
            // ReferenceError on first render).
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
            // reads must count toward useState / store hook decisions.
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
/// markers plus `$concat` / `$call` / `$signal_concat` nesting). The names
/// are the emitted JS variable names — bare signals use their source name,
/// store fields the `Store_field` form — matching `format_style_value_expr`.
fn collect_inline_style_signals(val: &serde_json::Value, acc: &mut FxHashSet<String>) {
    match val {
        serde_json::Value::Object(obj) => {
            if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
                acc.insert(sig_name.to_string());
            }
            for v in obj.values() {
                collect_inline_style_signals(v, acc);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_inline_style_signals(item, acc);
            }
        }
        _ => {}
    }
}

/// Collect store ref names from event handlers in the view tree.
/// Handles both StoreAction handlers (e.g. `NotificationStore.clear`) and
/// inline handlers that reference store objects directly.
fn collect_view_store_refs_from_handlers(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        // Store name is before the last dot: "NotificationStore.clear" → "NotificationStore"
                        if let Some(dot_pos) = r#ref.rfind('.') {
                            acc.insert(r#ref[..dot_pos].to_string());
                        }
                    }
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        acc.insert(r#ref.clone());
                    }
                    EventHandlerIR::InlineFn { body, .. } => {
                        for stmt in &body.statements {
                            collect_expr_ids_from_stmt(stmt, acc);
                        }
                    }
                }
            }
            for child in &el.children {
                collect_view_store_refs_from_handlers(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_store_refs_from_handlers(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_store_refs_from_handlers(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_store_refs_from_handlers(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_store_refs_from_handlers(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_store_refs_from_handlers(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_view_store_refs_from_handlers(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

// ═══════════════════════════════════════════════════════════
// Ref codegen: ref declarations → useRef hooks
// ═══════════════════════════════════════════════════════════

/// Build a map of ref name → HTML element tag by scanning the view tree for
/// elements carrying `ref_binding: Some(name)`. `RefDeclIR.element_tag` is
/// currently always None (lowering does not populate it), so this scan is the
/// source of truth for typing useRef declarations. The first binding found
/// wins (all bindings of one ref should share a tag anyway).
pub(super) fn collect_ref_tags(view: Option<&ViewTree>) -> FxHashMap<String, String> {
    let mut acc = FxHashMap::default();
    if let Some(view) = view {
        collect_ref_tags_inner(view, &mut acc);
    }
    acc
}

fn collect_ref_tags_inner(node: &ViewTree, acc: &mut FxHashMap<String, String>) {
    match node {
        ViewTree::Element(el) => {
            if let Some(ref binding) = el.ref_binding {
                acc.entry(binding.clone()).or_insert_with(|| el.tag.clone());
            }
            for child in &el.children {
                collect_ref_tags_inner(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_ref_tags_inner(body, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_ref_tags_inner(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_ref_tags_inner(body, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_ref_tags_inner(empty_body, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_ref_tags_inner(child, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_ref_tags_inner(child, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Generate `useRef<T>(null)` declarations for each ref in the component.
/// Maps known HTML element tags to their TypeScript types (e.g., input→HTMLInputElement).
/// Falls back to HTMLElement for unknown or missing tags.
/// Called after gen_state_hooks and before gen_effects in gen_component_tsx.
///
/// `motion_internal_refs` are synthesized `__el_N` refs for motion elements
/// without a user `@ref` — (ref_name, tag) pairs in DFS pre-order.
pub(super) fn gen_ref_hooks(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    ref_tags: &FxHashMap<String, String>,
    motion_internal_refs: &[(String, String)],
) {
    if comp.refs.is_empty() && motion_internal_refs.is_empty() {
        return;
    }
    for ref_decl in &comp.refs {
        // Prefer the declared element_tag if a future pass populates it;
        // otherwise use the tag of the element bound via @ref in the view.
        let tag = ref_decl
            .element_tag
            .as_deref()
            .or_else(|| ref_tags.get(&ref_decl.name).map(String::as_str));
        let ts_type = ref_element_type(tag);
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<{}>(null);",
            ref_decl.name, ts_type
        ));
    }
    for (ref_name, tag) in motion_internal_refs {
        let ts_type = ref_element_type(Some(tag));
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<{}>(null);",
            ref_name, ts_type
        ));
    }
    w.emit("");
}

/// Map an OmniView HTML element tag to its TypeScript element type.
///
/// The mapping must be accurate enough that `useRef<T>` is assignable to the
/// JSX `ref` prop — React refs are invariant in `current`, so a generic
/// `HTMLElement` fallback is NOT assignable to e.g. `Ref<HTMLHeadingElement>`.
fn ref_element_type(tag: Option<&str>) -> &str {
    match tag {
        Some("input") => "HTMLInputElement",
        Some("textarea") => "HTMLTextAreaElement",
        Some("select") => "HTMLSelectElement",
        Some("option") => "HTMLOptionElement",
        Some("optgroup") => "HTMLOptGroupElement",
        Some("button") => "HTMLButtonElement",
        Some("form") => "HTMLFormElement",
        Some("fieldset") => "HTMLFieldSetElement",
        Some("legend") => "HTMLLegendElement",
        Some("label") => "HTMLLabelElement",
        Some("output") => "HTMLOutputElement",
        Some("img") | Some("image") => "HTMLImageElement",
        Some("video") => "HTMLVideoElement",
        Some("audio") => "HTMLAudioElement",
        Some("canvas") => "HTMLCanvasElement",
        Some("a") | Some("link") => "HTMLAnchorElement",
        Some("table") => "HTMLTableElement",
        Some("thead") | Some("tbody") | Some("tfoot") => "HTMLTableSectionElement",
        Some("tr") => "HTMLTableRowElement",
        Some("td") | Some("th") => "HTMLTableCellElement",
        Some("caption") => "HTMLTableCaptionElement",
        Some("progress") => "HTMLProgressElement",
        Some("meter") => "HTMLMeterElement",
        Some("svg") => "SVGSVGElement",
        Some("div") | Some("section") | Some("article") | Some("nav") | Some("header")
        | Some("footer") | Some("main") | Some("aside") => "HTMLDivElement",
        Some("h1") | Some("h2") | Some("h3") | Some("h4") | Some("h5") | Some("h6") => {
            "HTMLHeadingElement"
        }
        Some("p") => "HTMLParagraphElement",
        Some("span") => "HTMLSpanElement",
        Some("ul") => "HTMLUListElement",
        Some("ol") => "HTMLOListElement",
        Some("li") => "HTMLLIElement",
        Some("dl") => "HTMLDListElement",
        Some("blockquote") | Some("q") => "HTMLQuoteElement",
        Some("pre") => "HTMLPreElement",
        Some("br") => "HTMLBRElement",
        Some("hr") => "HTMLHRElement",
        Some("time") => "HTMLTimeElement",
        Some("iframe") => "HTMLIFrameElement",
        Some("details") => "HTMLDetailsElement",
        Some("dialog") => "HTMLDialogElement",
        Some("menu") => "HTMLMenuElement",
        Some("map") => "HTMLMapElement",
        Some("area") => "HTMLAreaElement",
        Some("picture") => "HTMLPictureElement",
        Some("source") => "HTMLSourceElement",
        Some("track") => "HTMLTrackElement",
        Some("object") => "HTMLObjectElement",
        Some("embed") => "HTMLEmbedElement",
        // For unknown tags, use HTMLElement (generic) — better than HTMLDivElement
        _ => "HTMLElement",
    }
}

// ═══════════════════════════════════════════════════════════
// Semantic binding codegen: focus={sig} / scroll-x / scroll-y → useEffect
// (the value={sig} form is removed — design D2 — bind:value is emitted as
// value + onChange in view.rs, never as a one-way effect here)
// ═══════════════════════════════════════════════════════════

/// Whether any element in the view tree carries semantic ref bindings
/// (focus={sig}, scroll-x={sig}, scroll-y={sig}) — used to decide whether
/// the component needs a useEffect import.
pub(super) fn view_has_semantic_bindings(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            !el.semantic_bindings.is_empty() || el.children.iter().any(view_has_semantic_bindings)
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_semantic_bindings))
                || cond
                    .fallback
                    .as_ref()
                    .is_some_and(view_has_semantic_bindings)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_semantic_bindings)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_has_semantic_bindings)
        }
        ViewTree::Fragment(children) => children.iter().any(view_has_semantic_bindings),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_semantic_bindings),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Generate `useEffect` calls for semantic ref bindings on @ref-bound elements.
///
/// Each binding kind maps to a native DOM side effect keyed on the bound
/// signal (validation guarantees the element has a ref binding):
///
/// - `focus={sig}`    → focus()/blur() driven by the boolean signal
/// - `scroll-x={sig}` → scrollTo({ left: sig })
/// - `scroll-y={sig}` → scrollTo({ top: sig })
///
/// (`value={sig}` is removed — design D2 — `bind:value` is emitted as a
/// JSX value + onChange setter, never as a one-way effect.)
///
/// Called after gen_effects in gen_component_tsx.
pub(super) fn gen_semantic_binding_effects(
    w: &mut CodeWriter,
    view: Option<&ViewTree>,
    store_signal_map: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    hoisted_signals: &FxHashSet<String>,
) {
    let Some(view) = view else {
        return;
    };
    let mut emitted = false;
    walk_semantic_binding_effects(
        w,
        view,
        store_signal_map,
        ref_names,
        store_field_renames,
        hoisted_signals,
        &mut emitted,
    );
}

fn walk_semantic_binding_effects(
    w: &mut CodeWriter,
    node: &ViewTree,
    store_signal_map: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    hoisted_signals: &FxHashSet<String>,
    emitted: &mut bool,
) {
    match node {
        ViewTree::Element(el) => {
            if let Some(ref ref_name) = el.ref_binding {
                if !el.semantic_bindings.is_empty() {
                    emit_element_binding_effects(
                        w,
                        ref_name,
                        &el.semantic_bindings,
                        store_signal_map,
                        ref_names,
                        store_field_renames,
                        hoisted_signals,
                        emitted,
                    );
                }
            }
            for child in &el.children {
                walk_semantic_binding_effects(
                    w,
                    child,
                    store_signal_map,
                    ref_names,
                    store_field_renames,
                    hoisted_signals,
                    emitted,
                );
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_semantic_binding_effects(
                        w,
                        body,
                        store_signal_map,
                        ref_names,
                        store_field_renames,
                        hoisted_signals,
                        emitted,
                    );
                }
            }
            if let Some(ref fb) = cond.fallback {
                walk_semantic_binding_effects(
                    w,
                    fb,
                    store_signal_map,
                    ref_names,
                    store_field_renames,
                    hoisted_signals,
                    emitted,
                );
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                walk_semantic_binding_effects(
                    w,
                    body,
                    store_signal_map,
                    ref_names,
                    store_field_renames,
                    hoisted_signals,
                    emitted,
                );
            }
            if let Some(ref empty_body) = list.empty_body {
                walk_semantic_binding_effects(
                    w,
                    empty_body,
                    store_signal_map,
                    ref_names,
                    store_field_renames,
                    hoisted_signals,
                    emitted,
                );
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_semantic_binding_effects(
                    w,
                    child,
                    store_signal_map,
                    ref_names,
                    store_field_renames,
                    hoisted_signals,
                    emitted,
                );
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_semantic_binding_effects(
                    w,
                    child,
                    store_signal_map,
                    ref_names,
                    store_field_renames,
                    hoisted_signals,
                    emitted,
                );
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Emit one useEffect per semantic binding on a ref-bound element.
#[allow(clippy::too_many_arguments)]
fn emit_element_binding_effects(
    w: &mut CodeWriter,
    ref_name: &str,
    bindings: &FxHashMap<String, ExprIR>,
    store_signal_map: &FxHashMap<String, String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    hoisted_signals: &FxHashSet<String>,
    emitted: &mut bool,
) {
    let base = format!("{}Ref.current", ref_name);
    for (kind, expr) in bindings {
        let val = expr::format_expr_ir_tsx_renamed(expr, ref_names, store_field_renames);
        let deps_str =
            semantic_binding_deps(expr, store_signal_map, store_field_renames, hoisted_signals);
        if !*emitted {
            w.emit("");
            *emitted = true;
        }
        match kind.as_str() {
            // focus={sig}: boolean signal drives focus()/blur()
            "focus" => {
                w.emit("useEffect(() => {");
                w.indent();
                w.emit_fmt(format_args!("if ({}) {{", val));
                w.indent();
                w.emit_fmt(format_args!("{base}?.focus();"));
                w.dedent();
                w.emit("} else {");
                w.indent();
                w.emit_fmt(format_args!("{base}?.blur();"));
                w.dedent();
                w.emit("}");
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            }
            // scroll-y={sig}: vertical scroll position
            "scroll-y" => {
                w.emit("useEffect(() => {");
                w.indent();
                w.emit_fmt(format_args!("{base}?.scrollTo({{ top: {} }});", val));
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            }
            // scroll-x={sig}: horizontal scroll position
            "scroll-x" => {
                w.emit("useEffect(() => {");
                w.indent();
                w.emit_fmt(format_args!("{base}?.scrollTo({{ left: {} }});", val));
                w.dedent();
                w.emit_fmt(format_args!("}}, [{}]);", deps_str));
            }
            // The `value={sig}` semantic-binding form is removed (design D2):
            // `bind:value` is the sole two-way binding, emitted as a JSX
            // `value` + `onChange` setter — it never reaches this effect
            // path, and the old one-way DOM-syncing effect is deleted.
            // Unknown binding kind — emit nothing (future-proofing; the
            // semantic pass only produces focus/scroll-x/scroll-y).
            _ => {}
        }
    }
}

/// Collect the dependency list for a semantic binding effect.
///
/// - A store field read (`CounterStore.text`) depends on the exact value the
///   effect body reads: the slice local (`text`, T3.3) when the store is
///   subscribed per field, otherwise the qualified expression
///   (`CounterStore.text`) — never the store hook object, which is recreated
///   on every store write and would re-fire the effect on unrelated updates.
/// - A bare signal depends on its own name (resolved through the store
///   signal map for future-proofing); anything else falls back to collecting
///   identifiers, sorted for deterministic output.
/// - Hoisted signals (perf M3) are module-scope consts — never reactive —
///   and are excluded from the deps (eslint react-hooks flags outer-scope
///   values as "unnecessary dependency").
fn semantic_binding_deps(
    expr: &ExprIR,
    store_signal_map: &FxHashMap<String, String>,
    store_field_renames: &FxHashMap<String, String>,
    hoisted_signals: &FxHashSet<String>,
) -> String {
    if let ExprIR::FieldAccessExpr { object, field } = expr {
        if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
            return store_field_renames
                .get(&format!("{}.{}", name, field))
                .cloned()
                .unwrap_or_else(|| format!("{}.{}", name, field));
        }
    }
    let mut ids: Vec<String> = Vec::new();
    let mut acc = FxHashSet::default();
    collect_expr_identifiers(expr, &mut acc);
    for id in acc {
        if hoisted_signals.contains(&id) {
            continue;
        }
        let qualified = store_signal_map
            .get(&id)
            .cloned()
            .unwrap_or_else(|| id.clone());
        ids.push(
            store_field_renames
                .get(&qualified)
                .cloned()
                .unwrap_or(qualified),
        );
    }
    ids.sort();
    ids.join(", ")
}
