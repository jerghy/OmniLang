//! View tree generation for Dioxus RSX.

use super::{emit_event_object_dioxus, format_expr_ir_dioxus, to_dioxus_attr, to_dioxus_event};
use super::{generate_statement_dioxus, CodeWriter};
use crate::ir::*;

/// Generate a Dioxus RSX view node.
///
/// `is_root` controls trailing-comma behavior: root-level nodes
/// inside `rsx! {{ ... }}` should NOT have a trailing comma (it causes
/// a syntax error), while nested nodes DO need commas between siblings.
pub(super) fn gen_view_node_dioxus(w: &mut CodeWriter, node: &ViewTree, is_root: bool) {
    match node {
        ViewTree::Element(el) => {
            let mut attrs: Vec<String> = Vec::new();

            // Static props
            for (prop_name, prop_value) in &el.props {
                let attr_name = to_dioxus_attr(prop_name);
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value
                            .as_str()
                            .unwrap_or_default(); // defense-in-depth fallback
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                        attrs.push(format!(r#"{}: "{}","#, attr_name, escaped));
                    }
                    _ => {
                        let val = format_expr_ir_dioxus(prop_value);
                        attrs.push(format!("{}: {},", attr_name, val));
                    }
                }
            }

            // Signal bindings (two-way)
            for (bind_name, signal_ref) in &el.bindings {
                let attr_name = to_dioxus_attr(bind_name);
                attrs.push(format!(r#"{}: "{{{}}}","#, attr_name, signal_ref.name));
                attrs.push(format!(
                    "oninput: move |e| {}.set(e.value()),",
                    signal_ref.name
                ));
            }

            // Events
            for (event_name, handler) in &el.events {
                let dioxus_event = to_dioxus_event(event_name);
                match handler.kind {
                    EventHandlerKind::FnRef => {
                        if let Some(ref fn_name) = handler.r#ref {
                            if handler.event_data.is_some() {
                                let event_obj = emit_event_object_dioxus(
                                    &handler.event_data,
                                    &handler.used_fields,
                                    "e",
                                );
                                attrs.push(format!(
                                    "{}: move |e| {{ {}({}); }},",
                                    dioxus_event, fn_name, event_obj
                                ));
                            } else {
                                attrs.push(format!("{}: {},", dioxus_event, fn_name));
                            }
                        }
                    }
                    EventHandlerKind::InlineFn => {
                        if let Some(ref body) = handler.body {
                            if handler.event_data.is_some() {
                                let _event_obj = emit_event_object_dioxus(
                                    &handler.event_data,
                                    &handler.used_fields,
                                    "e",
                                );
                                let mut hw = CodeWriter::new();
                                for stmt in &body.statements {
                                    generate_statement_dioxus(&mut hw, stmt);
                                }
                                let content = hw.into_string();
                                let mut result = String::from("move |e| {\n");
                                for line in content.lines() {
                                    result.push_str(&format!("                {}\n", line));
                                }
                                result.push_str("            }");
                                attrs.push(format!("{}: {},", dioxus_event, result));
                            } else {
                                attrs.push(format!(
                                    "{}: {},",
                                    dioxus_event,
                                    format_inline_handler_dioxus(body)
                                ));
                            }
                        }
                    }
                    EventHandlerKind::StoreAction => {
                        if let Some(ref action) = handler.r#ref {
                            attrs.push(format!(
                                "{}: move |_| {{ {}(); }},",
                                dioxus_event, action
                            ));
                        }
                    }
                }
            }

            // Style classes
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    attrs.push(format!("class: \"{}\",", classes_str));
                }
            }

            // Emit element
            if attrs.is_empty() && el.children.is_empty() {
                // Self-closing empty element
                let suffix = if is_root { "" } else { "," };
                w.emit_fmt(format_args!("{} {{}}{}", el.tag, suffix));
                return;
            }

            let suffix = if is_root { "" } else { "," };
            w.emit_fmt(format_args!("{} {{", el.tag));
            w.indent();
            for attr in &attrs {
                w.emit(attr);
            }
            for child in &el.children {
                gen_view_node_dioxus(w, child, false);
            }
            w.dedent();
            w.emit_fmt(format_args!("}}{}", suffix));
        }
        ViewTree::Conditional(cond) => {
            if cond.branches.is_empty() {
                if let Some(ref fb) = cond.fallback {
                    gen_view_node_dioxus(w, fb, is_root);
                }
                return;
            }

            // First branch
            let cond_str = format_expr_ir_dioxus(&cond.branches[0].condition);
            w.emit_fmt(format_args!("if {} {{", cond_str));
            w.indent();
            w.emit("rsx! {");
            w.indent();
            if let Some(ref body) = cond.branches[0].body {
                gen_view_node_dioxus(w, body, true);
            }
            w.dedent();
            w.emit("},");
            w.dedent();

            // Else-if branches
            for branch in cond.branches[1..].iter() {
                let else_if_cond = format_expr_ir_dioxus(&branch.condition);
                w.emit_fmt(format_args!("}} else if {} {{", else_if_cond));
                w.indent();
                w.emit("rsx! {");
                w.indent();
                if let Some(ref body) = branch.body {
                    gen_view_node_dioxus(w, body, true);
                }
                w.dedent();
                w.emit("},");
                w.dedent();
            }

            // Fallback
            if let Some(ref fb) = cond.fallback {
                w.emit("} else {");
                w.indent();
                w.emit("rsx! {");
                w.indent();
                gen_view_node_dioxus(w, fb, true);
                w.dedent();
                w.emit("},");
                w.dedent();
            }

            w.emit("}");
        }
        ViewTree::ListRendering(list) => {
            let iterable_str = format_expr_ir_dioxus(&list.iterable);
            let item_name = &list.item_binding.name;
            w.emit_fmt(format_args!(
                "{}.iter().map(|{}| {{",
                iterable_str, item_name
            ));
            w.indent();
            w.emit("rsx! {");
            w.indent();
            if let Some(ref body) = list.body {
                gen_view_node_dioxus(w, body, true);
            }
            w.dedent();
            w.emit("},");
            w.dedent();
            w.emit("})");
        }
        ViewTree::Text(text) => {
            let comma = if is_root { "" } else { "," };
            if text.starts_with('{') && text.ends_with('}') {
                // Interpolation: {name} -> "{name}"
                let inner = &text[1..text.len() - 1];
                w.emit_fmt(format_args!("\"{{{}}}\"{}", inner, comma));
            } else {
                // Plain text
                w.emit_fmt(format_args!("\"{}\"{}", text, comma));
            }
        }
        ViewTree::Interpolation(expr) => {
            let comma = if is_root { "" } else { "," };
            let formatted = format_expr_ir_dioxus(expr);
            w.emit_fmt(format_args!("\"{formatted}\"{comma}"));
        }
        ViewTree::Slot(_) => {
            // slots not supported in Dioxus — emit placeholder
        }
    }
}

/// Build an inline Rust closure string from a BlockIR for Dioxus event handlers.
pub(super) fn format_inline_handler_dioxus(body: &BlockIR) -> String {
    if body.statements.is_empty() {
        return "move |_| {}".to_string();
    }
    let mut hw = CodeWriter::new();
    for stmt in &body.statements {
        generate_statement_dioxus(&mut hw, stmt);
    }
    let content = hw.into_string();
    let mut result = String::from("move |_| {\n");
    for line in content.lines() {
        result.push_str(&format!("                {}\n", line));
    }
    result.push_str("            }");
    result
}
