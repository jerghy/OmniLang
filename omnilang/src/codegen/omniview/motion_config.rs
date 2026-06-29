use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    // Motion / Animation Codegen
    // ══════════════════════════════════════════════════════

    /// Convert a MotionBodyIR to a JS config object string for AnimationPlayer.
    pub(super) fn motion_body_to_config(&self, body: &MotionBodyIR) -> String {
        match body {
            MotionBodyIR::Keyframe(kf) => self.keyframe_to_config(kf),
            MotionBodyIR::Timeline(tl) => self.timeline_body_to_config(tl),
            MotionBodyIR::CallbackOnly(cb) => self.callback_to_config(cb),
        }
    }

    /// Convert a KeyframeIR to a JS config object fragment.
    pub(super) fn keyframe_to_config(&self, kf: &KeyframeIR) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Extract from (position 0.0) and to (position 1.0) stops
        let from_stop = kf
            .stops
            .iter()
            .find(|s| (s.position - 0.0).abs() < f64::EPSILON);
        let to_stop = kf
            .stops
            .iter()
            .find(|s| (s.position - 1.0).abs() < f64::EPSILON);

        if let Some(from) = from_stop {
            let props: Vec<String> = from
                .properties
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                .collect();
            if !props.is_empty() {
                parts.push(format!("\"from\": {{{}}}", props.join(", ")));
            }
        }

        if let Some(to) = to_stop {
            let props: Vec<String> = to
                .properties
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                .collect();
            if !props.is_empty() {
                parts.push(format!("\"to\": {{{}}}", props.join(", ")));
            }
        }

        // `over` duration (in milliseconds)
        match &kf.over {
            ExprIR::LiteralExpr { value } => {
                if let Some(n) = value.as_f64() {
                    parts.push(format!("\"over\": {}", n));
                }
            }
            _ => {
                let js = self.expr_to_js(&kf.over);
                parts.push(format!("\"over\": {}", js));
            }
        }

        // Easing
        if let Some(ref ease) = kf.ease {
            parts.push(format!("\"easing\": {}", self.easing_to_js(ease)));
        }

        // Path animation
        if let Some(ref path) = kf.path {
            parts.push(format!("\"path\": \"{}\"", path.replace('"', "\\\"")));
            if let Some(rotate) = kf.path_rotate {
                parts.push(format!("\"pathRotate\": {}", rotate));
            }
        }

        // Playback config properties
        let playback_parts = self.playback_to_config(&kf.playback);
        parts.extend(playback_parts);

        // Path animation support
        if let Some(ref path) = kf.path {
            parts.push(format!("\"path\": \"{}\"", path.replace('"', "\\\"")));
        }
        if let Some(path_rotate) = kf.path_rotate {
            if path_rotate {
                parts.push("\"pathRotate\": true".to_string());
            }
        }

        // Callback blocks (onStart, onUpdate, onComplete, onRepeat, onInterrupt)
        if !kf.callbacks.is_empty() {
            let callbacks: Vec<String> = kf
                .callbacks
                .iter()
                .map(|(kind, expr)| {
                    let name = match kind {
                        CallbackKindIR::OnStart => "onStart",
                        CallbackKindIR::OnUpdate => "onUpdate",
                        CallbackKindIR::OnComplete => "onComplete",
                        CallbackKindIR::OnRepeat => "onRepeat",
                        CallbackKindIR::OnInterrupt => "onInterrupt",
                    };
                    let body_js = self.expr_to_js(expr);
                    format!("\"{}\": function() {{ {} }}", name, body_js)
                })
                .collect();
            parts.push(format!("\"callbacks\": {{{}}}", callbacks.join(", ")));
        }

        parts.join(", ")
    }

    /// Convert a TimelineIR body to a JS config fragment.
    pub(super) fn timeline_body_to_config(&self, tl: &TimelineIR) -> String {
        // For inline timeline in a motion definition, we serialize as a
        // keyframe-like structure. The timeline orchestration is handled
        // at the element trigger level via emit_timeline_js.
        let mut parts: Vec<String> = Vec::new();
        parts.push("\"type\": \"timeline\"".into());
        if !tl.items.is_empty() {
            parts.push(format!("\"itemCount\": {}", tl.items.len()));
        }
        let playback_parts = self.playback_to_config(&tl.playback);
        parts.extend(playback_parts);
        parts.join(", ")
    }

    /// Convert a CallbackBodyIR to a JS config fragment.
    pub(super) fn callback_to_config(&self, cb: &CallbackBodyIR) -> String {
        let mut parts: Vec<String> = Vec::new();
        // `over` duration
        match &cb.over {
            ExprIR::LiteralExpr { value } => {
                if let Some(n) = value.as_f64() {
                    parts.push(format!("\"over\": {}", n));
                }
            }
            _ => {
                let js = self.expr_to_js(&cb.over);
                parts.push(format!("\"over\": {}", js));
            }
        }
        // Callback functions
        if !cb.callbacks.is_empty() {
            let callbacks: Vec<String> = cb
                .callbacks
                .iter()
                .map(|(kind, expr)| {
                    let name = match kind {
                        CallbackKindIR::OnStart => "onStart",
                        CallbackKindIR::OnUpdate => "onUpdate",
                        CallbackKindIR::OnComplete => "onComplete",
                        CallbackKindIR::OnRepeat => "onRepeat",
                        CallbackKindIR::OnInterrupt => "onInterrupt",
                    };
                    // Callback body is an expression that becomes a function body
                    let body_js = self.expr_to_js(expr);
                    format!("\"{}\": function() {{ {} }}", name, body_js)
                })
                .collect();
            parts.push(format!("\"callbacks\": {{{}}}", callbacks.join(", ")));
        }
        let playback_parts = self.playback_to_config(&cb.playback);
        parts.extend(playback_parts);
        parts.join(", ")
    }

    /// Convert an EasingIR to a JS string or expression.
    pub(super) fn easing_to_js(&self, easing: &EasingIR) -> String {
        match easing {
            EasingIR::Named(name) => format!("\"{}\"", name),
            EasingIR::CubicBezier { x1, y1, x2, y2 } => {
                format!("\"cubic-bezier({},{},{},{})\"", x1, y1, x2, y2)
            }
            EasingIR::Spring {
                tension,
                friction,
                velocity,
            } => {
                format!(
                    "new SpringSolver({}, {}, 1, {})",
                    tension,
                    friction,
                    velocity.unwrap_or(0.0)
                )
            }
            EasingIR::Back {
                intensity: _,
                direction,
            } => {
                // The parser stores the full name ("backIn"), also support short form ("in")
                let dir = direction.as_deref().unwrap_or("backOut");
                let name = match dir {
                    "backIn" | "in" => "backIn",
                    "backOut" | "out" => "backOut",
                    "backInOut" | "inOut" => "backInOut",
                    _ => "backOut",
                };
                format!("\"{}\"", name)
            }
            EasingIR::Elastic {
                amplitude: _,
                period: _,
                direction,
            } => {
                // The parser stores the full name ("elasticIn"), also support short form ("in")
                let dir = direction.as_deref().unwrap_or("elasticOut");
                let name = match dir {
                    "elasticIn" | "in" => "elasticIn",
                    "elasticOut" | "out" => "elasticOut",
                    "elasticInOut" | "inOut" => "elasticInOut",
                    _ => "elasticOut",
                };
                format!("\"{}\"", name)
            }
            EasingIR::Steps { count, direction } => {
                format!("\"steps({},{})\"", count, direction)
            }
        }
    }

    /// Convert PlaybackIR to a list of JS config property strings.
    pub(super) fn playback_to_config(&self, playback: &PlaybackIR) -> Vec<String> {
        let mut parts = Vec::new();
        if let Some(ref loop_count) = playback.loop_count {
            match loop_count {
                LoopValue::Infinite => parts.push("\"iterations\": Infinity".into()),
                LoopValue::Count(n) => parts.push(format!("\"iterations\": {}", n)),
            }
        }
        if let Some(ref direction) = playback.direction {
            let dir_str = match direction {
                DirectionValue::Normal => "normal",
                DirectionValue::Reverse => "reverse",
                DirectionValue::Alternate => "alternate",
                DirectionValue::AlternateReverse => "alternate-reverse",
            };
            parts.push(format!("\"direction\": \"{}\"", dir_str));
        }
        if let Some(ref fill) = playback.fill {
            let fill_str = match fill {
                FillValue::None => "none",
                FillValue::Forwards => "forwards",
                FillValue::Backwards => "backwards",
                FillValue::Both => "both",
            };
            parts.push(format!("\"fill\": \"{}\"", fill_str));
        }
        if let Some(ref delay) = playback.delay {
            match delay {
                ExprIR::LiteralExpr { value } => {
                    if let Some(n) = value.as_f64() {
                        parts.push(format!("\"delay\": {}", n));
                    }
                }
                _ => {
                    let js = self.expr_to_js(delay);
                    parts.push(format!("\"delay\": {}", js));
                }
            }
        }
        if playback.paused {
            parts.push("\"paused\": true".into());
        }
        if playback.respect_reduced_motion {
            parts.push("\"respectReducedMotion\": true".into());
        }
        if let Some(ref when) = playback.when {
            let when_str = match when {
                WhenValue::Idle => "beforeChildren",
                WhenValue::Active => "afterChildren",
                WhenValue::IdleOrActive => "withChildren",
            };
            parts.push(format!("\"when\": \"{}\"", when_str));
        }
        if let Some(ref dc) = playback.delay_children {
            let js = self.expr_to_js(dc);
            parts.push(format!("\"delayChildren\": {}", js));
        }
        if let Some(ref sc) = playback.stagger_children {
            let js = self.expr_to_js(sc);
            parts.push(format!("\"staggerChildren\": {}", js));
        }
        if let Some(ref sf) = playback.stagger_from {
            let from_str = match sf {
                StaggerDirection::First => "start",
                StaggerDirection::Last => "end",
                StaggerDirection::Center => "center",
                StaggerDirection::Edges => "edges",
                StaggerDirection::Random => "edges",
            };
            parts.push(format!("\"staggerFrom\": \"{}\"", from_str));
        }
        if let Some(ref rd) = playback.repeat_delay {
            let js = self.expr_to_js(rd);
            parts.push(format!("\"repeatDelay\": {}", js));
        }
        if playback.repeat_refresh {
            parts.push("\"repeatRefresh\": true".into());
        }
        if let Some(ref ts) = playback.time_scale {
            let js = self.expr_to_js(ts);
            parts.push(format!("\"timeScale\": {}", js));
        }
        if let Some(ref interrupt) = playback.interrupt {
            let int_str = match interrupt {
                InterruptValue::Compose => "compose",
                InterruptValue::Cancel => "cancel",
                InterruptValue::Finish => "finish",
                InterruptValue::Replace => "replace",
            };
            parts.push(format!("\"interrupt\": \"{}\"", int_str));
        }
        parts
    }

    /// Check if a property is a CSS transform property.
    pub(super) fn is_transform_prop(key: &str) -> bool {
        matches!(
            key,
            "x" | "y"
                | "scale"
                | "scaleX"
                | "scaleY"
                | "rotate"
                | "rotateX"
                | "rotateY"
                | "rotateZ"
        )
    }
}
