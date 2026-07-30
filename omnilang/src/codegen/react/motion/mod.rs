// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React) — module root
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

mod keyframes;
#[cfg(test)]
mod tests;
mod transforms;

// Re-export sub-module public items so they are visible at the `motion` level
pub(super) use transforms::{
    emit_drag_effects_tsx, emit_inview_effects_tsx, emit_scroll_effects_tsx,
};

/// Emit React hooks for @enter, @hover, @press, @focus motion triggers.
///
/// Scans for named motion triggers and emits:
/// 1. A `_M` constant mapping motion names → `{kf:..., opts:...}` objects
/// 2. A `useEffect` with `[]` deps for @enter (one-shot on mount)
/// 3. A `useEffect` with `[]` deps for @hover/@press/@focus (interactive event
///    listeners with proper cleanup via returned teardown function)
///
/// `motion_bodies` maps motion name → its MotionBodyIR, from both component
/// and module-level @motion definitions.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
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

    // ── Effect 1: @enter (mount-only + MutationObserver for conditional rendering) ──
    w.emit("// @enter: mount animations + MutationObserver for conditional rendering");
    w.emit("useEffect(()=>{");
    w.indent();
    w.emit("const runEnter=el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("animate(el,kf,opts);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("};");
    w.emit("document.querySelectorAll('[data-motion-enter]').forEach(runEnter);");
    w.emit("const obs=new MutationObserver(muts=>{");
    w.indent();
    w.emit("muts.forEach(mut=>{");
    w.indent();
    w.emit("mut.addedNodes.forEach(node=>{");
    w.indent();
    w.emit("if(node.nodeType===1){");
    w.indent();
    w.emit("if(node.matches&&node.matches('[data-motion-enter]'))runEnter(node);");
    w.emit("if(node.querySelectorAll)node.querySelectorAll('[data-motion-enter]').forEach(runEnter);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.emit("obs.observe(document.body,{childList:true,subtree:true});");
    w.emit("return()=>obs.disconnect();");
    w.dedent();
    w.emit("},[]);");
    w.emit("");

    // ── Effect 2: interactive triggers (@hover, @press) with cleanup + MutationObserver ──
    w.emit("// @hover/@press: interactive triggers with cleanup + MutationObserver");
    w.emit("useEffect(()=>{");
    w.indent();
    w.emit("const hoverHandlers=[];");
    w.emit("const pressHandlers=[];");
    // @hover setup function
    w.emit("const setupHover=el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-hover-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("const enter=()=>animate(el,kf,opts);");
    w.emit("const leave=()=>animate(el,[...kf].reverse(),opts);");
    w.emit("el.addEventListener('mouseenter',enter);");
    w.emit("el.addEventListener('mouseleave',leave);");
    w.emit("hoverHandlers.push({el,enter,leave});");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("};");
    // @press setup function
    w.emit("const setupPress=el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-press-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const{kf,opts}=_M[n];");
    w.emit("const down=()=>animate(el,kf,opts);");
    w.emit("const up=()=>animate(el,[...kf].reverse(),opts);");
    w.emit("el.addEventListener('mousedown',down);");
    w.emit("el.addEventListener('mouseup',up);");
    w.emit("pressHandlers.push({el,down,up});");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("};");
    // Process existing elements
    w.emit("document.querySelectorAll('[data-motion-hover]').forEach(setupHover);");
    w.emit("document.querySelectorAll('[data-motion-press]').forEach(setupPress);");
    // MutationObserver for conditionally rendered elements
    w.emit("const obs=new MutationObserver(muts=>{");
    w.indent();
    w.emit("muts.forEach(mut=>{");
    w.indent();
    w.emit("mut.addedNodes.forEach(node=>{");
    w.indent();
    w.emit("if(node.nodeType===1){");
    w.indent();
    w.emit("if(node.matches&&node.matches('[data-motion-hover]'))setupHover(node);");
    w.emit("if(node.matches&&node.matches('[data-motion-press]'))setupPress(node);");
    w.emit("if(node.querySelectorAll){");
    w.indent();
    w.emit("node.querySelectorAll('[data-motion-hover]').forEach(setupHover);");
    w.emit("node.querySelectorAll('[data-motion-press]').forEach(setupPress);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.emit("obs.observe(document.body,{childList:true,subtree:true});");
    // Cleanup
    w.emit("return()=>{");
    w.indent();
    w.emit("hoverHandlers.forEach(({el,enter,leave})=>{");
    w.indent();
    w.emit("el.removeEventListener('mouseenter',enter);");
    w.emit("el.removeEventListener('mouseleave',leave);");
    w.dedent();
    w.emit("});");
    w.emit("pressHandlers.forEach(({el,down,up})=>{");
    w.indent();
    w.emit("el.removeEventListener('mousedown',down);");
    w.emit("el.removeEventListener('mouseup',up);");
    w.dedent();
    w.emit("});");
    w.emit("obs.disconnect();");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("},[]);");
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
