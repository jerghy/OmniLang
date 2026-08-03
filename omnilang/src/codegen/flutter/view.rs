//! View tree formatting for Flutter codegen.

use crate::codegen::json::escape_literal_str;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::capitalize_first;
use super::design::DesignResolver;
use super::design::{class_factory_expr, decoration_fields_to_expr, merge_decoration_fields};
use super::design::{resolve_class_decoration, resolve_variant_decoration, DecorationFields};
use super::expr::{
    format_expr_ir_flutter, rewrite_stmt_store_accesses, rewrite_stmt_widget_refs,
    rewrite_store_accesses, rewrite_widget_refs,
};
use super::motion::{
    is_flutter_motion_element, motion_active_name, motion_ctrl_name, wrap_with_motion_widgets,
};
use super::motion_widgets::{emit_event_object_flutter, field_extract_flutter};
use super::r#ref::{ref_focus_name, ref_key_name, ref_scroll_name, ref_text_name, RefUsage};
use super::stmt::{generate_statement_flutter, FlutterStmtFormatter};
use super::utils::{
    binding_to_flutter_param, callback_is_void, convert_literal_prop_value, css_color_to_dart,
    event_to_flutter_callback, icon_name_to_dart, lower_first, prop_to_flutter_param_name,
    tag_to_flutter_widget, widget_allows_event, widget_allows_param, widget_has_const_constructor,
    widget_param_rename, widget_param_wrapper,
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
    pub(crate) warnings: &'a mut Vec<String>,
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
}

impl ViewContext<'_> {
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

    /// The referenced fn's FIRST param type, when it has exactly one param
    /// (for cast-coercion wrappers on nullable event payloads).
    fn ref_single_param_type(&self, ref_name: &str) -> Option<String> {
        let find = |f: &FunctionIR| -> Option<String> {
            if f.name == ref_name && f.params.len() == 1 {
                f.params[0]
                    .r#type
                    .as_ref()
                    .map(|t| super::super::dart::dart_type_ir(t))
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
    fn format_expr(&self, expr: &ExprIR) -> String {
        let rewritten = rewrite_store_accesses(expr, self.store_provider_map, self.ir);
        let rewritten = rewrite_widget_refs(&rewritten, self.widget_names, self.fn_renames);
        format_expr_ir_flutter(&rewritten)
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
    format!(
        "Column(mainAxisSize: MainAxisSize.min, children: [{}])",
        children.join(", ")
    )
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

/// Collect slot declarations (name + type constraint) from a view tree.
/// Used to emit `Widget?` constructor params for declared slots. Slots
/// inside a child component's children are consumer-side projections, not
/// declarations — they are skipped (the child component types them via its
/// own declarations; mirrors the React backend).
pub(crate) fn collect_view_slots(
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    acc: &mut Vec<(Option<String>, Option<TypeIR>)>,
) {
    match node {
        ViewTree::Slot {
            name,
            type_constraint,
            fallback,
            ..
        } => {
            acc.push((name.clone(), type_constraint.clone()));
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
/// The scan is conservative: any usage (resolved or not) counts, so an
/// unresolved reference leaves the import unused rather than a resolved one
/// missing it.
pub(crate) fn view_uses_design_classes(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            let class_prop_used = match el.props.get("class") {
                Some(ExprIR::LiteralExpr { value }) => {
                    value.as_str().is_some_and(|s| !s.trim().is_empty())
                }
                Some(_) => true,
                None => false,
            };
            if class_prop_used || el.props.contains_key("_design_variant") {
                return true;
            }
            // Literal-false bindings emit nothing — they do not need design.dart.
            if el.class_map_bindings.values().any(|e| match e {
                ExprIR::LiteralExpr { value } => value.as_bool() != Some(false),
                _ => true,
            }) {
                return true;
            }
            el.children.iter().any(view_uses_design_classes)
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_uses_design_classes))
                || cond.fallback.as_ref().is_some_and(view_uses_design_classes)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_uses_design_classes)
                || list
                    .empty_body
                    .as_ref()
                    .is_some_and(view_uses_design_classes)
        }
        ViewTree::Fragment(children) => children.iter().any(view_uses_design_classes),
        ViewTree::Slot { fallback, .. } => fallback.iter().any(view_uses_design_classes),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
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
            let mut acc: Vec<(Option<String>, Option<TypeIR>)> = Vec::new();
            collect_view_slots(view, component_map, &mut acc);
            for (n, _) in acc {
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
        "background-color" => Some("color"),
        _ => None,
    }
}

/// Resolve one bare class name against the design IR. Class names arrive
/// bare (the `Design.` prefix is dropped during lowering — `class:
/// LearnDesign.Card` becomes the `class` prop "Card"). Returns the resolved
/// decoration fields plus the `AppDesign.<class>()` factory reference (valid
/// only when the decoration derives from exactly this one class). Unknown
/// class names and layout-only classes warn and return `None`.
fn resolve_static_class(
    name: &str,
    ctx: &mut ViewContext,
) -> Option<(DecorationFields, Option<String>)> {
    let Some((design, class)) = ctx.design.class_owner(name) else {
        ctx.warnings.push(format!(
            "[flutter] class '{name}' is not declared by any design block; \
             skipped (no Flutter equivalent)"
        ));
        return None;
    };
    let (fields, skipped) = resolve_class_decoration(design, class);
    warn_design_skipped(ctx, &format!("class '{name}'"), skipped);
    if fields.is_empty() {
        ctx.warnings.push(format!(
            "[flutter] design class '{name}' has no decoration-mappable properties; \
             skipped (layout/text-only classes have no Flutter decoration equivalent)"
        ));
        None
    } else {
        let factory = class_factory_expr(design, class);
        Some((fields, factory))
    }
}

/// Surface unmappable (property, value) pairs on the warnings channel,
/// naming the resolved thing they belong to (class/variant).
fn warn_design_skipped(ctx: &mut ViewContext, what: &str, skipped: Vec<(String, String)>) {
    for (prop, value) in skipped {
        ctx.warnings.push(format!(
            "[flutter] {what} property '{prop}' value '{value}' is not mappable to a \
             BoxDecoration field; skipped"
        ));
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
            ctx.warnings.push(format!(
                "[flutter] non-literal `class` prop values are not supported; \
                 class styling skipped"
            ));
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
        if let Some(resolved) = resolve_static_class(&name, ctx) {
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
                        warn_design_skipped(ctx, &format!("variant '{variant_name}'"), skipped);
                        if fields.is_empty() {
                            ctx.warnings.push(format!(
                                "[flutter] variant '{variant_name}' of design \
                                 '{design_name}' has no decoration-mappable properties; \
                                 no visual effect on Flutter"
                            ));
                        } else {
                            variant_fields = Some(fields);
                        }
                    }
                    None => ctx.warnings.push(format!(
                        "[flutter] variant '{variant_name}' not found in design \
                         '{design_name}'"
                    )),
                },
                None => ctx.warnings.push(format!(
                    "[flutter] design '{design_name}' not found for variant binding"
                )),
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
                if let Some(resolved) = resolve_static_class(name, ctx) {
                    static_entries.push((name.clone(), resolved.0, resolved.1));
                }
            }
            ExprIR::LiteralExpr { value } if value.as_bool() == Some(false) => {}
            _ => {
                if let Some(resolved) = resolve_static_class(name, ctx) {
                    dynamic.push((ctx.format_expr(expr), resolved.0));
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
fn role_to_semantics_flag(role: &str) -> Option<&'static str> {
    match role {
        "button" => Some("button: true"),
        "textField" => Some("textField: true"),
        "link" => Some("link: true"),
        "checkbox" => Some("checkbox: true"),
        "radio" => Some("radio: true"),
        "image" => Some("image: true"),
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
        semantics_args.push(format!("label: '{}'", escape_literal_str(label, '\'')));
    }
    if let Some(ref role) = a11y.role {
        match role_to_semantics_flag(role) {
            Some(flag) => semantics_args.push(flag.to_string()),
            None => {
                ctx.warnings.push(format!(
                    "[flutter] a11y role '{role}' is not in the Flutter \
                     Semantics subset (button/textField/link/checkbox/radio/\
                     image); degraded — no Semantics flag (the role still \
                     passes through on the web targets)"
                ));
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
                ctx.warnings.push(
                    "[flutter] a11y alt 仅对 img 元素生效（当前元素非图像）；\
                     与 a11y label 同时出现，alt 被忽略（label 已提供可访问名称）"
                        .into(),
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
            degradation_comment = format!(
                "/* a11y: role '{role}' has no Flutter Semantics flag \
                 (subset: button/textField/link/checkbox/radio/image); \
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
    for (_, v) in &el.props {
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

/// Format an ElementIR node as a Flutter widget constructor expression.
/// Wraps with motion animation widgets (AnimatedBuilder, MouseRegion, GestureDetector)
/// when the element has motion triggers.
fn format_element(el: &ElementIR, motions: &[MotionIntent], ctx: &mut ViewContext) -> String {
    // Component composition: check if tag is a known component
    if let Some(_child_comp) = ctx.component_map.get(el.tag.as_str()) {
        let mut child_args: Vec<String> = Vec::new();
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
                            format!("{renamed}")
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
                    ctx.warnings.push(format!(
                        "[flutter] component '{}' has no slot '{}'; content emitted as default child",
                        el.tag, name
                    ));
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
            ctx.warnings.push(format!(
                "[flutter] component '{}' declares no default slot; {} child node(s) dropped",
                el.tag,
                default_children.len()
            ));
            format!("{}({})", el.tag, args_str)
        };
        // Design classes/variants resolve to a decoration on a wrapping
        // Container — real visual output on component instantiations too.
        let decorated = wrap_design_decorations(el, child_expr, ctx);
        // a11y block → Semantics/ExcludeSemantics wrappers around the
        // component widget (the semantic node describes the whole subtree).
        return wrap_a11y(el, decorated, ctx);
    }
    // Otherwise, fall through to HTML tag mapping
    let widget = tag_to_flutter_widget(&el.tag);
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
            args.insert(0, ctx.format_expr(value));
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
                    ctx.warnings.push(format!(
                        "[flutter] icon name {bad} has no Material Icons mapping; \
                         falling back to Icons.help_outline"
                    ));
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
        if !widget_allows_param(widget, param) {
            ctx.warnings.push(format!(
                "[flutter] prop '{name}' → param '{param}' is not supported on '{widget}'; skipped"
            ));
            continue;
        }
        args.push(format!("{}: {}", param, formatted));
    }
    // Merge wrapped params into sub-object constructor args
    for (wrapper, members) in wrapped {
        let arg_name = match wrapper {
            "TextStyle" => "style",
            "InputDecoration" => "decoration",
            _ => wrapper,
        };
        args.push(format!("{}: {}({})", arg_name, wrapper, members.join(", ")));
    }

    // a11y block (design doc 域 10): `alt` on image widgets maps to the
    // Image.network `semanticLabel` constructor param — the same mapping as
    // the plain `alt` prop (utils.rs prop_to_flutter_param_name). Skipped
    // when a literal `alt` prop already carries the semanticLabel, so the
    // generated Dart never duplicates the named argument.
    if widget == "Image.network" {
        if let Some(ref alt) = el.a11y.as_ref().and_then(|a| a.alt.as_ref()) {
            if !el.props.contains_key("alt") {
                args.push(format!("semanticLabel: {}", ctx.format_expr(alt)));
            }
        }
    }

    // Inline style (style: { prop: value }) — map CSS properties to Flutter widget parameters
    if let Some(obj) = el
        .inline_style
        .as_ref()
        .and_then(|style_json| style_json.as_object())
    {
        for (css_prop, css_val) in obj {
            let css_val_str = css_val.as_str().unwrap_or("");
            // T5.7: inline-style props go through the same per-widget
            // whitelist as plain props — `Text(style: { color: ... })` used
            // to emit `color: Color(...)` on a widget with no `color` param
            // (invalid Dart; review's String->Color error class).
            let inline_param = css_prop_to_flutter_param(css_prop);
            if let Some(param) = inline_param {
                if !widget_allows_param(widget, param) {
                    ctx.warnings.push(format!(
                        "[flutter] inline style property '{css_prop}' → param '{param}' \
                         is not supported on '{widget}'; skipped"
                    ));
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
                    match css_color_to_dart(css_val_str) {
                        Some(color_expr) => args.push(format!("color: {}", color_expr)),
                        None => ctx.warnings.push(format!(
                            "[flutter] unsupported color value '{css_val_str}' for '{css_prop}'; skipped"
                        )),
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

    // Events -> callback parameters (C6: per-widget event whitelist)
    for (name, handler) in &el.events {
        let callback_name = event_to_flutter_callback(name);
        if !widget_allows_event(widget, &callback_name) {
            ctx.warnings.push(format!(
                "[flutter] event '{name}' → callback '{callback_name}' is not supported on '{widget}'; skipped"
            ));
            continue;
        }
        // VoidCallback-family params (onPressed/onLongPress/...) receive NO
        // event argument in Flutter — the closure must be parameterless or
        // the generated Dart fails to compile (argument_type_not_assignable).
        let is_void = callback_is_void(&callback_name);
        let body = match handler {
            EventHandlerIR::FnRef { r#ref, .. } => {
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
                        ctx.warnings,
                        name,
                        &callback_name,
                        &format!("'{ref}'"),
                    );
                }
                if ctx.widget_names.contains(r#ref) {
                    format!("widget.{}", r#ref)
                } else {
                    let renamed = ctx
                        .fn_renames
                        .get(r#ref)
                        .cloned()
                        .unwrap_or_else(|| r#ref.clone());
                    if !is_void && ctx.ref_is_zero_param_fn(r#ref) {
                        // A 0-arg fn tear-off is not assignable to the
                        // payload-bearing callback types (`ValueChanged<String>`)
                        // — accept the event param and ignore it.
                        format!("(event) {{ {renamed}(); }}")
                    } else {
                        format!("{renamed}")
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
                        "e",
                        ctx.warnings,
                    );
                    let fields: Vec<String> = match handler.used_fields() {
                        Some(ref f) if !f.is_empty() => f.clone(),
                        _ => vec![],
                    };
                    let mut hw = CodeWriter::new();
                    hw.emit("(e) {");
                    hw.indent();
                    // Emit individual field extractions as local variables
                    // This shadows e.field references in the body with extracted values
                    for field in &fields {
                        let extract = field_extract_flutter(
                            handler.event_data().as_ref().unwrap(),
                            field,
                            "e",
                            ctx.warnings,
                        );
                        hw.emit_fmt(format_args!("final {} = {};", field, extract));
                    }
                    for stmt in &body.statements {
                        // Store accesses inside handler bodies resolve to
                        // ref.watch/ref.read on the Riverpod providers (C4);
                        // widget-scope props/events rewrite to widget.<name>.
                        let s1 = rewrite_stmt_store_accesses(stmt, ctx.store_provider_map, ctx.ir);
                        let s2 = rewrite_stmt_widget_refs(&s1, ctx.widget_names, ctx.fn_renames);
                        // Use custom formatter with event_param_name set
                        // so FieldAccess on "e" emits bare field name (local final binding)
                        let fmt = FlutterStmtFormatter {
                            expr: super::r#ref::FlutterFormatExpr::new(Some("e".to_string())),
                        };
                        fmt.format_stmt(&mut hw, &s2);
                    }
                    hw.dedent();
                    hw.emit("}");
                    let result = hw.into_string().trim().to_string();
                    result
                } else {
                    if is_void
                        && handler
                            .used_fields()
                            .as_ref()
                            .is_some_and(|f| !f.is_empty())
                    {
                        warn_dropped_event_fields(
                            ctx.warnings,
                            name,
                            &callback_name,
                            "the handler",
                        );
                    }
                    format_inline_handler_flutter(body, ctx)
                }
            }
            EventHandlerIR::StoreAction { r#ref, .. } => format_store_action_handler(r#ref),
        };
        args.push(format!("{}: {}", callback_name, body));
    }

    // Bindings (C6: per-widget whitelist)
    for (bind_name, signal_ref) in &el.bindings {
        let param = binding_to_flutter_param(bind_name);
        if param.is_empty() {
            continue;
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
        if !widget_allows_param(widget, param) {
            ctx.warnings.push(format!(
                "[flutter] binding 'bind:{bind_name}' → param '{param}' is not supported on '{widget}'; skipped"
            ));
            continue;
        }
        args.push(format!("{}: {}", param, signal_ref.name));
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

    // Ref binding (Step 6): attach the GlobalKey plus any on-demand
    // controllers the component uses on this ref. Controllers attach only to
    // widgets that support them (TextField/scrollables); focus on other
    // widgets falls back to a Focus wrapper below.
    let mut focus_wrap_ref: Option<&str> = None;
    if let Some(ref_name) = &el.ref_binding {
        let usage = ctx.ref_usage.get(ref_name).copied().unwrap_or_default();
        args.push(format!("key: {}", ref_key_name(ref_name)));
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
    let mut text_column_fallback: Option<Vec<String>> = None;
    if el.children.is_empty() {
        // No children
    } else if widget == "Text" {
        // Text widgets take their content as the positional `data`
        // argument — `Text(child: ...)` is invalid Dart (Text has no
        // `child` parameter). Text-ish children (plain text nodes /
        // interpolations) concatenate into the data string; non-text
        // children degrade to a Column-wrapped expression so the output
        // still compiles.
        let mut parts: Vec<String> = Vec::new();
        let mut all_text = true;
        for child in &el.children {
            match child {
                ViewTree::Text(t) => parts.push(t.replace('\'', "\\'")),
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
            ctx.warnings.push(format!(
                "[flutter] element '{}' maps to Text but has non-text children; \
                 rendering a Column instead (Text cannot host child widgets)",
                el.tag
            ));
            text_column_fallback = Some(
                el.children
                    .iter()
                    .map(|c| format_view_tree(c, motions, ctx))
                    .collect(),
            );
        }
    } else if el.children.len() == 1 {
        let child_str = format_view_tree(&el.children[0], motions, ctx);
        args.push(format!("child: {}", child_str));
    } else {
        let children: Vec<String> = el
            .children
            .iter()
            .map(|c| format_view_tree(c, motions, ctx))
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

    let inner = if let Some(children) = text_column_fallback {
        wrap_multi_widget(&children)
    } else if args.is_empty() {
        // Task 5.8: an empty element mapped to Text (`<span/>` with style
        // only) emitted `Text()` — Text requires its positional `data`
        // argument (1 positional expected, 0 found).
        if widget == "Text" {
            "Text('')".to_string()
        } else {
            format!("{}()", widget)
        }
    } else {
        format!("{}({})", widget, args.join(", "))
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
    let has_inview = el
        .motion_behavior()
        .and_then(|m| m.in_view_once.as_ref().or(m.in_view_repeat.as_ref()))
        .is_some();
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
                result = format!(
                    "OmniInViewWidget(\n  once: {},\n  threshold: {},\n  onInView: () {{ {} = true; {}.forward(); }},\n  child: {},\n)",
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
                                        ctx.warnings.push(format!(
                                            "@drag-{} without constraints and motion '{}' has no literal translateX/Y keyframe values — drag is unbounded (no movement); add a literal translate keyframe range to the motion",
                                            if is_horizontal { "horizontal" } else { "vertical" },
                                            motion_name
                                        ));
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
                            let (elastic_val, threshold_val, velocity_max_val) = match &trigger.kind
                            {
                                TriggerKind::DragHorizontal {
                                    elastic,
                                    threshold,
                                    velocity_max,
                                    ..
                                }
                                | TriggerKind::DragVertical {
                                    elastic,
                                    threshold,
                                    velocity_max,
                                    ..
                                } => (
                                    elastic
                                        .as_ref()
                                        .map(|e| ctx.format_expr(e))
                                        .unwrap_or_else(|| "0.0".to_string()),
                                    threshold
                                        .as_ref()
                                        .map(|e| ctx.format_expr(e))
                                        .unwrap_or_else(|| "0.0".to_string()),
                                    velocity_max
                                        .as_ref()
                                        .map(|e| ctx.format_expr(e))
                                        .unwrap_or_else(|| "2000.0".to_string()),
                                ),
                                _ => ("0.0".to_string(), "0.0".to_string(), "2000.0".to_string()),
                            };
                            result = format!(
                                "{extent_warning}OmniDragWidget(\n  axis: {},\n{}  elastic: {},\n  threshold: {},\n  velocityMax: {},\n  dragExtent: {},\n  onDragStart: () {{ {} = true; }},\n  onDragProgress: (p) {{ {} = true; {}.value = p; }},\n  onDragEnd: (completed) {{ if (completed) {{ {}.forward(); }} else {{ {}.reverse().whenComplete(() {{ if (mounted && {}.value == 0.0) setState(() {{ {} = false; }}); }}); }} }},\n  child: {},\n)",
                                axis_type, constraints_args, elastic_val, threshold_val, velocity_max_val, extent_dart, active_name, active_name, ctrl_name, ctrl_name, ctrl_name, ctrl_name, active_name, result,
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

    result
}

/// Format a ConditionalIntent as a Dart ternary expression wrapped in Builder
/// so that setState rebuilds re-evaluate conditions reactively.
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
        let cond_str = ctx.format_expr(&branch.condition);
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
    let iterable_str = ctx.format_expr(&list.iterable);
    let item_name = &list.item_binding.name;
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
    let rendered = if list.scroll_hint {
        format!(
            "ListView.builder(\n  itemCount: {iter}.length,\n  itemBuilder: (context, index) {{\n    var {item} = {iter}[index];\n    return {item_widget};\n  }},\n)",
            iter = iterable_str, item = item_name, item_widget = item_widget
        )
    } else {
        format!(
            "Column(mainAxisSize: MainAxisSize.min, children: [for (final {item} in {iter}) {item_widget}])",
            item = item_name, iter = iterable_str, item_widget = item_widget
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
            iterable_str, empty_str, rendered
        )
    } else {
        rendered
    }
}

// ═══════════════════════════════════════════════════════════════
// Inline event handler formatting
// ═══════════════════════════════════════════════════════════════

/// Build an inline callback string from a BlockIR for Flutter event handlers.
/// Store accesses in the body are rewritten to Riverpod provider reads (C4).
fn format_inline_handler_flutter(body: &BlockIR, ctx: &ViewContext) -> String {
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
                rewrite_stmt_widget_refs(&s1, ctx.widget_names, ctx.fn_renames)
            })
            .collect(),
    };
    build_inline_handler(&rewritten, &config, generate_statement_flutter)
}

/// Warn that event field accesses in an event handler were dropped because
/// the target callback is a Flutter `VoidCallback` (onPressed/onLongPress/
/// onTap/... receive no event argument — the generated closure is
/// parameterless, so field extractions cannot be preserved).
fn warn_dropped_event_fields(
    warnings: &mut Vec<String>,
    event_name: &str,
    callback_name: &str,
    handler_label: &str,
) {
    warnings.push(format!(
        "[flutter] event '{event_name}' → callback '{callback_name}' is a \
         VoidCallback in Flutter (no event argument); event field accesses \
         in {handler_label} were dropped"
    ));
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
