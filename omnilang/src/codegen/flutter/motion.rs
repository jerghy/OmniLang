//! Motion / Animation codegen helpers for Flutter codegen.

use crate::ir::*;

use super::expr::format_expr_ir_flutter;
use rustc_hash::FxHashSet;

// ═══════════════════════════════════════════════════════════════════
// Motion / Animation codegen helpers
// ═══════════════════════════════════════════════════════════════════

/// Check if any motion in the component uses spring easing (needs physics import).
pub(crate) fn comp_has_spring_motion(ir: &IRProgram) -> bool {
    ir.modules.values().flat_map(|m| &m.motions).any(|m| {
        if let MotionBodyIR::Keyframe(kf) = &m.body {
            matches!(&kf.ease, Some(EasingIR::Spring { .. }))
        } else {
            false
        }
    })
}

/// Check if any motion in the component uses path animation (needs dart:ui import).
pub(crate) fn comp_has_path_motion(ir: &IRProgram) -> bool {
    ir.modules
        .values()
        .flat_map(|m| &m.motions)
        .any(motion_has_path)
}

/// Check if a single motion uses path animation.
pub(crate) fn motion_has_path(motion: &MotionIntent) -> bool {
    match &motion.body {
        MotionBodyIR::Keyframe(kf) => kf.path.is_some(),
        _ => false,
    }
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
        Some(EasingIR::Spring { .. }) => "Curves.linear".to_string(),
        Some(EasingIR::Back { .. }) => "Curves.easeOutBack".to_string(),
        Some(EasingIR::Elastic { .. }) => "Curves.easeOutElastic".to_string(),
        Some(EasingIR::Steps { .. }) => "Curves.linear".to_string(),
        None => "Curves.linear".to_string(),
    }
}

/// Map a named easing string to a Flutter Curves constant.
pub(crate) fn format_named_curve(name: &str) -> String {
    match name {
        "linear" => "Curves.linear",
        "ease" => "Curves.ease",
        "easeIn" | "ease_in" => "Curves.easeIn",
        "easeOut" | "ease_out" => "Curves.easeOut",
        "easeInOut" | "ease_in_out" => "Curves.easeInOut",
        "power1In" | "easeInQuad" | "quadIn" => "Curves.easeInQuad",
        "power1Out" | "easeOutQuad" | "quadOut" => "Curves.easeOutQuad",
        "power2In" | "easeInCubic" | "cubicIn" => "Curves.easeInCubic",
        "power2Out" | "easeOutCubic" | "cubicOut" => "Curves.easeOutCubic",
        "power3In" | "easeInQuart" | "quartIn" => "Curves.easeInQuart",
        "power3Out" | "easeOutQuart" | "quartOut" => "Curves.easeOutQuart",
        "power4In" | "easeInQuint" | "quintIn" => "Curves.easeInQuint",
        "power4Out" | "easeOutQuint" | "quintOut" => "Curves.easeOutQuint",
        "easeInSine" | "sineIn" => "Curves.easeInSine",
        "easeOutSine" | "sineOut" => "Curves.easeOutSine",
        "easeInOutSine" => "Curves.easeInOutSine",
        "easeInExpo" | "expoIn" => "Curves.easeInExpo",
        "easeOutExpo" | "expoOut" => "Curves.easeOutExpo",
        "easeInOutExpo" => "Curves.easeInOutExpo",
        "easeInCirc" | "circIn" => "Curves.easeInCirc",
        "easeOutCirc" | "circOut" => "Curves.easeOutCirc",
        "easeInOutCirc" => "Curves.easeInOutCirc",
        "easeInBack" | "backIn" => "Curves.easeInBack",
        "easeOutBack" | "backOut" => "Curves.easeOutBack",
        "easeInOutBack" => "Curves.easeInOutBack",
        "easeInElastic" | "elasticIn" => "Curves.easeInElastic",
        "easeOutElastic" | "elasticOut" => "Curves.easeOutElastic",
        "easeInOutElastic" => "Curves.easeInOutElastic",
        "bounceIn" => "Curves.bounceIn",
        "bounceOut" => "Curves.bounceOut",
        "bounceInOut" => "Curves.bounceInOut",
        _ => "Curves.linear",
    }
    .to_string()
}

/// Build the AnimatedBuilder `builder` body expression for a motion.
/// Inspects keyframe properties to determine what widget wrappers to use
/// (Opacity, Transform.scale, Transform.translate, Transform.rotate).
pub(crate) fn build_animated_builder_body(motion: &MotionIntent, anim_name: &str) -> String {
    if let MotionBodyIR::Keyframe(kf) = &motion.body {
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
    } else {
        format!("Opacity(opacity: {}.value, child: child)", anim_name)
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

    // Check for hover/unhover triggers
    let has_hover = triggers
        .iter()
        .any(|t| matches!(t.kind, TriggerKind::Hover | TriggerKind::Unhover));
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
            .find(|t| matches!(t.kind, TriggerKind::Hover | TriggerKind::Unhover))
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

/// Wrap a widget with states: support — either reserved (idle/hover/pressed) with
/// MouseRegion+GestureDetector, or custom signal-driven with named state animations.
pub(crate) fn wrap_flutter_states(inner: String, states_ir: &StatesIR, _el: &ElementIR) -> String {
    let mut result = inner;

    // Check if states are reserved (idle, hover, pressed) or custom
    let reserved_names: FxHashSet<&str> = ["idle", "hover", "pressed"].iter().copied().collect();
    let state_names: Vec<&str> = states_ir.states.keys().map(|s| s.as_str()).collect();
    let is_reserved = state_names.iter().all(|n| reserved_names.contains(n));

    if is_reserved {
        // Generate MouseRegion for hover state + GestureDetector for pressed state
        let has_hover = state_names.contains(&"hover");
        let has_pressed = state_names.contains(&"pressed");

        if has_pressed {
            // Get the hover/pressed animation controllers
            if let Some(idle_def) = states_ir.states.get("idle") {
                let over_ms = idle_def
                    .over
                    .as_ref()
                    .and_then(|e| {
                        if let ExprIR::LiteralExpr { value } = e {
                            value.as_str().and_then(parse_dart_duration_ms)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(150.0);
                result = format!(
                    "GestureDetector(\n  onTapDown: (_) => _statesController.animateTo(1.0, duration: Duration(milliseconds: {})),\n  onTapUp: (_) => _statesController.animateTo(0.0, duration: Duration(milliseconds: {})),\n  onTapCancel: (_) => _statesController.animateTo(0.0, duration: Duration(milliseconds: {})),\n  child: {},\n)",
                    over_ms as u64, over_ms as u64, over_ms as u64, result
                );
            }
        }
        if has_hover {
            if let Some(hover_def) = states_ir.states.get("hover") {
                let over_ms = hover_def
                    .over
                    .as_ref()
                    .and_then(|e| {
                        if let ExprIR::LiteralExpr { value } = e {
                            value.as_str().and_then(parse_dart_duration_ms)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(150.0);
                result = format!(
                    "MouseRegion(\n  onEnter: (_) => _statesController.animateTo(1.0, duration: Duration(milliseconds: {})),\n  onExit: (_) => _statesController.animateTo(0.0, duration: Duration(milliseconds: {})),\n  child: {},\n)",
                    over_ms as u64, over_ms as u64, result
                );
            }
        }
    } else if let Some(ref _signal) = states_ir.signal {
        // Custom signal-driven states: use AnimatedBuilder with state lookup
        let signal_dart = format_expr_ir_flutter(_signal);
        result = format!(
            "AnimatedBuilder(\n  animation: _statesController,\n  builder: (context, child) {{\n    final _state = {};\n    // Transition to matching state based on signal value\n    return child!;\n  }},\n  child: {},\n)",
            signal_dart, result
        );
    }

    result
}

/// Parse a duration expression string ("300ms", "1.5s") for Dart codegen.
pub(crate) fn parse_dart_duration_ms(val: &str) -> Option<f64> {
    if let Some(ms) = val.strip_suffix("ms") {
        ms.parse::<f64>().ok()
    } else if let Some(s) = val.strip_suffix('s') {
        s.parse::<f64>().ok().map(|s| s * 1000.0)
    } else {
        None
    }
}

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
