use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    // Motion / Animation Codegen
    // ══════════════════════════════════════════════════════

    /// Convert a MotionBodyIR to (keyframes_js, opts_js) for Omni.animate().
    pub(super) fn motion_body_to_config(&self, body: &MotionBodyIR) -> (String, String) {
        match body {
            MotionBodyIR::Keyframe(kf) => self.keyframe_to_animate_call(kf),
            MotionBodyIR::Timeline(tl) => self.timeline_to_factory("_inline_timeline", tl),
            MotionBodyIR::CallbackOnly(cb) => self.callback_to_animate_call(cb),
        }
    }


    /// Convert an EasingIR to a JS expression for Omni.resolveEasing().
    /// Output format: a string like `"easeOut"` or an object like
    /// `{type:"spring", tension:180, friction:12}` for structured easings.
    pub(super) fn easing_to_js(&self, easing: &EasingIR) -> String {
        match easing {
            EasingIR::Named(name) => format!("\"{}\"", name),
            EasingIR::CubicBezier { x1, y1, x2, y2 } => {
                format!(
                    "{{type:\"cubicBezier\",x1:{},y1:{},x2:{},y2:{}}}",
                    x1, y1, x2, y2
                )
            }
            EasingIR::Spring {
                tension,
                friction,
                velocity,
            } => {
                let vel = velocity.unwrap_or(0.0);
                format!(
                    "{{type:\"spring\",tension:{},friction:{},velocity:{}}}",
                    tension, friction, vel
                )
            }
            EasingIR::Back {
                intensity,
                direction,
            } => {
                let dir = direction.as_deref().unwrap_or("out");
                format!(
                    "{{type:\"back\",intensity:{},direction:\"{}\"}}",
                    intensity, dir
                )
            }
            EasingIR::Elastic {
                amplitude,
                period,
                direction,
            } => {
                let dir = direction.as_deref().unwrap_or("out");
                format!(
                    "{{type:\"elastic\",amplitude:{},period:{},direction:\"{}\"}}",
                    amplitude, period, dir
                )
            }
            EasingIR::Steps { count, direction } => {
                format!(
                    "{{type:\"steps\",count:{},direction:\"{}\"}}",
                    count, direction
                )
            }
        }
    }

    /// Emit JS code that creates keyframes + options and calls Omni.animate().
    /// Returns (keyframes_js, options_js) — two JS expressions.
    pub(super) fn keyframe_to_animate_call(&self, kf: &KeyframeIR) -> (String, String) {
        // Build keyframes array: [{offset:0,properties:{opacity:"0",transform:"translateY(20px)"}}, ...]
        let mut kf_items: Vec<String> = Vec::new();
        for stop in &kf.stops {
            let mut props: Vec<String> = Vec::new();
            for (prop, value) in &stop.properties {
                let val_js = self.expr_to_js(value);
                props.push(format!("\"{}\":{}", prop, val_js));
            }
            let mut item = format!(
                "{{offset:{},properties:{{{}}}}}",
                stop.position,
                props.join(",")
            );
            if let Some(ref ease) = stop.ease {
                let ease_js = self.easing_to_js(ease);
                item.push_str(&format!(",easing:{}", ease_js));
            }
            kf_items.push(item);
        }
        let kf_js = format!("[{}]", kf_items.join(","));

        // Build options object
        let mut opts: Vec<String> = Vec::new();
        let dur_js = self.expr_to_js(&kf.over);
        opts.push(format!("duration:{}", dur_js));
        if let Some(ref ease) = kf.ease {
            let ease_js = self.easing_to_js(ease);
            // resolveEasing at runtime
            opts.push(format!("easing:Omni.resolveEasing({})", ease_js));
        }
        // Playback options (fill, direction, iterations, delay)
        let fill_str = match &kf.playback.fill {
            Some(FillValue::None) => "\"none\"",
            Some(FillValue::Forwards) => "\"forwards\"",
            Some(FillValue::Backwards) => "\"backwards\"",
            Some(FillValue::Both) => "\"both\"",
            None => "\"both\"",
        };
        opts.push(format!("fill:{}", fill_str));
        if let Some(ref dir) = kf.playback.direction {
            let d = match dir {
                DirectionValue::Normal => "normal",
                DirectionValue::Reverse => "reverse",
                DirectionValue::Alternate => "alternate",
                DirectionValue::AlternateReverse => "alternate-reverse",
            };
            opts.push(format!("direction:\"{}\"", d));
        }
        if let Some(ref loop_val) = kf.playback.loop_count {
            match loop_val {
                LoopValue::Infinite => opts.push("iterations:Infinity".into()),
                LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
            }
        }
        if let Some(ref delay) = kf.playback.delay {
            let d = self.expr_to_js(delay);
            opts.push(format!("delay:{}", d));
        }
        // No braces around opts — the factory's Object.assign({{{opts}}}) adds them
        let opts_js = opts.join(",");

        (kf_js, opts_js)
    }

    /// Emit a factory for timeline-based motion definitions.
    /// Returns (keyframes_js, opts_js).
    pub(super) fn timeline_to_factory(&self, _name: &str, tl: &TimelineIR) -> (String, String) {
        let kf_js = "[{offset:0}]".to_string();
        let mut opts: Vec<String> = Vec::new();
        opts.push("duration:300".into());
        let playback_parts = self.playback_to_config(&tl.playback);
        opts.extend(playback_parts);
        // No braces — factory adds them via {{{opts}}}
        let opts_js = opts.join(",");
        (kf_js, opts_js)
    }

    /// Convert a CallbackBodyIR to (keyframes_js, opts_js) for Omni.animate().
    /// Callback-only animations have no visual properties, just duration and callbacks.
    pub(super) fn callback_to_animate_call(&self, cb: &CallbackBodyIR) -> (String, String) {
        let mut opts: Vec<String> = Vec::new();
        let dur_js = self.expr_to_js(&cb.over);
        opts.push(format!("duration:{}", dur_js));
        let playback_parts = self.playback_to_config(&cb.playback);
        opts.extend(playback_parts);
        // Callback functions as extra options
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
                    let body_js = self.expr_to_js(expr);
                    format!("\"{}\":function(){{ {} }}", name, body_js)
                })
                .collect();
            opts.push(format!("callbacks:{{{}}}", callbacks.join(",")));
        }
        // No braces — factory adds them via {{{opts}}}
        let opts_js = opts.join(",");
        let kf_js = "[{offset:0},{offset:1}]".to_string();
        (kf_js, opts_js)
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
