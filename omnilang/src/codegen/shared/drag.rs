//! Shared drag-extent derivation for @drag-horizontal / @drag-vertical.
//!
//! ADR-053 removed the `@drag-constraints` attribute (the parser has no token
//! for it), so drag triggers are effectively always unconstrained. Without an
//! extent the old progress math ran `Math.abs(delta) / Infinity` → 0 and the
//! element never moved (deferral #19).
//!
//! The fix: when no constraints are given, map the gesture progress onto the
//! motion's OWN keyframe range in the drag axis — dragging right by the
//! distance the animation travels advances it 0 → 1, so the element follows
//! its animation.

use crate::ir::*;

/// The transform property a drag axis maps to.
pub fn axis_translate_prop(is_horizontal: bool) -> &'static str {
    if is_horizontal {
        "translateX"
    } else {
        "translateY"
    }
}

/// Extract a literal number from an ExprIR — including unary negation
/// (`-300` lowers to `UnaryOpExpr(Neg, 300)`).
fn literal_number(value: &ExprIR) -> Option<f64> {
    match value {
        ExprIR::LiteralExpr { value: v } if v.is_number() => v.as_f64(),
        ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Neg,
            operand,
        } => literal_number(operand).map(|n| -n),
        _ => None,
    }
}

/// Positive range of a list of literal keyframe values.
///
/// - Multiple values: `max - min` (so `300 → 0` still yields 300 — dragging
///   right always plays the animation forward).
/// - A single value (from-only / to-only motion): `|value|`, matching the
///   synthetic counterpart keyframe WAAPI emission adds at offset 0/1.
/// - Zero or degenerate ranges: `None`.
fn extent_of(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let extent = max - min;
    if extent > 0.0 && extent.is_finite() {
        return Some(extent);
    }
    if values.len() == 1 {
        let single = values[0].abs();
        if single > 0.0 && single.is_finite() {
            return Some(single);
        }
    }
    None
}

/// Derive the drag extent (px) for a motion body on the given axis:
///
/// 1. The axis-appropriate translate range (translateX for horizontal drag,
///    translateY for vertical drag) — the element follows its own animation.
/// 2. Fallback: any translate property's range (cross-axis), so a motion
///    without an axis-appropriate keyframe still maps onto real movement.
/// 3. `None` — the motion has no usable translate range; the caller falls
///    back to the previous unbounded (inert) behavior with a warning.
pub fn drag_extent(body: &KeyframeIR, is_horizontal: bool) -> Option<f64> {
    let axis_prop = axis_translate_prop(is_horizontal);
    let mut axis_vals: Vec<f64> = Vec::new();
    let mut any_vals: Vec<f64> = Vec::new();
    for stop in &body.stops {
        for (prop, expr) in &stop.properties {
            if !matches!(prop.as_str(), "translateX" | "translateY" | "translateZ") {
                continue;
            }
            if let Some(n) = literal_number(expr) {
                any_vals.push(n);
                if prop == axis_prop {
                    axis_vals.push(n);
                }
            }
        }
    }
    extent_of(&axis_vals).or_else(|| extent_of(&any_vals))
}

/// JS number literal for an extent (300.0 → `300`, 150.5 → `150.5`).
pub fn format_extent_js(extent: f64) -> String {
    extent.to_string()
}

/// Dart double literal for an extent (300.0 → `300.0`, 150.5 → `150.5`).
/// Only referenced by the Flutter backend (feature `codegen-flutter`).
#[cfg_attr(not(feature = "codegen-flutter"), allow(dead_code))]
pub fn format_extent_dart(extent: f64) -> String {
    if extent.fract() == 0.0 {
        format!("{:.0}.0", extent)
    } else {
        extent.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    fn stop(position: f64, props: &[(&str, f64)]) -> KeyframeStopIR {
        KeyframeStopIR {
            position,
            properties: FxHashMap::from_iter(
                props
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), ExprIR::LiteralExpr { value: (*v).into() })),
            ),
            ease: None,
            source_span: None,
        }
    }

    fn body(stops: Vec<KeyframeStopIR>) -> KeyframeIR {
        KeyframeIR {
            stops,
            over: ExprIR::LiteralExpr { value: 300.into() },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        }
    }

    #[test]
    fn horizontal_drag_uses_translate_x_range() {
        let b = body(vec![
            stop(0.0, &[("translateX", 0.0)]),
            stop(1.0, &[("translateX", 300.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), Some(300.0));
    }

    #[test]
    fn vertical_drag_uses_translate_y_range() {
        let b = body(vec![
            stop(0.0, &[("translateY", 10.0)]),
            stop(1.0, &[("translateY", 110.0)]),
        ]);
        assert_eq!(drag_extent(&b, false), Some(100.0));
    }

    #[test]
    fn axis_appropriate_range_wins_over_cross_axis() {
        // Both translate props present: each drag axis uses its own range.
        let b = body(vec![
            stop(0.0, &[("translateX", 0.0), ("translateY", 0.0)]),
            stop(1.0, &[("translateX", 300.0), ("translateY", 200.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), Some(300.0));
        assert_eq!(drag_extent(&b, false), Some(200.0));
    }

    #[test]
    fn negative_ranges_yield_positive_extent() {
        let b = body(vec![
            stop(0.0, &[("translateX", 300.0)]),
            stop(1.0, &[("translateX", 0.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), Some(300.0));
    }

    #[test]
    fn negative_literal_values_are_parsed() {
        let b = body(vec![
            stop(0.0, &[("translateX", -300.0)]),
            stop(1.0, &[("translateX", 0.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), Some(300.0));
    }

    #[test]
    fn cross_axis_fallback_uses_any_translate() {
        // Horizontal drag over a translateY-only motion still maps onto the
        // animation's real movement via the cross-axis fallback.
        let b = body(vec![
            stop(0.0, &[("translateY", 0.0)]),
            stop(1.0, &[("translateY", 100.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), Some(100.0));
    }

    #[test]
    fn single_stop_motion_uses_absolute_value() {
        // to-only: synthetic 0% keyframe implies the range 0 → |value|.
        let b = body(vec![stop(1.0, &[("translateX", 250.0)])]);
        assert_eq!(drag_extent(&b, true), Some(250.0));
    }

    #[test]
    fn no_translate_keyframes_yield_none() {
        let b = body(vec![
            stop(0.0, &[("opacity", 0.0)]),
            stop(1.0, &[("opacity", 1.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), None);
        assert_eq!(drag_extent(&b, false), None);
    }

    #[test]
    fn zero_range_yields_none() {
        let b = body(vec![
            stop(0.0, &[("translateX", 0.0)]),
            stop(1.0, &[("translateX", 0.0)]),
        ]);
        assert_eq!(drag_extent(&b, true), None);
    }

    #[test]
    fn format_helpers() {
        assert_eq!(format_extent_js(300.0), "300");
        assert_eq!(format_extent_js(150.5), "150.5");
        assert_eq!(format_extent_dart(300.0), "300.0");
        assert_eq!(format_extent_dart(150.5), "150.5");
    }
}
