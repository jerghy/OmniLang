//! Motion trigger emission for element triggers (@enter, @hover, @press, etc.).
use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    /// Emit motion trigger JS for element triggers (@enter, @hover, etc.).
    /// Runs in the deferred/mount JS context where DOM elements exist.
    pub(super) fn emit_motion_triggers_for_element(&mut self, nid: &str, el: &ElementIR) {
        for trigger in &el.motion_triggers {
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
            let config_override = if let Some(ref inline) = trigger.inline_motion {
                self.motion_body_to_config(inline)
            } else {
                String::new()
            };

            if let Some(ref motion_name) = trigger.motion_ref {
                match trigger.kind {
                    TriggerKind::Enter => {
                        // @enter: create animation and play immediately (in mount JS)
                        // Also register for replay when component signals change
                        self.enter_trigger_nids.push((
                            nid.to_string(),
                            motion_name.clone(),
                            "enter".into(),
                        ));
                        self.mount_js.push_str(&format!(
                            "// @enter trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 if (el.offsetParent !== null) player.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Hover => {
                        self.js.push_str(&format!(
                            "// @hover trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Press => {
                        self.js.push_str(&format!(
                            "// @press trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"mousedown\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseup\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Focus => {
                        self.js.push_str(&format!(
                            "// @focus trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"focus\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"blur\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Leave => {
                        self.mount_js.push_str(&format!(
                            "// @leave trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 __omni_createAnimation(\"{nid}\", config);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str
                        ));
                    }
                    TriggerKind::InView => {
                        let repeat_val = match &el.in_view_repeat {
                            Some(expr) => {
                                let js = self.expr_to_js(expr);
                                js == "true"
                            }
                            None => false,
                        };
                        let should_disconnect = if repeat_val { "false" } else { "true" };
                        let threshold_val = match &el.in_view_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @in-view trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 const threshold = {threshold};\n\
                             \x20 const observer = new IntersectionObserver((entries) => {{\n\
                             \x20\x20 entries.forEach(entry => {{\n\
                             \x20\x20\x20 if (entry.isIntersecting) {{\n\
                             \x20\x20\x20\x20 player.play();\n\
                             \x20\x20\x20\x20 if ({disconnect}) {{\n\
                             \x20\x20\x20\x20\x20 observer.disconnect();\n\
                             \x20\x20\x20\x20 }}\n\
                             \x20\x20\x20 }}\n\
                             \x20\x20 }});\n\
                             \x20 }}, {{ threshold: threshold }});\n\
                             \x20 observer.observe(el);\n\
                             \x20 __omni_inview_observers = __omni_inview_observers || [];\n\
                             \x20 __omni_inview_observers.push(observer);\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name,
                            disconnect = should_disconnect,
                            threshold = threshold_val,
                        ));
                    }
                    TriggerKind::Scroll => {
                        let scroll_start_str = el.scroll_start.as_deref().unwrap_or("top 80%");
                        let scroll_end_str = el.scroll_end.as_deref().unwrap_or("bottom 20%");
                        let scrub_val = match &el.scroll_scrub {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @scroll trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
                             \x20 const startCfg = \"{scroll_start}\";\n\
                             \x20 const endCfg = \"{scroll_end}\";\n\
                             \x20 const scrubDelay = {scrub};\n\
                             \x20 function computeProgress() {{\n\
                             \x20\x20 const rect = el.getBoundingClientRect();\n\
                             \x20\x20 const vh = window.innerHeight;\n\
                             \x20\x20 const startPos = __omni_parseScrollTrigger(startCfg, el, vh);\n\
                             \x20\x20 const endPos = __omni_parseScrollTrigger(endCfg, el, vh);\n\
                             \x20\x20 const raw = (window.scrollY - startPos) / (endPos - startPos);\n\
                             \x20\x20 return Math.max(0, Math.min(1, raw));\n\
                             \x20 }}\n\
                             \x20 let currentProgress = 0;\n\
                             \x20 window.addEventListener('scroll', () => {{\n\
                             \x20\x20 const target = computeProgress();\n\
                             \x20\x20 if (scrubDelay > 0) {{\n\
                             \x20\x20\x20 currentProgress += (target - currentProgress) * Math.min(1, 16.67 / (scrubDelay * 1000));\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 currentProgress = target;\n\
                             \x20\x20 }}\n\
                             \x20\x20 player.seek(currentProgress);\n\
                             \x20 }}, {{ passive: true }});\n\
                             \x20 player.seek(computeProgress());\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name,
                            scroll_start = scroll_start_str,
                            scroll_end = scroll_end_str,
                            scrub = scrub_val,
                        ));
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        let axis_label = if is_horizontal { "x" } else { "y" };
                        let pointer_axis = if is_horizontal { "clientX" } else { "clientY" };
                        let edge_min = if is_horizontal { "left" } else { "top" };
                        let edge_max = if is_horizontal { "right" } else { "bottom" };
                        let rect_size = if is_horizontal { "width" } else { "height" };
                        let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                        let elastic_val = match &el.drag_elastic {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let threshold_val = match &el.drag_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let velocity_max_val = match &el.drag_velocity_max {
                            Some(expr) => self.expr_to_js(expr),
                            None => "2000".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @drag-{axis} trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
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
                             \x20\x20 player.seek(Math.min(1, progress));\n\
                             \x20 }});\n\
                             \x20 el.addEventListener('pointerup', (e) => {{\n\
                             \x20\x20 isDragging = false;\n\
                             \x20\x20 el.releasePointerCapture(e.pointerId);\n\
                             \x20\x20 const progress = Math.abs(currentDelta) / maxExtent;\n\
                             \x20\x20 if (progress >= threshold && maxExtent > 0) {{\n\
                             \x20\x20\x20 player.play();\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 player.reverse();\n\
                             \x20\x20 }}\n\
                             \x20 }});\n\
                             }})();\n",
                            nid = nid,
                            axis = axis_label,
                            trigger = trigger_str,
                            motion = motion_name,
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
                    TriggerKind::Animate => {
                        self.mount_js.push_str(&format!(
                            "// @animate trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 player.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Timeline => {
                        self.enter_trigger_nids.push((
                            nid.to_string(),
                            motion_name.clone(),
                            "timeline".into(),
                        ));
                        self.mount_js.push_str(&format!(
                            "// @timeline trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    _ => {
                        // Other triggers: create animation, don't auto-play
                        self.js.push_str(&format!(
                            "// @{trigger} trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                }
            } else if !config_override.is_empty() {
                // Inline motion definition — full config embedded
                match trigger.kind {
                    TriggerKind::Enter => {
                        self.mount_js.push_str(&format!(
                            "// @enter (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20\x20 const player = new AnimationPlayer(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 if (el.offsetParent !== null) player.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override
                        ));
                    }
                    TriggerKind::Hover => {
                        self.js.push_str(&format!(
                            "// @hover (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20\x20 const player = new AnimationPlayer(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override
                        ));
                    }
                    TriggerKind::InView => {
                        let once_val = match &el.in_view_once {
                            Some(expr) => self.expr_to_js(expr),
                            None => "true".to_string(),
                        };
                        let threshold_val = match &el.in_view_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @in-view (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20 const player = new AnimationPlayer(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 const threshold = {threshold};\n\
                             \x20 const observer = new IntersectionObserver((entries) => {{\n\
                             \x20\x20 entries.forEach(entry => {{\n\
                             \x20\x20\x20 if (entry.isIntersecting) {{\n\
                             \x20\x20\x20\x20 player.play();\n\
                             \x20\x20\x20\x20 if ({once}) {{\n\
                             \x20\x20\x20\x20\x20 observer.disconnect();\n\
                             \x20\x20\x20\x20 }}\n\
                             \x20\x20\x20 }}\n\
                             \x20\x20 }});\n\
                             \x20 }}, {{ threshold: threshold }});\n\
                             \x20 observer.observe(el);\n\
                             \x20 __omni_inview_observers = __omni_inview_observers || [];\n\
                             \x20 __omni_inview_observers.push(observer);\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override,
                            once = once_val,
                            threshold = threshold_val,
                        ));
                    }
                    TriggerKind::Scroll => {
                        let scroll_start_str = el.scroll_start.as_deref().unwrap_or("top 80%");
                        let scroll_end_str = el.scroll_end.as_deref().unwrap_or("bottom 20%");
                        let scrub_val = match &el.scroll_scrub {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @scroll (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20 const player = new AnimationPlayer(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
                             \x20 const startCfg = \"{scroll_start}\";\n\
                             \x20 const endCfg = \"{scroll_end}\";\n\
                             \x20 const scrubDelay = {scrub};\n\
                             \x20 function computeProgress() {{\n\
                             \x20\x20 const rect = el.getBoundingClientRect();\n\
                             \x20\x20 const vh = window.innerHeight;\n\
                             \x20\x20 const startPos = __omni_parseScrollTrigger(startCfg, el, vh);\n\
                             \x20\x20 const endPos = __omni_parseScrollTrigger(endCfg, el, vh);\n\
                             \x20\x20 const raw = (window.scrollY - startPos) / (endPos - startPos);\n\
                             \x20\x20 return Math.max(0, Math.min(1, raw));\n\
                             \x20 }}\n\
                             \x20 let currentProgress = 0;\n\
                             \x20 window.addEventListener('scroll', () => {{\n\
                             \x20\x20 const target = computeProgress();\n\
                             \x20\x20 if (scrubDelay > 0) {{\n\
                             \x20\x20\x20 currentProgress += (target - currentProgress) * Math.min(1, 16.67 / (scrubDelay * 1000));\n\
                             \x20\x20\x20 player.seek(currentProgress);\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 currentProgress = target;\n\
                             \x20\x20\x20 player.seek(currentProgress);\n\
                             \x20\x20 }}\n\
                             \x20 }}, {{ passive: true }});\n\
                             \x20 player.seek(computeProgress());\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override,
                            scroll_start = scroll_start_str,
                            scroll_end = scroll_end_str,
                            scrub = scrub_val,
                        ));
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        let pointer_axis = if is_horizontal { "clientX" } else { "clientY" };
                        let edge_min = if is_horizontal { "left" } else { "top" };
                        let edge_max = if is_horizontal { "right" } else { "bottom" };
                        let rect_size = if is_horizontal { "width" } else { "height" };
                        let axis_label = if is_horizontal { "x" } else { "y" };
                        let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                        let elastic_val = match &el.drag_elastic {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let threshold_val = match &el.drag_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let velocity_max_val = match &el.drag_velocity_max {
                            Some(expr) => self.expr_to_js(expr),
                            None => "2000".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @drag-{axis} (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\", {cfg} }};\n\
                             \x20 const player = new AnimationPlayer(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
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
                             \x20\x20 player.seek(Math.min(1, progress));\n\
                             \x20 }});\n\
                             \x20 el.addEventListener('pointerup', (e) => {{\n\
                             \x20\x20 isDragging = false;\n\
                             \x20\x20 el.releasePointerCapture(e.pointerId);\n\
                             \x20\x20 const progress = Math.abs(currentDelta) / maxExtent;\n\
                             \x20\x20 if (progress >= threshold && maxExtent > 0) {{\n\
                             \x20\x20\x20 player.play();\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 player.reverse();\n\
                             \x20\x20 }}\n\
                             \x20 }});\n\
                             }})();\n",
                            nid = nid,
                            cfg = config_override,
                            axis = axis_label,
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
                    _ => {
                        self.js.push_str(&format!(
                            "// @{trigger} (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20\x20 const player = new AnimationPlayer(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override
                        ));
                    }
                }
            }
        }
    }
}
