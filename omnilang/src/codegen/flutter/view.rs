//! View tree formatting for Flutter codegen.

use crate::codegen::shared::expr::DartFormatExpr;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;

use super::capitalize_first;
use super::expr::format_expr_ir_flutter;
use super::motion::{wrap_flutter_states, wrap_with_motion_widgets};
use super::motion_widgets::{emit_event_object_flutter, field_extract_flutter};
use super::stmt::{generate_statement_flutter, FlutterStmtFormatter};
use super::utils::{
    binding_to_flutter_param, event_to_flutter_callback, prop_to_flutter_param_name,
    tag_to_flutter_widget,
};

// ═══════════════════════════════════════════════════════════════
// View tree formatting (single-line Flutter widget expressions)
// ═══════════════════════════════════════════════════════════════

/// Render a ViewTree as a single-line Flutter widget expression string.
pub(crate) fn format_view_tree(
    node: &ViewTree,
    motions: &[MotionIntent],
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> String {
    match node {
        ViewTree::Element(el) => format_element(el, motions, component_map),
        ViewTree::Text(text) => {
            if text.starts_with('{') && text.ends_with('}') {
                // OmniView interpolation: {name} -> Text('$name')
                let inner = &text[1..text.len() - 1];
                if inner.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    format!("Text('${}')", inner)
                } else {
                    format!("Text('${{{}}}')", inner)
                }
            } else {
                format!("Text('{}')", text.replace('\'', "\\'"))
            }
        }
        ViewTree::Interpolation(expr) => {
            let dart_expr = format_expr_ir_flutter(expr);
            format!("Text({})", dart_expr)
        }
        ViewTree::Conditional(cond) => format_conditional(cond, motions, component_map),
        ViewTree::ListRendering(list) => format_list_rendering(list, motions, component_map),
        ViewTree::Slot(_) => "SizedBox.shrink()".to_string(),
    }
}

/// Format an ElementIR node as a Flutter widget constructor expression.
/// Wraps with motion animation widgets (AnimatedBuilder, MouseRegion, GestureDetector)
/// when the element has motion triggers.
fn format_element(
    el: &ElementIR,
    motions: &[MotionIntent],
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> String {
    // Component composition: check if tag is a known component
    if let Some(_child_comp) = component_map.get(el.tag.as_str()) {
        let mut child_args: Vec<String> = Vec::new();
        // Props become constructor parameters
        for (prop_name, prop_value) in &el.props {
            child_args.push(format!(
                "{}: {}",
                prop_name,
                format_expr_ir_flutter(prop_value)
            ));
        }
        // Events become callback parameters
        for (event_name, handler) in &el.events {
            let cb = match handler {
                EventHandlerIR::FnRef { r#ref, .. } => {
                    format!("() {{ {}(); }}", r#ref)
                }
                EventHandlerIR::InlineFn { body, .. } => format_inline_handler_flutter(body),
                EventHandlerIR::StoreAction { r#ref, .. } => {
                    format_store_action_handler(r#ref)
                }
            };
            let stem = event_name.strip_prefix("on").unwrap_or(event_name);
            child_args.push(format!("on{}: {}", capitalize_first(stem), cb));
        }
        // Return component constructor with children
        let args_str = child_args.join(", ");
        if el.children.is_empty() {
            return format!("{}({})", el.tag, args_str);
        } else {
            let children_str: Vec<String> = el
                .children
                .iter()
                .map(|c| format_view_tree(c, motions, component_map))
                .collect();
            return format!(
                "{}({}, children: [{}])",
                el.tag,
                args_str,
                children_str.join(", ")
            );
        }
    }
    // Otherwise, fall through to HTML tag mapping
    let widget = tag_to_flutter_widget(&el.tag);
    let mut args: Vec<String> = Vec::new();

    // Props -> widget constructor parameters
    for (name, value) in &el.props {
        let formatted = format_expr_ir_flutter(value);
        match name.as_str() {
            "style" => {
                // Map common CSS properties to Flutter widget parameters
                if let ExprIR::LiteralExpr {
                    value: serde_json::Value::Object(ref style_obj),
                } = value
                {
                    for (css_prop, css_val) in style_obj {
                        match css_prop.as_str() {
                            "padding" => {
                                if let Some(v) = css_val.as_str() {
                                    args.push(format!("padding: EdgeInsets.all({})", v));
                                }
                            }
                            "margin" => {
                                if let Some(v) = css_val.as_str() {
                                    args.push(format!("margin: EdgeInsets.all({})", v));
                                }
                            }
                            "color" => {
                                if let Some(v) = css_val.as_str() {
                                    args.push(format!("color: {}", v));
                                }
                            }
                            "background-color" => {
                                if let Some(v) = css_val.as_str() {
                                    args.push(format!("color: {}", v));
                                }
                            }
                            _ => {} // skip unknown CSS properties
                        }
                    }
                }
            }
            other => {
                let param = prop_to_flutter_param_name(other);
                if !param.is_empty() {
                    args.push(format!("{}: {}", param, formatted));
                }
            }
        }
    }

    // Events -> callback parameters
    for (name, handler) in &el.events {
        let callback_name = event_to_flutter_callback(name);
        let body = match handler {
            EventHandlerIR::FnRef {
                r#ref, event_data, ..
            } => {
                if event_data.is_some() {
                    let event_obj = emit_event_object_flutter(
                        handler.event_data(),
                        handler.used_fields(),
                        "event",
                    );
                    format!("(event) {{ {}({}); }}", r#ref, event_obj)
                } else {
                    format!("() {{ {}(); }}", r#ref)
                }
            }
            EventHandlerIR::InlineFn {
                body, event_data, ..
            } => {
                if event_data.is_some() {
                    let _event_obj =
                        emit_event_object_flutter(handler.event_data(), handler.used_fields(), "e");
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
                        );
                        hw.emit_fmt(format_args!("final {} = {};", field, extract));
                    }
                    for stmt in &body.statements {
                        // Use custom formatter with event_param_name set
                        // so FieldAccess on "e" emits bare field name (local final binding)
                        let fmt = FlutterStmtFormatter {
                            expr: DartFormatExpr {
                                event_param_name: Some("e".to_string()),
                            },
                            in_store_action: false,
                            store_prefix: None,
                        };
                        fmt.format_stmt(&mut hw, stmt);
                    }
                    hw.dedent();
                    hw.emit("}");
                    let result = hw.into_string().trim().to_string();
                    result
                } else {
                    format_inline_handler_flutter(body)
                }
            }
            EventHandlerIR::StoreAction { r#ref, .. } => {
                format_store_action_handler(r#ref)
            }
        };
        args.push(format!("{}: {}", callback_name, body));
    }

    // Bindings
    for (bind_name, signal_ref) in &el.bindings {
        let param = binding_to_flutter_param(bind_name);
        if !param.is_empty() {
            args.push(format!("{}: {}", param, signal_ref.name));
        }
    }

    // Children
    if el.children.is_empty() {
        // No children
    } else if el.children.len() == 1 {
        let child_str = format_view_tree(&el.children[0], motions, component_map);
        args.push(format!("child: {}", child_str));
    } else {
        let children: Vec<String> = el
            .children
            .iter()
            .map(|c| format_view_tree(c, motions, component_map))
            .collect();
        args.push(format!("children: [{}]", children.join(", ")));
    }

    let inner = if args.is_empty() {
        format!("{}()", widget)
    } else {
        format!("{}({})", widget, args.join(", "))
    };

    // Wrap with motion animation widgets if the element has triggers
    let has_inview = el
        .motion_behavior()
        .and_then(|m| m.in_view_once.as_ref().or(m.in_view_repeat.as_ref()))
        .is_some();
    let has_scroll = el.scroll_behavior().is_some();
    let mut result = if el.motion_behavior().is_some_and(|m| !m.triggers.is_empty())
        || el
            .motion_behavior()
            .and_then(|m| m.driven_by.as_ref())
            .is_some()
        || el
            .motion_behavior()
            .and_then(|m| m.states.as_ref())
            .is_some()
        || has_inview
        || has_scroll
    {
        let mut result = inner;

        // Handle @driven-by: wrap with signal-driven animation progress
        if let Some(driven_expr) = el.motion_behavior().and_then(|m| m.driven_by.as_ref()) {
            let driven_dart = format_expr_ir_flutter(driven_expr);
            // Generate a listener that seeks the animation controller based on signal value
            result = format!(
                "DrivenByBuilder(\n  progress: () => ({}).toDouble().clamp(0.0, 1.0),\n  builder: (context, child, progress) {{\n    // Apply animation at the given progress\n    return child!;\n  }},\n  child: {},\n)",
                driven_dart, result
            );
        }

        // Handle states:: wrap with gesture-detector or signal-driven state machine
        if let Some(states_ir) = el.motion_behavior().and_then(|m| m.states.as_ref()) {
            result = wrap_flutter_states(result, states_ir, el);
        }

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
                Some(expr) => format_expr_ir_flutter(expr),
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
                result = format!(
                    "_OmniInViewWidget(\n  once: {},\n  threshold: {},\n  onInView: () {{ {}.forward(); }},\n  child: {},\n)",
                    once, threshold, ctrl_name, result
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
                Some(expr) => format_expr_ir_flutter(expr),
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
                result = format!(
                    "_OmniScrollDrivenWidget(\n  start: '{}',\n  end: '{}',\n  scrub: {},\n  onProgress: (p) {{ {}.value = p; }},\n  child: {},\n)",
                    scroll_start, scroll_end, scrub, ctrl_name, result
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
                            let constraints_str = el
                                .drag_behavior()
                                .and_then(|d| d.constraints.as_deref())
                                .unwrap_or("");
                            let elastic_val =
                                match el.drag_behavior().and_then(|d| d.elastic.as_ref()) {
                                    Some(expr) => format_expr_ir_flutter(expr),
                                    None => "0.0".to_string(),
                                };
                            let threshold_val =
                                match el.drag_behavior().and_then(|d| d.threshold.as_ref()) {
                                    Some(expr) => format_expr_ir_flutter(expr),
                                    None => "0.0".to_string(),
                                };
                            result = format!(
                                "_OmniDragWidget(\n  axis: {},\n  constraints: '{}',\n  elastic: {},\n  threshold: {},\n  onDragStart: () {{ }},\n  onDragProgress: (p) {{ {}.value = p; }},\n  onDragEnd: (completed) {{ if (completed) {{ {}.forward(); }} else {{ {}.reverse(); }} }},\n  child: {},\n)",
                                axis_type, constraints_str, elastic_val, threshold_val, ctrl_name, ctrl_name, ctrl_name, result
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

    // Wrap with Hero widget if hero_tag is set (shared element transition)
    if let Some(hero_tag) = el.hero_behavior().and_then(|h| h.tag.as_ref()) {
        let tag_dart = format!("'{}'", hero_tag);
        if let Some(transition) = el.hero_behavior().and_then(|h| h.transition.as_ref()) {
            let trans_dart = format_expr_ir_flutter(transition);
            result = format!(
                "Hero(\n  tag: {},\n  flightShuttleBuilder: ({}, child, animation, flightDirection, fromHeroContext, toHeroContext) {{\n    return child!;\n  }},\n  child: {},\n)",
                tag_dart, trans_dart, result
            );
        } else {
            result = format!("Hero(\n  tag: {},\n  child: {},\n)", tag_dart, result);
        }
    }

    result
}

/// Format a ConditionalIntent as a Dart ternary expression wrapped in Builder
/// so that setState rebuilds re-evaluate conditions reactively.
fn format_conditional(
    cond: &ConditionalIntent,
    motions: &[MotionIntent],
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> String {
    if cond.branches.is_empty() {
        if let Some(ref fb) = cond.fallback {
            return format_view_tree(fb, motions, component_map);
        }
        return "SizedBox()".to_string();
    }

    let mut result = String::new();
    for (i, branch) in cond.branches.iter().enumerate() {
        let cond_str = format_expr_ir_flutter(&branch.condition);
        let body_str = branch
            .body
            .as_ref()
            .map(|b| format_view_tree(b, motions, component_map))
            .unwrap_or_else(|| "SizedBox()".to_string());
        if i == 0 {
            result.push_str(&format!("{} ? {}", cond_str, body_str));
        } else {
            result.push_str(&format!(" : {} ? {}", cond_str, body_str));
        }
    }

    // Fallback
    if let Some(ref fb) = cond.fallback {
        result.push_str(&format!(
            " : {}",
            format_view_tree(fb, motions, component_map)
        ));
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
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> String {
    let iterable_str = format_expr_ir_flutter(&list.iterable);
    let item_name = &list.item_binding.name;
    let body_str = list
        .body
        .as_ref()
        .map(|b| format_view_tree(b, motions, component_map))
        .unwrap_or_else(|| "SizedBox()".to_string());
    let list_builder = format!(
        "ListView.builder(\n  itemCount: {iter}.length,\n  itemBuilder: (context, index) {{\n    var {item} = {iter}[index];\n    return {body};\n  }},\n)",
        iter = iterable_str, item = item_name, body = body_str
    );
    if list.empty_body.is_some() {
        let empty_str = list
            .empty_body
            .as_ref()
            .map(|b| format_view_tree(b, motions, component_map))
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
fn format_inline_handler_flutter(body: &BlockIR) -> String {
    let config = InlineHandlerConfig {
        opener: "() {".into(),
        closer: "}".into(),
        empty_handler: "() {}".into(),
        indent: "    ",
        use_writer: true,
    };
    build_inline_handler(body, &config, generate_statement_flutter)
}

/// Format a StoreAction event handler that calls a store action function with ref.
///
/// The `r#ref` format is `"StoreName.actionName"`, which is transformed into
/// `"() {{ storeActionName(ref); }}"` where the function name is computed as
/// `{lowercase_first(store_name)}{capitalize_first(action_name)}`.
fn format_store_action_handler(r#ref: &str) -> String {
    let action_fn = r#ref.split('.').next_back().unwrap_or(r#ref);
    let parts: Vec<&str> = r#ref.split('.').collect();
    let store_lower = if parts.len() >= 2 {
        let s = parts[0];
        let mut c = s.chars();
        match c.next() {
            None => s.to_string(),
            Some(first) => first.to_lowercase().to_string() + c.as_str(),
        }
    } else {
        action_fn.to_string()
    };
    let full_action = format!("{}{}", store_lower, capitalize_first(action_fn));
    format!("() {{ {}(ref); }}", full_action)
}
