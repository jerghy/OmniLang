//! View tree formatting for Flutter codegen.

use crate::codegen::json::escape_literal_str;
use crate::codegen::shared::direct_write::{
    match_style_pattern, DirectWriteEntry, DirectWriteSets, StylePattern,
};
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde_json::Value;
use std::cell::Cell;

use super::capitalize_first;
use super::design::DesignResolver;
use super::design::{
    class_factory_expr, decoration_fields_to_expr, merge_decoration_fields,
    resolve_class_decoration, resolve_variant_decoration, DecorationFields,
};
use super::design_classes::resolve_style_rule_decoration;
use super::design_tokens::dart_string_literal;
use super::expr::{
    format_expr_ir_flutter, format_expr_ir_flutter_ctx, rewrite_stmt_store_accesses,
    rewrite_stmt_widget_refs, rewrite_store_accesses, rewrite_widget_refs, watch_to_read_stmt,
    ClosureHoister,
};
use super::motion::{
    is_flutter_motion_element, motion_active_name, motion_ctrl_name, wrap_with_motion_widgets,
};
use super::motion_widgets::{emit_event_object_flutter, field_extract_flutter};
use super::r#ref::{ref_focus_name, ref_key_name, ref_scroll_name, ref_text_name, RefUsage};
use super::stmt::{generate_statement_flutter_dw, FlutterStmtFormatter};
use super::utils::{
    bind_value_write_back, binding_to_flutter_param, callback_is_void, convert_literal_prop_value,
    css_color_to_dart, event_to_flutter_callback, generic_event_wrapper_callback,
    icon_name_to_dart, lower_first, merge_write_back_into_on_changed, prop_to_flutter_param_name,
    tag_to_flutter_widget, widget_allows_event, widget_allows_param, widget_has_const_constructor,
    widget_param_rename, widget_param_wrapper, EventWrapperKind,
};
use std::fmt::Write;

// ═══════════════════════════════════════════════════════════════
// View tree formatting (single-line Flutter widget expressions)
// ═══════════════════════════════════════════════════════════════

/// Per-component context threaded through view-tree formatting.
pub(crate) struct ViewContext<'a> {
    pub(crate) component_map: &'a FxHashMap<&'a str, &'a ComponentIR>,
    /// Store alias → Riverpod provider variable name
    /// (e.g. "TodoStore" → "todoStoreProvider").
    pub(crate) store_provider_map: &'a FxHashMap<String, String>,
    /// Per-ref capability flags — drives on-demand controller attachment
    /// (focusNode/controller params) on ref-bound elements (Task 5.2).
    pub(crate) ref_usage: &'a FxHashMap<String, RefUsage>,
    /// Component name → declared slot names (`"children"` for the unnamed
    /// slot) — drives consumer-side slot projection (Task 5.3).
    pub(crate) component_slots: &'a FxHashMap<String, FxHashSet<String>>,
    pub(crate) ir: &'a IRProgram,
    /// Design lookups for `class:`/`variant:` resolution (Task 4.3).
    pub(crate) design: &'a DesignResolver<'a>,
    /// Compiler warnings accumulated during view formatting (C6).
    pub(crate) warnings: &'a mut Vec<crate::codegen::CodegenWarning>,
    /// Source file of the component being generated (`comp.source_file`) —
    /// anchors warnings at the element's file (C2d span threading).
    pub(crate) file: Option<String>,
    /// Motion-element ordinal counter (T5.7): assigns each motion element
    /// its DFS pre-order index so its controllers are named per element
    /// instance (`_{motion}Controller_{idx}`). MUST increment with the same
    /// rule and order as `collect_motion_element_usage` (the contract
    /// mod.rs's declaration/initState/dispose emitters rely on).
    pub(crate) motion_counter: &'a mut usize,
    /// Props/events/slots of the CURRENT component — bare identifier
    /// references rewrite to `widget.<name>` (Task 5.8).
    pub(crate) widget_names: &'a FxHashSet<String>,
    /// Component-fn renames (`double` → `double_` for Dart type shadowing).
    pub(crate) fn_renames: &'a FxHashMap<String, String>,
    /// The component being generated — fn-arity lookups for handler wiring.
    pub(crate) comp: &'a ComponentIR,
    /// Hoisted store reads (Task 3.7, Riverpod 3.0 scope compliance):
    /// `(local_name, read_dart)` pairs collected while formatting the view
    /// tree. mod.rs emits `final {name} = {expr};` in build() before the
    /// return, so lazy closures (`.where` predicates, ListView itemBuilder
    /// bodies) reference build-scope locals instead of calling `ref.watch`
    /// — Riverpod 3.0 forbids watch outside the build phase.
    pub(crate) hoisted: &'a mut Vec<(String, String)>,
    /// Unique-name counter for hoisted locals — shared across the whole
    /// view tree so names cannot collide (`__omni0`, `__omni1`, ...).
    pub(crate) hoist_next: &'a mut usize,
    /// True while formatting a ListView itemBuilder body (and its iterable):
    /// the entire expression executes at LAYOUT time, so EVERY store read is
    /// hoisted, not just closure-internal ones.
    pub(crate) lazy_context: bool,
    /// Component-level static-type scope (signals + computeds) — the shared
    /// Dart method table (get/contains/concat shaping) reads
    /// `IdentifierExpr.type_ann`, which the IR never populates (lower
    /// resolves against fresh scopes); this scope re-annotates view-tree
    /// expressions the same way fn/effect bodies are annotated in mod.rs
    /// (Task 5.8a). Without it a typed Map signal's `contains` fell to the
    /// runtime `is Map` dispatch — unnecessary_type_check under dart analyze.
    pub(crate) comp_type_scope: &'a FxHashMap<String, String>,
    /// Compiler errors accumulated during view formatting (Task 14) —
    /// unmappable direct-write style values fail compilation instead of
    /// silently dropping (the old `Some(_) => skip` behavior).
    pub(crate) errors: &'a mut Vec<Diagnostic>,
    /// Target-independent direct-write classification (Task 12): entries =
    /// dynamic style values on @ref'd elements; d1/d2 = ValueNotifier-stored
    /// signal names (Task 13 storage), b2_entries = render-path-dependent
    /// entries (setState rebuild channel, bare signal names).
    pub(crate) direct_write: &'a DirectWriteSets,
    /// Phase 3 (Task 13, ADR-071 decision 2): d1 ∪ d2 signal NAMES — inline
    /// handler statements mutate those signals via `_name.value` (no
    /// setState); the State fields were renamed `_name` (bare names would be
    /// undeclared identifiers in Dart).
    pub(crate) direct_write_signals: &'a FxHashSet<String>,
    /// Task 8 (@load 多例): per-instance builtin identifiers — map from
    /// builtin name (`loading`/`error`/`data`/`retry`/`reset`) to the FINAL
    /// rendered form (`{key}Async.isLoading`, `() => ref.invalidate(...)`,
    /// ...). Empty when the component has no `@load`. Values are only valid
    /// inside build() — the `{key}Async` watch local is build-scoped.
    pub(crate) load_builtins: &'a FxHashMap<String, String>,
    /// Task 5 (@form): whether the component declares a `<form @form>` —
    /// builtin `form.valid`/`form.errors`/`form.submitting` field reads map
    /// to the reachability-gated locals.
    pub(crate) form_present: bool,
    /// Task 8 (@load 参数): param-signal providers (signal → `{signal}Provider`)
    /// for loader arguments — identifier reads map to `ref.watch(...)` inside
    /// build (auto-reload), `ref.read(...)` in lazy closures. Empty when the
    /// component has no param-signal @load args.
    pub(crate) param_providers: &'a FxHashMap<String, String>,
    /// Task 9 (无限滚动 §2.3): paginated+scroll @load instance keys, in
    /// declaration order — each `[scroll]` @for loop consumes the next key
    /// (pairing is by position) to attach its ScrollController.
    pub(crate) scroll_keys: &'a [String],
    /// Ordinal of the NEXT scroll pairing to consume (advances per `[scroll]`
    /// @for loop in the view).
    pub(crate) scroll_next: &'a mut usize,
    /// Task 9 (debounce/throttle §2.5): per-kind Timer indices — inline
    /// handlers number their map keys (`_omniTimers['debounce_{n}']`) in
    /// visit order so re-renders reuse stable entries.
    // Interior-mutable counters (`Cell`) — the timing-call interceptor runs
    // inside `build_inline_handler`'s `impl Fn` closure (Task 9 §2.5), which
    // forbids mutable captures; `Cell` gives per-ordinal map keys
    // (`_omniTimers['debounce_{n}']`) without any mutable borrow.
    pub(crate) debounce_next: &'a Cell<usize>,
    pub(crate) throttle_next: &'a Cell<usize>,
}

impl ViewContext<'_> {
    /// Push a codegen warning positioned at the element's source span (C2d
    /// span threading) — file from the component's source file, line/col
    /// from the element's span. Elements without a span (hand-built IR)
    /// stay honest `-:0:0` (never a fabricated span).
    pub(crate) fn warn_at(&mut self, message: impl Into<String>, span: Option<crate::lexer::Span>) {
        let (line, col) = span.map_or((0, 0), |s| (s.line, s.col));
        self.warnings.push(crate::codegen::CodegenWarning::at(
            message,
            self.file.clone(),
            line,
            col,
        ));
    }

    /// Push a compile-time codegen error positioned at the element's source
    /// span (Task 14) — the diagnostic channel for unmappable direct-write
    /// style values (replaces the old silent drop). Same anchor rules as
    /// `warn_at`: file from the component's source file, line/col from the
    /// element's span, honest `-:0:0` for span-less hand-built IR.
    pub(crate) fn error_at(
        &mut self,
        message: impl Into<String>,
        span: Option<crate::lexer::Span>,
    ) {
        let (line, col) = span.map_or((0, 0), |s| (s.line, s.col));
        self.errors.push(Diagnostic {
            severity: Severity::Error,
            kind: DiagnosticKind::ECodegen,
            message: message.into(),
            file: self.file.clone(),
            line,
            col,
            end_line: None,
            end_col: None,
            suggestion: None,
        });
    }

    /// Does the named handler reference a function with NO parameters?
    /// (A 0-arg fn tear-off is not assignable to `ValueChanged<String>` —
    /// the payload-bearing callback types — so those wire through a wrapper
    /// closure instead.)
    fn ref_is_zero_param_fn(&self, ref_name: &str) -> bool {
        self.comp
            .fns
            .iter()
            .any(|f| f.name == ref_name && f.params.is_empty())
            || self
                .ir
                .all_functions()
                .iter()
                .any(|f| f.name == ref_name && f.params.is_empty())
    }

    /// Does the named handler REQUIRE an event parameter? (A param-bearing
    /// fn cannot tear off to a 0-arity VoidCallback-family callback —
    /// `argument_type_not_assignable`.) Unlike `ref_is_zero_param_fn` this
    /// is FALSE for unknown names — an undeclared fn has unknown arity and
    /// keeps the legacy tear-off emission.
    fn ref_takes_params(&self, ref_name: &str) -> bool {
        let takes = |f: &FunctionIR| !f.params.is_empty();
        self.comp.fns.iter().any(|f| f.name == ref_name && takes(f))
            || self
                .ir
                .all_functions()
                .iter()
                .any(|f| f.name == ref_name && takes(f))
    }

    /// The referenced fn's FIRST param type, when it has exactly one param
    /// (for cast-coercion wrappers on nullable event payloads).
    fn ref_single_param_type(&self, ref_name: &str) -> Option<String> {
        let find = |f: &FunctionIR| -> Option<String> {
            if f.name == ref_name && f.params.len() == 1 {
                f.params[0]
                    .r#type
                    .as_ref()
                    .map(super::super::dart::dart_type_ir)
            } else {
                None
            }
        };
        self.comp
            .fns
            .iter()
            .find_map(find)
            .or_else(|| self.ir.all_functions().iter().find_map(|f| find(f)))
    }

    /// Is the CHILD component's event payload nullable (or void)? Drives the
    /// FnRef wrapper shape — a nullable payload cannot tear off onto a
    /// non-nullable handler param (`dynamic Function(String)` is not
    /// assignable to `void Function(String?)?`), and a void payload must not
    /// receive a closure that takes the payload.
    fn child_event_payload(&self, child_tag: &str, event_name: &str) -> Option<String> {
        let child = self.component_map.get(child_tag)?;
        child
            .events
            .iter()
            .find(|e| e.name == event_name)
            .map(|e| e.param_type.clone())
    }
}

impl ViewContext<'_> {
    /// Format an expression in view context — `StoreFieldAccessExpr` is
    /// rewritten to Riverpod 3.0 `ref.watch(...)` / `ref.read(...)` forms (C4)
    /// and widget-scope identifiers to `widget.<name>` (Task 5.8).
    ///
    /// Task 3.7: store reads inside CLOSURE bodies (lazy `.where`
    /// predicates, event handlers) are hoisted into build locals registered
    /// in `self.hoisted` — the closure gets a reference to the local instead
    /// of a `ref.watch`, which Riverpod 3.0 forbids outside the build phase.
    /// In `lazy_context` (ListView itemBuilder) every read is hoisted, since
    /// the whole expression executes at layout time.
    fn format_expr(&mut self, expr: &ExprIR) -> String {
        // Task 5.8a: re-annotate IdentifierExpr type_ann from the
        // component's signals/computeds (see comp_type_scope doc) BEFORE the
        // rewrites — the shared method table shapes get/contains/concat from
        // the static types. Unannotated identifiers (loop vars, props) keep
        // the runtime dispatch, unchanged.
        let expr = super::type_annotate::annotate_expr_types(expr, self.comp_type_scope);
        // Task 3.7: hoist FIRST — the closure walk must see the ORIGINAL
        // store-access nodes (`S.filter`), before rewrite_store_accesses
        // turns them into `ref.watch(...)` method calls. The hoisted locals
        // carry the rewritten read expressions directly.
        let mut hoister = ClosureHoister::new(
            self.store_provider_map,
            self.param_providers,
            self.ir,
            self.lazy_context,
            self.hoist_next,
        );
        let (expr, locals) = hoister.hoist(&expr);
        for (name, read) in &locals {
            self.hoisted
                .push((name.clone(), format_expr_ir_flutter(read)));
        }
        let rewritten = rewrite_store_accesses(&expr, self.store_provider_map, self.ir);
        let rewritten = rewrite_widget_refs(&rewritten, self.widget_names, self.fn_renames);
        // Task 5/8: context-carrying format — @load builtins (loading/error/
        // data/retry/reset per instance) map to their rendered shapes, @form
        // builtins (form.*) map to the reachability-gated locals, param
        // signals read through `ref.watch({p}Provider)` (build phase —
        // auto-reload on param change).
        format_expr_ir_flutter_ctx(
            &rewritten,
            self.direct_write_signals,
            self.load_builtins,
            self.form_present,
            self.param_providers,
            true,
        )
    }
}

/// Render a ViewTree as a single-line Flutter widget expression string.
pub(crate) fn format_view_tree(
    node: &ViewTree,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    match node {
        ViewTree::Element(el) => format_element(el, motions, ctx),
        ViewTree::Text(text) => {
            // NOTE: `{`-leading text is NOT special-cased — legit
            // interpolations arrive as ViewTree::Interpolation nodes; a
            // `{`-shaped TEXT node only survives parsing via the parser's
            // malformed-interpolation fallback (parser/view/mod.rs), and
            // treating it as `${{…}}` Dart interpolation was a live
            // injection vector (`Text('${{print(1), "x"}}')` executes,
            // Task 3.1 review CRITICAL round 2). Such text is escaped as
            // plain text and rendered literally.
            // Plain text — Dart string escaping. `$` must be escaped too:
            // `$var` / `${expr}` inside a Dart string is interpolation,
            // so an unescaped `$` from text content executes code
            // (injection corpus finding, Task 3.1).
            let escaped = escape_literal_str(text, '\'');
            // T5.7 const injection: a plain literal text node is a constant
            // expression — `const Text(...)` avoids re-allocating it on every
            // build. The `\$` escaping is a plain string escape (still a
            // constant expression).
            format!("const Text('{}')", escaped.replace('$', "\\$"))
        }
        ViewTree::Interpolation(expr) => {
            let dart_expr = ctx.format_expr(expr);
            // `Text()` takes a String — non-String expressions (store
            // signal reads, arithmetic, ...) would be a static type
            // error. `'${...}'` string interpolation accepts any value
            // (Dart stringifies), so interpolated view expressions are
            // always emitted in that form.
            format!("Text('${{{}}}')", dart_expr)
        }
        ViewTree::Conditional(cond) => format_conditional(cond, motions, ctx),
        ViewTree::ListRendering(list) => format_list_rendering(list, motions, ctx),
        ViewTree::Fragment(children) => children
            .iter()
            .map(|c| format_view_tree(c, motions, ctx))
            // Empty fragments (e.g. the @load directive lowering to
            // Fragment(vec![])) render to "" — skipping them avoids a
            // dangling `, ` in the generated children list (dart analyze
            // missing_identifier; LoadDemo.dart:27).
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        ViewTree::Slot { name, fallback, .. } => {
            // Slots render as `widget.{name} ?? fallback` — parent-projected
            // content takes precedence; the preserved fallback (Task 1.2)
            // renders otherwise. Unnamed slots map to the `children` param
            // (Flutter analogue of React's `{props[name] ?? (fallback)}`).
            let param = format!("widget.{}", name.as_deref().unwrap_or("children"));
            match format_slot_content(fallback, motions, ctx) {
                Some(fb) => format!("{} ?? {}", param, fb),
                // No fallback: `widget.x` is `Widget?` — a bare entry would
                // be a static error in `List<Widget>` contexts. The shrink
                // box renders nothing when the parent projects no content.
                None => format!("{} ?? SizedBox.shrink()", param),
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Format slot content (fallback or consumer-projected) as a single widget
/// expression. Multi-node content is wrapped in a Column — the Flutter
/// analogue of React's fragment — so `??`/param expressions stay single
/// widgets. Fragments are flattened transparently. Returns `None` when the
/// content is empty (a bare consumer `@slot name` directive projects
/// nothing).
fn format_slot_content(
    content: &[ViewTree],
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> Option<String> {
    let nodes: Vec<&ViewTree> = content
        .iter()
        .flat_map(|c| match c {
            ViewTree::Fragment(children) => children.iter().collect(),
            other => vec![other],
        })
        .collect();
    match nodes.len() {
        0 => None,
        1 => Some(format_view_tree(nodes[0], motions, ctx)),
        _ => {
            let children: Vec<String> = nodes
                .iter()
                .map(|c| format_view_tree(c, motions, ctx))
                .collect();
            Some(wrap_multi_widget(&children))
        }
    }
}

/// Wrap a list of widget expressions in a single-widget `Column` — the
/// Flutter analogue of the React backend's fragment wrapper
/// (`gen_branch_body_tsx` emits `<>...</>`). Used wherever a view fragment
/// must produce ONE widget expression: build() returns, ternary branches,
/// and list item bodies.
pub(crate) fn wrap_multi_widget(children: &[String]) -> String {
    // F2: empty renderings (e.g. the `@load` directive lowering to
    // `Fragment(vec![])`) produce "" — dropping them here avoids a dangling
    // `, ` in the generated children list (dart analyze
    // missing_identifier).
    let children: Vec<&str> = children
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect();
    format!(
        "Column(mainAxisSize: MainAxisSize.min, children: [{}])",
        children.join(", ")
    )
}

/// Compose mixed Text content into a `TextSpan(children: [...])` list for
/// `Text.rich` (Task C2b #5): text/interpolation nodes become
/// `TextSpan(text:)` entries; bare Text-mapped elements (no props/events —
/// inline semantic tags like `span`/`label`) recurse as nested TextSpan
/// children (preserving the inline flow); everything else embeds as a
/// `WidgetSpan(child:)`.
fn format_text_span_children(
    children: &[ViewTree],
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    let spans: Vec<String> = children
        .iter()
        .map(|c| format_text_span_fragment(c, motions, ctx))
        .collect();
    format!("TextSpan(children: [{}])", spans.join(", "))
}

/// One TextSpan fragment for [`format_text_span_children`].
fn format_text_span_fragment(
    child: &ViewTree,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    match child {
        ViewTree::Text(t) => {
            // v4 rust HIGH: `$` 必须转义——Dart 字符串插值（与文本路径
            // :222-227 同模式；此处原为手写 replace 漏掉 `$`，注入语料
            // rawtext-multi 点捕获）。
            let escaped = escape_literal_str(t, '\'');
            format!("TextSpan(text: '{}')", escaped.replace('$', "\\$"))
        }
        ViewTree::Interpolation(e) => format!("TextSpan(text: '${{{}}}')", ctx.format_expr(e)),
        ViewTree::Element(el) => {
            let is_bare_text = tag_to_flutter_widget(&el.tag) == "Text"
                && el.props.is_empty()
                && el.events.is_empty()
                && el.bindings.is_empty()
                && el.inline_style.is_none()
                && el.a11y.is_none()
                && !el.children.is_empty();
            if is_bare_text {
                format_text_span_children(&el.children, motions, ctx)
            } else {
                format!(
                    "WidgetSpan(child: {})",
                    format_single_view(child, motions, ctx)
                )
            }
        }
        // Fragments and other view nodes embed as a single widget
        // (format_single_view Column-wraps multi-node fragments).
        _ => format!(
            "WidgetSpan(child: {})",
            format_single_view(child, motions, ctx)
        ),
    }
}

/// Format a view tree as a single widget expression — valid anywhere a
/// Widget expression is required (`return` statements, ternary branches,
/// `itemBuilder` bodies). Multi-node root fragments are wrapped in a
/// Column (`format_view_tree` itself comma-joins fragments, which is only
/// legal inside `children: [...]` lists).
pub(crate) fn format_single_view(
    node: &ViewTree,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    match node {
        ViewTree::Fragment(children) if children.len() > 1 => {
            let parts: Vec<String> = children
                .iter()
                .map(|c| format_view_tree(c, motions, ctx))
                .collect();
            wrap_multi_widget(&parts)
        }
        _ => format_view_tree(node, motions, ctx),
    }
}

/// F1 (3.2 review finding): input widgets (TextField/Checkbox/Switch/Slider/
/// DropdownButton) need a Material ancestor — a bare TextField explodes to
/// ~100k px tall in the widget-test environment (a Material wrapper restores
/// the 48 px input) and looks broken in real apps. Components whose view
/// contains such a widget get the ROOT wrapped in `Material(child: ...)`.
/// Roots that already provide a material surface themselves (Scaffold /
/// Material / Card — explicit in the source) are left alone; page-level
/// shells are the 3.9 Scaffold task's territory.
pub(crate) fn wrap_root_material_if_inputs(node: &ViewTree, tree_expr: String) -> String {
    let root_widget = match node {
        ViewTree::Element(el) => Some(tag_to_flutter_widget(&el.tag)),
        _ => None,
    };
    if matches!(
        root_widget,
        Some("Scaffold" | "Material" | "Card" | "ListTile")
    ) {
        return tree_expr;
    }
    if view_has_material_dependent_widget(node) {
        return format!("Material(child: {tree_expr})");
    }
    tree_expr
}

/// Whether any element in the tree maps to a widget that renders incorrectly
/// without a Material ancestor.
fn view_has_material_dependent_widget(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if matches!(
                tag_to_flutter_widget(&el.tag),
                "TextField" | "Checkbox" | "Switch" | "Slider" | "DropdownButton"
            ) {
                return true;
            }
            el.children.iter().any(view_has_material_dependent_widget)
        }
        ViewTree::Conditional(cond) => cond
            .branches
            .iter()
            .filter_map(|b| b.body.as_ref())
            .chain(cond.fallback.iter())
            .any(view_has_material_dependent_widget),
        ViewTree::ListRendering(list) => list
            .body
            .iter()
            .chain(list.empty_body.iter())
            .any(view_has_material_dependent_widget),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_material_dependent_widget),
        ViewTree::Fragment(children) => children.iter().any(view_has_material_dependent_widget),
        ViewTree::Interpolation(_) | ViewTree::Text(_) => false,
        // ViewTree is #[non_exhaustive] (core IR contract).
        _ => false,
    }
}

/// Consumer-side slot projection name for a child of a component
/// instantiation. Two forms are recognized:
/// - an element with a literal `slot="name"` prop (mirrors the OmniView
///   backend's `has_slot_attr`);
/// - a consumer-position `@slot name { ... }` directive, whose fallback
///   content is the projected content.
fn child_slot_name(node: &ViewTree) -> Option<String> {
    match node {
        ViewTree::Element(el) => el.props.get("slot").and_then(|v| match v {
            ExprIR::LiteralExpr { value } => value.as_str().map(str::to_string),
            _ => None,
        }),
        ViewTree::Slot { name, .. } => name.clone(),
        _ => None,
    }
}

/// Remove the `slot` prop from an element subtree root so it never reaches
/// the generated widget after projection into a named slot param.
fn strip_slot_prop(node: &ViewTree) -> ViewTree {
    match node {
        ViewTree::Element(el) if el.props.contains_key("slot") => {
            let mut props = el.props.clone();
            props.remove("slot");
            ViewTree::Element(Box::new(ElementIR {
                props,
                ..*el.clone()
            }))
        }
        other => other.clone(),
    }
}

/// Collect slot declarations (names) from a view tree.
/// Used to emit `Widget?` constructor params for declared slots. Slots
/// inside a child component's children are consumer-side projections, not
/// declarations — they are skipped (the child component types them via its
/// own declarations; mirrors the React backend).
pub(crate) fn collect_view_slots(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    acc: &mut Vec<Option<String>>,
) {
    match node {
        ViewTree::Slot { name, fallback, .. } => {
            acc.push(name.clone());
            for child in fallback {
                collect_view_slots(child, component_map, acc);
            }
        }
        ViewTree::Element(el) => {
            if component_map.contains_key(el.tag.as_str()) {
                // Children of a component instantiation are projections —
                // the child component types them via its own declarations.
                return;
            }
            for child in &el.children {
                collect_view_slots(child, component_map, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_view_slots(body, component_map, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_view_slots(fb, component_map, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_view_slots(body, component_map, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_view_slots(empty_body, component_map, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_view_slots(child, component_map, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Whether any element in the view references design classes or variants
/// (`class:` prop, `_design_variant` prop, or `class_map_bindings`) —
/// drives the `import 'design.dart';` decision in the component generator.
///
/// Task 5.8a: the scan now RESOLVES every reference against the design IR,
/// mirroring `wrap_design_decorations` — only references whose emission
/// actually references design.dart names count (a bare `class: wrapper`
/// that resolves to no design block, or a resolved class with zero
/// decoration-mappable properties, emits no design reference). The old
/// "any usage counts" scan left the import unused in ~30 showcase files
/// (unused_import warnings).
pub(crate) fn view_uses_design_classes(node: &ViewTree, design: &DesignResolver) -> bool {
    match node {
        ViewTree::Element(el) => {
            let class_prop_used = match el.props.get("class") {
                Some(ExprIR::LiteralExpr { value }) => value.as_str().is_some_and(|s| {
                    s.split_whitespace()
                        .any(|c| class_emits_design_ref(c, design))
                }),
                Some(_) => false,
                None => false,
            };
            let variant_used = el
                .props
                .get("_design_variant")
                .is_some_and(|e| variant_emits_design_ref(e, design));
            let class_map_used = el
                .class_map_bindings
                .keys()
                .any(|c| class_emits_design_ref(c, design));
            if class_prop_used || variant_used || class_map_used {
                return true;
            }
            el.children
                .iter()
                .any(|c| view_uses_design_classes(c, design))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .is_some_and(|b| view_uses_design_classes(b, design))
            }) || cond
                .fallback
                .as_ref()
                .is_some_and(|b| view_uses_design_classes(b, design))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .is_some_and(|b| view_uses_design_classes(b, design))
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(|b| view_uses_design_classes(b, design))
        }
        ViewTree::Fragment(children) => {
            children.iter().any(|c| view_uses_design_classes(c, design))
        }
        ViewTree::Slot { fallback, .. } => {
            fallback.iter().any(|c| view_uses_design_classes(c, design))
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Task 9 (§2.5): whether any inline event handler in the view is a
/// debounce/throttle call — drives the `import 'dart:async';` emission and
/// the `_omniTimers` field in the component State.
///
/// Must mirror the interception predicate EXACTLY: only an InlineFn whose
/// body is a single `debounce(fn, ms)` / `throttle(fn, ms)` expression
/// statement emits a Timer closure; timing calls in prop VALUES (which
/// render as stdlib placeholders) never emit timers, so a coarse scan would
/// over-report and leave `_omniTimers` unused.
pub(crate) fn view_has_timing_handlers(view: &ViewTree) -> bool {
    match view {
        ViewTree::Element(el) => {
            el.events.values().any(|h| match h {
                EventHandlerIR::InlineFn { body, .. } => is_timing_call_body(body),
                _ => false,
            }) || el.children.iter().any(view_has_timing_handlers)
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_has_timing_handlers))
                || cond.fallback.as_ref().is_some_and(view_has_timing_handlers)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_has_timing_handlers)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_has_timing_handlers)
        }
        ViewTree::Fragment(children) => children.iter().any(view_has_timing_handlers),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_has_timing_handlers),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Whether a BlockIR is a single debounce/throttle expression statement —
/// the exact predicate the Timer-closure interception matches on (Task 9
/// §2.5; shared with `view_has_timing_handlers` so the dart:async/`_omniTimers`
/// gating never drifts from the interception).
fn is_timing_call_body(body: &BlockIR) -> bool {
    matches!(
        body.statements.as_slice(),
        [StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr { name, .. },
            ..
        }] if crate::semantic::load_contract::TIMING_FUNCS.contains(&name.as_str())
    )
}

/// Whether a bare class name resolves to a design class whose decoration
/// emission references design.dart (a factory or a merged BoxDecoration —
/// exactly what `resolve_static_class`/`wrap_design_decorations` emit).
fn class_emits_design_ref(name: &str, design: &DesignResolver) -> bool {
    let Some((d, class)) = design.class_owner(name) else {
        return false;
    };
    let (fields, _) = resolve_class_decoration(d, class);
    !fields.is_empty()
}

/// Whether a `_design_variant` prop resolves to a variant whose decoration
/// emission references design.dart (mirrors the wrapper's resolution).
fn variant_emits_design_ref(e: &ExprIR, design: &DesignResolver) -> bool {
    let ExprIR::LiteralExpr { value } = e else {
        return false;
    };
    let (Some(design_name), Some(variant_name)) = (
        value.get("design_name").and_then(|v| v.as_str()),
        value.get("variant_name").and_then(|v| v.as_str()),
    ) else {
        return false;
    };
    let Some(d) = design.design(design_name) else {
        return false;
    };
    let Some(variant) = design.variant(d, variant_name) else {
        return false;
    };
    let dim_values: FxHashMap<String, String> = value
        .get("dimension_values")
        .and_then(|v| v.as_object())
        .map(|o| {
            o.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default();
    let (fields, _) = resolve_variant_decoration(d, variant, &dim_values);
    !fields.is_empty()
}

/// Collect the tags of all component instantiations in a view tree — the
/// child components whose generated `.dart` files the generated component
/// file must import (cross-file composition). Children of a component
/// instantiation ARE descended into: consumer-side slot projections
/// (`@slot header { <Avatar /> }` inside a `<Card>`) are emitted as widget
/// expressions in THIS component's file, so the projected component's
/// class must resolve here too.
pub(crate) fn collect_component_tags<'a>(
    node: &'a ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    acc: &mut Vec<&'a str>,
) {
    match node {
        ViewTree::Element(el) => {
            if component_map.contains_key(el.tag.as_str()) {
                acc.push(&el.tag);
            }
            for child in &el.children {
                collect_component_tags(child, component_map, acc);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_component_tags(body, component_map, acc);
                }
            }
            if let Some(ref fb) = cond.fallback {
                collect_component_tags(fb, component_map, acc);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_component_tags(body, component_map, acc);
            }
            if let Some(ref empty_body) = list.empty_body {
                collect_component_tags(empty_body, component_map, acc);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_component_tags(child, component_map, acc);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                collect_component_tags(child, component_map, acc);
            }
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Precompute each component's declared slot names for consumer-side
/// projection. The unnamed slot is keyed as `"children"` — the Dart
/// constructor param it maps to.
pub(crate) fn build_component_slot_map(
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> FxHashMap<String, FxHashSet<String>> {
    let mut map: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    for (name, comp) in component_map {
        let mut slots: FxHashSet<String> = comp
            .slots
            .iter()
            .map(|s| s.name.clone().unwrap_or_else(|| "children".to_string()))
            .collect();
        if let Some(ref view) = comp.view {
            let mut acc: Vec<Option<String>> = Vec::new();
            collect_view_slots(view, component_map, &mut acc);
            for n in acc {
                slots.insert(n.unwrap_or_else(|| "children".to_string()));
            }
        }
        map.insert(name.to_string(), slots);
    }
    map
}

/// Strip CSS units from a value string and return a Dart-compatible numeric literal.
/// "16px" -> "16.0", "8" -> "8.0", "1.5em" -> "1.5"
fn css_value_to_dart_number(css_val: &str) -> String {
    let trimmed = css_val.trim();
    let numeric_part: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    if numeric_part.is_empty() || numeric_part == "-" {
        "0.0".to_string()
    } else if numeric_part.contains('.') {
        numeric_part
    } else {
        format!("{}.0", numeric_part)
    }
}

/// Map a CSS property name to its Flutter widget parameter.
/// Returns None for unsupported properties.
fn css_prop_to_flutter_param(prop: &str) -> Option<&'static str> {
    match prop {
        "padding" => Some("padding"),
        "margin" => Some("margin"),
        "width" => Some("width"),
        "height" => Some("height"),
        "color" => Some("color"),
        // CSS shorthand `background` (the form OmniView demos write)
        // maps like background-color: Container's `color` IS its
        // background; on Text the wrapper routes it into the TextStyle.
        "background" => Some("color"),
        "background-color" => Some("color"),
        _ => None,
    }
}

/// Resolve one bare class name against the design IR. Class names arrive
/// bare (the `Design.` prefix is dropped during lowering — `class:
/// LearnDesign.Card` becomes the `class` prop "Card"). Returns the resolved
/// decoration fields plus the `AppDesign.<class>()` factory reference (valid
/// only when the decoration derives from exactly this one class). Unknown
/// class names and layout-only classes warn and return `None`. The element's
/// source span anchors those warnings at the usage point (C2d).
fn resolve_static_class(
    name: &str,
    span: Option<crate::lexer::Span>,
    ctx: &mut ViewContext,
) -> Option<(DecorationFields, Option<String>)> {
    let Some((design, class)) = ctx.design.class_owner(name) else {
        // Component-local style-block class (`.themeCard` / `.active` in the
        // component's own `style { }`) — not declared by any design block.
        // Resolve it from `comp.styles` so `class:` and `class: { name: cond }`
        // bindings on local classes emit real decorations (previously every
        // local class was dropped with the "not declared by any design block"
        // warning — ThemeSwitcher cards, DynamicTokens preset buttons).
        // Compound selectors (`.themeCard.active`) also match their tail
        // class: `class: { themeCard: true, active: cond }` binds `active` to
        // the combined rule, whose fields layer on while the condition holds.
        if let Some(rule) = ctx.comp.styles.iter().find(|s| {
            !s.is_tag_selector
                && (s.class_name == name || s.class_name.ends_with(&format!(".{name}")))
        }) {
            let (fields, skipped) = resolve_style_rule_decoration(ctx.design.any_design(), rule);
            warn_design_skipped(ctx, &format!("class '{name}'"), skipped, span);
            if fields.is_empty() {
                ctx.warn_at(
                    format!(
                        "[flutter] component-local class '{name}' has no \
                         decoration-mappable properties; skipped"
                    ),
                    span,
                );
                return None;
            }
            // Local classes have no `AppDesign.<class>()` factory — the
            // decoration is composed inline from the fields.
            return Some((fields, None));
        }
        ctx.warn_at(
            format!(
                "[flutter] class '{name}' is not declared by any design block; \
                 skipped (no Flutter equivalent)"
            ),
            span,
        );
        return None;
    };
    let (fields, skipped) = resolve_class_decoration(design, class);
    warn_design_skipped(ctx, &format!("class '{name}'"), skipped, span);
    if fields.is_empty() {
        ctx.warn_at(
            format!(
                "[flutter] design class '{name}' has no decoration-mappable properties; \
                 skipped (layout/text-only classes have no Flutter decoration equivalent)"
            ),
            span,
        );
        None
    } else {
        let factory = class_factory_expr(design, class);
        Some((fields, factory))
    }
}

/// Surface unmappable (property, value) pairs on the warnings channel,
/// naming the resolved thing they belong to (class/variant).
fn warn_design_skipped(
    ctx: &mut ViewContext,
    what: &str,
    skipped: Vec<(String, String)>,
    span: Option<crate::lexer::Span>,
) {
    for (prop, value) in skipped {
        ctx.warn_at(
            format!(
                "[flutter] {what} property '{prop}' value '{value}' is not mappable to a \
                 BoxDecoration field; skipped"
            ),
            span,
        );
    }
}

/// Resolve the element's design classes/variants (`class:` prop,
/// `_design_variant` prop, `class_map_bindings`) against the design IR and
/// wrap `inner` in a `Container` carrying the resolved `BoxDecoration`
/// (Task 4.3 — the review's HIGH: static class styles vanished and dynamic
/// `class:active={cond}` toggled an empty `BoxDecoration()` stub).
///
/// Resolution rules:
/// - A single static class resolves to its generated factory —
///   `Container(decoration: AppDesign.card(), ...)` — the design doc's
///   hand-written form, so the generated factories are actually referenced.
/// - Composed decorations (multi-class, class+variant, static+dynamic)
///   merge per `BoxDecoration` field with LATER classes winning on conflict
///   (the CSS cascade analogue for multiple classes on one element).
/// - Dynamic classes (`class: { active: cond }`) emit
///   `decoration: cond ? <resolved> : null` with REAL resolved values;
///   combined with static classes they emit
///   `decoration: cond ? <all merged> : <static merged>`.
/// - Unknown class names, layout-only classes, and unmappable values warn
///   naming the class and the reason (spec §10.3 — no silent drops).
fn wrap_design_decorations(el: &ElementIR, inner: String, ctx: &mut ViewContext) -> String {
    // 1. Static classes from the `class` prop (`class: Design.Card` lowers to
    //    a literal "Card"; `class: { X: true }` entries join it) and the
    //    `style_classes` carrier (never populated by lowering today — view
    //    classes flow through the `class` prop — but a hand-built IR could
    //    carry it; resolving it closes the review's "style_classes is never
    //    read by the Flutter backend").
    let mut class_names: Vec<String> = Vec::new();
    if let Some(ExprIR::LiteralExpr { value }) = el.props.get("class") {
        if let Some(classes) = value.as_str() {
            class_names.extend(classes.split_whitespace().map(str::to_string));
        } else {
            ctx.warn_at(
                "[flutter] non-literal `class` prop values are not supported; \
                 class styling skipped",
                el.source_span,
            );
        }
    }
    if let Some(ref classes) = el.style_classes {
        if let Some(classes) = classes.as_str() {
            class_names.extend(classes.split_whitespace().map(str::to_string));
        }
    }
    let mut static_entries: Vec<(String, DecorationFields, Option<String>)> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for name in class_names {
        if !seen.insert(name.clone()) {
            continue; // same class listed twice — resolve once
        }
        if let Some(resolved) = resolve_static_class(&name, el.source_span, ctx) {
            static_entries.push((name, resolved.0, resolved.1));
        }
    }

    // 2. Variant binding (`variant: Design.Button(color=primary)` lowers to
    //    the `_design_variant` prop). Resolved at compile time from the
    //    merged base ∪ selected-branch properties — the generated
    //    ThemeExtension consts are not used because the extension is never
    //    registered in the generated ThemeData, so a Theme.of lookup would
    //    return null at runtime (invisible decoration — the exact bug this
    //    task removes).
    let mut variant_fields: Option<DecorationFields> = None;
    if let Some(ExprIR::LiteralExpr { value }) = el.props.get("_design_variant") {
        if let (Some(design_name), Some(variant_name)) = (
            value.get("design_name").and_then(|v| v.as_str()),
            value.get("variant_name").and_then(|v| v.as_str()),
        ) {
            let dim_values: FxHashMap<String, String> = value
                .get("dimension_values")
                .and_then(|v| v.as_object())
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect()
                })
                .unwrap_or_default();
            match ctx.design.design(design_name) {
                Some(design) => match ctx.design.variant(design, variant_name) {
                    Some(variant) => {
                        let (fields, skipped) =
                            resolve_variant_decoration(design, variant, &dim_values);
                        warn_design_skipped(
                            ctx,
                            &format!("variant '{variant_name}'"),
                            skipped,
                            el.source_span,
                        );
                        if fields.is_empty() {
                            ctx.warn_at(
                                format!(
                                    "[flutter] variant '{variant_name}' of design \
                                     '{design_name}' has no decoration-mappable properties; \
                                     no visual effect on Flutter"
                                ),
                                el.source_span,
                            );
                        } else {
                            variant_fields = Some(fields);
                        }
                    }
                    None => ctx.warn_at(
                        format!(
                            "[flutter] variant '{variant_name}' not found in design \
                             '{design_name}'"
                        ),
                        el.source_span,
                    ),
                },
                None => ctx.warn_at(
                    format!("[flutter] design '{design_name}' not found for variant binding"),
                    el.source_span,
                ),
            }
        }
    }

    // 3. Dynamic class bindings (`class: { active: cond }`). Literal-true
    //    entries are always-applied static classes; literal-false entries
    //    are skipped entirely.
    let mut dynamic: Vec<(String, DecorationFields)> = Vec::new(); // (cond, fields)
    for (name, expr) in &el.class_map_bindings {
        match expr {
            ExprIR::LiteralExpr { value } if value.as_bool() == Some(true) => {
                if let Some(resolved) = resolve_static_class(name, el.source_span, ctx) {
                    static_entries.push((name.clone(), resolved.0, resolved.1));
                }
            }
            ExprIR::LiteralExpr { value } if value.as_bool() == Some(false) => {}
            _ => {
                if let Some(resolved) = resolve_static_class(name, el.source_span, ctx) {
                    dynamic.push((format_bool_condition(ctx, expr), resolved.0));
                }
            }
        }
    }

    // 4. Compose.
    let mut merged_static = DecorationFields::default();
    for (_, fields, _) in &static_entries {
        merged_static = merge_decoration_fields(merged_static, fields.clone());
    }
    if let Some(vf) = &variant_fields {
        merged_static = merge_decoration_fields(merged_static, vf.clone());
    }
    // Factory reference only when the decoration derives from exactly one
    // static class — no composition involved.
    let factory = if static_entries.len() == 1 && variant_fields.is_none() && dynamic.is_empty() {
        static_entries[0].2.clone()
    } else {
        None
    };

    if dynamic.is_empty() {
        if let Some(factory) = factory {
            format!("Container(decoration: {}, child: {})", factory, inner)
        } else if merged_static.is_empty() {
            inner
        } else {
            format!(
                "Container(decoration: {}, child: {})",
                decoration_fields_to_expr(&merged_static),
                inner
            )
        }
    } else if merged_static.is_empty() {
        // Only dynamic classes — the decoration is off (null) until the
        // condition holds.
        let cond = dynamic
            .iter()
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join(" && ");
        let mut merged_all = DecorationFields::default();
        for (_, fields) in &dynamic {
            merged_all = merge_decoration_fields(merged_all, fields.clone());
        }
        if merged_all.is_empty() {
            return inner; // all dynamic classes unresolved — warned above
        }
        format!(
            "Container(\n  decoration: {} ? {} : null,\n  child: {},\n)",
            cond,
            decoration_fields_to_expr(&merged_all),
            inner
        )
    } else {
        // Static + dynamic — the decoration swaps between the static set and
        // the full set (static classes always apply; dynamic classes layer on
        // top and win per field while their condition holds).
        let cond = dynamic
            .iter()
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join(" && ");
        let mut merged_all = merged_static.clone();
        for (_, fields) in &dynamic {
            merged_all = merge_decoration_fields(merged_all, fields.clone());
        }
        format!(
            "Container(\n  decoration: {} ? {} : {},\n  child: {},\n)",
            cond,
            decoration_fields_to_expr(&merged_all),
            decoration_fields_to_expr(&merged_static),
            inner
        )
    }
}

/// Map a semantic role (design doc 域 10 Flutter role 子集) to its
/// `Semantics` flag argument. Returns `None` for unmappable roles — the
/// caller warns and degrades (honest degradation: the role still passes
/// through as an attribute on the web targets).
///
/// The vocabulary is WAI-ARIA canonical (Task 4.3, domain HIGH-2): `textbox`
/// and `img` map internally to the Flutter flags `textField: true` /
/// `image: true`. The legacy flag names still map too — `semantic/
/// validate_a11y` warns users toward the canonical names, so pre-v3 sources
/// keep working while migrating and web output never carries
/// `role="textField"`.
fn role_to_semantics_flag(role: &str) -> Option<&'static str> {
    match role {
        "button" => Some("button: true"),
        // textbox (ARIA canonical) | textField (legacy flag name)
        "textbox" | "textField" => Some("textField: true"),
        "link" => Some("link: true"),
        "checkbox" => Some("checkbox: true"),
        "radio" => Some("radio: true"),
        // img (ARIA canonical) | image (legacy flag name)
        "img" | "image" => Some("image: true"),
        _ => None,
    }
}

/// Wrap the element's widget expression in the a11y block's semantic
/// wrappers (design doc 域 10 mapping table):
///
/// - `label` → `Semantics(label: ...)`
/// - `role`  → Semantics flags (subset above); unmappable roles → warning
///   on the T2.4 warnings channel + a generated-code degradation comment
///   (the design doc's "warning + 生成代码注释" honest degradation)
/// - `live`  → `Semantics(liveRegion: true)`
/// - `alt`   → on image widgets the semanticLabel constructor param is
///   already emitted in `format_element`; on other widgets it degrades to
///   the accessible-name `label` (image alt text IS the accessible name)
/// - `hidden`→ `ExcludeSemantics(excluding: <expr>, ...)` — the widget
///   excludes itself from the semantics tree while the expression holds
///
/// label/role/live/alt merge into ONE `Semantics(...)` call; `hidden`
/// wraps outside it (`ExcludeSemantics(excluding: ..., child:
/// Semantics(...))`). The review's MEDIUM-2 (a11y intent silently dropped
/// on Flutter) is gone — the semantic node is always emitted.
fn wrap_a11y(el: &ElementIR, inner: String, ctx: &mut ViewContext) -> String {
    let Some(ref a11y) = el.a11y else {
        return inner;
    };
    let mut semantics_args: Vec<String> = Vec::new();
    if let Some(ref label) = a11y.label {
        // Dart string escaping — `$` must be escaped like the text path:
        // `$var` / `${expr}` inside a Dart string is interpolation, so an
        // unescaped `$` from a payload executes code (injection corpus
        // finding, Task 3.1 — surfaced by the final T7.3 a11y corpus point).
        let escaped = escape_literal_str(label, '\'');
        semantics_args.push(format!("label: '{}'", escaped.replace('$', "\\$")));
    }
    if let Some(ref role) = a11y.role {
        match role_to_semantics_flag(role) {
            Some(flag) => semantics_args.push(flag.to_string()),
            None => {
                ctx.warn_at(
                    format!(
                        "[flutter] a11y role '{role}' is not in the Flutter \
                         Semantics subset (button/textbox/link/checkbox/radio/\
                         img); degraded — no Semantics flag (the role still \
                         passes through on the web targets)"
                    ),
                    el.source_span,
                );
            }
        }
    }
    if a11y.live.is_some() {
        // Both "polite" and "assertive" map to liveRegion: true (validated
        // by semantic/validate_a11y — the value is one of the two).
        semantics_args.push("liveRegion: true".into());
    }
    if let Some(ref alt) = a11y.alt {
        // Image widgets carry alt via the semanticLabel constructor param
        // (format_element); everywhere else it degrades to the accessible
        // name — no information is lost.
        if !matches!(tag_to_flutter_widget(&el.tag), "Image.network") {
            if a11y.label.is_some() {
                // `label` + `alt` on a non-image element: both would emit
                // `label:` into the SAME Semantics(...) call — duplicate
                // named argument, invalid Dart. Prefer `label` (it IS the
                // accessible name); `alt` is image-specific intent on a
                // non-image element, so it warns and is dropped (honest
                // degradation — never a silent drop).
                ctx.warn_at(
                    "[flutter] a11y alt 仅对 img 元素生效（当前元素非图像）；\
                     与 a11y label 同时出现，alt 被忽略（label 已提供可访问名称）",
                    el.source_span,
                );
            } else {
                semantics_args.push(format!("label: {}", ctx.format_expr(alt)));
            }
        }
    }

    let mut result = inner;
    let mut degradation_comment = String::new();
    if let Some(ref role) = a11y.role {
        if role_to_semantics_flag(role).is_none() {
            // Honest degradation: the generated code says what happened.
            // The role is embedded in a Dart block comment — sanitize `*/`
            // so a payload cannot terminate the comment and leave code
            // behind (injection corpus a11y point, final T7.3 regression).
            let safe_role = role.replace("*/", "* /");
            degradation_comment = format!(
                "/* a11y: role '{safe_role}' has no Flutter Semantics flag \
                 (subset: button/textbox/link/checkbox/radio/img); \
                 degraded — no Semantics flag emitted */ "
            );
        }
    }
    if !semantics_args.is_empty() {
        semantics_args.push(format!("child: {}", result));
        result = format!("Semantics({})", semantics_args.join(", "));
    }
    if let Some(ref hidden) = a11y.hidden {
        result = format!(
            "ExcludeSemantics(excluding: {}, child: {})",
            ctx.format_expr(hidden),
            result
        );
    }
    if degradation_comment.is_empty() {
        result
    } else {
        format!("{degradation_comment}{result}")
    }
}

// ═══════════════════════════════════════════════════════════════
// Conservative const injection (T5.7)
// ═══════════════════════════════════════════════════════════════

/// Whether a view node formats to a constant widget expression.
fn view_node_is_const(node: &ViewTree, ctx: &ViewContext) -> bool {
    match node {
        ViewTree::Text(_) => true,
        ViewTree::Element(el) => element_is_const(el, ctx),
        _ => false,
    }
}

/// Whether an element's widget constructor can be emitted `const` —
/// CONSERVATIVE, statically decidable only:
///
/// - no events, bindings, refs, a11y, design classes/variants, inline
///   styles, or motion behavior (any of them introduces a non-constant
///   expression or a runtime wrapper)
/// - every prop is a literal scalar (string/number/bool/null) — convertible
///   values format to constant expressions (Color(...), EdgeInsets.all(...),
///   FontWeight.w700, ...); raw literals are constants too
/// - every child is const-able (literal text or a const-able element) — the
///   `const` context propagates to them implicitly
///
/// Exclusions: component instantiations (user constructors are const but
/// prop expressions are not decidable here) and every widget whose SDK
/// constructor is NOT const (Container, AppBar, ListView, Image.network,
/// ... — see [`widget_has_const_constructor`]).
fn element_is_const(el: &ElementIR, ctx: &ViewContext) -> bool {
    if ctx.component_map.contains_key(el.tag.as_str()) {
        return false;
    }
    // The widget's constructor must BE const in the SDK — Container, AppBar,
    // ListView, Image.network, ... are not (`const_with_non_const` error).
    if !widget_has_const_constructor(tag_to_flutter_widget(&el.tag)) {
        return false;
    }
    if !el.events.is_empty()
        || !el.bindings.is_empty()
        || el.ref_binding.is_some()
        || !el.class_map_bindings.is_empty()
        || el.style_classes.is_some()
        || el.inline_style.is_some()
        || el.a11y.is_some()
        || el.motion_behavior().is_some_and(|m| {
            !m.triggers.is_empty() || m.in_view_once.is_some() || m.in_view_repeat.is_some()
        })
    {
        return false;
    }
    if el.props.contains_key("class") || el.props.contains_key("_design_variant") {
        return false;
    }
    for v in el.props.values() {
        if !matches!(
            v,
            ExprIR::LiteralExpr { value }
                if value.is_string() || value.is_number() || value.is_boolean() || value.is_null()
        ) {
            return false;
        }
    }
    el.children.iter().all(|c| view_node_is_const(c, ctx))
}

/// Direct-write channel wraps accumulated while formatting an element
/// (Task 14, ADR-071): the entry's D1/D2 read signals drive a
/// ListenableBuilder, and the translate shape maps to a
/// `Transform.translate` around the fully rendered widget.
#[derive(Default)]
struct StyleChannelWrap {
    /// `offset:` expression for the Transform.translate wrap
    /// (`Offset(x, y)` with signal reads resolved to storage form).
    offset: Option<String>,
    /// Sorted, deduplicated ValueNotifier names (`_dragX`) for
    /// `Listenable.merge([...])`.
    listenables: Vec<String>,
    /// Extra constructor args from Direct-channel shapes mapped to widget
    /// parameters (`width: _w.value`, `padding: EdgeInsets.all(_p.value)`).
    params: Vec<String>,
}

/// Emit one active direct-write entry's style value into the channel wraps:
///
/// * `Translate` — `"translate(xpx, ypx)"` concat → `Transform.translate`
///   wrap (`offset: Offset(x, y)`).
/// * `Direct` — single-value expression → widget parameter by CSS property
///   (width/height → number, padding/margin → EdgeInsets.all). Mapped
///   properties outside the whitelist (letterSpacing, ...) fail with a
///   compile-time diagnostic — never a silent skip.
/// * `Unmappable` — any other shape → compile-time diagnostic.
///
/// Signal reads resolve PER-SIGNAL (review HIGH-2): D1/D2 signals are
/// ValueNotifier fields (`_name.value`, Task 13) and read as such wherever
/// they appear, even inside an entry whose other reads live on the render
/// path (computed/store field — bare State name, setState rebuild). The
/// ListenableBuilder merge list collects the entry's D1/D2 members
/// regardless of entry-level classification; render-path-only entries get
/// an empty merge list and no wrap.
fn handle_channel_style(
    entry: &DirectWriteEntry,
    widget: &str,
    el: &ElementIR,
    wrap: &mut StyleChannelWrap,
    ctx: &mut ViewContext,
) {
    match match_style_pattern(&entry.value) {
        StylePattern::Translate { x, y } => {
            if style_value_is_string(&x, ctx) || style_value_is_string(&y, ctx) {
                ctx.error_at(
                    "[flutter] dynamic style transform translate reads a \
                     String-typed x/y value; Flutter Offset params require numbers \
                     (double) — use a numeric (f64) signal for the drag offset",
                    el.source_span,
                );
                return;
            }
            wrap.offset = Some(format!(
                "Offset({}, {})",
                style_json_to_dart_expr(&x, ctx),
                style_json_to_dart_expr(&y, ctx)
            ));
            collect_channel_listenables(entry, ctx, wrap);
        }
        StylePattern::Direct(v) => {
            match css_prop_to_flutter_param(&entry.prop) {
                Some("width") | Some("height") => {
                    let param = css_prop_to_flutter_param(&entry.prop).unwrap();
                    if !widget_allows_param(widget, param) {
                        ctx.warn_at(
                            format!(
                                "[flutter] dynamic style property '{}' → param '{}' is not \
                                 supported on '{}'; skipped",
                                entry.prop, param, widget
                            ),
                            el.source_span,
                        );
                        return;
                    }
                    if style_value_is_string(&v, ctx) {
                        ctx.error_at(
                            format!(
                                "[flutter] dynamic style property '{}' reads a String-typed \
                                 value; Flutter '{}' requires a number (double) — use a \
                                 numeric (f64) signal and derive the px display string \
                                 separately",
                                entry.prop, param
                            ),
                            el.source_span,
                        );
                        return;
                    }
                    wrap.params.push(format!(
                        "{}: {}",
                        entry.prop,
                        style_json_to_dart_expr(&v, ctx)
                    ));
                    collect_channel_listenables(entry, ctx, wrap);
                }
                Some("padding") | Some("margin") => {
                    if style_value_is_string(&v, ctx) {
                        ctx.error_at(
                            format!(
                                "[flutter] dynamic style property '{}' reads a String-typed \
                                 value; Flutter EdgeInsets.all requires a number (double) — \
                                 use a numeric (f64) signal",
                                entry.prop
                            ),
                            el.source_span,
                        );
                        return;
                    }
                    wrap.params.push(format!(
                        "{}: EdgeInsets.all({})",
                        entry.prop,
                        style_json_to_dart_expr(&v, ctx)
                    ));
                    collect_channel_listenables(entry, ctx, wrap);
                }
                _ => {
                    // Mapped CSS property with no Flutter parameter
                    // (letterSpacing, transform handled above, ...) — the
                    // old code silently dropped these; honest degradation
                    // (ADR-054): surface a warning instead of failing the
                    // build, the style entry is not emitted.
                    ctx.warn_at(
                        format!(
                            "[flutter] dynamic style property '{}' on a @ref'd element has no \
                             Flutter widget parameter mapping (only width/height, \
                             padding/margin, color/background-color, and the \
                             translate(xpx, ypx) transform shape are supported); \
                             the style entry was not emitted",
                            entry.prop
                        ),
                        el.source_span,
                    );
                }
            }
        }
        StylePattern::Unmappable => {
            ctx.error_at(
                format!(
                    "[flutter] dynamic style value for '{}' cannot map to any Flutter widget \
                     parameter (only single-value expressions and the translate(xpx, ypx) \
                     concat shape are supported); the style entry was not emitted",
                    entry.prop
                ),
                el.source_span,
            );
        }
    }
}

/// Whether a style VALUE is statically a String: a String literal, or a
/// `$signal` read whose `comp_type_scope` type is "String" (signals +
/// computeds). Flutter numeric params (width/height `double?`,
/// EdgeInsets.all, Offset) cannot receive Strings — emitting them produced
/// `argument_type_not_assignable` Dart (ElementRefDemo regression, 2×);
/// the compiler diagnoses instead. `$ident`/`$field`/`$call` reads are
/// statically unknown — left to runtime dispatch.
fn style_value_is_string(val: &Value, ctx: &ViewContext) -> bool {
    if let Value::Object(obj) = val {
        if let Some(name) = obj.get("$signal").and_then(|v| v.as_str()) {
            return ctx
                .comp_type_scope
                .get(name)
                .map(|t| t == "String")
                .unwrap_or(false);
        }
        return false;
    }
    matches!(val, Value::String(_))
}

/// Collect the entry's read signals that are D1/D2-stored (Task 13
/// ValueNotifier fields) into the ListenableBuilder merge list — sorted +
/// deduplicated for deterministic output. Resolution is PER-SIGNAL (review
/// HIGH-2): a B2 entry may mix a render-path read (computed/store field —
/// bare State name, setState rebuild) with D1/D2 members (ValueNotifier
/// fields) — the D1/D2 members still need their listenables so the merge
/// list is collected regardless of the entry-level B2 classification.
fn collect_channel_listenables(
    entry: &DirectWriteEntry,
    ctx: &ViewContext,
    wrap: &mut StyleChannelWrap,
) {
    for name in &entry.read_signals {
        if ctx.direct_write.d1.contains(name) || ctx.direct_write.d2.contains(name) {
            wrap.listenables.push(format!("_{}", name));
        }
    }
    wrap.listenables.sort();
    wrap.listenables.dedup();
}

/// Convert a style VALUE tree (the direct-write entry's JSON, shape from
/// `lower_expr_to_json_from_ir`) to a Dart expression. Signal reads resolve
/// to their storage form PER-SIGNAL (review HIGH-2): D1/D2 signals are
/// ValueNotifier fields (`_name.value`, Task 13) wherever they are read —
/// even inside a B2 entry mixed with render-path reads; every other read
/// (computed, store field, ident) is a bare State name resolved on the
/// render path (setState rebuilds).
fn style_json_to_dart_expr(val: &Value, ctx: &ViewContext) -> String {
    if let Value::Object(obj) = val {
        if let Some(name) = obj.get("$signal").and_then(|v| v.as_str()) {
            if ctx.direct_write.d1.contains(name) || ctx.direct_write.d2.contains(name) {
                return format!("_{}.value", name);
            }
            return name.to_string();
        }
        if let Some(name) = obj.get("$ident").and_then(|v| v.as_str()) {
            return name.to_string();
        }
        if let Some(arr) = obj.get("$field").and_then(|v| v.as_array()) {
            if let Some(field) = arr.get(1).and_then(|v| v.as_str()) {
                return format!("{}.{}", style_json_to_dart_expr(&arr[0], ctx), field);
            }
        }
        if let Some(call) = obj.get("$call").and_then(|v| v.as_object()) {
            let name = call.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = call
                .get("args")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .map(|x| style_json_to_dart_expr(x, ctx))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            return format!("{}({})", name, args);
        }
    }
    match val {
        Value::Number(n) => format!("{}", n),
        Value::String(s) => dart_string_literal(s),
        Value::Bool(b) => b.to_string(),
        _ => "0.0".to_string(),
    }
}

/// Format an ElementIR node as a Flutter widget constructor expression.
/// Wraps with motion animation widgets (AnimatedBuilder, MouseRegion, GestureDetector)
/// when the element has motion triggers.
fn format_element(el: &ElementIR, motions: &[MotionIntent], ctx: &mut ViewContext) -> String {
    // Component composition: check if tag is a known component
    if let Some(_child_comp) = ctx.component_map.get(el.tag.as_str()) {
        let mut child_args: Vec<String> = Vec::new();
        // @drag (ADR-073): the pan wiring lives on the rendered element's
        // GestureDetector wrapper — component instances take callback props
        // instead (no wrapper machinery on this path). Warn + skip (mirrors
        // the React backend's component-instance handling).
        if el.drag_binding.is_some() {
            ctx.warn_at(
                "[flutter] @drag 在组件实例上不支持（组件实例走回调 prop 通道，无 GestureDetector 包装）——@drag 已跳过，请在原生元素上使用",
                el.source_span,
            );
        }
        // Ref binding (Step 6): GlobalKey attaches to any widget, including
        // custom component widgets.
        if let Some(ref_name) = &el.ref_binding {
            child_args.push(format!("key: {}", ref_key_name(ref_name)));
        }
        // Props become constructor parameters. `class` and `_design_variant`
        // are design-class/variant carriers resolved by
        // `wrap_design_decorations` below — never constructor params (the
        // child component declares no such field, and the generated Dart
        // would not compile).
        for (prop_name, prop_value) in &el.props {
            if prop_name == "class" || prop_name == "_design_variant" {
                continue;
            }
            child_args.push(format!("{}: {}", prop_name, ctx.format_expr(prop_value)));
        }
        // Events become callback parameters. Task 5.8: a FnRef passes the
        // handler by TEAR-OFF — `onSwitch: switchTab` — instead of a closure
        // that dropped the event payload (`() { switchTab(); }` → the
        // callback's parameter was never forwarded). Refs naming the current
        // component's OWN event/prop (event forwarding `@onNavigate={onNavigate}`)
        // tear off the widget field: `onNavigate: widget.onNavigate`.
        for (event_name, handler) in &el.events {
            let cb = match handler {
                EventHandlerIR::FnRef { r#ref, .. } => {
                    if ctx.widget_names.contains(r#ref) {
                        format!("widget.{}", r#ref)
                    } else {
                        let renamed = ctx
                            .fn_renames
                            .get(r#ref)
                            .cloned()
                            .unwrap_or_else(|| r#ref.clone());
                        let payload = ctx.child_event_payload(el.tag.as_str(), event_name);
                        let payload_nullable = payload
                            .as_deref()
                            .is_some_and(|p| p.ends_with('?') || p.ends_with(" | undefined"));
                        let payload_void = payload
                            .as_deref()
                            .is_none_or(|p| p.is_empty() || p == "void");
                        if ctx.ref_is_zero_param_fn(r#ref) {
                            // 0-arg fn: the child's payload cannot flow — a
                            // 0-arg tear-off is not assignable to the
                            // payload-bearing callback type. The wrapper's
                            // arity must MATCH the child's payload (a void
                            // payload forbids a `(v)` closure).
                            if payload_void {
                                format!("() {{ {renamed}(); }}")
                            } else {
                                format!("(v) {{ {renamed}(); }}")
                            }
                        } else if payload_nullable {
                            // Nullable payload vs a non-nullable handler
                            // param: the tear-off type does not match
                            // (`dynamic Function(String)` vs
                            // `void Function(String?)?`) — cast the payload.
                            let param = ctx
                                .ref_single_param_type(r#ref)
                                .unwrap_or_else(|| "dynamic".into());
                            format!("(v) {{ {renamed}(v as {param}); }}")
                        } else {
                            renamed.to_string()
                        }
                    }
                }
                EventHandlerIR::InlineFn { body, .. } => format_inline_handler_flutter(body, ctx),
                EventHandlerIR::StoreAction { r#ref, .. } => format_store_action_handler(r#ref),
            };
            let stem = event_name.strip_prefix("on").unwrap_or(event_name);
            child_args.push(format!("on{}: {}", capitalize_first(stem), cb));
        }
        // Slot projection (Task 5.3): children carrying a literal
        // `slot="name"` prop, or a consumer-position `@slot name { ... }`
        // directive, matching a slot the component declares, project into
        // the named constructor param; everything else flows into the
        // unnamed `children` param. `slot` props are stripped so they never
        // reach the generated widget (mirrors the React backend).
        let target_slots = ctx
            .component_slots
            .get(el.tag.as_str())
            .cloned()
            .unwrap_or_default();
        let mut default_children: Vec<String> = Vec::new();
        for child in &el.children {
            let slot_name = child_slot_name(child);
            let content = match child {
                // Consumer-position `@slot { ... }` — the fallback IS the
                // projected content; a bare `@slot name` projects nothing.
                ViewTree::Slot { fallback, .. } => format_slot_content(fallback, motions, ctx),
                _ => Some(format_view_tree(&strip_slot_prop(child), motions, ctx)),
            };
            let Some(content) = content else { continue };
            match slot_name.as_deref() {
                Some(name) if target_slots.contains(name) => {
                    child_args.push(format!("{}: {}", name, content));
                }
                Some(name) => {
                    ctx.warn_at(
                        format!(
                            "[flutter] component '{}' has no slot '{}'; content emitted as default child",
                            el.tag, name
                        ),
                        el.source_span,
                    );
                    default_children.push(content);
                }
                None => default_children.push(content),
            }
        }
        // Unnamed content projects into the `children` param — only when
        // the component declares the unnamed slot; otherwise the generated
        // Dart would be invalid, so warn and drop. The slot param is a
        // single `Widget?` — multi-node content is Column-wrapped (a bare
        // list literal would not match the parameter type).
        let args_str = child_args.join(", ");
        let child_expr = if default_children.is_empty() {
            format!("{}({})", el.tag, args_str)
        } else if target_slots.contains("children") {
            let children_arg = if default_children.len() == 1 {
                default_children[0].clone()
            } else {
                wrap_multi_widget(&default_children)
            };
            format!("{}({}, children: {})", el.tag, args_str, children_arg)
        } else {
            ctx.warn_at(
                format!(
                    "[flutter] component '{}' declares no default slot; {} child node(s) dropped",
                    el.tag,
                    default_children.len()
                ),
                el.source_span,
            );
            format!("{}({})", el.tag, args_str)
        };
        // Design classes/variants resolve to a decoration on a wrapping
        // Container — real visual output on component instantiations too.
        let decorated = wrap_design_decorations(el, child_expr, ctx);
        // a11y block → Semantics/ExcludeSemantics wrappers around the
        // component widget (the semantic node describes the whole subtree).
        return wrap_a11y(el, decorated, ctx);
    }
    // @modal (Task 5 F1): `<dialog @modal={openSig}>` renders a dialog
    // route. The element is driven by the component State's
    // `bool _modalShown` idempotence snapshot (declared in mod.rs when
    // has_modal_bindings): a post-frame callback polls the signal — on a
    // rising edge it pushes showDialog (FocusTraversalGroup for a11y
    // traversal, barrierDismissible keeps outside-tap semantics), on a
    // falling edge it pops via the root navigator (showDialog uses
    // rootNavigator: true by default). The `.then` on the route future
    // syncs the signal back to false on ANY pop — barrier dismiss, system
    // back, programmatic pop — keeping the controlled signal the single
    // source of truth. Honest note: no Esc-key handling on desktop (Global
    // Constraints 7 — no new listeners); the mobile system back button and
    // barrier tap are covered by the route's own pop.
    if let Some(modal) = &el.modal {
        let motion_idx = if is_flutter_motion_element(el, ctx.component_map) {
            let idx = *ctx.motion_counter;
            *ctx.motion_counter += 1;
            Some(idx)
        } else {
            None
        };
        // Slot projection (document order, mirrors the React backend):
        // named @slot fallbacks first — the slot set is not limited to
        // header/body/footer — then anonymous children fall through into
        // the body slot. Lowering keeps the @slot directives in
        // el.children AND in modal.slots (identical content), so
        // ViewTree::Slot nodes are skipped here to avoid double render.
        let mut parts: Vec<String> = Vec::new();
        for slot in &modal.slots {
            if let Some(content) = format_slot_content(&slot.fallback, motions, ctx) {
                parts.push(content);
            }
        }
        for child in &el.children {
            if matches!(child, ViewTree::Slot { .. }) {
                continue;
            }
            parts.push(format_view_tree(child, motions, ctx));
        }
        let mut content = match parts.len() {
            0 => "SizedBox.shrink()".to_string(),
            1 => parts.remove(0),
            _ => wrap_multi_widget(&parts),
        };
        if let Some(midx) = motion_idx {
            content = wrap_with_motion_widgets(
                content,
                el.motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers),
                motions,
                midx,
            );
        }
        let ref_arg = el
            .ref_binding
            .as_ref()
            .map(|n| format!("key: {}, ", ref_key_name(n)))
            .unwrap_or_default();
        let sig = &modal.open_signal;
        let (read_expr, write_false) = if ctx.direct_write_signals.contains(sig) {
            (
                format!("_{}.value", sig),
                format!("_{}.value = false;", sig),
            )
        } else {
            (sig.clone(), format!("setState(() {{ {} = false; }});", sig))
        };
        let out = format!(
            "Builder(builder: (context) {{\n\
             \x20\x20WidgetsBinding.instance.addPostFrameCallback((_) {{\n\
             \x20\x20\x20\x20if (!mounted) return;\n\
             \x20\x20\x20\x20if ({read_expr}) {{\n\
             \x20\x20\x20\x20\x20\x20if (!_modalShown) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20_modalShown = true;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20showDialog(context: context, barrierDismissible: true, builder: (context) => FocusTraversalGroup(child: Dialog({ref_arg}child: {content}))).then((_) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20if (!mounted) return;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20_modalShown = false;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20{write_false}\n\
             \x20\x20\x20\x20\x20\x20\x20\x20}});\n\
             \x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20}} else {{\n\
             \x20\x20\x20\x20\x20\x20if (_modalShown) {{\n\
             \x20\x20\x20\x20\x20\x20\x20\x20_modalShown = false;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20Navigator.of(context, rootNavigator: true).pop();\n\
             \x20\x20\x20\x20\x20\x20}}\n\
             \x20\x20\x20\x20}}\n\
             \x20\x20}});\n\
             \x20\x20return const SizedBox.shrink();\n\
             }})"
        );
        return out;
    }
    // @skeleton (Task 5 F3): static placeholder — grey block/circle with
    // ZERO shimmer dependency (honest degradation: the plan forbids new
    // packages, so no shimmer animation is possible). `circle` → 48x48
    // grey circle; every other shape (text/line/card, default text) →
    // 16-high rounded grey rect. The element is a placeholder: its own
    // children are not rendered.
    if let Some(skeleton) = &el.skeleton {
        let motion_idx = if is_flutter_motion_element(el, ctx.component_map) {
            let idx = *ctx.motion_counter;
            *ctx.motion_counter += 1;
            Some(idx)
        } else {
            None
        };
        let shape = skeleton.shape.as_deref().unwrap_or("text");
        let placeholder = if shape == "circle" {
            "Container(width: 48, height: 48, decoration: BoxDecoration(shape: BoxShape.circle, color: Color(0xFFE0E0E0)))"
                .to_string()
        } else {
            "Container(height: 16, decoration: BoxDecoration(color: Color(0xFFE0E0E0), borderRadius: BorderRadius.circular(4)))"
                .to_string()
        };
        let mut out = format!(
            "// 注记：@skeleton 静态占位（诚实降级——零 shimmer 依赖，纯灰块）\n{}",
            placeholder
        );
        if let Some(midx) = motion_idx {
            out = wrap_with_motion_widgets(
                out,
                el.motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers),
                motions,
                midx,
            );
        }
        return out;
    }
    // @errorBoundary (Task 5 F4): honest-degradation empty shell — Flutter
    // has no ErrorBoundary primitive, so the element emits NO wrapper: the
    // content children render exactly as if the directive were absent, and
    // the `@slot fallback` fallback UI is NOT rendered (a silent fallback
    // would mask render failures). The note comment keeps the directive
    // visible in generated code for source-mapping / debugging.
    if el.error_boundary.is_some() {
        let motion_idx = if is_flutter_motion_element(el, ctx.component_map) {
            let idx = *ctx.motion_counter;
            *ctx.motion_counter += 1;
            Some(idx)
        } else {
            None
        };
        // Slot projection: drop the @slot fallback (not rendered), render
        // the remaining children exactly as the HTML-mapping path would.
        let mut parts: Vec<String> = Vec::new();
        for child in &el.children {
            if matches!(child, ViewTree::Slot { .. }) {
                continue;
            }
            parts.push(format_view_tree(child, motions, ctx));
        }
        let content = match parts.len() {
            0 => "SizedBox.shrink()".to_string(),
            1 => parts.remove(0),
            _ => wrap_multi_widget(&parts),
        };
        let mut out = format!(
            "// 注记：@errorBoundary 空壳（诚实降级——Flutter 无 ErrorBoundary 原语，渲染失败不兜底，fallback 槽不渲染；数据级错误请走 @load error 信号）\n{}",
            content
        );
        if let Some(midx) = motion_idx {
            out = wrap_with_motion_widgets(
                out,
                el.motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers),
                motions,
                midx,
            );
        }
        return out;
    }
    // Otherwise, fall through to HTML tag mapping
    // @form (Task 5): inside an @form component, `<input>` maps to
    // `TextFormField` so bound fields get `validator` (Form validation only
    // works on FormField descendants).
    let widget = form_field_widget(ctx, el, tag_to_flutter_widget(&el.tag));
    let mut args: Vec<String> = Vec::new();

    // T5.7: motion-element ordinal — every element that will receive motion
    // wrappers is counted (same rule/order as `collect_motion_element_usage`
    // in mod.rs, so declarations/initState/dispose agree on the names).
    let motion_idx = if is_flutter_motion_element(el, ctx.component_map) {
        let idx = *ctx.motion_counter;
        *ctx.motion_counter += 1;
        Some(idx)
    } else {
        None
    };

    // Props -> widget constructor parameters (C6: per-widget whitelist —
    // unmapped params emit a compiler warning instead of invalid Dart).
    // Params that belong to a sub-object (TextStyle/InputDecoration) are
    // collected and merged into a single wrapper argument.
    let mut wrapped: Vec<(&'static str, Vec<String>)> = Vec::new();
    // Task 3.10 (flutter LOW): a relative/asset `src` degrades the emitted
    // constructor from Image.network to Image.asset (see the src branch
    // below) — every whitelist/event/alt check keeps the Image.network
    // param set, because Image.asset shares it; only the constructor NAME
    // emitted at the tail honors the switch.
    let mut use_asset_image = false;
    // Wrapper widgets applied around the fully-rendered element — filled by
    // BOTH the props loop (C2b #4: `title` → Tooltip) and the events loop
    // (C2b #3: MouseRegion/Focus/GestureDetector); applied below.
    let mut wrap_args: Vec<(&'static str, Vec<(String, String)>)> = Vec::new();
    for (name, value) in &el.props {
        // `_design_variant` is a design-class/variant carrier resolved by
        // `wrap_design_decorations` — never a widget param (the `class` prop
        // maps to "" via the slug table below and is dropped there).
        if name == "_design_variant" {
            continue;
        }
        // Image.network takes the URL as a POSITIONAL argument —
        // `src` maps to the first positional arg, NOT a named `image:`
        // param (`Image.network(image: ...)` is invalid Dart — the named
        // param list has no `image`; the fixture gate surfaced this).
        if widget == "Image.network" && name == "src" {
            let src_expr = ctx.format_expr(value);
            // Task 3.10 (flutter LOW): a web-relative src (`/dot.png`) thrown
            // at Image.network throws NetworkImageLoadException at runtime in
            // a NATIVE app (relative paths only resolve in the web harness).
            // Absolute http(s) URLs keep Image.network; anything else
            // degrades to Image.asset with an honest warning — the file must
            // live under assets/ and be referenced by its asset key.
            let absolute_url = match value {
                ExprIR::LiteralExpr { value } => value
                    .as_str()
                    .is_some_and(|s| s.starts_with("http://") || s.starts_with("https://")),
                // Non-literal src (interpolation/store read) is not
                // statically decidable — keep Image.network, the author's
                // explicit network intent.
                _ => true,
            };
            if !absolute_url {
                use_asset_image = true;
                let shown = match value {
                    ExprIR::LiteralExpr { value } => value.as_str().unwrap_or("?"),
                    _ => "?",
                };
                ctx.warn_at(
                    format!(
                        "[flutter] img src '{shown}' is not an absolute http(s) URL — \
                         Image.network throws NetworkImageLoadException at runtime in \
                         a native app; emitting Image.asset (move the file under \
                         assets/ and reference it by its asset key)"
                    ),
                    el.source_span,
                );
            }
            args.insert(0, src_expr);
            continue;
        }
        // Icon's `icon` positional argument is REQUIRED — `Icon()` with no
        // args is invalid Dart (review HIGH-4). `name`/`icon` props map to
        // `Icons.<name>` via the known-name map; unmappable names warn and
        // fall back to a compile-able placeholder.
        if widget == "Icon" && (name == "name" || name == "icon") {
            let known = match value {
                ExprIR::LiteralExpr { value } => value.as_str().and_then(icon_name_to_dart),
                _ => None,
            };
            match known {
                Some(icon) => args.insert(0, icon.to_string()),
                None => {
                    let bad = match value {
                        ExprIR::LiteralExpr { value } => value
                            .as_str()
                            .map(|s| format!("'{s}'"))
                            .unwrap_or_else(|| "(non-string literal)".into()),
                        _ => "(non-literal)".into(),
                    };
                    ctx.warn_at(
                        format!(
                            "[flutter] icon name {bad} has no Material Icons mapping; \
                             falling back to Icons.help_outline"
                        ),
                        el.source_span,
                    );
                    args.insert(0, "Icons.help_outline".to_string());
                }
            }
            continue;
        }
        let param = widget_param_rename(widget, prop_to_flutter_param_name(name));
        if param.is_empty() {
            continue; // slug props (class/style/id/value/...) — intentionally dropped
        }
        // T5.7: literal values convert to typed expressions for the params
        // that need them (`fontWeight:"bold"` → FontWeight.w700,
        // `color:"#ff0000"` → Color(...), `textAlign:"center"` → TextAlign.*,
        // ...). Unmappable literals keep the raw value and warn below.
        let formatted = match convert_literal_prop_value(widget, param, value) {
            Some(converted) => converted,
            None => ctx.format_expr(value),
        };
        if let Some((wrapper, member)) = widget_param_wrapper(widget, param) {
            match wrapped.iter_mut().find(|(w, _)| *w == wrapper) {
                Some((_, members)) => members.push(format!("{}: {}", member, formatted)),
                None => wrapped.push((wrapper, vec![format!("{}: {}", member, formatted)])),
            }
            continue;
        }
        // Task C2b (#4): button widgets have no `enabled` param — a
        // disabled button is CALLBACK-LESS, so `disabled` maps to the
        // required param's null form (`onPressed: null` renders the button
        // disabled). `disabled={false}` emits nothing (the button stays
        // enabled); non-literal values keep the honest warning below.
        if name == "disabled" && matches!(widget, "ElevatedButton" | "TextButton" | "IconButton") {
            if let ExprIR::LiteralExpr { value } = value {
                if value.as_bool() == Some(true) || value.is_null() {
                    args.push("onPressed: null".into());
                }
                continue;
            }
        }
        // Task C2b (#4): `data-testid` maps to `key: ValueKey('...')` —
        // the Flutter testing hook (widget tests find elements by key).
        // Also accepts the untranslated source name `testId` — the Web
        // backends translate it to `data-testid` at codegen, but Flutter
        // consumes the raw IR attribute, so without this the documented
        // `testId → ValueKey` mapping (learn-omniview showcase) degraded
        // to a "param not supported" warning on every widget.
        // Skipped when a `key` prop already emitted the arg, or when the
        // element's ref binding occupies the slot (the ref's GlobalKey is
        // pushed LATER in the ref section — two `key:` args would not
        // compile; the GlobalKey must win, and the drop is an honest
        // warning, never silent — review MEDIUM-2).
        if name == "data-testid" || name == "testId" {
            if let ExprIR::LiteralExpr { value } = value {
                if let Some(s) = value.as_str() {
                    if el.ref_binding.is_some() {
                        ctx.warn_at(
                            format!(
                                "[flutter] prop '{name}' is dropped on '{widget}': \
                                 the element's ref binding occupies the key slot"
                            ),
                            el.source_span,
                        );
                        continue;
                    }
                    if !args.iter().any(|a| a.starts_with("key:")) {
                        args.push(format!("key: ValueKey({})", dart_string_literal(s)));
                    }
                    continue;
                }
            }
        }
        if !widget_allows_param(widget, param) {
            // Task C2b (#4): `title` on widgets with no title/tooltip param
            // wraps the element in a Tooltip — the DOM title attribute's
            // Flutter analogue (hover / long-press hint).
            if name == "title" {
                wrap_push(&mut wrap_args, "Tooltip", "message", &formatted);
                continue;
            }
            ctx.warn_at(
                format!(
                    "[flutter] prop '{name}' → param '{param}' is not supported on '{widget}'; skipped"
                ),
                el.source_span,
            );
            continue;
        }
        args.push(format!("{}: {}", param, formatted));
    }

    // Task 3.5: `<textarea>` is a multiline intent — `TextField(maxLines: 4)`
    // (a bare `<textarea/>` rendered `Container()` before: invisible). An
    // explicit `maxLines` prop wins; the default applies only to the
    // textarea tag, not to single-line `<input>`s.
    if el.tag == "textarea" && !args.iter().any(|a| a.starts_with("maxLines:")) {
        args.push("maxLines: 4".into());
    }

    // a11y block (design doc 域 10): `alt` on image widgets maps to the
    // Image.network `semanticLabel` constructor param — the same mapping as
    // the plain `alt` prop (utils.rs prop_to_flutter_param_name). Skipped
    // when a literal `alt` prop already carries the semanticLabel, so the
    // generated Dart never duplicates the named argument.
    if widget == "Image.network" {
        if let Some(alt) = el.a11y.as_ref().and_then(|a| a.alt.as_ref()) {
            if !el.props.contains_key("alt") {
                args.push(format!("semanticLabel: {}", ctx.format_expr(alt)));
            }
        }
    }

    // Inline style (style: { prop: value }) — map CSS properties to Flutter widget parameters
    // Task 14 (ADR-071 direct-write channel): channel wraps accumulated from
    // active entries on this element — applied around the FULLY rendered
    // widget (after the event-wrapper stack) at the end of format_element.
    let mut channel_wrap = StyleChannelWrap::default();
    if let Some(obj) = el
        .inline_style
        .as_ref()
        .and_then(|style_json| style_json.as_object())
    {
        for (css_prop, css_val) in obj {
            // Task 14: the direct-write channel takes precedence over the
            // static-value path — an active entry's dynamic value is never
            // silently skipped (`Some(_) => skip` was the old behavior).
            let channel_ref = el.ref_binding.as_deref().unwrap_or("");
            if let Some(entry) = ctx
                .direct_write
                .entries
                .iter()
                .find(|e| e.ref_name == channel_ref && e.prop == *css_prop)
            {
                handle_channel_style(entry, widget, el, &mut channel_wrap, ctx);
                continue;
            }
            let css_val_str = css_val.as_str().unwrap_or("");
            // T5.7: inline-style props go through the same per-widget
            // whitelist as plain props — `Text(style: { color: ... })` used
            // to emit `color: Color(...)` on a widget with no `color` param
            // (invalid Dart; review's String->Color error class).
            let inline_param = css_prop_to_flutter_param(css_prop);
            if let Some(param) = inline_param {
                // Task C2b (#8): Text inline styles route through the
                // TextStyle wrapper — `style: { color: ... }` →
                // `style: TextStyle(color: ...)` (Text has no color param;
                // the wrapper merges with the props' TextStyle members).
                if let Some((wrapper, member)) = widget_param_wrapper(widget, param) {
                    if param == "color" {
                        // CSS `background` on a text element is a BACKGROUND
                        // color — route it to TextStyle.backgroundColor;
                        // `color` stays the text color. Without this the two
                        // props collided on one `color:` member
                        // (TextStyle(color: bg, color: fg) — duplicate arg).
                        let member = if css_prop == "background" {
                            "backgroundColor"
                        } else {
                            member
                        };
                        if let Some(color_expr) = css_color_to_dart(css_val_str) {
                            match wrapped.iter_mut().find(|(w, _)| *w == wrapper) {
                                Some((_, members)) => {
                                    members.push(format!("{}: {}", member, color_expr))
                                }
                                None => wrapped
                                    .push((wrapper, vec![format!("{}: {}", member, color_expr)])),
                            }
                            continue;
                        }
                        // Unsupported color value — falls through to the
                        // existing color-value warning below.
                    } else {
                        // font*/other wrapped members — no inline value
                        // converter; keep the honest warning below.
                    }
                }
                if !widget_allows_param(widget, param) {
                    ctx.warn_at(
                        format!(
                            "[flutter] inline style property '{css_prop}' → param '{param}' \
                             is not supported on '{widget}'; skipped"
                        ),
                        el.source_span,
                    );
                    continue;
                }
            }
            match inline_param {
                Some("padding") | Some("margin") => {
                    if css_val_str.ends_with("px")
                        || css_val_str.ends_with("em")
                        || css_val_str.ends_with("rem")
                        || css_val_str.ends_with("%")
                        || css_val_str.ends_with("pt")
                    {
                        let num_str = css_value_to_dart_number(css_val_str);
                        args.push(format!("{}: EdgeInsets.all({})", css_prop, num_str));
                    } else if let Ok(num) = css_val_str.parse::<f64>() {
                        args.push(format!("{}: EdgeInsets.all({})", css_prop, num));
                    } else {
                        args.push(format!(
                            "{}: EdgeInsets.all({})",
                            css_prop,
                            css_value_to_dart_number(css_val_str)
                        ));
                    }
                }
                Some("width") | Some("height") => {
                    let num_str = css_value_to_dart_number(css_val_str);
                    args.push(format!("{}: {}", css_prop, num_str));
                }
                Some("color") => {
                    // C5: CSS colors → Dart Color literals (#ff0000 → Color(0xFFFF0000))
                    // A non-text widget's `color:` slot IS its background —
                    // when both `background` and `color` (DOM foreground,
                    // no Container equivalent) appear, `background` owns
                    // the slot deterministically (map iteration order is
                    // unstable).
                    if css_prop == "color" && obj.contains_key("background") {
                        continue;
                    }
                    match css_color_to_dart(css_val_str) {
                        Some(color_expr) => args.push(format!("color: {}", color_expr)),
                        None => ctx.warn_at(
                            format!(
                                "[flutter] unsupported color value '{css_val_str}' for '{css_prop}'; skipped"
                            ),
                            el.source_span,
                        ),
                    }
                }
                Some(_) => {
                    // Known Flutter param but unexpected value format — skip
                }
                None => {
                    // Unsupported CSS property — skip
                }
            }
        }
    }

    // Merge wrapped params into sub-object constructor args — AFTER the
    // inline-style loop (C2b #8: Text inline `color` joins the props'
    // TextStyle members into ONE `style: TextStyle(...)` arg).
    for (wrapper, members) in wrapped {
        let arg_name = match wrapper {
            "TextStyle" => "style",
            "InputDecoration" => "decoration",
            _ => wrapper,
        };
        args.push(format!("{}: {}({})", arg_name, wrapper, members.join(", ")));
    }

    // Task 14: Direct-channel params (width/height/padding/margin from
    // dynamic expressions) join the constructor args like static values.
    args.append(&mut channel_wrap.params);

    // Events -> callback parameters (C6: per-widget event whitelist)
    //
    // Task 3.6: tap-family events on generic elements (Container/Text/...)
    // have NO whitelisted callback — instead of silently dropping the
    // handler (flutter HIGH: ThemeSwitcher's theme cards emitted no tap
    // target at all), the rendered element wraps in a GestureDetector
    // carrying the tap callback. Task C2b (#3) extends the wrapper set:
    // pointer enter/exit/hover wrap in a `MouseRegion`, focus/blur and
    // key events wrap in a `Focus`. `wrap_args` (declared above the props
    // loop — `title` → Tooltip joins it) collects (wrapper, [(param,
    // body)]) pairs; the wrappers are applied around the fully-rendered
    // element below in a deterministic nesting order.
    // onFocus + onBlur on the same element merge into ONE
    // Focus(onFocusChange:) closure — `(v) { if (v) {...} else {...} }`.
    let mut focus_change_bodies: Vec<(&'static str, String)> = Vec::new();
    // Focus(onKeyEvent:) — one callback per element (a second key event
    // would collide on the single param; warn and drop).
    let mut key_wrap_body: Option<String> = None;
    for (name, handler) in &el.events {
        // @form (Task 5): the `submit` event is the form-level submit intent
        // — a Flutter Form has no submit callback; submission is intercepted
        // on the type="submit" button's onPressed (validation gate). Consumed
        // silently: warning on the canonical @form pairing is noise.
        if el.tag == "form" && name == "submit" && ctx.form_present {
            continue;
        }
        let callback_name = event_to_flutter_callback(name);
        let direct_ok = widget_allows_event(widget, &callback_name);
        // Image.network's errorBuilder is the Flutter analogue of the DOM
        // `error` event — a builder that REPLACES the failed image and must
        // return a Widget. The handler body runs on failure, then a
        // placeholder keeps the build valid. Checked BEFORE the wrapper
        // decision: errorBuilder is a direct Image.network constructor arg.
        if widget == "Image.network" && callback_name == "onError" {
            match format_error_builder_handler(handler, ctx) {
                Some(builder) => args.push(format!("errorBuilder: {builder}")),
                None => ctx.warn_at(
                    format!(
                        "[flutter] event '{name}' → callback '{callback_name}' is not supported on '{widget}'; skipped"
                    ),
                    el.source_span,
                ),
            }
            continue;
        }
        let wrapper = if direct_ok {
            None
        } else {
            generic_event_wrapper_callback(&callback_name)
        };
        if !direct_ok && wrapper.is_none() {
            ctx.warn_at(
                format!(
                    "[flutter] event '{name}' → callback '{callback_name}' is not supported on '{widget}'; skipped"
                ),
                el.source_span,
            );
            continue;
        }
        // Focus.onKeyEvent takes `KeyEventResult Function(FocusNode,
        // KeyEvent)` — a param-bearing fn ref cannot satisfy the signature
        // AND its `e.key`-style accesses cannot read a raw KeyEvent — honest
        // degradation (C2b #3). The callback fires for BOTH keydown and
        // keyup events — discriminate by event type so an `@onKeyUp` handler
        // does not run on presses (review LOW-1).
        let key_event = matches!(wrapper, Some(EventWrapperKind::KeyEvent));
        let key_event_kind = match callback_name.as_str() {
            "onKeyDown" => Some("KeyDownEvent"),
            "onKeyUp" => Some("KeyUpEvent"),
            _ => None,
        };
        if key_event {
            if let EventHandlerIR::FnRef { r#ref, .. } = handler {
                if !ctx.widget_names.contains(r#ref) && ctx.ref_takes_params(r#ref) {
                    ctx.warn_at(
                        format!(
                            "[flutter] event '{name}' → callback '{callback_name}' is a Flutter \
                             key-event callback (Focus.onKeyEvent takes (FocusNode, KeyEvent) and \
                             returns KeyEventResult); param-bearing handler '{ref}' — dropped",
                            ref = r#ref
                        ),
                        el.source_span,
                    );
                    continue;
                }
            }
        }
        // FocusChange bodies are STATEMENTS — the merged closure's bool `v`
        // discriminates focus gain vs loss; a closure body would never run.
        if let Some(EventWrapperKind::FocusChange(which)) = wrapper {
            let Some(stmts) = format_focus_change_body(handler, ctx) else {
                ctx.warn_at(
                    format!(
                        "[flutter] event '{name}' → callback '{callback_name}' is not supported on '{widget}'; skipped"
                    ),
                    el.source_span,
                );
                continue;
            };
            focus_change_bodies.push((which, stmts));
            continue;
        }
        // VoidCallback-family params (onPressed/onLongPress/...) receive NO
        // event argument in Flutter — the closure must be parameterless or
        // the generated Dart fails to compile (argument_type_not_assignable).
        let is_void = callback_is_void(&callback_name);
        // Key-event callbacks receive the raw KeyEvent — the field-extraction
        // machinery names it `event` (Focus.onKeyEvent's second param).
        let raw_var = if key_event { "event" } else { "e" };
        let body: Option<String> = match handler {
            EventHandlerIR::FnRef { r#ref, .. } => {
                // @load (Task 8): `retry` / `reset` are reserved refs on @load
                // components — the map values are the FINAL closure forms
                // (`() => ref.invalidate({key}Provider)` /
                // `() => ref.read({key}Provider.notifier).reset()`), single
                // source: FnRef binding, value form, and call form all resolve
                // through ctx.load_builtins.
                if let Some(v) = ctx.load_builtins.get(r#ref) {
                    Some(v.clone())
                } else {
                    // Task 5.8: pass the handler by TEAR-OFF — `onChanged:
                    // syncMirror` — instead of `(event) { syncMirror({}); }`
                    // (an empty event-object literal was passed to every fn ref,
                    // producing extra_positional_arguments / type errors). Dart's
                    // function subtyping accepts both arities; the payload flows
                    // through the callback's own parameters. Field accesses on
                    // the event are dropped — warn when the ref's target fn uses
                    // them (unrepresentable through a tear-off).
                    if handler
                        .used_fields()
                        .as_ref()
                        .is_some_and(|f| !f.is_empty())
                    {
                        warn_dropped_event_fields(
                            ctx,
                            name,
                            &callback_name,
                            &format!("'{ref}'"),
                            el.source_span,
                        );
                    }
                    if ctx.widget_names.contains(r#ref) {
                        if key_event {
                            // A tear-off cannot satisfy (FocusNode, KeyEvent) →
                            // KeyEventResult — wrap it in a call.
                            Some(format!(
                                "(node, event) {{ widget.{}(); return KeyEventResult.ignored; }}",
                                r#ref
                            ))
                        } else {
                            Some(format!("widget.{}", r#ref))
                        }
                    } else {
                        let renamed = ctx
                            .fn_renames
                            .get(r#ref)
                            .cloned()
                            .unwrap_or_else(|| r#ref.clone());
                        let zero_param = ctx.ref_is_zero_param_fn(r#ref);
                        if is_void && ctx.ref_takes_params(r#ref) {
                            // Task 3.6: a param-bearing fn CANNOT wire to a
                            // 0-arity VoidCallback-family param — its tear-off
                            // (`dynamic Function(dynamic)`) is not assignable to
                            // `GestureTapCallback?` (argument_type_not_assignable;
                            // Modal's `handleContentClick(e)` surfaced this on the
                            // new tap wrapper) and the payload cannot flow INTO a
                            // void callback either — honest degradation: warn and
                            // drop (mirrors the direct-param path).
                            ctx.warn_at(
                                format!(
                                    "[flutter] event '{name}' → callback '{callback_name}' is a \
                                 VoidCallback in Flutter (no event argument); handler \
                                 '{ref}' takes an event parameter — dropped"
                                ),
                                el.source_span,
                            );
                            None
                        } else if !is_void && zero_param {
                            // A 0-arg fn tear-off is not assignable to the
                            // payload-bearing callback types (`ValueChanged<String>`)
                            // — accept the event param and ignore it. Key events
                            // additionally need the (node, event) signature and a
                            // KeyEventResult return (C2b #3).
                            Some(if key_event {
                                format!(
                                "(node, event) {{ {renamed}(); return KeyEventResult.ignored; }}"
                            )
                            } else {
                                format!("(event) {{ {renamed}(); }}")
                            })
                        } else {
                            Some(renamed.to_string())
                        }
                    }
                }
            }
            EventHandlerIR::InlineFn {
                body, event_data, ..
            } => {
                if event_data.is_some() && !is_void {
                    let _event_obj = emit_event_object_flutter(
                        handler.event_data(),
                        handler.used_fields(),
                        raw_var,
                        ctx.warnings,
                    );
                    let fields: Vec<String> = match handler.used_fields() {
                        Some(ref f) if !f.is_empty() => f.clone(),
                        _ => vec![],
                    };
                    let mut hw = CodeWriter::new();
                    // Key events (C2b #3) run under the Focus.onKeyEvent
                    // signature `(FocusNode, KeyEvent) → KeyEventResult`.
                    hw.emit(if key_event {
                        "(node, event) {"
                    } else {
                        "(e) {"
                    });
                    hw.indent();
                    // Emit individual field extractions as local variables
                    // This shadows raw_var.field references in the body with
                    // extracted values
                    for field in &fields {
                        let extract = field_extract_flutter(
                            handler.event_data().as_ref().unwrap(),
                            field,
                            raw_var,
                            ctx.warnings,
                        );
                        hw.emit_fmt(format_args!("final {} = {};", field, extract));
                    }
                    for stmt in &body.statements {
                        // Store accesses inside handler bodies resolve to
                        // ref.watch/ref.read on the Riverpod providers (C4);
                        // widget-scope props/events rewrite to widget.<name>.
                        // Task 3.7: the handler runs at EVENT time (outside
                        // the build phase) — reads must be ref.read, not
                        // ref.watch (Riverpod 3.0 throws on watch in
                        // callbacks).
                        let s1 = rewrite_stmt_store_accesses(stmt, ctx.store_provider_map, ctx.ir);
                        let s1 = watch_to_read_stmt(&s1);
                        let s2 = rewrite_stmt_widget_refs(&s1, ctx.widget_names, ctx.fn_renames);
                        // Use custom formatter with event_param_name set
                        // so FieldAccess on the raw var emits bare field name
                        // (local final binding). Phase 3: carries the D1/D2
                        // set — handler mutations write `_name.value`.
                        let fmt = FlutterStmtFormatter {
                            expr: super::r#ref::FlutterFormatExpr::new_with_context(
                                Some(raw_var.to_string()),
                                ctx.direct_write_signals,
                                ctx.load_builtins,
                                ctx.form_present,
                                ctx.param_providers,
                                false,
                            ),
                            direct_write_signals: ctx.direct_write_signals.clone(),
                            local_names: FxHashSet::default(),
                            param_providers: ctx.param_providers.clone(),
                        };
                        fmt.format_stmt(&mut hw, &s2);
                    }
                    if key_event {
                        hw.emit("return KeyEventResult.ignored;");
                    }
                    hw.dedent();
                    hw.emit("}");
                    let result = hw.into_string().trim().to_string();
                    Some(result)
                } else {
                    if is_void
                        && handler
                            .used_fields()
                            .as_ref()
                            .is_some_and(|f| !f.is_empty())
                    {
                        warn_dropped_event_fields(
                            ctx,
                            name,
                            &callback_name,
                            "the handler",
                            el.source_span,
                        );
                    }
                    if key_event {
                        Some(format_key_handler_flutter(body, ctx))
                    } else {
                        Some(format_inline_handler_flutter(body, ctx))
                    }
                }
            }
            EventHandlerIR::StoreAction { r#ref, .. } => {
                let base = format_store_action_handler(r#ref);
                if key_event {
                    // `() { X(); }` → `(node, event) { X(); return
                    // KeyEventResult.ignored; }`
                    Some(format!(
                        "(node, event) {{ {} return KeyEventResult.ignored; }}",
                        base.trim_end_matches('}').trim_start_matches("() {")
                    ))
                } else {
                    Some(base)
                }
            }
        };
        let Some(body) = body else { continue };
        match wrapper {
            Some(EventWrapperKind::GestureDetector(cb)) => {
                // A GestureDetector carries ONE callback per gesture — a
                // second tap-family event on the same element would collide
                // on the wrapper; warn and drop it (never emit duplicate
                // onTap params).
                if wrap_args
                    .iter()
                    .any(|(w, ms)| *w == "GestureDetector" && ms.iter().any(|(p, _)| p == cb))
                {
                    ctx.warn_at(
                        format!(
                            "[flutter] event '{name}' → callback '{callback_name}' collides with the tap wrapper ('{cb}') on '{widget}'; skipped"
                        ),
                        el.source_span,
                    );
                    continue;
                }
                wrap_push(&mut wrap_args, "GestureDetector", cb, &body);
            }
            Some(EventWrapperKind::MouseRegion(param)) => {
                wrap_push(&mut wrap_args, "MouseRegion", param, &body);
            }
            Some(EventWrapperKind::Listener(param)) => {
                // Listener callbacks take an event arg — a bare `() { ... }`
                // closure is `void Function()`, which is NOT assignable to
                // `PointerDownEventListener` (Dart subtyping is contravariant
                // in parameters). Rewrap no-arg closures with `(event)`; a
                // tear-off gets invoked inside the closure.
                wrap_push(
                    &mut wrap_args,
                    "Listener",
                    param,
                    &listener_callback_body(&body, ctx.comp),
                );
            }
            Some(EventWrapperKind::ScrollNotif) => {
                // NotificationListener's onNotification returns bool —
                // convert the void handler into
                // `(notification) { <stmts> return false; }`. Closures have
                // their statements extracted; tear-offs are invoked with the
                // notification when the fn takes an argument
                // (EventsDemo's handleScroll(e)).
                let trimmed = body.trim();
                let inner = if trimmed.starts_with('(') && trimmed.ends_with('}') {
                    closure_body(trimmed).to_string()
                } else if ctx
                    .comp
                    .fns
                    .iter()
                    .any(|f| f.name == trimmed && !f.params.is_empty())
                {
                    format!("{trimmed}(notification);")
                } else {
                    format!("{trimmed}();")
                };
                let notif_body = format!("(notification) {{ {inner} return false; }}");
                wrap_push(
                    &mut wrap_args,
                    "NotificationListener<ScrollNotification>",
                    "onNotification",
                    &notif_body,
                );
            }
            Some(EventWrapperKind::FocusChange(_)) => {
                // handled above — bodies must be statements, not closures
                unreachable!("FocusChange bodies are formatted before the generic path")
            }
            Some(EventWrapperKind::KeyEvent) => {
                if key_wrap_body.is_some() {
                    ctx.warn_at(
                        format!(
                            "[flutter] event '{name}' → callback '{callback_name}' collides with the key wrapper ('onKeyEvent') on '{widget}'; skipped"
                        ),
                        el.source_span,
                    );
                    continue;
                }
                // Review LOW-1: Focus.onKeyEvent fires for BOTH keydown and
                // keyup — inject the event-type guard so the handler runs
                // only for its declared event kind.
                key_wrap_body = Some(match key_event_kind {
                    Some(kind) => inject_key_event_guard(&body, kind),
                    None => body,
                });
            }
            None => args.push(format!("{}: {}", callback_name, body)),
        }
    }
    // Merge onFocus + onBlur into the single Focus(onFocusChange:) closure —
    // the bool `v` is true on focus gain, false on blur. Deterministic
    // focus-first order (FxHashMap event iteration is not).
    if !focus_change_bodies.is_empty() {
        focus_change_bodies.sort_by_key(|(k, _)| if *k == "focus" { 0 } else { 1 });
        let inner = focus_change_bodies
            .iter()
            .map(|(which, stmts)| {
                if *which == "focus" {
                    format!("if (v) {{ {stmts} }}")
                } else {
                    format!("else {{ {stmts} }}")
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        wrap_push(
            &mut wrap_args,
            "Focus",
            "onFocusChange",
            &format!("(v) {{ {inner} }}"),
        );
    }
    if let Some(body) = key_wrap_body {
        wrap_push(&mut wrap_args, "Focus", "onKeyEvent", &body);
    }

    // @drag draggable-intent wiring (ADR-073): drag is not a DOM event —
    // the GestureDetector pan callbacks synthesize the DragIntent payload
    // ({ x, y, dx, dy, pointerId, startX, startY }). onPanStart snapshots
    // the start Offset onto the shared State field `_dragStart`; onPanUpdate
    // computes dx/dy against the snapshot and calls the move handler;
    // onPanEnd calls the optional end handler and clears the snapshot.
    // The pan members MERGE into an existing GestureDetector wrapper
    // (tap family) — one wrapper, never two (ADR-071 decision 3: the pan
    // wiring is a compiler-internal detail, never user-facing syntax).
    if let Some(drag) = &el.drag_binding {
        emit_drag_gesture_members(drag, ctx, &mut wrap_args);
    }

    // Bindings (C6: per-widget whitelist)
    for (bind_name, signal_ref) in &el.bindings {
        let param = binding_to_flutter_param(bind_name);
        if param.is_empty() {
            continue;
        }
        // @form (Task 5): a bound field's validate: rules attach to the
        // TextFormField's validator chain (errors surface via errorText).
        // Runs BEFORE the controller_covers branch so a ref'd field still
        // gets its validator even though value/onChanged move into the
        // text controller listener.
        if widget == "TextFormField" && bind_name == "value" {
            form_validator_args(ctx, signal_ref, &mut args);
        }
        // A ref'd TextField's bind:value is carried by the text controller
        // (needs_text → `controller:` param + guarded listener + sync), not
        // by a `value:` constructor arg — skip silently instead of warning.
        let controller_covers = widget == "TextField"
            && bind_name == "value"
            && el
                .ref_binding
                .as_ref()
                .is_some_and(|r| ctx.ref_usage.get(r).is_some_and(|u| u.needs_text));
        if controller_covers {
            continue;
        }
        // Task 3.3: `bind:value` is a two-way binding — the write-back
        // direction must survive even without a ref'd controller. Widgets
        // with an `onChanged` callback get a setState signal assignment
        // (a pre-existing onChanged — from an `@onChange` event — wins;
        // never duplicate the callback). On widgets with NO `value:` param
        // (TextField) this REPLACES the previously silently-dropped binding
        // (the `onChanged: null` disabled-control form); on widgets that
        // take both (Checkbox/Switch/Slider/DropdownButton) it is emitted
        // alongside `value:` so the control is not stuck disabled. Scoped to
        // `bind:value` only: `bind:checked` on an `<input type="checkbox">`
        // maps to a TextField whose String callback cannot be assigned to a
        // bool signal (invalid_assignment) — that pairing keeps its warning.
        //
        // The write-back targets BARE component signals only, and only with
        // a type the widget's onChanged payload can assign (String on
        // TextField, bool on Checkbox/Switch, double on Slider — mirroring
        // ref.rs::has_text_listeners' "only bare component signals" rule).
        // A store-qualified target (`bind:value={TodoStore.query}`) names a
        // Notifier class / immutable store state with no writable member,
        // and a mismatched signal type would emit an invalid_assignment —
        // both would turn a diagnosable warning into a raw analyzer error.
        // Such shapes keep the original warning fallback (honest
        // degradation, ADR-054).
        let already_has_on_changed = args.iter().any(|a| a.starts_with("onChanged:"));
        let signal = ctx.comp.signals.iter().find(|s| s.name == signal_ref.name);
        let write_back_type_ok = signal.is_some_and(|s| match widget {
            "TextField" => matches!(s.r#type.kind, TypeKind::String),
            "Checkbox" | "Switch" => matches!(s.r#type.kind, TypeKind::Bool),
            "Slider" => {
                matches!(s.r#type.kind, TypeKind::Float | TypeKind::Int)
            }
            _ => true, // DropdownButton's callback is T? — any bare signal type writes back
        });
        let signal_kind = signal.map(|s| s.r#type.kind.clone());
        let write_back_eligible = (bind_name == "value"
            || (bind_name == "checked" && matches!(widget, "Checkbox" | "Switch")))
            && write_back_type_ok
            && widget_allows_param(widget, "onChanged");
        let write_back = write_back_eligible
            .then(|| bind_value_write_back(widget, &signal_ref.name, signal_kind.as_ref()));
        if !widget_allows_param(widget, param) {
            if let Some(cb) = write_back {
                if already_has_on_changed {
                    // Task C2b (#9): the user's @onChange owns the onChanged
                    // slot — MERGE the write-back INTO it (the two-way
                    // binding must survive; the callback cannot duplicate).
                    if let Some(idx) = args.iter().position(|a| a.starts_with("onChanged:")) {
                        let body = args[idx].trim_start_matches("onChanged: ");
                        args[idx] = format!(
                            "onChanged: {}",
                            merge_write_back_into_on_changed(body, &signal_ref.name)
                        );
                    }
                } else {
                    args.push(format!("onChanged: {cb}"));
                }
            } else {
                ctx.warn_at(
                    format!(
                        "[flutter] binding 'bind:{bind_name}' → param '{param}' is not supported on '{widget}'; skipped"
                    ),
                    el.source_span,
                );
            }
            continue;
        }
        if write_back_eligible && !already_has_on_changed {
            if let Some(cb) = write_back {
                args.push(format!("onChanged: {cb}"));
            }
        }
        // Slider's value is double — an `i32` signal converts at the read
        // site (`value: (rangeVal).toDouble()`), mirroring the write-back's
        // `.toInt()` (FormControlsDemo rangeVal: i32).
        let value_arg = if widget == "Slider"
            && signal.is_some_and(|s| matches!(s.r#type.kind, TypeKind::Int))
        {
            format!("({}).toDouble()", signal_ref.name)
        } else {
            signal_ref.name.clone()
        };
        args.push(format!("{}: {}", param, value_arg));
    }

    // @form (Task 5): the type="submit" button intercepts submission —
    // `Form.validate()` gates the handler (errors already surfaced per-field
    // via errorText). Runs BEFORE required_defaults so the `onPressed: null`
    // default cannot steal the slot. An explicit onPressed (event or prop)
    // always wins over the interception. Any of the three button widgets can
    // carry type="submit" (button→ElevatedButton, a→TextButton, icon
    // button→IconButton); a form with NO submit button loses the validate
    // gate by design (v0 boundary — the handler fires ungated).
    if matches!(widget, "ElevatedButton" | "TextButton" | "IconButton") && ctx.form_present {
        let is_submit = match el.props.get("type") {
            Some(ExprIR::LiteralExpr { value }) => value.as_str() == Some("submit"),
            _ => false,
        };
        if is_submit {
            if let Some(handler) =
                ctx.comp.form_intent.as_ref().and_then(|f| {
                    (!f.submit_handler.is_empty()).then_some(f.submit_handler.as_str())
                })
            {
                let already = args.iter().any(|a| a.starts_with("onPressed:"));
                if !already {
                    let renamed = ctx
                        .fn_renames
                        .get(handler)
                        .cloned()
                        .unwrap_or_else(|| handler.to_string());
                    args.push(format!(
                        "onPressed: () {{ if (!_formKey.currentState!.validate()) return; {renamed}(); }}"
                    ));
                }
            }
        }
    }

    // Required-parameter defaults: Flutter widgets with REQUIRED named
    // params emit a default when the .omni source provides none —
    // `ElevatedButton(child: ...)` without `onPressed:` is invalid Dart
    // (onPressed is required). `onPressed: null`/`onChanged: null` render
    // the control disabled; `value: false` / `value: 0.0` render
    // toggles/sliders off. Runs AFTER props/events/bindings so a bound
    // `value:` (bind:value) or emitted callback is not duplicated.
    let required_defaults: &[(&str, &str)] = match widget {
        "ElevatedButton" | "TextButton" | "IconButton" => &[("onPressed", "null")],
        "Checkbox" | "Switch" => &[("value", "false"), ("onChanged", "null")],
        "Slider" => &[("value", "0.0"), ("onChanged", "null")],
        _ => &[],
    };
    for (param, default) in required_defaults {
        let present = args.iter().any(|a| a.starts_with(&format!("{}:", param)));
        if !present {
            args.push(format!("{}: {}", param, default));
        }
    }

    // @form (Task 5): the Form element carries the validation GlobalKey
    // (the submit interception's validate() gate runs on it). If the author
    // also pinned a `key:` prop, the slot is occupied — the GlobalKey is
    // dropped with an honest warning (the validation gate breaks).
    if widget == "Form" && ctx.form_present {
        if !args.iter().any(|a| a.starts_with("key:")) {
            args.push("key: _formKey".to_string());
        } else {
            ctx.warn_at(
                format!(
                    "[flutter] @form on '{widget}': the element's key slot is occupied by a key prop — the Form's GlobalKey is dropped (submit validation will not work)"
                ),
                el.source_span,
            );
        }
    }

    // Ref binding (Step 6): attach the GlobalKey plus any on-demand
    // controllers the component uses on this ref. Controllers attach only to
    // widgets that support them (TextField/scrollables); focus on other
    // widgets falls back to a Focus wrapper below.
    let mut focus_wrap_ref: Option<&str> = None;
    if let Some(ref_name) = &el.ref_binding {
        let usage = ctx.ref_usage.get(ref_name).copied().unwrap_or_default();
        // Defense in depth (review MEDIUM-2): a `key` prop emitted its arg
        // in the props loop — a second `key:` would not compile. The
        // GlobalKey is what the ref machinery needs, so its loss is an
        // honest warning, never a silent skip.
        if !args.iter().any(|a| a.starts_with("key:")) {
            args.push(format!("key: {}", ref_key_name(ref_name)));
        } else {
            ctx.warn_at(
                format!(
                    "[flutter] ref binding '{ref_name}' on '{widget}': the element's \
                     key slot is occupied by a key prop — the ref GlobalKey is dropped \
                     (ref lookups by key will not work)"
                ),
                el.source_span,
            );
        }
        if usage.needs_text && widget == "TextField" {
            args.push(format!("controller: {}", ref_text_name(ref_name)));
        }
        if usage.needs_focus && widget == "TextField" {
            args.push(format!("focusNode: {}", ref_focus_name(ref_name)));
        }
        if usage.needs_scroll && matches!(widget, "ListView" | "SingleChildScrollView") {
            args.push(format!("controller: {}", ref_scroll_name(ref_name)));
        }
        if usage.needs_focus && widget != "TextField" {
            focus_wrap_ref = Some(ref_name);
        }
    }

    // Children
    // Task C2b (#5): a Text element with non-text children no longer
    // degrades to a Column — mixed content composes via
    // `Text.rich(TextSpan(children: [...]))`: text/interpolation nodes
    // become TextSpan(text:) entries and child widgets embed as
    // WidgetSpan(child:) — the inline-flow semantics the DOM gives
    // `<p>text <strong>bold</strong></p>`.
    let mut rich_text = false;
    if el.children.is_empty() {
        // No children
    } else if widget == "Text" {
        // Text widgets take their content as the positional `data`
        // argument — `Text(child: ...)` is invalid Dart (Text has no
        // `child` parameter). Text-ish children (plain text nodes /
        // interpolations) concatenate into the data string; mixed content
        // emits Text.rich with the composed TextSpan.
        let mut parts: Vec<String> = Vec::new();
        let mut all_text = true;
        for child in &el.children {
            match child {
                ViewTree::Text(t) => {
                    // v4 rust HIGH: 与 TextSpan 路径同修复——共享转义模式。
                    let escaped = escape_literal_str(t, '\'');
                    parts.push(escaped.replace('$', "\\$"));
                }
                ViewTree::Interpolation(e) => {
                    parts.push(format!("${{{}}}", ctx.format_expr(e)));
                }
                _ => {
                    all_text = false;
                    break;
                }
            }
        }
        if all_text {
            args.insert(0, format!("'{}'", parts.join("")));
        } else {
            rich_text = true;
            args.insert(0, format_text_span_children(&el.children, motions, ctx));
        }
    } else if el.children.len() == 1 {
        let child_str = format_view_tree(&el.children[0], motions, ctx);
        // F2: a single child that renders nothing (e.g. a lone `@load`
        // directive → `Fragment(vec![])` → "") must not emit
        // `child: ` — the element renders as an empty widget instead.
        if !child_str.is_empty() {
            args.push(format!("child: {}", child_str));
        }
    } else {
        let children: Vec<String> = el
            .children
            .iter()
            .map(|c| format_view_tree(c, motions, ctx))
            // F2: empty renderings (the `@load` directive lowering to
            // `Fragment(vec![])`, or any directive expansion that produces
            // nothing) must not dangle as `, ` in the children list (dart
            // analyze missing_identifier; LoadDemo.omni div.Card).
            .filter(|s| !s.is_empty())
            .collect();
        let children_arg = format!("children: [{}]", children.join(", "));
        if widget_allows_param(widget, "children") {
            // Multi-child widgets (Row, Column, Stack, ListView, ...) take
            // `children:` directly.
            args.push(children_arg);
        } else {
            // Single-child widgets (Container, Center, ...) host multiple
            // children via an inner Column — `Container(children: [...])`
            // would be invalid Dart.
            args.push(format!("child: Column({})", children_arg));
        }
    }

    // Task 3.10: a relative src swaps the emitted constructor to
    // Image.asset (whitelist/event/alt checks keep the Image.network param
    // set — Image.asset shares every one of them). Task C2b (#5): mixed
    // content emits `Text.rich` with the composed TextSpan.
    let render_widget = if use_asset_image {
        "Image.asset"
    } else if rich_text {
        "Text.rich"
    } else {
        widget
    };
    // Text requires the positional `data` argument — an element with NO
    // text children but other args (`<span style: {...} />` → args carry
    // the TextStyle) must still pass `''`: `Text(style: TextStyle(...))`
    // is invalid Dart (1 positional expected). The args-less case is
    // handled by the `Text('')` fallback below.
    if render_widget == "Text" && el.children.is_empty() && !args.is_empty() {
        args.insert(0, "''".to_string());
    }
    let inner = if args.is_empty() {
        // Task 5.8: an empty element mapped to Text (`<span/>` with style
        // only) emitted `Text()` — Text requires its positional `data`
        // argument (1 positional expected, 0 found).
        if render_widget == "Text" {
            "Text('')".to_string()
        } else {
            format!("{}()", render_widget)
        }
    } else {
        format!("{}({})", render_widget, args.join(", "))
    };

    // Ref-bound elements that need focus but map to a non-TextField widget:
    // wrap in `Focus(focusNode: _nameFocus, ...)` so the FocusNode is
    // attached and `requestFocus()`/`unfocus()` work (Task 5.2).
    let inner = match focus_wrap_ref {
        Some(ref_name) => {
            format!(
                "Focus(focusNode: {}, child: {})",
                ref_focus_name(ref_name),
                inner
            )
        }
        None => inner,
    };

    // ── Conservative const injection (T5.7) ──
    // Literal-only subtrees (no events/bindings/refs/design/a11y/motion,
    // all props literal scalars, all children const-able) get a `const`
    // constructor so they are not re-allocated on every build. The const
    // context propagates to the children automatically.
    let inner = if element_is_const(el, ctx) {
        format!("const {}", inner)
    } else {
        inner
    };

    // ── Design classes/variants (Task 4.3) ──
    // `class: Design.Card`, `variant: Design.Button(...)`, and
    // `class: { active: cond }` resolve against the design IR to REAL
    // BoxDecoration values (the AppDesign.<class>() factory for single
    // classes; field-merged inline decorations for composed sets) — the
    // review's HIGH (empty `BoxDecoration()` stub, invisible decorations) is
    // gone. Unknown/layout-only classes and unmappable values warn.
    let inner = wrap_design_decorations(el, inner, ctx);

    // ── a11y block (design doc 域 10) ──
    // label → Semantics(label:), role → Semantics flags subset (unmappable
    // roles warn + degrade with a generated comment), live → liveRegion,
    // hidden → ExcludeSemantics(excluding:). alt on image widgets is
    // carried by the semanticLabel constructor param (above); on other
    // widgets it degrades to the accessible-name label. The review's
    // MEDIUM-2 (a11y intent silently dropped on Flutter) is gone — the
    // semantic node is always emitted.
    let inner = wrap_a11y(el, inner, ctx);

    // Wrap with motion animation widgets if the element has triggers
    // Task 3.4: read the TRIGGER SET for @in-view — the legacy
    // in_view_once/in_view_repeat fields are filled by lowering as a mirror
    // (defense in depth), but the trigger set is the source of truth.
    let has_inview = el.motion_behavior().is_some_and(|m| {
        m.triggers
            .iter()
            .any(|t| matches!(t.kind, TriggerKind::InView))
    });
    let has_scroll = el.motion_behavior().is_some_and(|m| {
        m.triggers
            .iter()
            .any(|t| matches!(&t.kind, TriggerKind::Scroll { .. }))
    });
    let result = if let Some(motion_idx) = motion_idx {
        let mut result = inner;

        // Standard motion triggers (enter, hover, press, etc.) — all
        // controller references carry the element's ordinal (T5.7)
        if el.motion_behavior().is_some_and(|m| !m.triggers.is_empty()) {
            result = wrap_with_motion_widgets(
                result,
                el.motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers),
                motions,
                motion_idx,
            );
        }

        // Handle @in-view trigger: wrap with OmniInViewWidget
        if has_inview {
            let threshold = match el
                .motion_behavior()
                .and_then(|m| m.in_view_threshold.as_ref())
            {
                Some(expr) => ctx.format_expr(expr),
                None => "0.0".to_string(),
            };
            let once = if el
                .motion_behavior()
                .and_then(|m| m.in_view_once.as_ref())
                .is_some()
            {
                "true"
            } else {
                "false"
            };
            let motion_ref = el
                .motion_behavior()
                .and_then(|m| {
                    m.triggers
                        .iter()
                        .find(|t| matches!(t.kind, TriggerKind::InView))
                })
                .and_then(|t| t.motion_ref.as_ref());

            if let Some(motion_name) = motion_ref {
                let ctrl_name = motion_ctrl_name(motion_name, motion_idx);
                let active_name = motion_active_name(motion_name, motion_idx);
                // Constructor args are inViewOnce/inViewThreshold (not
                // once/threshold) — the runtime widget's field names; the
                // named shape keeps the emitted config self-documenting.
                result = format!(
                    "OmniInViewWidget(\n  inViewOnce: {},\n  inViewThreshold: {},\n  onInView: () {{ {} = true; {}.forward(); }},\n  child: {},\n)",
                    once, threshold, active_name, ctrl_name, result
                );
            }
        }

        // Handle @scroll trigger: wrap with OmniScrollDrivenWidget
        if has_scroll {
            let motion_ref = el
                .motion_behavior()
                .and_then(|m| {
                    m.triggers
                        .iter()
                        .find(|t| matches!(&t.kind, TriggerKind::Scroll { .. }))
                })
                .and_then(|t| t.motion_ref.as_ref());
            // v1 control dimensions (Task 5.1): numeric start/end are passed
            // as typed constructor args; without them the widget falls back
            // to the viewport-relative string configs (defaults "top 80%" /
            // "bottom 20%", parsed at runtime).
            let (start_dart, end_dart) = match el
                .motion_behavior()
                .and_then(|m| {
                    m.triggers
                        .iter()
                        .find(|t| matches!(&t.kind, TriggerKind::Scroll { .. }))
                })
                .map(|t| match &t.kind {
                    TriggerKind::Scroll { start, end, .. } => (start.as_ref(), end.as_ref()),
                    _ => (None, None),
                }) {
                Some((Some(s), Some(e))) => (ctx.format_expr(s), ctx.format_expr(e)),
                _ => (String::new(), String::new()),
            };
            let scroll_range_args = if start_dart.is_empty() && end_dart.is_empty() {
                String::new()
            } else {
                format!("  start: {},\n  end: {},\n", start_dart, end_dart)
            };
            let scrub = match el
                .motion_behavior()
                .and_then(|m| {
                    m.triggers
                        .iter()
                        .find(|t| matches!(&t.kind, TriggerKind::Scroll { .. }))
                })
                .and_then(|t| match &t.kind {
                    TriggerKind::Scroll { scrub, .. } => scrub.as_ref(),
                    _ => None,
                }) {
                // `scrub: true` (v1 BNF) coerces to the numeric smoothing
                // factor 1.0 — the widget field is a `double`, so a raw bool
                // literal would not compile.
                Some(ExprIR::LiteralExpr { value }) if value.is_boolean() => {
                    if value.as_bool() == Some(true) {
                        "1.0".to_string()
                    } else {
                        "0.0".to_string()
                    }
                }
                Some(expr) => ctx.format_expr(expr),
                None => "0.0".to_string(),
            };

            if let Some(motion_name) = motion_ref {
                let ctrl_name = motion_ctrl_name(motion_name, motion_idx);
                let active_name = motion_active_name(motion_name, motion_idx);
                result = format!(
                    "OmniScrollDrivenWidget(\n{}  scrub: {},\n  onProgress: (p) {{ {} = true; {}.value = p; }},\n  child: {},\n)",
                    scroll_range_args, scrub, active_name, ctrl_name, result
                );
            }
        }

        // Handle @drag-horizontal / @drag-vertical trigger: wrap with OmniDragWidget
        let has_drag = el.motion_behavior().is_some_and(|m| {
            m.triggers.iter().any(|t| {
                matches!(
                    &t.kind,
                    TriggerKind::DragHorizontal { .. } | TriggerKind::DragVertical { .. }
                )
            })
        });
        if has_drag {
            for trigger in el
                .motion_behavior()
                .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
            {
                match &trigger.kind {
                    TriggerKind::DragHorizontal { .. } | TriggerKind::DragVertical { .. } => {
                        if let Some(motion_name) = trigger.motion_ref.as_ref() {
                            let is_horizontal =
                                matches!(&trigger.kind, TriggerKind::DragHorizontal { .. });
                            let axis_type = if is_horizontal {
                                "Axis.horizontal"
                            } else {
                                "Axis.vertical"
                            };
                            let ctrl_name = motion_ctrl_name(motion_name, motion_idx);
                            let active_name = motion_active_name(motion_name, motion_idx);
                            // v1 control dimensions (Task 5.1): numeric
                            // constraints pass straight into the widget as
                            // typed constructor args; when none are given,
                            // drag the extent from the motion's own keyframe
                            // range in the drag axis so the element follows
                            // its animation (deferral #19). 0.0 = no usable
                            // range → unbounded (inert, previous behavior).
                            let (constraints_min_dart, constraints_max_dart) = match &trigger.kind {
                                TriggerKind::DragHorizontal { constraints, .. }
                                | TriggerKind::DragVertical { constraints, .. } => {
                                    match constraints {
                                        Some((min, max)) => {
                                            (ctx.format_expr(min), ctx.format_expr(max))
                                        }
                                        None => (String::new(), String::new()),
                                    }
                                }
                                _ => (String::new(), String::new()),
                            };
                            let has_constraints = !constraints_min_dart.is_empty()
                                && !constraints_max_dart.is_empty();
                            let constraints_args = if has_constraints {
                                format!(
                                    "  constraintsMin: {},\n  constraintsMax: {},\n",
                                    constraints_min_dart, constraints_max_dart
                                )
                            } else {
                                String::new()
                            };
                            let (extent_dart, extent_warning) = if !has_constraints {
                                match motions.iter().find(|m| m.name == *motion_name).and_then(
                                    |m| {
                                        crate::codegen::shared::drag::drag_extent(
                                            &m.body,
                                            is_horizontal,
                                        )
                                    },
                                ) {
                                    Some(e) => (
                                        crate::codegen::shared::drag::format_extent_dart(e),
                                        String::new(),
                                    ),
                                    None => {
                                        ctx.warn_at(
                                            format!(
                                                "@drag-{} without constraints and motion '{}' has no literal translateX/Y keyframe values — drag is unbounded (no movement); add a literal translate keyframe range to the motion",
                                                if is_horizontal { "horizontal" } else { "vertical" },
                                                motion_name
                                            ),
                                            el.source_span,
                                        );
                                        (
                                            "0.0".to_string(),
                                            format!(
                                                "// WARNING: @drag-{} without constraints and no literal translateX/Y keyframe values in motion '{}' — drag is unbounded (no movement)\n",
                                                if is_horizontal { "horizontal" } else { "vertical" },
                                                motion_name
                                            ),
                                        )
                                    }
                                }
                            } else {
                                ("0.0".to_string(), String::new())
                            };
                            let (elastic_val, threshold_val) = match &trigger.kind {
                                TriggerKind::DragHorizontal {
                                    elastic, threshold, ..
                                }
                                | TriggerKind::DragVertical {
                                    elastic, threshold, ..
                                } => (
                                    elastic
                                        .as_ref()
                                        .map(|e| ctx.format_expr(e))
                                        .unwrap_or_else(|| "0.0".to_string()),
                                    threshold
                                        .as_ref()
                                        .map(|e| ctx.format_expr(e))
                                        .unwrap_or_else(|| "0.0".to_string()),
                                ),
                                _ => ("0.0".to_string(), "0.0".to_string()),
                            };
                            result = format!(
                                "{extent_warning}OmniDragWidget(\n  axis: {},\n{}  elastic: {},\n  threshold: {},\n  dragExtent: {},\n  onDragStart: () {{ {} = true; }},\n  onDragProgress: (p) {{ {} = true; {}.value = p; }},\n  onDragEnd: (completed) {{ if (completed) {{ {}.forward(); }} else {{ {}.reverse().whenComplete(() {{ if (mounted && {}.value == 0.0) setState(() {{ {} = false; }}); }}); }} }},\n  child: {},\n)",
                                axis_type, constraints_args, elastic_val, threshold_val, extent_dart, active_name, active_name, ctrl_name, ctrl_name, ctrl_name, ctrl_name, active_name, result,
                                extent_warning = extent_warning
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        result
    } else {
        inner
    };

    // Task 3.6 / C2b (#3): wrapper callbacks (GestureDetector tap family,
    // MouseRegion enter/exit/hover, Focus focus/key) wrap the FULLY
    // rendered element (design decoration / a11y / motion wrappers
    // included) so the interaction target covers the whole visual element.
    // Nesting order is deterministic — FxHashMap event iteration is not.
    let result = if wrap_args.is_empty() {
        result
    } else {
        const WRAP_ORDER: [&str; 6] = [
            "Tooltip",
            "Focus",
            "MouseRegion",
            "Listener",
            "NotificationListener<ScrollNotification>",
            "GestureDetector",
        ];
        let mut sorted: Vec<_> = wrap_args.into_iter().collect();
        sorted.sort_by_key(|(w, _)| {
            WRAP_ORDER
                .iter()
                .position(|o| o == w)
                .unwrap_or(WRAP_ORDER.len())
        });
        let mut wrapped = result;
        for (wrapper, mut members) in sorted {
            members.sort(); // deterministic param order within a wrapper
            let member_args = members
                .iter()
                .map(|(p, b)| format!("{p}: {b}"))
                .collect::<Vec<_>>()
                .join(", ");
            wrapped = format!("{wrapper}({member_args}, child: {wrapped})");
        }
        wrapped
    };

    // Task 14 (ADR-071 direct-write channel): channel wraps applied OUTSIDE
    // the event-wrapper stack so the ListenableBuilder rebuild scope covers
    // the whole rendered subtree:
    //   Transform.translate(offset: Offset(x, y), child: <subtree>) for the
    //   translate shape, then
    //   ListenableBuilder(listenable: Listenable.merge([_dragX, _dragY]),
    //   builder: (context, _) => <subtree>) reacting to the entry's D1/D2
    //   ValueNotifier writes (Task 13 storage) — no setState, no whole-tree
    //   build on every pan delta.
    let result = if channel_wrap.offset.is_some() || !channel_wrap.listenables.is_empty() {
        let mut wrapped = result;
        if let Some(offset) = channel_wrap.offset {
            wrapped = format!(
                "Transform.translate(offset: {}, child: {})",
                offset, wrapped
            );
        }
        if !channel_wrap.listenables.is_empty() {
            wrapped = format!(
                "ListenableBuilder(listenable: Listenable.merge([{}]), builder: (context, _) => {})",
                channel_wrap.listenables.join(", "),
                wrapped
            );
        }
        wrapped
    } else {
        result
    };

    result
}

/// @form (Task 5): `<input>` inside an @form component maps to
/// `TextFormField` so bound fields get `validator` — Form validation only
/// works on FormField descendants.
fn form_field_widget<'a>(ctx: &ViewContext<'_>, el: &ElementIR, base: &'a str) -> &'a str {
    if el.tag == "input" {
        // `type` selects the control widget — `<input type="checkbox">`
        // previously mapped to TextField (wrong control, bind:checked
        // dropped); range maps to Slider.
        if let Some(ExprIR::LiteralExpr { value }) = el.props.get("type") {
            match value.as_str() {
                Some("checkbox") => return "Checkbox",
                Some("range") => return "Slider",
                Some("radio") => return "Radio",
                _ => {}
            }
        }
        if ctx.form_present {
            return "TextFormField";
        }
    }
    base
}

/// Extract an integer literal from a validate: rule value (`min: 2`).
fn rule_literal_int(rule: &ValidateRuleIR) -> Option<i64> {
    match &rule.value {
        ExprIR::LiteralExpr { value } => value.as_i64(),
        _ => None,
    }
}

/// Extract a string literal from a validate: rule value (`format: "email"`).
fn rule_literal_str(rule: &ValidateRuleIR) -> Option<String> {
    match &rule.value {
        ExprIR::LiteralExpr { value } => value.as_str().map(str::to_string),
        _ => None,
    }
}

/// Build the custom-validator tail for a `custom:` rule — the rule VALUE
/// names the component fn (`custom: noLeadingSpace`), renamed via fn_renames
/// when the wrapper renames it. Returns None for non-identifier values
/// (dynamic rules cannot be proven at compile time; skipped, mirroring the
/// React backend).
fn custom_rule_tail(ctx: &ViewContext<'_>, rule: &ValidateRuleIR) -> Option<String> {
    let ExprIR::IdentifierExpr { name, .. } = &rule.value else {
        return None;
    };
    let renamed = ctx
        .fn_renames
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.to_string());
    Some(format!("{renamed}(v)"))
}

/// @form (Task 5): push `validator:` — the rule chain
/// `(v) => cond1 ? 'msg1' : cond2 ? 'msg2' : TAIL` where TAIL is the custom
/// component fn (when present) else `null` — plus `autovalidateMode:` onto a
/// bound TextFormField's constructor args. Errors surface via errorText; the
/// Form's validate() gate drives submit.
fn form_validator_args(ctx: &ViewContext<'_>, signal_ref: &SignalRefIR, args: &mut Vec<String>) {
    let Some(field) = ctx
        .comp
        .form_intent
        .as_ref()
        .and_then(|fi| fi.fields.iter().find(|f| f.name == signal_ref.name))
    else {
        return;
    };
    if field.rules.is_empty() {
        return;
    }
    let mut checks: Vec<(String, String)> = Vec::new(); // (condition, message)
    let mut custom_tail: Option<String> = None;
    // Dart flow analysis promotes `v` to non-null in every ternary else-branch
    // after `v == null || v.isEmpty` has been evaluated false — a later
    // `v != null &&` is therefore always true (unnecessary_null_comparison
    // warning). Track whether a null guard already appeared earlier in the
    // chain and drop the redundant prefix for subsequent rules.
    let mut null_guarded = false;
    for rule in &field.rules {
        match rule.name.as_str() {
            "required" => {
                checks.push(("v == null || v.isEmpty".to_string(), "此项必填".to_string()));
                null_guarded = true;
            }
            "min" => {
                if let Some(n) = rule_literal_int(rule) {
                    let guard = if null_guarded { "" } else { "v != null && " };
                    checks.push((
                        format!("{guard}v.length < {n}"),
                        format!("此项至少 {n} 个字符"),
                    ));
                }
            }
            "max" => {
                if let Some(n) = rule_literal_int(rule) {
                    let guard = if null_guarded { "" } else { "v != null && " };
                    checks.push((
                        format!("{guard}v.length > {n}"),
                        format!("此项最多 {n} 个字符"),
                    ));
                }
            }
            // `format` is a CLOSED name set — dispatch on the name (mirrors
            // the React backend): "email"/"url" map to concrete regexes; any
            // other value is a dynamic expression or unknown name, skipped
            // (a non-literal like `format: fmt` cannot be proven at compile
            // time; the rule must be implemented as a custom validator).
            "format" => {
                if let Some(p) = rule_literal_str(rule) {
                    let guard = if null_guarded { "" } else { "v != null && " };
                    match p.as_str() {
                        "email" => checks.push((
                            format!("{guard}!RegExp(r'^[^\\s@]+@[^\\s@]+\\.[^\\s@]+').hasMatch(v)"),
                            "邮箱格式不正确".to_string(),
                        )),
                        "url" => checks.push((
                            format!("{guard}!RegExp(r'^(https?:\\/\\/)?([\\w-]+\\.)+[\\w-]{{2,}}(\\/\\S*)?').hasMatch(v)"),
                            "URL 格式不正确".to_string(),
                        )),
                        _ => {}
                    }
                }
            }
            // `pattern` takes the RAW pattern string — the user's regex is
            // the rule (unlike `format`, where the name picks a built-in).
            "pattern" => {
                if let Some(p) = rule_literal_str(rule) {
                    let escaped = p.replace('\'', "\\'");
                    let guard = if null_guarded { "" } else { "v != null && " };
                    checks.push((
                        format!("{guard}!RegExp(r'{escaped}').hasMatch(v)"),
                        "格式不正确".to_string(),
                    ));
                }
            }
            // `custom` is the ONLY keyword rule treated as a custom
            // component fn — the fn lives in the rule VALUE
            // (`custom: noLeadingSpace`), mirroring the React backend. It
            // becomes the chain's tail (returns null when valid).
            "custom" => {
                custom_tail = custom_rule_tail(ctx, rule);
            }
            // Any other rule name must have been rejected by the semantic
            // pass (V-F4 whitelist) — emit nothing, mirroring the React
            // backend's `_ =>` comment. Never treat an unknown rule name
            // as a component fn call (R6: the previous catch-all silently
            // emitted `other(v)` Dart for e.g. a typo'd rule name).
            _ => {}
        }
    }
    // (v) => c1 ? 'm1' : c2 ? 'm2' : TAIL — TAIL is the custom fn when
    // present, else `null` (a bare "required" chain ends in : null).
    let mut validator = String::from("(v) => ");
    for (i, (cond, msg)) in checks.iter().enumerate() {
        if i > 0 {
            validator.push_str(" : ");
        }
        validator.push_str(&format!("{cond} ? '{msg}'"));
    }
    if let Some(custom) = custom_tail {
        if !checks.is_empty() {
            validator.push_str(" : ");
        }
        validator.push_str(&custom);
    } else if !checks.is_empty() {
        validator.push_str(" : null");
    }
    args.push(format!("validator: {validator}"));
    args.push("autovalidateMode: AutovalidateMode.onUserInteraction".to_string());
}

/// Extract the statement block of a formatted handler closure (`() { X }`,
/// `(event) { X }`) — returns the inner statements. Non-closures (tear-offs)
/// are returned unchanged.
fn closure_body(body: &str) -> &str {
    let trimmed = body.trim();
    match trimmed.find('{') {
        Some(idx) if trimmed.ends_with('}') => trimmed[idx + 1..]
            .trim_end()
            .strip_suffix('}')
            .unwrap_or(trimmed),
        _ => trimmed,
    }
}

/// Rewrap a handler body for a Listener callback (pointer events): the
/// callback takes an event argument, so a no-arg closure `() { X }` becomes
/// `(event) { X }` (`void Function()` is not assignable to
/// `PointerDownEventListener` — contravariant parameters), an already-arged
/// closure is kept, and a bare tear-off is invoked with the event when the
/// component fn takes a parameter (`handleWheel(e)` → `handleWheel(event)`)
/// or with no args otherwise.
fn listener_callback_body(body: &str, comp: &ComponentIR) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('(') && trimmed.ends_with('}') {
        if trimmed.starts_with("() {") {
            let inner = closure_body(trimmed);
            format!("(event) {{ {} }}", inner)
        } else {
            trimmed.to_string()
        }
    } else if comp
        .fns
        .iter()
        .any(|f| f.name == trimmed && !f.params.is_empty())
    {
        format!("(event) {{ {trimmed}(event); }}")
    } else {
        format!("(event) {{ {trimmed}(); }}")
    }
}

/// Append `(param, body)` to a wrapper's member list, creating the wrapper
/// entry on first use (the wrapper is applied around the fully-rendered
/// element by the caller).
fn wrap_push(
    wrap_args: &mut Vec<(&'static str, Vec<(String, String)>)>,
    wrapper: &'static str,
    param: &str,
    body: &str,
) {
    match wrap_args.iter_mut().find(|(w, _)| *w == wrapper) {
        Some((_, members)) => members.push((param.to_string(), body.to_string())),
        None => wrap_args.push((wrapper, vec![(param.to_string(), body.to_string())])),
    }
}

/// Emit the GestureDetector pan callbacks implementing `@drag` (ADR-073).
///
/// Flutter's gesture arena has no DOM-style pointer capture — the pan
/// recognizer IS the capture: onPanStart wins the arena for the element,
/// onPanUpdate streams drag deltas, onPanEnd fires on release. The
/// DragIntent payload is synthesized against the start Offset snapshot in
/// the shared State field `_dragStart`:
///
/// ```dart
/// onPanStart: (d) { _dragStart = d.localPosition; },
/// onPanUpdate: (d) { final s = _dragStart; if (s == null) return;
///                    move(_DragEvent(x: d.localPosition.dx,
///                                    dx: d.localPosition.dx - s.dx, ...)); },
/// onPanEnd: (d) { final s = _dragStart; if (s != null) { end(_DragEvent(...)); }
///                  _dragStart = null; },
/// ```
///
/// Handlers are FnRef by contract (validate_drag enforces bare fn names);
/// a zero-param handler is called BARE (a `{...}` argument would be
/// extra_positional_arguments on a 0-arity Dart method).
fn emit_drag_gesture_members(
    drag: &DragBindingIR,
    ctx: &mut ViewContext,
    wrap_args: &mut Vec<(&'static str, Vec<(String, String)>)>,
) {
    // onPanStart: snapshot the drag start onto the shared State field.
    wrap_push(
        wrap_args,
        "GestureDetector",
        "onPanStart",
        "(d) { _dragStart = d.localPosition; }",
    );

    let EventHandlerIR::FnRef {
        r#ref: move_fn,
        used_fields,
        ..
    } = &drag.on_drag
    else {
        // Unreachable: lowering always produces FnRef for drag handlers
        // (validate_drag enforces bare function names).
        ctx.warn_at("[flutter] @drag 处理器必须是函数名（FnRef），已跳过", None);
        return;
    };
    let renamed = ctx
        .fn_renames
        .get(move_fn)
        .cloned()
        .unwrap_or_else(|| move_fn.clone());
    let move_obj = drag_event_object_flutter(used_fields, "s", "d");
    let move_call = drag_handler_call(&renamed, move_fn, &move_obj, ctx);
    // onPanUpdate: drag-guarded move-handler call — `s` is a local so the
    // null check promotes it under Dart's flow analysis (an instance field
    // would not).
    wrap_push(
        wrap_args,
        "GestureDetector",
        "onPanUpdate",
        &format!("(d) {{ final s = _dragStart; if (s == null) return; {move_call} }}"),
    );

    // onPanEnd: optional end handler (same DragIntent shape read from the
    // snapshot `s`, x/y from the release position `d.localPosition`) + clear.
    let end_body = match &drag.on_drag_end {
        Some(EventHandlerIR::FnRef {
            r#ref: end_fn,
            used_fields: end_fields,
            ..
        }) => {
            let end_renamed = ctx
                .fn_renames
                .get(end_fn)
                .cloned()
                .unwrap_or_else(|| end_fn.clone());
            let end_obj = drag_event_object_flutter(end_fields, "s", "d");
            let end_call = drag_handler_call(&end_renamed, end_fn, &end_obj, ctx);
            format!(
                "(d) {{ final s = _dragStart; if (s != null) {{ {end_call} }} _dragStart = null; }}"
            )
        }
        Some(_) => {
            // Unreachable (validate_drag enforces FnRef); honest warning.
            ctx.warn_at(
                "[flutter] @dragEnd 处理器必须是函数名（FnRef），已跳过",
                None,
            );
            "(d) { _dragStart = null; }".to_string()
        }
        None => "(d) { _dragStart = null; }".to_string(),
    };
    wrap_push(wrap_args, "GestureDetector", "onPanEnd", &end_body);
}

/// Build the DragIntent carrier for a drag handler call.
///
/// DragIntent fields are synthesized (start snapshot + pointer delta), not
/// raw event passthrough — the shared `emit_event_object_flutter` cannot
/// express them (its schema has no snapshot-dependent dx/dy/startX/startY),
/// so the payload is built here as a `_DragEvent(...)` constructor call,
/// honoring `used_fields` with the same filter-in-schema-order semantics.
///
/// The TYPED carrier (not a `Map<String,num>` literal) is load-bearing:
/// handler bodies read `e.dx`/`e.startX` as dot-accesses, and Dart maps
/// have no getters — a Map payload would NoSuchMethodError on the first
/// pan (Fix round 1 of the ADR-073 review). The handler param is untyped
/// (`dynamic`), so the dot-accesses resolve via dynamic dispatch on the
/// instance. All-optional named constructor params keep `_DragEvent()`
/// valid for handlers that read no fields. `state` names the drag-start
/// snapshot local (`s`), `raw` the pan details param (`d`). pointerId has
/// no Flutter analogue — it degrades to 0 (single-touch default), per the
/// ADR-073 cross-target schema contract.
fn drag_event_object_flutter(used_fields: &Option<Vec<String>>, state: &str, raw: &str) -> String {
    let all: Vec<(String, String)> = vec![
        ("x".into(), format!("{raw}.localPosition.dx")),
        ("y".into(), format!("{raw}.localPosition.dy")),
        ("dx".into(), format!("{raw}.localPosition.dx - {state}.dx")),
        ("dy".into(), format!("{raw}.localPosition.dy - {state}.dy")),
        ("pointerId".into(), "0".into()),
        ("startX".into(), format!("{state}.dx")),
        ("startY".into(), format!("{state}.dy")),
    ];
    let fields: Vec<String> = match used_fields {
        Some(used) if !used.is_empty() => all
            .into_iter()
            .filter(|(name, _)| used.contains(name))
            .map(|(name, extract)| format!("{name}: {extract}"))
            .collect(),
        // Empty used-fields: `_DragEvent()` — the all-optional constructor
        // keeps the call valid for handlers that read no DragIntent fields.
        Some(_) => Vec::new(),
        None => all
            .into_iter()
            .map(|(name, extract)| format!("{name}: {extract}"))
            .collect(),
    };
    format!("_DragEvent({})", fields.join(", "))
}

/// Handler call form for a drag handler: the synthesized DragIntent carrier
/// as-is, or no arguments for a zero-param handler (a `_DragEvent(...)`
/// argument would be extra_positional_arguments on a 0-arity Dart method —
/// mirroring the React backend's arity resolution). `original` is the fn's
/// pre-rename name — the arity lookup keys off it (fn_renames only fire for
/// Dart-type shadowing, e.g. `double` → `double_`).
fn drag_handler_call(renamed: &str, original: &str, obj: &str, ctx: &ViewContext) -> String {
    if ctx.ref_is_zero_param_fn(original) {
        format!("{renamed}();")
    } else {
        format!("{renamed}({obj});")
    }
}

/// Statement body for the merged Focus(onFocusChange:) closure — the bool
/// `v` discriminates focus gain vs blur loss, so the handler must be a
/// CALL (or inline statements), never a closure (which would never run).
/// Returns None for shapes that cannot wire honestly (param-bearing fns).
fn format_focus_change_body(handler: &EventHandlerIR, ctx: &ViewContext) -> Option<String> {
    match handler {
        EventHandlerIR::FnRef { r#ref, .. } => {
            if ctx.widget_names.contains(r#ref) {
                Some(format!("widget.{}();", r#ref))
            } else {
                let renamed = ctx
                    .fn_renames
                    .get(r#ref)
                    .cloned()
                    .unwrap_or_else(|| r#ref.clone());
                if ctx.ref_is_zero_param_fn(r#ref) {
                    Some(format!("{renamed}();"))
                } else {
                    // The bool payload cannot satisfy a param-bearing
                    // handler — honest degradation.
                    None
                }
            }
        }
        EventHandlerIR::InlineFn { body, .. } => Some(format_inline_statements_flutter(body, ctx)),
        EventHandlerIR::StoreAction { r#ref, .. } => Some(format_store_action_handler(r#ref)),
    }
}

/// The Image.network errorBuilder closure: `(context, error, stackTrace) {
/// <handler>; return const SizedBox.shrink(); }` — the builder replaces the
/// failed image and MUST return a Widget, so the handler body is followed
/// by a placeholder. Returns None for unwireable handler shapes.
fn format_error_builder_handler(handler: &EventHandlerIR, ctx: &ViewContext) -> Option<String> {
    let stmts = match handler {
        EventHandlerIR::FnRef { r#ref, .. } => {
            if ctx.widget_names.contains(r#ref) {
                format!("widget.{}();", r#ref)
            } else {
                let renamed = ctx
                    .fn_renames
                    .get(r#ref)
                    .cloned()
                    .unwrap_or_else(|| r#ref.clone());
                if ctx.ref_is_zero_param_fn(r#ref) {
                    format!("{renamed}();")
                } else {
                    return None;
                }
            }
        }
        EventHandlerIR::InlineFn { body, .. } => format_inline_statements_flutter(body, ctx),
        EventHandlerIR::StoreAction { r#ref, .. } => format_store_action_handler(r#ref),
    };
    Some(format!(
        "(context, error, stackTrace) {{ {stmts} return const SizedBox.shrink(); }}"
    ))
}

/// Review LOW-1: Focus.onKeyEvent receives BOTH keydown and keyup events —
/// inject an event-type guard so an `@onKeyDown` handler does not fire on
/// key releases (and vice versa). The guard returns early with
/// `KeyEventResult.ignored`, keeping the event flowing to other handlers.
fn inject_key_event_guard(body: &str, event_type: &str) -> String {
    match body.strip_prefix("(node, event) {") {
        Some(rest) => format!(
            "(node, event) {{ if (event is! {event_type}) {{ return KeyEventResult.ignored; }}{rest}"
        ),
        None => body.to_string(),
    }
}

/// The Focus.onKeyEvent closure: `(node, event) { <stmts>; return
/// KeyEventResult.ignored; }` — `KeyEventResult.ignored` lets the event
/// continue to other key handlers (the DOM default).
fn format_key_handler_flutter(body: &BlockIR, ctx: &ViewContext) -> String {
    if body.statements.is_empty() {
        return "(node, event) { return KeyEventResult.ignored; }".to_string();
    }
    let mut hw = CodeWriter::new();
    hw.emit("(node, event) {");
    hw.indent();
    for stmt in &body.statements {
        let s1 = rewrite_stmt_store_accesses(stmt, ctx.store_provider_map, ctx.ir);
        let s1 = watch_to_read_stmt(&s1);
        let s2 = rewrite_stmt_widget_refs(&s1, ctx.widget_names, ctx.fn_renames);
        let fmt = FlutterStmtFormatter {
            expr: super::r#ref::FlutterFormatExpr::new_with_context(
                None,
                ctx.direct_write_signals,
                ctx.load_builtins,
                ctx.form_present,
                ctx.param_providers,
                false,
            ),
            direct_write_signals: ctx.direct_write_signals.clone(),
            local_names: FxHashSet::default(),
            param_providers: ctx.param_providers.clone(),
        };
        fmt.format_stmt(&mut hw, &s2);
    }
    hw.emit("return KeyEventResult.ignored;");
    hw.dedent();
    hw.emit("}");
    hw.into_string().trim().to_string()
}

/// Format a ConditionalIntent as a Dart ternary expression wrapped in Builder
/// so that setState rebuilds re-evaluate conditions reactively.
/// Detect a bare `form.errors.<field>` read used in a BOOL position (e.g.
/// `@if form.errors.name` or a class-map condition). `format_expr` lowers it
/// to the Dart index access `formErrors["<field>"]` (a `String?`) — Dart
/// rejects that as a ternary condition (`non_bool_condition`). The React
/// runtime semantics (entry truthy iff a NON-EMPTY message is present) map
/// to `(formErrors["<field>"] ?? '').isNotEmpty`. Matches the same
/// nested-chain shape as `FlutterFormatExpr::format_special_field_access`;
/// compound conditions (e.g. `!form.errors.name`) are out of scope and stay
/// as emitted (registered in the Task 8 report as a boundary note).
fn form_errors_bool_condition(cond: &ExprIR) -> Option<String> {
    // outer: form.errors.<field>  (field = the per-field error key)
    let (obj, field) = match cond {
        ExprIR::FieldAccessExpr { object, field } | ExprIR::OptionalChainExpr { object, field } => {
            (object.as_ref(), field.as_str())
        }
        _ => return None,
    };
    // inner: form.errors (must be rooted at `form` — never match unrelated
    // maps, same guard as format_special_field_access)
    let (inner_obj, inner_field) = match obj {
        ExprIR::FieldAccessExpr { object, field } | ExprIR::OptionalChainExpr { object, field } => {
            (object.as_ref(), field.as_str())
        }
        _ => return None,
    };
    if inner_field != "errors" {
        return None;
    }
    if !matches!(inner_obj, ExprIR::IdentifierExpr { name, .. } if name == "form") {
        return None;
    }
    Some(format!("(formErrors[\"{field}\"] ?? '').isNotEmpty"))
}

/// Format a ternary condition — `form.errors.<field>` reads are wrapped
/// into a bool-typed Dart expression (see form_errors_bool_condition).
fn format_bool_condition(ctx: &mut ViewContext, cond: &ExprIR) -> String {
    form_errors_bool_condition(cond).unwrap_or_else(|| ctx.format_expr(cond))
}

fn format_conditional(
    cond: &ConditionalIntent,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    if cond.branches.is_empty() {
        if let Some(ref fb) = cond.fallback {
            return format_single_view(fb, motions, ctx);
        }
        return "SizedBox()".to_string();
    }

    // mode:"fade" — the AnimatedSwitcher below only animates when its child
    // CHANGES identity. Widget.canUpdate treats keyless children of the same
    // runtimeType as the same widget (an in-place update, no transition), so
    // each branch (and the fallback) gets a distinct, stable ValueKey that
    // flips whenever the active branch flips.
    let keyed_branches = cond.transition_mode.as_deref() == Some("fade");

    let mut result = String::new();
    for (i, branch) in cond.branches.iter().enumerate() {
        let cond_str = format_bool_condition(ctx, &branch.condition);
        // Branch bodies must be single widget expressions — multi-node
        // fragments are Column-wrapped (format_view_tree comma-joins,
        // which is only valid inside `children: [...]` lists).
        let body_str = branch
            .body
            .as_ref()
            .map(|b| {
                let inner = format_single_view(b, motions, ctx);
                if keyed_branches {
                    format!("KeyedSubtree(key: ValueKey('omni_if_{i}'), child: {inner})")
                } else {
                    inner
                }
            })
            .unwrap_or_else(|| "SizedBox()".to_string());
        if i == 0 {
            let _ = write!(result, "{} ? {}", cond_str, body_str);
        } else {
            let _ = write!(result, " : {} ? {}", cond_str, body_str);
        }
    }

    // Fallback
    if let Some(ref fb) = cond.fallback {
        let fb_str = format_single_view(fb, motions, ctx);
        let fb_str = if keyed_branches {
            format!("KeyedSubtree(key: ValueKey('omni_if_fb'), child: {fb_str})")
        } else {
            fb_str
        };
        let _ = write!(result, " : {fb_str}");
    } else {
        result.push_str(" : SizedBox()");
    }

    // For wait mode, wrap with AnimatedSwitcher for enter/leave sequencing
    // (unchanged — the ADR-054 wait degradation comments live on the
    // OmniView/React backends).
    //
    // For fade mode, wrap with AnimatedSwitcher using the DEFAULT transition
    // (the design doc's Flutter mapping: fade → AnimatedSwitcher). The DSL
    // exposes no fade duration (transition_mode is a bare string token per
    // the v2 BNF), so the duration matches the wait-mode emission's 300ms.
    if let Some(ref mode) = cond.transition_mode {
        match mode.as_str() {
            "wait" => {
                result = format!(
                    "AnimatedSwitcher(\n  duration: Duration(milliseconds: 300),\n  switchInCurve: Curves.easeOut,\n  switchOutCurve: Curves.easeIn,\n  transitionBuilder: (child, animation) => FadeTransition(opacity: animation, child: child),\n  child: {},\n)",
                    result
                );
            }
            "fade" => {
                result = format!(
                    "AnimatedSwitcher(\n  duration: Duration(milliseconds: 300),\n  child: {},\n)",
                    result
                );
            }
            _ => {}
        }
    }

    // Wrap in Builder so setState triggers re-evaluation of the ternary condition
    format!("Builder(\n  builder: (context) => {},\n)", result)
}

/// Is `node` a single page — a component instance, possibly inside the
/// synthetic "fragment" element that directive lowering wraps every
/// conditional branch body in (ConstructionKind::Conditional)? A
/// single-child fragment unwraps; multi-node branches (Column-wrapped) or
/// plain-subtree branches are not pages.
fn is_component_instance(node: &ViewTree, ctx: &ViewContext) -> bool {
    let el = match node {
        ViewTree::Element(el) => el,
        _ => return false,
    };
    if ctx.component_map.contains_key(el.tag.as_str()) {
        return true;
    }
    if el.tag == "fragment" && el.children.len() == 1 {
        return is_component_instance(&el.children[0], ctx);
    }
    false
}

/// Task 3.9 (flutter MEDIUM): a component-ROOT conditional in the
/// PAGE-SWITCH shape — every branch (and the fallback, when present) is a
/// single component instance (`<PageA/>`), no fade/wait transition — emits
/// an `IndexedStack` instead of the `cond ? child : SizedBox()` ternary.
/// IndexedStack keeps EVERY branch mounted, so switching tabs preserves the
/// outgoing page's state (scroll position, form input); the ternary
/// unmounted it on every switch (the OmniView runtime toggles visibility
/// instead — the Flutter backend diverged).
///
/// `index:` evaluates the SAME condition chain the ternary used
/// (`c0 ? 0 : c1 ? 1 : n`), so exactly one branch is shown — the conversion
/// is selection-semantics preserving; only the mounting behavior changes. A
/// missing fallback keeps the ternary's `: SizedBox()` tail as a shrink slot
/// at the last index (no branch matched → nothing shown).
///
/// Returns `None` when the conditional is NOT the page-switch shape — the
/// caller falls back to the ternary (non-top-level and element-branch
/// conditionals are not page switches; mounting every branch offstage would
/// run hidden initState/side effects for non-page toggles).
pub(crate) fn format_page_switcher(
    cond: &ConditionalIntent,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> Option<String> {
    // Task 3.9 I-1 (3.10): a SINGLE-branch @if with no @else is not the
    // page-switch shape — an IndexedStack would mount the branch
    // unconditionally (hidden when the condition is false), running its
    // initState/timers behind an offstage slot (the criterion-2
    // contradiction). Note `@else` lowers to `fallback` (not a branch), so
    // `@if a { A } @else { B }` (branches=[A], fallback=B) IS a two-page
    // switch and must keep the IndexedStack; only the one-page shape
    // (single branch, no fallback) is excluded.
    if cond.transition_mode.is_some()
        || cond.branches.is_empty()
        || (cond.branches.len() == 1 && cond.fallback.is_none())
    {
        return None;
    }
    let mut children: Vec<String> = Vec::with_capacity(cond.branches.len() + 1);
    for branch in &cond.branches {
        let body = branch.body.as_ref()?;
        if !is_component_instance(body, ctx) {
            return None;
        }
        children.push(format_single_view(body, motions, ctx));
    }
    let mut index_expr = String::new();
    for (i, branch) in cond.branches.iter().enumerate() {
        let cond_str = format_bool_condition(ctx, &branch.condition);
        if i == 0 {
            index_expr = format!("{cond_str} ? {i}");
        } else {
            index_expr = format!("{index_expr} : {cond_str} ? {i}");
        }
    }
    let fallback_idx = cond.branches.len();
    match &cond.fallback {
        None => {
            // No fallback: an unmatched condition must render nothing — the
            // ternary's `: SizedBox()` tail becomes a shrink slot at the
            // last index.
            children.push("SizedBox.shrink()".to_string());
            index_expr = format!("{index_expr} : {fallback_idx}");
        }
        Some(fb) => {
            // A present-but-not-a-page fallback is NOT the page-switch shape
            // — dropping it would lose real content; the ternary keeps it.
            if !is_component_instance(fb, ctx) {
                return None;
            }
            children.push(format_single_view(fb, motions, ctx));
            index_expr = format!("{index_expr} : {fallback_idx}");
        }
    }
    Some(format!(
        "IndexedStack(\n  index: {},\n  children: [\n    {},\n  ],\n)",
        index_expr,
        children.join(",\n    ")
    ))
}

/// Format a ListRenderingIntent for reactive re-rendering.
///
/// Task 5.5 dispatch (design doc 域 6 mapping / spec §7.4):
/// - `[scroll]` hint → `ListView.builder` (lazy viewport)
/// - no hint → `Column(children: [for ...])` — document flow, never an
///   unconditional ListView.builder (review CRITICAL); nested @for stays in
///   flow (inner lists emit nested Columns, no unbounded viewports)
/// - `key={expr}` → `KeyedSubtree(key: ValueKey(expr), child: ...)` per item
///   on both paths
/// - `@empty` → `Builder(builder: (context) => iterable.isEmpty ? empty : rendered)`
fn format_list_rendering(
    list: &ListRenderingIntent,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    let item_name = &list.item_binding.name;
    // Task 3.7 (Riverpod 3.0 scope compliance): the [scroll] path emits
    // `ListView.builder` whose itemBuilder executes at LAYOUT time — after
    // the build phase, where `ref.watch` throws. The itemBuilder body AND
    // the iterable (referenced inside the closure as `{iter}[index]`) must
    // be free of watches: while `lazy_context` is set, every store read is
    // hoisted to a build local. The plain-flow path stays build-scoped (the
    // collection-for runs during build) — there only closure-internal reads
    // (lazy `.where` predicates) are hoisted, which `format_expr` does
    // unconditionally.
    let saved_lazy = ctx.lazy_context;
    if list.scroll_hint {
        ctx.lazy_context = true;
    }
    let iterable_str = ctx.format_expr(&list.iterable);
    // Task 3.8 (perf H2): the [scroll] path references the iterable up to
    // THREE times — `itemCount`, `var item = {iter}[index]` inside the
    // itemBuilder (layout-time, per item), and the @empty Builder — so a
    // COMPUTED iterable (`S.items.filter(...)`) re-evaluates N+1 times per
    // build (a plain Dart getter has no memo → O(N²) for filtered lists).
    // Hoist it to a build local evaluated ONCE per build; the itemBuilder
    // and Builder closures reference the local by capture — the same
    // mechanism as the 3.7 hoisted reads (build-scope locals are legal
    // closure captures; only `ref.watch` calls are layout-forbidden). The
    // name is counter-suffixed so multiple [scroll] @for in one view cannot
    // collide.
    let iter_ref = if list.scroll_hint {
        // First [scroll] @for in the build gets the plain `__omniIter`;
        // subsequent ones (multiple scroll lists in one view) get
        // counter-suffixed names — never collide.
        let name = if ctx.hoisted.iter().any(|(n, _)| n == "__omniIter") {
            format!("__omniIter{}", *ctx.hoist_next)
        } else {
            "__omniIter".to_string()
        };
        *ctx.hoist_next += 1;
        ctx.hoisted.push((name.clone(), iterable_str.clone()));
        name
    } else {
        iterable_str.clone()
    };
    // The item body must be a single widget expression — multi-node
    // fragments are Column-wrapped so the emitted expression stays valid.
    let body_str = list
        .body
        .as_ref()
        .map(|b| format_single_view(b, motions, ctx))
        .unwrap_or_else(|| "SizedBox()".to_string());
    // key={expr} → per-item KeyedSubtree (identity tracking for
    // state-preserving reconciliation on both paths; the item widget cannot
    // take `key:` generically, so the wrapper carries it).
    let item_widget = match &list.key_expr {
        Some(ke) => format!(
            "KeyedSubtree(key: ValueKey({}), child: {})",
            ctx.format_expr(ke),
            body_str
        ),
        None => body_str,
    };
    ctx.lazy_context = saved_lazy;
    let rendered = if list.scroll_hint {
        // Task 9 (§2.3): each `[scroll]` @for consumes the next paginated+
        // scroll @load instance (pairing is by position, declaration order)
        // and attaches its ScrollController — the initState listener fires
        // `{key}LoadMore()` once `pixels >= maxScrollExtent - 200`.
        let mut controller_line = String::new();
        if *ctx.scroll_next < ctx.scroll_keys.len() {
            let key = &ctx.scroll_keys[*ctx.scroll_next];
            *ctx.scroll_next += 1;
            controller_line = format!(
                "  controller: _scrollController{},\n",
                super::pascal_case(key)
            );
        }
        format!(
            "ListView.builder(\n{controller}  itemCount: {iter}.length,\n  itemBuilder: (context, index) {{\n    var {item} = {iter}[index];\n    return {item_widget};\n  }},\n)",
            controller = controller_line, iter = iter_ref, item = item_name, item_widget = item_widget
        )
    } else {
        format!(
            "Column(mainAxisSize: MainAxisSize.min, children: [for (final {item} in {iter}) {item_widget}])",
            item = item_name, iter = iter_ref, item_widget = item_widget
        )
    };
    if list.empty_body.is_some() {
        let empty_str = list
            .empty_body
            .as_ref()
            .map(|b| format_single_view(b, motions, ctx))
            .unwrap_or_else(|| "SizedBox()".to_string());
        format!(
            "Builder(\n  builder: (context) => {}.isEmpty ? {} : {},\n)",
            iter_ref, empty_str, rendered
        )
    } else {
        rendered
    }
}

// ═══════════════════════════════════════════════════════════════
// Inline event handler formatting
// ═══════════════════════════════════════════════════════════════

/// Format an inline handler's STATEMENTS without the closure wrapper —
/// used by handlers that run inside a shared closure (C2b #3:
/// Focus.onFocusChange merges focus/blur into one `(v)` closure, and
/// Image.network's errorBuilder must return a Widget).
fn format_inline_statements_flutter(body: &BlockIR, ctx: &ViewContext) -> String {
    if body.statements.is_empty() {
        return String::new();
    }
    let mut hw = CodeWriter::new();
    for stmt in &body.statements {
        let s1 = rewrite_stmt_store_accesses(stmt, ctx.store_provider_map, ctx.ir);
        let s1 = watch_to_read_stmt(&s1);
        let s2 = rewrite_stmt_widget_refs(&s1, ctx.widget_names, ctx.fn_renames);
        let fmt = FlutterStmtFormatter {
            expr: super::r#ref::FlutterFormatExpr::new_with_context(
                None,
                ctx.direct_write_signals,
                ctx.load_builtins,
                ctx.form_present,
                ctx.param_providers,
                false,
            ),
            direct_write_signals: ctx.direct_write_signals.clone(),
            local_names: FxHashSet::default(),
            param_providers: ctx.param_providers.clone(),
        };
        fmt.format_stmt(&mut hw, &s2);
    }
    hw.into_string().trim().to_string()
}

/// Build an inline callback string from a BlockIR for Flutter event handlers.
/// Store accesses in the body are rewritten to Riverpod provider reads (C4).
/// Task 3.7: reads become `ref.read` — the handler runs at EVENT time
/// (outside the build phase), where Riverpod 3.0 forbids `ref.watch`.
fn format_inline_handler_flutter(body: &BlockIR, ctx: &mut ViewContext) -> String {
    let config = InlineHandlerConfig {
        opener: "() {".into(),
        closer: "}".into(),
        empty_handler: "() {}".into(),
        indent: "    ",
        use_writer: true,
    };
    let rewritten = BlockIR {
        statements: body
            .statements
            .iter()
            .map(|s| {
                let s1 = rewrite_stmt_store_accesses(s, ctx.store_provider_map, ctx.ir);
                let s1 = watch_to_read_stmt(&s1);
                rewrite_stmt_widget_refs(&s1, ctx.widget_names, ctx.fn_renames)
            })
            .collect(),
    };
    build_inline_handler(&rewritten, &config, |w, s| {
        // Task 9 (§2.5): `debounce(fn, ms)` / `throttle(fn, ms)` stdlib calls
        // map to Timer-based closures — per-kind ordinal map keys
        // (`_omniTimers['debounce_{n}']`) keep stable entries across
        // re-renders; mod.rs emits the `_omniTimers` State field + dispose
        // cancellation when the component has timing handlers. The target is
        // formatted through the free functions (NOT `ctx.format_expr` — a
        // `&mut self` receiver would conflict with the closure's disjoint
        // field captures); an event-time call never carries a page/fn-arg
        // hoist, so `use_watch_reads=true` is irrelevant for the fn name.
        if let StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr { name, args },
            ..
        } = s
        {
            if crate::semantic::load_contract::TIMING_FUNCS.contains(&name.as_str()) {
                let is_debounce = name.as_str() == "debounce";
                let n = if is_debounce {
                    let n = ctx.debounce_next.get();
                    ctx.debounce_next.set(n + 1);
                    n
                } else {
                    let n = ctx.throttle_next.get();
                    ctx.throttle_next.set(n + 1);
                    n
                };
                let target = args
                    .first()
                    .map(|a| {
                        format_expr_ir_flutter_ctx(
                            a,
                            ctx.direct_write_signals,
                            ctx.load_builtins,
                            ctx.form_present,
                            ctx.param_providers,
                            true,
                        )
                    })
                    .unwrap_or_default();
                let ms = args
                    .get(1)
                    .map(|a| {
                        format_expr_ir_flutter_ctx(
                            a,
                            ctx.direct_write_signals,
                            ctx.load_builtins,
                            ctx.form_present,
                            ctx.param_providers,
                            true,
                        )
                    })
                    .unwrap_or_default();
                if is_debounce {
                    // trailing-only debounce: cancel any pending timer, then
                    // arm a fresh one that fires the wrapped call.
                    w.emit_fmt(format_args!(
                        "() {{ _omniTimers['debounce_{n}']?.cancel(); _omniTimers['debounce_{n}'] = Timer(const Duration(milliseconds: {ms}), () => {target}()); }}"
                    ));
                } else {
                    // leading-edge throttle: ignore while a timer is active,
                    // then arm the next window.
                    w.emit_fmt(format_args!(
                        "() {{ if (_omniTimers['throttle_{n}']?.isActive ?? false) return; _omniTimers['throttle_{n}'] = Timer(const Duration(milliseconds: {ms}), () => {target}()); }}"
                    ));
                }
                return;
            }
        }
        // Phase 3: inline handler statements carry the D1/D2 set — handler
        // mutations write `_name.value` (no setState in the view tree).
        // Task 8: @load builtin calls (`retry()` / `reset()`) resolve to the
        // per-instance actions; param-signal reads use ref.read (event time).
        generate_statement_flutter_dw(
            w,
            s,
            ctx.direct_write_signals,
            ctx.load_builtins,
            ctx.form_present,
            ctx.param_providers,
            false,
            &FxHashSet::default(),
        )
    })
}

/// Warn that event field accesses in an event handler were dropped because
/// the target callback is a Flutter `VoidCallback` (onPressed/onLongPress/
/// onTap/... receive no event argument — the generated closure is
/// parameterless, so field extractions cannot be preserved).
fn warn_dropped_event_fields(
    ctx: &mut ViewContext,
    event_name: &str,
    callback_name: &str,
    handler_label: &str,
    span: Option<crate::lexer::Span>,
) {
    ctx.warn_at(
        format!(
            "[flutter] event '{event_name}' → callback '{callback_name}' is a \
             VoidCallback in Flutter (no event argument); event field accesses \
             in {handler_label} were dropped"
        ),
        span,
    );
}

/// Format a StoreAction event handler that calls a store action through its
/// Riverpod 3.0 Notifier provider.
///
/// The `r#ref` format is `"StoreName.actionName"`, transformed into
/// `"() {{ ref.read(storeNameProvider.notifier).actionName(); }}"`.
pub(crate) fn format_store_action_handler(r#ref: &str) -> String {
    let parts: Vec<&str> = r#ref.split('.').collect();
    let store = parts.first().copied().unwrap_or(r#ref);
    let action = parts.last().copied().unwrap_or(r#ref);
    let provider = format!("{}Provider", lower_first(store));
    format!("() {{ ref.read({}.notifier).{}(); }}", provider, action)
}
