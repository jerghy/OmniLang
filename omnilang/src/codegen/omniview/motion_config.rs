use super::OmniViewGenerator;
use crate::codegen::escape_literal_str;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    // Motion / Animation Codegen
    // ══════════════════════════════════════════════════════

    /// Convert a MotionBodyIR to (keyframes_js, opts_js) for Omni.animate().
    pub(super) fn motion_body_to_config(&mut self, body: &MotionBodyIR) -> (String, String) {
        match body {
            kf => self.keyframe_to_animate_call(kf),
        }
    }

    /// Convert an EasingIR to a JS expression for Omni.resolveEasing().
    pub(super) fn easing_to_js(&self, easing: &EasingIR) -> String {
        match easing {
            EasingIR::Named(name) => format!("\"{}\"", escape_literal_str(name, '"')),
            EasingIR::CubicBezier { x1, y1, x2, y2 } => {
                format!(
                    "{{type:\"cubicBezier\",x1:{},y1:{},x2:{},y2:{}}}",
                    x1, y1, x2, y2
                )
            }
            EasingIR::Back { direction } => {
                let dir = direction.as_deref().unwrap_or("out");
                format!(
                    "{{type:\"back\",direction:\"{}\"}}",
                    escape_literal_str(dir, '"')
                )
            }
            EasingIR::Elastic { direction } => {
                let dir = direction.as_deref().unwrap_or("out");
                format!(
                    "{{type:\"elastic\",direction:\"{}\"}}",
                    escape_literal_str(dir, '"')
                )
            }
        }
    }

    /// Emit JS code that creates keyframes + options and calls Omni.animate().
    /// Returns (keyframes_js, options_js) — two JS expressions.
    pub(super) fn keyframe_to_animate_call(&mut self, kf: &KeyframeIR) -> (String, String) {
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
                let _ = write!(item, ",easing:{}", ease_js);
            }
            kf_items.push(item);
        }
        let kf_js = format!("[{}]", kf_items.join(","));

        // Build options object
        let mut opts: Vec<String> = Vec::new();
        let dur_js = duration_expr_to_js(&kf.over, self.expr_to_js(&kf.over));
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
            let d = duration_expr_to_js(delay, self.expr_to_js(delay));
            opts.push(format!("delay:{}", d));
        }
        // No braces around opts — the factory's Object.assign({{{opts}}}) adds them
        let opts_js = opts.join(",");

        (kf_js, opts_js)
    }
}

/// Convert a motion `over`/`delay` ExprIR to a numeric milliseconds JS
/// literal. WAAPI's `el.animate()` rejects string durations (`"500ms"`)
/// with a TypeError, so durations must always emit as numbers.
///
/// Semantics mirror the Flutter backend's `extract_duration_ms` and the
/// shared `parse_duration_ms`:
/// - `"300ms"` → `300`, `"1.5s"` → `1500`
/// - bare numeric strings and numeric literals `< 1.0` are seconds
///   (`"0.5"` → `500`)
/// - numeric literals `>= 1.0` are milliseconds (`over: 400` → `400`)
/// - non-literal expressions (signal-driven delays) emit `raw` unchanged
fn duration_expr_to_js(expr: &ExprIR, raw: String) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::Number(n) => match n.as_f64() {
                Some(ms) if ms < 1.0 => format!("{:.0}", (ms * 1000.0).round()),
                Some(ms) => format!("{:.0}", ms.round()),
                None => raw,
            },
            serde_json::Value::String(s) => {
                if let Ok(ms) = omnilang_core::semantic::parse_duration_ms(s) {
                    format!("{:.0}", ms.round())
                } else {
                    match s.trim().parse::<f64>() {
                        Ok(secs) => format!("{:.0}", (secs * 1000.0).round()),
                        Err(_) => raw,
                    }
                }
            }
            _ => raw,
        },
        _ => raw,
    }
}
