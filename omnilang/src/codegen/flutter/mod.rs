//! Flutter codegen: IR -> Flutter Dart components/widgets.
//!
//! Each component generates a `.dart` file with a StatefulWidget + State class.
//! Each store generates a `xxx_store.dart` file with Riverpod 3.0 providers
//! (`Notifier<T>` + `NotifierProvider`).
//! Uses `super::dart::dart_type()` for Dart type annotations.

mod design;
mod design_classes;
mod design_theme;
mod design_tokens;
mod expr;
mod motion;
mod motion_widgets;
mod r#ref;
mod stmt;
mod type_annotate;
mod types;
pub(super) mod utils;
mod view;

pub(crate) use super::capitalize_first;
pub(crate) use expr::*;
pub(crate) use motion::*;
pub(crate) use motion_widgets::*;
pub(crate) use r#ref::*;
pub(crate) use stmt::*;
#[allow(unused_imports)]
pub(crate) use utils::*;
pub(crate) use view::*;

#[cfg(test)]
mod tests;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::build_component_map;
use crate::codegen::shared::direct_write::classify_direct_write;
use crate::codegen::{format_json_value, JSON_FLUTTER};
use crate::error::Diagnostic;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

pub struct FlutterCodegen;

impl Codegen for FlutterCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &FxHashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let component_map = build_component_map(ir);
        // Slot declarations per component (Task 5.3) — named slot names plus
        // "children" for the unnamed slot; drives consumer-side projection.
        let component_slots = view::build_component_slot_map(&component_map);
        for (name, comp) in ir.all_components() {
            let (content, w, errs) =
                self.gen_component_dart(name, comp, ir, &component_map, &component_slots);
            warnings.extend(w);
            errors.extend(errs);
            files.push(CompiledFile {
                path: format!("{}.dart", name),
                content,
            });
        }
        for (name, store) in ir.all_stores() {
            files.push(CompiledFile {
                path: format!("{}_store.dart", name),
                content: self.gen_store_dart(name, store, ir),
            });
        }
        // Trigger-helper runtime (Task 5.6): the Omni* wrapper classes for
        // @in-view/@scroll/@drag triggers are emitted ONCE into
        // _omniview_runtime.dart, imported by component files that use the
        // triggers (usage-gated — no runtime file in trigger-free programs).
        // The effective view mirrors gen_component_dart (body-slot
        // materialization, ADR-053 D5) so the usage check matches the import
        // decision of each component.
        let needs_trigger_runtime = ir.all_components().values().any(|comp| {
            let materialized: Option<ViewTree> = comp
                .view
                .as_ref()
                .filter(|_| !comp.slots.is_empty())
                .map(|v| materialize_view_slots(v, comp, &slot_collision_names(comp)));
            materialized
                .as_ref()
                .or(comp.view.as_ref())
                .is_some_and(view_uses_trigger_helpers)
        });
        if needs_trigger_runtime {
            files.push(CompiledFile {
                path: "_omniview_runtime.dart".to_string(),
                content: emit_trigger_helpers_runtime(),
            });
        }
        // Tagged unions referenced by generated code → types.dart
        // (deferral #30: match variant arms / variant literals need the
        // union classes declared).
        files.extend(types::gen_types_files(ir));
        // Module-level design blocks → design.dart with typed token
        // constants, BoxDecoration factories, ThemeExtension variants,
        // theme switching, and the dynamic token provider (Task 5.5).
        // Unmappable token/class/variant values surface on the warnings
        // channel; spec §10.3 layout rows split channels — ZIndex/cursor
        // warn, display:grid/float hard-error (never silent drops).
        let (design_files, design_warnings, design_errors) = design::gen_design_files(ir);
        warnings.extend(design_warnings);
        errors.extend(design_errors);
        files.extend(design_files);
        // App shell (main.dart) — MaterialApp wired to the theme runtime
        // when @themes + a @theme signal are declared (Task 4.2 D1).
        if let Some(shell) = design::gen_theme_app_shell(ir) {
            files.push(shell);
        }
        // Dedupe per-ref pointer-capture errors by message (multiple refs /
        // multiple components surface one diagnostic, matching the warning
        // dedup below).
        let mut seen: FxHashSet<String> = FxHashSet::default();
        errors.retain(|e| seen.insert(e.message.clone()));
        CompileOutput {
            files,
            errors,
            warnings,
            source_map: vec![],
        }
    }
}

impl FlutterCodegen {
    /// Generate a complete .dart file for a component as a StatefulWidget.
    ///
    /// Returns `(file_content, warnings, errors)` — warnings carry C6
    /// diagnostics for props/bindings/events that could not be mapped onto
    /// the target widget; errors carry hard diagnostics for ops the Flutter
    /// target does not support (pointer capture, design D3 / spec §10.3).
    fn gen_component_dart(
        &self,
        name: &str,
        comp: &ComponentIR,
        ir: &IRProgram,
        component_map: &FxHashMap<&str, &ComponentIR>,
        component_slots: &FxHashMap<String, FxHashSet<String>>,
    ) -> (String, Vec<crate::codegen::CodegenWarning>, Vec<Diagnostic>) {
        let mut w = CodeWriter::new();
        let mut warnings: Vec<crate::codegen::CodegenWarning> = Vec::new();
        // Task 14: compile-time diagnostics from view formatting (unmappable
        // direct-write style values) — merged into the ref_errors return.
        let mut view_errors: Vec<Diagnostic> = Vec::new();
        // ADR-053 D5: body-level `@slot` declarations surface in the view as
        // `{name}` interpolations — materialize them into `ViewTree::Slot`
        // nodes ONCE so the trigger-helper import decision (below), the
        // widget rendering, and the runtime-usage detection all see the
        // declared fallbacks, exactly like view-level `@slot` directives.
        // Fast path: components without body slots keep the original view
        // (no tree clone).
        let materialized_view: Option<ViewTree> = comp
            .view
            .as_ref()
            .filter(|_| !comp.slots.is_empty())
            .map(|v| materialize_view_slots(v, comp, &slot_collision_names(comp)));
        let view = materialized_view.as_ref().or(comp.view.as_ref());

        // ── Phase 3 (ADR-071 decision 2): signal direct-write classification ──
        // D1 signals (reads confined to dynamic-style entries) and D2 signals
        // (write-only) stop being plain State fields mutated via setState —
        // they become `final _name = ValueNotifier<T>(init);` fields whose
        // mutations write `_name.value` directly (zero whole-tree rebuild).
        // Computed signals / store fields never enter the set (shared pass).
        let direct_write_sets = classify_direct_write(comp);
        let direct_write_names: FxHashSet<String> = direct_write_sets
            .d1
            .iter()
            .chain(direct_write_sets.d2.iter())
            .cloned()
            .collect();

        // ── Cross-file library function inlining (Task 5.8) ──
        // Functions defined in OTHER .omni files (pure-logic modules like
        // array_ext.omni / lib.omni) generate no Dart file of their own; a
        // component calling them by name previously emitted bare calls with
        // no declaration (~130 undefined_method errors: demo_*, safe_*,
        // fetch_demo, format_count, ...). Mirroring the React backend: every
        // public fn whose name is referenced anywhere in the component (or
        // transitively by another emitted lib fn) is emitted as a top-level
        // function in the referencing component's file.
        let mut used_fn_names: FxHashSet<String> = FxHashSet::default();
        for computed in &comp.computeds {
            collect_expr_identifiers(&computed.body, &mut used_fn_names);
        }
        for effect in &comp.effects {
            for stmt in &effect.body.statements {
                collect_stmt_identifiers(stmt, &mut used_fn_names);
            }
        }
        for lc in &comp.lifecycles {
            for stmt in &lc.body.statements {
                collect_stmt_identifiers(stmt, &mut used_fn_names);
            }
        }
        for fun in &comp.fns {
            for stmt in &fun.body.statements {
                collect_stmt_identifiers(stmt, &mut used_fn_names);
            }
        }
        if let Some(view) = view {
            collect_view_identifiers(view, &mut used_fn_names);
        }
        // Transitive closure: an emitted lib fn may call other lib fns.
        let all_lib_fns: Vec<&FunctionIR> = ir.all_functions();
        let mut prev_len = 0;
        while used_fn_names.len() > prev_len {
            prev_len = used_fn_names.len();
            for fun in &all_lib_fns {
                if used_fn_names.contains(&fun.name) {
                    for stmt in &fun.body.statements {
                        collect_stmt_identifiers(stmt, &mut used_fn_names);
                    }
                }
            }
        }
        let lib_fns: Vec<&&FunctionIR> = all_lib_fns
            .iter()
            .filter(|f| used_fn_names.contains(&f.name))
            .collect();
        let inlined_lib_fns: FxHashSet<String> = lib_fns.iter().map(|f| f.name.clone()).collect();

        // Task 5.8a: out-of-range integer literals — Dart's int is a SIGNED
        // 64-bit value; a u64-max literal (18446744073709551615) is
        // unrepresentable (integer_literal_out_of_range in dart analyze).
        // Honest degradation (ADR-054): compile-time WARNING naming the
        // limitation; the literal still emits (the analyzer's error is the
        // developer's to resolve — changing it to BigInt would alter the
        // fn's Dart type). Checked in the same per-expression scans that
        // drive the stdlib imports below.
        let mut warn_out_of_range = |e: &ExprIR| {
            if let ExprIR::LiteralExpr { value } = e {
                if value.as_u64().is_some_and(|n| n > i64::MAX as u64) {
                    // No element context at this scan point (per-expression
                    // stdlib/import scan) — honest `-:0:0`.
                    warnings.push(crate::codegen::CodegenWarning::new(
                        "字面量超出 Dart int 范围（64 位有符号）——u64-max 等值在 \
                         Flutter 目标上无法忠实表示，请改用可表示值或 BigInt",
                    ));
                }
            }
        };

        // Stdlib-driven imports (Task 5.8): the stdlib registry emits
        // top-level Dart names (`sqrt`, `Random`, `json.decode`, `http.get`,
        // `SharedPreferences`) that require imports the generated files
        // never added. Scans the component's expressions AND the inlined
        // lib-fn bodies (a lib fn may call math_random internally).
        let mut uses_dart_math = false;
        let mut uses_dart_convert = false;
        let mut uses_http = false;
        let mut uses_prefs = false;
        let mut uses_intl = false;
        for fun in &lib_fns {
            for stmt in &fun.body.statements {
                uses_dart_math |= stmt_uses_stdlib_fns(stmt, DART_MATH_STDLIB_FNS);
                uses_dart_convert |= stmt_uses_stdlib_fns(stmt, DART_CONVERT_STDLIB_FNS);
                uses_http |= stmt_uses_stdlib_fns(stmt, DART_HTTP_STDLIB_FNS);
                uses_prefs |= stmt_uses_stdlib_fns(stmt, DART_PREFS_STDLIB_FNS);
                uses_intl |= stmt_uses_stdlib_fns(stmt, DART_INTL_STDLIB_FNS);
                for e in statement_exprs(stmt) {
                    warn_out_of_range(e);
                }
            }
        }
        let mut scan_expr_for_imports = |e: &ExprIR| {
            uses_dart_math |= expr_uses_stdlib_fns(e, DART_MATH_STDLIB_FNS);
            uses_dart_convert |= expr_uses_stdlib_fns(e, DART_CONVERT_STDLIB_FNS);
            uses_http |= expr_uses_stdlib_fns(e, DART_HTTP_STDLIB_FNS);
            uses_prefs |= expr_uses_stdlib_fns(e, DART_PREFS_STDLIB_FNS);
            uses_intl |= expr_uses_stdlib_fns(e, DART_INTL_STDLIB_FNS);
            warn_out_of_range(e);
        };
        for computed in &comp.computeds {
            scan_expr_for_imports(&computed.body);
        }
        for effect in &comp.effects {
            for stmt in &effect.body.statements {
                for e in statement_exprs(stmt) {
                    scan_expr_for_imports(e);
                }
            }
        }
        for lc in &comp.lifecycles {
            for stmt in &lc.body.statements {
                for e in statement_exprs(stmt) {
                    scan_expr_for_imports(e);
                }
            }
        }
        for fun in &comp.fns {
            for stmt in &fun.body.statements {
                for e in statement_exprs(stmt) {
                    scan_expr_for_imports(e);
                }
            }
        }
        if let Some(view) = view {
            for e in view_exprs(view) {
                scan_expr_for_imports(e);
            }
        }
        for s in &comp.signals {
            scan_expr_for_imports(&s.initial_value);
        }

        // Motion usage (T5.7 per-element ordinals + per-property
        // animations). Computed BEFORE the imports below: skewX/skewY
        // keyframes emit `tan(...)` in the AnimatedBuilder body, which
        // needs `import 'dart:math';` (Task C2b #7).
        let motion_usage: Vec<MotionElementUsage> = view
            .map(|v| motion::collect_motion_element_usage(v, component_map))
            .unwrap_or_default();
        let used_motion_names: FxHashSet<String> = motion_usage
            .iter()
            .flat_map(|u| u.motions.iter().map(|(n, _)| n.clone()))
            .collect();
        let uses_skew_motion = ir.modules.values().any(|m| {
            m.motions.iter().any(|mo| {
                used_motion_names.contains(&mo.name)
                    && mo
                        .body
                        .stops
                        .iter()
                        .any(|s| s.properties.keys().any(|p| p == "skewX" || p == "skewY"))
            })
        });
        uses_dart_math |= uses_skew_motion;

        w.emit("// Generated by OmniLang \u{2014} Flutter target");
        // Source traceability marker (task 7.5, regeneration contract):
        // maps the generated file back to the .omni declaration — edit the
        // .omni source and regenerate, never hand-edit generated output.
        // `source_file`/`source_span` are populated during lowering
        // (ComponentIR.source_span; hand-built IR fixtures emit no marker).
        if let Some(file) = comp.source_file.as_deref() {
            let line = comp.source_span.map(|s| s.line).unwrap_or(0);
            w.emit(&format!("// [omni] {}:{}", file, line));
        }
        w.emit("");

        // Component imports (cross-file composition): each child component
        // instantiated in this component's view lives in its own generated
        // .dart file — import it so the widget expression resolves. The
        // component itself is skipped (self-import would be invalid).
        let mut child_tags: Vec<&str> = Vec::new();
        if let Some(view) = comp.view.as_ref() {
            view::collect_component_tags(view, component_map, &mut child_tags);
        }
        child_tags.sort();
        child_tags.dedup();
        let child_imports: Vec<&str> = child_tags.into_iter().filter(|t| *t != name).collect();

        // Task 5.8: a component whose NAME collides with a material widget
        // class (TabBar, DataTable, ...) would make the local import
        // ambiguous — hide the clashing name from material.dart so the
        // local class resolves. `import 'package:flutter/material.dart'
        // hide TabBar;` — the component IS the widget in this file.
        let material_hides: Vec<String> = {
            let mut names: Vec<String> = Vec::new();
            for tag in std::iter::once(name).chain(child_imports.iter().copied()) {
                if MATERIAL_WIDGET_CLASS_NAMES.contains(&tag) {
                    names.push(tag.to_string());
                }
            }
            names.sort();
            names.dedup();
            names
        };
        if material_hides.is_empty() {
            w.emit("import 'package:flutter/material.dart';");
        } else {
            w.emit_fmt(format_args!(
                "import 'package:flutter/material.dart' hide {};",
                material_hides.join(", ")
            ));
        }
        w.emit("");

        // Store imports — Task 5.8a: gated on ACTUAL usage (a declared but
        // unreferenced `@store` left the import unused — main.omni's App
        // declares TodoStore/ThemeStore/NotificationStore for its children).
        let mut used_stores: FxHashSet<String> = FxHashSet::default();
        {
            let scan_expr = collect_expr_store_aliases;
            for fun in &lib_fns {
                for stmt in &fun.body.statements {
                    collect_stmt_store_aliases(stmt, &mut used_stores);
                }
            }
            for computed in &comp.computeds {
                scan_expr(&computed.body, &mut used_stores);
            }
            for effect in &comp.effects {
                for stmt in &effect.body.statements {
                    collect_stmt_store_aliases(stmt, &mut used_stores);
                }
            }
            for lc in &comp.lifecycles {
                for stmt in &lc.body.statements {
                    collect_stmt_store_aliases(stmt, &mut used_stores);
                }
            }
            for fun in &comp.fns {
                for stmt in &fun.body.statements {
                    collect_stmt_store_aliases(stmt, &mut used_stores);
                }
            }
            if let Some(view) = view {
                for e in view_exprs(view) {
                    scan_expr(e, &mut used_stores);
                }
            }
            for s in &comp.signals {
                scan_expr(&s.initial_value, &mut used_stores);
            }
            for default in comp.prop_defaults.values() {
                scan_expr(default, &mut used_stores);
            }
        }
        for store_ref in &comp.store_refs {
            if ir.has_store(store_ref.as_str()) && used_stores.contains(store_ref.as_str()) {
                w.emit_fmt(format_args!("import './{}_store.dart';", store_ref));
            }
        }
        for tag in &child_imports {
            w.emit_fmt(format_args!("import './{}.dart';", tag));
        }
        // Tagged unions referenced by this component's generated code live in
        // the shared types.dart (deferral #30) — import it when used. The
        // lib-fn scan is limited to the fns actually inlined here
        // (Task 5.8a: the previous whole-program scan made ~50 components
        // import types.dart without referencing it — unused_import).
        if types::component_uses_types(comp, ir, &inlined_lib_fns) {
            w.emit("import './types.dart';");
        }
        // Stdlib-driven imports (Task 5.8): the stdlib registry emits
        // top-level Dart names that need imports the generated files never
        // added (`sqrt`/`Random` → dart:math, `json.*` → dart:convert,
        // `http.*` → package:http, `SharedPreferences` → shared_preferences).
        if uses_dart_math {
            w.emit("import 'dart:math';");
        }
        if uses_dart_convert {
            w.emit("import 'dart:convert';");
        }
        if uses_http {
            w.emit("import 'package:http/http.dart' as http;");
        }
        if uses_prefs {
            w.emit("import 'package:shared_preferences/shared_preferences.dart';");
        }
        if uses_intl {
            w.emit("import 'package:intl/intl.dart';");
        }
        // Design classes/variants resolve against design.dart (AppDesign
        // factories, AppTokens constants) — import it when the view
        // references them and the program declares at least one design.
        // Task 5.8a: the check RESOLVES every reference against the design
        // IR (mirroring the decoration wrapper) — bare local class names
        // and unresolved classes no longer leave the import unused. Motion
        // color stops with `var(--token)` values also emit
        // `{Design}Tokens.<name>` refs into this component's initState
        // (AnimPropertiesDemo's textColorDemo) — a referenced motion using
        // tokens counts too.
        let uses_motion_tokens = ir.modules.values().any(|m| {
            m.motions.iter().any(|mo| {
                used_motion_names.contains(&mo.name)
                    && mo.body.stops.iter().any(|s| {
                        s.properties.values().any(|v| {
                            matches!(v, ExprIR::LiteralExpr { value } if value
                                .as_str()
                                .is_some_and(|s| s.trim().starts_with("var(--")))
                        })
                    })
            })
        });
        let design_resolver = design::DesignResolver::new(ir);
        if ir.modules.values().any(|m| !m.designs.is_empty())
            && (uses_motion_tokens
                || comp
                    .view
                    .as_ref()
                    .is_some_and(|v| view::view_uses_design_classes(v, &design_resolver)))
        {
            w.emit("import 'design.dart';");
        }
        // Trigger-helper runtime (Task 5.6): the Omni* wrapper classes live
        // ONCE in the shared _omniview_runtime.dart (emitted by the module
        // generator when any component uses the triggers) — import it here
        // instead of re-emitting ~90 lines of helper per component file.
        if view.is_some_and(view_uses_trigger_helpers) {
            w.emit("import './_omniview_runtime.dart';");
        }
        // Review LOW-1: the Focus(onKeyEvent:) wrapper's KeyDownEvent/
        // KeyUpEvent guards live in flutter/services.dart — material.dart
        // does not re-export them. Gated on key events that actually WIRE a
        // guard (a param-bearing fn-ref key handler degrades honestly and
        // emits no guard — no import, else unnecessary_import).
        if view.is_some_and(|v| {
            let widget_names: FxHashSet<String> = comp
                .props
                .keys()
                .cloned()
                .chain(comp.events.iter().map(|e| e.name.clone()))
                .collect();
            view_uses_wired_key_events(v, &widget_names, comp, ir)
        }) {
            w.emit("import 'package:flutter/services.dart';");
        }
        if !comp.store_refs.is_empty() || comp.load_intent.is_some() {
            w.emit("import 'package:flutter_riverpod/flutter_riverpod.dart';");
            w.emit("");
        }
        if !comp.store_refs.is_empty() || !child_imports.is_empty() {
            w.emit("");
        }

        // Ref usage analysis (Task 5.2) — computed BEFORE the top-level
        // emission below so the measure wrapper helper can be gated on it.
        let (ref_usage, mut ref_errors, dual_axis_warnings) = collect_ref_usage(comp);

        // v4 compiler HIGH-2: dual-axis scrollTo(x, y) silently drops x on
        // Flutter (single-axis scroll, vertical wins) — surface the
        // design-contract warning (设计文档 域 1 映射表) on the codegen
        // warnings channel, anchored at the component's span. Same
        // CodegenWarning::at conversion as `ViewContext::warn_at` (view.rs);
        // pushed here because ref analysis precedes view formatting (the
        // ViewContext does not exist yet at this point).
        for (msg, span) in &dual_axis_warnings {
            let (line, col) = span.map_or((0, 0), |s| (s.line, s.col));
            warnings.push(crate::codegen::CodegenWarning::at(
                msg.clone(),
                comp.source_file.clone(),
                line,
                col,
            ));
        }

        // Cross-file library functions inlined at top level (Task 5.8) —
        // pure-logic fns from other .omni files, emitted with Dart-typed
        // params/returns and async signatures when their bodies need `await`.
        for fun in &lib_fns {
            generate_lib_fn_flutter(&mut w, fun);
            w.emit("");
        }

        // DOMRect-parity measure wrapper (Task 5.8): `ref.measure()` emits
        // `_omni_measure(...)` — an x/y/width/height surface over the
        // Offset/Size pair — emitted when any ref uses Measure.
        // Task C2b (#7): CSS `steps(N, start|end)` easings map to a
        // per-component `_OmniStepsCurve` Curve subclass (Flutter has no
        // built-in discrete curve) — emitted when any used motion declares
        // a steps easing.
        let uses_steps_easing = ir.modules.values().any(|m| {
            m.motions.iter().any(|mo| {
                used_motion_names.contains(&mo.name)
                    && motion::motion_uses_steps_easing(&mo.body.ease)
            })
        });
        if uses_steps_easing {
            w.emit("class _OmniStepsCurve extends Curve {");
            w.indent();
            w.emit("    final int steps;");
            w.emit("    final bool start;");
            w.emit("    const _OmniStepsCurve(this.steps, {this.start = false});");
            w.emit("    @override");
            w.emit("    double transformInternal(double t) {");
            w.indent();
            w.emit("    if (t <= 0.0) return 0.0;");
            w.emit("    if (t >= 1.0) return 1.0;");
            w.emit("    final s = steps < 1 ? 1 : steps;");
            w.emit("    final step = t * s;");
            w.emit("    return (start ? step.ceil() / s : step.floor() / s).clamp(0.0, 1.0);");
            w.dedent();
            w.emit("    }");
            w.dedent();
            w.emit("}");
            w.emit("");
        }
        if ref_usage.values().any(|u| u.needs_measure) {
            w.emit("class _OmniMeasureResult {");
            w.indent();
            w.emit("    final Offset? _pos;");
            w.emit("    final Size? _size;");
            w.emit("    _OmniMeasureResult(this._pos, this._size);");
            // Non-nullable with a 0 fallback — the language's measure
            // surface is f64-typed (Task 5.8: boxWidth = w where w came
            // from a nullable getter was a double? -> double error).
            w.emit("    double get x => _pos?.dx ?? 0;");
            w.emit("    double get y => _pos?.dy ?? 0;");
            w.emit("    double get width => _size?.width ?? 0;");
            w.emit("    double get height => _size?.height ?? 0;");
            w.dedent();
            w.emit("}");
            w.emit("");
            w.emit("_OmniMeasureResult _omni_measure(RenderBox? box) => box == null");
            w.emit("    ? _OmniMeasureResult(null, null)");
            w.emit("    : _OmniMeasureResult(box.localToGlobal(Offset.zero), box.size);");
            w.emit("");
        }
        // @drag (ADR-073): the typed DragIntent carrier for the pan wiring —
        // handler bodies read `e.dx` etc. as dynamic dot-accesses, which
        // REQUIRE an object with real getters: a Dart `Map<String,num>`
        // literal has none (NoSuchMethodError on the first pan — Fix round
        // 1 of the ADR-073 review). Emitted per component (library-private
        // `_` name — no cross-file collision). Fix round 2: the declaration
        // is driven by the union of fields the component's drag handlers
        // actually read (a hardcoded all-7-fields carrier produced 5
        // `unused_element_parameter` warnings in dart analyze once a real
        // demo read only dx/dy); the class is skipped entirely when no
        // handler references it (all drag handlers zero-param ⇒ bare calls).
        let drag_spec = drag_event_class_spec(comp, ir);
        if drag_spec.needed {
            w.emit("class _DragEvent {");
            w.indent();
            for f in &drag_spec.fields {
                let ty = if f == "pointerId" { "int" } else { "double" };
                w.emit(&format!("    final {ty} {f};"));
            }
            if drag_spec.fields.is_empty() {
                // no handler reads any DragIntent field — the carrier is
                // only ever constructed as `_DragEvent()`
                w.emit("    const _DragEvent();");
            } else {
                w.emit("    const _DragEvent({");
                for f in &drag_spec.fields {
                    w.emit(&format!("        this.{f} = 0,"));
                }
                w.emit("    });");
            }
            w.dedent();
            w.emit("}");
            w.emit("");
        }

        // Store provider map (C4): store alias (as written in .omni source) →
        // Riverpod provider variable name (e.g. "TodoStore" → "todoStoreProvider").
        // INLINE stores (flattened into the component, never providers) map
        // to the EMPTY string sentinel — the store rewrite then emits the
        // bare field/action name (Task 5.8: InlineStoreDemo's
        // `CartStore.addItem(...)` referenced a provider that never exists).
        let store_provider_map: FxHashMap<String, String> = comp
            .store_refs
            .iter()
            .map(|s| {
                if ir.has_store(s.as_str()) {
                    (s.clone(), format!("{}Provider", lower_first(s)))
                } else {
                    (s.clone(), String::new())
                }
            })
            .collect();

        // The post-frame re-apply + controller wiring serve both semantic
        // bindings (focus/scroll) and bind:value text listeners (design D2).
        let needs_ref_setup = has_semantic_bindings(comp) || has_text_listeners(comp);
        let needs_text_listeners = has_text_listeners(comp);
        let refs_to_dispose = has_ref_dispose(comp, &ref_usage);

        // Per-element motion usage (T5.7): which elements reference which
        // motions, in DFS pre-order — drives the per-element controller
        // declarations/initState/dispose below AND prunes motions the
        // component never references (previously EVERY module motion got a
        // controller in EVERY component — 10k+ unused-field warnings and
        // duplicate definitions in the showcase). Computed BEFORE the
        // import emission so the design.dart gate can count motion token
        // refs (Task 5.8a).
        let has_motions = !motion_usage.is_empty();

        let state_name = format!("_{}State", name);

        // Determine widget/state base classes — @load components watch a
        // FutureProvider, so ConsumerState is required even without @store.
        let (widget_base, state_base) = if comp.store_refs.is_empty() && comp.load_intent.is_none()
        {
            ("StatefulWidget", "State")
        } else {
            ("ConsumerStatefulWidget", "ConsumerState")
        };

        // ── Widget class ──
        w.emit_fmt(format_args!("class {} extends {} {{", name, widget_base));
        w.indent();

        // Slot declarations (Task 5.3): `@slot` placeholders in the view —
        // and body-level `@slot` declarations (ADR-053 D5) — become `Widget?`
        // constructor params — same-named for named slots, `children` for
        // the unnamed slot. Consumer-side projections inside child
        // components are skipped (they are content, not declarations).
        // v2 (design D4): slots carry no type constraint — `Widget?` is the
        // only param type (slot = parent-provided widget content).
        let mut declared_slots: Vec<Option<String>> =
            comp.slots.iter().map(|s| s.name.clone()).collect();
        if let Some(ref view) = comp.view {
            view::collect_view_slots(view, component_map, &mut declared_slots);
        }
        // Collision guard + de-duplicate (mirrors the React backend): a slot
        // whose param name collides with a prop, event, signal, computed, or
        // fn would emit duplicate constructor params/fields — invalid
        // Dart — so it is skipped (the semantic layer warns on the
        // collision). The same slot name used twice emits a single param.
        let slot_collides: FxHashSet<&str> = comp
            .signals
            .iter()
            .map(|s| s.name.as_str())
            .chain(comp.computeds.iter().map(|c| c.name.as_str()))
            .chain(comp.fns.iter().map(|f| f.name.as_str()))
            .collect();
        let mut seen_slots: FxHashSet<String> = FxHashSet::default();
        declared_slots.retain(|n| {
            let slot_prop = n.clone().unwrap_or_else(|| "children".to_string());
            !comp.props.contains_key(&slot_prop)
                && !comp.events.iter().any(|e| e.name == slot_prop)
                && !slot_collides.contains(slot_prop.as_str())
                && seen_slots.insert(slot_prop)
        });

        // Widget-scope names (Task 5.8): props, events, and slots live on the
        // StatefulWidget — bare references from the State class (fn bodies,
        // computed getters, effects, lifecycles, view expressions) are
        // rewritten to `widget.<name>`. Previously props/events emitted bare
        // (`icon`, `label`, `onOptionalAck`) → undefined identifiers.
        let mut widget_names: FxHashSet<String> = comp
            .props
            .keys()
            .cloned()
            .chain(comp.events.iter().map(|e| e.name.clone()))
            .collect();
        for slot_name in &declared_slots {
            widget_names.insert(slot_name.clone().unwrap_or_else(|| "children".into()));
        }

        // Component-fn renames (Task 5.8): a fn whose name shadows a Dart
        // builtin TYPE used in the same class (`fn double()` → the State
        // class's `double randomValue = 0;` declaration could not resolve
        // the type) is emitted as `<name>_` and every reference is renamed.
        // Dart has no type qualification, so the member must move.
        let mut fn_renames: FxHashMap<String, String> = FxHashMap::default();
        for fun in &comp.fns {
            if DART_BUILTIN_TYPE_NAMES.contains(&fun.name.as_str()) {
                fn_renames.insert(fun.name.clone(), format!("{}_", fun.name));
            }
        }

        let has_events = !comp.events.is_empty();
        if comp.props.is_empty() && declared_slots.is_empty() && !has_events {
            w.emit_fmt(format_args!("const {}({{super.key}});", name));
        } else {
            // Constructor with named parameters for props + slots + events
            // (C3: `const CompName({`)
            w.emit_fmt(format_args!("const {}({{", name));
            w.indent();
            w.emit("super.key,");
            for prop_name in comp.props.keys() {
                w.emit_fmt(format_args!("required this.{},", prop_name));
            }
            for event in &comp.events {
                w.emit_fmt(format_args!("this.{},", event.name));
            }
            for slot_name in &declared_slots {
                let param_name = slot_name.as_deref().unwrap_or("children");
                w.emit_fmt(format_args!("this.{},", param_name));
            }
            w.dedent();
            w.emit("});");

            // Field declarations for props
            w.emit("");
            for (prop_name, prop_type) in &comp.props {
                let ann = super::dart::dart_type_ir(prop_type);
                w.emit_fmt(format_args!("final {} {};", ann, prop_name));
            }
            // Field declarations for events (Task 5.8): `event onSwitch: String`
            // → `final void Function(String)? onSwitch;` — consumers pass a
            // callback, emits forward through `widget.onSwitch?.call(...)`.
            for event in &comp.events {
                let payload = event_payload_dart_type(&event.param_type);
                w.emit_fmt(format_args!(
                    "final void Function({})? {};",
                    payload, event.name
                ));
            }
            // Field declarations for slots — v2 (design D4): the typed-slot
            // constraint/degradation path is gone; `Widget?` is the only
            // slot field type (slot = parent-provided widget content).
            for slot_name in &declared_slots {
                let param_name = slot_name.as_deref().unwrap_or("children");
                w.emit_fmt(format_args!("final Widget? {};", param_name));
            }
        }

        w.emit("");
        w.emit("@override");
        w.emit_fmt(format_args!(
            "{}<{}> createState() => {}();",
            state_base, name, state_name
        ));

        w.dedent();
        w.emit("}");
        w.emit("");

        // ── State class ──
        // T5.7: the TickerProvider mixin is needed only when THIS component
        // actually uses motions (previously any module-level motion forced
        // the mixin onto every component).
        let state_mixin = if has_motions {
            " with TickerProviderStateMixin".to_string()
        } else {
            String::new()
        };
        w.emit_fmt(format_args!(
            "class {} extends {}<{}>{} {{",
            state_name, state_base, name, state_mixin
        ));
        w.indent();

        // State variables as class fields. Task 5.8: an initializer that
        // references ANOTHER signal (`'Hello ' + userName + ...`) cannot run
        // in a plain field initializer (implicit_this_reference_in_initializer)
        // — `late` defers the evaluation until first access, which may read
        // sibling fields. Nullable annotations are preserved (dart_type_ir).
        let signal_names: FxHashSet<String> = comp.signals.iter().map(|s| s.name.clone()).collect();
        for state in &comp.signals {
            let ann = super::dart::dart_type_ir(&state.r#type);
            // Phase 3 (ADR-071 decision 2): D1/D2 signals are stored in
            // ValueNotifier cells — `final _dragX = ValueNotifier<double>(0);`
            // — and every identifier resolves through `_name.value` (the
            // initializer can reference sibling D1/D2 cells, so it formats
            // with the set-aware formatter).
            if direct_write_names.contains(&state.name) {
                let init = format_expr_ir_flutter_dw(&state.initial_value, &direct_write_names);
                let mut refs = FxHashSet::default();
                collect_expr_identifiers(&state.initial_value, &mut refs);
                let cross_refs_sibling = refs
                    .iter()
                    .any(|r| r != &state.name && signal_names.contains(r));
                if cross_refs_sibling {
                    // A field initializer referencing sibling fields cannot
                    // run at construction (implicit_this_reference_in_initializer)
                    // — `late` defers to first access, by which time every
                    // notifier cell exists.
                    w.emit_fmt(format_args!(
                        "late final _{} = ValueNotifier<{}>({});",
                        state.name, ann, init
                    ));
                } else {
                    w.emit_fmt(format_args!(
                        "final _{} = ValueNotifier<{}>({});",
                        state.name, ann, init
                    ));
                }
                continue;
            }
            // Initializers may reference D1/D2 sibling cells (`a = dragX + 1`
            // — signal initializers are not view reads, so dragX stays D1) —
            // format with the set-aware formatter in both branches.
            let init = format_expr_ir_flutter_dw(&state.initial_value, &direct_write_names);
            let mut refs = FxHashSet::default();
            collect_expr_identifiers(&state.initial_value, &mut refs);
            let cross_refs_sibling = refs
                .iter()
                .any(|r| r != &state.name && signal_names.contains(r));
            if cross_refs_sibling {
                w.emit_fmt(format_args!("late {} {} = {};", ann, state.name, init));
            } else {
                w.emit_fmt(format_args!("{} {} = {};", ann, state.name, init));
            }
        }
        if !comp.signals.is_empty() {
            w.emit("");
        }

        // @form (Task 5): one GlobalKey drives Form validation and submit
        // interception. The `form.*` builtin signals degrade to reachability-
        // gated constants below — the Flutter backend validates natively
        // (validator → errorText), so the 语言面 signal reads compile but hold
        // static values (see form_builtin_usage scan scope).
        if comp.form_intent.is_some() {
            w.emit("final _formKey = GlobalKey<FormState>();");
            w.emit("");
        }

        // @load (Task 5): the FutureProvider owns the fetch lifecycle; the
        // build method hoists `ref.watch(...)` into `{fn}Async` so loading/
        // error/data map onto AsyncValue's tri-state.
        let load_ret = comp.load_intent.as_ref().map(|li| {
            comp.fns
                .iter()
                .find(|f| f.name == li.fn_name)
                .and_then(|f| f.return_type.as_ref())
                .map(super::dart::dart_type_ir)
                .unwrap_or_else(|| "Object".to_string())
        });
        if let (Some(li), Some(ret)) = (&comp.load_intent, &load_ret) {
            w.emit_fmt(format_args!(
                "late final {} = FutureProvider<{}>((ref) async => {}());",
                load_provider_name(&li.fn_name),
                ret,
                li.fn_name
            ));
            w.emit("");
        }

        // Reachability-gated `form.valid/errors/submitting` constants — only
        // emitted when the view actually reads the builtin (else dart analyze
        // flags them unused; form_builtin_usage scans props, semantic
        // bindings, inline handlers at all depths, and interpolations).
        let form_usage = form_builtin_usage(comp);
        if form_usage.valid {
            w.emit("static const formValid = true;");
            w.emit("// 语言面信号保留 — Flutter 端校验经 errorText 展示");
            w.emit("");
        }
        if form_usage.errors {
            w.emit("static const formErrors = <String, String>{};");
            w.emit("// 语言面信号保留 — Flutter 校验错误经 errorText 展示");
            w.emit("");
        }
        if form_usage.submitting {
            w.emit("static const formSubmitting = false;");
            w.emit("// @load 三态经 FutureProvider — form.submitting 恒为 false");
            w.emit("");
        }

        // Ref declarations: GlobalKey + on-demand controllers (Task 5.2)
        emit_ref_declarations(&mut w, comp, &ref_usage);
        if !comp.refs.is_empty() {
            w.emit("");
        }

        // @drag (ADR-073): the GestureDetector pan callbacks share ONE
        // State field snapshotting the drag start — onPanStart writes it,
        // onPanUpdate/onPanEnd read it (Flutter's class-instance state
        // space; the element has no DOM attributes).
        if has_drag_bindings(comp) {
            w.emit("Offset? _dragStart;");
            w.emit("");
        }

        // Effect dependency snapshot fields (C2): previous values of non-prop
        // deps so didUpdateWidget can detect changes and re-run @effect bodies.
        let mut prev_fields: FxHashSet<String> = FxHashSet::default();
        for effect in &comp.effects {
            // An unresolvable dep (store action, unknown name) collapses the
            // didUpdateWidget condition to `if (true)` (Task 5.8a
            // conservative re-run) — the effect never reads its prev
            // fields, so emitting them would trigger dart unused_field
            // (FilterBar: @effect (searchText, setFilter)).
            if effect_collapses_to_true(comp, effect) {
                continue;
            }
            for dep in &effect.deps {
                if comp.props.contains_key(dep) || !prev_fields.insert(dep.clone()) {
                    continue;
                }
                if let Some(dep_type) = find_dep_type(comp, dep) {
                    let ann = super::dart::dart_type_ir(dep_type);
                    w.emit_fmt(format_args!("late {} _prev{};", ann, dep));
                }
            }
        }
        if !prev_fields.is_empty() {
            w.emit("");
        }

        // Semantic binding snapshot fields (Task 5.6): previous values of
        // focus/scroll/value binding expressions so `_syncSemanticBindings`
        // re-applies only when a bound value CHANGED (review HIGH-2 — the
        // previous unconditional re-apply ran after every rebuild, yanking
        // focus on unrelated setState and replaying scroll animations).
        if needs_ref_setup && emit_semantic_binding_prev_fields(&mut w, comp, ir) {
            w.emit("");
        }

        // AnimationController/Animation field declarations for motion animations.
        // T5.7: PER ELEMENT INSTANCE — one controller per (element, motion)
        // pair (`_{motion}Controller_{idx}`), one Animation<T> per animated
        // property per element (`_{motion}{Prop}_{idx}`), and a per-element
        // `_{motion}Active_{idx}` gate for the animated wrappers (while false
        // the element keeps its natural style — fill: none/backwards reset to
        // natural on completion instead of holding the 0% keyframe). Only
        // motions the component's view actually references are declared.
        for usage in &motion_usage {
            for (motion_name, _) in &usage.motions {
                let Some(motion) = find_module_motion(ir, motion_name) else {
                    continue;
                };
                let ctrl_name = motion_ctrl_name(motion_name, usage.idx);
                w.emit_fmt(format_args!(
                    "late final AnimationController {};",
                    ctrl_name
                ));
                for prop in flutter_motion_properties(&motion.body) {
                    let field = prop_anim_field_for(motion_name, &prop, usage.idx);
                    let ann = prop_anim_type(&prop);
                    w.emit_fmt(format_args!("late final Animation<{}> {};", ann, field));
                }
                if !flutter_motion_properties(&motion.body).is_empty() {
                    w.emit_fmt(format_args!(
                        "bool {} = false;",
                        motion_active_name(motion_name, usage.idx)
                    ));
                }
            }
        }
        if has_motions {
            w.emit("");
        }

        // Component-level type scope (signals + computeds) — Task 5.8a
        // static-type re-annotation. Built before the computed loop so
        // computed expressions (which reference signals/computeds) are
        // annotated too; also used by effect/lifecycle bodies below and
        // threaded into ViewContext for view-tree expressions.
        let comp_type_scope = type_annotate::build_comp_type_scope(comp);

        // Computed properties as getters (or methods for the function-form
        // `computed name() -> Type { ... }` — Task 5.8: the old emission put
        // the statement block into a getter body where the block-expression
        // formatter dropped the statements and returned `null` →
        // return_of_invalid_type + the view's `fullName()` call could not
        // invoke a getter).
        for computed in &comp.computeds {
            let ret_type = super::dart::dart_type_ir(&computed.r#type);
            let is_fn_form = matches!(
                &computed.body,
                ExprIR::BlockExpr { statements, .. }
                    if statements.iter().any(|s| matches!(s, StatementIR::ReturnStmtIR { .. }))
            );
            // Task 3.7 (Riverpod 3.0 scope compliance): store reads inside
            // CLOSURE bodies in the computed expression (e.g. TodoList's
            // `array_filter(TodoStore.items, |item| ...)` predicate) are
            // lazy — hoist them into locals FIRST (the walk needs the
            // original store-access nodes), then apply the standard store
            // rewrite. The locals are emitted at the top of the getter/fn
            // body, so the closure references them instead of a
            // `ref.watch` that could fire outside the build phase.
            let mut hoist_next = 0usize;
            let mut hoister = ClosureHoister::new(&store_provider_map, ir, false, &mut hoist_next);
            // Task 5.8a: re-annotate the computed expression from the
            // component's signals/computeds (same rationale as fn bodies —
            // the IR's type_ann is never populated). A typed Map signal's
            // `contains` would otherwise fall to the runtime `is Map`
            // dispatch — unnecessary_type_check under dart analyze.
            let computed_body =
                type_annotate::annotate_expr_types(&computed.body, &comp_type_scope);
            let (computed_body, hoisted_locals) = hoister.hoist(&computed_body);
            let rewritten = rewrite_store_accesses(&computed_body, &store_provider_map, ir);
            let rewritten = rewrite_widget_refs(&rewritten, &widget_names, &fn_renames);
            if is_fn_form {
                w.emit_fmt(format_args!("{} {}() {{", ret_type, computed.name));
                w.indent();
                for (name, read) in &hoisted_locals {
                    w.emit_fmt(format_args!(
                        "final {} = {};",
                        name,
                        format_expr_ir_flutter(read)
                    ));
                }
                if let ExprIR::BlockExpr { statements, .. } = &rewritten {
                    for stmt in statements {
                        generate_statement_flutter_dw(&mut w, stmt, &direct_write_names);
                    }
                }
                w.dedent();
                w.emit("}");
            } else {
                w.emit_fmt(format_args!("{} get {} {{", ret_type, computed.name));
                w.indent();
                for (name, read) in &hoisted_locals {
                    w.emit_fmt(format_args!(
                        "final {} = {};",
                        name,
                        format_expr_ir_flutter(read)
                    ));
                }
                w.emit_fmt(format_args!(
                    "return {};",
                    format_expr_ir_flutter(&rewritten)
                ));
                w.dedent();
                w.emit("}");
            }
            w.emit("");
        }

        // initState (for mount lifecycle, effects, and motion animations)
        let has_mount = comp
            .lifecycles
            .iter()
            .any(|lc| lc.kind == LifecycleKind::Mount);
        let has_effects = !comp.effects.is_empty();
        if has_mount || has_effects || has_motions || needs_text_listeners {
            w.emit("@override");
            w.emit("void initState() {");
            w.indent();
            w.emit("super.initState();");

            // Create the PER-ELEMENT AnimationControllers and per-property
            // Animations for each (element, motion) the view references
            // (T5.7). Motion-level diagnostics are deduped per motion name.
            let mut diag_seen: FxHashSet<String> = FxHashSet::default();
            for usage in &motion_usage {
                for (motion_name, has_enter) in &usage.motions {
                    let Some(motion) = find_module_motion(ir, motion_name) else {
                        continue;
                    };
                    let ctrl_name = motion_ctrl_name(motion_name, usage.idx);
                    let kf = &motion.body;
                    let duration_ms = extract_duration_ms(&kf.over);
                    let ease_str = format_easing_flutter(&kf.ease);

                    // Motion-level diagnostics (Task 5.4): steps() easing has
                    // no Flutter Curves equivalent; width/height are
                    // layout-dependent; skew and unknown properties are
                    // skipped.
                    if diag_seen.insert(motion_name.clone()) {
                        // C2d span threading: anchor at the motion
                        // definition's span in its REAL source file — the
                        // module key is a per-file placeholder ("input.omni",
                        // parser/mod.rs) and must not reach diagnostics;
                        // `source_file` is patched at IR merge
                        // (analyse.rs merge_file_ir_into, same pattern as
                        // component/store/design).
                        let motion_def = ir
                            .modules
                            .values()
                            .flat_map(|m| &m.motions)
                            .find(|mm| mm.name == *motion_name);
                        let (motion_file, m_line, m_col) = motion_def
                            .and_then(|mm| mm.source_span.map(|s| (s.line, s.col)))
                            .map_or((None, 0, 0), |(l, c)| {
                                (motion_def.and_then(|mm| mm.source_file.clone()), l, c)
                            });
                        let anchor = |msg: String| {
                            crate::codegen::CodegenWarning::at(
                                msg,
                                motion_file.clone(),
                                m_line,
                                m_col,
                            )
                        };
                        if let Some(diag) = easing_flutter_diagnostic(motion_name, &kf.ease) {
                            warnings.push(anchor(diag));
                        }
                        for prop in extract_keyframe_properties(kf) {
                            if let Some(diag) = prop_flutter_diagnostic(motion_name, &prop) {
                                warnings.push(anchor(diag));
                            }
                        }
                    }

                    // Create AnimationController
                    w.emit_fmt(format_args!(
                        "{0} = AnimationController(\n  duration: Duration(milliseconds: {1}),\n  vsync: this,\n);",
                        ctrl_name, duration_ms
                    ));
                    // Per-property Animation<T>: Tween (2 stops) or
                    // TweenSequence (3+ stops), chained with the motion-level
                    // curve. Color tweens use CurvedAnimation — CurveTween is
                    // Animatable<double> and cannot chain onto Tween<Color>.
                    // `direction: reverse` with a loop swaps the tween
                    // direction (Flutter can only repeat forward passes).
                    let reverse_tweens = kf.playback.loop_count.is_some()
                        && matches!(kf.playback.direction, Some(DirectionValue::Reverse));
                    for prop in flutter_motion_properties(kf) {
                        let field = prop_anim_field_for(motion_name, &prop, usage.idx);
                        let tween = build_prop_tween_expr_with(kf, &prop, reverse_tweens, Some(ir));
                        if motion_prop_kind(&prop) == MotionPropKind::Color {
                            w.emit_fmt(format_args!(
                                "{0} = {1}.animate(CurvedAnimation(parent: {2}, curve: {3}));",
                                field, tween, ctrl_name, ease_str
                            ));
                        } else {
                            w.emit_fmt(format_args!(
                                "{0} = {1}.chain(CurveTween(curve: {2})).animate({3});",
                                field, tween, ease_str, ctrl_name
                            ));
                        }
                    }

                    // @enter trigger: start playback honoring
                    // loop_count/direction/fill/delay (and gate the animated
                    // wrappers on the active flag)
                    if *has_enter {
                        let active_name = (!flutter_motion_properties(kf).is_empty())
                            .then(|| motion_active_name(motion_name, usage.idx));
                        for stmt in
                            emit_playback_start(&ctrl_name, active_name.as_deref(), &kf.playback)
                        {
                            w.emit(&stmt);
                        }
                    }
                }
            }

            // Mount lifecycle
            for lc in &comp.lifecycles {
                if lc.kind == LifecycleKind::Mount {
                    let mut rewritten: Vec<StatementIR> = lc
                        .body
                        .statements
                        .iter()
                        .map(|s| {
                            let s0 = type_annotate::annotate_stmt_types(s, &comp_type_scope);
                            let s1 = rewrite_stmt_store_accesses(&s0, &store_provider_map, ir);
                            rewrite_stmt_widget_refs(&s1, &widget_names, &fn_renames)
                        })
                        .collect();
                    // Pointer-capture ref ops hard-throw — later statements
                    // are unreachable (dead_code in dart analyze, Task 5.8a).
                    stmt::truncate_after_pointer_capture(&mut rewritten);
                    if stmt_block_emits_await(&rewritten) {
                        // `initState` cannot be async — storage/http calls
                        // whose Flutter emission embeds `await` run inside an
                        // immediately-invoked async closure (Task 5.8:
                        // UserSettings' storage_get in @mount).
                        w.emit("() async {");
                        w.indent();
                        for stmt in &rewritten {
                            generate_statement_flutter_dw(&mut w, stmt, &direct_write_names);
                        }
                        w.dedent();
                        w.emit("}();");
                    } else {
                        for stmt in &rewritten {
                            generate_statement_flutter_dw(&mut w, stmt, &direct_write_names);
                        }
                    }
                }
            }
            // Effects — run once on mount (C2: real _runEffect_N methods,
            // re-run from didUpdateWidget when deps change)
            for (i, _effect) in comp.effects.iter().enumerate() {
                w.emit_fmt(format_args!("_runEffect_{}();", i));
            }
            // Semantic ref bindings: guarded value listeners (user edits →
            // signal via setState, Task 5.2)
            if needs_text_listeners {
                w.emit("");
                emit_semantic_binding_listeners(&mut w, comp);
            }
            w.dedent();
            w.emit("}");
            w.emit("");
        }

        // didUpdateWidget — re-run @effect bodies whose deps changed (C2).
        // The parameter uses the concrete widget type (legal override of
        // `State<T>.didUpdateWidget(covariant T oldWidget)`), so prop deps can
        // be read via `oldWidget.<prop>`; current prop values via `widget.<prop>`.
        if has_effects {
            w.emit("@override");
            w.emit_fmt(format_args!("void didUpdateWidget({} oldWidget) {{", name));
            w.indent();
            w.emit("super.didUpdateWidget(oldWidget);");
            for (i, effect) in comp.effects.iter().enumerate() {
                let mut conditions: Vec<String> = Vec::new();
                for dep in &effect.deps {
                    if comp.props.contains_key(dep) {
                        // Prop deps: compare against the previous widget
                        conditions.push(format!("oldWidget.{} != widget.{}", dep, dep));
                    } else if find_dep_type(comp, dep).is_some() {
                        // Signal/computed deps: compare against the last
                        // observed value (snapshotted at the end of each run).
                        // A D1/D2 dep resolves through its notifier cell
                        // (`_dep.value` — the State field was renamed `_dep`;
                        // bare `dep` would be an undeclared identifier).
                        // Note: a D1/D2 mutation does not rebuild, so this
                        // comparison alone never re-triggers — the dep is
                        // effectively inert (documented corner, ADR-071 P1).
                        let rhs = if direct_write_names.contains(dep) {
                            format!("_{}.value", dep)
                        } else {
                            dep.clone()
                        };
                        conditions.push(format!("_prev{} != {}", dep, rhs));
                    } else {
                        // Unresolvable dep — conservatively re-run
                        conditions.push("true".to_string());
                    }
                }
                if conditions.is_empty() {
                    continue; // no deps — runs once in initState only
                }
                // Task 5.8a: a single unresolvable dep already makes the
                // whole condition always-true — `if (true || ...)` flagged
                // the right operand as dead_code in dart analyze
                // (NotificationsPanel:43). Collapse to `if (true)`.
                if conditions.iter().any(|c| c == "true") {
                    conditions = vec!["true".to_string()];
                }
                w.emit_fmt(format_args!("if ({}) {{", conditions.join(" || ")));
                w.indent();
                w.emit_fmt(format_args!("_runEffect_{}();", i));
                w.dedent();
                w.emit("}");
            }
            w.dedent();
            w.emit("}");
            w.emit("");
        }

        // dispose (for unmount lifecycle and motion controllers)
        let has_unmount = comp
            .lifecycles
            .iter()
            .any(|lc| lc.kind == LifecycleKind::Unmount);
        if has_unmount || has_motions || refs_to_dispose {
            w.emit("@override");
            w.emit("void dispose() {");
            w.indent();
            for lc in &comp.lifecycles {
                if lc.kind == LifecycleKind::Unmount {
                    let mut rewritten: Vec<StatementIR> = lc
                        .body
                        .statements
                        .iter()
                        .map(|s| {
                            let s0 = type_annotate::annotate_stmt_types(s, &comp_type_scope);
                            let s1 = rewrite_stmt_store_accesses(&s0, &store_provider_map, ir);
                            rewrite_stmt_widget_refs(&s1, &widget_names, &fn_renames)
                        })
                        .collect();
                    stmt::truncate_after_pointer_capture(&mut rewritten);
                    for stmt in &rewritten {
                        generate_statement_flutter_dw(&mut w, stmt, &direct_write_names);
                    }
                }
            }
            // Dispose the per-element motion AnimationControllers (T5.7)
            for usage in &motion_usage {
                for (motion_name, _) in &usage.motions {
                    if find_module_motion(ir, motion_name).is_none() {
                        continue;
                    }
                    let ctrl_name = motion_ctrl_name(motion_name, usage.idx);
                    w.emit_fmt(format_args!("{}.dispose();", ctrl_name));
                }
            }
            // Dispose on-demand ref controllers (Task 5.2)
            emit_ref_dispose(&mut w, comp, &ref_usage);
            w.emit("super.dispose();");
            w.dedent();
            w.emit("}");
            w.emit("");
        }

        // _syncSemanticBindings — re-applies focus/scroll/value bindings.
        // Registered as a post-frame callback from build (below), so it runs
        // after every rebuild (Task 5.2, reactive parity with React's
        // useEffect-on-deps).
        if needs_ref_setup {
            emit_semantic_binding_sync(&mut w, comp, &store_provider_map, ir);
            w.emit("");
        }

        // Functions as methods on State (Task 5.8: bodies are rewritten for
        // store accesses → ref.watch/ref.read, widget-scope props/events →
        // widget.<name>, and component-fn renames before emission).
        for func in &comp.fns {
            let type_scope = type_annotate::build_fn_type_scope(comp, func);
            let mut rewritten: Vec<StatementIR> = func
                .body
                .statements
                .iter()
                .map(|s| {
                    // Task 5.8a: re-annotate IdentifierExpr type_ann (the
                    // IR's annotation is never populated — lower resolves
                    // against fresh scopes) so the shared Dart method table
                    // can shape get/contains/concat from the static types.
                    let s0 = type_annotate::annotate_stmt_types(s, &type_scope);
                    let s1 = rewrite_stmt_store_accesses(&s0, &store_provider_map, ir);
                    rewrite_stmt_widget_refs(&s1, &widget_names, &fn_renames)
                })
                .collect();
            // Pointer-capture ref ops hard-throw — later statements are
            // unreachable (dead_code in dart analyze, Task 5.8a:
            // ElementRefDemo's onPtrDown/onPtrUp).
            stmt::truncate_after_pointer_capture(&mut rewritten);
            // Task 5.8: a fn renamed for Dart-type shadowing (`double` →
            // `double_`) must be DECLARED under the renamed name too — the
            // reference rewrite alone left the old declaration shadowing
            // the type (StatsDashboard's `double randomValue`).
            let mut func = func.clone();
            if let Some(renamed) = fn_renames.get(&func.name) {
                func.name = renamed.clone();
            }
            generate_function_flutter_with(&mut w, &func, &rewritten, &direct_write_names);
            w.emit("");
        }

        // @effect method bodies (C2) — Task 5.8: rewritten for store/widget
        // refs, and emitted `async` when the body needs `await`.
        // Task 3.7: `_runEffect_N` runs from initState / didUpdateWidget —
        // OUTSIDE the build phase, where Riverpod 3.0 forbids `ref.watch`.
        // Reads become `ref.read` (the callback-legal form); the effect is
        // re-driven by its deps (didUpdateWidget comparison), so
        // watch-style reactivity is not needed inside the body.
        for (i, effect) in comp.effects.iter().enumerate() {
            let mut rewritten: Vec<StatementIR> = effect
                .body
                .statements
                .iter()
                .map(|s| {
                    let s0 = type_annotate::annotate_stmt_types(s, &comp_type_scope);
                    let s1 = rewrite_stmt_store_accesses(&s0, &store_provider_map, ir);
                    let s1 = watch_to_read_stmt(&s1);
                    rewrite_stmt_widget_refs(&s1, &widget_names, &fn_renames)
                })
                .collect();
            stmt::truncate_after_pointer_capture(&mut rewritten);
            let is_async = stmt_block_emits_await(&rewritten);
            if is_async {
                w.emit_fmt(format_args!("Future<void> _runEffect_{}() async {{", i));
            } else {
                w.emit_fmt(format_args!("void _runEffect_{}() {{", i));
            }
            w.indent();
            for stmt in &rewritten {
                generate_statement_flutter_dw(&mut w, stmt, &direct_write_names);
            }
            // Snapshot non-prop deps for the didUpdateWidget comparison
            // (skipped entirely when the effect collapses to `if (true)` —
            // the fields do not exist and the values are never compared).
            if !effect_collapses_to_true(comp, effect) {
                for dep in &effect.deps {
                    if !comp.props.contains_key(dep) && find_dep_type(comp, dep).is_some() {
                        // Phase 3: a D1/D2 dep snapshots its notifier cell —
                        // `_prevX = _X.value;` (the field was renamed `_X`).
                        let rhs = if direct_write_names.contains(dep) {
                            format!("_{}.value", dep)
                        } else {
                            dep.clone()
                        };
                        w.emit_fmt(format_args!("_prev{} = {};", dep, rhs));
                    }
                }
            }
            w.dedent();
            w.emit("}");
            w.emit("");
        }

        // Build method
        w.emit("@override");
        w.emit("Widget build(BuildContext context) {");
        w.indent();

        // Semantic ref bindings are re-applied after every frame: any
        // setState-driven signal change triggers a rebuild whose post-frame
        // callback re-runs _syncSemanticBindings() (Task 5.2). The mounted
        // guard prevents a use-after-dispose crash on fast unmount.
        if needs_ref_setup {
            w.emit("WidgetsBinding.instance.addPostFrameCallback((_) {");
            w.indent();
            w.emit("if (!mounted) return;");
            w.emit("_syncSemanticBindings();");
            w.dedent();
            w.emit("});");
        }

        // @load (Task 5): hoisted AsyncValue read — `loading` maps to
        // `.isLoading`, `error` to `.error`, `data` to `.valueOrNull` in the
        // view tree (format_identifier override in ref.rs).
        if let (Some(li), Some(ret)) = (&comp.load_intent, &load_ret) {
            w.emit_fmt(format_args!(
                "final AsyncValue<{}> {} = ref.watch({});",
                ret,
                load_async_local_name(&li.fn_name),
                load_provider_name(&li.fn_name)
            ));
            w.emit("");
        }

        if let Some(view) = view {
            let motions = ir
                .modules
                .values()
                .flat_map(|m| &m.motions)
                .cloned()
                .collect::<Vec<_>>();
            let mut motion_counter = 0usize;
            // Task 3.7: hoisted store reads collected while formatting the
            // view tree — emitted as `final __omniN = ref.watch(...);`
            // build locals below so lazy closures (`.where` predicates,
            // ListView itemBuilder bodies) reference locals by capture
            // instead of watching outside the build phase.
            let mut hoisted: Vec<(String, String)> = Vec::new();
            let mut hoist_next = 0usize;
            let mut ctx = view::ViewContext {
                component_map,
                store_provider_map: &store_provider_map,
                ref_usage: &ref_usage,
                component_slots,
                ir,
                design: &design_resolver,
                warnings: &mut warnings,
                // C2d span threading: view warnings anchor at the element's
                // span in the component's source file.
                file: comp.source_file.clone(),
                motion_counter: &mut motion_counter,
                widget_names: &widget_names,
                fn_renames: &fn_renames,
                comp,
                hoisted: &mut hoisted,
                hoist_next: &mut hoist_next,
                lazy_context: false,
                comp_type_scope: &comp_type_scope,
                // Phase 3: D1/D2 set — inline-handler formatters inside the
                // view tree emit `_name.value` mutations (no setState).
                direct_write_signals: &direct_write_names,
                // Task 14: unmappable direct-write style values fail
                // compilation (diagnostics merged into ref_errors below) —
                // never a silent drop.
                errors: &mut view_errors,
                direct_write: &direct_write_sets,
                // Task 5: @load/@form context for view-tree builtin mapping.
                load_async_name: comp
                    .load_intent
                    .as_ref()
                    .map(|li| load_async_local_name(&li.fn_name)),
                form_present: comp.form_intent.is_some(),
            };
            // Multi-node root fragments are wrapped in a Column —
            // `return Text('a'), Text('b');` would be invalid Dart
            // (format_single_view is the Flutter analogue of React's
            // fragment wrapper).
            // Task 3.9: a component-root conditional in the PAGE-SWITCH
            // shape (every branch a component instance) becomes an
            // IndexedStack so tab switches keep the outgoing page mounted
            // (state preserved) — non-page shapes fall back to the ternary.
            let tree_expr = match view {
                ViewTree::Conditional(cond) => view::format_page_switcher(cond, &motions, &mut ctx)
                    .unwrap_or_else(|| format_single_view(view, &motions, &mut ctx)),
                _ => format_single_view(view, &motions, &mut ctx),
            };
            // F1: input-bearing components get a Material shell at the root
            // (bare TextField renders ~100k px tall without one — 3.2
            // review finding; page shells are the 3.9 Scaffold task's).
            let tree_expr = view::wrap_root_material_if_inputs(view, tree_expr);
            for (name, expr) in &hoisted {
                w.emit_fmt(format_args!("final {} = {};", name, expr));
            }
            w.emit_fmt(format_args!("return {};", tree_expr));
        } else {
            // Default widget tree
            w.emit("return Container(");
            w.indent();
            w.emit_fmt(format_args!("child: Text('{}'),", name));
            w.dedent();
            w.emit(");");
        }

        w.dedent();
        w.emit("}");

        w.dedent();
        w.emit("}");

        // Motion diagnostics are module-global and emitted once per
        // component — dedupe so multi-component modules report each once.
        // Keyed by MESSAGE (not the whole warning): identical messages from
        // different element spans collapse to the first occurrence, keeping
        // the pre-C2d per-component semantics unchanged.
        let mut seen: FxHashSet<String> = FxHashSet::default();
        warnings.retain(|w| seen.insert(w.message.clone()));

        // Task 14: view-formatting errors (unmappable direct-write style
        // values) join the ref errors on the compile-error channel.
        ref_errors.append(&mut view_errors);

        (w.into_string(), warnings, ref_errors)
    }

    /// Generate a store .dart file with Riverpod 3.0 providers.
    ///
    /// Emits an immutable state class with `copyWith`, computed getters, a
    /// `Notifier<T>` subclass with actions as methods, and a
    /// `NotifierProvider<NotifierT, StateT>`.
    fn gen_store_dart(&self, name: &str, store: &StoreIR, ir: &IRProgram) -> String {
        let mut w = CodeWriter::new();
        w.emit_fmt(format_args!(
            "// Generated by OmniLang \u{2014} Store: {}",
            name
        ));
        w.emit("");

        if store.signals.is_empty() && store.getters.is_empty() {
            w.emit("import 'package:flutter_riverpod/flutter_riverpod.dart';");
            w.emit("");
            w.emit("// Empty store \u{2014} no signals, getters, or actions to emit");
            return w.into_string();
        }

        w.emit("import 'package:flutter_riverpod/flutter_riverpod.dart';");
        // Tagged unions referenced by this store's generated code live in the
        // shared types.dart (deferral #30) — import it when used.
        if types::store_uses_types(store, ir) {
            w.emit("import './types.dart';");
        }
        // Stdlib-driven imports (Task 5.8): store actions/getters call
        // storage_* (SharedPreferences), json_*, math_* — the registry emits
        // top-level names that need imports. Task 5.8a: imports are emitted
        // ONCE per store from the scan flags — the previous inline emission
        // inside the scan duplicated `shared_preferences` when several
        // actions used storage_* (NotificationStore: duplicate_import).
        let mut uses_prefs = false;
        let mut uses_convert = false;
        let mut uses_math = false;
        let mut scan_store_expr = |e: &ExprIR| {
            uses_prefs |= expr_uses_stdlib_fns(e, DART_PREFS_STDLIB_FNS);
            uses_convert |= expr_uses_stdlib_fns(e, DART_CONVERT_STDLIB_FNS);
            uses_math |= expr_uses_stdlib_fns(e, DART_MATH_STDLIB_FNS);
        };
        for expr in store.getters.values() {
            scan_store_expr(expr);
        }
        for a in &store.actions {
            for s in &a.body.statements {
                for e in statement_exprs(s) {
                    scan_store_expr(e);
                }
            }
        }
        if uses_prefs {
            w.emit("import 'package:shared_preferences/shared_preferences.dart';");
        }
        if uses_convert {
            w.emit("import 'dart:convert';");
        }
        if uses_math {
            w.emit("import 'dart:math';");
        }
        w.emit("");

        let state_name = format!("{}State", name);
        let provider_name = format!("{}Provider", lower_first(name));

        // ── Immutable state class ──
        w.emit_fmt(format_args!("/// Immutable state for {}.", name));
        w.emit_fmt(format_args!("class {} {{", state_name));
        w.indent();

        // Raw signal fields
        for (sig_name, sig_type) in &store.signals {
            let ann = super::dart::dart_type_ir(sig_type);
            w.emit_fmt(format_args!("final {} {};", ann, sig_name));
        }
        if !store.signals.is_empty() {
            w.emit("");
        }

        // Computed getters on the state class (referenced via `s.<getter>` in
        // `provider.select((s) => s.<getter>)` from components)
        for (getter_name, getter_expr) in &store.getters {
            let ret_type = store
                .getter_types
                .get(getter_name)
                .map(super::dart::dart_type_ir)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "dynamic".into());
            w.emit_fmt(format_args!("/// Computed getter: {}.", getter_name));
            w.emit_fmt(format_args!(
                "{} get {} => {};",
                ret_type,
                getter_name,
                format_expr_ir_flutter(getter_expr)
            ));
        }
        if !store.getters.is_empty() {
            w.emit("");
        }

        // Constructor
        if store.signals.is_empty() {
            w.emit_fmt(format_args!("const {}();", state_name));
        } else {
            w.emit_fmt(format_args!("const {}({{", state_name));
            w.indent();
            for sig_name in store.signals.keys() {
                w.emit_fmt(format_args!("required this.{},", sig_name));
            }
            w.dedent();
            w.emit("});");
        }

        // Equality (T5.7): `==`/`hashCode` over ALL signal fields with a
        // runtimeType guard — the Riverpod-docs pattern (no Equatable
        // dependency). `provider.select((s) => s.field)` consumers compare
        // state instances; identity-only comparison made every state change
        // look like a change even when the values were equal.
        w.emit("");
        w.emit("@override");
        w.emit("bool operator ==(Object other) =>");
        w.indent();
        let mut eq_lines: Vec<String> = vec![
            "identical(this, other)".into(),
            format!("other is {}", state_name),
            "runtimeType == other.runtimeType".into(),
        ];
        eq_lines.extend(
            store
                .signals
                .keys()
                .map(|sig| format!("{} == other.{}", sig, sig)),
        );
        for (i, line) in eq_lines.iter().enumerate() {
            let suffix = if i == 0 {
                " ||"
            } else if i + 1 == eq_lines.len() {
                ";"
            } else {
                " &&"
            };
            w.emit_fmt(format_args!("{}{}", line, suffix));
        }
        w.dedent();
        w.emit("");
        w.emit("@override");
        // `Object.hash` requires TWO+ positional args in Dart — single-field
        // states hash the field directly; empty states fall back to the
        // runtimeType (consistent with the == runtimeType guard).
        let hash_fields: Vec<String> = store.signals.keys().cloned().collect();
        let hash_expr = match hash_fields.len() {
            0 => "runtimeType.hashCode".to_string(),
            1 => format!("{}.hashCode", hash_fields[0]),
            _ => format!("Object.hash({})", hash_fields.join(", ")),
        };
        w.emit_fmt(format_args!("int get hashCode => {};", hash_expr));

        // copyWith — sentinel-based (T5.7): `T? x = _unset` with an
        // `== _unset ? this.x : x as T` restore. The old `x ?? this.x`
        // form could never set a NULLABLE field back to null (null meant
        // "keep"), and a raw `T? x` param made `copyWith(note: null)`
        // ambiguous. The private `_unset` const disambiguates "not passed"
        // from "explicitly null".
        w.emit("");
        w.emit("static const _unset = Object();");
        w.emit("");
        w.emit_fmt(format_args!("{} copyWith({{", state_name));
        w.indent();
        for sig_name in store.signals.keys() {
            w.emit_fmt(format_args!("Object? {} = _unset,", sig_name));
        }
        w.dedent();
        w.emit("}) {");
        w.indent();
        if store.signals.is_empty() {
            w.emit_fmt(format_args!("return {}();", state_name));
        } else {
            w.emit_fmt(format_args!("return {}(", state_name));
            w.indent();
            for (sig_name, sig_type) in &store.signals {
                let ann = super::dart::dart_type_ir(sig_type);
                w.emit_fmt(format_args!(
                    "{}: {} == _unset ? this.{} : {} as {},",
                    sig_name, sig_name, sig_name, sig_name, ann
                ));
            }
            w.dedent();
            w.emit(");");
        }
        w.dedent();
        w.emit("}");
        w.dedent();
        w.emit("}");
        w.emit("");

        // ── Notifier class ──
        w.emit_fmt(format_args!("/// Riverpod 3.0 Notifier for {}.", name));
        w.emit_fmt(format_args!(
            "class {} extends Notifier<{}> {{",
            name, state_name
        ));
        w.indent();

        w.emit("@override");
        w.emit_fmt(format_args!("{} build() {{", state_name));
        w.indent();
        let defaults: Vec<String> = store
            .signals
            .keys()
            .map(|sig| {
                let v = store
                    .signal_defaults
                    .get(sig)
                    .map(|v| format_json_value(v, &JSON_FLUTTER))
                    .unwrap_or_else(|| "null".into());
                format!("{}: {}", sig, v)
            })
            .collect();
        w.emit_fmt(format_args!(
            "return const {}({});",
            state_name,
            defaults.join(", ")
        ));
        w.dedent();
        w.emit("}");
        w.emit("");

        // ── Actions as Notifier methods ──
        for action in &store.actions {
            // Task 5.8: use the IR return type (actions with `: String`
            // returns previously emitted `void add(...)` → the body's
            // `return 'msg'` failed to type-check) and place the `async`
            // keyword AFTER the name (the old
            // `Future<void> async removeLast()` put the keyword in
            // parameter position — a parse error). Actions whose body
            // needs `await` (storage_set → SharedPreferences) are made
            // async automatically.
            let is_async = action.is_async || stmt_block_emits_await(&action.body.statements);
            let ret = action
                .return_type
                .as_ref()
                .filter(|rt| !matches!(rt.kind, TypeKind::Void))
                .map(super::dart::dart_type_ir)
                .unwrap_or_else(|| "void".into());
            let prefix = if is_async {
                format!("Future<{ret}>")
            } else {
                ret
            };
            let async_kw = if is_async { " async" } else { "" };
            let mut params: Vec<String> = Vec::new();
            for p in &action.params {
                let ty = p
                    .r#type
                    .as_ref()
                    .map(super::dart::dart_type_ir)
                    .unwrap_or_else(|| "dynamic".into());
                params.push(format!("{} {}", ty, p.name));
            }
            w.emit_fmt(format_args!(
                "{} {}({}){} {{",
                prefix,
                action.name,
                params.join(", "),
                async_kw
            ));
            w.indent();
            // Action params shadow same-named store signals (fn_locals
            // pattern, OmniView store.rs): a bare `items` in the body is the
            // param, so it stays bare instead of `state.items`
            // (maintainability HIGH-1).
            let param_names: FxHashSet<String> =
                action.params.iter().map(|p| p.name.clone()).collect();
            for stmt in &action.body.statements {
                generate_store_action_stmt(&mut w, stmt, store, &param_names);
            }
            w.dedent();
            w.emit("}");
            w.emit("");
        }

        w.dedent();
        w.emit("}");
        w.emit("");

        // ── Provider ──
        w.emit_fmt(format_args!(
            "final {} = NotifierProvider<{}, {}>({}.new);",
            provider_name, name, state_name, name
        ));
        w.into_string()
    }
}

// Resolve the type of an @effect dependency: a component signal or computed.
// True when an @effect's didUpdateWidget condition collapses to `if (true)`
// (Task 5.8a): any dep that is neither a prop nor a component signal/computed
// (e.g. a store action like `setFilter`) forces the conservative re-run —
// and its prev-value snapshot fields are never read, so they must not be
// emitted (dart unused_field) nor assigned (undefined name).
// ── Task 5 (@load) naming —───────────────
// The load fn `fetchTodos` becomes the State field
// `final fetchTodosProvider = FutureProvider<...>((ref) async => fetchTodos());`
// and the build hoists `final AsyncValue<...> fetchTodosAsync = ref.watch(...);`.
// Both names are mechanical (fn name + suffix) so any @load fn maps
// predictably; the async local is what builtins (loading/error/data) and the
// retry handler reference.

/// AsyncValue local name hoisted in build() (`{fn}Async`).
pub(crate) fn load_async_local_name(fn_name: &str) -> String {
    format!("{fn_name}Async")
}

/// FutureProvider State-field name (`{fn}Provider`).
pub(crate) fn load_provider_name(fn_name: &str) -> String {
    format!("{fn_name}Provider")
}

fn effect_collapses_to_true(comp: &ComponentIR, effect: &EffectIntent) -> bool {
    effect
        .deps
        .iter()
        .any(|d| !comp.props.contains_key(d) && find_dep_type(comp, d).is_none())
}

fn find_dep_type<'a>(comp: &'a ComponentIR, dep: &str) -> Option<&'a TypeIR> {
    comp.signals
        .iter()
        .find(|s| s.name == dep)
        .map(|s| &s.r#type)
        .or_else(|| {
            comp.computeds
                .iter()
                .find(|c| c.name == dep)
                .map(|c| &c.r#type)
        })
}

/// Map a component event's payload type string to a Dart type annotation.
/// `EventDeclIR.param_type` holds the source-level type ("String", "i32",
/// "String?", "string | undefined", ...) — unknown/empty types degrade to
/// `dynamic`, and a `void` payload means NO parameter (`void Function()`).
fn event_payload_dart_type(param_type: &str) -> String {
    let nullable = param_type.ends_with('?') || param_type.ends_with(" | undefined");
    let base = param_type
        .trim_end_matches('?')
        .trim_end_matches(" | undefined");
    let mapped = match base {
        "" | "unknown" | "any" => "dynamic".to_string(),
        "void" => "".to_string(),
        "String" | "string" | "str" => "String".to_string(),
        "i32" | "i64" | "u32" | "u64" | "int" | "isize" | "usize" | "number" => "int".to_string(),
        "f32" | "f64" | "float" | "double" => "double".to_string(),
        "bool" => "bool".to_string(),
        other => other.to_string(),
    };
    if nullable && !mapped.ends_with('?') {
        format!("{mapped}?")
    } else {
        mapped
    }
}

/// Dart builtin type names that a component member could shadow (a fn named
/// `double` makes `double randomValue = 0;` fail to resolve the type).
const DART_BUILTIN_TYPE_NAMES: &[&str] = &[
    "double",
    "int",
    "num",
    "String",
    "bool",
    "void",
    "dynamic",
    "Object",
    "List",
    "Map",
    "Set",
    "Future",
    "Function",
    "Never",
    "Null",
    "DateTime",
    "Duration",
    "Iterable",
    "Stream",
    "Record",
    "Symbol",
    "Type",
    "Error",
    "Exception",
];

/// Material widget/class names that collide with generated component class
/// names — hidden from the material.dart import in files importing both
/// (Task 5.8: App.dart's `TabBar`/`DataTable` were ambiguous).
const MATERIAL_WIDGET_CLASS_NAMES: &[&str] = &[
    "TabBar",
    "DataTable",
    "Table",
    "Form",
    "Card",
    "Icon",
    "ModalBarrier",
    "Radio",
    "Checkbox",
    "Switch",
    "Slider",
    "ListTile",
    "Divider",
    "AppBar",
    "Button",
    "Text",
    "TextField",
    "Menu",
    "Tooltip",
    "Badge",
    "Chip",
    "Drawer",
    "SnackBar",
    "Dialog",
    "AlertDialog",
    "BottomSheet",
    "ExpansionTile",
    "NavigationRail",
    "Hero",
    "Banner",
    "Image",
    "Tab",
    "ProgressIndicator",
    "RefreshIndicator",
    "PopupMenuButton",
    "DropdownButton",
    "InkWell",
    "GestureDetector",
    "AnimatedContainer",
    "AnimatedOpacity",
    "AnimatedSwitcher",
    "FadeTransition",
    "SlideTransition",
    "ScaleTransition",
    "Positioned",
    "Transform",
    "Opacity",
    "Visibility",
    "Spacer",
    "Center",
    "Padding",
    "SizedBox",
    "Expanded",
    "Flexible",
    "Row",
    "Column",
    "Stack",
    "ListView",
    "GridView",
    "SingleChildScrollView",
    "CustomScrollView",
    "Scaffold",
    "Container",
    "Semantics",
    "ExcludeSemantics",
    "Focus",
    "Builder",
    "FutureBuilder",
    "StreamBuilder",
    "PageView",
    "TabBarView",
    "TabController",
    "DefaultTabController",
];

/// Stdlib function names whose Flutter emission is a top-level `dart:math`
/// name (`sqrt`, `Random`, `pi`, `log`, ...) — the file needs
/// `import 'dart:math';` (Task 5.8: StatsDashboard's `sqrt(counter)` /
/// `Random().nextDouble()` produced undefined names).
const DART_MATH_STDLIB_FNS: &[&str] = &[
    "math_sqrt",
    "math_pow",
    "math_random",
    "math_random_int",
    "math_sin",
    "math_cos",
    "math_tan",
    "math_log",
    "math_log2",
    "math_log10",
    "math_pi",
    "math_e",
];

/// Stdlib function names whose Flutter emission is `json.*` /
/// `base64Encode` / `utf8.encode` — the file needs
/// `import 'dart:convert';` (Task 5.8: encode_demo's base64/utf8 helpers
/// were undefined names).
const DART_CONVERT_STDLIB_FNS: &[&str] = &[
    "json_parse",
    "json_stringify",
    "encode_base64_encode",
    "encode_base64_decode",
    "encode_hex_encode",
    "encode_hex_decode",
    "encode_url_encode",
    "encode_url_decode",
];

/// Stdlib function names whose Flutter emission is `DateFormat(...)` — the
/// file needs `import 'package:intl/intl.dart';`.
const DART_INTL_STDLIB_FNS: &[&str] = &["datetime_format"];

/// Stdlib function names whose Flutter emission is `http.*` — the file
/// needs `import 'package:http/http.dart' as http;`.
const DART_HTTP_STDLIB_FNS: &[&str] = &[
    "http_get",
    "http_post",
    "http_put",
    "http_delete",
    "http_request",
    "http_patch",
    "http_head",
];

/// Stdlib function names whose Flutter emission uses `SharedPreferences` —
/// the file needs `import 'package:shared_preferences/shared_preferences.dart';`.
const DART_PREFS_STDLIB_FNS: &[&str] = &["storage_get", "storage_set"];

/// Collect bare identifier references from a statement block.
fn collect_stmt_identifiers(stmt: &StatementIR, acc: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::LetStmtIR { value: Some(v), .. } => collect_expr_identifiers(v, acc),
        StatementIR::ExprStmtIR { expr } => collect_expr_identifiers(expr, acc),
        StatementIR::ReturnStmtIR { value: Some(v), .. } => collect_expr_identifiers(v, acc),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_expr_identifiers(condition, acc);
            for s in &then_body.statements {
                collect_stmt_identifiers(s, acc);
            }
            for (c, b) in else_ifs {
                collect_expr_identifiers(c, acc);
                for s in &b.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
            if let Some(b) = else_body {
                for s in &b.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_expr_identifiers(condition, acc);
            for s in &body.statements {
                collect_stmt_identifiers(s, acc);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_expr_identifiers(iterable, acc);
            for s in &body.statements {
                collect_stmt_identifiers(s, acc);
            }
        }
        StatementIR::MutationStmt { intent: Some(i) } => collect_mutation_identifiers(i, acc),
        StatementIR::EventEmitStmtIR { args, .. } => {
            for a in args {
                collect_expr_identifiers(a, acc);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_identifiers(s, acc);
            }
        }
        StatementIR::ThrowStmtIR { expr } => collect_expr_identifiers(expr, acc),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_identifiers(s, acc);
            }
            if let Some(b) = catch_body {
                for s in &b.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
            if let Some(b) = finally_body {
                for s in &b.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

fn collect_mutation_identifiers(intent: &MutationIntent, acc: &mut FxHashSet<String>) {
    match intent {
        MutationIntent::DirectAssign { value: Some(v), .. } => collect_expr_identifiers(v, acc),
        MutationIntent::Increment {
            amount: Some(v), ..
        }
        | MutationIntent::Decrement {
            amount: Some(v), ..
        }
        | MutationIntent::Multiply {
            amount: Some(v), ..
        }
        | MutationIntent::Divide {
            amount: Some(v), ..
        }
        | MutationIntent::Modulo {
            amount: Some(v), ..
        } => collect_expr_identifiers(v, acc),
        MutationIntent::Push { item: Some(v), .. } => collect_expr_identifiers(v, acc),
        MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Collect bare identifier references from an expression.
fn collect_expr_identifiers(expr: &ExprIR, acc: &mut FxHashSet<String>) {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => {
            acc.insert(name.clone());
        }
        ExprIR::CallExpr { name, args } => {
            acc.insert(name.clone());
            for a in args {
                collect_expr_identifiers(a, acc);
            }
        }
        ExprIR::FieldAccessExpr { object, .. } => collect_expr_identifiers(object, acc),
        ExprIR::OptionalChainExpr { object, .. } => collect_expr_identifiers(object, acc),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_expr_identifiers(left, acc);
            collect_expr_identifiers(right, acc);
        }
        ExprIR::UnaryOpExpr { operand, .. } => collect_expr_identifiers(operand, acc),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr_identifiers(cond, acc);
            collect_expr_identifiers(then_expr, acc);
            collect_expr_identifiers(else_expr, acc);
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                collect_expr_identifiers(e, acc);
            }
        }
        ExprIR::SpreadExpr { operand } => collect_expr_identifiers(operand, acc),
        ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                collect_expr_identifiers(v, acc);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for a in call_args {
                collect_expr_identifiers(a, acc);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            collect_expr_identifiers(object, acc);
            for a in args {
                collect_expr_identifiers(a, acc);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for s in statements {
                collect_stmt_identifiers(s, acc);
            }
            collect_expr_identifiers(tail, acc);
        }
        ExprIR::IndexExpr { object, index } => {
            collect_expr_identifiers(object, acc);
            collect_expr_identifiers(index, acc);
        }
        ExprIR::AwaitExpr { operand } => collect_expr_identifiers(operand, acc),
        ExprIR::ClosureExpr { body, .. } => collect_expr_identifiers(body, acc),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                collect_expr_identifiers(v, acc);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_expr_identifiers(matched, acc);
            for arm in arms {
                for s in &arm.body.statements {
                    collect_stmt_identifiers(s, acc);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_expr_identifiers(k, acc);
                collect_expr_identifiers(v, acc);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_expr_identifiers(subject, acc);
            collect_expr_identifiers(predicate, acc);
        }
        ExprIR::RefOpExpr { ref_name, op } => {
            // v4 compiler HIGH-1: 记录 ref_name（本意），同时下钻 op 参数
            // ——参数内调用的库函数/标识符此前逃逸收集（lib-fn 不内联 →
            // 生成 Dart 未定义名）。经 expr::map_ref_op_args 统一下钻。
            acc.insert(ref_name.clone());
            let mut f = |e: &ExprIR| {
                collect_expr_identifiers(e, acc);
                e.clone()
            };
            map_ref_op_args(op, &mut f);
        }
        ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Collect bare identifier references from the view tree (fns called in
/// handlers/bindings/conditions).
fn collect_view_identifiers(node: &ViewTree, acc: &mut FxHashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for v in el.props.values() {
                collect_expr_identifiers(v, acc);
            }
            for h in el.events.values() {
                match h {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        acc.insert(r#ref.clone());
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        acc.insert(r#ref.clone());
                    }
                    EventHandlerIR::InlineFn { body, .. } => {
                        for s in &body.statements {
                            collect_stmt_identifiers(s, acc);
                        }
                    }
                }
            }
            for v in el.bindings.values() {
                acc.insert(v.name.clone());
            }
            if let Some(b) = &el.ref_binding {
                acc.insert(b.clone());
            }
            for v in el.class_map_bindings.values() {
                collect_expr_identifiers(v, acc);
            }
            for child in &el.children {
                collect_view_identifiers(child, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_expr_identifiers(&branch.condition, acc);
                if let Some(b) = &branch.body {
                    collect_view_identifiers(b, acc);
                }
            }
            if let Some(fb) = &cond.fallback {
                collect_view_identifiers(fb, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            collect_expr_identifiers(&list.iterable, acc);
            if let Some(b) = &list.body {
                collect_view_identifiers(b, acc);
            }
            if let Some(b) = &list.empty_body {
                collect_view_identifiers(b, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for c in children {
                collect_view_identifiers(c, acc);
            }
        }
        // Task 5.8: interpolations `{demo_remove([1, 2, 3], 0)}` call lib
        // fns — they were previously skipped, so the inlined fn set missed
        // ~40 of the stdlib-demo helpers.
        ViewTree::Interpolation(expr) => collect_expr_identifiers(expr, acc),
        ViewTree::Text(_) | ViewTree::Slot { .. } => {}
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// All expressions appearing anywhere in a statement (for stdlib-import
/// scanning).
fn statement_exprs(stmt: &StatementIR) -> Vec<&ExprIR> {
    let mut out: Vec<&ExprIR> = Vec::new();
    fn push_expr<'a>(expr: &'a ExprIR, out: &mut Vec<&'a ExprIR>) {
        out.push(expr);
    }
    fn walk_stmt<'a>(stmt: &'a StatementIR, out: &mut Vec<&'a ExprIR>) {
        match stmt {
            StatementIR::LetStmtIR { value: Some(v), .. } => push_expr(v, out),
            StatementIR::ExprStmtIR { expr } => push_expr(expr, out),
            StatementIR::ReturnStmtIR { value: Some(v), .. } => push_expr(v, out),
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                push_expr(condition, out);
                for s in &then_body.statements {
                    walk_stmt(s, out);
                }
                for (c, b) in else_ifs {
                    push_expr(c, out);
                    for s in &b.statements {
                        walk_stmt(s, out);
                    }
                }
                if let Some(b) = else_body {
                    for s in &b.statements {
                        walk_stmt(s, out);
                    }
                }
            }
            StatementIR::WhileStmtIR { condition, body } => {
                push_expr(condition, out);
                for s in &body.statements {
                    walk_stmt(s, out);
                }
            }
            StatementIR::ForInStmtIR { iterable, body, .. } => {
                push_expr(iterable, out);
                for s in &body.statements {
                    walk_stmt(s, out);
                }
            }
            StatementIR::MutationStmt { intent: Some(i) } => match i {
                MutationIntent::DirectAssign { value: Some(v), .. }
                | MutationIntent::Increment {
                    amount: Some(v), ..
                }
                | MutationIntent::Decrement {
                    amount: Some(v), ..
                }
                | MutationIntent::Multiply {
                    amount: Some(v), ..
                }
                | MutationIntent::Divide {
                    amount: Some(v), ..
                }
                | MutationIntent::Modulo {
                    amount: Some(v), ..
                }
                | MutationIntent::Push { item: Some(v), .. } => push_expr(v, out),
                MutationIntent::Pop { .. } => {}
                #[allow(unreachable_patterns)]
                _ => {}
            },
            StatementIR::EventEmitStmtIR { args, .. } => {
                for a in args {
                    push_expr(a, out);
                }
            }
            StatementIR::BlockStmtIR { body } => {
                for s in &body.statements {
                    walk_stmt(s, out);
                }
            }
            StatementIR::ThrowStmtIR { expr } => push_expr(expr, out),
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                for s in &try_body.statements {
                    walk_stmt(s, out);
                }
                if let Some(b) = catch_body {
                    for s in &b.statements {
                        walk_stmt(s, out);
                    }
                }
                if let Some(b) = finally_body {
                    for s in &b.statements {
                        walk_stmt(s, out);
                    }
                }
            }
            StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    walk_stmt(stmt, &mut out);
    out
}

/// All expressions appearing anywhere in the view tree (for stdlib-import
/// scanning).
fn view_exprs(node: &ViewTree) -> Vec<&ExprIR> {
    let mut out: Vec<&ExprIR> = Vec::new();
    fn walk<'a>(node: &'a ViewTree, out: &mut Vec<&'a ExprIR>) {
        match node {
            ViewTree::Element(el) => {
                for v in el.props.values() {
                    out.push(v);
                }
                for h in el.events.values() {
                    match h {
                        EventHandlerIR::InlineFn { body, .. } => {
                            for s in &body.statements {
                                for e in statement_exprs(s) {
                                    out.push(e);
                                }
                            }
                        }
                        EventHandlerIR::FnRef { .. } | EventHandlerIR::StoreAction { .. } => {}
                    }
                }
                for v in el.class_map_bindings.values() {
                    out.push(v);
                }
                for c in &el.children {
                    walk(c, out);
                }
            }
            ViewTree::Conditional(cond) => {
                for b in &cond.branches {
                    out.push(&b.condition);
                    if let Some(body) = &b.body {
                        walk(body, out);
                    }
                }
                if let Some(fb) = &cond.fallback {
                    walk(fb, out);
                }
            }
            ViewTree::ListRendering(list) => {
                out.push(&list.iterable);
                if let Some(b) = &list.body {
                    walk(b, out);
                }
                if let Some(b) = &list.empty_body {
                    walk(b, out);
                }
            }
            ViewTree::Fragment(children) => {
                for c in children {
                    walk(c, out);
                }
            }
            ViewTree::Interpolation(expr) => out.push(expr),
            ViewTree::Text(_) | ViewTree::Slot { .. } => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    walk(node, &mut out);
    out
}

/// Does the expression reference a stdlib fn from `names`?
/// Collect every store alias referenced by an expression
/// (`StoreFieldAccessExpr.store_alias` — the ONLY way generated code
/// references a store provider; Task 5.8a gates the per-store imports on
/// actual usage, so a declared-but-unreferenced `@store` (main.omni's App)
/// no longer leaves the import unused).
fn collect_expr_store_aliases(expr: &ExprIR, out: &mut FxHashSet<String>) {
    match expr {
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            call_args,
            ..
        } => {
            out.insert(store_alias.clone());
            for a in call_args {
                collect_expr_store_aliases(a, out);
            }
        }
        ExprIR::FieldAccessExpr { object, .. } => {
            // Store property reads lower to FieldAccessExpr on the store
            // identifier (`DynamicTokenStore.liveColor` — only the CALL
            // form becomes StoreFieldAccessExpr) — the object's name is a
            // candidate alias.
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                out.insert(name.clone());
            }
            collect_expr_store_aliases(object, out);
        }
        ExprIR::OptionalChainExpr { object, .. } => collect_expr_store_aliases(object, out),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_expr_store_aliases(left, out);
            collect_expr_store_aliases(right, out);
        }
        ExprIR::UnaryOpExpr { operand, .. } => collect_expr_store_aliases(operand, out),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr_store_aliases(cond, out);
            collect_expr_store_aliases(then_expr, out);
            collect_expr_store_aliases(else_expr, out);
        }
        ExprIR::CallExpr { args, .. } => {
            for a in args {
                collect_expr_store_aliases(a, out);
            }
        }
        ExprIR::ArrayExpr { elements } => {
            for e in elements {
                collect_expr_store_aliases(e, out);
            }
        }
        ExprIR::SpreadExpr { operand } => collect_expr_store_aliases(operand, out),
        ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                collect_expr_store_aliases(v, out);
            }
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            collect_expr_store_aliases(object, out);
            for a in args {
                collect_expr_store_aliases(a, out);
            }
        }
        ExprIR::BlockExpr { statements, tail } => {
            for s in statements {
                collect_stmt_store_aliases(s, out);
            }
            collect_expr_store_aliases(tail, out);
        }
        ExprIR::IndexExpr { object, index } => {
            collect_expr_store_aliases(object, out);
            collect_expr_store_aliases(index, out);
        }
        ExprIR::AwaitExpr { operand } => collect_expr_store_aliases(operand, out),
        ExprIR::ClosureExpr { body, .. } => collect_expr_store_aliases(body, out),
        ExprIR::VariantLiteral { fields, .. } => {
            for (_, v) in fields {
                collect_expr_store_aliases(v, out);
            }
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_expr_store_aliases(matched, out);
            for a in arms {
                for s in &a.body.statements {
                    collect_stmt_store_aliases(s, out);
                }
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_expr_store_aliases(k, out);
                collect_expr_store_aliases(v, out);
            }
        }
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_expr_store_aliases(subject, out);
            collect_expr_store_aliases(predicate, out);
        }
        // v4 compiler HIGH-1: ref-op 参数内的 store 访问也必须计入 alias
        // 收集——否则 store import 不发射，重写后的 `ref.read(dataStoreProvider
        // ...)` 在 App.dart 里 `Undefined name 'dataStoreProvider'`（dart
        // analyze 实证）。经 expr::map_ref_op_args 统一下钻（同构纪律）。
        ExprIR::RefOpExpr { op, .. } => {
            let mut f = |e: &ExprIR| {
                collect_expr_store_aliases(e, out);
                e.clone()
            };
            map_ref_op_args(op, &mut f);
        }
        _ => {}
    }
}

fn collect_stmt_store_aliases(stmt: &StatementIR, out: &mut FxHashSet<String>) {
    match stmt {
        StatementIR::LetStmtIR { value: Some(v), .. } => collect_expr_store_aliases(v, out),
        StatementIR::ExprStmtIR { expr } => collect_expr_store_aliases(expr, out),
        StatementIR::ReturnStmtIR { value: Some(v), .. } => collect_expr_store_aliases(v, out),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_expr_store_aliases(condition, out);
            for s in &then_body.statements {
                collect_stmt_store_aliases(s, out);
            }
            for (c, b) in else_ifs {
                collect_expr_store_aliases(c, out);
                for s in &b.statements {
                    collect_stmt_store_aliases(s, out);
                }
            }
            if let Some(b) = else_body {
                for s in &b.statements {
                    collect_stmt_store_aliases(s, out);
                }
            }
        }
        StatementIR::MutationStmt { intent: Some(i) } => {
            for e in mutation_intent_exprs(i) {
                collect_expr_store_aliases(e, out);
            }
        }
        StatementIR::EventEmitStmtIR { args, .. } => {
            for a in args {
                collect_expr_store_aliases(a, out);
            }
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_store_aliases(s, out);
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            collect_expr_store_aliases(condition, out);
            for s in &body.statements {
                collect_stmt_store_aliases(s, out);
            }
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_expr_store_aliases(iterable, out);
            for s in &body.statements {
                collect_stmt_store_aliases(s, out);
            }
        }
        StatementIR::ThrowStmtIR { expr } => collect_expr_store_aliases(expr, out),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_store_aliases(s, out);
            }
            if let Some(b) = catch_body {
                for s in &b.statements {
                    collect_stmt_store_aliases(s, out);
                }
            }
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => {}
        // StatementIR is #[non_exhaustive] — cross-crate matches need a
        // wildcard.
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// The value expressions of a mutation intent (shared walk helper for the
/// store-alias collection).
fn mutation_intent_exprs(i: &MutationIntent) -> Vec<&ExprIR> {
    let mut out = Vec::new();
    match i {
        MutationIntent::DirectAssign { value: Some(v), .. } => out.push(v.as_ref()),
        MutationIntent::Increment {
            amount: Some(v), ..
        }
        | MutationIntent::Decrement {
            amount: Some(v), ..
        }
        | MutationIntent::Multiply {
            amount: Some(v), ..
        }
        | MutationIntent::Divide {
            amount: Some(v), ..
        }
        | MutationIntent::Modulo {
            amount: Some(v), ..
        } => out.push(v.as_ref()),
        MutationIntent::Push { item: Some(v), .. } => out.push(v.as_ref()),
        MutationIntent::Pop { .. } => {}
        // MutationIntent is #[non_exhaustive] — cross-crate matches need a
        // wildcard.
        #[allow(unreachable_patterns)]
        _ => {}
    }
    out
}

fn expr_uses_stdlib_fns(expr: &ExprIR, names: &[&str]) -> bool {
    match expr {
        ExprIR::CallExpr { name, args } => {
            names.contains(&name.as_str()) || args.iter().any(|a| expr_uses_stdlib_fns(a, names))
        }
        ExprIR::FieldAccessExpr { object, .. } => expr_uses_stdlib_fns(object, names),
        ExprIR::OptionalChainExpr { object, .. } => expr_uses_stdlib_fns(object, names),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            expr_uses_stdlib_fns(left, names) || expr_uses_stdlib_fns(right, names)
        }
        ExprIR::UnaryOpExpr { operand, .. } => expr_uses_stdlib_fns(operand, names),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_uses_stdlib_fns(cond, names)
                || expr_uses_stdlib_fns(then_expr, names)
                || expr_uses_stdlib_fns(else_expr, names)
        }
        ExprIR::ArrayExpr { elements } => elements.iter().any(|e| expr_uses_stdlib_fns(e, names)),
        ExprIR::SpreadExpr { operand } => expr_uses_stdlib_fns(operand, names),
        ExprIR::StructExpr { fields, .. } => {
            fields.iter().any(|(_, v)| expr_uses_stdlib_fns(v, names))
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            call_args.iter().any(|a| expr_uses_stdlib_fns(a, names))
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            expr_uses_stdlib_fns(object, names)
                || args.iter().any(|a| expr_uses_stdlib_fns(a, names))
        }
        ExprIR::BlockExpr { statements, tail } => {
            statements.iter().any(|s| stmt_uses_stdlib_fns(s, names))
                || expr_uses_stdlib_fns(tail, names)
        }
        ExprIR::IndexExpr { object, index } => {
            expr_uses_stdlib_fns(object, names) || expr_uses_stdlib_fns(index, names)
        }
        ExprIR::AwaitExpr { operand } => expr_uses_stdlib_fns(operand, names),
        ExprIR::ClosureExpr { body, .. } => expr_uses_stdlib_fns(body, names),
        ExprIR::VariantLiteral { fields, .. } => {
            fields.iter().any(|(_, v)| expr_uses_stdlib_fns(v, names))
        }
        ExprIR::MatchExpr { matched, arms } => {
            expr_uses_stdlib_fns(matched, names)
                || arms.iter().any(|a| {
                    a.body
                        .statements
                        .iter()
                        .any(|s| stmt_uses_stdlib_fns(s, names))
                })
        }
        ExprIR::MapExpr { entries } => entries
            .iter()
            .any(|(k, v)| expr_uses_stdlib_fns(k, names) || expr_uses_stdlib_fns(v, names)),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => expr_uses_stdlib_fns(subject, names) || expr_uses_stdlib_fns(predicate, names),
        _ => false,
    }
}

fn stmt_uses_stdlib_fns(stmt: &StatementIR, names: &[&str]) -> bool {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => value
            .as_ref()
            .is_some_and(|v| expr_uses_stdlib_fns(v, names)),
        StatementIR::ExprStmtIR { expr } => expr_uses_stdlib_fns(expr, names),
        StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .is_some_and(|v| expr_uses_stdlib_fns(v, names)),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            expr_uses_stdlib_fns(condition, names)
                || then_body
                    .statements
                    .iter()
                    .any(|s| stmt_uses_stdlib_fns(s, names))
                || else_ifs.iter().any(|(c, b)| {
                    expr_uses_stdlib_fns(c, names)
                        || b.statements.iter().any(|s| stmt_uses_stdlib_fns(s, names))
                })
                || else_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(|s| stmt_uses_stdlib_fns(s, names)))
        }
        StatementIR::WhileStmtIR { condition, body } => {
            expr_uses_stdlib_fns(condition, names)
                || body
                    .statements
                    .iter()
                    .any(|s| stmt_uses_stdlib_fns(s, names))
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            expr_uses_stdlib_fns(iterable, names)
                || body
                    .statements
                    .iter()
                    .any(|s| stmt_uses_stdlib_fns(s, names))
        }
        StatementIR::MutationStmt { intent } => intent.as_ref().is_some_and(|i| match i {
            MutationIntent::DirectAssign { value, .. }
            | MutationIntent::Increment { amount: value, .. }
            | MutationIntent::Decrement { amount: value, .. }
            | MutationIntent::Multiply { amount: value, .. }
            | MutationIntent::Divide { amount: value, .. }
            | MutationIntent::Modulo { amount: value, .. }
            | MutationIntent::Push { item: value, .. } => value
                .as_ref()
                .is_some_and(|v| expr_uses_stdlib_fns(v, names)),
            MutationIntent::Pop { .. } => false,
            #[allow(unreachable_patterns)]
            _ => false,
        }),
        StatementIR::EventEmitStmtIR { args, .. } => {
            args.iter().any(|a| expr_uses_stdlib_fns(a, names))
        }
        StatementIR::BlockStmtIR { body } => body
            .statements
            .iter()
            .any(|s| stmt_uses_stdlib_fns(s, names)),
        StatementIR::ThrowStmtIR { expr } => expr_uses_stdlib_fns(expr, names),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body
                .statements
                .iter()
                .any(|s| stmt_uses_stdlib_fns(s, names))
                || catch_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(|s| stmt_uses_stdlib_fns(s, names)))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| b.statements.iter().any(|s| stmt_uses_stdlib_fns(s, names)))
        }
        StatementIR::BreakStmtIR | StatementIR::ContinueStmtIR => false,
        #[allow(unreachable_patterns)]
        _ => false,
    }
}
