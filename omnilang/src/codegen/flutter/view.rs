//! View tree formatting for Flutter codegen.

use std::collections::HashMap;

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::capitalize_first;
use super::expr::format_expr_ir_flutter;
use super::motion::{wrap_flutter_states, wrap_with_motion_widgets};
use super::motion_widgets::emit_event_object_flutter;
use super::stmt::generate_statement_flutter;
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
    component_map: &HashMap<String, ComponentIR>,
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
    component_map: &HashMap<String, ComponentIR>,
) -> String {
    // Component composition: check if tag is a known component
    if let Some(_child_comp) = component_map.get(&el.tag) {
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
            let cb = match handler.kind.as_str() {
                "fn_ref" => {
                    if let Some(ref fn_name) = handler.r#ref {
                        format!("() {{ {}(); }}", fn_name)
                    } else {
                        "() {}".to_string()
                    }
                }
                "inline_fn" => {
                    if let Some(ref body) = handler.body {
                        format_inline_handler_flutter(body)
                    } else {
                        "() {}".to_string()
                    }
                }
                "store_action" => {
                    if let Some(ref action) = handler.r#ref {
                        let action_fn = action.split('.').next_back().unwrap_or(action);
                        format!("() {{ {}(); }}", action_fn)
                    } else {
                        "() {}".to_string()
                    }
                }
                _ => "() {}".to_string(),
            };
            child_args.push(format!("on{}: {}", capitalize_first(event_name), cb));
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
        let body = match handler.kind.as_str() {
            "fn_ref" => {
                if let Some(ref fn_name) = handler.r#ref {
                    if handler.event_data.is_some() {
                        let event_obj = emit_event_object_flutter(
                            &handler.event_data,
                            &handler.used_fields,
                            "event",
                        );
                        format!("(event) {{ {}({}); }}", fn_name, event_obj)
                    } else {
                        format!("() {{ {}(); }}", fn_name)
                    }
                } else {
                    "() {}".to_string()
                }
            }
            "inline_fn" => {
                if let Some(ref body) = handler.body {
                    if handler.event_data.is_some() {
                        let _event_obj = emit_event_object_flutter(
                            &handler.event_data,
                            &handler.used_fields,
                            "e",
                        );
                        let mut hw = CodeWriter::new();
                        hw.emit("(e) {");
                        hw.indent();
                        for stmt in &body.statements {
                            generate_statement_flutter(&mut hw, stmt);
                        }
                        hw.dedent();
                        hw.emit("}");
                        let result = hw.into_string().trim().to_string();
                        // Replace the extracted event object usage: the body references
                        // e via the parameter, which already has the extracted fields
                        result
                    } else {
                        format_inline_handler_flutter(body)
                    }
                } else {
                    "() {}".to_string()
                }
            }
            "store_action" => {
                if let Some(ref action) = handler.r#ref {
                    let action_fn = action.split('.').next_back().unwrap_or(action);
                    format!("() {{ {}(); }}", action_fn)
                } else {
                    "() {}".to_string()
                }
            }
            other => format!("() {{ /* TODO: kind '{}' */ }}", other),
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
    let has_inview = el.in_view_once.is_some() || el.in_view_repeat.is_some();
    let has_scroll = el.scroll_start.is_some() || el.scroll_end.is_some();
    let mut result = if !el.motion_triggers.is_empty()
        || el.driven_by.is_some()
        || el.states.is_some()
        || has_inview
        || has_scroll
    {
        let mut result = inner;

        // Handle @driven-by: wrap with signal-driven animation progress
        if let Some(ref driven_expr) = el.driven_by {
            let driven_dart = format_expr_ir_flutter(driven_expr);
            // Generate a listener that seeks the animation controller based on signal value
            result = format!(
                "DrivenByBuilder(\n  progress: () => ({}).toDouble().clamp(0.0, 1.0),\n  builder: (context, child, progress) {{\n    // Apply animation at the given progress\n    return child!;\n  }},\n  child: {},\n)",
                driven_dart, result
            );
        }

        // Handle states:: wrap with gesture-detector or signal-driven state machine
        if let Some(ref states_ir) = el.states {
            result = wrap_flutter_states(result, states_ir, el);
        }

        // Standard motion triggers (enter, hover, press, etc.)
        if !el.motion_triggers.is_empty() {
            result = wrap_with_motion_widgets(result, &el.motion_triggers, motions);
        }

        // Handle @in-view trigger: wrap with _OmniInViewWidget
        if has_inview {
            let threshold = match &el.in_view_threshold {
                Some(expr) => format_expr_ir_flutter(expr),
                None => "0.0".to_string(),
            };
            let once = if el.in_view_once.is_some() {
                "true"
            } else {
                "false"
            };
            let motion_ref = el
                .motion_triggers
                .iter()
                .find(|t| matches!(t.kind, TriggerKind::InView))
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
            let scroll_start = el.scroll_start.as_deref().unwrap_or("top 80%");
            let scroll_end = el.scroll_end.as_deref().unwrap_or("bottom 20%");
            let scrub = match &el.scroll_scrub {
                Some(expr) => format_expr_ir_flutter(expr),
                None => "0.0".to_string(),
            };
            let motion_ref = el
                .motion_triggers
                .iter()
                .find(|t| matches!(t.kind, TriggerKind::Scroll))
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
        let has_drag = el.motion_triggers.iter().any(|t| {
            matches!(
                t.kind,
                TriggerKind::DragHorizontal | TriggerKind::DragVertical
            )
        });
        if has_drag {
            for trigger in &el.motion_triggers {
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
                            let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                            let elastic_val = match &el.drag_elastic {
                                Some(expr) => format_expr_ir_flutter(expr),
                                None => "0.0".to_string(),
                            };
                            let threshold_val = match &el.drag_threshold {
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
    if let Some(ref hero_tag) = el.hero_tag {
        let tag_dart = format!("'{}'", hero_tag);
        if let Some(ref transition) = el.hero_transition {
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
    component_map: &HashMap<String, ComponentIR>,
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
    component_map: &HashMap<String, ComponentIR>,
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
    if body.statements.is_empty() {
        return "() {}".to_string();
    }
    let mut hw = CodeWriter::new();
    hw.emit("() {");
    hw.indent();
    for stmt in &body.statements {
        generate_statement_flutter(&mut hw, stmt);
    }
    hw.dedent();
    hw.emit("}");
    hw.into_string().trim().to_string()
}
