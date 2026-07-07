// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React) — module root
// ═══════════════════════════════════════════════════════════

use std::collections::{HashMap, HashSet};

use crate::codegen::CodeWriter;
use crate::ir::*;

mod keyframes;
mod transforms;

// Re-export sub-module public items so they are visible at the `motion` level
pub(super) use transforms::{
    emit_drag_effects_tsx, emit_driven_by_effects_tsx, emit_inview_effects_tsx,
    emit_scroll_effects_tsx,
};

/// Emit React hooks for @enter motion triggers found in the view tree.
///
/// Scans for @enter triggers and emits:
/// 1. A `_M` constant mapping motion names → `[keyframes[], options{}]` tuples
/// 2. A `useEffect` that reads `data-motion-name` from each element,
///    looks up the motion definition, and calls `el.animate(kf, opt)`
///
/// `motion_bodies` maps motion name → its MotionBodyIR, from both component
/// and module-level @motion definitions.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &HashMap<String, MotionBodyIR>,
    signal_names: &HashSet<String>,
) {
    // Collect all unique motion names referenced by @enter triggers in this tree
    let motion_refs = collect_enter_motion_refs(node);
    if motion_refs.is_empty() {
        return;
    }

    // Filter to only those we have definitions for
    let defined_refs: HashSet<&String> = motion_refs
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
    let mut motion_warnings: Vec<String> = Vec::new();
    for name in &sorted {
        if let Some(body) = motion_bodies.get(name.as_str()) {
            let def_js = keyframes::format_motion_to_waapi(body, &mut motion_warnings);
            w.emit_fmt(format_args!("{}:{},", name, def_js));
        }
    }
    // Surface any motion conversion warnings as code comments
    for warn in &motion_warnings {
        w.emit_fmt(format_args!("// WARN: {}", warn));
    }
    w.dedent();
    w.emit("};");
    w.emit("");

    // ── Emit useEffect that applies animations ──
    w.emit("useEffect(()=>{");
    w.indent();
    w.emit("const els=document.querySelectorAll('[data-motion-enter]');");
    w.emit("els.forEach(el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const[kf,opt,setup]=_M[n];");
    w.emit("if(setup)setup(el);");
    w.emit("el.animate(kf,opt);");
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
/// Collect all unique motion names referenced by @enter triggers in a view tree.
fn collect_enter_motion_refs(node: &ViewTree) -> HashSet<String> {
    let mut refs = HashSet::new();
    collect_enter_motion_refs_inner(node, &mut refs);
    refs
}

fn collect_enter_motion_refs_inner(node: &ViewTree, refs: &mut HashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for trigger in el.motion_behavior().map_or(&[] as &[TriggerIntent], |m| &m.triggers) {
                if matches!(trigger.kind, TriggerKind::Enter) {
                    if let Some(ref name) = trigger.motion_ref {
                        refs.insert(name.clone());
                    }
                }
            }
            for child in &el.children {
                collect_enter_motion_refs_inner(child, refs);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(body) = &branch.body {
                    collect_enter_motion_refs_inner(body, refs);
                }
            }
            if let Some(fb) = &cond.fallback {
                collect_enter_motion_refs_inner(fb, refs);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(body) = &list.body {
                collect_enter_motion_refs_inner(body, refs);
            }
            if let Some(empty) = &list.empty_body {
                collect_enter_motion_refs_inner(empty, refs);
            }
        }
        _ => {}
    }
}
