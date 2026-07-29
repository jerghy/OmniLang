//! View tree generation for Dioxus RSX.

use super::{
    emit_event_object_dioxus, field_extract_dioxus, format_expr_ir_dioxus, to_dioxus_attr,
    to_dioxus_event,
};
use super::{generate_statement_dioxus, CodeWriter, DioxusStmtFormatter};
use crate::codegen::shared::expr::RustFormatExpr;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::stmt::FormatStmt;
use crate::ir::*;

/// Generate a Dioxus RSX view node.
///
/// `is_root` controls trailing-comma behavior: root-level nodes
/// inside `rsx! {{ ... }}` should NOT have a trailing comma (it causes
/// a syntax error), while nested nodes DO need commas between siblings.
pub(crate) fn gen_view_node_dioxus(w: &mut CodeWriter, node: &ViewTree, is_root: bool) {
    match node {
        ViewTree::Element(el) => {
            let mut attrs: Vec<String> = Vec::new();

            // Static props
            for (prop_name, prop_value) in &el.props {
                let attr_name = to_dioxus_attr(prop_name);
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value.as_str().unwrap_or_default(); // defense-in-depth fallback
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
                match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        if event_data.is_some() {
                            let event_obj = emit_event_object_dioxus(event_data, used_fields, "e");
                            attrs.push(format!(
                                "{}: move |e| {{ {}({}); }},",
                                dioxus_event, r#ref, event_obj
                            ));
                        } else {
                            attrs.push(format!("{}: {},", dioxus_event, r#ref));
                        }
                    }
                    EventHandlerIR::InlineFn {
                        body,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        if event_data.is_some() {
                            let _event_obj = emit_event_object_dioxus(event_data, used_fields, "e");
                            let mut hw = CodeWriter::new();
                            // Dioxus generates Rust code (not JS), so instead of Object.assign
                            // we emit individual let bindings that shadow e.field accesses.
                            // Each binding extracts the field value from the web_sys event
                            // using the correct Rust API (e.g. e.target().value().unwrap_or_default()).
                            let kind = event_data.as_ref().unwrap();
                            let fields: Vec<&str> = match used_fields {
                                Some(ref f) if !f.is_empty() => {
                                    f.iter().map(|s| s.as_str()).collect()
                                }
                                _ => vec![],
                            };
                            for field in &fields {
                                let extract = field_extract_dioxus(kind, field, "e");
                                hw.emit_fmt(format_args!("let {} = {};", field, extract));
                            }
                            for stmt in &body.statements {
                                // Use custom formatter with event_param_name set
                                // so FieldAccess on "e" emits bare field name (local let binding)
                                let fmt = DioxusStmtFormatter {
                                    expr: RustFormatExpr {
                                        move_closures: true,
                                        event_param_name: Some("e".to_string()),
                                    },
                                };
                                fmt.format_stmt(&mut hw, stmt);
                            }
                            let content = hw.into_string();
                            let mut result = String::from("move |e| {\n");
                            for line in content.lines() {
                                result.push_str(&format!("                {}\n", line));
                            }
                            result.push_str("            }");
                            attrs.push(format!("{}: {},", dioxus_event, result));
                        } else {
                            let handler_str = format_inline_handler_dioxus(body);
                            attrs.push(format!("{}: {},", dioxus_event, handler_str));
                        }
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        attrs.push(format!("{}: move |_| {{ {}(); }},", dioxus_event, r#ref));
                    }
                }
            }

            // Style classes
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    attrs.push(format!("class: \"{}\",", classes_str));
                }
            }

            // Dynamic class bindings (class_map_bindings)
            for (cls, expr) in &el.class_map_bindings {
                match expr {
                    ExprIR::LiteralExpr { value } if value.as_bool() == Some(true) => {
                        attrs.push(format!("class: if true {{ \"{}\" }},", cls));
                    }
                    ExprIR::LiteralExpr { value } if value.as_bool() == Some(false) => {
                        // skip — class is never applied
                    }
                    _ => {
                        let cond = format_expr_ir_dioxus(expr);
                        attrs.push(format!("class: if {} {{ \"{}\" }},", cond, cls));
                    }
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
            let has_empty = list.empty_body.is_some();
            if has_empty {
                w.emit_fmt(format_args!("if {}.len() == 0 {{", iterable_str));
                w.indent();
                w.emit("rsx! {");
                w.indent();
                if let Some(ref empty_body) = list.empty_body {
                    gen_view_node_dioxus(w, empty_body, true);
                }
                w.dedent();
                w.emit("},");
                w.dedent();
                w.emit("} else {");
                w.indent();
            }
            w.emit_fmt(format_args!(
                "{}.iter().map(|{}| {{",
                iterable_str, item_name
            ));
            w.indent();
            w.emit("rsx! {");
            w.indent();
            if let Some(ref key_expr) = list.key_expr {
                let key_str = format_expr_ir_dioxus(key_expr);
                w.emit_fmt(format_args!("key: {key_str},"));
            }
            if let Some(ref body) = list.body {
                gen_view_node_dioxus(w, body, true);
            }
            w.dedent();
            w.emit("},");
            w.dedent();
            w.emit("})");
            if has_empty {
                w.dedent();
                w.emit("}");
            }
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
        ViewTree::Fragment(children) => {
            for child in children {
                gen_view_node_dioxus(w, child, is_root);
            }
        }
    }
}

/// Build an inline Rust closure string from a BlockIR for Dioxus event handlers.
pub(super) fn format_inline_handler_dioxus(body: &BlockIR) -> String {
    let config = InlineHandlerConfig {
        opener: "move |_| {\n".into(),
        closer: "\n            }".into(),
        empty_handler: "move |_| {}".into(),
        indent: "            ",
        use_writer: false,
    };
    build_inline_handler(body, &config, generate_statement_dioxus)
}
