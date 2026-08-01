// ═══════════════════════════════════════════════════════════
// Trigger effects: @in-view, @scroll, @drag (per-ref, no DOM scanning)
// ═══════════════════════════════════════════════════════════
//
// Each effect targets one element ref, created at compile time from the
// element's trigger + behavior IR. No data-* attributes, no
// document.querySelectorAll, no MutationObserver.

use crate::codegen::react::expr;
use crate::codegen::react::motion::resolve_def;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Find the first trigger of the given kind on an element.
/// (TriggerKind has no PartialEq derive — compare by discriminant.)
fn find_trigger(el: &ElementIR, kind: TriggerKind) -> Option<&TriggerIntent> {
    el.motion_behavior()?
        .triggers
        .iter()
        .find(|t| std::mem::discriminant(&t.kind) == std::mem::discriminant(&kind))
}

/// `_M["name"]` JS reference for a trigger's named motion (inline bodies are
/// hoisted to named motions by lowering).
fn def_ref_js(trigger: &TriggerIntent) -> Option<String> {
    trigger
        .motion_ref
        .as_deref()
        .map(|name| format!("_M[\"{}\"]", name))
}

// ══════════════════════════════════════════════════════════════════════════
// @in-view — IntersectionObserver per ref
// ══════════════════════════════════════════════════════════════════════════

/// Emit a per-ref useEffect for @in-view triggers using IntersectionObserver.
/// On first intersection the motion plays and the observer disconnects
/// (repeat behavior can be re-enabled when the in_view_repeat attr lands).
pub(super) fn emit_inview_effect_tsx(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    let Some(trigger) = find_trigger(el, TriggerKind::InView) else {
        return;
    };
    if resolve_def(trigger, motion_bodies).is_none() {
        return;
    }
    let Some(def_ref) = def_ref_js(trigger) else {
        return;
    };
    let threshold = el
        .motion_behavior()
        .and_then(|m| m.in_view_threshold.as_ref())
        .map_or_else(
            || "0".to_string(),
            |e| expr::format_expr_ir_tsx(e, &FxHashSet::default()),
        );

    w.emit_fmt(format_args!(
        "// @in-view: element {} — IntersectionObserver",
        idx
    ));
    w.emit("useEffect(() => {");
    w.indent();
    w.emit_fmt(format_args!("const el = {};", ref_ident));
    w.emit("if (!el) return;");
    w.emit_fmt(format_args!("const threshold = {};", threshold));
    w.emit("const observer = new IntersectionObserver((entries) => {");
    w.indent();
    w.emit("if (entries[0].isIntersecting) {");
    w.indent();
    w.emit_fmt(format_args!(
        "el.animate({}.kf, {}.opts);",
        def_ref, def_ref
    ));
    w.emit("observer.disconnect();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}, { threshold });");
    w.emit("observer.observe(el);");
    w.emit("return () => observer.disconnect();");
    w.dedent();
    w.emit("}, []);");
    w.emit("");
}

// ══════════════════════════════════════════════════════════════════════════
// @scroll — scroll-driven animation progress
// ══════════════════════════════════════════════════════════════════════════

/// Emit a per-ref useEffect for @scroll triggers: the motion is created
/// paused and scrubbed by a window scroll listener using the element's
/// viewport-relative position (start/end configs from the scroll behavior,
/// defaults "top 80%" → "bottom 20%").
pub(super) fn emit_scroll_effect_tsx(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    let Some(trigger) = find_trigger(el, TriggerKind::Scroll) else {
        return;
    };
    if resolve_def(trigger, motion_bodies).is_none() {
        return;
    }
    let Some(def_ref) = def_ref_js(trigger) else {
        return;
    };
    let scroll = el.scroll_behavior();
    let start_str = scroll.and_then(|s| s.start.as_deref()).unwrap_or("top 80%");
    let end_str = scroll
        .and_then(|s| s.end.as_deref())
        .unwrap_or("bottom 20%");
    let scrub_js = scroll.and_then(|s| s.scrub.as_ref()).map_or_else(
        || "0".to_string(),
        |e| expr::format_expr_ir_tsx(e, &FxHashSet::default()),
    );

    w.emit_fmt(format_args!(
        "// @scroll: element {} — scroll-driven progress",
        idx
    ));
    w.emit("useEffect(() => {");
    w.indent();
    w.emit_fmt(format_args!("const el = {};", ref_ident));
    w.emit("if (!el) return;");
    w.emit_fmt(format_args!(
        "const anim = el.animate({}.kf, {}.opts);",
        def_ref, def_ref
    ));
    w.emit("anim.pause();");
    w.emit("const parseScrollTrigger = (cfg) => {");
    w.indent();
    w.emit("if (!cfg || typeof cfg !== 'string') return 0;");
    w.emit("const parts = cfg.split(' ');");
    w.emit("if (parts.length < 2) return 0;");
    w.emit("const edge = parts[0];");
    w.emit("const offset = parts[1];");
    w.emit("const vh = window.innerHeight;");
    w.emit("const elRect = el.getBoundingClientRect();");
    w.emit("let elPos;");
    w.emit("if (edge === 'top') elPos = elRect.top;");
    w.emit("else if (edge === 'center') elPos = elRect.top + elRect.height / 2;");
    w.emit("else if (edge === 'bottom') elPos = elRect.bottom;");
    w.emit("else elPos = elRect.top;");
    w.emit("if (offset.endsWith('%')) return window.scrollY + elPos - vh * (parseFloat(offset) / 100);");
    w.emit("else if (offset === 'bottom') return window.scrollY + elPos - vh;");
    w.emit("else if (offset === 'top') return window.scrollY + elPos;");
    w.emit("return window.scrollY + elPos - vh * 0.5;");
    w.dedent();
    w.emit("};");
    w.emit_fmt(format_args!(
        "const startPos = parseScrollTrigger(\"{}\");",
        start_str
    ));
    w.emit_fmt(format_args!(
        "const endPos = parseScrollTrigger(\"{}\");",
        end_str
    ));
    w.emit("let scrubCache = 0;");
    w.emit("const handleScroll = () => {");
    w.indent();
    w.emit("const raw = (window.scrollY - startPos) / (endPos - startPos);");
    w.emit("const target = Math.max(0, Math.min(1, raw));");
    w.emit_fmt(format_args!("const scrub = {};", scrub_js));
    w.emit("const scrubbed = scrub > 0 ? scrubCache + (target - scrubCache) * Math.min(1, 16.67 / (scrub * 1000)) : target;");
    w.emit("scrubCache = scrubbed;");
    w.emit("const duration = Number(anim.effect?.getTiming()?.duration) || 300;");
    w.emit("anim.currentTime = scrubbed * duration;");
    w.dedent();
    w.emit("};");
    w.emit("window.addEventListener('scroll', handleScroll, { passive: true });");
    w.emit("handleScroll();");
    w.emit("return () => {");
    w.indent();
    w.emit("window.removeEventListener('scroll', handleScroll);");
    w.emit("anim.cancel();");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
    w.emit("");
}

// ══════════════════════════════════════════════════════════════════════════
// @drag — gesture-driven animation progress
// ══════════════════════════════════════════════════════════════════════════

/// Emit a per-ref useEffect for @drag-horizontal / @drag-vertical triggers:
/// the motion is created paused and scrubbed by pointer events on the ref'd
/// element (distance-to-progress mapping with elastic constraints).
pub(super) fn emit_drag_effect_tsx(
    w: &mut CodeWriter,
    el: &ElementIR,
    idx: usize,
    ref_ident: &str,
    motion_bodies: &FxHashMap<String, MotionBodyIR>,
) {
    let Some(trigger) = find_drag_trigger(el) else {
        return;
    };
    if resolve_def(trigger, motion_bodies).is_none() {
        return;
    }
    let Some(def_ref) = def_ref_js(trigger) else {
        return;
    };
    let axis = if matches!(trigger.kind, TriggerKind::DragHorizontal) {
        "x"
    } else {
        "y"
    };
    let drag = el.drag_behavior();
    let constraints = drag.and_then(|d| d.constraints.as_deref()).unwrap_or("");
    // ADR-053 removed @drag-constraints — without them the gesture progress
    // maps onto the motion's own keyframe range (the distance the animation
    // travels in the drag axis), so the element follows its animation instead
    // of dividing by Infinity (deferral #19).
    let derived_extent = if constraints.is_empty() {
        resolve_def(trigger, motion_bodies)
            .and_then(|body| crate::codegen::shared::drag::drag_extent(body, axis == "x"))
    } else {
        None
    };
    let elastic_js = drag.and_then(|d| d.elastic.as_ref()).map_or_else(
        || "0".to_string(),
        |e| expr::format_expr_ir_tsx(e, &FxHashSet::default()),
    );
    let threshold_js = drag.and_then(|d| d.threshold.as_ref()).map_or_else(
        || "0".to_string(),
        |e| expr::format_expr_ir_tsx(e, &FxHashSet::default()),
    );

    w.emit_fmt(format_args!(
        "// @drag-{}: element {} — gesture-driven progress",
        axis, idx
    ));
    w.emit("useEffect(() => {");
    w.indent();
    w.emit_fmt(format_args!("const el = {};", ref_ident));
    w.emit("if (!el) return;");
    w.emit_fmt(format_args!(
        "const anim = el.animate({}.kf, {}.opts);",
        def_ref, def_ref
    ));
    w.emit("anim.pause();");
    // `: string` prevents literal-type narrowing (TS2367 on `axis === 'x'`)
    w.emit_fmt(format_args!("const axis: string = \"{}\";", axis));
    w.emit_fmt(format_args!("const constraints = \"{}\";", constraints));
    w.emit_fmt(format_args!("const elastic = {};", elastic_js));
    w.emit_fmt(format_args!("const threshold = {};", threshold_js));
    w.emit("const parseConstraint = (cfg) => {");
    w.indent();
    w.emit("if (!cfg) return { min: -Infinity, max: Infinity };");
    w.emit("if (cfg === 'parent') {");
    w.indent();
    w.emit("const parent = el.parentElement;");
    w.emit("if (!parent) return { min: -Infinity, max: Infinity };");
    w.emit("const elRect = el.getBoundingClientRect();");
    w.emit("const parentRect = parent.getBoundingClientRect();");
    w.emit("return { min: 0, max: parentRect.width - elRect.width };");
    w.dedent();
    w.emit("}");
    w.emit("const parts = cfg.split(' ');");
    w.emit("let min = -Infinity, max = Infinity;");
    w.emit("parts.forEach(p => {");
    w.indent();
    w.emit("const [key, val] = p.split(':');");
    w.emit("if (key === 'left' || key === 'top') min = parseFloat(val) || 0;");
    w.emit("if (key === 'right' || key === 'bottom') max = parseFloat(val) || Infinity;");
    w.dedent();
    w.emit("});");
    w.emit("return { min, max };");
    w.dedent();
    w.emit("};");
    w.emit("const constraint = parseConstraint(constraints);");
    if !constraints.is_empty() {
        w.emit("const maxExtent = Math.abs(constraint.max - constraint.min) || 1;");
    } else if let Some(extent) = derived_extent {
        // No constraints: derive the extent from the motion's own keyframe
        // range in the drag axis, so dragging moves the element along its
        // animation (translateX 0→300 ⇒ maxExtent 300).
        w.emit_fmt(format_args!(
            "// no constraints — maxExtent {}px derived from the motion's keyframe range",
            extent
        ));
        w.emit_fmt(format_args!(
            "const maxExtent = {};",
            crate::codegen::shared::drag::format_extent_js(extent)
        ));
    } else {
        // The motion has no translateX/Y range to map onto — fall back to the
        // previous unbounded behavior (progress stays 0, drag is inert) and
        // surface a compiler warning comment in the generated code.
        w.emit("// WARNING: @drag-* without constraints and no literal translateX/Y keyframe values in the motion — drag is unbounded (no movement); add a literal translate keyframe range to the motion");
        w.emit("const maxExtent = Number.POSITIVE_INFINITY;");
    }
    w.emit("let startPos = 0, currentDelta = 0, isDragging = false;");
    w.emit("const onPointerDown = (e) => {");
    w.indent();
    w.emit("isDragging = true;");
    w.emit("startPos = axis === 'x' ? e.clientX : e.clientY;");
    w.emit("currentDelta = 0;");
    w.emit("el.setPointerCapture(e.pointerId);");
    w.dedent();
    w.emit("};");
    w.emit("const onPointerMove = (e) => {");
    w.indent();
    w.emit("if (!isDragging) return;");
    w.emit("const raw = (axis === 'x' ? e.clientX : e.clientY) - startPos;");
    w.emit("let constrained = raw;");
    w.emit("if (raw < constraint.min) {");
    w.indent();
    w.emit("constrained = constraint.min - (constraint.min - raw) * (1 - elastic);");
    w.dedent();
    w.emit("} else if (raw > constraint.max) {");
    w.indent();
    w.emit("constrained = constraint.max + (raw - constraint.max) * (1 - elastic);");
    w.dedent();
    w.emit("}");
    w.emit("currentDelta = raw;");
    w.emit("const progress = Math.min(1, Math.abs(constrained) / maxExtent);");
    w.emit("const duration = Number(anim.effect?.getTiming()?.duration) || 300;");
    w.emit("anim.currentTime = progress * duration;");
    w.dedent();
    w.emit("};");
    w.emit("const onPointerUp = (e) => {");
    w.indent();
    w.emit("isDragging = false;");
    w.emit("el.releasePointerCapture(e.pointerId);");
    w.emit("const progress = Math.abs(currentDelta) / maxExtent;");
    w.emit("if (progress >= threshold && maxExtent > 0) {");
    w.indent();
    w.emit("anim.play();");
    w.dedent();
    w.emit("} else {");
    w.indent();
    w.emit("anim.reverse();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("};");
    w.emit("el.addEventListener('pointerdown', onPointerDown);");
    w.emit("el.addEventListener('pointermove', onPointerMove);");
    w.emit("el.addEventListener('pointerup', onPointerUp);");
    w.emit("el.addEventListener('pointercancel', onPointerUp);");
    w.emit("return () => {");
    w.indent();
    w.emit("el.removeEventListener('pointerdown', onPointerDown);");
    w.emit("el.removeEventListener('pointermove', onPointerMove);");
    w.emit("el.removeEventListener('pointerup', onPointerUp);");
    w.emit("el.removeEventListener('pointercancel', onPointerUp);");
    w.emit("anim.cancel();");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
    w.emit("");
}

/// Find the first drag trigger on an element (horizontal or vertical).
fn find_drag_trigger(el: &ElementIR) -> Option<&TriggerIntent> {
    el.motion_behavior()?.triggers.iter().find(|t| {
        matches!(
            t.kind,
            TriggerKind::DragHorizontal | TriggerKind::DragVertical
        )
    })
}
