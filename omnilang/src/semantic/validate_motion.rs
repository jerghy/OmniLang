//! Motion and timeline validation for Pass 4.
//!
//! Validates `@motion` definitions, `@timeline` definitions, element-level
//! motion triggers, `states:` blocks, and related syntactic/semantic rules
//! (easing names, keyframe types, clip-path shapes, etc.).

use std::collections::{HashMap, HashSet};

use super::helpers::{collect_timeline_refs, is_non_interpolatable};
use super::motion::{
    collect_element_ids, contains_velocity_ref, is_keyword_value, is_valid_easing_name,
};
use super::SemanticAnalyzer;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Validate all motion definitions and timeline triggers in a module.
    pub(super) fn validate_motions_in_module(&mut self, module: &Module) {
        // Collect all module-level motion/timeline names for reference resolution
        let module_motion_names: HashSet<&str> = module
            .motions
            .iter()
            .map(|m| m.name.as_str())
            .chain(module.timelines.iter().map(|t| t.name.as_str()))
            .collect();

        // Validate module-level motion definitions
        for motion in &module.motions {
            self.validate_motion_def(motion, &module_motion_names);
        }

        // Validate module-level timeline definitions
        let timeline_pairs: Vec<(&str, Vec<&str>)> = module
            .timelines
            .iter()
            .map(|tl| (tl.name.as_str(), collect_timeline_refs(tl)))
            .collect();
        for timeline in &module.timelines {
            self.validate_timeline_def(timeline, &module_motion_names);
        }
        self.detect_timeline_cycles(&timeline_pairs);

        // Validate component-level motions and timelines
        for comp in &module.components {
            let comp_motion_names: HashSet<&str> = module_motion_names
                .iter()
                .copied()
                .chain(comp.motions.iter().map(|m| m.name.as_str()))
                .chain(comp.timelines.iter().map(|t| t.name.as_str()))
                .collect();

            for motion in &comp.motions {
                self.validate_motion_def(motion, &comp_motion_names);
            }

            let comp_timeline_pairs: Vec<(&str, Vec<&str>)> = comp
                .timelines
                .iter()
                .map(|tl| (tl.name.as_str(), collect_timeline_refs(tl)))
                .collect();
            for timeline in &comp.timelines {
                self.validate_timeline_def(timeline, &comp_motion_names);
            }
            self.detect_timeline_cycles(&comp_timeline_pairs);

            // Validate motion triggers in view
            if let Some(ref view) = comp.view {
                // Collect element ids for @timeline validation
                let element_ids = collect_element_ids(&view.children);
                self.validate_view_motion_triggers(
                    &view.children,
                    &comp_motion_names,
                    &element_ids,
                );
            }
        }
    }

    /// Validate a single motion definition (expanded with all spec §20.1 checks).
    fn validate_motion_def(&mut self, motion: &MotionDef, all_motion_names: &HashSet<&str>) {
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
            MotionBody::Timeline(tl) => {
                // Check: set is allowed in timeline; stagger is allowed in timeline
                // Check: valid easing names in timeline items
                for item in &tl.items {
                    if let TimelineItemKind::Direction(dir) = &item.kind {
                        if let Some(ref e) = dir.ease {
                            self.validate_easing_name(e, &motion.span);
                        }
                    }
                }
                for item in &tl.items {
                    match &item.kind {
                        TimelineItemKind::Direction(dir) => {
                            for (prop, expr) in &dir.properties {
                                if is_non_interpolatable(prop) {
                                    self.diagnostics.push(Diagnostic::error(
                                        DiagnosticKind::E0006,
                                        &format!("@motion '{}': '{}' is not interpolatable and cannot be animated", motion.name, prop),
                                        &motion.span,
                                    ));
                                }
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
                        TimelineItemKind::MotionRef(ref_name) => {
                            if !all_motion_names.contains(ref_name.as_str()) {
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0006,
                                    &format!(
                                        "@motion '{}': references undefined motion '{}'",
                                        motion.name, ref_name
                                    ),
                                    &motion.span,
                                ));
                            }
                            // Check circular self-reference
                            if ref_name == &motion.name {
                                self.diagnostics.push(Diagnostic::error(
                                    DiagnosticKind::E0006,
                                    &format!(
                                        "@motion '{}': circular reference to itself",
                                        motion.name
                                    ),
                                    &motion.span,
                                ));
                            }
                        }
                        TimelineItemKind::Stagger(stagger) => {
                            if let Expr::VarRef(ref_name, _) = stagger.motion.as_ref() {
                                if !all_motion_names.contains(ref_name.as_str()) {
                                    self.diagnostics.push(Diagnostic::error(
                                        DiagnosticKind::E0006,
                                        &format!(
                                            "@motion '{}': references undefined motion '{}'",
                                            motion.name, ref_name
                                        ),
                                        &motion.span,
                                    ));
                                }
                                if ref_name == &motion.name {
                                    self.diagnostics.push(Diagnostic::error(
                                        DiagnosticKind::E0006,
                                        &format!("@motion '{}': circular reference to itself via stagger", motion.name),
                                        &motion.span,
                                    ));
                                }
                            }
                        }
                        TimelineItemKind::SetBlock(_)
                        | TimelineItemKind::Label(_, _)
                        | TimelineItemKind::StartPos(_) => {}
                    }
                }

                // Validate playback property conflicts
                self.validate_playback_conflicts(motion, &tl.playback);
            }
            MotionBody::CallbackOnly(cb) => {
                // Check: $velocity only valid in gesture contexts
                for spec in &cb.callbacks {
                    let expr = match spec {
                        CallbackSpec::OnStart(e)
                        | CallbackSpec::OnUpdate(e)
                        | CallbackSpec::OnComplete(e)
                        | CallbackSpec::OnRepeat(e)
                        | CallbackSpec::OnInterrupt(e) => e,
                    };
                    if contains_velocity_ref(expr) {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            &format!("@motion '{}': '$velocity' is only valid in @drag-horizontal/@drag-vertical callbacks, not in standalone motion definitions", motion.name),
                            &motion.span,
                        ));
                        break;
                    }
                }
            }
        }

        // Emit warnings for all motion types
        self.emit_motion_warnings(motion);
    }

    /// Validate playback property conflicts (§20.1).
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
                PlaybackProp::When(_) => {
                    self.diagnostics.push(Diagnostic::error(
                        DiagnosticKind::E0006,
                        &format!("@motion '{}': 'when' is only valid on @enter/@leave element triggers, not on @motion definitions", motion.name),
                        &motion.span,
                    ));
                }
                PlaybackProp::DelayChildren(_) | PlaybackProp::StaggerChildren(_) => {
                    // These ARE valid on @motion definitions — they configure
                    // how the motion distributes animations among child elements.
                    // The runtime processes them via playback_to_config.
                }
                _ => {}
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

    /// Validate element-level animation conflicts.
    pub(super) fn validate_element_motion_props(
        &mut self,
        el: &Element,
        all_motion_names: &HashSet<&str>,
    ) {
        let mut has_driven_by = false;
        let mut motion_count: u32 = 0;

        for prop in &el.props {
            match prop {
                ElementProp::DrivenBy(_) => {
                    has_driven_by = true;
                }
                ElementProp::MotionTrigger(trigger) => {
                    motion_count += 1;
                    // Check driven-by + loop conflict: can't fully check here
                    // since loop is in the motion def; validate when resolving
                    if let Expr::VarRef(name, span) = &trigger.motion_expr {
                        if !all_motion_names.contains(name.as_str()) {
                            self.diagnostics.push(Diagnostic::error(
                                DiagnosticKind::E0006,
                                &format!(
                                    "Undefined motion '{}' referenced in element trigger",
                                    name
                                ),
                                span,
                            ));
                        }
                    }
                }
                ElementProp::When(_)
                | ElementProp::DelayChildren(_)
                | ElementProp::StaggerChildren(_) => {
                    // These are only valid on @enter/@leave triggers
                    // Check: if there's no @enter or @leave trigger, warn
                    let has_enter_leave = el.props.iter().any(|p| {
                        if let ElementProp::MotionTrigger(mt) = p {
                            matches!(mt.trigger, TriggerKind::Enter | TriggerKind::Leave)
                        } else {
                            false
                        }
                    });
                    if !has_enter_leave {
                        self.diagnostics.push(Diagnostic::error(
                            DiagnosticKind::E0006,
                            "when:/delayChildren:/staggerChildren: are only valid on @enter/@leave triggers",
                            &el.span,
                        ));
                    }
                }
                ElementProp::StatesMap(states) => {
                    // Validate states: block
                    self.validate_states_block(el, states);
                }
                _ => {}
            }
        }

        // Warning: >10 animations on a single element (performance risk)
        if motion_count > 10 {
            self.diagnostics.push(Diagnostic::warning(
                DiagnosticKind::WMotion,
                &format!(
                    "Element has {} motion triggers — performance risk (>10 recommended max)",
                    motion_count
                ),
                &el.span,
            ));
        }

        // Warning: driven-by + loop conflict (if element has both)
        if has_driven_by {
            for prop in &el.props {
                if let ElementProp::MotionTrigger(trigger) = prop {
                    if let Expr::VarRef(_name, _) = &trigger.motion_expr {
                        // If the referenced motion has loop, that's a conflict
                        // This requires cross-referencing with motion defs
                        // For now, mark as warning
                    }
                }
            }
        }
    }

    /// Validate states: block (§20.1).
    fn validate_states_block(&mut self, el: &Element, states: &[(String, StateDef)]) {
        let has_reserved = states
            .iter()
            .any(|(name, _)| matches!(name.as_str(), "idle" | "hover" | "pressed"));
        let has_custom = !has_reserved;
        let has_on_signal = el
            .props
            .iter()
            .any(|p| matches!(p, ElementProp::AnimateSignal(_)));

        if has_reserved && has_custom && has_on_signal {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0006,
                "states: cannot mix reserved names (idle/hover/pressed) with custom names and on: signal binding",
                &el.span,
            ));
        }

        // Custom states require on: signal
        if !has_reserved && !has_on_signal {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0006,
                "states: with custom state names requires on: signal binding to drive state selection",
                &el.span,
            ));
        }

        // Reserved names with on: present is an error
        if has_reserved && has_on_signal {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0006,
                "states: reserved names (idle/hover/pressed) auto-bind to mouse events; on: is only valid with custom state names",
                &el.span,
            ));
        }

        // Check for duplicate state names
        let mut seen: HashSet<&str> = HashSet::new();
        for (name, _) in states {
            if !seen.insert(name.as_str()) {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::E0006,
                    &format!("Duplicate state name '{}' in states: block", name),
                    &el.span,
                ));
            }
        }
    }

    /// Validate @if mode string (§20.1).
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
        let mut prop_types: HashMap<&str, &str> = HashMap::new();
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
                        // Extract shape function name (e.g., "circle", "inset", "polygon")
                        let shape = val.trim_start().split('(').next().unwrap_or("");
                        // Record and compare across stops
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

// ── Motion Warnings (§20.2) ─────────────────────────

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
            MotionBody::Timeline(tl) => tl
                .items
                .iter()
                .filter_map(|item| match &item.kind {
                    TimelineItemKind::Direction(dir) => Some(&dir.over),
                    _ => None,
                })
                .collect(),
            MotionBody::CallbackOnly(cb) => vec![&cb.over],
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
            MotionBody::Timeline(tl) => tl
                .items
                .iter()
                .filter_map(|item| match &item.kind {
                    TimelineItemKind::Direction(dir) => Some(&dir.properties),
                    TimelineItemKind::SetBlock(props) => Some(props),
                    _ => None,
                })
                .flat_map(|p| p.iter())
                .collect(),
            _ => vec![],
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
