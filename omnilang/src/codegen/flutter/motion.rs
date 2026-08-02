//! Motion / Animation codegen helpers for Flutter codegen.
//!
//! Per-property animation model (Task 5.4): every animated semantic property
//! of a motion gets its own `Animation<T>` field (`_{motion}{Prop}`) driven
//! by the shared `_{motion}Controller`, so multi-property and multi-stop
//! keyframes interpolate each property independently:
//!
//! - 2 stops → `Tween<T>(begin: ..., end: ...)`
//! - 3+ stops → `TweenSequence<T>` with weight-proportional segments
//!
//! The `AnimatedBuilder` body nests widget wrappers (Opacity, Transform.*,
//! Container, DefaultTextStyle, SizedBox) per property.

use crate::codegen::flutter::{capitalize_first, css_color_to_dart, format_expr_ir_flutter};
use crate::ir::*;

use rustc_hash::FxHashSet;
use std::cmp::Ordering;

// ═══════════════════════════════════════════════════════════════════
// Motion / Animation codegen helpers
// ═══════════════════════════════════════════════════════════════════

/// Extract the "to" properties from a keyframe to determine what properties are animated.
pub(crate) fn extract_keyframe_properties(kf: &KeyframeIR) -> Vec<String> {
    let mut props: Vec<String> = Vec::new();
    for stop in &kf.stops {
        for key in stop.properties.keys() {
            if !props.contains(key) {
                props.push(key.clone());
            }
        }
    }
    props
}

/// Semantic properties the Flutter motion codegen maps onto widgets.
///
/// Anything outside this set is skipped with a warning (see
/// [`prop_flutter_diagnostic`]) — the cross-target feature floor covers the
/// core transform/opacity/color family; skews are flagged separately.
/// `borderRadius`/`fontSize` (Task 5.2) animate via `BorderRadiusTween`
/// (Flutter SDK class) and `TextStyle(fontSize: ...)` respectively.
pub(crate) fn is_supported_motion_prop(prop: &str) -> bool {
    matches!(
        prop,
        "opacity"
            | "scale"
            | "translateX"
            | "translateY"
            | "rotate"
            | "backgroundColor"
            | "color"
            | "width"
            | "height"
            | "borderRadius"
            | "fontSize"
    )
}

/// Properties of a motion that Flutter codegen will emit animations for
/// (supported subset, in keyframe order — deterministic for a given IR).
pub(crate) fn flutter_motion_properties(kf: &KeyframeIR) -> Vec<String> {
    extract_keyframe_properties(kf)
        .into_iter()
        .filter(|p| is_supported_motion_prop(p))
        .collect()
}

/// Value kind of an animated semantic property — decides the Dart type of
/// the per-property `Animation<T>` and how stop values are formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MotionPropKind {
    /// Numeric scalar (opacity, scale, translate, rotate, width, height,
    /// fontSize).
    Numeric,
    /// CSS color (backgroundColor, color) — `Color` literal values.
    Color,
    /// Length radius (borderRadius) — `BorderRadius.all(Radius.circular(...))`
    /// literal values animated through the Flutter SDK's `BorderRadiusTween`.
    BorderRadius,
    /// Anything else — formatted as a Dart expression, animated as double.
    Other,
}

pub(crate) fn motion_prop_kind(prop: &str) -> MotionPropKind {
    match prop {
        "backgroundColor" | "color" => MotionPropKind::Color,
        "borderRadius" => MotionPropKind::BorderRadius,
        "opacity" | "scale" | "translateX" | "translateY" | "rotate" | "width" | "height"
        | "fontSize" => MotionPropKind::Numeric,
        _ => MotionPropKind::Other,
    }
}

/// Dart value type of the per-property Animation for a property.
pub(crate) fn prop_anim_type(prop: &str) -> &'static str {
    match motion_prop_kind(prop) {
        MotionPropKind::Color => "Color",
        MotionPropKind::BorderRadius => "BorderRadius?",
        _ => "double",
    }
}

/// Dart field name of the per-property Animation for a motion property.
///
/// `_{motion}{Prop}` (e.g. `_fadeInOpacity`, `_fadeInTranslateX`) — the
/// lowerCamelCase form keeps generated Dart `non_constant_identifier_names`
/// clean.
pub(crate) fn prop_anim_field(motion_name: &str, prop: &str) -> String {
    format!("_{}{}", motion_name, capitalize_first(prop))
}

/// Format an f64 as a Dart double literal: integral values get a `.0`
/// suffix (`300` → `300.0`) so they type-check as `double`.
fn format_double_literal(f: f64) -> String {
    if f.fract() == 0.0 {
        format!("{:.1}", f)
    } else {
        f.to_string()
    }
}

/// Format a keyframe stop property value into a Dart expression.
///
/// - Numeric: number literals → Dart doubles; unit-bearing strings
///   (`"20px"`, `"45deg"`, `"50%"`) → bare numbers; anything else falls
///   back to the generic expression formatter.
/// - Color: CSS color strings (`#ff0000`, `rgb(...)`) → `Color(...)`.
/// - BorderRadius: lengths → `BorderRadius.all(Radius.circular(...))` (the
///   `BorderRadiusTween` value shape); non-literal expressions are wrapped
///   in the circular constructor (a borderRadius stop value is a radius
///   length — mirroring how `rotate` wraps expressions).
pub(crate) fn format_stop_value(prop: &str, value: &ExprIR) -> String {
    match motion_prop_kind(prop) {
        MotionPropKind::BorderRadius => {
            let radius = match value {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(n),
                } => n.as_f64().map(format_double_literal),
                ExprIR::LiteralExpr {
                    value: serde_json::Value::String(s),
                } => {
                    // Unit-bearing string literals ("16px"/"16") strip px —
                    // BorderRadius circular radii are unitless doubles.
                    let bare = s.trim().trim_end_matches("px");
                    bare.parse::<f64>().ok().map(format_double_literal)
                }
                _ => None,
            };
            match radius {
                Some(r) => format!("BorderRadius.all(Radius.circular({r}))"),
                None => format!(
                    "BorderRadius.all(Radius.circular({}))",
                    format_expr_ir_flutter(value)
                ),
            }
        }
        MotionPropKind::Color => match value {
            ExprIR::LiteralExpr {
                value: serde_json::Value::String(s),
            } => css_color_to_dart(s).unwrap_or_else(|| format_expr_ir_flutter(value)),
            _ => format_expr_ir_flutter(value),
        },
        MotionPropKind::Numeric => match value {
            ExprIR::LiteralExpr {
                value: serde_json::Value::Number(n),
            } => n
                .as_f64()
                .map(format_double_literal)
                .unwrap_or_else(|| "0.0".to_string()),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String(s),
            } => {
                // Unit-bearing string literals ("20px"/"45deg"/"50%") strip
                // their unit — Flutter transform/opacity values are unitless.
                let bare = s
                    .trim()
                    .trim_end_matches("px")
                    .trim_end_matches("deg")
                    .trim_end_matches('%');
                match bare.parse::<f64>() {
                    Ok(f) => format_double_literal(f),
                    Err(_) => format_expr_ir_flutter(value),
                }
            }
            _ => format_expr_ir_flutter(value),
        },
        MotionPropKind::Other => format_expr_ir_flutter(value),
    }
}

/// Natural (un-animated) boundary defaults for a property, used when a
/// keyframe omits the 0%/100% stop. WAAPI interpolates to-only/from-only
/// keyframes from/to the element's current (natural) value:
/// opacity/scale → 1.0 (fully visible, unscaled), translate/rotate → 0.0
/// (no offset, no rotation), colors → transparent/white.
fn natural_boundary_defaults(prop: &str) -> (&'static str, &'static str) {
    match motion_prop_kind(prop) {
        MotionPropKind::Color => ("Color(0x00000000)", "Color(0xFFFFFFFF)"),
        MotionPropKind::BorderRadius => ("BorderRadius.zero", "BorderRadius.zero"),
        MotionPropKind::Numeric if matches!(prop, "opacity" | "scale") => ("1.0", "1.0"),
        MotionPropKind::Numeric => ("0.0", "0.0"), // translate*/rotate/width/height/fontSize
        MotionPropKind::Other => ("0.0", "1.0"),
    }
}

/// Tween constructor for a property: `BorderRadiusTween` (Flutter SDK tween
/// over `BorderRadius?`, lerped via `BorderRadius.lerp`) for borderRadius,
/// generic `Tween<T>` for everything else.
fn prop_tween_ctor(prop: &str, type_ann: &str) -> String {
    if prop == "borderRadius" {
        "BorderRadiusTween".to_string()
    } else {
        format!("Tween<{type_ann}>")
    }
}

/// Degrees → radians: the DSL/React declare `rotate` in degrees
/// (`rotate: -30deg` → `rotate(-30deg)`), but `Transform.rotate` takes
/// radians. Literal numbers are precomputed; expressions are wrapped in a
/// multiplication.
fn rotate_deg_to_rad_expr(value: &str) -> String {
    match value.parse::<f64>() {
        Ok(deg) => format_double_literal(deg * DEG_TO_RAD),
        Err(_) => format!("({value}) * {DEG_TO_RAD}"),
    }
}

const DEG_TO_RAD: f64 = 0.017453292519943295;

/// Build the per-property Tween (2 stops) or TweenSequence (3+ stops)
/// expression for a motion property.
///
/// Stops are collected per property and sorted by position; missing 0%/100%
/// boundaries are synthesized from the property's natural values (see
/// [`natural_boundary_defaults`]). Sequence weights are proportional to the
/// position spans (0%→50%→100% ⇒ weights 50/50).
///
/// `reverse` swaps the interpolation direction (begin/end and segment order)
/// — used for `direction: reverse` with a loop, since `AnimationController`
/// can only repeat forward passes.
pub(crate) fn build_prop_tween_expr(kf: &KeyframeIR, prop: &str, reverse: bool) -> String {
    let type_ann = prop_anim_type(prop);

    // Stops defining this property, sorted by position.
    let mut points: Vec<(f64, String)> = kf
        .stops
        .iter()
        .filter(|s| s.properties.contains_key(prop))
        .map(|s| (s.position, format_stop_value(prop, &s.properties[prop])))
        .collect();
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    if points.is_empty() {
        return format!("Tween<{type_ann}>(begin: 0.0, end: 1.0)");
    }

    // Synthesize missing boundaries from the natural property values.
    let (begin_default, end_default) = natural_boundary_defaults(prop);
    if points.first().is_some_and(|p| p.0 != 0.0) {
        points.insert(0, (0.0, begin_default.to_string()));
    }
    if points.last().is_some_and(|p| p.0 != 1.0) {
        points.push((1.0, end_default.to_string()));
    }

    // rotate is declared in degrees — convert to radians for Flutter.
    if prop == "rotate" {
        for point in &mut points {
            point.1 = rotate_deg_to_rad_expr(&point.1);
        }
    }

    if reverse {
        points.reverse();
    }

    let ctor = prop_tween_ctor(prop, type_ann);

    if points.len() == 2 {
        return format!("{ctor}(begin: {}, end: {})", points[0].1, points[1].1);
    }

    // 3+ points → weight-proportional TweenSequence. When reversed the
    // positions descend, so spans use abs(). borderRadius keeps its
    // BorderRadiusTween items (a BorderRadiusTween IS a Tween<BorderRadius?>).
    let mut items: Vec<String> = Vec::new();
    for pair in points.windows(2) {
        let weight = ((pair[1].0 - pair[0].0).abs() * 100.0).round().max(1.0) as u32;
        items.push(format!(
            "TweenSequenceItem(tween: {ctor}(begin: {}, end: {}), weight: {})",
            pair[0].1, pair[1].1, weight
        ));
    }
    format!("TweenSequence<{type_ann}>([{}])", items.join(", "))
}

/// Extract duration in milliseconds from the `over` ExprIR.
///
/// String durations may carry units ("300ms", "1.5s") — delegated to the
/// shared [`parse_duration_ms`] parser so seconds convert to milliseconds.
/// Bare numeric strings ("0.5") follow CSS duration semantics: seconds.
/// Numeric literals below 1.0 are treated as seconds too ("0.5" → 500ms);
/// larger numeric literals are milliseconds (existing convention).
/// Format milliseconds as a Dart `int` literal for `Duration(milliseconds:)`.
/// `Duration(milliseconds:)` takes an `int`, so fractional values round.
fn format_ms_literal(ms: f64) -> String {
    format!("{:.0}", ms.round())
}

pub(crate) fn extract_duration_ms(over: &ExprIR) -> String {
    match over {
        ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::Number(n) => match n.as_f64() {
                Some(ms) if ms < 1.0 => format_ms_literal(ms * 1000.0),
                Some(ms) => format_ms_literal(ms),
                None => "300".to_string(),
            },
            serde_json::Value::String(s) => {
                match omnilang_core::semantic::parse_duration_ms(s) {
                    Ok(ms) => format_ms_literal(ms),
                    Err(_) => match s.trim().parse::<f64>() {
                        // Bare numbers are seconds (CSS duration semantics):
                        // "0.5" → 500ms.
                        Ok(secs) => format_ms_literal(secs * 1000.0),
                        Err(_) => "300".to_string(),
                    },
                }
            }
            _ => "300".to_string(),
        },
        _ => "300".to_string(),
    }
}

/// Map an OmniView easing to a Flutter Curves expression string.
pub(crate) fn format_easing_flutter(ease: &Option<EasingIR>) -> String {
    match ease {
        Some(EasingIR::Named(name)) => format_named_curve(name),
        Some(EasingIR::CubicBezier { x1, y1, x2, y2 }) => {
            format!("Cubic({}, {}, {}, {})", x1, y1, x2, y2)
        }
        Some(EasingIR::Back { direction }) => match direction.as_deref() {
            Some("in") => "Curves.easeInBack".to_string(),
            Some("inout") => "Curves.easeInOutBack".to_string(),
            _ => "Curves.easeOutBack".to_string(),
        },
        Some(EasingIR::Elastic { direction }) => match direction.as_deref() {
            Some("in") => "Curves.easeInElastic".to_string(),
            Some("inout") => "Curves.easeInOutElastic".to_string(),
            _ => "Curves.easeOutElastic".to_string(),
        },
        None => "Curves.linear".to_string(),
        // #[non_exhaustive] — future easing kinds fall back to linear.
        _ => "Curves.linear".to_string(),
    }
}

/// Map a named easing string to a Flutter Curves constant.
///
/// Delegates to the shared [`NAMED_EASINGS`] table.
/// Falls back to `Curves.linear` for unknown names.
pub(crate) fn format_named_curve(name: &str) -> String {
    omnilang_core::codegen::shared::easing::resolve_easing_flutter(name)
        .unwrap_or("Curves.linear")
        .to_string()
}

/// Returns a diagnostic for easings that cannot be mapped onto Flutter.
///
/// `steps(...)` easings (and the `stepStart`/`stepEnd` aliases, which lower
/// to `steps(1,...)`) are discrete — Flutter has no `Curves` equivalent, so
/// the generated code falls back to `Curves.linear` and the user is told.
pub(crate) fn easing_flutter_diagnostic(
    motion_name: &str,
    ease: &Option<EasingIR>,
) -> Option<String> {
    let name = match ease {
        Some(EasingIR::Named(n)) => n,
        _ => return None,
    };
    let css = omnilang_core::codegen::shared::easing::resolve_easing(name);
    if name.starts_with("steps(") || css.is_some_and(|c| c.starts_with("steps(")) {
        return Some(format!(
            "[flutter] motion '{motion_name}' uses steps() easing, which has no Flutter \
             Curves equivalent; falling back to Curves.linear"
        ));
    }
    None
}

/// Returns a diagnostic for a keyframe property that Flutter cannot animate
/// faithfully, or `None` for properties emitted normally.
///
/// - `width`/`height`: layout-dependent size animation — emitted via
///   `SizedBox` but flagged for prefer-transform.
/// - `skewX`/`skewY`: no stable Flutter transform constructor (the Flutter
///   SDK has no `Transform.skew`) — skipped.
/// - anything else outside the supported set: skipped.
///
/// `borderRadius`/`fontSize` (Task 5.2) are first-class: `BorderRadiusTween`
/// is a Flutter SDK class and fontSize applies through `TextStyle`, so no
/// diagnostic is emitted for them.
pub(crate) fn prop_flutter_diagnostic(motion_name: &str, prop: &str) -> Option<String> {
    match prop {
        "width" | "height" => Some(format!(
            "[flutter] motion '{motion_name}' animates '{prop}', which is layout-dependent \
             in Flutter; animating via SizedBox — prefer scale/transform for performant \
             size animation"
        )),
        "skewX" | "skewY" => Some(format!(
            "[flutter] motion '{motion_name}' animates '{prop}', which is rarely used and \
             has no Flutter transform constructor; skipping it"
        )),
        p if is_supported_motion_prop(p) => None,
        p => Some(format!(
            "[flutter] motion '{motion_name}' animates unsupported property '{p}'; skipping it"
        )),
    }
}

/// The AnimatedBuilder closure's `child` parameter is `Widget?`, but
/// `DefaultTextStyle.child` is a required non-nullable `Widget` — coalesce
/// the innermost text-style child so the generated code type-checks.
fn text_style_child_arg(body: &str) -> &str {
    if body == "child" {
        "child ?? const SizedBox.shrink()"
    } else {
        body
    }
}

/// Build the AnimatedBuilder `builder` body expression for a motion.
/// Inspects keyframe properties to determine what widget wrappers to use,
/// nesting per-property wrappers (Opacity, Transform.scale, Transform.translate,
/// Transform.rotate, Container, DefaultTextStyle, SizedBox; borderRadius and
/// fontSize join the Container/DefaultTextStyle families — Task 5.2).
///
/// The wrappers are gated on the motion's `_{motion}Active` flag: while
/// inactive the element keeps its natural style. This is what lets
/// `fill: none`/`backwards` return to the natural appearance on completion
/// (instead of holding the 0% keyframe), and it keeps trigger animations
/// (e.g. to-only hover scales) from being visible at their synthesized
/// begin value before the first trigger fires.
pub(crate) fn build_animated_builder_body(motion: &MotionIntent, motion_name: &str) -> String {
    let active_name = format!("_{}Active", motion_name);
    let kf = &motion.body;
    let properties = extract_keyframe_properties(kf);
    if properties.is_empty() {
        // No properties to wrap — still return a Widget (the builder closure's
        // `child` parameter is `Widget?`, so a bare `child` would not satisfy
        // the `Widget` return type).
        return "child ?? const SizedBox.shrink()".to_string();
    }
    let has = |p: &str| properties.iter().any(|q| q == p);

    let mut body = "child".to_string();

    // Opacity
    if has("opacity") {
        body = format!(
            "Opacity(opacity: {}.value, child: {})",
            prop_anim_field(motion_name, "opacity"),
            body
        );
    }

    // Scale
    if has("scale") {
        body = format!(
            "Transform.scale(scale: {}.value, child: {})",
            prop_anim_field(motion_name, "scale"),
            body
        );
    }

    // Translate
    if has("translateX") || has("translateY") {
        let x = if has("translateX") {
            format!("{}.value", prop_anim_field(motion_name, "translateX"))
        } else {
            "0.0".to_string()
        };
        let y = if has("translateY") {
            format!("{}.value", prop_anim_field(motion_name, "translateY"))
        } else {
            "0.0".to_string()
        };
        body = format!(
            "Transform.translate(offset: Offset({}, {}), child: {})",
            x, y, body
        );
    }

    // Rotate
    if has("rotate") {
        body = format!(
            "Transform.rotate(angle: {}.value, child: {})",
            prop_anim_field(motion_name, "rotate"),
            body
        );
    }

    // backgroundColor → Container
    if has("backgroundColor") {
        body = format!(
            "Container(color: {}.value, child: {})",
            prop_anim_field(motion_name, "backgroundColor"),
            body
        );
    }

    // borderRadius → Container with BoxDecoration (corner radius of a
    // rounded surface — BorderRadiusTween interpolation)
    if has("borderRadius") {
        body = format!(
            "Container(decoration: BoxDecoration(borderRadius: {}.value), child: {})",
            prop_anim_field(motion_name, "borderRadius"),
            body
        );
    }

    // color → DefaultTextStyle (text color; rebuilt every frame by the
    // AnimatedBuilder, so no AnimatedDefaultTextStyle controller needed).
    // DefaultTextStyle.child is a required non-nullable Widget, unlike the
    // builder closure's nullable `child` param — coalesce when the text-style
    // wrapper sits at the innermost nesting level.
    if has("color") {
        let child_arg = text_style_child_arg(&body);
        body = format!(
            "DefaultTextStyle(style: TextStyle(color: {}.value), child: {child_arg})",
            prop_anim_field(motion_name, "color"),
        );
    }

    // fontSize → DefaultTextStyle (text size; the per-property Tween<double>
    // is the TextStyle.lerp equivalent for the size axis — same wrapper as
    // the color property)
    if has("fontSize") {
        let child_arg = text_style_child_arg(&body);
        body = format!(
            "DefaultTextStyle(style: TextStyle(fontSize: {}.value), child: {child_arg})",
            prop_anim_field(motion_name, "fontSize"),
        );
    }

    // width/height → SizedBox (layout-dependent; warned in initState)
    if has("width") {
        body = format!(
            "SizedBox(width: {}.value, child: {})",
            prop_anim_field(motion_name, "width"),
            body
        );
    }
    if has("height") {
        body = format!(
            "SizedBox(height: {}.value, child: {})",
            prop_anim_field(motion_name, "height"),
            body
        );
    }

    // No supported property wrapped (e.g. only skew/unknown props) — return a
    // Widget, not the nullable `child` parameter. Nothing to gate either.
    if body == "child" {
        return "child ?? const SizedBox.shrink()".to_string();
    }

    // Gate the animated wrappers on the active flag. `??` binds tighter than
    // `?:`, so this parses as `_motionActive ? <wrappers> : (child ?? ...)`.
    format!("{active_name} ? {body} : child ?? const SizedBox.shrink()")
}

/// Statements that start playback for a motion's @enter trigger, honoring
/// `loop_count` / `direction` / `fill` / `delay` with WAAPI semantics:
///
/// - single pass (no loop): `forward()`; `direction: reverse` or
///   `alternate-reverse` → `reverse()` (WAAPI: with iterations=1 the first
///   pass direction decides — `alternate` plays a single forward pass)
/// - `loop_count: N`/`infinite` → `repeat(count: N)` / `repeat()`. The
///   per-property tweens are reversed for `direction: reverse` (Flutter can
///   only repeat forward passes); `alternate` → `repeat(reverse: true)`
///   (F,R,F,R); `alternate-reverse` → preset the controller to 1.0 first
///   (R,F,R,F)
/// - `fill: none|backwards` → deactivate the animated wrapper on completion
///   so the element returns to its natural style — not the 0% keyframe
///   (`ctrl.value = 0` would leave a fade-in permanently invisible)
/// - `delay` → `Future.delayed(...)` with a `mounted` guard around the start
///
/// Returns one or more Dart statements; the caller emits each on its own line.
pub(crate) fn emit_playback_start(
    ctrl_name: &str,
    active_name: Option<&str>,
    playback: &PlaybackIR,
) -> Vec<String> {
    let delay = playback.delay.as_ref();
    // Activation only applies when the motion has wrappers to gate; motions
    // with no supported properties play without a flag.
    let activate = active_name.map(|name| format!("{name} = true;"));

    // ── Looped playback: fill is meaningless (repeat never completes) ──
    if let Some(loop_val) = &playback.loop_count {
        let mut args: Vec<String> = Vec::new();
        if let LoopValue::Count(n) = loop_val {
            args.push(format!("count: {n}"));
        }
        let (repeat_reverse, needs_preset) = match playback.direction {
            Some(DirectionValue::Alternate) => (true, false),
            Some(DirectionValue::AlternateReverse) => (true, true),
            _ => (false, false),
        };
        if repeat_reverse {
            args.push("reverse: true".to_string());
        }
        let repeat_stmt = if args.is_empty() {
            format!("{ctrl_name}.repeat()")
        } else {
            format!("{ctrl_name}.repeat({})", args.join(", "))
        };
        let mut inner: Vec<String> = Vec::new();
        if let Some(activate) = &activate {
            inner.push(activate.clone());
        }
        if needs_preset {
            inner.push(format!("{ctrl_name}.value = 1.0;"));
        }
        inner.push(format!("{repeat_stmt};"));
        return wrap_delayed(delay, inner);
    }

    // ── Single pass ──
    let play_stmt = match playback.direction {
        Some(DirectionValue::Reverse) | Some(DirectionValue::AlternateReverse) => {
            format!("{ctrl_name}.reverse()")
        }
        _ => format!("{ctrl_name}.forward()"),
    };
    // Deactivation guard: only fire when the pass actually completed
    // (forward completes at 1.0, reverse at 0.0) — an interrupted pass means
    // the motion is being re-triggered and must stay active.
    let completion_value = if play_stmt.contains(".reverse()") {
        "0.0"
    } else {
        "1.0"
    };
    let reset_stmt = match active_name {
        Some(active_name) => format!(
            "{play_stmt}.whenComplete(() {{ if (mounted && {ctrl_name}.value == {completion_value}) \
             setState(() {{ {active_name} = false; }}); }})"
        ),
        None => play_stmt.clone(),
    };

    // fill: none|backwards → remove the animated wrapper on completion
    // (natural style); backwards|both → show the begin state during a delay.
    let reset_on_complete = matches!(
        playback.fill,
        Some(FillValue::None) | Some(FillValue::Backwards)
    );
    let activate_immediately = matches!(
        playback.fill,
        Some(FillValue::Backwards) | Some(FillValue::Both)
    );

    let play_core = if reset_on_complete {
        reset_stmt
    } else {
        play_stmt
    };
    let activate_at_start = match &activate {
        Some(activate) => format!("{activate} {play_core};"),
        None => format!("{play_core};"),
    };
    match delay {
        Some(delay) => {
            let ms = extract_duration_ms(delay);
            if activate_immediately && activate.is_some() {
                // Begin state during the delay (fill: backwards/both)
                vec![
                    activate.unwrap_or_default(),
                    format!(
                        "Future.delayed(Duration(milliseconds: {ms}), () {{ if (mounted) {{ {play_core}; }} }});"
                    ),
                ]
            } else {
                // Natural style during the delay; activation at play start
                vec![format!(
                    "Future.delayed(Duration(milliseconds: {ms}), () {{ if (mounted) {{ {activate_at_start} }} }});"
                )]
            }
        }
        None => {
            if activate_immediately && activate.is_some() {
                vec![activate.unwrap_or_default(), format!("{play_core};")]
            } else {
                vec![activate_at_start]
            }
        }
    }
}

/// Wrap playback statements in a delayed callback when `delay` is present.
/// The `mounted` guard prevents starting playback on a disposed State.
fn wrap_delayed(delay: Option<&ExprIR>, inner: Vec<String>) -> Vec<String> {
    match delay {
        Some(delay) => {
            let ms = extract_duration_ms(delay);
            vec![format!(
                "Future.delayed(Duration(milliseconds: {ms}), () {{ if (mounted) {{ {} }} }});",
                inner.join(" ")
            )]
        }
        None => inner,
    }
}

/// Wrap a widget expression with motion-triggered widgets:
/// AnimatedBuilder, MouseRegion, GestureDetector as needed.
pub(crate) fn wrap_with_motion_widgets(
    inner: String,
    triggers: &[TriggerIntent],
    motions: &[MotionIntent],
) -> String {
    if triggers.is_empty() {
        return inner;
    }

    let mut result = inner;

    // Check for hover triggers
    let has_hover = triggers
        .iter()
        .any(|t| matches!(t.kind, TriggerKind::Hover));
    let has_press = triggers
        .iter()
        .any(|t| matches!(t.kind, TriggerKind::Press));

    // Collect unique motion references
    let motion_refs: Vec<String> = {
        let mut seen = FxHashSet::default();
        triggers
            .iter()
            .filter_map(|t| t.motion_ref.clone())
            .filter(|n| seen.insert(n.clone()))
            .collect()
    };

    // For each motion ref, wrap with AnimatedBuilder. The builder listens to
    // the shared controller (the per-property Animation<T> fields are all
    // driven by it), and the body reads each property's animation value.
    for motion_ref in &motion_refs {
        if let Some(motion) = motions.iter().find(|m| m.name == *motion_ref) {
            let ctrl_name = format!("_{}Controller", motion_ref);
            let builder_body = build_animated_builder_body(motion, motion_ref);
            result = format!(
                "AnimatedBuilder(\n  animation: {},\n  builder: (context, child) => {},\n  child: {},\n)",
                ctrl_name, builder_body, result
            );
        }
    }

    // Wrap with MouseRegion for hover/unhover. Entering activates the motion
    // (its wrappers are gated on the active flag); exiting reverses and
    // deactivates once the controller reaches 0.0 — the `mounted` + value
    // guard skips the reset when the pass was interrupted by a re-enter.
    if has_hover {
        let hover_motion = triggers
            .iter()
            .find(|t| matches!(t.kind, TriggerKind::Hover))
            .and_then(|t| t.motion_ref.as_ref());
        let hover_ctrl = hover_motion
            .map(|name| format!("_{}Controller", name))
            .unwrap_or_else(|| "_controller".to_string());
        let hover_active = hover_motion
            .map(|name| format!("_{}Active", name))
            .unwrap_or_else(|| "_active".to_string());

        result = format!(
            "MouseRegion(\n  onEnter: (_) {{ {active} = true; {ctrl}.forward(); }},\n  onExit: (_) {{ {ctrl}.reverse().whenComplete(() {{ if (mounted && {ctrl}.value == 0.0) setState(() {{ {active} = false; }}); }}); }},\n  child: {result},\n)",
            active = hover_active, ctrl = hover_ctrl, result = result
        );
    }

    // Wrap with GestureDetector for press — same activation/reset contract.
    if has_press {
        let press_motion = triggers
            .iter()
            .find(|t| matches!(t.kind, TriggerKind::Press))
            .and_then(|t| t.motion_ref.as_ref());
        let press_ctrl = press_motion
            .map(|name| format!("_{}Controller", name))
            .unwrap_or_else(|| "_controller".to_string());
        let press_active = press_motion
            .map(|name| format!("_{}Active", name))
            .unwrap_or_else(|| "_active".to_string());

        result = format!(
            "GestureDetector(\n  onTapDown: (_) {{ {active} = true; {ctrl}.forward(); }},\n  onTapUp: (_) {{ {ctrl}.reverse().whenComplete(() {{ if (mounted && {ctrl}.value == 0.0) setState(() {{ {active} = false; }}); }}); }},\n  child: {result},\n)",
            active = press_active, ctrl = press_ctrl, result = result
        );
    }

    result
}

// wrap_flutter_states removed — StatesIR subsystem removed per ADR-032

/// Traverse the view tree to find motion names used with @enter triggers.
pub(crate) fn find_enter_motions(node: &ViewTree) -> Vec<String> {
    let mut result = Vec::new();
    find_enter_inner(node, &mut result);
    result
}

fn find_enter_inner(node: &ViewTree, result: &mut Vec<String>) {
    match node {
        ViewTree::Element(el) => {
            for trigger in el
                .motion_behavior()
                .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
            {
                if matches!(trigger.kind, TriggerKind::Enter) {
                    if let Some(ref name) = trigger.motion_ref {
                        if !result.contains(name) {
                            result.push(name.clone());
                        }
                    }
                }
            }
            for child in &el.children {
                find_enter_inner(child, result);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    find_enter_inner(body, result);
                }
            }
            if let Some(ref fb) = cond.fallback {
                find_enter_inner(fb, result);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                find_enter_inner(body, result);
            }
            if let Some(ref empty) = list.empty_body {
                find_enter_inner(empty, result);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                find_enter_inner(child, result);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            // Enter-triggered motions inside slot fallbacks play when the
            // fallback renders — their controllers must be created in
            // initState too.
            for child in fallback {
                find_enter_inner(child, result);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}
