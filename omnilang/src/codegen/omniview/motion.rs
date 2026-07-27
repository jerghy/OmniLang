//! Motion/animation codegen: emit_motion_js, format_transform_value, emit_states_js.
use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    /// Emit a JS function that creates a WAAPI animation for a named @motion block.
    pub(super) fn emit_motion_js(&mut self, motion: &MotionIntent) {
        let (kf_js, opts_js) = match &motion.body {
            MotionBodyIR::Keyframe(kf) => self.keyframe_to_animate_call(kf),
            MotionBodyIR::Timeline(tl) => self.timeline_to_factory(&motion.name, tl),
            MotionBodyIR::CallbackOnly(cb) => self.callback_to_animate_call(cb),
        };

        self.js.push_str(&format!(
            "function __omni_motion_{name}(el, overrides) {{\n\
             \x20 const opts = Object.assign({{{opts}}}, overrides || {{}});\n\
             \x20 const kf = {kf};\n\
             \x20 return Omni.animate(el, kf, opts);\n\
             }};\n",
            name = motion.name,
            opts = opts_js,
            kf = kf_js,
        ));
    }

    /// Format a transform property value into a CSS transform function string.
    pub(super) fn format_transform_value(key: &str, value_js: &str) -> String {
        match key {
            "x" => format!("translateX({}px)", value_js),
            "y" => format!("translateY({}px)", value_js),
            "scale" | "scaleX" | "scaleY" => format!("{}({})", key, value_js),
            "rotate" | "rotateX" | "rotateY" | "rotateZ" => format!("{}({}deg)", key, value_js),
            _ => format!("{}({})", key, value_js),
        }
    }

    /// Emit JS for states: { ... } multi-state animation on an element.
    /// Two modes:
    ///   - Reserved names (idle, hover, pressed): auto-bind mouse/touch events
    ///   - Custom names (loading, success, error): driven by `on: {signal}` via __omni_effect
    pub(super) fn emit_states_js(&mut self, nid: &str, states_ir: &StatesIR) {
        let nid_safe = nid.replace('-', "_");
        let state_names: Vec<&str> = states_ir.states.keys().map(|s| s.as_str()).collect();
        let is_reserved = state_names
            .iter()
            .all(|n| matches!(*n, "idle" | "hover" | "pressed"));

        if is_reserved {
            // Emit mouse event bindings for idle/hover/pressed state transitions
            self.mount_js.push_str(&format!(
                "// states: reserved (idle/hover/pressed)\n\
                 (function() {{\n\
                 \x20 const el = __omni_el(\"{nid}\");\n\
                 \x20 if (!el) return;\n\
                 \x20 let _state = \"idle\";\n\
                 \x20 function _transition(state) {{\n\
                 \x20\x20 if (state === _state) return;\n\
                 \x20\x20 const targets = __omni_states_{nid_safe}[state];\n\
                 \x20\x20 if (!targets) return;\n\
                 \x20\x20 const from = {{}};\n\
                 \x20\x20 for (const k of Object.keys(targets.to)) from[k] = getComputedStyle(el).getPropertyValue(k);\n\
                 \x20\x20 el.animate([from, targets.to], {{\n\
                 \x20\x20\x20 duration: targets.over || 150,\n\
                 \x20\x20\x20 easing: Omni.resolveEasing(targets.ease || \"ease-out\"),\n\
                 \x20\x20\x20 fill: \"forwards\"\n\
                 \x20\x20 }});\n\
                 \x20\x20 _state = state;\n\
                 \x20 }}\n\
                 \x20 el.addEventListener(\"mouseenter\", () => _transition(\"hover\"));\n\
                 \x20 el.addEventListener(\"mouseleave\", () => _transition(\"idle\"));\n\
                 \x20 el.addEventListener(\"mousedown\", () => _transition(\"pressed\"));\n\
                 \x20 el.addEventListener(\"mouseup\", () => _transition(\"hover\"));\n\
                 }})();\n",
                nid = nid,
                nid_safe = nid_safe
            ));
        } else if let Some(ref signal_expr) = states_ir.signal {
            // Custom states driven by signal value
            let signal_js = self.expr_to_js(signal_expr);
            let signal_access = if self.signal_names.contains(&signal_js) {
                format!("{}.value", signal_js)
            } else {
                signal_js.clone()
            };
            self.mount_js.push_str(&format!(
                "// states: custom (signal-driven)\n\
                 __omni_effect(() => {{\n\
                 \x20 const el = __omni_el(\"{nid}\");\n\
                 \x20 if (!el) return;\n\
                 \x20 const stateVal = String({sig});\n\
                 \x20 const targets = __omni_states_{nid_safe}[stateVal];\n\
                 \x20 if (!targets) return;\n\
                 \x20 const from = {{}};\n\
                 \x20 for (const k of Object.keys(targets.to)) from[k] = getComputedStyle(el).getPropertyValue(k);\n\
                 \x20 el.animate([from, targets.to], {{\n\
                 \x20\x20 duration: targets.over || 300,\n\
                 \x20\x20 easing: Omni.resolveEasing(targets.ease || \"ease-out\"),\n\
                 \x20\x20 fill: \"forwards\"\n\
                 \x20 }});\n\
                 }});\n",
                nid = nid,
                sig = signal_access,
                nid_safe = nid_safe
            ));
        }

        // Emit the state targets lookup table
        self.js.push_str(&format!(
            "var __omni_states_{nid_safe} = {{\n",
            nid_safe = nid_safe
        ));
        for (state_name, tween) in &states_ir.states {
            let over_ms = tween
                .over
                .as_ref()
                .map_or("150".to_string(), |o| self.expr_to_js(o));
            let ease_str = tween
                .ease
                .as_ref()
                .map_or("\"ease-out\"".to_string(), |e| self.easing_to_js(e));
            // Separate transform and non-transform properties
            let mut transform_parts: Vec<String> = Vec::new();
            let mut non_transform: Vec<String> = Vec::new();
            for (k, v) in &tween.properties {
                let v_js = self.expr_to_js(v);
                if Self::is_transform_prop(k) {
                    transform_parts.push(Self::format_transform_value(k, &v_js));
                } else {
                    non_transform.push(format!("\"{}\": \"{}\"", k, v_js));
                }
            }
            let mut all_props = non_transform;
            if !transform_parts.is_empty() {
                all_props.push(format!("\"transform\": \"{}\"", transform_parts.join(" ")));
            }
            self.js.push_str(&format!(
                "  \"{name}\": {{ to: {{ {props} }}, over: {over}, ease: {ease} }},\n",
                name = state_name,
                props = all_props.join(", "),
                over = over_ms,
                ease = ease_str
            ));
        }
        self.js.push_str("};\n");
    }
}
