//! Motion / Animation codegen helpers for Flutter codegen.

use crate::ir::*;

use rustc_hash::FxHashSet;

// ═══════════════════════════════════════════════════════════════════
// Motion / Animation codegen helpers
// ═══════════════════════════════════════════════════════════════════

/// Check if any motion in the component uses path animation (needs dart:ui import).
pub(crate) fn comp_has_path_motion(ir: &IRProgram) -> bool {
    ir.modules
        .values()
        .flat_map(|m| &m.motions)
        .any(motion_has_path)
}

/// Check if a single motion uses path animation.
pub(crate) fn motion_has_path(motion: &MotionIntent) -> bool {
    let MotionBodyIR::Keyframe(kf) = &motion.body;
    kf.path.is_some()
}

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

/// Extract the begin (from) tween value from keyframe stops.
pub(crate) fn extract_tween_begin(kf: &KeyframeIR) -> String {
    for stop in &kf.stops {
        if stop.position == 0.0 {
            for value in stop.properties.values() {
                if let ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(n),
                } = value
                {
                    return n.to_string();
                }
            }
        }
    }
    // Default: for opacity/translate, start from 0.0; for scale, 0.0 is fine
    "0.0".to_string()
}

/// Extract the end (to) tween value from keyframe stops.
pub(crate) fn extract_tween_end(kf: &KeyframeIR) -> String {
    for stop in &kf.stops {
        if stop.position == 1.0 {
            for value in stop.properties.values() {
                if let ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(n),
                } = value
                {
                    return n.to_string();
                }
            }
        }
    }
    "1.0".to_string()
}

/// Extract duration in milliseconds from the `over` ExprIR.
pub(crate) fn extract_duration_ms(over: &ExprIR) -> String {
    match over {
        ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(|f| f.to_string())
                .unwrap_or_else(|| "300".to_string()),
            serde_json::Value::String(s) => s.trim_end_matches("ms").to_string(),
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
        Some(EasingIR::Back { .. }) => "Curves.easeOutBack".to_string(),
        Some(EasingIR::Elastic { .. }) => "Curves.easeOutElastic".to_string(),
        None => "Curves.linear".to_string(),
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

/// Build the AnimatedBuilder `builder` body expression for a motion.
/// Inspects keyframe properties to determine what widget wrappers to use
/// (Opacity, Transform.scale, Transform.translate, Transform.rotate).
pub(crate) fn build_animated_builder_body(motion: &MotionIntent, anim_name: &str) -> String {
    let MotionBodyIR::Keyframe(kf) = &motion.body;
    let properties = extract_keyframe_properties(kf);
        if properties.is_empty() {
            return format!("Opacity(opacity: {}.value, child: child)", anim_name);
        }

        let mut body = "child".to_string();

        // Opacity
        if properties.iter().any(|p| p == "opacity") {
            body = format!("Opacity(opacity: {}.value, child: {})", anim_name, body);
        }

        // Scale
        if properties.iter().any(|p| p == "scale") {
            body = format!(
                "Transform.scale(scale: {}.value, child: {})",
                anim_name, body
            );
        }

        // Translate
        if properties
            .iter()
            .any(|p| p == "translateX" || p == "translateY")
        {
            let x = if properties.iter().any(|p| p == "translateX") {
                format!("{}.value", anim_name)
            } else {
                "0.0".to_string()
            };
            let y = if properties.iter().any(|p| p == "translateY") {
                format!("{}.value", anim_name)
            } else {
                "0.0".to_string()
            };
            body = format!(
                "Transform.translate(offset: Offset({}, {}), child: {})",
                x, y, body
            );
        }

        // Rotate
        if properties.iter().any(|p| p == "rotate") {
            body = format!(
                "Transform.rotate(angle: {}.value, child: {})",
                anim_name, body
            );
        }

        body
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

    // For each motion ref, wrap with AnimatedBuilder
    for motion_ref in &motion_refs {
        if let Some(motion) = motions.iter().find(|m| m.name == *motion_ref) {
            let anim_name = format!("_{}Animation", motion_ref);
            let builder_body = build_animated_builder_body(motion, &anim_name);
            result = format!(
                "AnimatedBuilder(\n  animation: {},\n  builder: (context, child) => {},\n  child: {},\n)",
                anim_name, builder_body, result
            );
        }
    }

    // Wrap with MouseRegion for hover/unhover
    if has_hover {
        let hover_ctrl = triggers
            .iter()
            .find(|t| matches!(t.kind, TriggerKind::Hover))
            .and_then(|t| t.motion_ref.as_ref())
            .map(|name| format!("_{}Controller", name))
            .unwrap_or_else(|| "_controller".to_string());

        result = format!(
            "MouseRegion(\n  onEnter: (_) => {}.forward(),\n  onExit: (_) => {}.reverse(),\n  child: {},\n)",
            hover_ctrl, hover_ctrl, result
        );
    }

    // Wrap with GestureDetector for press
    if has_press {
        let press_ctrl = triggers
            .iter()
            .find(|t| matches!(t.kind, TriggerKind::Press))
            .and_then(|t| t.motion_ref.as_ref())
            .map(|name| format!("_{}Controller", name))
            .unwrap_or_else(|| "_controller".to_string());

        result = format!(
            "GestureDetector(\n  onTapDown: (_) => {}.forward(),\n  onTapUp: (_) => {}.reverse(),\n  child: {},\n)",
            press_ctrl, press_ctrl, result
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
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => {}
    }
}
