//! Motion/animation codegen: emit_motion_js, format_transform_value, emit_states_js.
use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    /// Emit a JS function that creates a WAAPI animation for a named @motion block.
    pub(super) fn emit_motion_js(&mut self, motion: &MotionIntent) {
        let (kf_js, opts_js) = match &motion.body {
            MotionBodyIR::Keyframe(kf) => self.keyframe_to_animate_call(kf),
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
    #[allow(dead_code)]
    pub(super) fn format_transform_value(key: &str, value_js: &str) -> String {
        match key {
            "x" => format!("translateX({}px)", value_js),
            "y" => format!("translateY({}px)", value_js),
            "scale" | "scaleX" | "scaleY" => format!("{}({})", key, value_js),
            "rotate" | "rotateX" | "rotateY" | "rotateZ" => format!("{}({}deg)", key, value_js),
            _ => format!("{}({})", key, value_js),
        }
    }

}
