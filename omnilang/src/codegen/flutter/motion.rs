//! Motion / Animation codegen helpers for Flutter codegen.

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::expr::format_expr_ir_flutter;

// ═══════════════════════════════════════════════════════════════════
// Motion / Animation codegen helpers
// ═══════════════════════════════════════════════════════════════════

/// Check if any motion in the component uses spring easing (needs physics import).
pub(crate) fn comp_has_spring_motion(comp: &ComponentIR) -> bool {
    comp.motions.iter().any(|m| {
        if let MotionBodyIR::Keyframe(kf) = &m.body {
            matches!(&kf.ease, Some(EasingIR::Spring { .. }))
        } else {
            false
        }
    })
}

/// Check if any motion in the component uses path animation (needs dart:ui import).
pub(crate) fn comp_has_path_motion(comp: &ComponentIR) -> bool {
    comp.motions.iter().any(|m| motion_has_path(m))
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
                if let ExprIR::LiteralExpr { value: v } = value {
                    if let serde_json::Value::Number(n) = v {
                        return n.to_string();
                    }
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
                if let ExprIR::LiteralExpr { value: v } = value {
                    if let serde_json::Value::Number(n) = v {
                        return n.to_string();
                    }
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
        let mut seen = std::collections::HashSet::new();
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
    let reserved_names: std::collections::HashSet<&str> =
        ["idle", "hover", "pressed"].iter().copied().collect();
    let state_names: Vec<&str> = states_ir.states.keys().map(|s| s.as_str()).collect();
    let is_reserved = state_names.iter().all(|n| reserved_names.contains(n));

    if is_reserved {
        // Generate MouseRegion for hover state + GestureDetector for pressed state
        let has_hover = state_names.contains(&"hover");
        let has_pressed = state_names.contains(&"pressed");

        if has_pressed {
            // Get the hover/pressed animation controllers
            if let Some(ref idle_def) = states_ir.states.get("idle") {
                let over_ms = idle_def
                    .over
                    .as_ref()
                    .and_then(|e| {
                        if let ExprIR::LiteralExpr { value } = e {
                            value.as_str().and_then(|s| parse_dart_duration_ms(s))
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
            if let Some(ref hover_def) = states_ir.states.get("hover") {
                let over_ms = hover_def
                    .over
                    .as_ref()
                    .and_then(|e| {
                        if let ExprIR::LiteralExpr { value } = e {
                            value.as_str().and_then(|s| parse_dart_duration_ms(s))
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
            for trigger in &el.motion_triggers {
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
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => {}
    }
}

// ═══════════════════════════════════════════════════════════════════
// InView / Scroll helper widget codegen
// ═══════════════════════════════════════════════════════════════════

/// Check if the view tree contains any element with an @in-view trigger.
pub(crate) fn has_trigger_kind_in_view(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::InView))
            {
                return true;
            }
            el.children.iter().any(|c| has_trigger_kind_in_view(c))
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    if has_trigger_kind_in_view(body) {
                        return true;
                    }
                }
            }
            cond.fallback
                .as_ref()
                .map_or(false, |fb| has_trigger_kind_in_view(fb))
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                if has_trigger_kind_in_view(body) {
                    return true;
                }
            }
            list.empty_body
                .as_ref()
                .map_or(false, |e| has_trigger_kind_in_view(e))
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => false,
    }
}

/// Check if the view tree contains any element with an @scroll trigger.
pub(crate) fn has_trigger_kind_scroll(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::Scroll))
            {
                return true;
            }
            el.children.iter().any(|c| has_trigger_kind_scroll(c))
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    if has_trigger_kind_scroll(body) {
                        return true;
                    }
                }
            }
            cond.fallback
                .as_ref()
                .map_or(false, |fb| has_trigger_kind_scroll(fb))
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                if has_trigger_kind_scroll(body) {
                    return true;
                }
            }
            list.empty_body
                .as_ref()
                .map_or(false, |e| has_trigger_kind_scroll(e))
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => false,
    }
}

/// Emit the _OmniInViewWidget class for @in-view support.
pub(crate) fn emit_inview_widget_class(w: &mut CodeWriter) {
    w.emit("// ── InView wrapper widget (@in-view support) ──");
    w.emit("");
    w.emit("class _OmniInViewWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    w.emit("final bool once;");
    w.emit("final double threshold;");
    w.emit("final VoidCallback onInView;");
    w.emit("");
    w.emit("const _OmniInViewWidget({");
    w.indent();
    w.emit("required this.child,");
    w.emit("this.once = true,");
    w.emit("this.threshold = 0.0,");
    w.emit("required this.onInView,");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("@override");
    w.emit("State<_OmniInViewWidget> createState() => _OmniInViewWidgetState();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class _OmniInViewWidgetState extends State<_OmniInViewWidget> {");
    w.indent();
    w.emit("ScrollPosition? _scrollPosition;");
    w.emit("bool _hasTriggered = false;");
    w.emit("");
    w.emit("@override");
    w.emit("void initState() {");
    w.indent();
    w.emit("super.initState();");
    w.emit("WidgetsBinding.instance.addPostFrameCallback((_) => _checkVisibility());");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("void didChangeDependencies() {");
    w.indent();
    w.emit("super.didChangeDependencies();");
    w.emit("_attachScrollListener();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _attachScrollListener() {");
    w.indent();
    w.emit("final scrollable = Scrollable.maybeOf(context);");
    w.emit("if (scrollable != null && _scrollPosition != scrollable.position) {");
    w.indent();
    w.emit("_scrollPosition?.removeListener(_checkVisibility);");
    w.emit("_scrollPosition = scrollable.position;");
    w.emit("_scrollPosition!.addListener(_checkVisibility);");
    w.emit("_checkVisibility();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("void dispose() {");
    w.indent();
    w.emit("_scrollPosition?.removeListener(_checkVisibility);");
    w.emit("super.dispose();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _checkVisibility() {");
    w.indent();
    w.emit("if (_hasTriggered && widget.once) return;");
    w.emit("final renderBox = context.findRenderObject() as RenderBox?;");
    w.emit("if (renderBox == null || !renderBox.hasSize) return;");
    w.emit("");
    w.emit("final position = renderBox.localToGlobal(Offset.zero);");
    w.emit("final viewportHeight = MediaQuery.of(context).size.height;");
    w.emit("final threshold = widget.threshold;");
    w.emit("");
    w.emit("final visible = position.dy < viewportHeight * (1.0 - threshold) &&");
    w.emit("    position.dy + renderBox.size.height > viewportHeight * threshold;");
    w.emit("");
    w.emit("if (visible) {");
    w.indent();
    w.emit("_hasTriggered = true;");
    w.emit("widget.onInView();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("Widget build(BuildContext context) => widget.child;");
    w.dedent();
    w.emit("}");
}

/// Emit the _OmniScrollDrivenWidget class for @scroll support.
pub(crate) fn emit_scroll_widget_class(w: &mut CodeWriter) {
    w.emit("// ── Scroll-driven wrapper widget (@scroll support) ──");
    w.emit("");
    w.emit("class _OmniScrollDrivenWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    w.emit("final String start;");
    w.emit("final String end;");
    w.emit("final double scrub;");
    w.emit("final ValueChanged<double> onProgress;");
    w.emit("");
    w.emit("const _OmniScrollDrivenWidget({");
    w.indent();
    w.emit("required this.child,");
    w.emit("this.start = 'top 80%',");
    w.emit("this.end = 'bottom 20%',");
    w.emit("this.scrub = 0.0,");
    w.emit("required this.onProgress,");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("@override");
    w.emit("State<_OmniScrollDrivenWidget> createState() => _OmniScrollDrivenWidgetState();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class _OmniScrollDrivenWidgetState extends State<_OmniScrollDrivenWidget> {");
    w.indent();
    w.emit("ScrollPosition? _scrollPosition;");
    w.emit("double _currentProgress = 0.0;");
    w.emit("");
    w.emit("@override");
    w.emit("void initState() {");
    w.indent();
    w.emit("super.initState();");
    w.emit("WidgetsBinding.instance.addPostFrameCallback((_) => _updateProgress());");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("void didChangeDependencies() {");
    w.indent();
    w.emit("super.didChangeDependencies();");
    w.emit("_attachScrollListener();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _attachScrollListener() {");
    w.indent();
    w.emit("final scrollable = Scrollable.maybeOf(context);");
    w.emit("if (scrollable != null && _scrollPosition != scrollable.position) {");
    w.indent();
    w.emit("_scrollPosition?.removeListener(_onScroll);");
    w.emit("_scrollPosition = scrollable.position;");
    w.emit("_scrollPosition!.addListener(_onScroll);");
    w.emit("_updateProgress();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("void dispose() {");
    w.indent();
    w.emit("_scrollPosition?.removeListener(_onScroll);");
    w.emit("super.dispose();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _onScroll() {");
    w.indent();
    w.emit("_updateProgress();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _updateProgress() {");
    w.indent();
    w.emit("final renderBox = context.findRenderObject() as RenderBox?;");
    w.emit("if (renderBox == null || !renderBox.hasSize) return;");
    w.emit("");
    w.emit("final viewportHeight = MediaQuery.of(context).size.height;");
    w.emit("final elementTop = renderBox.localToGlobal(Offset.zero).dy;");
    w.emit("final elementHeight = renderBox.size.height;");
    w.emit("");
    w.emit("// Parse start config: \"top 80%\" -> edge=top, pct=0.8");
    w.emit("final startParts = widget.start.split(' ');");
    w.emit("final startEdge = startParts.isNotEmpty ? startParts[0] : 'top';");
    w.emit("final startPct = (startParts.length == 2)");
    w.emit("    ? (double.tryParse(startParts[1].replaceAll('%', '')) ?? 80.0) / 100.0");
    w.emit("    : 0.8;");
    w.emit("");
    w.emit("// Parse end config: \"bottom 20%\" -> edge=bottom, pct=0.2");
    w.emit("final endParts = widget.end.split(' ');");
    w.emit("final endEdge = endParts.isNotEmpty ? endParts[0] : 'bottom';");
    w.emit("final endPct = (endParts.length == 2)");
    w.emit("    ? (double.tryParse(endParts[1].replaceAll('%', '')) ?? 20.0) / 100.0");
    w.emit("    : 0.2;");
    w.emit("");
    w.emit("// Compute element edge position based on edge keyword");
    w.emit("double getEdgePosition(String edge) {");
    w.emit("  switch (edge) {");
    w.emit("    case 'center': return elementTop + elementHeight / 2;");
    w.emit("    case 'bottom': return elementTop + elementHeight;");
    w.emit("    default: return elementTop; // 'top'");
    w.emit("  }");
    w.emit("}");
    w.emit("");
    w.emit("final startY = viewportHeight * startPct;");
    w.emit("final endY = viewportHeight * endPct;");
    w.emit("final elementStartEdge = getEdgePosition(startEdge);");
    w.emit("final elementEndEdge = getEdgePosition(endEdge);");
    w.emit("");
    w.emit("final totalDistance = startY - (endY - elementHeight);");
    w.emit("final elementProgress = startY - elementStartEdge;");
    w.emit("");
    w.emit("if (totalDistance <= 0) return;");
    w.emit("");
    w.emit("final raw = elementProgress / totalDistance;");
    w.emit("final progress = raw.clamp(0.0, 1.0);");
    w.emit("");
    w.emit("if (widget.scrub > 0) {");
    w.indent();
    w.emit("_currentProgress += (progress - _currentProgress) * (1.0 / (widget.scrub * 60.0));");
    w.emit("_currentProgress = _currentProgress.clamp(0.0, 1.0);");
    w.dedent();
    w.emit("} else {");
    w.indent();
    w.emit("_currentProgress = progress;");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("widget.onProgress(_currentProgress);");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("Widget build(BuildContext context) => widget.child;");
    w.dedent();
    w.emit("}");
}

/// Check if the view tree contains any element with a @drag-horizontal or @drag-vertical trigger.
pub(crate) fn has_trigger_kind_drag(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el.motion_triggers.iter().any(|t| {
                matches!(
                    t.kind,
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical
                )
            }) {
                return true;
            }
            el.children.iter().any(|c| has_trigger_kind_drag(c))
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    if has_trigger_kind_drag(body) {
                        return true;
                    }
                }
            }
            cond.fallback
                .as_ref()
                .map_or(false, |fb| has_trigger_kind_drag(fb))
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                if has_trigger_kind_drag(body) {
                    return true;
                }
            }
            list.empty_body
                .as_ref()
                .map_or(false, |e| has_trigger_kind_drag(e))
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => false,
    }
}

/// Emit the _OmniDragWidget class for @drag-horizontal / @drag-vertical support.
pub(crate) fn emit_drag_widget_class(w: &mut CodeWriter) {
    w.emit("// ── Drag gesture wrapper widget (@drag-horizontal / @drag-vertical support) ──");
    w.emit("");
    w.emit("class _OmniDragWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    w.emit("final Axis axis;");
    w.emit("final String constraints;");
    w.emit("final double elastic;");
    w.emit("final double threshold;");
    w.emit("final VoidCallback onDragStart;");
    w.emit("final ValueChanged<double> onDragProgress;");
    w.emit("final ValueChanged<bool> onDragEnd;");
    w.emit("");
    w.emit("const _OmniDragWidget({");
    w.indent();
    w.emit("required this.child,");
    w.emit("required this.axis,");
    w.emit("this.constraints = '',");
    w.emit("this.elastic = 0.0,");
    w.emit("this.threshold = 0.0,");
    w.emit("required this.onDragStart,");
    w.emit("required this.onDragProgress,");
    w.emit("required this.onDragEnd,");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("@override");
    w.emit("State<_OmniDragWidget> createState() => _OmniDragWidgetState();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class _OmniDragWidgetState extends State<_OmniDragWidget> {");
    w.indent();
    w.emit("double _startPos = 0;");
    w.emit("double _currentDelta = 0;");
    w.emit("double _dragExtent = 300.0; // default max drag distance");
    w.emit("");
    w.emit("double _computeProgress(double delta) {");
    w.indent();
    w.emit("// Parse constraints string: \"left:0 right:300\" or \"top:0 bottom:500\"");
    w.emit("double minVal = double.negativeInfinity;");
    w.emit("double maxVal = double.infinity;");
    w.emit("if (widget.constraints.isNotEmpty) {");
    w.indent();
    w.emit("final parts = widget.constraints.split(' ');");
    w.emit("for (final part in parts) {");
    w.indent();
    w.emit("final kv = part.split(':');");
    w.emit("if (kv.length != 2) continue;");
    w.emit("final key = kv[0];");
    w.emit("final val = double.tryParse(kv[1]) ?? 0;");
    w.emit("if (key == 'left' || key == 'top') minVal = val;");
    w.emit("if (key == 'right' || key == 'bottom') maxVal = val;");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("final maxExtent = (maxVal - minVal).abs().clamp(1.0, double.infinity);");
    w.emit("double constrained = delta;");
    w.emit("if (delta < minVal) {");
    w.indent();
    w.emit("final overshoot = minVal - delta;");
    w.emit("constrained = minVal - overshoot * (1.0 - widget.elastic);");
    w.dedent();
    w.emit("} else if (delta > maxVal) {");
    w.indent();
    w.emit("final overshoot = delta - maxVal;");
    w.emit("constrained = maxVal + overshoot * (1.0 - widget.elastic);");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("return (constrained / maxExtent).clamp(0.0, 1.0);");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _onDragStart(DragStartDetails details) {");
    w.indent();
    w.emit("_startPos = widget.axis == Axis.horizontal");
    w.emit("    ? details.localPosition.dx");
    w.emit("    : details.localPosition.dy;");
    w.emit("_currentDelta = 0;");
    w.emit("widget.onDragStart();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _onDragUpdate(DragUpdateDetails details) {");
    w.indent();
    w.emit("_currentDelta += widget.axis == Axis.horizontal");
    w.emit("    ? details.delta.dx");
    w.emit("    : details.delta.dy;");
    w.emit("final progress = _computeProgress(_currentDelta);");
    w.emit("widget.onDragProgress(progress);");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _onDragEnd(DragEndDetails details) {");
    w.indent();
    w.emit("final progress = _computeProgress(_currentDelta);");
    w.emit("final completed = progress >= widget.threshold;");
    w.emit("widget.onDragEnd(completed);");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("@override");
    w.emit("Widget build(BuildContext context) {");
    w.indent();
    w.emit("return GestureDetector(");
    w.indent();
    w.emit("onHorizontalDragStart: widget.axis == Axis.horizontal ? _onDragStart : null,");
    w.emit("onHorizontalDragUpdate: widget.axis == Axis.horizontal ? _onDragUpdate : null,");
    w.emit("onHorizontalDragEnd: widget.axis == Axis.horizontal ? _onDragEnd : null,");
    w.emit("onVerticalDragStart: widget.axis == Axis.vertical ? _onDragStart : null,");
    w.emit("onVerticalDragUpdate: widget.axis == Axis.vertical ? _onDragUpdate : null,");
    w.emit("onVerticalDragEnd: widget.axis == Axis.vertical ? _onDragEnd : null,");
    w.emit("child: widget.child,");
    w.dedent();
    w.emit(");");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════════
// Flutter event data extraction
// ═══════════════════════════════════════════════════════════════

/// Flutter target field extraction — maps schema fields to Dart expressions.
/// Modifier keys use HardwareKeyboard.instance, button uses bitmask conversion.
pub(crate) fn field_extract_flutter(kind: &EventDataKind, field: &str, raw: &str) -> String {
    match (kind, field) {
        // PointerEvent — from pointer event details (TapDownDetails, etc.)
        (EventDataKind::PointerEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::PointerEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::PointerEvent, "button") => {
            format!("({}.buttons & 0x1 != 0 ? 0 : {}.buttons & 0x2 != 0 ? 2 : {}.buttons & 0x4 != 0 ? 1 : 0)", raw, raw, raw)
        }
        (EventDataKind::PointerEvent, "pressure") => format!("{}.pressure", raw),
        (EventDataKind::PointerEvent, "tiltX") => format!("{}.tilt", raw),
        (EventDataKind::PointerEvent, "tiltY") => format!("0.0"),
        (EventDataKind::PointerEvent, "pointerType") => format!("'pointer'"),
        (EventDataKind::PointerEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // TouchEvent
        (EventDataKind::TouchEvent, "touches") => format!("<Map<String,dynamic>>[]"),
        (EventDataKind::TouchEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // WheelEvent — from PointerSignalEvent
        (EventDataKind::WheelEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::WheelEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::WheelEvent, "deltaX") => format!("{}.scrollDelta.dx", raw),
        (EventDataKind::WheelEvent, "deltaY") => format!("{}.scrollDelta.dy", raw),
        (EventDataKind::WheelEvent, "deltaZ") => format!("{}.scrollDelta.dy", raw),
        (EventDataKind::WheelEvent, "deltaMode") => format!("0"),
        (EventDataKind::WheelEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // KeyEvent — from KeyEvent
        (EventDataKind::KeyEvent, "key") => format!("{}.logicalKey.keyLabel", raw),
        (EventDataKind::KeyEvent, "code") => format!("{}.logicalKey.debugName", raw),
        (EventDataKind::KeyEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // ScrollEvent — from ScrollNotification
        (EventDataKind::ScrollEvent, "x") => format!("{}.metrics.pixels", raw),
        (EventDataKind::ScrollEvent, "y") => format!("{}.metrics.pixels", raw),

        // InputEvent
        (EventDataKind::InputEvent, "value") => format!("{}", raw),
        (EventDataKind::InputEvent, "checked") => format!("{}", raw),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => format!("{}", raw),

        // SubmitEvent
        (EventDataKind::SubmitEvent, "values") => {
            format!("{{}} /* FormData not available in Flutter */")
        }

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => format!("{}", raw),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => format!("''"),

        // DragEvent — from DragUpdateDetails
        (EventDataKind::DragEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::DragEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::DragEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // DropEvent
        (EventDataKind::DropEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::DropEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::DropEvent, "files") => format!("<String>[]"),
        (EventDataKind::DropEvent, "text") => format!("''"),
        (EventDataKind::DropEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // ContextMenuEvent
        (EventDataKind::ContextMenuEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::ContextMenuEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::ContextMenuEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // SelectEvent
        (EventDataKind::SelectEvent, "start") => format!("0"),
        (EventDataKind::SelectEvent, "end") => format!("0"),
        (EventDataKind::SelectEvent, "text") => format!("''"),

        // ResizeEvent — Flutter uses MediaQuery or LayoutBuilder
        (EventDataKind::ResizeEvent, "width") => format!("MediaQuery.of(context).size.width"),
        (EventDataKind::ResizeEvent, "height") => format!("MediaQuery.of(context).size.height"),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => format!("{}.position.inSeconds", raw),
        (EventDataKind::MediaEvent, "duration") => format!("{}.duration?.inSeconds ?? 0.0", raw),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => format!("0.0"),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => format!("0.0"),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => format!("''"),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => format!("''"),
        (EventDataKind::AnimationEvent, "elapsedTime") => format!("{}.value", raw),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => format!("''"),
        (EventDataKind::TransitionEvent, "elapsedTime") => format!("{}.value", raw),

        _ => format!("null /* {} */", field),
    }
}

/// Emit a Dart Map literal for Flutter event data extraction.
/// Only emits fields in `used_fields` (or all fields from schema if None).
pub(crate) fn emit_event_object_flutter(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        None => return "{}".to_string(),
    };

    let schema = event_data_schema(kind);
    let fields: Vec<String> = match used_fields {
        Some(fields) if !fields.is_empty() => schema
            .iter()
            .filter(|f| fields.contains(&f.name))
            .map(|f| {
                let extract = field_extract_flutter(kind, &f.name, raw_var);
                format!("'{}': {}", f.name, extract)
            })
            .collect(),
        Some(_) => return "{}".to_string(),
        None => schema
            .iter()
            .map(|f| {
                let extract = field_extract_flutter(kind, &f.name, raw_var);
                format!("'{}': {}", f.name, extract)
            })
            .collect(),
    };

    format!("{{ {} }}", fields.join(", "))
}
