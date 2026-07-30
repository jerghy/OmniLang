//! Motion trigger emission for element triggers (@enter, @hover, @press, etc.).
use super::OmniViewGenerator;
use crate::codegen::escape_literal_str;
use crate::ir::*;
use std::fmt::Write;

/// Generate a standard IIFE-wrapped motion trigger JS string.
/// Used by hover and press (and their inline variants), which follow
/// the common pattern:
///
/// ```js
/// // {comment}
/// (function() {
///   const el = __omni_el("{nid}");
///   if (el) {
///     const anim = {anim_orchestrator};
///     if (!__omni_animations.has("{nid}")) __omni_animations.set("{nid}", new Map());
///     __omni_animations.get("{nid}").set("{trigger}", anim);
///     {extra_body}
///   }
/// })();
/// ```
fn format_simple_trigger_js(
    comment: &str,
    nid: &str,
    trigger_str: &str,
    anim_orchestrator: &str,
    extra_body: &str,
) -> String {
    format!(
        "{comment}\n\
         (function() {{\n\
         \x20 const el = __omni_el(\"{nid}\");\n\
         \x20 if (el) {{\n\
         \x20\x20 const anim = {anim};\n\
         \x20\x20 if (!__omni_animations.has(\"{nid}\")) __omni_animations.set(\"{nid}\", new Map());\n\
         \x20\x20 __omni_animations.get(\"{nid}\").set(\"{trigger}\", anim);\n\
         {extra}\
         \x20 }}\n\
         }})();\n",
        comment = comment,
        nid = nid,
        anim = anim_orchestrator,
        trigger = trigger_str,
        extra = extra_body,
    )
}

impl OmniViewGenerator {
    /// Generate @in-view trigger JS. Shared by both named motion ref and inline paths.
    fn emit_trigger_inview_js(
        &mut self,
        nid: &str,
        trigger_str: &str,
        anim_call: &str,
        el: &ElementIR,
        is_inline: bool,
    ) {
        // Compute the disconnect condition differently for each branch:
        // - Named: uses in_view_repeat (boolean) → "true"/"false" string
        // - Inline: uses in_view_once (JS expression) → literal JS expression
        let disconnect_js = if is_inline {
            match el.motion_behavior().and_then(|m| m.in_view_once.as_ref()) {
                Some(expr) => self.expr_to_js(expr),
                None => "true".to_string(),
            }
        } else {
            let repeat_val = match el.motion_behavior().and_then(|m| m.in_view_repeat.as_ref()) {
                Some(expr) => self.expr_to_js(expr) == "true",
                None => false,
            };
            if repeat_val {
                "false".to_string()
            } else {
                "true".to_string()
            }
        };
        let threshold_val = match el
            .motion_behavior()
            .and_then(|m| m.in_view_threshold.as_ref())
        {
            Some(expr) => self.expr_to_js(expr),
            None => "0".to_string(),
        };
        let comment = if is_inline {
            "// @in-view (inline)"
        } else {
            "// @in-view trigger"
        };
        let _ = writeln!(
            self.js,
            "{comment}\n\
             (function() {{\n\
             \x20 const el = __omni_el(\"{nid}\");\n\
             \x20 if (!el) return;\n\
             \x20 const anim = {anim_call};\n\
             \x20 if (!__omni_animations.has(\"{nid}\")) __omni_animations.set(\"{nid}\", new Map());\n\
             \x20 __omni_animations.get(\"{nid}\").set(\"{trigger}\", anim);\n\
             \x20 const threshold = {threshold};\n\
             \x20 const observer = new IntersectionObserver((entries) => {{\n\
             \x20\x20 entries.forEach(entry => {{\n\
             \x20\x20\x20 if (entry.isIntersecting) {{\n\
             \x20\x20\x20\x20 anim.play();\n\
             \x20\x20\x20\x20 if ({disconnect_js}) {{\n\
             \x20\x20\x20\x20\x20 observer.disconnect();\n\
             \x20\x20\x20\x20 }}\n\
             \x20\x20\x20 }}\n\
             \x20\x20 }});\n\
             \x20 }}, {{ threshold: threshold }});\n\
             \x20 observer.observe(el);\n\
             \x20 window.__omni_inview_observers = window.__omni_inview_observers || [];\n\
             \x20 __omni_inview_observers.push({{ nid: \"{nid}\", observer: observer }});\n\
             }})();",
            comment = comment,
            nid = nid,
            anim_call = anim_call,
            trigger = trigger_str,
            threshold = threshold_val,
            disconnect_js = disconnect_js,
        );
    }

    /// Emit the module-level shared scroll handler and variables if not already emitted.
    /// After this, per-element triggers push entries to `__omni_scrollElements` instead of
    /// registering their own `window.addEventListener('scroll', ...)` listener.
    fn emit_shared_scroll_handler(&mut self) {
        if self.scroll_handler_emitted {
            return;
        }
        self.scroll_handler_emitted = true;
        self.js.push_str(
            "/// ─── Shared scroll handler — single listener for all @scroll triggers\n\
             var __omni_scrollElements = [];\n\
             var __omni_scrollHandler = null;\n\
             function __omni_ensureScrollHandler() {\n\
             \x20 if (__omni_scrollHandler) return;\n\
             \x20 __omni_scrollHandler = function() {\n\
             \x20\x20 __omni_scrollElements.forEach(function(entry) {\n\
             \x20\x20\x20 var el = entry.el;\n\
             \x20\x20\x20 var anim = entry.anim;\n\
             \x20\x20\x20 var dur = entry.dur;\n\
             \x20\x20\x20 var target = Omni.scrollProgress(el, entry.startCfg, entry.endCfg);\n\
             \x20\x20\x20 if (entry.scrubDelay > 0) {\n\
             \x20\x20\x20\x20 entry.currentProgress += (target - entry.currentProgress) * Math.min(1, 16.67 / (entry.scrubDelay * 1000));\n\
             \x20\x20\x20\x20 anim.currentTime = entry.currentProgress * dur;\n\
             \x20\x20\x20 } else {\n\
             \x20\x20\x20\x20 entry.currentProgress = target;\n\
             \x20\x20\x20\x20 anim.currentTime = target * dur;\n\
             \x20\x20\x20 }\n\
             \x20\x20 });\n\
             \x20 };\n\
             \x20 window.addEventListener('scroll', __omni_scrollHandler, { passive: true });\n\
             }\n\
             function __omni_scrollRemoveEntry(nid) {\n\
             \x20 var el = __omni_el(nid);\n\
             \x20 if (!el || !__omni_scrollElements) return;\n\
             \x20 __omni_scrollElements = __omni_scrollElements.filter(function(entry) {\n\
             \x20\x20 return entry.el !== el;\n\
             \x20 });\n\
             }\n\n",
        );
    }

    /// Generate @scroll trigger JS. Shared by both named motion ref and inline paths.
    /// Uses a single shared scroll handler (`__omni_ensureScrollHandler`) instead of
    /// per-element `window.addEventListener('scroll', ...)` listeners.
    fn emit_trigger_scroll_js(
        &mut self,
        nid: &str,
        trigger_str: &str,
        anim_call: &str,
        el: &ElementIR,
        is_inline: bool,
    ) {
        let scroll_start_str = escape_literal_str(
            el.scroll_behavior()
                .and_then(|s| s.start.as_deref())
                .unwrap_or("top 80%"),
            '"',
        );
        let scroll_end_str = escape_literal_str(
            el.scroll_behavior()
                .and_then(|s| s.end.as_deref())
                .unwrap_or("bottom 20%"),
            '"',
        );
        let scrub_val = match el.scroll_behavior().and_then(|s| s.scrub.as_ref()) {
            Some(expr) => self.expr_to_js(expr),
            None => "0".to_string(),
        };
        let comment = if is_inline {
            "// @scroll (inline)"
        } else {
            "// @scroll trigger"
        };
        // Emit the shared handler + variables on first use
        self.emit_shared_scroll_handler();
        // Both named and inline branches produce the same structure now — the scrub
        // logic moved into the shared handler.
        let _ = writeln!(self.js, "{comment}\n\
             (function() {{\n\
             \x20 const el = __omni_el(\"{nid}\");\n\
             \x20 if (!el) return;\n\
             \x20 const anim = {anim_call};\n\
             \x20 if (!__omni_animations.has(\"{nid}\")) __omni_animations.set(\"{nid}\", new Map());\n\
             \x20 __omni_animations.get(\"{nid}\").set(\"{trigger}\", anim);\n\
             \x20 anim.pause();\n\
             \x20 const dur = anim.effect?.getTiming()?.duration || 300;\n\
             \x20 __omni_scrollElements.push({{\n\
             \x20\x20 el,\n\
             \x20\x20 anim,\n\
             \x20\x20 dur,\n\
             \x20\x20 startCfg: \"{scroll_start}\",\n\
             \x20\x20 endCfg: \"{scroll_end}\",\n\
             \x20\x20 scrubDelay: {scrub},\n\
             \x20\x20 currentProgress: 0,\n\
             \x20 }});\n\
             \x20 __omni_ensureScrollHandler();\n\
             \x20 anim.currentTime = Omni.scrollProgress(el, \"{scroll_start}\", \"{scroll_end}\") * dur;\n\
             }})();", comment = comment,
            nid = nid,
            anim_call = anim_call,
            trigger = trigger_str,
            scroll_start = scroll_start_str,
            scroll_end = scroll_end_str,
            scrub = scrub_val,);
    }

    /// Generate @drag trigger JS. Shared by both named motion ref and inline paths.
    fn emit_trigger_drag_js(
        &mut self,
        nid: &str,
        trigger_str: &str,
        anim_call: &str,
        el: &ElementIR,
        is_horizontal: bool,
        is_inline: bool,
    ) {
        let axis_label = if is_horizontal { "x" } else { "y" };
        let pointer_axis = if is_horizontal { "clientX" } else { "clientY" };
        let edge_min = if is_horizontal { "left" } else { "top" };
        let edge_max = if is_horizontal { "right" } else { "bottom" };
        let rect_size = if is_horizontal { "width" } else { "height" };
        let constraints_str = escape_literal_str(
            el.drag_behavior()
                .and_then(|d| d.constraints.as_deref())
                .unwrap_or(""),
            '"',
        );
        let elastic_val = match el.drag_behavior().and_then(|d| d.elastic.as_ref()) {
            Some(expr) => self.expr_to_js(expr),
            None => "0".to_string(),
        };
        let threshold_val = match el.drag_behavior().and_then(|d| d.threshold.as_ref()) {
            Some(expr) => self.expr_to_js(expr),
            None => "0".to_string(),
        };
        let velocity_max_val = match el.drag_behavior().and_then(|d| d.velocity_max.as_ref()) {
            Some(expr) => self.expr_to_js(expr),
            None => "2000".to_string(),
        };
        let comment = format!(
            "// @drag-{} {}",
            axis_label,
            if is_inline { "(inline)" } else { "trigger" },
        );
        let _ = writeln!(
            self.js,
            "{comment}\n\
             (function() {{\n\
             \x20 const el = __omni_el(\"{nid}\");\n\
             \x20 if (!el) return;\n\
             \x20 const anim = {anim_call};\n\
             \x20 if (!__omni_animations.has(\"{nid}\")) __omni_animations.set(\"{nid}\", new Map());\n\
             \x20 __omni_animations.get(\"{nid}\").set(\"{trigger}\", anim);\n\
             \x20 anim.pause();\n\
             \x20 const dur = anim.effect?.getTiming()?.duration || 300;\n\
             \x20 let startPos = 0, currentDelta = 0;\n\
             \x20 let isDragging = false;\n\
             \x20 let lastMoveTime = 0, lastMovePos = 0;\n\
             \x20 let velocity = 0;\n\
             \x20 const constraints = \"{constraints}\";\n\
             \x20 const elastic = {elastic};\n\
             \x20 const threshold = {threshold};\n\
             \x20 const velocityMax = {velocity_max};\n\
             \x20 function parseConstraint(cfg) {{\n\
             \x20\x20 if (!cfg) return {{ min: -Infinity, max: Infinity }};\n\
             \x20\x20 if (cfg === 'parent') {{\n\
             \x20\x20\x20 const parent = el.parentElement;\n\
             \x20\x20\x20 if (!parent) return {{ min: -Infinity, max: Infinity }};\n\
             \x20\x20\x20 const elRect = el.getBoundingClientRect();\n\
             \x20\x20\x20 const parentRect = parent.getBoundingClientRect();\n\
             \x20\x20\x20 return {{ min: 0, max: parentRect.{size} - elRect.{size} }};\n\
             \x20\x20 }}\n\
             \x20\x20 if (cfg.startsWith('ref:')) {{\n\
             \x20\x20\x20 const refId = cfg.slice(4);\n\
             \x20\x20\x20 const refEl = __omni_el(refId);\n\
             \x20\x20\x20 if (!refEl) return {{ min: -Infinity, max: Infinity }};\n\
             \x20\x20\x20 const elRect = el.getBoundingClientRect();\n\
             \x20\x20\x20 const refRect = refEl.getBoundingClientRect();\n\
             \x20\x20\x20 return {{ min: 0, max: refRect.{size} - elRect.{size} }};\n\
             \x20\x20 }}\n\
             \x20\x20 const parts = cfg.split(' ');\n\
             \x20\x20 let min = -Infinity, max = Infinity;\n\
             \x20\x20 parts.forEach(p => {{\n\
             \x20\x20\x20 const [key, val] = p.split(':');\n\
             \x20\x20\x20 if (key === '{edge_min}') min = parseFloat(val) || 0;\n\
             \x20\x20\x20 if (key === '{edge_max}') max = parseFloat(val) || Infinity;\n\
             \x20\x20 }});\n\
             \x20\x20 return {{ min, max }};\n\
             \x20 }}\n\
             \x20 const constraint = parseConstraint(constraints);\n\
             \x20 const maxExtent = Math.abs(constraint.max - constraint.min) || 1;\n\
             \x20 el.addEventListener('pointerdown', (e) => {{\n\
             \x20\x20 isDragging = true;\n\
             \x20\x20 startPos = e.{ptr_axis};\n\
             \x20\x20 currentDelta = 0;\n\
             \x20\x20 el.setPointerCapture(e.pointerId);\n\
             \x20 }});\n\
             \x20 el.addEventListener('pointermove', (e) => {{\n\
             \x20\x20 if (!isDragging) return;\n\
             \x20\x20 const now = performance.now();\n\
             \x20\x20 const raw = e.{ptr_axis} - startPos;\n\
             \x20\x20 if (lastMoveTime > 0) {{\n\
             \x20\x20\x20 const dt = now - lastMoveTime;\n\
             \x20\x20\x20 const dp = Math.abs(raw - lastMovePos);\n\
             \x20\x20\x20 if (dt > 0) velocity = Math.min(velocityMax, dp / dt * 1000);\n\
             \x20\x20 }}\n\
             \x20\x20 lastMoveTime = now;\n\
             \x20\x20 lastMovePos = raw;\n\
             \x20\x20 let constrained = raw;\n\
             \x20\x20 if (raw < constraint.min) {{\n\
             \x20\x20\x20 const overshoot = constraint.min - raw;\n\
             \x20\x20\x20 constrained = constraint.min - overshoot * (1 - elastic);\n\
             \x20\x20 }} else if (raw > constraint.max) {{\n\
             \x20\x20\x20 const overshoot = raw - constraint.max;\n\
             \x20\x20\x20 constrained = constraint.max + overshoot * (1 - elastic);\n\
             \x20\x20 }}\n\
             \x20\x20 currentDelta = raw;\n\
             \x20\x20 const progress = Math.abs(constrained) / maxExtent;\n\
             \x20\x20 anim.currentTime = Math.min(1, progress) * dur;\n\
             \x20 }});\n\
             \x20 el.addEventListener('pointerup', (e) => {{\n\
             \x20\x20 isDragging = false;\n\
             \x20\x20 el.releasePointerCapture(e.pointerId);\n\
             \x20\x20 const progress = Math.abs(currentDelta) / maxExtent;\n\
             \x20\x20 if (progress >= threshold && maxExtent > 0) {{\n\
             \x20\x20\x20 anim.play();\n\
             \x20\x20 }} else {{\n\
             \x20\x20\x20 anim.reverse();\n\
             \x20\x20 }}\n\
             \x20 }});\n\
             }})();",
            comment = comment,
            nid = nid,
            anim_call = anim_call,
            trigger = trigger_str,
            constraints = constraints_str,
            elastic = elastic_val,
            threshold = threshold_val,
            ptr_axis = pointer_axis,
            edge_min = edge_min,
            edge_max = edge_max,
            size = rect_size,
            velocity_max = velocity_max_val,
        );
    }

    /// Emit trigger JS for Enter, Hover, Press — shared by both named motion ref and inline paths.
    fn emit_trigger_with_anim_call(
        &mut self,
        nid: &str,
        trigger_str: &str,
        anim_call: &str,
        kind: TriggerKind,
        is_inline: bool,
    ) {
        match kind {
            TriggerKind::Enter => {
                let comment = if is_inline {
                    "// @enter (inline)"
                } else {
                    "// @enter trigger"
                };
                let _ = writeln!(
                    self.mount_js,
                    "{comment}\n\
                     (function() {{\n\
                     \x20 const el = __omni_el(\"{nid}\");\n\
                     \x20 if (el) {{\n\
                     \x20\x20 const anim = {anim};\n\
                     \x20\x20 if (!__omni_animations.has(\"{nid}\")) __omni_animations.set(\"{nid}\", new Map());\n\
                     \x20\x20 __omni_animations.get(\"{nid}\").set(\"{trigger}\", anim);\n\
                     \x20\x20 if (el.offsetParent !== null) anim.play();\n\
                     \x20 }}\n\
                     }})();",
                    comment = comment,
                    nid = nid,
                    anim = anim_call,
                    trigger = trigger_str,
                );
            }
            TriggerKind::Hover => {
                let comment = if is_inline {
                    "// @hover (inline)"
                } else {
                    "// @hover trigger"
                };
                let extra = format!(
                    "\x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_animations.get(\"{nid}\")?.get(\"{trigger}\")?.play(); }});\n\
                     \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_animations.get(\"{nid}\")?.get(\"{trigger}\")?.reverse(); }});\n",
                    nid = nid,
                    trigger = trigger_str,
                );
                self.js.push_str(&format_simple_trigger_js(
                    comment,
                    nid,
                    trigger_str,
                    anim_call,
                    &extra,
                ));
            }
            TriggerKind::Press => {
                let comment = if is_inline {
                    "// @press (inline)"
                } else {
                    "// @press trigger"
                };
                let extra = format!(
                    "\x20\x20 el.addEventListener(\"mousedown\", function() {{ __omni_animations.get(\"{nid}\")?.get(\"{trigger}\")?.play(); }});\n\
                     \x20\x20 el.addEventListener(\"mouseup\", function() {{ __omni_animations.get(\"{nid}\")?.get(\"{trigger}\")?.reverse(); }});\n",
                    nid = nid,
                    trigger = trigger_str,
                );
                self.js.push_str(&format_simple_trigger_js(
                    comment,
                    nid,
                    trigger_str,
                    anim_call,
                    &extra,
                ));
            }
            TriggerKind::InView
            | TriggerKind::Scroll
            | TriggerKind::DragHorizontal
            | TriggerKind::DragVertical => {
                unreachable!(
                    "emit_trigger_with_anim_call only handles Enter/Hover/Press — \
                     other variants are dispatched to their own dedicated methods"
                )
            }
        }
    }

    /// Emit motion trigger JS for element triggers (@enter, @hover, etc.).
    /// Runs in the deferred/mount JS context where DOM elements exist.
    pub(super) fn emit_motion_triggers_for_element(&mut self, nid: &str, el: &ElementIR) {
        self.assert_can_emit_js();
        for trigger in el
            .motion_behavior()
            .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
        {
            let trigger_str = match trigger.kind {
                TriggerKind::Enter => "enter",
                TriggerKind::Hover => "hover",
                TriggerKind::Press => "press",
                TriggerKind::InView => "inView",
                TriggerKind::Scroll => "scroll",
                TriggerKind::DragHorizontal => "dragHorizontal",
                TriggerKind::DragVertical => "dragVertical",
            };

            // If there's an inline motion definition, emit it as a player config
            // If there's a motion_ref, reference the named motion function
            let (kf_override, opts_override) = if let Some(ref inline) = trigger.inline_motion {
                self.motion_body_to_config(inline)
            } else {
                (String::new(), String::new())
            };

            if let Some(ref motion_name) = trigger.motion_ref {
                // Pre-compute the animation orchestrator call for all named triggers
                let anim_call = format!(
                    "__omni_motion_{}(el, {{ trigger: \"{}\" }})",
                    motion_name, trigger_str,
                );
                // Handle Enter-specific replay registration (named only)
                if let TriggerKind::Enter = trigger.kind {
                    self.enter_trigger_nids.push((
                        nid.to_string(),
                        motion_name.clone(),
                        "enter".into(),
                        self.scope.current_component.clone(),
                    ));
                }
                match trigger.kind {
                    TriggerKind::Enter | TriggerKind::Hover | TriggerKind::Press => {
                        self.emit_trigger_with_anim_call(
                            nid,
                            trigger_str,
                            &anim_call,
                            trigger.kind,
                            false,
                        );
                    }
                    TriggerKind::InView => {
                        self.emit_trigger_inview_js(nid, trigger_str, &anim_call, el, false);
                    }
                    TriggerKind::Scroll => {
                        self.emit_trigger_scroll_js(nid, trigger_str, &anim_call, el, false);
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        self.emit_trigger_drag_js(
                            nid,
                            trigger_str,
                            &anim_call,
                            el,
                            is_horizontal,
                            false,
                        );
                    }
                }
            } else if !kf_override.is_empty() {
                // Inline motion definition — full config embedded
                let anim_call =
                    format!("Omni.animate(el, {}, {{ {} }})", kf_override, opts_override);
                match trigger.kind {
                    TriggerKind::Enter | TriggerKind::Hover | TriggerKind::Press => {
                        self.emit_trigger_with_anim_call(
                            nid,
                            trigger_str,
                            &anim_call,
                            trigger.kind,
                            true,
                        );
                    }
                    TriggerKind::InView => {
                        self.emit_trigger_inview_js(nid, trigger_str, &anim_call, el, true);
                    }
                    TriggerKind::Scroll => {
                        self.emit_trigger_scroll_js(nid, trigger_str, &anim_call, el, true);
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        self.emit_trigger_drag_js(
                            nid,
                            trigger_str,
                            &anim_call,
                            el,
                            is_horizontal,
                            true,
                        );
                    }
                }
            }
        }
    }
}
