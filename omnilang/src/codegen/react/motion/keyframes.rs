// ═══════════════════════════════════════════════════════════
// Keyframe formatting helpers — native WAAPI el.animate()
// ═══════════════════════════════════════════════════════════

use crate::codegen::react::expr;
use crate::codegen::{escape_literal_str, format_json_value, JSON_REACT};
use crate::ir::*;
use rustc_hash::FxHashSet;

/// Format a MotionBodyIR into JS: `{kf:[...],opts:{...}}` for use with the
/// native WAAPI `el.animate(kf, opts)`. Keyframe stops are converted from
/// semantic properties to WAAPI property names:
///
/// - `opacity` → `opacity`
/// - `scale`/`scaleX`/`scaleY`/`translateX`/`translateY`/`translateZ`/
///   `rotate`/`rotateX`/`rotateY`/`rotateZ`/`skewX`/`skewY` → `transform`
///   (composed into a single transform string)
/// - `backgroundColor`/`color`/`width`/`height` → passed through as-is
pub(super) fn format_motion_to_animate(motion: &MotionBodyIR) -> String {
    let kf_items = format_waapi_keyframes(motion);
    let opts = format_animate_options(&motion.over, &motion.ease, &motion.playback);
    // `as KeyframeAnimationOptions` — the DOM type for el.animate() options;
    // without it TS widens `fill:"forwards"` to `string` and rejects the call
    // (KeyframeAnimationOptions.fill is a narrow union).
    format!(
        "{{kf:{kf},opts:{opts} as KeyframeAnimationOptions}}",
        kf = kf_items,
        opts = opts
    )
}

/// Whether a motion body is a compile-time constant — every keyframe stop
/// value, `over`, and `delay` must be a literal (numbers/strings/bools, with
/// literal negation). Non-literal values (store fields, signals, calls) are
/// resolved at render time and keep the `_M` table inside the component
/// function (task 2.6 — the table hoist is only sound for constants).
pub(super) fn motion_body_is_constant(body: &MotionBodyIR) -> bool {
    if !expr_is_constant(&body.over) {
        return false;
    }
    if let Some(ref delay) = body.playback.delay {
        if !expr_is_constant(delay) {
            return false;
        }
    }
    body.stops
        .iter()
        .all(|stop| stop.properties.values().all(expr_is_constant))
}

/// A literal-only expression (negative literals included — `-5` lowers to
/// `UnaryOpExpr(Neg, 5)`); anything else is render-time dependent.
fn expr_is_constant(expr: &ExprIR) -> bool {
    match expr {
        ExprIR::LiteralExpr { .. } => true,
        ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Neg,
            operand,
        } => expr_is_constant(operand),
        _ => false,
    }
}

/// Transform-family semantic properties, in canonical composition order.
/// FxHashMap iteration order is nondeterministic, so transform parts are
/// always emitted in this fixed order for deterministic output.
const TRANSFORM_PROPS: [&str; 12] = [
    "translateX",
    "translateY",
    "translateZ",
    "scale",
    "scaleX",
    "scaleY",
    "rotate",
    "rotateX",
    "rotateY",
    "rotateZ",
    "skewX",
    "skewY",
];

/// Length-valued CSS properties that need a "px" unit when the stop value is
/// a bare number. WAAPI keyframe values must be valid CSS values for their
/// property — a bare number is not a valid length (Chrome leniently treats
/// numbers as px, Firefox/Safari reject the keyframe). The OmniView runtime
/// applies the same rule for its PX_PROPS set. Semantic-set additions from
/// Task 5.2; width/height keep their legacy emission.
const PX_SUFFIX_PROPS: [&str; 2] = ["borderRadius", "fontSize"];

/// Format keyframe stops into WAAPI `Keyframe[]` format.
///
/// Each stop becomes a flat keyframe object: `{offset, opacity, transform}`
/// (one `transform` entry per stop, composed from all transform-family
/// properties present in the stop).
fn format_waapi_keyframes(kf: &KeyframeIR) -> String {
    let mut items: Vec<String> = Vec::new();

    if kf.stops.len() == 1 {
        let stop = &kf.stops[0];
        if stop.position >= 0.5 {
            // to-only: synthetic 0% keyframe (start from current state)
            items.push("{offset:0}".to_string());
            items.push(format_waapi_stop(stop));
        } else {
            // from-only: synthetic 100% keyframe (return to current state)
            items.push(format_waapi_stop(stop));
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
        items.push(format_waapi_stop(stop));
    }

    format!("[{}]", items.join(","))
}

/// Format one keyframe stop as a flat WAAPI keyframe object.
fn format_waapi_stop(stop: &KeyframeStopIR) -> String {
    let mut entries: Vec<String> = vec![format!("offset:{}", stop.position)];

    // Transform-family props → one composed `transform` string.
    // Plain props are emitted sorted for deterministic output.
    let mut transform_parts: Vec<String> = Vec::new();
    let mut plain_pairs: Vec<(&String, &ExprIR)> = Vec::new();
    for (prop, value) in &stop.properties {
        if TRANSFORM_PROPS.contains(&prop.as_str()) {
            transform_parts.push(format_transform_part(prop, value));
        } else {
            plain_pairs.push((prop, value));
        }
    }
    if !transform_parts.is_empty() {
        entries.push(format_transform_entry(&transform_parts));
    }
    plain_pairs.sort_by(|a, b| a.0.cmp(b.0));
    for (prop, value) in plain_pairs {
        let mut val_str = expr::format_expr_ir_tsx(value, &FxHashSet::default());
        // Length properties: bare numbers need a px unit to be valid CSS
        // keyframe values (borderRadius/fontSize, Task 5.2 semantic set).
        // String literals already carry their unit; expressions are emitted
        // as-is (their runtime value must be a CSS string or number).
        if PX_SUFFIX_PROPS.contains(&prop.as_str()) {
            if let Some(num) = literal_number_string(value) {
                val_str = format!("\"{num}px\"");
            }
        }
        entries.push(format!("{}:{}", prop, val_str));
    }

    // Per-keyframe easing (WAAPI-native)
    if let Some(ref ease) = stop.ease {
        entries.push(format!("easing:{}", format_easing_to_js(ease)));
    }

    format!("{{{}}}", entries.join(","))
}

/// Extract a literal number from an ExprIR — including unary negation
/// (`-5` lowers to `UnaryOpExpr(Neg, 5)`). Returns the JSON-formatted number.
fn literal_number_string(value: &ExprIR) -> Option<String> {
    match value {
        ExprIR::LiteralExpr { value: v } if v.is_number() => {
            Some(format_json_value(v, &JSON_REACT))
        }
        ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Neg,
            operand,
        } => {
            if let ExprIR::LiteralExpr { value: v } = operand.as_ref() {
                if v.is_number() {
                    return Some(format!("-{}", format_json_value(v, &JSON_REACT)));
                }
            }
            None
        }
        _ => None,
    }
}

/// Format one transform component, e.g. `translateY(20px)`, `scale(1.05)`,
/// `rotate(45deg)`. Literal values are inlined; expressions are embedded
/// with template-literal `${...}` escapes.
fn format_transform_part(prop: &str, value: &ExprIR) -> String {
    // `translate*` values are lengths (px unless the source said otherwise);
    // `rotate*`/`skew*` are angles (deg); `scale*` are unitless factors.
    let unit = match prop {
        "translateX" | "translateY" | "translateZ" => "px",
        "rotate" | "rotateX" | "rotateY" | "rotateZ" | "skewX" | "skewY" => "deg",
        _ => "",
    };
    if let Some(num) = literal_number_string(value) {
        return format!("{}({}{})", prop, num, unit);
    }
    match value {
        ExprIR::LiteralExpr { value: v } => {
            if let Some(s) = v.as_str() {
                // Literal string: already carries its own unit, if any.
                // Escape for BOTH JS string contexts the entry wrapper may
                // use (security HIGH-1, verified `[alert(1)]` vector):
                // - the plain `"…"` string: a raw `"` in the value would
                //   re-close the literal and leave the rest of the payload
                //   in code position;
                // - the template literal used when a sibling part is an
                //   expression: a raw `` ` `` would terminate the template
                //   and a raw `${` would start interpolation.
                // `\${` and `` \` `` are inert in plain strings and escape
                // the template forms; JS drops the backslash of
                // non-escape sequences, so the runtime value is unchanged.
                let escaped = escape_literal_str(s, '"')
                    .replace('`', "\\`")
                    .replace("${", "\\${");
                return format!("{}({})", prop, escaped);
            }
            // Non-numeric non-string literal (bool/null) — emit as-is
            let js = expr::format_expr_ir_tsx(value, &FxHashSet::default());
            format!("{}({})", prop, js)
        }
        _ => {
            // Expression — embed as a template-literal escape so units
            // compose: `translateX(${dx}px)`. The entry wrapper adds the
            // backticks. Built with push_str to avoid format! brace escapes.
            let js = expr::format_expr_ir_tsx(value, &FxHashSet::default());
            let mut s = String::new();
            s.push_str(prop);
            s.push('(');
            s.push_str("${");
            s.push_str(&js);
            s.push('}');
            s.push_str(unit);
            s.push(')');
            s
        }
    }
}

/// Join transform parts into a single `transform` entry. If every part is a
/// plain literal, emit a plain quoted string; otherwise emit a template
/// literal so `${expr}` escapes compose with literal parts.
fn format_transform_entry(parts: &[String]) -> String {
    let is_template = parts.iter().any(|p| p.contains("${"));
    if is_template {
        format!("transform:`{}`", parts.join(" "))
    } else {
        format!("transform:\"{}\"", parts.join(" "))
    }
}

/// Format motion options for the native WAAPI animate() call.
fn format_animate_options(over: &ExprIR, ease: &Option<EasingIR>, playback: &PlaybackIR) -> String {
    let mut opts: Vec<String> = Vec::new();
    let dur = duration_expr_to_js(over, expr::format_expr_ir_tsx(over, &FxHashSet::default()));
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
        // #[non_exhaustive]
        _ => "\"forwards\"",
    };
    opts.push(format!("fill:{}", fill));
    // direction
    if let Some(dir) = &playback.direction {
        let d = match dir {
            DirectionValue::Normal => "normal",
            DirectionValue::Reverse => "reverse",
            DirectionValue::Alternate => "alternate",
            DirectionValue::AlternateReverse => "alternate-reverse",
            // #[non_exhaustive]
            _ => "normal",
        };
        opts.push(format!("direction:\"{}\"", d));
    }
    // iterations
    if let Some(loop_val) = &playback.loop_count {
        match loop_val {
            LoopValue::Infinite => opts.push("iterations:Infinity".into()),
            LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
            // #[non_exhaustive]
            _ => opts.push("iterations:Infinity".into()),
        }
    }
    // delay
    if let Some(delay) = &playback.delay {
        opts.push(format!(
            "delay:{}",
            duration_expr_to_js(
                delay,
                expr::format_expr_ir_tsx(delay, &FxHashSet::default())
            )
        ));
    }
    format!("{{{}}}", opts.join(","))
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

/// Format easing value as a JS string literal.
fn format_easing_to_js(ease: &EasingIR) -> String {
    let val = translate_easing(ease);
    format!("\"{}\"", val)
}

/// Translate an Omni easing IR to a WAAPI-compatible easing string.
fn translate_easing(ease: &EasingIR) -> String {
    match ease {
        EasingIR::Named(name) => translate_named_easing(name),
        EasingIR::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2)
        }
        EasingIR::Back { direction } => {
            let dir = direction.as_deref().unwrap_or("out");
            // Capitalize first letter: "in" → "In", "inout" → "InOut"
            let capitalized: String = {
                let mut c = dir.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            };
            let name = format!("back{}", capitalized);
            omnilang_core::codegen::shared::easing::resolve_easing(&name)
                .unwrap_or("cubic-bezier(0.175,0.885,0.320,1.275)")
                .to_string()
        }
        EasingIR::Elastic { direction } => {
            let dir = direction.as_deref().unwrap_or("out");
            let capitalized: String = {
                let mut c = dir.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            };
            let name = format!("elastic{}", capitalized);
            omnilang_core::codegen::shared::easing::resolve_easing(&name)
                .unwrap_or("cubic-bezier(0.250,0.460,0.450,0.940)")
                .to_string()
        }
        // #[non_exhaustive] — future easing kinds degrade to linear.
        _ => "linear".to_string(),
    }
}

fn translate_named_easing(name: &str) -> String {
    // "steps(N,dir)" strings (from steps() easing) are not in NAMED_EASINGS
    // but are natively supported by WAAPI — emit them as-is.
    if name.starts_with("steps(") {
        return name.to_string();
    }
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
