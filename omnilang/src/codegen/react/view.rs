// ═══════════════════════════════════════════════════════════
// View tree generation (JSX)
// ═══════════════════════════════════════════════════════════

use std::collections::HashMap;

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::expr::{format_expr_ir_tsx, generate_statement_tsx};
use super::utils::{
    capitalize_first, emit_event_object_react, format_json_value_tsx, to_css_prop, to_jsx_attr,
    to_react_event,
};

pub(super) fn gen_view_node_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    component_map: &HashMap<String, ComponentIR>,
) {
    match node {
        ViewTree::Element(el) => {
            // Component composition: check if tag is a known component
            if let Some(_child_comp) = component_map.get(&el.tag) {
                let mut comp_attrs: Vec<String> = Vec::new();
                for (prop_name, prop_value) in &el.props {
                    let js_val = format_expr_ir_tsx(prop_value);
                    comp_attrs.push(format!("{}={{{}}}", prop_name, js_val));
                }
                for (event_name, handler) in &el.events {
                    let jsx_event = to_react_event(event_name);
                    match handler.kind.as_str() {
                        "fn_ref" => {
                            if let Some(ref fn_name) = handler.r#ref {
                                comp_attrs.push(format!("{}={{{}}}", jsx_event, fn_name));
                            }
                        }
                        "inline_fn" => {
                            if let Some(ref body) = handler.body {
                                let handler_str = format_inline_handler(body);
                                comp_attrs.push(format!("{}={{{}}}", jsx_event, handler_str));
                            }
                        }
                        "store_action" => {
                            if let Some(ref action) = handler.r#ref {
                                let action_fn = action.split('.').last().unwrap_or(action);
                                comp_attrs.push(format!(
                                    "{}={{() => {{ {}(); }}}}",
                                    jsx_event, action_fn
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                let attrs_str = comp_attrs.join(" ");
                if el.children.is_empty() {
                    w.emit(&format!("<{} {}/>", el.tag, attrs_str));
                } else {
                    w.emit(&format!("<{} {}>", el.tag, attrs_str));
                    w.indent();
                    for child in &el.children {
                        gen_view_node_tsx(w, child, component_map);
                    }
                    w.dedent();
                    w.emit(&format!("</{}>", el.tag));
                }
                return; // IMPORTANT: skip HTML element handling
            }
            // Otherwise, fall through to existing HTML element handling
            // Build attribute list from props, bindings, events
            let mut attrs: Vec<String> = Vec::new();

            // Static props
            for (prop_name, prop_value) in &el.props {
                let jsx_attr = to_jsx_attr(prop_name);
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value
                            .as_str()
                            .expect("literal string value confirmed by is_string()");
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                        attrs.push(format!("{}=\"{}\"", jsx_attr, escaped));
                    }
                    _ => {
                        let jsx_val = format_expr_ir_tsx(prop_value);
                        attrs.push(format!("{}={{{}}}", jsx_attr, jsx_val));
                    }
                }
            }

            // Two-way bindings: value={x} onChange={e => setX(e.target.value)}
            for (bind_name, signal_ref) in &el.bindings {
                let jsx_attr = to_jsx_attr(bind_name);
                let setter = format!("set{}", capitalize_first(&signal_ref.name));
                attrs.push(format!("{}={{{}}}", jsx_attr, signal_ref.name));
                match bind_name.as_str() {
                    "value" => {
                        attrs.push(format!(
                            "onChange={{(e) => {{ {}(e.target.value); }}}}",
                            setter
                        ));
                    }
                    "checked" => {
                        attrs.push(format!(
                            "onChange={{(e) => {{ {}(e.target.checked); }}}}",
                            setter
                        ));
                    }
                    _ => {}
                }
            }

            // Motion triggers — add data-motion-enter for @enter effects, inline handlers for interactive
            for trigger in &el.motion_triggers {
                match trigger.kind {
                    TriggerKind::Enter => {
                        attrs.push("data-motion-enter=\"true\"".into());
                    }
                    TriggerKind::Hover => {
                        attrs.push("onMouseEnter={(e) => { e.currentTarget.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 300, easing: 'ease-out' }); }}".into());
                        attrs.push("onMouseLeave={(e) => { e.currentTarget.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 300, easing: 'ease-out' }); }}".into());
                    }
                    TriggerKind::Press => {
                        attrs.push("onMouseDown={(e) => { e.currentTarget.animate([{ scale: 1 }, { scale: 0.95 }], { duration: 150, easing: 'ease-out', fill: 'both' }); }}".into());
                        attrs.push("onMouseUp={(e) => { e.currentTarget.animate([{ scale: 0.95 }, { scale: 1 }], { duration: 150, easing: 'ease-out', fill: 'both' }); }}".into());
                    }
                    TriggerKind::Focus => {
                        attrs.push("onFocus={(e) => { e.currentTarget.animate([{ boxShadow: '0 0 0 0 rgba(59,130,246,0)' }, { boxShadow: '0 0 0 3px rgba(59,130,246,0.5)' }], { duration: 200, easing: 'ease-out', fill: 'both' }); }}".into());
                        attrs.push("onBlur={(e) => { e.currentTarget.animate([{ boxShadow: '0 0 0 3px rgba(59,130,246,0.5)' }, { boxShadow: '0 0 0 0 rgba(59,130,246,0)' }], { duration: 200, easing: 'ease-out', fill: 'both' }); }}".into());
                    }
                    TriggerKind::InView => {
                        let threshold_val = match &el.in_view_threshold {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        attrs.push("data-motion-inview=\"true\"".into());
                        attrs.push(format!("data-inview-threshold=\"{}\"", threshold_val));
                    }
                    TriggerKind::Scroll => {
                        let scroll_start_str = el.scroll_start.as_deref().unwrap_or("top 80%");
                        let scroll_end_str = el.scroll_end.as_deref().unwrap_or("bottom 20%");
                        let scrub_val = match &el.scroll_scrub {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        attrs.push("data-motion-scroll=\"true\"".into());
                        attrs.push(format!("data-scroll-start=\"{}\"", scroll_start_str));
                        attrs.push(format!("data-scroll-end=\"{}\"", scroll_end_str));
                        attrs.push(format!("data-scroll-scrub=\"{}\"", scrub_val));
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        let axis_str = if is_horizontal { "x" } else { "y" };
                        let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                        let elastic_val = match &el.drag_elastic {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        let threshold_val = match &el.drag_threshold {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "0".to_string(),
                        };
                        let velocity_max_val = match &el.drag_velocity_max {
                            Some(expr) => format_expr_ir_tsx(expr),
                            None => "2000".to_string(),
                        };
                        attrs.push(format!("data-motion-drag=\"{}\"", axis_str));
                        attrs.push(format!("data-drag-constraints=\"{}\"", constraints_str));
                        attrs.push(format!("data-drag-elastic=\"{}\"", elastic_val));
                        attrs.push(format!("data-drag-threshold=\"{}\"", threshold_val));
                        attrs.push(format!("data-drag-velocity-max=\"{}\"", velocity_max_val));
                    }
                    _ => {}
                }
            }

            // @driven-by — external progress control marker attribute
            if let Some(ref driven_expr) = el.driven_by {
                let driven_js = format_expr_ir_tsx(driven_expr);
                attrs.push(format!("data-driven-by={{{}}}", driven_js));
            }

            // Events
            for (event_name, handler) in &el.events {
                let react_event = to_react_event(event_name);
                match handler.kind.as_str() {
                    "fn_ref" => {
                        if let Some(ref fn_name) = handler.r#ref {
                            if handler.event_data.is_some() {
                                let event_obj = emit_event_object_react(
                                    &handler.event_data,
                                    &handler.used_fields,
                                    "e",
                                );
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}({}) }}}}",
                                    react_event, fn_name, event_obj
                                ));
                            } else {
                                attrs.push(format!("{}={{{}}}", react_event, fn_name));
                            }
                        }
                    }
                    "inline_fn" => {
                        if let Some(ref body) = handler.body {
                            let handler_str = format_inline_handler(body);
                            attrs.push(format!("{}={{{}}}", react_event, handler_str));
                        }
                    }
                    "store_action" => {
                        if let Some(ref action) = handler.r#ref {
                            let action_fn = action.split('.').last().unwrap_or(action);
                            attrs.push(format!(
                                "{react_event}={{() => {{ {action_fn}(); }}}}",
                                react_event = react_event,
                                action_fn = action_fn
                            ));
                        }
                    }
                    _ => {
                        attrs.push(format!(
                            "{}={{/* TODO: handler kind '{}' */}}",
                            react_event, handler.kind
                        ));
                    }
                }
            }

            // Style classes from view
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    attrs.push(format!("className={{styles['{}']}}", classes_str));
                }
            }

            // Inline style
            if let Some(ref style_json) = el.inline_style {
                if let Some(obj) = style_json.as_object() {
                    let mut pairs: Vec<String> = Vec::new();
                    for (prop, val) in obj {
                        let js_prop = to_css_prop(prop);
                        if let Some(s) = val.as_str() {
                            if s.starts_with("$signal:") {
                                let sig_name = &s[8..];
                                pairs.push(format!("{}: {{{}}}", js_prop, sig_name));
                            } else if s.starts_with("$call:") {
                                let fn_call = &s[6..];
                                pairs.push(format!("{}: {{{}}}", js_prop, fn_call));
                            } else {
                                pairs.push(format!("{}: '{}'", js_prop, s));
                            }
                        } else {
                            pairs.push(format!("{}: {{{}}}", js_prop, format_json_value_tsx(val)));
                        }
                    }
                    if !pairs.is_empty() {
                        attrs.push(format!("style={{{{ {} }}}}", pairs.join(", ")));
                    }
                }
            }

            // Dynamic class bindings: class:active={isOpen} -> className={`base ${isOpen ? 'active' : ''}`}
            if !el.class_map_bindings.is_empty() {
                let dynamic_parts: Vec<String> = el
                    .class_map_bindings
                    .iter()
                    .map(|(cls, expr)| {
                        let cond = format_expr_ir_tsx(expr);
                        format!("${{{cond} ? '{cls}' : ''}}")
                    })
                    .collect();
                if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
                    let old = std::mem::take(&mut attrs[pos]);
                    attrs[pos] = old.replace("}}", &format!(" {}", dynamic_parts.join(" ")));
                } else {
                    attrs.push(format!("className={{`{}`}}", dynamic_parts.join(" ")));
                }
            }

            // Emit opening tag
            if attrs.is_empty() && el.children.is_empty() {
                // Self-closing tag
                w.emit(&format!("<{} />", el.tag));
                return;
            }

            if attrs.is_empty() {
                w.emit(&format!("<{}>", el.tag));
            } else {
                w.emit(&format!("<{}", el.tag));
                w.indent();
                for attr in &attrs {
                    w.emit(&attr.to_string());
                }
                if el.children.is_empty() {
                    w.dedent();
                    w.emit("/>");
                    return;
                }
                w.dedent();
                w.emit(">");
            }

            // Children
            w.indent();
            for child in &el.children {
                gen_view_node_tsx(w, child, component_map);
            }
            w.dedent();

            // Closing tag
            w.emit(&format!("</{}>", el.tag));
        }
        ViewTree::Conditional(cond) => {
            let branches = &cond.branches;
            let fallback = &cond.fallback;

            if branches.is_empty() {
                if let Some(ref fb) = fallback {
                    gen_view_node_tsx(w, fb, component_map);
                }
                return;
            }

            // Single branch without fallback → {condition && (...)}
            if branches.len() == 1 && fallback.is_none() {
                w.emit("{");
                w.indent();
                let cond_str = format_expr_ir_tsx(&branches[0].condition);
                w.emit(&format!("{} && (", cond_str));
                w.indent();
                if let Some(ref body) = branches[0].body {
                    gen_view_node_tsx(w, body, component_map);
                }
                w.dedent();
                w.emit(")");
                w.dedent();
                w.emit("}");
                return;
            }

            // Multiple branches or fallback → nested ternary
            w.emit("{");
            w.indent();
            for (i, branch) in branches.iter().enumerate() {
                if i > 0 {
                    w.emit(": (");
                    w.indent();
                }
                let cond_str = format_expr_ir_tsx(&branch.condition);
                w.emit(&format!("{} ? (", cond_str));
                w.indent();
                if let Some(ref body) = branch.body {
                    gen_view_node_tsx(w, body, component_map);
                }
                w.dedent();
            }
            // Fallback or null
            if let Some(ref fb) = fallback {
                w.emit(") : (");
                w.indent();
                gen_view_node_tsx(w, fb, component_map);
                w.dedent();
                w.emit(")");
                // Close all open parenthesis from chained ternary
                for _ in 0..branches.len() {
                    w.emit(")");
                }
                w.dedent();
                w.emit("}");
            } else {
                w.emit(") : null");
                // Close all open parenthesis
                for _ in 1..branches.len() {
                    w.emit(")");
                }
                // Close the open indent for branch 0
                w.dedent();
                w.emit("}");
            }
        }
        ViewTree::ListRendering(list) => {
            let iterable_str = format_expr_ir_tsx(&list.iterable);
            let item_name = &list.item_binding.name;
            let key_expr = if let Some(ref ke) = list.key_expr {
                format!("key={{{}}}", format_expr_ir_tsx(ke))
            } else {
                "key={index}".to_string()
            };
            if list.empty_body.is_some() {
                w.emit("{");
                w.indent();
                w.emit(&format!(
                    "({} && {}.length === 0) ? (",
                    iterable_str, iterable_str
                ));
                w.indent();
                if let Some(ref empty_body) = list.empty_body {
                    gen_view_node_tsx(w, empty_body, component_map);
                }
                w.dedent();
                w.emit(&format!(
                    ") : ({}.map(({}, index) => (",
                    iterable_str, item_name
                ));
            } else {
                w.emit("{");
                w.indent();
                w.emit(&format!("{}.map(({}, index) => (", iterable_str, item_name));
            }
            w.indent();
            if let Some(ref body) = list.body {
                gen_view_node_tsx(w, body, component_map);
            }
            w.dedent();
            w.emit(&format!(", {})))", key_expr));
            w.dedent();
            if list.empty_body.is_some() {
                w.emit(")}");
            } else {
                w.emit("}");
            }
        }
        ViewTree::Text(text) => {
            if text.starts_with('{') && text.ends_with('}') {
                // Interpolation expression like {name} or {param:xxx}
                let inner = &text[1..text.len() - 1];
                w.emit(&format!("{{{}}}", inner));
            } else {
                // Plain text — escape JSX-significant characters
                let escaped = text
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('&', "&amp;");
                w.emit(&format!("{{'{}'}}", escaped));
            }
        }
        ViewTree::Interpolation(expr) => {
            let js_expr = format_expr_ir_tsx(expr);
            w.emit(&format!("{{{}}}", js_expr));
        }
        ViewTree::Slot(_) => {
            // slots not supported in React — emit placeholder
        }
    }
}

/// Build an inline arrow function string from a BlockIR.
fn format_inline_handler(body: &BlockIR) -> String {
    if body.statements.is_empty() {
        return "(e) => {}".to_string();
    }
    let mut hw = CodeWriter::new();
    for stmt in &body.statements {
        generate_statement_tsx(&mut hw, stmt);
    }
    let content = hw.into_string();
    // Build a multi-line arrow function
    let mut result = String::from("(e) => {\n");
    for line in content.lines() {
        result.push_str(&format!("        {}\n", line));
    }
    result.push_str("      }");
    result
}
