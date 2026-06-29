// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React)
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;

/// Emit React hooks for @enter motion triggers found in the view tree.
/// Scans recursively for @enter triggers and emits a single `useEffect`
/// that animates all @enter elements using `data-motion-enter` attribute.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    _ref_counter: &mut u32,
    _comp_name: &str,
) {
    // First pass: check if any @enter triggers exist in the tree
    if !has_enter_trigger(node) {
        return;
    }

    // Emit a single useEffect that animates all elements with data-motion-enter
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-enter]');");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 300, easing: 'ease-out', fill: 'both' });");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("}, []);");
}

/// Recursively check if a view tree contains any @enter motion triggers.
pub(super) fn has_enter_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::Enter))
            {
                return true;
            }
            el.children.iter().any(|c| has_enter_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .map_or(false, |body| has_enter_trigger(body))
            }) || cond
                .fallback
                .as_ref()
                .map_or(false, |fb| has_enter_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_enter_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_enter_trigger(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @driven-by external progress control.
/// Uses data-driven-by attributes on JSX elements and a useEffect that watches
/// the driven signal and seeks WAAPI animations accordingly.
pub(super) fn emit_driven_by_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_driven_by(node) {
        return;
    }

    // Emit a single useEffect that handles all driven-by elements.
    // Each element has a data-driven-by attribute with the signal expression.
    w.emit("// @driven-by bindings: external progress control via signal");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-driven-by]');");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const drivenVal = el.getAttribute('data-driven-by');");
    w.emit("const progress = Math.max(0, Math.min(1, Number(drivenVal)));");
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => {");
    w.indent();
    w.emit("if (a.playState !== 'finished') {");
    w.indent();
    w.emit("const duration = a.effect?.getTiming()?.duration || 300;");
    w.emit("a.currentTime = progress * duration;");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
}

/// Check if any element in the view tree has @driven-by.
pub(super) fn has_driven_by(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el.driven_by.is_some() {
                return true;
            }
            el.children.iter().any(|c| has_driven_by(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().map_or(false, |body| has_driven_by(body)))
                || cond.fallback.as_ref().map_or(false, |fb| has_driven_by(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().map_or(false, |body| has_driven_by(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_driven_by(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @in-view triggers using IntersectionObserver.
pub(super) fn emit_inview_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_inview_trigger(node) {
        return;
    }

    w.emit("// @in-view: IntersectionObserver for entrance animations");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-inview]');");
    w.emit("const observers = [];");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const threshold = parseFloat(el.getAttribute('data-inview-threshold') || '0');");
    w.emit("const observer = new IntersectionObserver((entries) => {");
    w.indent();
    w.emit("entries.forEach(entry => {");
    w.indent();
    w.emit("if (entry.isIntersecting) {");
    w.indent();
    w.emit("el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 300, easing: 'ease-out', fill: 'both' });");
    w.emit("observer.disconnect();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("}, { threshold });");
    w.emit("observer.observe(el);");
    w.emit("observers.push(observer);");
    w.dedent();
    w.emit("});");
    w.emit("return () => observers.forEach(o => o.disconnect());");
    w.dedent();
    w.emit("}, []);");
}

/// Recursively check if view tree has any @in-view triggers.
pub(super) fn has_inview_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::InView))
            {
                return true;
            }
            el.children.iter().any(|c| has_inview_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .map_or(false, |body| has_inview_trigger(body))
            }) || cond
                .fallback
                .as_ref()
                .map_or(false, |fb| has_inview_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_inview_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_inview_trigger(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @scroll triggers using scroll-driven animation progress.
pub(super) fn emit_scroll_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_scroll_trigger(node) {
        return;
    }

    w.emit("// @scroll: scroll-driven animation progress");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-scroll]');");
    w.emit("if (!els.length) return;");
    w.emit("function parseScrollTrigger(cfg, el, vh) {");
    w.indent();
    w.emit("if (!cfg || typeof cfg !== 'string') return 0;");
    w.emit("const parts = cfg.split(' ');");
    w.emit("if (parts.length < 2) return 0;");
    w.emit("const edge = parts[0];");
    w.emit("const offset = parts[1];");
    w.emit("const elRect = el.getBoundingClientRect();");
    w.emit("let elPos;");
    w.emit("if (edge === 'top') elPos = elRect.top;");
    w.emit("else if (edge === 'center') elPos = elRect.top + elRect.height / 2;");
    w.emit("else if (edge === 'bottom') elPos = elRect.bottom;");
    w.emit("else elPos = elRect.top;");
    w.emit("if (offset.endsWith('%')) {");
    w.indent();
    w.emit("return window.scrollY + elPos - vh * (parseFloat(offset) / 100);");
    w.dedent();
    w.emit("} else if (offset === 'bottom') {");
    w.indent();
    w.emit("return window.scrollY + elPos - vh;");
    w.dedent();
    w.emit("} else if (offset === 'top') {");
    w.indent();
    w.emit("return window.scrollY + elPos;");
    w.dedent();
    w.emit("}");
    w.emit("return window.scrollY + elPos - vh * 0.5;");
    w.dedent();
    w.emit("}");
    w.emit("function computeProgress(el) {");
    w.indent();
    w.emit("const rect = el.getBoundingClientRect();");
    w.emit("const vh = window.innerHeight;");
    w.emit("const startCfg = el.getAttribute('data-scroll-start') || 'top 80%';");
    w.emit("const endCfg = el.getAttribute('data-scroll-end') || 'bottom 20%';");
    w.emit("const startPos = parseScrollTrigger(startCfg, el, vh);");
    w.emit("const endPos = parseScrollTrigger(endCfg, el, vh);");
    w.emit("const raw = (window.scrollY - startPos) / (endPos - startPos);");
    w.emit("return Math.max(0, Math.min(1, raw));");
    w.dedent();
    w.emit("}");
    w.emit("function scrubValue(el, target) {");
    w.indent();
    w.emit("const delay = parseFloat(el.getAttribute('data-scroll-scrub') || '0');");
    w.emit("if (delay > 0) {");
    w.indent();
    w.emit("return (scrubCache.get(el) || 0) + (target - (scrubCache.get(el) || 0)) * Math.min(1, 16.67 / (delay * 1000));");
    w.dedent();
    w.emit("}");
    w.emit("return target;");
    w.dedent();
    w.emit("}");
    w.emit("const scrubCache = new Map();");
    w.emit("const handleScroll = () => {");
    w.indent();
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const target = computeProgress(el);");
    w.emit("const scrubbed = scrubValue(el, target);");
    w.emit("scrubCache.set(el, scrubbed);");
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => {");
    w.indent();
    w.emit("const duration = a.effect?.getTiming()?.duration || 300;");
    w.emit("a.currentTime = scrubbed * duration;");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("};");
    w.emit("window.addEventListener('scroll', handleScroll, { passive: true });");
    w.emit("handleScroll();");
    w.emit("return () => window.removeEventListener('scroll', handleScroll);");
    w.dedent();
    w.emit("}, []);");
}

/// Recursively check if view tree has any @scroll triggers.
pub(super) fn has_scroll_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::Scroll))
            {
                return true;
            }
            el.children.iter().any(|c| has_scroll_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .map_or(false, |body| has_scroll_trigger(body))
            }) || cond
                .fallback
                .as_ref()
                .map_or(false, |fb| has_scroll_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_scroll_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_scroll_trigger(empty))
        }
        _ => false,
    }
}

/// Recursively check if view tree has any @drag-horizontal or @drag-vertical triggers.
pub(super) fn has_drag_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el.motion_triggers.iter().any(|t| {
                matches!(
                    t.kind,
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical
                )
            }) {
                return true;
            }
            el.children.iter().any(|c| has_drag_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().map_or(false, |body| has_drag_trigger(body)))
                || cond
                    .fallback
                    .as_ref()
                    .map_or(false, |fb| has_drag_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_drag_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_drag_trigger(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @drag-horizontal / @drag-vertical triggers using pointer events.
pub(super) fn emit_drag_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_drag_trigger(node) {
        return;
    }

    w.emit("// @drag-horizontal / @drag-vertical: gesture-driven animation progress");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-drag]');");
    w.emit("if (!els.length) return;");
    w.emit("");
    w.emit("function parseConstraint(cfg) {");
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
    w.emit("}");
    w.emit("");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const axis = el.getAttribute('data-motion-drag') || 'x';");
    w.emit("const constraints = el.getAttribute('data-drag-constraints') || '';");
    w.emit("const elastic = parseFloat(el.getAttribute('data-drag-elastic') || '0');");
    w.emit("const threshold = parseFloat(el.getAttribute('data-drag-threshold') || '0');");
    w.emit("const velocityMax = parseFloat(el.getAttribute('data-drag-velocity-max') || '2000');");
    w.emit("const constraint = parseConstraint(constraints);");
    w.emit("const maxExtent = Math.abs(constraint.max - constraint.min) || 1;");
    w.emit("let startPos = 0, currentDelta = 0, isDragging = false;");
    w.emit("let lastMoveTime = 0, lastMovePos = 0, velocity = 0;");
    w.emit("");
    w.emit("const onPointerDown = (e) => {");
    w.indent();
    w.emit("isDragging = true;");
    w.emit("startPos = axis === 'x' ? e.clientX : e.clientY;");
    w.emit("currentDelta = 0;");
    w.emit("el.setPointerCapture(e.pointerId);");
    w.dedent();
    w.emit("};");
    w.emit("");
    w.emit("const onPointerMove = (e) => {");
    w.indent();
    w.emit("if (!isDragging) return;");
    w.emit("const now = performance.now();");
    w.emit("const raw = (axis === 'x' ? e.clientX : e.clientY) - startPos;");
    w.emit("if (lastMoveTime > 0) {");
    w.indent();
    w.emit("const dt = now - lastMoveTime;");
    w.emit("const dp = Math.abs(raw - lastMovePos);");
    w.emit("if (dt > 0) velocity = Math.min(velocityMax, dp / dt * 1000);");
    w.dedent();
    w.emit("}");
    w.emit("lastMoveTime = now;");
    w.emit("lastMovePos = raw;");
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
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => {");
    w.indent();
    w.emit("const duration = a.effect?.getTiming()?.duration || 300;");
    w.emit("a.currentTime = progress * duration;");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("};");
    w.emit("");
    w.emit("const onPointerUp = (e) => {");
    w.indent();
    w.emit("isDragging = false;");
    w.emit("el.releasePointerCapture(e.pointerId);");
    w.emit("const progress = Math.abs(currentDelta) / maxExtent;");
    w.emit("if (progress >= threshold && maxExtent > 0) {");
    w.indent();
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => a.play());");
    w.dedent();
    w.emit("} else {");
    w.indent();
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => a.reverse());");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("};");
    w.emit("");
    w.emit("el.addEventListener('pointerdown', onPointerDown);");
    w.emit("el.addEventListener('pointermove', onPointerMove);");
    w.emit("el.addEventListener('pointerup', onPointerUp);");
    w.emit("el.addEventListener('pointercancel', onPointerUp);");
    w.emit("");
    w.emit("// Cleanup");
    w.emit("el._dragCleanup = () => {");
    w.indent();
    w.emit("el.removeEventListener('pointerdown', onPointerDown);");
    w.emit("el.removeEventListener('pointermove', onPointerMove);");
    w.emit("el.removeEventListener('pointerup', onPointerUp);");
    w.emit("el.removeEventListener('pointercancel', onPointerUp);");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("return () => {");
    w.indent();
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("if (el._dragCleanup) el._dragCleanup();");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
}
