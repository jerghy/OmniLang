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
                .any(|t| matches!(&t.kind, TriggerKind::Scroll { .. }))
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
    w.emit("// start/end are the v1 numeric scroll range (Task 5.1) — absolute");
    w.emit("// scroll positions where the mapping starts/ends. When null the");
    w.emit("// widget falls back to the legacy viewport-relative string configs");
    w.emit("// (\"top 80%\" / \"bottom 20%\").");
    w.emit("");
    w.emit("class _OmniScrollDrivenWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    w.emit("final double? start;");
    w.emit("final double? end;");
    w.emit("final double scrub;");
    w.emit("final ValueChanged<double> onProgress;");
    w.emit("");
    w.emit("const _OmniScrollDrivenWidget({");
    w.indent();
    w.emit("required this.child,");
    w.emit("this.start,");
    w.emit("this.end,");
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
    w.emit("final double? typedStart = widget.start;");
    w.emit("final double? typedEnd = widget.end;");
    w.emit("double progress;");
    w.emit("if (typedStart != null && typedEnd != null) {");
    w.indent();
    w.emit("// v1 numeric scroll range: progress = (element edge - start) / (end - start)");
    w.emit("final scrollOffset = _scrollPosition?.pixels ?? 0.0;");
    w.emit("final absEdge = scrollOffset + elementTop;");
    w.emit("final range = typedEnd - typedStart;");
    w.emit("if (range <= 0) return;");
    w.emit("final raw = (absEdge - typedStart) / range;");
    w.emit("progress = raw.clamp(0.0, 1.0);");
    w.dedent();
    w.emit("} else {");
    w.indent();
    w.emit("// Legacy viewport-relative configs: \"top 80%\" -> edge=top, pct=0.8");
    w.emit("final startCfg = 'top 80%';");
    w.emit("final endCfg = 'bottom 20%';");
    w.emit("final startParts = startCfg.split(' ');");
    w.emit("final startEdge = startParts.isNotEmpty ? startParts[0] : 'top';");
    w.emit("final startPct = (startParts.length == 2)");
    w.emit("    ? (double.tryParse(startParts[1].replaceAll('%', '')) ?? 80.0) / 100.0");
    w.emit("    : 0.8;");
    w.emit("");
    w.emit("final endParts = endCfg.split(' ');");
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
    w.emit("");
    w.emit("final totalDistance = startY - (endY - elementHeight);");
    w.emit("final elementProgress = startY - elementStartEdge;");
    w.emit("");
    w.emit("if (totalDistance <= 0) return;");
    w.emit("");
    w.emit("final raw = elementProgress / totalDistance;");
    w.emit("progress = raw.clamp(0.0, 1.0);");
    w.dedent();
    w.emit("}");
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
                    &t.kind,
                    TriggerKind::DragHorizontal { .. } | TriggerKind::DragVertical { .. }
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
    // v1 numeric constraints (Task 5.1): constraintsMin/Max define the drag
    // range directly. The legacy `constraints` string ("left:0 right:300")
    // remains as a runtime-parsed fallback when the typed pair is absent.
    w.emit("final String constraints;");
    w.emit("final double? constraintsMin;");
    w.emit("final double? constraintsMax;");
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
    w.emit("this.constraintsMin,");
    w.emit("this.constraintsMax,");
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
    w.emit("double _currentDelta = 0;");
    w.emit("");
    w.emit("double _computeProgress(double delta) {");
    w.indent();
    w.emit("double minVal = double.negativeInfinity;");
    w.emit("double maxVal = double.infinity;");
    w.emit("final bool hasTypedConstraints = widget.constraintsMin != null && widget.constraintsMax != null;");
    w.emit("if (hasTypedConstraints) {");
    w.indent();
    w.emit("// v1 numeric constraints: constraints: [min, max]");
    w.emit("minVal = widget.constraintsMin!;");
    w.emit("maxVal = widget.constraintsMax!;");
    w.dedent();
    w.emit("} else if (widget.constraints.isNotEmpty) {");
    w.indent();
    w.emit("// Parse legacy constraints string: \"left:0 right:300\" or \"top:0 bottom:500\"");
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
    w.emit("final maxExtent = hasTypedConstraints || widget.constraints.isNotEmpty");
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

/// Warn that an event field cannot be mapped on the Flutter target and that
/// the generated code carries a degradation placeholder (ADR-054 honest
/// degradation — silent stubs are forbidden).
fn warn_event_field_stub(
    warnings: &mut Vec<String>,
    kind: &EventDataKind,
    field: &str,
    reason: &str,
) {
    warnings.push(format!(
        "[flutter] event field '{field}' of {} is not mappable: {reason}; \
         emitting a degradation placeholder",
        event_data_kind_label(kind)
    ));
}

/// Flutter target field extraction — maps schema fields to Dart expressions.
/// Modifier keys use HardwareKeyboard.instance, button uses bitmask conversion.
///
/// Contract (ADR-054): every unmapped/dropped field emits a warning naming
/// the field AND the event kind through the warnings channel, and the
/// generated expression carries an inline degradation comment — a stub must
/// never be silent.
pub(crate) fn field_extract_flutter(
    kind: &EventDataKind,
    field: &str,
    raw: &str,
    warnings: &mut Vec<String>,
) -> String {
    match (kind, field) {
        // PointerEvent — from pointer event details (TapDownDetails, etc.)
        (EventDataKind::PointerEvent, "x") => format!("{}.localPosition.dx", raw),
        (EventDataKind::PointerEvent, "y") => format!("{}.localPosition.dy", raw),
        (EventDataKind::PointerEvent, "button") => {
            format!("({}.buttons & 0x1 != 0 ? 0 : {}.buttons & 0x2 != 0 ? 2 : {}.buttons & 0x4 != 0 ? 1 : 0)", raw, raw, raw)
        }
        (EventDataKind::PointerEvent, "pressure") => format!("{}.pressure", raw),
        (EventDataKind::PointerEvent, "tiltX") => format!("{}.tilt", raw),
        (EventDataKind::PointerEvent, "tiltY") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "Flutter PointerEvent exposes only a single tilt angle",
            );
            "0.0 /* tiltY: Flutter exposes tilt as a single angle, no per-axis component */"
                .to_string()
        }
        // pointerType: real mapping — PointerDeviceKind is the stable
        // analogue of DOM pointerType ("mouse"/"touch"/"stylus"/...).
        (EventDataKind::PointerEvent, "pointerType") => {
            format!(
                "{}.kind.name /* pointerType: PointerDeviceKind analogue of DOM pointerType */",
                raw
            )
        }
        (EventDataKind::PointerEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let prop = match m {
                "alt" => "isAltPressed",
                "ctrl" => "isControlPressed",
                "shift" => "isShiftPressed",
                _ => "isMetaPressed",
            };
            format!("HardwareKeyboard.instance.{}", prop)
        }

        // TouchEvent — the touch point list is not exposed by Flutter
        // gesture callbacks → degraded constant, warned.
        (EventDataKind::TouchEvent, "touches") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "Flutter gesture callbacks do not expose the touch point list",
            );
            "<Map<String,dynamic>>[] /* touches: not exposed by Flutter gesture callbacks */"
                .to_string()
        }
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
        // deltaZ: PointerScrollEvent is 2D — the z-axis has no source; the
        // vertical delta is a degraded fallback, warned.
        (EventDataKind::WheelEvent, "deltaZ") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "PointerScrollEvent has no z-axis; vertical delta used as fallback",
            );
            format!(
                "{}.scrollDelta.dy /* deltaZ: no z-axis in PointerScrollEvent; vertical delta fallback */",
                raw
            )
        }
        (EventDataKind::WheelEvent, "deltaMode") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "no DOM-style delta mode in Flutter scroll events",
            );
            "0 /* deltaMode: no DOM-style delta mode in Flutter */".to_string()
        }
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
        // code: STABLE mapping — physicalKey.keyId is the hardware key id
        // (layout-independent, the DOM 'code' analogue). debugName is
        // debug-oriented and must never be emitted as event data.
        (EventDataKind::KeyEvent, "code") => {
            format!(
                "{}.physicalKey.keyId /* code: stable physical key id (DOM 'code' analogue) */",
                raw
            )
        }
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

        // SubmitEvent — no FormData equivalent in Flutter callbacks.
        (EventDataKind::SubmitEvent, "values") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "no FormData equivalent in Flutter callbacks",
            );
            "{} /* values: FormData not available in Flutter */".to_string()
        }

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => raw.to_string(),

        // ClipboardEvent — no clipboard access from a Flutter event callback.
        (EventDataKind::ClipboardEvent, "text") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "no clipboard access from a Flutter event callback",
            );
            "'' /* text: no clipboard access in Flutter events */".to_string()
        }

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
        (EventDataKind::DropEvent, "files") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "no file payload in Flutter drop callbacks",
            );
            "<String>[] /* files: no file payload in Flutter drop callbacks */".to_string()
        }
        (EventDataKind::DropEvent, "text") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "no clipboard access from a Flutter event callback",
            );
            "'' /* text: no clipboard access in Flutter events */".to_string()
        }
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

        // SelectEvent — text selection is not exposed by Flutter events.
        (EventDataKind::SelectEvent, m @ ("start" | "end" | "text")) => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "text selection is not exposed by Flutter event callbacks",
            );
            match m {
                "start" => "0 /* start: no selection data in Flutter events */".to_string(),
                "end" => "0 /* end: no selection data in Flutter events */".to_string(),
                _ => "'' /* text: no selection data in Flutter events */".to_string(),
            }
        }

        // ResizeEvent — Flutter uses MediaQuery or LayoutBuilder
        (EventDataKind::ResizeEvent, "width") => "MediaQuery.of(context).size.width".to_string(),
        (EventDataKind::ResizeEvent, "height") => "MediaQuery.of(context).size.height".to_string(),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => format!("{}.position.inSeconds", raw),
        (EventDataKind::MediaEvent, "duration") => format!("{}.duration?.inSeconds ?? 0.0", raw),

        // MediaLoadEvent — Image.network reports dimensions only via its
        // ImageStream, not through a load event.
        (EventDataKind::MediaLoadEvent, "naturalWidth") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "Image.network reports dimensions only via its ImageStream, not an event",
            );
            "0.0 /* naturalWidth: not exposed by Flutter load events */".to_string()
        }
        (EventDataKind::MediaLoadEvent, "naturalHeight") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "Image.network reports dimensions only via its ImageStream, not an event",
            );
            "0.0 /* naturalHeight: not exposed by Flutter load events */".to_string()
        }

        // MediaErrorEvent — Flutter's errorBuilder receives an exception
        // object, not an event message.
        (EventDataKind::MediaErrorEvent, "message") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "Flutter errorBuilder receives an exception object, not an event message",
            );
            "'' /* message: Flutter errorBuilder receives an exception, not an event message */"
                .to_string()
        }

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => {
            warn_event_field_stub(warnings, kind, field, "AnimationController exposes no name");
            "'' /* animationName: AnimationController exposes no name */".to_string()
        }
        (EventDataKind::AnimationEvent, "elapsedTime") => format!("{}.value", raw),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => {
            warn_event_field_stub(
                warnings,
                kind,
                field,
                "implicit animations do not report a property name",
            );
            "'' /* propertyName: implicit animations report no property name */".to_string()
        }
        (EventDataKind::TransitionEvent, "elapsedTime") => format!("{}.value", raw),

        _ => {
            warn_event_field_stub(warnings, kind, field, "field is not in the event schema");
            format!("null /* {field}: unknown field */")
        }
    }
}

/// Emit a Dart Map literal for Flutter event data extraction.
/// Only emits fields in `used_fields` (or all fields from schema if None).
///
/// Threads the compiler warnings channel into per-field extraction so
/// dropped/unmapped fields surface diagnostics (ADR-054) — never silence.
pub(crate) fn emit_event_object_flutter(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
    warnings: &mut Vec<String>,
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
                let extract = field_extract_flutter(kind, &f.name, raw_var, warnings);
                format!("'{}': {}", f.name, extract)
            })
            .collect(),
        Some(_) => return "{}".to_string(),
        None => schema
            .iter()
            .map(|f| {
                let extract = field_extract_flutter(kind, &f.name, raw_var, warnings);
                format!("'{}': {}", f.name, extract)
            })
            .collect(),
    };

    format!("{{ {} }}", fields.join(", "))
}
