//! Motion and timeline validation for Pass 4.
//!
//! Validates `@motion` definitions, element-level motion triggers, and related
//! syntactic/semantic rules (easing names, keyframe types, clip-path shapes, etc.).

use super::helpers::is_non_interpolatable;
use super::motion::{is_keyword_value, is_valid_easing_name};
use super::SemanticAnalyzer;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

impl SemanticAnalyzer {
    /// Validate all motion definitions and triggers in a module.
    pub(super) fn validate_motions_in_module(&mut self, module: &Module) {
        // Collect all module-level motion names for reference resolution
        let module_motion_names: FxHashSet<&str> =
            module.motions.iter().map(|m| m.name.as_str()).collect();

        // Validate module-level motion definitions
        for motion in &module.motions {
            self.validate_motion_def(motion, &module_motion_names);
        }

        // Validate component-level motions
        for comp in &module.components {
            let comp_motion_names: FxHashSet<&str> = module_motion_names
                .iter()
                .copied()
                .chain(comp.motions.iter().map(|m| m.name.as_str()))
                .collect();

            for motion in &comp.motions {
                self.validate_motion_def(motion, &comp_motion_names);
            }

            // Validate motion triggers in view
            if let Some(ref view) = comp.view {
                self.validate_view_motion_triggers(&view.children, &comp_motion_names);
            }
        }
    }

    /// Validate a single motion definition (expanded with all spec checks).
    fn validate_motion_def(&mut self, motion: &MotionDef, _all_motion_names: &FxHashSet<&str>) {
        match &motion.body {
            MotionBody::Keyframe(kf) => {
                // Check: from must be paired with to
                let has_from = kf
                    .stops
                    .iter()
                    .any(|s| matches!(s.position, StopPosition::From));
                let has_to = kf
                    .stops
                    .iter()
                    .any(|s| matches!(s.position, StopPosition::To));
                if has_from && !has_to {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::E0006,
                        &format!("@motion '{}': 'from' must be paired with 'to'", motion.name),
                        &motion.span,
                    ));
                }

                // Check: non-interpolatable properties
                for stop in &kf.stops {
                    for (prop, expr) in &stop.properties {
                        if is_non_interpolatable(prop) {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0006,
                                &format!("@motion '{}': '{}' is not interpolatable and cannot be animated", motion.name, prop),
                                &motion.span,
                            ));
                        }
                        // Check: keyword values in animation (e.g. width: auto→300px)
                        if let Expr::VarRef(name, _) = expr {
                            if is_keyword_value(name) {
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0006,
                                    &format!("@motion '{}': keyword value '{}' cannot be used in animation for property '{}'", motion.name, name, prop),
                                    &motion.span,
                                ));
                            }
                        }
                    }
                }

                // Check: type mismatch across stops
                self.validate_keyframe_type_mismatch(motion, kf);

                // Check: valid easing name
                if let Some(ref e) = kf.ease {
                    self.validate_easing_name(e, &motion.span);
                }
                for stop in &kf.stops {
                    if let Some(ref e) = stop.ease {
                        self.validate_easing_name(e, &motion.span);
                    }
                }

                // Check: clipPath with different shape functions
                self.validate_clip_path_shapes(motion, kf);

                // Check: boxShadow with different value structures
                self.validate_box_shadow_structure(motion, kf);

                // Warning: d attribute path structure mismatch
                self.warn_path_structure_mismatch(motion, kf);

                // Validate playback property conflicts
                self.validate_playback_conflicts(motion, &kf.playback);
            }
        }

        // Emit warnings for all motion types
        self.emit_motion_warnings(motion);
    }

    /// Validate playback property conflicts.
    fn validate_playback_conflicts(&mut self, motion: &MotionDef, playback: &[PlaybackProp]) {
        let mut has_loop_infinite = false;
        let mut has_fill_both = false;

        for prop in playback {
            match prop {
                PlaybackProp::Loop(lv) => {
                    has_loop_infinite = matches!(lv, LoopValue::Infinite);
                }
                PlaybackProp::Fill(fv) => {
                    has_fill_both = matches!(fv, FillValue::Both);
                }
                PlaybackProp::Direction(dv) => {
                    // Warning: direction: alternate + loop: 1
                    if let Some(lv) = playback.iter().find_map(|p| {
                        if let PlaybackProp::Loop(lv) = p {
                            Some(lv)
                        } else {
                            None
                        }
                    }) {
                        if matches!(
                            dv,
                            DirectionValue::Alternate | DirectionValue::AlternateReverse
                        ) && matches!(lv, LoopValue::Count(1))
                        {
                            self.diagnostics.push(Diagnostic::warning(
                                DiagnosticKind::WMotion,
                                &format!("@motion '{}': direction 'alternate' with loop: 1 — animation plays forward then reverse; is this intended?", motion.name),
                                &motion.span,
                            ));
                        }
                    }
                }
                PlaybackProp::Paused(_) => {
                    // No conflict, just a state flag
                }
                PlaybackProp::Delay(_) => {}
            }
        }

        // Check fill: both + loop: infinite
        if has_fill_both && has_loop_infinite {
            self.diagnostics.push(Diagnostic::warning(
                DiagnosticKind::WMotion,
                &format!(
                    "@motion '{}': fill 'both' has no meaning with loop 'infinite'",
                    motion.name
                ),
                &motion.span,
            ));
        }
    }

    /// Validate that an easing expression uses a recognized name.
    fn validate_easing_name(&mut self, e: &EasingExpr, span: &Span) {
        if let EasingExpr::Named(name) = e {
            if !is_valid_easing_name(name) {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::E0006,
                    &format!("Unknown easing name '{}'", name),
                    span,
                ));
            }
        }
        if let EasingExpr::Back {
            direction: Some(ref dir_name),
            ..
        } = e
        {
            if !is_valid_easing_name(dir_name) {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::E0006,
                    &format!("Unknown easing name '{}'", dir_name),
                    span,
                ));
            }
        }
        if let EasingExpr::Elastic {
            direction: Some(ref dir_name),
            ..
        } = e
        {
            if !is_valid_easing_name(dir_name) {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::E0006,
                    &format!("Unknown easing name '{}'", dir_name),
                    span,
                ));
            }
        }
    }

    /// Validate element-level motion properties.
    pub(super) fn validate_element_motion_props(
        &mut self,
        el: &Element,
        all_motion_names: &FxHashSet<&str>,
    ) {
        for prop in &el.props {
            if let ElementProp::MotionTrigger(trigger) = prop {
                if let Expr::VarRef(name, span) = &trigger.motion_expr {
                    if !all_motion_names.contains(name.as_str()) {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            &format!("Undefined motion '{}' referenced in element trigger", name),
                            span,
                        ));
                    }
                }
            }
        }
    }

    /// Validate @if mode string.
    pub(super) fn validate_if_mode(&mut self, mode: &str, span: &Span) {
        if mode != "sync" && mode != "wait" {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0006,
                &format!("@if mode '{}' is invalid; expected 'sync' or 'wait'", mode),
                span,
            ));
        }
    }

    /// Check for type mismatches across keyframe stops for the same property.
    fn validate_keyframe_type_mismatch(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        let mut prop_types: FxHashMap<&str, &str> = FxHashMap::default();
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                let tag = match expr {
                    Expr::Literal(lit, _) => match lit {
                        Literal::Int(_) | Literal::Float(_) => "number",
                        Literal::String(_) => "string",
                        Literal::Bool(_) => "bool",
                        Literal::Null => "null",
                    },
                    _ => "unknown",
                };
                if let Some(prev_tag) = prop_types.get(prop.as_str()) {
                    if *prev_tag != tag && *prev_tag != "unknown" && tag != "unknown" {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            &format!(
                                "@motion '{}': type mismatch for property '{}' ({} vs {})",
                                motion.name, prop, prev_tag, tag
                            ),
                            &motion.span,
                        ));
                    }
                } else {
                    prop_types.insert(prop.as_str(), tag);
                }
            }
        }
    }

    /// Validate clipPath shape consistency across keyframe stops.
    fn validate_clip_path_shapes(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                if prop == "clipPath" || prop == "clip-path" {
                    if let Expr::Literal(Literal::String(val), _) = expr {
                        let shape = val.trim_start().split('(').next().unwrap_or("");
                        for other_stop in &kf.stops {
                            for (other_prop, other_expr) in &other_stop.properties {
                                if other_prop == prop {
                                    if let Expr::Literal(Literal::String(other_val), _) = other_expr
                                    {
                                        let other_shape =
                                            other_val.trim_start().split('(').next().unwrap_or("");
                                        if shape != other_shape
                                            && !shape.is_empty()
                                            && !other_shape.is_empty()
                                        {
                                            self.diagnostics.push(Diagnostic::error(
                                                DiagnosticKind::E0006,
                                                &format!("@motion '{}': clipPath shape mismatch — '{}()' and '{}()' are different shape functions", motion.name, shape, other_shape),
                                                &motion.span,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate boxShadow value structure consistency.
    fn validate_box_shadow_structure(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        let mut first_box_shadow: Option<&str> = None;
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                if prop == "boxShadow" || prop == "box-shadow" {
                    if let Expr::Literal(Literal::String(val), _) = expr {
                        if let Some(first) = first_box_shadow {
                            let has_rgba1 = first.contains("rgba");
                            let has_rgba2 = val.contains("rgba");
                            if has_rgba1 != has_rgba2 {
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0006,
                                    &format!("@motion '{}': boxShadow value structure mismatch — one uses rgba() and the other does not", motion.name),
                                    &motion.span,
                                ));
                            }
                        } else {
                            first_box_shadow = Some(val);
                        }
                    }
                }
            }
        }
    }

    /// Warning: SVG path 'd' attribute structure mismatch.
    fn warn_path_structure_mismatch(&mut self, motion: &MotionDef, kf: &KeyframeBody) {
        let mut first_path: Option<&str> = None;
        for stop in &kf.stops {
            for (prop, expr) in &stop.properties {
                if prop == "d" {
                    if let Expr::Literal(Literal::String(val), _) = expr {
                        if let Some(first) = first_path {
                            let cmds1: Vec<char> =
                                first.chars().filter(|c| c.is_alphabetic()).collect();
                            let cmds2: Vec<char> =
                                val.chars().filter(|c| c.is_alphabetic()).collect();
                            if cmds1 != cmds2 {
                                self.diagnostics.push(Diagnostic::warning(
                                    DiagnosticKind::WMotion,
                                    &format!("@motion '{}': SVG path 'd' attribute command structure differs between keyframe stops — interpolation may not work as expected", motion.name),
                                    &motion.span,
                                ));
                            }
                        } else {
                            first_path = Some(val);
                        }
                    }
                }
            }
        }
    }
}

// ── Motion Warnings ─────────────────────────

impl SemanticAnalyzer {
    /// Emit warnings for motion-specific issues.
    fn emit_motion_warnings(&mut self, motion: &MotionDef) {
        self.check_over_duration_warnings(motion);
        self.check_opacity_range_warnings(motion);
    }

    fn check_over_duration_warnings(&mut self, motion: &MotionDef) {
        use super::motion::parse_duration_ms;

        let over_exprs: Vec<&Expr> = match &motion.body {
            MotionBody::Keyframe(kf) => vec![&kf.over],
        };

        for expr in &over_exprs {
            if let Expr::Literal(Literal::String(val), _) = expr {
                if val == "0ms" || val == "0s" {
                    self.diagnostics.push(Diagnostic::warning(
                        DiagnosticKind::WMotion,
                        &format!("@motion '{}': over: 0ms — consider using 'set' instead for instant assignment", motion.name),
                        &motion.span,
                    ));
                }
                match parse_duration_ms(val) {
                    Ok(ms) if ms > 3_600_000.0 => {
                        self.diagnostics.push(Diagnostic::warning(
                            DiagnosticKind::WMotion,
                            &format!(
                                "@motion '{}': over > 3600s ({}) — suspiciously long duration",
                                motion.name, val
                            ),
                            &motion.span,
                        ));
                    }
                    Err(err_msg) => {
                        self.diagnostics.push(Diagnostic::warning(
                            DiagnosticKind::WMotion,
                            &format!("@motion '{}': {}", motion.name, err_msg),
                            &motion.span,
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    fn check_opacity_range_warnings(&mut self, motion: &MotionDef) {
        let props: Vec<&(String, Expr)> = match &motion.body {
            MotionBody::Keyframe(kf) => kf.stops.iter().flat_map(|s| &s.properties).collect(),
        };

        for (prop, expr) in &props {
            if prop == "opacity" {
                if let Expr::Literal(lit, _) = expr {
                    match lit {
                        Literal::Float(f) if *f < 0.0 || *f > 1.0 => {
                            self.diagnostics.push(Diagnostic::warning(
                                DiagnosticKind::WMotion,
                                &format!(
                                    "@motion '{}': opacity value {} is outside [0, 1] range",
                                    motion.name, f
                                ),
                                &motion.span,
                            ));
                        }
                        Literal::Int(i) if *i > 1 => {
                            self.diagnostics.push(Diagnostic::warning(
                                DiagnosticKind::WMotion,
                                &format!(
                                    "@motion '{}': opacity value {} is outside [0, 1] range",
                                    motion.name, i
                                ),
                                &motion.span,
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
