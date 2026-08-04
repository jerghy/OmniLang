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

/// Emit the OmniInViewWidget class for @in-view support.
///
/// Public class names (no underscore): the helpers live in the shared
/// `_omniview_runtime.dart` (Task 5.6) and are imported by every component
/// that uses the trigger — Dart privacy is library-scoped, so an imported
/// `_private` name would not be visible to the component files.
pub(crate) fn emit_inview_widget_class(w: &mut CodeWriter) {
    w.emit("// ── InView wrapper widget (@in-view support) ──");
    w.emit("");
    w.emit("class OmniInViewWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    // Task 3.4: inViewOnce / inViewThreshold — the emitted constructor
    // passes the config under these names (v2 DSL: an @in-view trigger is
    // once-semantics by default; threshold stays 0.0 unless the legacy
    // in_view_threshold IR field is populated).
    w.emit("final bool inViewOnce;");
    w.emit("final double inViewThreshold;");
    w.emit("final VoidCallback onInView;");
    w.emit("");
    w.emit("const OmniInViewWidget({");
    w.indent();
    w.emit("required this.child,");
    w.emit("this.inViewOnce = true,");
    w.emit("this.inViewThreshold = 0.0,");
    w.emit("required this.onInView,");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("@override");
    w.emit("State<OmniInViewWidget> createState() => _OmniInViewWidgetState();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class _OmniInViewWidgetState extends State<OmniInViewWidget> {");
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
    w.emit("if (_hasTriggered && widget.inViewOnce) return;");
    w.emit("final renderBox = context.findRenderObject() as RenderBox?;");
    w.emit("if (renderBox == null || !renderBox.hasSize) return;");
    w.emit("");
    w.emit("final position = renderBox.localToGlobal(Offset.zero);");
    w.emit("final viewportHeight = MediaQuery.of(context).size.height;");
    w.emit("final threshold = widget.inViewThreshold;");
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

/// Emit the OmniScrollDrivenWidget class for @scroll support.
///
/// Public class name (no underscore): the helper lives in the shared
/// `_omniview_runtime.dart` (Task 5.6) and is imported by trigger-using
/// component files.
pub(crate) fn emit_scroll_widget_class(w: &mut CodeWriter) {
    w.emit("// ── Scroll-driven wrapper widget (@scroll support) ──");
    w.emit("// start/end are the v1 numeric scroll range (Task 5.1) — absolute");
    w.emit("// scroll positions where the mapping starts/ends. When null the");
    w.emit("// widget falls back to the legacy viewport-relative defaults");
    w.emit("// (top edge 80% → 20% of the viewport) compiled to typed");
    w.emit("// constants — no runtime string parsing (Task 5.6).");
    w.emit("");
    w.emit("class OmniScrollDrivenWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    w.emit("final double? start;");
    w.emit("final double? end;");
    w.emit("final double scrub;");
    w.emit("final ValueChanged<double> onProgress;");
    w.emit("");
    w.emit("const OmniScrollDrivenWidget({");
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
    w.emit("State<OmniScrollDrivenWidget> createState() => _OmniScrollDrivenWidgetState();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class _OmniScrollDrivenWidgetState extends State<OmniScrollDrivenWidget> {");
    w.indent();
    w.emit("ScrollPosition? _scrollPosition;");
    w.emit("double _currentProgress = 0.0;");
    w.emit("// Frame clock of the last progress update (Task 5.6): scrub");
    w.emit("// smoothing derives elapsed time from the ticker-driven frame");
    w.emit("// timestamps instead of assuming 60fps.");
    w.emit("Duration _lastUpdate = Duration.zero;");
    w.emit("bool _hasLastUpdate = false;");
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
    w.emit("// Frame-timestamp read that never asserts: currentFrameTimeStamp");
    w.emit("// throws outside a frame; scroll notifications can legally fire");
    w.emit("// outside one (Task 3.4 side-fix).");
    w.emit("Duration _safeFrameTime() {");
    w.indent();
    w.emit("try {");
    w.indent();
    w.emit("return WidgetsBinding.instance.currentFrameTimeStamp;");
    w.dedent();
    w.emit("} catch (_) {");
    w.indent();
    w.emit("return _lastUpdate + const Duration(milliseconds: 16);");
    w.dedent();
    w.emit("}");
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
    w.emit("// v1 numeric scroll range: progress = (scrollOffset - start) / (end - start)");
    w.emit("// Mirrors React/OmniView's (scrollY - start) / (end - start) — the");
    w.emit("// scroll offset alone drives the mapping (elementTop is viewport-");
    w.emit("// relative and would make the sum constant while scrolling).");
    w.emit("final scrollOffset = _scrollPosition?.pixels ?? 0.0;");
    w.emit("final range = typedEnd - typedStart;");
    w.emit("if (range <= 0) return;");
    w.emit("final raw = (scrollOffset - typedStart) / range;");
    w.emit("progress = raw.clamp(0.0, 1.0);");
    w.dedent();
    w.emit("} else {");
    w.indent();
    w.emit("// Legacy viewport-relative bridge (pre-v1): the element's top edge");
    w.emit("// crossing the 80% → 20% viewport band. The former \"top 80%\" /");
    w.emit("// \"bottom 20%\" string configs are compiled to typed constants —");
    w.emit("// no runtime string parsing (Task 5.6).");
    w.emit("final double startY = viewportHeight * 0.8;");
    w.emit("final double endY = viewportHeight * 0.2;");
    w.emit("final double elementStartEdge = elementTop;");
    w.emit("");
    w.emit("final double totalDistance = startY - (endY - elementHeight);");
    w.emit("final double elementProgress = startY - elementStartEdge;");
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
    w.emit("// Frame-rate-independent smoothing: the previous math moved");
    w.emit("// 1 / (scrub * 60.0) of the way per frame — a 60fps hardcode that");
    w.emit("// over-smooths on 120Hz displays. Elapsed time comes from the");
    w.emit("// ticker-driven frame clock (WidgetsBinding.currentFrameTimeStamp —");
    w.emit("// the scheduler's frame timestamp, the same clock the ticker runs");
    w.emit("// on), and the factor is the exact exponential-smoothing term");
    w.emit("// whose 60Hz value matches the old curve (Task 5.6). scrub is the");
    w.emit("// time constant in seconds.");
    w.emit("// Task 3.4 side-fix: the frame-timestamp getter ASSERTs when read");
    w.emit("// outside a frame — and scroll notifications legally fire outside");
    w.emit("// frames (pointer events in widget tests, programmatic scrolls) —");
    w.emit("// so the read goes through a helper that falls back to the last");
    w.emit("// update time plus one nominal frame instead of crashing.");
    w.emit("final Duration now = _safeFrameTime();");
    w.emit("final double dtSeconds = _hasLastUpdate");
    w.emit("    ? (now - _lastUpdate).inMicroseconds / 1000000.0");
    w.emit("    : (1.0 / 60.0);");
    w.emit("_lastUpdate = now;");
    w.emit("_hasLastUpdate = true;");
    w.emit("final double factor = 1.0 - math.exp(-dtSeconds / widget.scrub);");
    w.emit("_currentProgress += (progress - _currentProgress) * factor;");
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

/// Emit the OmniDragWidget class for @drag-horizontal / @drag-vertical support.
///
/// Public class name (no underscore): the helper lives in the shared
/// `_omniview_runtime.dart` (Task 5.6) and is imported by trigger-using
/// component files. The legacy stringly-typed `constraints` field
/// ("left:0 right:300") is removed — the codegen always emits the typed
/// `constraintsMin`/`constraintsMax` pair (Task 5.6, verified against the
/// emission sites), so the runtime-parsed fallback was dead code.
pub(crate) fn emit_drag_widget_class(w: &mut CodeWriter) {
    w.emit("// ── Drag gesture wrapper widget (@drag-horizontal / @drag-vertical support) ──");
    w.emit("");
    w.emit("class OmniDragWidget extends StatefulWidget {");
    w.indent();
    w.emit("final Widget child;");
    w.emit("final Axis axis;");
    // v1 numeric constraints (Task 5.1): constraintsMin/Max define the drag
    // range directly; the stringly-typed `constraints` fallback is removed
    // (Task 5.6 — the codegen never emits it).
    w.emit("final double? constraintsMin;");
    w.emit("final double? constraintsMax;");
    w.emit("final double elastic;");
    w.emit("final double threshold;");
    // v1 velocity_max (Task 5.1): cap for the drag-velocity tracker (px/s),
    // mirroring the OmniView/React semantics. Inertia decay after release is
    // not implemented on any target (honest degradation, ADR-054) — the
    // velocity is tracked and clamped during the drag only.
    w.emit("final double velocityMax;");
    // Max drag distance (px) derived at compile time from the motion's own
    // keyframe range in the drag axis — used when no constraints are given
    // (ADR-053 removed @drag-constraints; deferral #19). 0.0 means unbounded.
    w.emit("final double dragExtent;");
    w.emit("final VoidCallback onDragStart;");
    w.emit("final ValueChanged<double> onDragProgress;");
    w.emit("final ValueChanged<bool> onDragEnd;");
    w.emit("");
    w.emit("const OmniDragWidget({");
    w.indent();
    w.emit("required this.child,");
    w.emit("required this.axis,");
    w.emit("this.constraintsMin,");
    w.emit("this.constraintsMax,");
    w.emit("this.elastic = 0.0,");
    w.emit("this.threshold = 0.0,");
    w.emit("this.velocityMax = 2000.0,");
    w.emit("this.dragExtent = 0.0,");
    w.emit("required this.onDragStart,");
    w.emit("required this.onDragProgress,");
    w.emit("required this.onDragEnd,");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("@override");
    w.emit("State<OmniDragWidget> createState() => _OmniDragWidgetState();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class _OmniDragWidgetState extends State<OmniDragWidget> {");
    w.indent();
    w.emit("double _currentDelta = 0;");
    w.emit("int _lastMoveTime = 0;");
    w.emit("double _lastMovePos = 0;");
    // velocity_max tracker: written and capped during the drag; inertia
    // decay after release is not implemented on any target (honest
    // degradation, ADR-054) so the value is not read back yet — the
    // analyzer ignore documents the intentional write-only field.
    w.emit("// ignore: unused_field");
    w.emit("double _velocity = 0;");
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
    w.emit("}");
    w.emit("");
    w.emit("final maxExtent = hasTypedConstraints");
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
    w.emit("_lastMoveTime = 0;");
    w.emit("_lastMovePos = 0;");
    w.emit("_velocity = 0;");
    w.emit("widget.onDragStart();");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("void _onDragUpdate(DragUpdateDetails details) {");
    w.indent();
    w.emit("_currentDelta += widget.axis == Axis.horizontal");
    w.emit("    ? details.delta.dx");
    w.emit("    : details.delta.dy;");
    w.emit("// v1 velocity_max (Task 5.1): track drag velocity (px/s) capped at");
    w.emit("// velocityMax — mirrors the OmniView/React drag trackers. Inertia");
    w.emit("// decay after release is not implemented (honest degradation, ADR-054).");
    w.emit("final now = DateTime.now().millisecondsSinceEpoch;");
    w.emit("if (_lastMoveTime > 0) {");
    w.indent();
    w.emit("final dt = now - _lastMoveTime;");
    w.emit("final dp = (_currentDelta - _lastMovePos).abs();");
    w.emit("if (dt > 0) {");
    w.indent();
    w.emit("_velocity = (dp / dt * 1000).clamp(0.0, widget.velocityMax);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("_lastMoveTime = now;");
    w.emit("_lastMovePos = _currentDelta;");
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

/// Whether a view uses any trigger-helper wrapper (@in-view, @scroll, or
/// @drag-*) — gates the `_omniview_runtime.dart` import on component files
/// and the shared runtime emission on the program (Task 5.6).
pub(crate) fn view_uses_trigger_helpers(view: &ViewTree) -> bool {
    has_trigger_kind_in_view(view) || has_trigger_kind_scroll(view) || has_trigger_kind_drag(view)
}

/// Emit the shared `_omniview_runtime.dart` — the Omni* trigger wrapper
/// classes emitted ONCE per program and imported by every component that
/// uses @in-view/@scroll/@drag triggers (Task 5.6).
///
/// Mirrors the design.dart "emit once, import everywhere" pattern: the
/// helpers previously landed verbatim (~90 lines each) in every component
/// file that used the trigger, so multi-component apps drifted. The classes
/// are public (Dart privacy is library-scoped — an imported `_private` name
/// would not be visible to component files); their State subclasses stay
/// private inside this library.
pub(crate) fn emit_trigger_helpers_runtime() -> String {
    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang \u{2014} Flutter trigger-helper runtime");
    w.emit("//");
    w.emit("// Shared wrapper widgets for the @in-view / @scroll / @drag-* motion");
    w.emit("// triggers (Task 5.6). Emitted ONCE per program and imported by every");
    w.emit("// component file that uses the triggers — the previous per-file");
    w.emit("// re-emission drifted across components of a multi-component app.");
    w.emit("");
    w.emit("import 'package:flutter/material.dart';");
    // math.exp — frame-rate-independent scrub smoothing (Ticker-derived
    // elapsed time replaces the old 60fps hardcode).
    w.emit("import 'dart:math' as math;");
    w.emit("");
    emit_inview_widget_class(&mut w);
    w.emit("");
    emit_scroll_widget_class(&mut w);
    w.emit("");
    emit_drag_widget_class(&mut w);
    w.into_string()
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
