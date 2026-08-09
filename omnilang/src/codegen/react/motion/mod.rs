// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React) — module root
// ═══════════════════════════════════════════════════════════
//
// Motion triggers are emitted as per-element native effects driven by
// element refs (useRef + useCallback + useEffect), NOT data-attribute
// scanning (document.querySelectorAll / MutationObserver were removed).
// Each element with motion triggers gets a ref — the user's @ref when
// present, otherwise a synthesized `__el_N` ref — and its triggers emit
// either JSX handlers (@hover/@press) or useEffect blocks
// (@enter/@focus/@in-view/@scroll/@drag-*).

use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

mod keyframes;
#[cfg(test)]
mod tests;
mod transforms;

/// The ref naming contract shared by the JSX emitter (view.rs) and the
/// effect emitter (this module):
///
/// - A user `@ref` on the element wins: ref ident is `{name}Ref.current`.
/// - Otherwise the element is assigned `__el_{idx}` where `idx` is the
///   zero-based ordinal of the element among motion elements in DFS
///   pre-order (the same order both emitters traverse the view tree in).
///
/// Elements qualify as motion elements iff they are HTML elements (not a
/// `fragment` wrapper, not a custom component instance) carrying motion
/// triggers. The JSX emitter must call [`classify_motion_el`] exactly once
/// per element, at the same point the effect emitter counts it, so both
/// sides agree on ref names and handler names.
pub(super) fn classify_motion_el(
    el: &ElementIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    counter: &mut usize,
) -> Option<(String, usize)> {
    if el.tag == "fragment" || component_map.contains_key(el.tag.as_str()) {
        return None;
    }
    if el.motion_behavior().is_none_or(|m| m.triggers.is_empty()) {
        return None;
    }
    let idx = *counter;
    *counter += 1;
    let ref_name = el
        .ref_binding
        .clone()
        .unwrap_or_else(|| format!("__el_{}", idx));
    Some((ref_name, idx))
}

/// Walk the view tree in DFS pre-order, calling `visit` for each motion
/// element with its ordinal index. Uses the same rules as the JSX emitter
/// ([`classify_motion_el`]), so both sides assign identical ref names.
pub(super) fn walk_motion_elements(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    mut visit: impl FnMut(&ElementIR, usize),
) {
    let mut counter = 0usize;
    walk_motion_elements_inner(node, component_map, &mut counter, &mut visit);
}

fn walk_motion_elements_inner(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    counter: &mut usize,
    visit: &mut dyn FnMut(&ElementIR, usize),
) {
    match node {
        ViewTree::Element(el) => {
            if let Some((_, idx)) = classify_motion_el(el, component_map, counter) {
                visit(el, idx);
            } else if el.modal.is_some() {
                // @modal dialogs share the synthesized-ref namespace: they get
                // a `__el_{idx}` ref (same ordinal counter as motion elements)
                // so the JSX emitter and the effect emitter agree on names.
                // F3: a modal carrying a USER `@ref` does NOT consume the
                // shared counter — the JSX emitter uses the user's ref name
                // in that case (its `ref_binding` wins), so the walk must
                // mirror it or every motion element after the modal shifts
                // by one ordinal (`__el_0` in JSX vs `__el_1` here →
                // undeclared-ref TS2304). The element still gets visited so
                // the useLayoutEffect sync emits; only the counter is skipped.
                if el.ref_binding.is_none() {
                    let idx = *counter;
                    *counter += 1;
                    visit(el, idx);
                } else {
                    visit(el, 0);
                }
            }
            for child in &el.children {
                walk_motion_elements_inner(child, component_map, counter, visit);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_motion_elements_inner(body, component_map, counter, visit);
                }
            }
            if let Some(ref fb) = cond.fallback {
                walk_motion_elements_inner(fb, component_map, counter, visit);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                walk_motion_elements_inner(body, component_map, counter, visit);
            }
            if let Some(ref empty) = list.empty_body {
                walk_motion_elements_inner(empty, component_map, counter, visit);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_motion_elements_inner(child, component_map, counter, visit);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_motion_elements_inner(child, component_map, counter, visit);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Collect `(ref_name, open_signal)` pairs for every `@modal` dialog in the
/// view tree, in the same DFS pre-order the JSX emitter walks. Reuses
/// [`walk_motion_elements`]: the shared traversal already visits every
/// modal element (classify hit OR the modal-only branch), so the motion
/// counter — and therefore every synthesized `__el_{idx}` ref name — agrees
/// exactly with the JSX emitter and with `motion_internal_refs` (whose
/// `useRef` declarations the modal `useLayoutEffect` references).
pub(super) fn walk_modal_elements(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk_motion_elements(node, component_map, |el, idx| {
        if let Some(modal) = &el.modal {
            let ref_name = el
                .ref_binding
                .clone()
                .unwrap_or_else(|| format!("__el_{}", idx));
            out.push((ref_name, modal.open_signal.clone()));
        }
    });
    out
}

/// Whether the view declares any `@if mode: "fade"` conditional — gates the
/// CSS-module import: the fade rules (`.omni-if-fade` class + keyframes)
/// live in the component's `.module.css`.
pub(super) fn view_has_fade_conditional(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => el.children.iter().any(view_has_fade_conditional),
        ViewTree::Conditional(cond) => {
            cond.transition_mode.as_deref() == Some("fade")
                || cond
                    .branches
                    .iter()
                    .any(|b| b.body.as_ref().is_some_and(view_has_fade_conditional))
                || cond
                    .fallback
                    .as_ref()
                    .is_some_and(view_has_fade_conditional)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_fade_conditional)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_has_fade_conditional)
        }
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_fade_conditional),
        ViewTree::Fragment(children) => children.iter().any(view_has_fade_conditional),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Whether the view tree contains any `@skeleton` placeholder — gates the
/// shimmer keyframes emission (`.module.css`).
pub(super) fn view_has_skeleton(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => el.skeleton.is_some() || el.children.iter().any(view_has_skeleton),
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_skeleton))
                || cond.fallback.as_ref().is_some_and(view_has_skeleton)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_skeleton)
                || list.empty_body.as_ref().is_some_and(view_has_skeleton)
        }
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_skeleton),
        ViewTree::Fragment(children) => children.iter().any(view_has_skeleton),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Whether the view tree contains any `@errorBoundary` wrapper — gates the
/// emitted `ErrorBoundary` class and the `React` default import.
pub(super) fn view_has_error_boundary(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            el.error_boundary.is_some() || el.children.iter().any(view_has_error_boundary)
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_error_boundary))
                || cond.fallback.as_ref().is_some_and(view_has_error_boundary)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_error_boundary)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_has_error_boundary)
        }
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_error_boundary),
        ViewTree::Fragment(children) => children.iter().any(view_has_error_boundary),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Whether any motion element carries an interactive trigger (@hover or
/// @press) that emits useCallback handlers wired into the JSX — decides the
/// `useCallback` import.
pub(super) fn view_has_interactive_triggers(
    view: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> bool {
    let mut found = false;
    walk_motion_elements(view, component_map, |el, _| {
        if el.motion_behavior().is_some_and(|m| {
            m.triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::Hover | TriggerKind::Press))
        }) {
            found = true;
        }
    });
    found
}

/// Emit the `_M` motion table at MODULE scope (task 2.6, perf L1): motion
/// definitions are compile-time constants, so the table was re-allocated
/// inside the component function on EVERY render — and each re-allocation
/// made every `_M["x"]` use site a fresh object identity (exhaustive-deps
/// errors at every reference). Returns whether the table was hoisted; when
/// false the caller emits it in-function (a referenced motion carries
/// non-literal values — store fields/signals — resolved at render time).
///
/// Task 2.10 (perf M1) interaction, confirmed: a store-ref motion STAYS
/// in-function — after the slice channel its values reference component-
/// scope slice locals, which a module-level table cannot see; the hoist
/// gate (`motion_body_is_constant`) never accepted store refs anyway, so
/// the decision is unchanged. The benefit is subscription granularity only
/// (whole-store → per-field slices), not table placement. The module-scope
/// table formats with EMPTY renames: constant bodies cannot carry store
/// refs, so the renames are never consulted.
pub(super) fn emit_motion_table_module_scope(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) -> bool {
    let Some(sorted) = collect_defined_motion_refs(node, motion_bodies) else {
        return false;
    };
    let all_constant = sorted
        .iter()
        .all(|name| keyframes::motion_body_is_constant(&motion_bodies[name.as_str()]));
    if !all_constant {
        return false;
    }
    emit_motion_table(w, &sorted, motion_bodies, &FxHashMap::default());
    true
}

/// Emit React hooks for all motion triggers in the view tree.
///
/// 1. A `_M` constant mapping motion names → `{kf:...,opts:...}` objects
///    (WAAPI keyframes + animate options). Emitted at MODULE scope when
///    `table_hoisted` (the table is a compile-time constant — see
///    [`emit_motion_table_module_scope`]), else in-function.
/// 2. Per motion element, in DFS order:
///    - `useCallback` handlers for @hover / @press (wired into the JSX by
///      view.rs as onMouseEnter/onMouseLeave, onPointerDown/onPointerUp)
///    - `useEffect` blocks for @enter / @focus / @in-view / @scroll / @drag
///      driven by the element ref.
///
/// `motion_bodies` maps motion name → its MotionBodyIR, from both component
/// and module-level @motion definitions. `store_field_renames` (task 2.10,
/// perf M1) resolve `Store.field` reads inside motion values and trigger
/// control dimensions to the component's slice locals — the in-function
/// table and the per-element effects are both inside the component body,
/// where those locals are in scope.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
    component_map: &FxHashMap<&str, &ComponentIR>,
    table_hoisted: bool,
    store_field_renames: &FxHashMap<String, String>,
) {
    let Some(sorted) = collect_defined_motion_refs(node, motion_bodies) else {
        return;
    };

    if !table_hoisted {
        // ── Emit the MOTION_DEFS constant in-function ──
        // (a referenced motion carries non-literal values resolved at
        // render time — store fields / signals — formatted with the slice
        // renames, task 2.10)
        emit_motion_table(w, &sorted, motion_bodies, store_field_renames);
    }

    // ── Per-element effects ──
    walk_motion_elements(node, component_map, |el, idx| {
        let ref_name = el
            .ref_binding
            .clone()
            .unwrap_or_else(|| format!("__el_{}", idx));
        let ref_ident = format!("{}Ref.current", ref_name);
        emit_element_motion(w, el, idx, &ref_ident, motion_bodies, store_field_renames);
    });
}

/// The sorted names of the motions the view references AND has definitions
/// for; `None` when the view references no defined motion (nothing to emit).
fn collect_defined_motion_refs(
    node: &ViewTree,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) -> Option<Vec<String>> {
    // Collect all unique motion names referenced by triggers in this tree
    let motion_refs = collect_all_motion_refs(node);
    if motion_refs.is_empty() {
        return None;
    }
    // Filter to only those we have definitions for
    let mut sorted: Vec<String> = motion_refs
        .into_iter()
        .filter(|name| motion_bodies.contains_key(name.as_str()))
        .collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort();
    Some(sorted)
}

/// Emit the `_M` motion table body: `const _M={name:{kf:...,opts:...},...};`.
fn emit_motion_table(
    w: &mut CodeWriter,
    sorted: &[String],
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
    store_field_renames: &FxHashMap<String, String>,
) {
    w.emit("// -- Motion definitions --");
    w.emit("const _M={");
    w.indent();
    for name in sorted {
        if let Some(body) = motion_bodies.get(name.as_str()) {
            let def_js = keyframes::format_motion_to_animate(body, store_field_renames);
            w.emit_fmt(format_args!("{}:{},", name, def_js));
        }
    }
    w.dedent();
    w.emit("};");
    w.emit("");
}

/// Emit all motion hooks for a single motion element (in a fixed order:
/// interactive handlers first, then one-shot/observer effects).
fn emit_element_motion(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
    store_field_renames: &FxHashMap<String, String>,
) {
    emit_interactive_handlers(w, el, idx, ref_ident, motion_bodies);
    emit_enter_effect(w, el, idx, ref_ident, motion_bodies);
    emit_focus_effect(w, el, idx, ref_ident, motion_bodies);
    transforms::emit_inview_effect_tsx(w, el, idx, ref_ident, motion_bodies, store_field_renames);
    transforms::emit_scroll_effect_tsx(w, el, idx, ref_ident, motion_bodies, store_field_renames);
    transforms::emit_drag_effect_tsx(w, el, idx, ref_ident, motion_bodies, store_field_renames);
}

/// Find the first trigger of the given kind on an element.
/// (TriggerKind has no PartialEq derive — compare by discriminant.)
fn find_trigger(el: &ElementIR, kind: TriggerKind) -> Option<&TriggerIntent> {
    el.motion_behavior()?
        .triggers
        .iter()
        .find(|t| std::mem::discriminant(&t.kind) == std::mem::discriminant(&kind))
}

/// Resolve a trigger's motion definition: named motion lookup, falling back
/// to the inline body (inline bodies are normally hoisted to named motions
/// by lowering, but handle the field anyway).
pub(super) fn resolve_def<'a>(
    trigger: &'a TriggerIntent,
    motion_bodies: &'a FxHashMap<String, MotionBodyIR>,
) -> Option<&'a MotionBodyIR> {
    if let Some(ref name) = trigger.motion_ref {
        if let Some(body) = motion_bodies.get(name.as_str()) {
            return Some(body);
        }
    }
    trigger.inline_motion.as_ref()
}

/// `_M["name"]` JS reference for a trigger's named motion. Inline bodies are
/// hoisted to named motions by lowering, so this is always `Some` for a
/// resolvable trigger; `None` (defensive) skips emission.
fn def_ref_js(trigger: &TriggerIntent) -> Option<String> {
    trigger
        .motion_ref
        .as_deref()
        .map(|name| format!("_M[\"{}\"]", name))
}

/// Emit useCallback handlers for @hover / @press. view.rs wires them into
/// the JSX as onMouseEnter/onMouseLeave and onPointerDown/onPointerUp.
fn emit_interactive_handlers(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    // @hover: onMouseEnter plays the motion, onMouseLeave plays it reversed
    if let Some(trigger) = find_trigger(el, TriggerKind::Hover) {
        if resolve_def(trigger, motion_bodies).is_none() {
            return;
        }
        let Some(def_ref) = def_ref_js(trigger) else {
            return;
        };
        w.emit_fmt(format_args!(
            "// @hover: element {} — enter/leave handlers",
            idx
        ));
        w.emit_fmt(format_args!(
            "const handleEnter_{} = useCallback(() => {{",
            idx
        ));
        w.indent();
        w.emit_fmt(format_args!(
            "{}?.animate({}.kf, {}.opts);",
            ref_ident, def_ref, def_ref
        ));
        w.dedent();
        w.emit("}, []);");
        w.emit_fmt(format_args!(
            "const handleLeave_{} = useCallback(() => {{",
            idx
        ));
        w.indent();
        w.emit_fmt(format_args!("const el = {};", ref_ident));
        w.emit("if (!el) return;");
        w.emit_fmt(format_args!(
            "el.animate({}.kf.map(f => ({{ ...f, offset: 1 - f.offset }})).sort((a, b) => a.offset - b.offset), {}.opts);",
            def_ref, def_ref
        ));
        w.dedent();
        w.emit("}, []);");
        w.emit("");
    }

    // @press: onPointerDown plays the motion, onPointerUp plays it reversed
    if let Some(trigger) = find_trigger(el, TriggerKind::Press) {
        if resolve_def(trigger, motion_bodies).is_none() {
            return;
        }
        let Some(def_ref) = def_ref_js(trigger) else {
            return;
        };
        w.emit_fmt(format_args!(
            "// @press: element {} — down/up handlers",
            idx
        ));
        w.emit_fmt(format_args!(
            "const handleDown_{} = useCallback(() => {{",
            idx
        ));
        w.indent();
        w.emit_fmt(format_args!(
            "{}?.animate({}.kf, {}.opts);",
            ref_ident, def_ref, def_ref
        ));
        w.dedent();
        w.emit("}, []);");
        w.emit_fmt(format_args!(
            "const handleUp_{} = useCallback(() => {{",
            idx
        ));
        w.indent();
        w.emit_fmt(format_args!("const el = {};", ref_ident));
        w.emit("if (!el) return;");
        w.emit_fmt(format_args!(
            "el.animate({}.kf.map(f => ({{ ...f, offset: 1 - f.offset }})).sort((a, b) => a.offset - b.offset), {}.opts);",
            def_ref, def_ref
        ));
        w.dedent();
        w.emit("}, []);");
        w.emit("");
    }
}

/// @enter: one-shot animation on mount, cancelled on unmount.
fn emit_enter_effect(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    let Some(trigger) = find_trigger(el, TriggerKind::Enter) else {
        return;
    };
    if resolve_def(trigger, motion_bodies).is_none() {
        return;
    }
    let Some(def_ref) = def_ref_js(trigger) else {
        return;
    };
    w.emit_fmt(format_args!("// @enter: element {} — mount animation", idx));
    w.emit("useEffect(() => {");
    w.indent();
    w.emit_fmt(format_args!("const el = {};", ref_ident));
    w.emit("if (!el) return;");
    w.emit_fmt(format_args!(
        "const anim = el.animate({}.kf, {}.opts);",
        def_ref, def_ref
    ));
    w.emit("return () => anim.cancel();");
    w.dedent();
    w.emit("}, []);");
    w.emit("");
}

/// @focus: focusin/focusout listeners on the ref'd element. Focus plays the
/// motion, blur plays it reversed.
fn emit_focus_effect(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    let Some(trigger) = find_trigger(el, TriggerKind::Focus) else {
        return;
    };
    if resolve_def(trigger, motion_bodies).is_none() {
        return;
    }
    let Some(def_ref) = def_ref_js(trigger) else {
        return;
    };
    w.emit_fmt(format_args!(
        "// @focus: element {} — focusin/focusout listeners",
        idx
    ));
    w.emit("useEffect(() => {");
    w.indent();
    w.emit_fmt(format_args!("const el = {};", ref_ident));
    w.emit("if (!el) return;");
    w.emit_fmt(format_args!(
        "const onFocus = () => {{ el.animate({}.kf, {}.opts); }};",
        def_ref, def_ref
    ));
    w.emit_fmt(format_args!(
        "const onBlur = () => {{ el.animate({}.kf.map(f => ({{ ...f, offset: 1 - f.offset }})).sort((a, b) => a.offset - b.offset), {}.opts); }};",
        def_ref, def_ref
    ));
    w.emit("el.addEventListener('focusin', onFocus);");
    w.emit("el.addEventListener('focusout', onBlur);");
    w.emit("return () => {");
    w.indent();
    w.emit("el.removeEventListener('focusin', onFocus);");
    w.emit("el.removeEventListener('focusout', onBlur);");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
    w.emit("");
}

/// ── Helpers ────────────────────────────────────────────────────────────
/// Collect all unique motion names referenced by any trigger in a view tree.
fn collect_all_motion_refs(node: &ViewTree) -> FxHashSet<String> {
    let mut refs = FxHashSet::default();
    collect_all_motion_refs_inner(node, &mut refs);
    refs
}

fn collect_all_motion_refs_inner(node: &ViewTree, refs: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for trigger in el
                .motion_behavior()
                .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
            {
                if let Some(ref name) = trigger.motion_ref {
                    refs.insert(name.clone());
                }
            }
            for child in &el.children {
                collect_all_motion_refs_inner(child, refs);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(body) = &branch.body {
                    collect_all_motion_refs_inner(body, refs);
                }
            }
            if let Some(fb) = &cond.fallback {
                collect_all_motion_refs_inner(fb, refs);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(body) = &list.body {
                collect_all_motion_refs_inner(body, refs);
            }
            if let Some(empty) = &list.empty_body {
                collect_all_motion_refs_inner(empty, refs);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_all_motion_refs_inner(child, refs);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_all_motion_refs_inner(child, refs);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}
