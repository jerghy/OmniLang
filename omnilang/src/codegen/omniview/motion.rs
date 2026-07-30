//! Motion/animation codegen: emit_motion_js, emit_states_js.
use super::OmniViewGenerator;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    /// Emit a JS function that creates a WAAPI animation for a named @motion block.
    pub(super) fn emit_motion_js(&mut self, motion: &MotionIntent) {
        let (kf_js, opts_js) = match &motion.body {
            kf => self.keyframe_to_animate_call(kf),
        };

        let _ = writeln!(
            self.js,
            "function __omni_motion_{name}(el, overrides) {{\n\
             \x20 const opts = Object.assign({{{opts}}}, overrides || {{}});\n\
             \x20 const kf = {kf};\n\
             \x20 return Omni.animate(el, kf, opts);\n\
             }};",
            name = motion.name,
            opts = opts_js,
            kf = kf_js,
        );
    }
}
