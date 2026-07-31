// ═══════════════════════════════════════════════════════════
// State codegen: signal → useState / setter codegen
// ═══════════════════════════════════════════════════════════

use crate::codegen::capitalize_first;
use crate::codegen::react::expr;
use crate::codegen::react::ReactCodegen;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashSet;

use super::collect_expr_identifiers;
use super::collect_stmt_identifiers;

impl ReactCodegen {
    /// Generate useState hooks for component state signals.
    /// Only generates setter if signal is mutated; non-mutated signals use plain const.
    /// Skips states never read in view/computeds/effects/lifecycles/called-fns.
    pub(super) fn gen_state_hooks(
        &self,
        w: &mut CodeWriter,
        comp: &ComponentIR,
        read_signals: &FxHashSet<String>,
        mutated_signals: &FxHashSet<String>,
        fn_names: &FxHashSet<String>,
    ) {
        for state in &comp.signals {
            if !read_signals.contains(&state.name) {
                continue;
            }
            let init_val = expr::format_expr_ir_tsx(&state.initial_value, &FxHashSet::default());
            if mutated_signals.contains(&state.name) {
                let setter_base = format!("set{}", capitalize_first(&state.name));
                let setter = if fn_names.contains(&setter_base) {
                    format!("__{}", setter_base)
                } else {
                    setter_base
                };
                w.emit_fmt(format_args!(
                    "const [{}, {}] = useState({});",
                    state.name, setter, init_val
                ));
            } else {
                w.emit_fmt(format_args!("const {} = {};", state.name, init_val));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Signal mutation & usage tracking
// ═══════════════════════════════════════════════════════════

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
        StatementIR::EventEmitStmtIR { .. }
        | StatementIR::BreakStmtIR
        | StatementIR::ContinueStmtIR => {}
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

/// Generate `useRef<T>(null)` declarations for each ref in the component.
/// Maps known HTML element tags to their TypeScript types (e.g., input→HTMLInputElement).
/// Falls back to HTMLElement for unknown or missing tags.
/// Called after gen_state_hooks and before gen_effects in gen_component_tsx.
pub(super) fn gen_ref_hooks(w: &mut CodeWriter, comp: &ComponentIR) {
    if comp.refs.is_empty() {
        return;
    }
    for ref_decl in &comp.refs {
        let ts_type = ref_element_type(ref_decl.element_tag.as_deref());
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<{}>(null);",
            ref_decl.name, ts_type
        ));
    }
    w.emit("");
}

/// Map an OmniView HTML element tag to its TypeScript element type.
fn ref_element_type(tag: Option<&str>) -> &str {
    match tag {
        Some("input") | Some("textarea") => "HTMLInputElement",
        Some("select") => "HTMLSelectElement",
        Some("button") => "HTMLButtonElement",
        Some("form") => "HTMLFormElement",
        Some("img") | Some("image") => "HTMLImageElement",
        Some("video") => "HTMLVideoElement",
        Some("audio") => "HTMLAudioElement",
        Some("canvas") => "HTMLCanvasElement",
        Some("a") | Some("link") => "HTMLAnchorElement",
        Some("table") => "HTMLTableElement",
        Some("svg") => "SVGSVGElement",
        Some("div") | Some("section") | Some("article") | Some("nav") | Some("header")
        | Some("footer") | Some("main") | Some("aside") => "HTMLDivElement",
        // For unknown tags, use HTMLElement (generic) — better than HTMLDivElement
        _ => "HTMLElement",
    }
}
