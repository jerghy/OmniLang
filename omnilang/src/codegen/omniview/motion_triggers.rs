//! Motion trigger emission for element triggers (@enter, @hover, @press, etc.).
use super::OmniViewGenerator;
use crate::ir::*;

/// Generate a standard IIFE-wrapped motion trigger JS string.
/// Used by hover, press, focus, unhover, leave, animate, timeline and their
/// inline variants, which all follow the common pattern:
///
/// ```js
/// // {comment}
/// (function() {
///   const el = __omni_el("{nid}");
///   if (el) {
///     const anim = {anim_orchestrator};
///     __omni_animations.set("{nid}_{trigger}", anim);
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
         \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
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
        self.js.push_str(&format!(
            "{comment}\n\
             (function() {{\n\
             \x20 const el = __omni_el(\"{nid}\");\n\
             \x20 if (!el) return;\n\
             \x20 const anim = {anim_call};\n\
             \x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
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
             \x20 __omni_inview_observers.push(observer);\n\
             }})();\n",
            comment = comment,
            nid = nid,
            anim_call = anim_call,
            trigger = trigger_str,
            threshold = threshold_val,
            disconnect_js = disconnect_js,
        ));
    }

    /// Generate @scroll trigger JS. Shared by both named motion ref and inline paths.
    /// NOTE: the JS body differs between named and inline branches (anim.currentTime placement),
    /// so we preserve the exact output via is_inline.
    fn emit_trigger_scroll_js(
        &mut self,
        nid: &str,
        trigger_str: &str,
        anim_call: &str,
        el: &ElementIR,
        is_inline: bool,
    ) {
        let scroll_start_str = el
            .scroll_behavior()
            .and_then(|s| s.start.as_deref())
            .unwrap_or("top 80%");
        let scroll_end_str = el
            .scroll_behavior()
            .and_then(|s| s.end.as_deref())
            .unwrap_or("bottom 20%");
        let scrub_val = match el.scroll_behavior().and_then(|s| s.scrub.as_ref()) {
            Some(expr) => self.expr_to_js(expr),
            None => "0".to_string(),
        };
        let comment = if is_inline {
            "// @scroll (inline)"
        } else {
            "// @scroll trigger"
        };
        if is_inline {
            // Inline branch: anim.currentTime inside each if/else arm
            self.js.push_str(&format!(
                "{comment}\n\
                 (function() {{\n\
                 \x20 const el = __omni_el(\"{nid}\");\n\
                 \x20 if (!el) return;\n\
                 \x20 const anim = {anim_call};\n\
                 \x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
                 \x20 anim.pause();\n\
                 \x20 const dur = anim.effect?.getTiming()?.duration || 300;\n\
                 \x20 const startCfg = \"{scroll_start}\";\n\
                 \x20 const endCfg = \"{scroll_end}\";\n\
                 \x20 const scrubDelay = {scrub};\n\
                 \x20 function computeProgress() {{\n\
                 \x20\x20 return Omni.scrollProgress(el, startCfg, endCfg);\n\
                 \x20 }}\n\
                 \x20 let currentProgress = 0;\n\
                 \x20 window.addEventListener('scroll', () => {{\n\
                 \x20\x20 const target = computeProgress();\n\
                 \x20\x20 if (scrubDelay > 0) {{\n\
                 \x20\x20\x20 currentProgress += (target - currentProgress) * Math.min(1, 16.67 / (scrubDelay * 1000));\n\
                 \x20\x20\x20 anim.currentTime = currentProgress * dur;\n\
                 \x20\x20 }} else {{\n\
                 \x20\x20\x20 currentProgress = target;\n\
                 \x20\x20\x20 anim.currentTime = currentProgress * dur;\n\
                 \x20\x20 }}\n\
                 \x20 }}, {{ passive: true }});\n\
                 \x20 anim.currentTime = computeProgress() * dur;\n\
                 }})();\n",
                comment = comment,
                nid = nid,
                anim_call = anim_call,
                trigger = trigger_str,
                scroll_start = scroll_start_str,
                scroll_end = scroll_end_str,
                scrub = scrub_val,
            ));
        } else {
            // Named branch: anim.currentTime once after the if/else
            self.js.push_str(&format!(
                "{comment}\n\
                 (function() {{\n\
                 \x20 const el = __omni_el(\"{nid}\");\n\
                 \x20 if (!el) return;\n\
                 \x20 const anim = {anim_call};\n\
                 \x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
                 \x20 anim.pause();\n\
                 \x20 const dur = anim.effect?.getTiming()?.duration || 300;\n\
                 \x20 const startCfg = \"{scroll_start}\";\n\
                 \x20 const endCfg = \"{scroll_end}\";\n\
                 \x20 const scrubDelay = {scrub};\n\
                 \x20 function computeProgress() {{\n\
                 \x20\x20 return Omni.scrollProgress(el, startCfg, endCfg);\n\
                 \x20 }}\n\
                 \x20 let currentProgress = 0;\n\
                 \x20 window.addEventListener('scroll', () => {{\n\
                 \x20\x20 const target = computeProgress();\n\
                 \x20\x20 if (scrubDelay > 0) {{\n\
                 \x20\x20\x20 currentProgress += (target - currentProgress) * Math.min(1, 16.67 / (scrubDelay * 1000));\n\
                 \x20\x20 }} else {{\n\
                 \x20\x20\x20 currentProgress = target;\n\
                 \x20\x20 }}\n\
                 \x20\x20 anim.currentTime = currentProgress * dur;\n\
                 \x20 }}, {{ passive: true }});\n\
                 \x20 anim.currentTime = computeProgress() * dur;\n\
                 }})();\n",
                comment = comment,
                nid = nid,
                anim_call = anim_call,
                trigger = trigger_str,
                scroll_start = scroll_start_str,
                scroll_end = scroll_end_str,
                scrub = scrub_val,
            ));
        }
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
        let constraints_str = el
            .drag_behavior()
            .and_then(|d| d.constraints.as_deref())
            .unwrap_or("");
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
        self.js.push_str(&format!(
            "{comment}\n\
             (function() {{\n\
             \x20 const el = __omni_el(\"{nid}\");\n\
             \x20 if (!el) return;\n\
             \x20 const anim = {anim_call};\n\
             \x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
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
             }})();\n",
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
        ));
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
                TriggerKind::Leave => "leave",
                TriggerKind::Hover => "hover",
                TriggerKind::Unhover => "unhover",
                TriggerKind::Press => "press",
                TriggerKind::Focus => "focus",
                TriggerKind::Animate => "animate",
                TriggerKind::InView => "inView",
                TriggerKind::Scroll => "scroll",
                TriggerKind::DragHorizontal => "dragHorizontal",
                TriggerKind::DragVertical => "dragVertical",
                TriggerKind::Timeline => "timeline",
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
                match trigger.kind {
                    TriggerKind::Enter => {
                        // @enter: create animation and play immediately (in mount JS)
                        // Also register for replay when component signals change
                        self.enter_trigger_nids.push((
                            nid.to_string(),
                            motion_name.clone(),
                            "enter".into(),
                            self.current_component.clone(),
                        ));
                        self.mount_js.push_str(&format!(
                            "// @enter trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const anim = {anim};\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
                             \x20\x20 if (el.offsetParent !== null) anim.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            anim = anim_call,
                        ));
                    }
                    TriggerKind::Hover => {
                        let extra = format!(
                            "\x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                            nid = nid,
                            trigger = trigger_str,
                        );
                        self.js.push_str(&format_simple_trigger_js(
                            "// @hover trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            &extra,
                        ));
                    }
                    TriggerKind::Press => {
                        let extra = format!(
                            "\x20\x20 el.addEventListener(\"mousedown\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseup\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                            nid = nid,
                            trigger = trigger_str,
                        );
                        self.js.push_str(&format_simple_trigger_js(
                            "// @press trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            &extra,
                        ));
                    }
                    TriggerKind::Focus => {
                        let extra = format!(
                            "\x20\x20 el.addEventListener(\"focus\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"blur\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                            nid = nid,
                            trigger = trigger_str,
                        );
                        self.js.push_str(&format_simple_trigger_js(
                            "// @focus trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            &extra,
                        ));
                    }
                    TriggerKind::Leave => {
                        self.mount_js.push_str(&format_simple_trigger_js(
                            "// @leave trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            "",
                        ));
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
                    TriggerKind::Animate => {
                        let extra = "\x20\x20 anim.play();\n";
                        self.mount_js.push_str(&format_simple_trigger_js(
                            "// @animate trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            extra,
                        ));
                    }
                    TriggerKind::Timeline => {
                        self.enter_trigger_nids.push((
                            nid.to_string(),
                            motion_name.clone(),
                            "timeline".into(),
                            self.current_component.clone(),
                        ));
                        self.mount_js.push_str(&format_simple_trigger_js(
                            "// @timeline trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            "",
                        ));
                    }
                    TriggerKind::Unhover => {
                        self.js.push_str(&format_simple_trigger_js(
                            "// @unhover trigger",
                            nid,
                            trigger_str,
                            &anim_call,
                            "",
                        ));
                    }
                }
            } else if !kf_override.is_empty() {
                // Inline motion definition — full config embedded
                let anim_call =
                    format!("Omni.animate(el, {}, {{ {} }})", kf_override, opts_override);
                match trigger.kind {
                    TriggerKind::Enter => {
                        self.mount_js.push_str(&format!(
                            "// @enter (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const anim = {anim};\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
                             \x20\x20 if (el.offsetParent !== null) anim.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            anim = anim_call,
                        ));
                    }
                    TriggerKind::Hover => {
                        let extra = format!(
                            "\x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                            nid = nid,
                            trigger = trigger_str,
                        );
                        self.js.push_str(&format_simple_trigger_js(
                            "// @hover (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            &extra,
                        ));
                    }
                    TriggerKind::Press => {
                        let extra = format!(
                            "\x20\x20 el.addEventListener(\"mousedown\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseup\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                            nid = nid,
                            trigger = trigger_str,
                        );
                        self.js.push_str(&format_simple_trigger_js(
                            "// @press (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            &extra,
                        ));
                    }
                    TriggerKind::Focus => {
                        let extra = format!(
                            "\x20\x20 el.addEventListener(\"focus\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"blur\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                            nid = nid,
                            trigger = trigger_str,
                        );
                        self.js.push_str(&format_simple_trigger_js(
                            "// @focus (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            &extra,
                        ));
                    }
                    TriggerKind::Leave => {
                        self.mount_js.push_str(&format_simple_trigger_js(
                            "// @leave (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            "",
                        ));
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
                    TriggerKind::Animate => {
                        let extra = "\x20\x20 anim.play();\n";
                        self.mount_js.push_str(&format_simple_trigger_js(
                            "// @animate (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            extra,
                        ));
                    }
                    TriggerKind::Timeline => {
                        self.mount_js.push_str(&format_simple_trigger_js(
                            "// @timeline (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            "",
                        ));
                    }
                    TriggerKind::Unhover => {
                        self.js.push_str(&format_simple_trigger_js(
                            "// @unhover (inline)",
                            nid,
                            trigger_str,
                            &anim_call,
                            "",
                        ));
                    }
                }
            }
        }
    }
}
