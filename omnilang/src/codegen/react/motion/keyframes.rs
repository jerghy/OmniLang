// ═══════════════════════════════════════════════════════════
// Keyframe formatting helpers for @omni/animation
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::ir::*;

/// Format a MotionBodyIR into JS: `{kf:{...},opts:{...}}` for use with animate().
pub(super) fn format_motion_to_animate(motion: &MotionBodyIR) -> String {
    match motion {
        MotionBodyIR::Keyframe(kf) => {
            let kf_items = format_keyframe_stops(kf);
            let opts = format_animate_options(&kf.over, &kf.ease, &kf.playback);
            format!("{{kf:{kf},opts:{opts}}}", kf = kf_items, opts = opts)
        }
    }
}

/// Format keyframe stops into the KeyframeStop[] format (not WAAPI Keyframe[]).
fn format_keyframe_stops(kf: &KeyframeIR) -> String {
    let mut items: Vec<String> = Vec::new();

    if kf.stops.len() == 1 {
        let stop = &kf.stops[0];
        let mut prop_pairs: Vec<String> = Vec::new();
        for (prop, value) in &stop.properties {
            let val_str = expr::format_expr_ir_tsx(value);
            prop_pairs.push(format!("\"{}\":{}", prop, val_str));
        }
        if stop.position >= 0.5 {
            // to-only: synthetic 0% keyframe (start from current state)
            items.push("{offset:0,properties:{}}".to_string());
            items.push(format!(
                "{{offset:{},properties:{{{}}}}}",
                stop.position,
                prop_pairs.join(",")
            ));
        } else {
            // from-only: synthetic 100% keyframe (return to current state)
            items.push(format!(
                "{{offset:{},properties:{{{}}}}}",
                stop.position,
                prop_pairs.join(",")
            ));
            items.push("{offset:1,properties:{}}".to_string());
        }
        return format!("[{}]", items.join(","));
    }

    // Sort stops by position
    let mut sorted_stops = kf.stops.clone();
    sorted_stops.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for stop in &sorted_stops {
        let mut prop_pairs: Vec<String> = Vec::new();
        for (prop, value) in &stop.properties {
            let val_str = expr::format_expr_ir_tsx(value);
            prop_pairs.push(format!("\"{}\":{}", prop, val_str));
        }
        items.push(format!(
            "{{offset:{},properties:{{{}}}}}",
            stop.position,
            prop_pairs.join(",")
        ));
    }

    format!("[{}]", items.join(","))
}

/// Format motion options for animate() call.
fn format_animate_options(over: &ExprIR, ease: &Option<EasingIR>, playback: &PlaybackIR) -> String {
    let mut opts: Vec<String> = Vec::new();
    let dur = expr::format_expr_ir_tsx(over);
    opts.push(format!("duration:{}", dur));
    if let Some(e) = ease {
        opts.push(format!("easing:{}", format_easing_to_js(e)));
    }
    // fill
    let fill = match &playback.fill {
        Some(FillValue::Forwards) | None => "\"forwards\"",
        Some(FillValue::None) => "\"none\"",
        Some(FillValue::Backwards) => "\"backwards\"",
        Some(FillValue::Both) => "\"both\"",
    };
    opts.push(format!("fill:{}", fill));
    // direction
    if let Some(dir) = &playback.direction {
        let d = match dir {
            DirectionValue::Normal => "normal",
            DirectionValue::Reverse => "reverse",
            DirectionValue::Alternate => "alternate",
            DirectionValue::AlternateReverse => "alternate-reverse",
        };
        opts.push(format!("direction:\"{}\"", d));
    }
    // iterations
    if let Some(loop_val) = &playback.loop_count {
        match loop_val {
            LoopValue::Infinite => opts.push("iterations:Infinity".into()),
            LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
        }
    }
    // delay
    if let Some(delay) = &playback.delay {
        opts.push(format!("delay:{}", expr::format_expr_ir_tsx(delay)));
    }
    format!("{{{}}}", opts.join(","))
}

/// Format easing value as a JS string literal.
fn format_easing_to_js(ease: &EasingIR) -> String {
    let mut warnings = Vec::new();
    let val = translate_easing(ease, &mut warnings);
    format!("\"{}\"", val)
}

/// Translate an Omni easing IR to a WAAPI-compatible easing string.
fn translate_easing(ease: &EasingIR, warnings: &mut Vec<String>) -> String {
    match ease {
        EasingIR::Named(name) => translate_named_easing(name),
        EasingIR::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2)
        }
        EasingIR::Back { direction } => {
            let dir = direction.as_deref().unwrap_or("Out");
            match dir {
                "In" => "cubic-bezier(0.600,0.165,0.735,0.045)".to_string(),
                "InOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)".to_string(),
                _ => "cubic-bezier(0.175,0.885,0.320,1.275)".to_string(),
            }
        }
        EasingIR::Elastic { direction } => {
            let dir = direction.as_deref().unwrap_or("Out");
            match dir {
                "In" => "cubic-bezier(0.600,0.040,0.980,0.335)".to_string(),
                "InOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)".to_string(),
                _ => "cubic-bezier(0.250,0.460,0.450,0.940)".to_string(),
            }
        }
    }
}

fn translate_named_easing(name: &str) -> String {
    // Normalize CSS kebab-case to Omni camelCase naming for standard keywords.
    let normalized = match name {
        "ease-in" => "easeIn",
        "ease-out" => "easeOut",
        "ease-in-out" => "easeInOut",
        _ => name,
    };
    omnilang_core::codegen::shared::easing::resolve_easing(normalized)
        .unwrap_or("ease-out")
        .to_string()
}
