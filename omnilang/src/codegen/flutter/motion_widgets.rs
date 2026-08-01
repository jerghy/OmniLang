//! InView / Scroll / Drag widget classes and event data extraction for Flutter codegen.

use crate::codegen::CodeWriter;
use crate::ir::*;

// ═══════════════════════════════════════════════════════════════════
// InView / Scroll helper widget codegen
// ═══════════════════════════════════════════════════════════════════

/// Check if the view tree contains any element with an @in-view trigger.
pub(crate) fn has_trigger_kind_in_view(node: &ViewTree) -> bool {
    omnilang_core::codegen::shared::view_tree::view_tree_any(node, |el| {
        el.motion_behavior().is_some_and(|m| {
            m.triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::InView))
        })
    })
}

/// Check if the view tree contains any element with an @scroll trigger.
pub(crate) fn has_trigger_kind_scroll(node: &ViewTree) -> bool {
    omnilang_core::codegen::shared::view_tree::view_tree_any(node, |el| {
        el.motion_behavior().is_some_and(|m| {
            m.triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::Scroll))
        })
    })
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
    omnilang_core::codegen::shared::view_tree::view_tree_any(node, |el| {
        el.motion_behavior().is_some_and(|m| {
            m.triggers.iter().any(|t| {
                matches!(
                    t.kind,
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical
                )
            })
        })
    })
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
    // Max drag distance (px) derived at compile time from the motion's own
    // keyframe range in the drag axis — used when no constraints are given
    // (ADR-053 removed @drag-constraints; deferral #19). 0.0 means unbounded.
    w.emit("final double dragExtent;");
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
    w.emit("this.dragExtent = 0.0,");
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
    w.emit("final maxExtent = widget.constraints.isNotEmpty");
    w.indent();
    w.emit("? (maxVal - minVal).abs().clamp(1.0, double.infinity)");
    w.emit(": (widget.dragExtent > 0 ? widget.dragExtent : double.infinity);");
    w.dedent();
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
        (EventDataKind::PointerEvent, "tiltY") => "0.0".to_string(),
        (EventDataKind::PointerEvent, "pointerType") => "'pointer'".to_string(),
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
        (EventDataKind::TouchEvent, "touches") => "<Map<String,dynamic>>[]".to_string(),
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
        (EventDataKind::WheelEvent, "deltaMode") => "0".to_string(),
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
        (EventDataKind::InputEvent, "value") => raw.to_string(),
        (EventDataKind::InputEvent, "checked") => raw.to_string(),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => raw.to_string(),

        // SubmitEvent
        (EventDataKind::SubmitEvent, "values") => {
            "{} /* FormData not available in Flutter */".to_string()
        }

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => raw.to_string(),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => "''".to_string(),

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
        (EventDataKind::DropEvent, "files") => "<String>[]".to_string(),
        (EventDataKind::DropEvent, "text") => "''".to_string(),
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
        (EventDataKind::SelectEvent, "start") => "0".to_string(),
        (EventDataKind::SelectEvent, "end") => "0".to_string(),
        (EventDataKind::SelectEvent, "text") => "''".to_string(),

        // ResizeEvent — Flutter uses MediaQuery or LayoutBuilder
        (EventDataKind::ResizeEvent, "width") => "MediaQuery.of(context).size.width".to_string(),
        (EventDataKind::ResizeEvent, "height") => "MediaQuery.of(context).size.height".to_string(),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => format!("{}.position.inSeconds", raw),
        (EventDataKind::MediaEvent, "duration") => format!("{}.duration?.inSeconds ?? 0.0", raw),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => "0.0".to_string(),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => "0.0".to_string(),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => "''".to_string(),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => "''".to_string(),
        (EventDataKind::AnimationEvent, "elapsedTime") => format!("{}.value", raw),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => "''".to_string(),
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
        None => return format!("{}.detail", raw_var),
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
