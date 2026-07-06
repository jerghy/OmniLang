// ═══════════════════════════════════════════════════════════
// Keyframe / WAAPI formatting helpers
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::ir::*;

/// Format a full MotionBodyIR into a JS tuple `[keyframes[], options{}, setupFn?]`.
pub(super) fn format_motion_to_waapi(body: &MotionBodyIR, warnings: &mut Vec<String>) -> String {
    match body {
        MotionBodyIR::Keyframe(kf) => {
            let keyframes = format_keyframes(kf);
            let options = format_playback_options(&kf.over, &kf.ease, &kf.playback, warnings);
            // Path setup: if motion has a path, emit a setup function
            if let Some(path) = &kf.path {
                let rot = kf.path_rotate.unwrap_or(false);
                format!(
                    "[{},{} ,(el)=>{{el.style.offsetPath='path(\"{}\")';{}}}]",
                    keyframes,
                    options,
                    path,
                    if rot { "el.style.offsetRotate='auto';" } else { "" }
                )
            } else {
                format!("[{},{}]", keyframes, options)
            }
        }
        MotionBodyIR::Timeline(_) => {
            // Timeline animations — basic fallback for now
            "[[{opacity:0,offset:0},{opacity:1,offset:1}],{duration:300,easing:\"ease-out\",fill:\"both\"}]".to_string()
        }
        MotionBodyIR::CallbackOnly(_) => {
            "[[{opacity:0,offset:0},{opacity:1,offset:1}],{duration:300,easing:\"ease-out\",fill:\"both\"}]".to_string()
        }
    }
}

/// Format keyframe stops into a WAAPI-compatible keyframes array string.
///   `[{opacity:0,transform:"translateX(-20px)",offset:0}, ...]`
fn format_keyframes(kf: &KeyframeIR) -> String {
    let mut items: Vec<String> = Vec::new();

    if kf.stops.len() == 1 {
        let stop = &kf.stops[0];
        if stop.position >= 0.5 {
            // to-only: synthetic 0% keyframe (start from current state)
            items.push("{offset:0}".to_string());
            items.push(format_stop(stop, true));
        } else {
            // from-only: synthetic 100% keyframe (return to current state)
            items.push(format_stop(stop, true));
            items.push("{offset:1}".to_string());
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
        items.push(format_stop(stop, kf.stops.len() <= 2));
    }

    format!("[{}]", items.join(","))
}

/// Format a single keyframe stop into a JS keyframe object like `{opacity:0,transform:"...",offset:0}`.
fn format_stop(stop: &KeyframeStopIR, is_binary: bool) -> String {
    let mut css_props: Vec<String> = Vec::new(); // name:value
    let mut transform_parts: Vec<String> = Vec::new(); // translateX(...) etc
    let mut filter_parts: Vec<String> = Vec::new(); // blur(...) etc

    for (prop, value) in &stop.properties {
        let val_str = expr::format_expr_ir_tsx(value);
        match prop.as_str() {
            // Transform shortcuts → combined into one `transform` string
            "x" => transform_parts.push(format!("translateX({}px)", val_str)),
            "y" => transform_parts.push(format!("translateY({}px)", val_str)),
            "rotate" => transform_parts.push(format!("rotate({}deg)", val_str)),
            "rotateX" => transform_parts.push(format!("rotateX({}deg)", val_str)),
            "rotateY" => transform_parts.push(format!("rotateY({}deg)", val_str)),
            "rotateZ" => transform_parts.push(format!("rotateZ({}deg)", val_str)),
            "scale" => transform_parts.push(format!("scale({})", val_str)),
            "scaleX" => transform_parts.push(format!("scaleX({})", val_str)),
            "scaleY" => transform_parts.push(format!("scaleY({})", val_str)),
            "skewX" => transform_parts.push(format!("skewX({}deg)", val_str)),
            "skewY" => transform_parts.push(format!("skewY({}deg)", val_str)),
            // Filter shortcuts
            "blur" => filter_parts.push(format!("blur({}px)", val_str)),
            "brightness" => filter_parts.push(format!("brightness({})", val_str)),
            "contrast" => filter_parts.push(format!("contrast({})", val_str)),
            "saturate" => filter_parts.push(format!("saturate({})", val_str)),
            "hueRotate" => filter_parts.push(format!("hue-rotate({}deg)", val_str)),
            "offsetDistance" => {
                // WAAPI requires a string like "0%" or "100%".
                // Omni uses 0..=1 range; convert to percentage.
                if let Ok(n) = val_str.parse::<f64>() {
                    css_props.push(format!("offsetDistance:\"{}%\"", n * 100.0));
                } else {
                    css_props.push(format!("offsetDistance:{}", val_str));
                }
            }
            // Everything else passes through as a camelCase CSS property
            _ => {
                if is_length_prop(prop) && is_numeric_str(&val_str) {
                    css_props.push(format!("{}:\"{}px\"", prop, val_str));
                } else {
                    css_props.push(format!("{}:{}", prop, val_str));
                }
            }
        }
    }

    // Build combined keyframe
    let mut parts: Vec<String> = Vec::new();
    parts.extend(css_props);
    if !transform_parts.is_empty() {
        parts.push(format!("transform:\"{}\"", transform_parts.join(" ")));
    }
    if !filter_parts.is_empty() {
        parts.push(format!("filter:\"{}\"", filter_parts.join(" ")));
    }
    if !is_binary {
        // Always include explicit offset for multi-stop keyframes
        parts.push(format!("offset:{}", stop.position));
    }

    format!("{{{}}}", parts.join(","))
}

/// Properties that need `px` suffix in WAAPI keyframes when value is numeric.
fn is_length_prop(prop: &str) -> bool {
    matches!(
        prop,
        "width"
            | "height"
            | "margin"
            | "padding"
            | "borderRadius"
            | "borderWidth"
            | "marginTop"
            | "marginRight"
            | "marginBottom"
            | "marginLeft"
            | "paddingTop"
            | "paddingRight"
            | "paddingBottom"
            | "paddingLeft"
            | "borderTopLeftRadius"
            | "borderTopRightRadius"
            | "borderBottomLeftRadius"
            | "borderBottomRightRadius"
            | "gap"
            | "rowGap"
            | "columnGap"
            | "maxWidth"
            | "minWidth"
            | "maxHeight"
            | "minHeight"
            | "top"
            | "left"
            | "right"
            | "bottom"
            | "fontSize"
            | "letterSpacing"
            | "wordSpacing"
    )
}

/// Check if a formatted JS value string is a bare number (not quoted).
fn is_numeric_str(s: &str) -> bool {
    !s.starts_with('"')
        && !s.starts_with('\'')
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
}

/// Format playback options into a WAAPI-compatible options object.
///   `{duration:400,easing:"ease-out",fill:"forwards"}`
fn format_playback_options(
    over: &ExprIR,
    ease: &Option<EasingIR>,
    playback: &PlaybackIR,
    warnings: &mut Vec<String>,
) -> String {
    let mut opts: Vec<String> = Vec::new();

    // Duration
    let dur_str = expr::format_expr_ir_tsx(over);
    opts.push(format!("duration:{}", dur_str));

    // Easing
    if let Some(easing) = ease {
        opts.push(format!("easing:\"{}\"", translate_easing(easing, warnings)));
    }

    // Fill mode
    if let Some(fill) = &playback.fill {
        let s = match fill {
            FillValue::None => "none",
            FillValue::Forwards => "forwards",
            FillValue::Backwards => "backwards",
            FillValue::Both => "both",
        };
        opts.push(format!("fill:\"{}\"", s));
    }

    // Direction
    if let Some(dir) = &playback.direction {
        let s = match dir {
            DirectionValue::Normal => "normal",
            DirectionValue::Reverse => "reverse",
            DirectionValue::Alternate => "alternate",
            DirectionValue::AlternateReverse => "alternate-reverse",
        };
        opts.push(format!("direction:\"{}\"", s));
    }

    // Loop count
    if let Some(loop_val) = &playback.loop_count {
        match loop_val {
            LoopValue::Infinite => opts.push("iterations:Infinity".into()),
            LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
        }
    }

    // Delay
    if let Some(delay) = &playback.delay {
        let d_str = expr::format_expr_ir_tsx(delay);
        opts.push(format!("delay:{}", d_str));
    }

    format!("{{{}}}", opts.join(","))
}

/// Translate an Omni easing IR to a WAAPI-compatible easing string.
fn translate_easing(ease: &EasingIR, warnings: &mut Vec<String>) -> String {
    match ease {
        EasingIR::Named(name) => translate_named_easing(name),
        EasingIR::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2)
        }
        EasingIR::Spring {
            tension,
            friction,
            velocity,
        } => {
            // Spring → approximate cubic-bezier based on parameters
            warnings.push(format!(
                "Spring velocity {:?} discarded in React motion conversion",
                velocity
            ));
            let ratio = if *tension > 0.0 {
                *friction / *tension
            } else {
                1.0
            };
            if ratio < 0.3 {
                // Underdamped (bouncy)
                let overshoot = (1.0 - ratio * 2.0).min(0.3);
                format!(
                    "cubic-bezier(0.250,{},0.450,{})",
                    0.460 + overshoot,
                    0.940 + overshoot
                )
            } else {
                "cubic-bezier(0.250,0.460,0.450,0.940)".to_string()
            }
        }
        EasingIR::Back {
            intensity,
            direction,
        } => {
            let dir = direction.as_deref().unwrap_or("Out");
            let i = intensity.clamp(0.5, 2.0);
            match dir {
                "In" => format!("cubic-bezier({},-{},0.735,0.045)", 0.600, 0.28 * i),
                "InOut" => format!(
                    "cubic-bezier({},-{},0.265,{})",
                    0.680,
                    0.55 * i,
                    (1.55 * i).min(1.8)
                ),
                _ => format!(
                    "cubic-bezier(0.175,{},0.320,{})",
                    0.885 * i.min(1.5),
                    (1.275 * i.min(1.5)).min(2.0)
                ),
            }
        }
        EasingIR::Elastic {
            amplitude,
            period,
            direction,
        } => {
            // Elastic → approximate cubic-bezier
            warnings.push(format!(
                "Elastic amplitude {:?} discarded in React motion conversion",
                amplitude
            ));
            warnings.push(format!(
                "Elastic period {:?} discarded in React motion conversion",
                period
            ));
            let dir = direction.as_deref().unwrap_or("Out");
            match dir {
                "In" => "cubic-bezier(0.600,0.040,0.980,0.335)".to_string(),
                "InOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)".to_string(),
                _ => "cubic-bezier(0.250,0.460,0.450,0.940)".to_string(),
            }
        }
        EasingIR::Steps { count, direction } => {
            format!("steps({},{})", count, direction)
        }
    }
}

fn translate_named_easing(name: &str) -> String {
    match name {
        // CSS standard keywords
        "linear" => "linear",
        "ease" => "ease",
        "easeIn" | "ease-in" => "ease-in",
        "easeOut" | "ease-out" => "ease-out",
        "easeInOut" | "ease-in-out" => "ease-in-out",
        // GSAP power1 (quadratic)
        "power1In" => "cubic-bezier(0.550,0.085,0.680,0.530)",
        "power1Out" => "cubic-bezier(0.250,0.460,0.450,0.940)",
        "power1InOut" => "cubic-bezier(0.455,0.030,0.515,0.955)",
        // GSAP power2 (cubic)
        "power2In" => "cubic-bezier(0.550,0.055,0.675,0.190)",
        "power2Out" => "cubic-bezier(0.215,0.610,0.355,1.000)",
        "power2InOut" => "cubic-bezier(0.645,0.045,0.355,1.000)",
        // GSAP power3 (quartic)
        "power3In" => "cubic-bezier(0.895,0.030,0.685,0.220)",
        "power3Out" => "cubic-bezier(0.165,0.840,0.440,1.000)",
        "power3InOut" => "cubic-bezier(0.770,0.000,0.175,1.000)",
        // GSAP power4 (quintic)
        "power4In" => "cubic-bezier(0.755,0.050,0.855,0.060)",
        "power4Out" => "cubic-bezier(0.230,1.000,0.320,1.000)",
        "power4InOut" => "cubic-bezier(0.860,0.000,0.070,1.000)",
        // Back easings (support overshoot in WAAPI)
        "backIn" => "cubic-bezier(0.600,-0.280,0.735,0.045)",
        "backOut" => "cubic-bezier(0.175,0.885,0.320,1.275)",
        "backInOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)",
        // Bounce → cubic-bezier approximations
        "bounceIn" => "cubic-bezier(0.215,0.610,0.355,1.000)",
        "bounceOut" => "cubic-bezier(0.215,0.610,0.355,1.000)",
        "bounceInOut" => "cubic-bezier(0.645,0.045,0.355,1.000)",
        // Elastic → cubic-bezier approximations
        "elasticIn" => "cubic-bezier(0.600,0.040,0.980,0.335)",
        "elasticOut" => "cubic-bezier(0.250,0.460,0.450,0.940)",
        "elasticInOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)",
        // Spring → generic bouncy
        "spring" => "cubic-bezier(0.250,0.460,0.450,0.940)",
        // Fallback
        _ => "ease-out",
    }
    .to_string()
}
