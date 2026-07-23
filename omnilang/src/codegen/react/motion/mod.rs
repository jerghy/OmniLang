// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React) — module root
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

mod keyframes;
mod transforms;

// Re-export sub-module public items so they are visible at the `motion` level
pub(super) use transforms::{
    emit_drag_effects_tsx, emit_driven_by_effects_tsx, emit_inview_effects_tsx,
    emit_scroll_effects_tsx,
};

/// Emit React hooks for @enter, @hover, @press, @focus motion triggers.
///
/// Scans for named motion triggers and emits:
/// 1. An `import { animate }` statement
/// 2. A `_M` constant mapping motion names → `{kf:..., opts:...}` objects
/// 3. A `useEffect` that handles @enter (one-shot on mount), @hover, @press,
///    and @focus (interactive event listeners) by calling `animate(el, kf, opts)`
///
/// `motion_bodies` maps motion name → its MotionBodyIR, from both component
/// and module-level @motion definitions.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
    signal_names: &FxHashSet<String>,
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

    // ── Emit useEffect that applies animations ──
    w.emit("useEffect(()=>{");
    w.indent();
    // @enter: one-shot mount animation
    w.emit("// @enter: mount animations");
    w.emit("document.querySelectorAll('[data-motion-enter]').forEach(el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("animate(el,kf,opts);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    // @hover: mouse enter/leave animations
    w.emit("// @hover: mouse enter/leave animations");
    w.emit("document.querySelectorAll('[data-motion-hover]').forEach(el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-hover-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("el.addEventListener('mouseenter',()=>animate(el,kf,opts));");
    w.emit("el.addEventListener('mouseleave',()=>animate(el,[...kf].reverse(),opts));");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    // @press: mouse down/up animations
    w.emit("// @press: mouse down/up animations");
    w.emit("document.querySelectorAll('[data-motion-press]').forEach(el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-press-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("el.addEventListener('mousedown',()=>animate(el,kf,opts));");
    w.emit("el.addEventListener('mouseup',()=>animate(el,[...kf].reverse(),opts));");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    // @focus: focus/blur animations
    w.emit("// @focus: focus/blur animations");
    w.emit("document.querySelectorAll('[data-motion-focus]').forEach(el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-focus-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("el.addEventListener('focus',()=>animate(el,kf,opts));");
    w.emit("el.addEventListener('blur',()=>animate(el,[...kf].reverse(),opts));");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    // Include signal names in dep array so @enter re-triggers on state changes
    if signal_names.is_empty() {
        w.emit("},[]);");
    } else {
        let mut sorted: Vec<&String> = signal_names.iter().collect();
        sorted.sort();
        let deps: Vec<&str> = sorted.iter().map(|s| s.as_str()).collect();
        let deps_str = deps.join(", ");
        w.emit_fmt(format_args!("}},[{}]);", deps_str));
    }
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
        _ => {}
    }
}
