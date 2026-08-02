//! View tree formatting for Flutter codegen.

use crate::codegen::json::escape_literal_str;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::capitalize_first;
use super::expr::{format_expr_ir_flutter, rewrite_stmt_store_accesses, rewrite_store_accesses};
use super::motion::wrap_with_motion_widgets;
use super::motion_widgets::{emit_event_object_flutter, field_extract_flutter};
use super::r#ref::{ref_focus_name, ref_key_name, ref_scroll_name, ref_text_name, RefUsage};
use super::stmt::{generate_statement_flutter, FlutterStmtFormatter};
use super::utils::{
    binding_to_flutter_param, callback_is_void, css_color_to_dart, event_to_flutter_callback,
    lower_first, prop_to_flutter_param_name, tag_to_flutter_widget, widget_allows_event,
    widget_allows_param, widget_param_rename, widget_param_wrapper,
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
    /// Compiler warnings accumulated during view formatting (C6).
    pub(crate) warnings: &'a mut Vec<String>,
}

impl ViewContext<'_> {
    /// Format an expression in view context — `StoreFieldAccessExpr` is
    /// rewritten to Riverpod 3.0 `ref.watch(...)` / `ref.read(...)` forms (C4).
    fn format_expr(&self, expr: &ExprIR) -> String {
        format_expr_ir_flutter(&rewrite_store_accesses(
            expr,
            self.store_provider_map,
            self.ir,
        ))
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
            format!("Text('{}')", escaped.replace('$', "\\$"))
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
        // Props become constructor parameters
        for (prop_name, prop_value) in &el.props {
            child_args.push(format!("{}: {}", prop_name, ctx.format_expr(prop_value)));
        }
        // Events become callback parameters
        for (event_name, handler) in &el.events {
            let cb = match handler {
                EventHandlerIR::FnRef { r#ref, .. } => {
                    format!("() {{ {}(); }}", r#ref)
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
        if default_children.is_empty() {
            return format!("{}({})", el.tag, args_str);
        }
        if target_slots.contains("children") {
            let children_arg = if default_children.len() == 1 {
                default_children[0].clone()
            } else {
                wrap_multi_widget(&default_children)
            };
            return format!("{}({}, children: {})", el.tag, args_str, children_arg);
        }
        ctx.warnings.push(format!(
            "[flutter] component '{}' declares no default slot; {} child node(s) dropped",
            el.tag,
            default_children.len()
        ));
        return format!("{}({})", el.tag, args_str);
    }
    // Otherwise, fall through to HTML tag mapping
    let widget = tag_to_flutter_widget(&el.tag);
    let mut args: Vec<String> = Vec::new();

    // Props -> widget constructor parameters (C6: per-widget whitelist —
    // unmapped params emit a compiler warning instead of invalid Dart).
    // Params that belong to a sub-object (TextStyle/InputDecoration) are
    // collected and merged into a single wrapper argument.
    let mut wrapped: Vec<(&'static str, Vec<String>)> = Vec::new();
    for (name, value) in &el.props {
        let formatted = ctx.format_expr(value);
        let param = widget_param_rename(widget, prop_to_flutter_param_name(name));
        if param.is_empty() {
            continue; // slug props (class/style/id/value/...) — intentionally dropped
        }
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

    // Inline style (style: { prop: value }) — map CSS properties to Flutter widget parameters
    if let Some(obj) = el
        .inline_style
        .as_ref()
        .and_then(|style_json| style_json.as_object())
    {
        for (css_prop, css_val) in obj {
            let css_val_str = css_val.as_str().unwrap_or("");
            match css_prop_to_flutter_param(css_prop) {
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
            EventHandlerIR::FnRef {
                r#ref, event_data, ..
            } => {
                if event_data.is_some() && !is_void {
                    let event_obj = emit_event_object_flutter(
                        handler.event_data(),
                        handler.used_fields(),
                        "event",
                        ctx.warnings,
                    );
                    format!("(event) {{ {}({}); }}", r#ref, event_obj)
                } else {
                    // Symmetric with the InlineFn path: warn when event
                    // field accesses were dropped (VoidCallback receives
                    // no event argument in Flutter).
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
                            &format!("'{ref}'"),
                        );
                    }
                    format!("() {{ {}(); }}", r#ref)
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
                        // ref.watch/ref.read on the Riverpod providers (C4).
                        let stmt =
                            rewrite_stmt_store_accesses(stmt, ctx.store_provider_map, ctx.ir);
                        // Use custom formatter with event_param_name set
                        // so FieldAccess on "e" emits bare field name (local final binding)
                        let fmt = FlutterStmtFormatter {
                            expr: super::r#ref::FlutterFormatExpr::new(Some("e".to_string())),
                        };
                        fmt.format_stmt(&mut hw, &stmt);
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
                        warn_dropped_event_fields(ctx.warnings, name, &callback_name, "the handler");
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
        format!("{}()", widget)
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

    // ── Dynamic class bindings (class_map_bindings) ──
    // Map class:active={condition} syntax to conditional decoration in Flutter.
    // For each (class_name, condition_expr), emit a conditional BoxDecoration
    // on a wrapping Container. Literal true/false expressions are handled
    // statically (always applied / never applied).
    let inner = if !el.class_map_bindings.is_empty() {
        let mut has_dynamic = false;
        let mut dynamic_conditions: Vec<String> = Vec::new();

        for expr in el.class_map_bindings.values() {
            match expr {
                ExprIR::LiteralExpr { value } if value.as_bool() == Some(true) => {
                    // Always-applied class (literal true):
                    // In a full implementation with style_classes resolution,
                    // this would add static style properties.
                }
                ExprIR::LiteralExpr { value } if value.as_bool() == Some(false) => {
                    // Never-applied class (literal false): skip entirely.
                }
                _ => {
                    let cond = ctx.format_expr(expr);
                    has_dynamic = true;
                    dynamic_conditions.push(cond);
                }
            }
        }

        if has_dynamic {
            let cond = dynamic_conditions.join(" && ");
            // Wrap in a Container with conditional decoration.
            // The decoration applies BoxDecoration when the condition is true.
            // TODO: When style_classes resolution is implemented, use the
            //       actual resolved class styles instead of BoxDecoration().
            format!(
                "Container(\n  decoration: {} ? BoxDecoration() : null,\n  child: {},\n)",
                cond, inner
            )
        } else {
            inner
        }
    } else {
        inner
    };

    // Wrap with motion animation widgets if the element has triggers
    let has_inview = el
        .motion_behavior()
        .and_then(|m| m.in_view_once.as_ref().or(m.in_view_repeat.as_ref()))
        .is_some();
    let has_scroll = el.scroll_behavior().is_some();
    let result = if el.motion_behavior().is_some_and(|m| !m.triggers.is_empty())
        || has_inview
        || has_scroll
    {
        let mut result = inner;

        // Standard motion triggers (enter, hover, press, etc.)
        if el.motion_behavior().is_some_and(|m| !m.triggers.is_empty()) {
            result = wrap_with_motion_widgets(
                result,
                el.motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers),
                motions,
            );
        }

        // Handle @in-view trigger: wrap with _OmniInViewWidget
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
                let ctrl_name = format!("_{}Controller", motion_name);
                let active_name = format!("_{}Active", motion_name);
                result = format!(
                    "_OmniInViewWidget(\n  once: {},\n  threshold: {},\n  onInView: () {{ {} = true; {}.forward(); }},\n  child: {},\n)",
                    once, threshold, active_name, ctrl_name, result
                );
            }
        }

        // Handle @scroll trigger: wrap with _OmniScrollDrivenWidget
        if has_scroll {
            let scroll_start = el
                .scroll_behavior()
                .and_then(|s| s.start.as_deref())
                .unwrap_or("top 80%");
            let scroll_end = el
                .scroll_behavior()
                .and_then(|s| s.end.as_deref())
                .unwrap_or("bottom 20%");
            let scrub = match el.scroll_behavior().and_then(|s| s.scrub.as_ref()) {
                Some(expr) => ctx.format_expr(expr),
                None => "0.0".to_string(),
            };
            let motion_ref = el
                .motion_behavior()
                .and_then(|m| {
                    m.triggers
                        .iter()
                        .find(|t| matches!(t.kind, TriggerKind::Scroll))
                })
                .and_then(|t| t.motion_ref.as_ref());

            if let Some(motion_name) = motion_ref {
                let ctrl_name = format!("_{}Controller", motion_name);
                let active_name = format!("_{}Active", motion_name);
                result = format!(
                    "_OmniScrollDrivenWidget(\n  start: '{}',\n  end: '{}',\n  scrub: {},\n  onProgress: (p) {{ {} = true; {}.value = p; }},\n  child: {},\n)",
                    scroll_start, scroll_end, scrub, active_name, ctrl_name, result
                );
            }
        }

        // Handle @drag-horizontal / @drag-vertical trigger: wrap with _OmniDragWidget
        let has_drag = el.motion_behavior().is_some_and(|m| {
            m.triggers.iter().any(|t| {
                matches!(
                    t.kind,
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical
                )
            })
        });
        if has_drag {
            for trigger in el
                .motion_behavior()
                .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
            {
                match trigger.kind {
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        if let Some(motion_name) = trigger.motion_ref.as_ref() {
                            let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                            let axis_type = if is_horizontal {
                                "Axis.horizontal"
                            } else {
                                "Axis.vertical"
                            };
                            let ctrl_name = format!("_{}Controller", motion_name);
                            let active_name = format!("_{}Active", motion_name);
                            let constraints_str = el
                                .drag_behavior()
                                .and_then(|d| d.constraints.as_deref())
                                .unwrap_or("");
                            // ADR-053 removed @drag-constraints — when none are
                            // given, drag the extent from the motion's own
                            // keyframe range in the drag axis so the element
                            // follows its animation (deferral #19). 0.0 = no
                            // usable range → unbounded (inert, previous behavior).
                            let (extent_dart, extent_warning) = if constraints_str.is_empty() {
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
                            let elastic_val =
                                match el.drag_behavior().and_then(|d| d.elastic.as_ref()) {
                                    Some(expr) => ctx.format_expr(expr),
                                    None => "0.0".to_string(),
                                };
                            let threshold_val =
                                match el.drag_behavior().and_then(|d| d.threshold.as_ref()) {
                                    Some(expr) => ctx.format_expr(expr),
                                    None => "0.0".to_string(),
                                };
                            result = format!(
                                "{extent_warning}_OmniDragWidget(\n  axis: {},\n  constraints: '{}',\n  elastic: {},\n  threshold: {},\n  dragExtent: {},\n  onDragStart: () {{ {} = true; }},\n  onDragProgress: (p) {{ {} = true; {}.value = p; }},\n  onDragEnd: (completed) {{ if (completed) {{ {}.forward(); }} else {{ {}.reverse().whenComplete(() {{ if (mounted && {}.value == 0.0) setState(() {{ {} = false; }}); }}); }} }},\n  child: {},\n)",
                                axis_type, constraints_str, elastic_val, threshold_val, extent_dart, active_name, active_name, ctrl_name, ctrl_name, ctrl_name, ctrl_name, active_name, result,
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

    let mut result = String::new();
    for (i, branch) in cond.branches.iter().enumerate() {
        let cond_str = ctx.format_expr(&branch.condition);
        // Branch bodies must be single widget expressions — multi-node
        // fragments are Column-wrapped (format_view_tree comma-joins,
        // which is only valid inside `children: [...]` lists).
        let body_str = branch
            .body
            .as_ref()
            .map(|b| format_single_view(b, motions, ctx))
            .unwrap_or_else(|| "SizedBox()".to_string());
        if i == 0 {
            let _ = write!(result, "{} ? {}", cond_str, body_str);
        } else {
            let _ = write!(result, " : {} ? {}", cond_str, body_str);
        }
    }

    // Fallback
    if let Some(ref fb) = cond.fallback {
        let _ = write!(result, " : {}", format_single_view(fb, motions, ctx));
    } else {
        result.push_str(" : SizedBox()");
    }

    // For wait mode, wrap with AnimatedSwitcher for enter/leave sequencing
    if let Some(ref mode) = cond.transition_mode {
        if mode == "wait" {
            result = format!(
                "AnimatedSwitcher(\n  duration: Duration(milliseconds: 300),\n  switchInCurve: Curves.easeOut,\n  switchOutCurve: Curves.easeIn,\n  transitionBuilder: (child, animation) => FadeTransition(opacity: animation, child: child),\n  child: {},\n)",
                result
            );
        }
    }

    // Wrap in Builder so setState triggers re-evaluation of the ternary condition
    format!("Builder(\n  builder: (context) => {},\n)", result)
}

/// Format a ListRenderingIntent as a ListView.builder for reactive re-rendering.
fn format_list_rendering(
    list: &ListRenderingIntent,
    motions: &[MotionIntent],
    ctx: &mut ViewContext,
) -> String {
    let iterable_str = ctx.format_expr(&list.iterable);
    let item_name = &list.item_binding.name;
    // The item body must be a single widget expression — multi-node
    // fragments are Column-wrapped so `return <body>` stays valid.
    let body_str = list
        .body
        .as_ref()
        .map(|b| format_single_view(b, motions, ctx))
        .unwrap_or_else(|| "SizedBox()".to_string());
    let list_builder = format!(
        "ListView.builder(\n  itemCount: {iter}.length,\n  itemBuilder: (context, index) {{\n    var {item} = {iter}[index];\n    return {body};\n  }},\n)",
        iter = iterable_str, item = item_name, body = body_str
    );
    if list.empty_body.is_some() {
        let empty_str = list
            .empty_body
            .as_ref()
            .map(|b| format_single_view(b, motions, ctx))
            .unwrap_or_else(|| "SizedBox()".to_string());
        format!(
            "Builder(\n  builder: (context) => {}.isEmpty ? {} : {},\n)",
            iterable_str, empty_str, list_builder
        )
    } else {
        list_builder
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
            .map(|s| rewrite_stmt_store_accesses(s, ctx.store_provider_map, ctx.ir))
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
