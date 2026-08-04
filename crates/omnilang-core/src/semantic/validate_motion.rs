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

        // Validate component-level motions. Motion names are module-scoped in
        // the IR (ModuleIR.motions / motion_index are name-keyed), so a
        // component motion colliding with a module-level motion or another
        // component's motion is a hard error — never silent shadowing.
        let mut seen_motion_names: FxHashSet<&str> = module_motion_names.iter().copied().collect();
        for comp in &module.components {
            let comp_motion_names: FxHashSet<&str> = module_motion_names
                .iter()
                .copied()
                .chain(comp.motions.iter().map(|m| m.name.as_str()))
                .collect();

            for motion in &comp.motions {
                if !seen_motion_names.insert(motion.name.as_str()) {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::E0006,
                        &format!(
                            "@motion '{}' in component '{}' collides with an existing motion name — motion names are module-scoped; rename it or move the definition to module level",
                            motion.name, comp.name
                        ),
                        &motion.span,
                    ));
                }
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
                if let Some(ref e) = kf.easing {
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
        // Back/Elastic direction strings ("in", "out", "inout") are validated by the parser
        // and handled correctly by codegen. No need to re-check against the full easing table.
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
                self.validate_trigger_params(trigger);
            }
        }
    }

    /// Validate the v1 control dimensions of `@scroll` / `@drag-*` triggers
    /// (Task 5.1, v2 BNF): `start`/`end` must be numbers, come in pairs, and
    /// form a strictly increasing range (`start < end` — an inverted or
    /// zero-width range would divide by zero on React/OmniView and silently
    /// no-op on Flutter); `scrub` must be a bool; `constraints` must be
    /// `[min, max]` numbers with min < max; `elastic`/`threshold`/
    /// `velocity_max` must be non-negative numbers. Errors are hard
    /// diagnostics in Chinese per the v2 policy — never parse-and-ignore.
    fn validate_trigger_params(&mut self, trigger: &MotionTrigger) {
        let span = trigger_expr_span(&trigger.motion_expr);
        match &trigger.trigger {
            TriggerKind::Scroll { start, end, scrub } => {
                if start.is_some() != end.is_some() {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::E0006,
                        "「start」与「end」必须成对提供（滚动映射区间）",
                        &span,
                    ));
                }
                for (name, expr) in [("start", start), ("end", end)] {
                    if let Some(e) = expr {
                        if !is_number_literal_expr(e) {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0006,
                                &format!("「{}」必须是数字（滚动位置）", name),
                                &span,
                            ));
                        }
                    }
                }
                // start == end (or inverted) would divide by zero / invert the
                // mapping on React/OmniView and silently no-op on Flutter —
                // require a strictly increasing range, mirroring the drag
                // constraints min < max check.
                if let (Some(s), Some(e)) = (start, end) {
                    if is_number_literal_expr(s)
                        && is_number_literal_expr(e)
                        && number_literal_value(s) >= number_literal_value(e)
                    {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            "「start」必须小于「end」（滚动映射区间）",
                            &span,
                        ));
                    }
                }
                if let Some(e) = scrub {
                    if !matches!(e, Expr::Literal(Literal::Bool(_), _)) {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            "「scrub」必须是布尔值 true/false",
                            &span,
                        ));
                    }
                }
            }
            TriggerKind::DragHorizontal {
                constraints,
                elastic,
                threshold,
                velocity_max,
            }
            | TriggerKind::DragVertical {
                constraints,
                elastic,
                threshold,
                velocity_max,
            } => {
                let label = if matches!(trigger.trigger, TriggerKind::DragHorizontal { .. }) {
                    "@drag-horizontal"
                } else {
                    "@drag-vertical"
                };
                if let Some((min, max)) = constraints {
                    if !is_number_literal_expr(min) || !is_number_literal_expr(max) {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            &format!("{}「constraints」必须是 [min, max] 两个数字", label),
                            &span,
                        ));
                    } else if number_literal_value(min) >= number_literal_value(max) {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            &format!("{}「constraints」min 必须小于 max", label),
                            &span,
                        ));
                    }
                }
                for (name, expr) in [
                    ("elastic", elastic),
                    ("threshold", threshold),
                    ("velocity_max", velocity_max),
                ] {
                    if let Some(e) = expr {
                        if !is_number_literal_expr(e) {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0006,
                                &format!("{}「{}」必须是数字", label, name),
                                &span,
                            ));
                        } else if number_literal_value(e) < 0.0 {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0006,
                                &format!("{}「{}」必须是非负数字（≥ 0）", label, name),
                                &span,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Validate @if mode string.
    ///
    /// Defense-in-depth: the parser already restricts `mode:` values to
    /// "fade" | "wait" | "none", but the semantic pass also runs on ASTs
    /// built programmatically (editor tooling, tests), so keep the check
    /// aligned with the parser's whitelist.
    pub(super) fn validate_if_mode(&mut self, mode: &str, span: &Span) {
        if !matches!(mode, "fade" | "wait" | "none") {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0006,
                &format!(
                    "@if mode '{}' is invalid; expected 'fade', 'wait', or 'none'",
                    mode
                ),
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

/// Span of a motion trigger's binding expr for diagnostics.
fn trigger_expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::VarRef(_, span) => *span,
        Expr::MapLiteral { span, .. } => *span,
        Expr::ArrayLiteral { span, .. } => *span,
        _ => Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        },
    }
}

/// True when the expr is a numeric literal (`0`, `1000`, `0.5`) or a
/// negated numeric literal (`-100`). The @scroll/@drag control dimensions
/// (Task 5.1) must be literal numbers so backends can consume them
/// statically.
fn is_number_literal_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(Literal::Int(_) | Literal::Float(_), _) => true,
        Expr::UnaryOp {
            op: UnaryOpKind::Neg,
            operand,
            ..
        } => matches!(
            operand.as_ref(),
            Expr::Literal(Literal::Int(_) | Literal::Float(_), _)
        ),
        _ => false,
    }
}

/// Numeric value of a number-literal expr (see [`is_number_literal_expr`]).
fn number_literal_value(expr: &Expr) -> f64 {
    match expr {
        Expr::Literal(Literal::Int(n), _) => *n as f64,
        Expr::Literal(Literal::Float(f), _) => *f,
        Expr::UnaryOp {
            op: UnaryOpKind::Neg,
            operand,
            ..
        } => -number_literal_value(operand),
        _ => f64::NAN,
    }
}

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

#[cfg(test)]
mod trigger_param_validation_tests {
    use super::*;

    /// Run the full pipeline over a source carrying one motion trigger and
    /// return the diagnostics.
    fn diags(source: &str) -> Vec<Diagnostic> {
        let result = crate::compile(source);
        result.errors
    }

    fn wrap(attr: &str, motion_body: &str) -> String {
        format!(
            "@motion m {{ {} }}\ncomponent C {{\n  view {{\n    <div {}>x</div>\n  }}\n}}",
            motion_body, attr
        )
    }

    #[test]
    fn valid_scroll_params_produce_no_errors() {
        let d = diags(&wrap(
            "@scroll start: 0 end: 1000 scrub: true ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.is_empty(),
            "valid @scroll params must compile clean, got: {:?}",
            d
        );
    }

    #[test]
    fn scroll_start_without_end_is_error() {
        let d = diags(&wrap(
            "@scroll start: 0 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter().any(|e| e.message.contains("必须成对")),
            "unpaired start must be diagnosed in Chinese, got: {:?}",
            d
        );
    }

    #[test]
    fn scroll_non_numeric_start_is_error() {
        let d = diags(&wrap(
            "@scroll start: \"top\" end: 1000 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter().any(|e| e.message.contains("start」必须是数字")),
            "string start must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn scroll_non_bool_scrub_is_error() {
        let d = diags(&wrap(
            "@scroll start: 0 end: 1000 scrub: 1 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter().any(|e| e.message.contains("scrub」必须是布尔值")),
            "numeric scrub must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn scroll_start_equal_end_is_error() {
        // Zero-width range → division by zero (NaN progress) on React/OmniView
        // and a silent no-op on Flutter — must be a hard error.
        let d = diags(&wrap(
            "@scroll start: 1000 end: 1000 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter()
                .any(|e| e.message.contains("「start」必须小于「end」")),
            "start == end must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn scroll_inverted_range_is_error() {
        let d = diags(&wrap(
            "@scroll start: 1000 end: 0 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter()
                .any(|e| e.message.contains("「start」必须小于「end」")),
            "inverted range must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn valid_drag_params_produce_no_errors() {
        let d = diags(&wrap(
            "@drag-horizontal constraints: [-100, 100] elastic: 0.5 threshold: 0.3 velocity_max: 500 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.is_empty(),
            "valid @drag params must compile clean, got: {:?}",
            d
        );
    }

    #[test]
    fn drag_constraints_min_ge_max_is_error() {
        let d = diags(&wrap(
            "@drag-horizontal constraints: [100, -100] ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter().any(|e| e.message.contains("min 必须小于 max")),
            "inverted constraints must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn drag_constraints_non_numeric_is_error() {
        let d = diags(&wrap(
            "@drag-horizontal constraints: [\"a\", 100] ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter().any(|e| e
                .message
                .contains("constraints」必须是 [min, max] 两个数字")),
            "non-numeric constraints must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn drag_negative_elastic_is_error() {
        let d = diags(&wrap(
            "@drag-horizontal constraints: [0, 100] elastic: -0.5 ={m}",
            "from { opacity: 0 } to { opacity: 1 }",
        ));
        assert!(
            d.iter()
                .any(|e| e.message.contains("elastic」必须是非负数字")),
            "negative elastic must be diagnosed, got: {:?}",
            d
        );
    }

    #[test]
    fn plain_triggers_still_compile() {
        let d = diags(&wrap("@enter={m}", "from { opacity: 0 } to { opacity: 1 }"));
        assert!(
            d.is_empty(),
            "plain @enter must compile clean, got: {:?}",
            d
        );
    }
}
