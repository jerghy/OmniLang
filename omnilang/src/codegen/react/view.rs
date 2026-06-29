// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React)
// ═══════════════════════════════════════════════════════════

use std::collections::HashMap;

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::expr::{format_expr_ir_tsx, generate_statement_tsx};
use super::utils::{
    capitalize_first, emit_event_object_react, format_json_value_tsx, to_css_prop, to_jsx_attr,
    to_react_event,
};

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
fn has_enter_trigger(node: &ViewTree) -> bool {
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
fn has_driven_by(node: &ViewTree) -> bool {
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
fn has_inview_trigger(node: &ViewTree) -> bool {
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
fn has_scroll_trigger(node: &ViewTree) -> bool {
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
fn has_drag_trigger(node: &ViewTree) -> bool {
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

// ═══════════════════════════════════════════════════════════
// View tree generation (JSX)
// ═══════════════════════════════════════════════════════════

pub(super) fn gen_view_node_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    component_map: &HashMap<String, ComponentIR>,
) {
    match node {
        ViewTree::Element(el) => {
            // Component composition: check if tag is a known component
            if let Some(_child_comp) = component_map.get(&el.tag) {
                let mut comp_attrs: Vec<String> = Vec::new();
                for (prop_name, prop_value) in &el.props {
                    let js_val = format_expr_ir_tsx(prop_value);
                    comp_attrs.push(format!("{}={{{}}}", prop_name, js_val));
                }
                for (event_name, handler) in &el.events {
                    let jsx_event = to_react_event(event_name);
                    match handler.kind.as_str() {
                        "fn_ref" => {
                            if let Some(ref fn_name) = handler.r#ref {
                                comp_attrs.push(format!("{}={{{}}}", jsx_event, fn_name));
                            }
                        }
                        "inline_fn" => {
                            if let Some(ref body) = handler.body {
                                let handler_str = format_inline_handler(body);
                                comp_attrs.push(format!("{}={{{}}}", jsx_event, handler_str));
                            }
                        }
                        "store_action" => {
                            if let Some(ref action) = handler.r#ref {
                                let action_fn = action.split('.').last().unwrap_or(action);
                                comp_attrs.push(format!(
                                    "{}={{() => {{ {}(); }}}}",
                                    jsx_event, action_fn
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                let attrs_str = comp_attrs.join(" ");
                if el.children.is_empty() {
                    w.emit(&format!("<{} {}/>", el.tag, attrs_str));
                } else {
                    w.emit(&format!("<{} {}>", el.tag, attrs_str));
                    w.indent();
                    for child in &el.children {
                        gen_view_node_tsx(w, child, component_map);
                    }
                    w.dedent();
                    w.emit(&format!("</{}>", el.tag));
                }
                return; // IMPORTANT: skip HTML element handling
            }
            // Otherwise, fall through to existing HTML element handling
            // Build attribute list from props, bindings, events
            let mut attrs: Vec<String> = Vec::new();

            // Static props
            for (prop_name, prop_value) in &el.props {
                let jsx_attr = to_jsx_attr(prop_name);
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value
                            .as_str()
                            .expect("literal string value confirmed by is_string()");
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                        attrs.push(format!("{}=\"{}\"", jsx_attr, escaped));
                    }
                    _ => {
                        let jsx_val = format_expr_ir_tsx(prop_value);
                        attrs.push(format!("{}={{{}}}", jsx_attr, jsx_val));
                    }
                }
            }

            // Two-way bindings: value={x} onChange={e => setX(e.target.value)}
            for (bind_name, signal_ref) in &el.bindings {
                let jsx_attr = to_jsx_attr(bind_name);
                let setter = format!("set{}", capitalize_first(&signal_ref.name));
                attrs.push(format!("{}={{{}}}", jsx_attr, signal_ref.name));
                match bind_name.as_str() {
                    "value" => {
                        attrs.push(format!(
                            "onChange={{(e) => {{ {}(e.target.value); }}}}",
                            setter
                        ));
                    }
                    "checked" => {
                        attrs.push(format!(
                            "onChange={{(e) => {{ {}(e.target.checked); }}}}",
                            setter
                        ));
                    }
                    _ => {}
                }
            }

            // Motion triggers — add data-motion-enter for @enter effects, inline handlers for interactive
            for trigger in &el.motion_triggers {
                match trigger.kind {
                    TriggerKind::Enter => {
                        attrs.push("data-motion-enter=\"true\"".into());
                    }
                    TriggerKind::Hover => {
                        attrs.push("onMouseEnter={(e) => { e.currentTarget.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 300, easing: 'ease-out' }); }}".into());
                        attrs.push("onMouseLeave={(e) => { e.currentTarget.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 300, easing: 'ease-out' }); }}".into());
                    }
                    TriggerKind::Press => {
                        attrs.push("onMouseDown={(e) => { e.currentTarget.animate([{ scale: 1 }, { scale: 0.95 }], { duration: 150, easing: 'ease-out', fill: 'both' }); }}".into());
                        attrs.push("onMouseUp={(e) => { e.currentTarget.animate([{ scale: 0.95 }, { scale: 1 }], { duration: 150, easing: 'ease-out', fill: 'both' }); }}".into());
                    }
                    TriggerKind::Focus => {
                        attrs.push("onFocus={(e) => { e.currentTarget.animate([{ boxShadow: '0 0 0 0 rgba(59,130,246,0)' }, { boxShadow: '0 0 0 3px rgba(59,130,246,0.5)' }], { duration: 200, easing: 'ease-out', fill: 'both' }); }}".into());
                        attrs.push("onBlur={(e) => { e.currentTarget.animate([{ boxShadow: '0 0 0 3px rgba(59,130,246,0.5)' }, { boxShadow: '0 0 0 0 rgba(59,130,246,0)' }], { duration: 200, easing: 'ease-out', fill: 'both' }); }}".into());
                    }
                    TriggerKind::InView => {
                        let threshold_val = match &el.in_view_threshold {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        attrs.push("data-motion-inview=\"true\"".into());
                        attrs.push(format!("data-inview-threshold=\"{}\"", threshold_val));
                    }
                    TriggerKind::Scroll => {
                        let scroll_start_str = el.scroll_start.as_deref().unwrap_or("top 80%");
                        let scroll_end_str = el.scroll_end.as_deref().unwrap_or("bottom 20%");
                        let scrub_val = match &el.scroll_scrub {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        attrs.push("data-motion-scroll=\"true\"".into());
                        attrs.push(format!("data-scroll-start=\"{}\"", scroll_start_str));
                        attrs.push(format!("data-scroll-end=\"{}\"", scroll_end_str));
                        attrs.push(format!("data-scroll-scrub=\"{}\"", scrub_val));
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        let axis_str = if is_horizontal { "x" } else { "y" };
                        let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                        let elastic_val = match &el.drag_elastic {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        let threshold_val = match &el.drag_threshold {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        let velocity_max_val = match &el.drag_velocity_max {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "2000".to_string(),
                        };
                        attrs.push(format!("data-motion-drag=\"{}\"", axis_str));
                        attrs.push(format!("data-drag-constraints=\"{}\"", constraints_str));
                        attrs.push(format!("data-drag-elastic=\"{}\"", elastic_val));
                        attrs.push(format!("data-drag-threshold=\"{}\"", threshold_val));
                        attrs.push(format!("data-drag-velocity-max=\"{}\"", velocity_max_val));
                    }
                    _ => {}
                }
            }

            // @driven-by — external progress control marker attribute
            if let Some(ref driven_expr) = el.driven_by {
                let driven_js = format_expr_ir_tsx(driven_expr);
                attrs.push(format!("data-driven-by={{{}}}", driven_js));
            }

            // Events
            for (event_name, handler) in &el.events {
                let react_event = to_react_event(event_name);
                match handler.kind.as_str() {
                    "fn_ref" => {
                        if let Some(ref fn_name) = handler.r#ref {
                            if handler.event_data.is_some() {
                                let event_obj = emit_event_object_react(
                                    &handler.event_data,
                                    &handler.used_fields,
                                    "e",
                                );
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}({}) }}}}",
                                    react_event, fn_name, event_obj
                                ));
                            } else {
                                attrs.push(format!("{}={{{}}}", react_event, fn_name));
                            }
                        }
                    }
                    "inline_fn" => {
                        if let Some(ref body) = handler.body {
                            let handler_str = format_inline_handler(body);
                            attrs.push(format!("{}={{{}}}", react_event, handler_str));
                        }
                    }
                    "store_action" => {
                        if let Some(ref action) = handler.r#ref {
                            let action_fn = action.split('.').last().unwrap_or(action);
                            attrs.push(format!(
                                "{react_event}={{() => {{ {action_fn}(); }}}}",
                                react_event = react_event,
                                action_fn = action_fn
                            ));
                        }
                    }
                    _ => {
                        attrs.push(format!(
                            "{}={{/* TODO: handler kind '{}' */}}",
                            react_event, handler.kind
                        ));
                    }
                }
            }

            // Style classes from view
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    attrs.push(format!("className={{styles['{}']}}", classes_str));
                }
            }

            // Inline style
            if let Some(ref style_json) = el.inline_style {
                if let Some(obj) = style_json.as_object() {
                    let mut pairs: Vec<String> = Vec::new();
                    for (prop, val) in obj {
                        let js_prop = to_css_prop(prop);
                        if let Some(s) = val.as_str() {
                            if s.starts_with("$signal:") {
                                let sig_name = &s[8..];
                                pairs.push(format!("{}: {{{}}}", js_prop, sig_name));
                            } else if s.starts_with("$call:") {
                                let fn_call = &s[6..];
                                pairs.push(format!("{}: {{{}}}", js_prop, fn_call));
                            } else {
                                pairs.push(format!("{}: '{}'", js_prop, s));
                            }
                        } else {
                            pairs.push(format!("{}: {{{}}}", js_prop, format_json_value_tsx(val)));
                        }
                    }
                    if !pairs.is_empty() {
                        attrs.push(format!("style={{{{ {} }}}}", pairs.join(", ")));
                    }
                }
            }

            // Dynamic class bindings: class:active={isOpen} -> className={`base ${isOpen ? 'active' : ''}`}
            if !el.class_map_bindings.is_empty() {
                let dynamic_parts: Vec<String> = el
                    .class_map_bindings
                    .iter()
                    .map(|(cls, expr)| {
                        let cond = format_expr_ir_tsx(expr);
                        format!("${{{cond} ? '{cls}' : ''}}")
                    })
                    .collect();
                if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
                    let old = std::mem::take(&mut attrs[pos]);
                    attrs[pos] = old.replace("}}", &format!(" {}", dynamic_parts.join(" ")));
                } else {
                    attrs.push(format!("className={{`{}`}}", dynamic_parts.join(" ")));
                }
            }

            // Emit opening tag
            if attrs.is_empty() && el.children.is_empty() {
                // Self-closing tag
                w.emit(&format!("<{} />", el.tag));
                return;
            }

            if attrs.is_empty() {
                w.emit(&format!("<{}>", el.tag));
            } else {
                w.emit(&format!("<{}", el.tag));
                w.indent();
                for attr in &attrs {
                    w.emit(&attr.to_string());
                }
                if el.children.is_empty() {
                    w.dedent();
                    w.emit("/>");
                    return;
                }
                w.dedent();
                w.emit(">");
            }

            // Children
            w.indent();
            for child in &el.children {
                gen_view_node_tsx(w, child, component_map);
            }
            w.dedent();

            // Closing tag
            w.emit(&format!("</{}>", el.tag));
        }
        ViewTree::Conditional(cond) => {
            let branches = &cond.branches;
            let fallback = &cond.fallback;

            if branches.is_empty() {
                if let Some(ref fb) = fallback {
                    gen_view_node_tsx(w, fb, component_map);
                }
                return;
            }

            // Single branch without fallback → {condition && (...)}
            if branches.len() == 1 && fallback.is_none() {
                w.emit("{");
                w.indent();
                let cond_str = format_expr_ir_tsx(&branches[0].condition);
                w.emit(&format!("{} && (", cond_str));
                w.indent();
                if let Some(ref body) = branches[0].body {
                    gen_view_node_tsx(w, body, component_map);
                }
                w.dedent();
                w.emit(")");
                w.dedent();
                w.emit("}");
                return;
            }

            // Multiple branches or fallback → nested ternary
            w.emit("{");
            w.indent();
            for (i, branch) in branches.iter().enumerate() {
                if i > 0 {
                    w.emit(": (");
                    w.indent();
                }
                let cond_str = format_expr_ir_tsx(&branch.condition);
                w.emit(&format!("{} ? (", cond_str));
                w.indent();
                if let Some(ref body) = branch.body {
                    gen_view_node_tsx(w, body, component_map);
                }
                w.dedent();
            }
            // Fallback or null
            if let Some(ref fb) = fallback {
                w.emit(") : (");
                w.indent();
                gen_view_node_tsx(w, fb, component_map);
                w.dedent();
                w.emit(")");
                // Close all open parenthesis from chained ternary
                for _ in 0..branches.len() {
                    w.emit(")");
                }
                w.dedent();
                w.emit("}");
            } else {
                w.emit(") : null");
                // Close all open parenthesis
                for _ in 1..branches.len() {
                    w.emit(")");
                }
                // Close the open indent for branch 0
                w.dedent();
                w.emit("}");
            }
        }
        ViewTree::ListRendering(list) => {
            let iterable_str = format_expr_ir_tsx(&list.iterable);
            let item_name = &list.item_binding.name;
            let key_expr = if let Some(ref ke) = list.key_expr {
                format!("key={{{}}}", format_expr_ir_tsx(ke))
            } else {
                "key={index}".to_string()
            };
            if list.empty_body.is_some() {
                w.emit("{");
                w.indent();
                w.emit(&format!(
                    "({} && {}.length === 0) ? (",
                    iterable_str, iterable_str
                ));
                w.indent();
                if let Some(ref empty_body) = list.empty_body {
                    gen_view_node_tsx(w, empty_body, component_map);
                }
                w.dedent();
                w.emit(&format!(
                    ") : ({}.map(({}, index) => (",
                    iterable_str, item_name
                ));
            } else {
                w.emit("{");
                w.indent();
                w.emit(&format!("{}.map(({}, index) => (", iterable_str, item_name));
            }
            w.indent();
            if let Some(ref body) = list.body {
                gen_view_node_tsx(w, body, component_map);
            }
            w.dedent();
            w.emit(&format!(", {})))", key_expr));
            w.dedent();
            if list.empty_body.is_some() {
                w.emit(")}");
            } else {
                w.emit("}");
            }
        }
        ViewTree::Text(text) => {
            if text.starts_with('{') && text.ends_with('}') {
                // Interpolation expression like {name} or {param:xxx}
                let inner = &text[1..text.len() - 1];
                w.emit(&format!("{{{}}}", inner));
            } else {
                // Plain text — escape JSX-significant characters
                let escaped = text
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('&', "&amp;");
                w.emit(&format!("{{'{}'}}", escaped));
            }
        }
        ViewTree::Interpolation(expr) => {
            let js_expr = format_expr_ir_tsx(expr);
            w.emit(&format!("{{{}}}", js_expr));
        }
        ViewTree::Slot(_) => {
            // slots not supported in React — emit placeholder
        }
    }
}

/// Build an inline arrow function string from a BlockIR.
fn format_inline_handler(body: &BlockIR) -> String {
    if body.statements.is_empty() {
        return "(e) => {}".to_string();
    }
    let mut hw = CodeWriter::new();
    for stmt in &body.statements {
        generate_statement_tsx(&mut hw, stmt);
    }
    let content = hw.into_string();
    // Build a multi-line arrow function
    let mut result = String::from("(e) => {\n");
    for line in content.lines() {
        result.push_str(&format!("        {}\n", line));
    }
    result.push_str("      }");
    result
}
