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

/// Emit React hooks for all motion triggers in the view tree.
///
/// 1. A `_M` constant mapping motion names → `{kf:...,opts:...}` objects
///    (WAAPI keyframes + animate options).
/// 2. Per motion element, in DFS order:
///    - `useCallback` handlers for @hover / @press (wired into the JSX by
///      view.rs as onMouseEnter/onMouseLeave, onPointerDown/onPointerUp)
///    - `useEffect` blocks for @enter / @focus / @in-view / @scroll / @drag
///      driven by the element ref.
///
/// `motion_bodies` maps motion name → its MotionBodyIR, from both component
/// and module-level @motion definitions.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
    component_map: &FxHashMap<&str, &ComponentIR>,
) {
    // Collect all unique motion names referenced by triggers in this tree
    let motion_refs = collect_all_motion_refs(node);
    if motion_refs.is_empty() {
        return;
    }

    // Filter to only those we have definitions for
    let defined_refs: FxHashSet<&String> = motion_refs
        .iter()
        .filter(|name| motion_bodies.contains_key(name.as_str()))
        .collect();
    if defined_refs.is_empty() {
        return;
    }

    // ── Emit the MOTION_DEFS constant ──
    w.emit("// -- Motion definitions --");
    w.emit("const _M={");
    w.indent();

    let mut sorted: Vec<&String> = defined_refs.into_iter().collect();
    sorted.sort();
    for name in &sorted {
        if let Some(body) = motion_bodies.get(name.as_str()) {
            let def_js = keyframes::format_motion_to_animate(body);
            w.emit_fmt(format_args!("{}:{},", name, def_js));
        }
    }
    w.dedent();
    w.emit("};");
    w.emit("");

    // ── Per-element effects ──
    walk_motion_elements(node, component_map, |el, idx| {
        let ref_name = el
            .ref_binding
            .clone()
            .unwrap_or_else(|| format!("__el_{}", idx));
        let ref_ident = format!("{}Ref.current", ref_name);
        emit_element_motion(w, el, idx, &ref_ident, motion_bodies);
    });
}

/// Emit all motion hooks for a single motion element (in a fixed order:
/// interactive handlers first, then one-shot/observer effects).
fn emit_element_motion(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    emit_interactive_handlers(w, el, idx, ref_ident, motion_bodies);
    emit_enter_effect(w, el, idx, ref_ident, motion_bodies);
    emit_focus_effect(w, el, idx, ref_ident, motion_bodies);
    transforms::emit_inview_effect_tsx(w, el, idx, ref_ident, motion_bodies);
    transforms::emit_scroll_effect_tsx(w, el, idx, ref_ident, motion_bodies);
    transforms::emit_drag_effect_tsx(w, el, idx, ref_ident, motion_bodies);
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
