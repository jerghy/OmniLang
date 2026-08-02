//! Vue template node generation.

use super::{generate_statement_vue, to_vue_attr, to_vue_event, CodeWriter, VueFormatExpr};
use crate::codegen::emit_event_object as emit_event_object_js;
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::style::{format_style_prop_key, format_style_value_expr};
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::ir::*;
use rustc_hash::FxHashSet;
use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;

// ═══════════════════════════════════════════════════════════
// Vue template node generation
// ═══════════════════════════════════════════════════════════

pub(crate) fn gen_template_node(w: &mut CodeWriter, node: &ViewTree, fmt: &VueFormatExpr) {
    match node {
        ViewTree::Element(el) => {
            let mut attrs: Vec<String> = Vec::new();

            // Static props -> plain HTML attributes
            for (prop_name, prop_value) in &el.props {
                let vue_attr = to_vue_attr(prop_name);
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value.as_str().unwrap_or_default(); // defense-in-depth fallback
                                                                    // Escape quotes for HTML attribute
                                                                    // Check for dangerous URL schemes in static URL attributes
                        const URL_ATTRS: &[&str] =
                            &["href", "src", "srcset", "action", "formaction", "poster"];
                        if URL_ATTRS.contains(&vue_attr.as_str())
                            && crate::codegen::sanitize::is_dangerous_url(s)
                        {
                            // Emit empty string to neutralize dangerous URLs at compile time
                            attrs.push(format!("{}=\"\"", vue_attr));
                        } else {
                            let escaped = s.replace('"', "&quot;");
                            attrs.push(format!("{}=\"{}\"", vue_attr, escaped));
                        }
                    }
                    _ => {
                        let vue_val = fmt.format_expr(prop_value);
                        attrs.push(format!(":{}={}", vue_attr, vue_val));
                    }
                }
            }

            // Signal bindings -> :prop="signalRef"
            for (bind_name, signal_ref) in &el.bindings {
                let vue_attr = to_vue_attr(bind_name);
                attrs.push(format!(":{}={}", vue_attr, signal_ref.name));
            }

            // Events -> @event="handler"
            for (event_name, handler) in &el.events {
                let vue_event = to_vue_event(event_name);
                match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        if event_data.is_some() {
                            let event_obj = emit_event_object_js(event_data, used_fields, "e");
                            attrs.push(format!(
                                "@{}=\"(e) => {{ {}({}) }}\"",
                                vue_event, r#ref, event_obj
                            ));
                        } else {
                            attrs.push(format!("@{}=\"{}\"", vue_event, r#ref));
                        }
                    }
                    EventHandlerIR::InlineFn {
                        body,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        if event_data.is_some() {
                            let event_obj = emit_event_object_js(event_data, used_fields, "e");
                            let mut hw = CodeWriter::new();
                            hw.emit_fmt(format_args!("Object.assign(e, {});", event_obj));
                            for stmt in &body.statements {
                                generate_statement_vue(&mut hw, stmt, &fmt.diagnostics);
                            }
                            let content = hw.into_string();
                            let mut result = String::from("(e) => {\n");
                            for line in content.lines() {
                                let _ = write!(result, "        {}\n", line);
                            }
                            result.push_str("      }");
                            attrs.push(format!("@{}=\"{}\"", vue_event, result));
                        } else {
                            let handler_str = format_inline_handler_vue(body, &fmt.diagnostics);
                            attrs.push(format!("@{}=\"{}\"", vue_event, handler_str));
                        }
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        attrs.push(format!("@{}=\"{}()\"", vue_event, r#ref));
                    }
                }
            }

            // Style classes from view
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    attrs.push(format!(":class=\"{}\"", classes_str));
                }
            }

            // Class map bindings -> :class="{ className: expr }"
            if !el.class_map_bindings.is_empty() {
                let bindings: Vec<String> = el
                    .class_map_bindings
                    .iter()
                    .map(|(class_name, expr)| format!("{}: {}", class_name, fmt.format_expr(expr)))
                    .collect();
                attrs.push(format!(":class=\"{{ {} }}\"", bindings.join(", ")));
            }

            // Inline style
            if let Some(ref style_json) = el.inline_style {
                if let Some(obj) = style_json.as_object() {
                    let mut pairs: Vec<String> = Vec::new();
                    for (prop, val) in obj {
                        let js_prop = format_style_prop_key(prop);
                        let js_val = format_style_value_expr(val, &FxHashSet::default());
                        pairs.push(format!("{}: {}", js_prop, js_val));
                    }
                    if !pairs.is_empty() {
                        attrs.push(format!(":style=\"{{ {} }}\"", pairs.join(", ")));
                    }
                }
            }

            // Emit opening tag
            if attrs.is_empty() && el.children.is_empty() {
                w.emit_fmt(format_args!("<{} />", el.tag));
                return;
            }

            if attrs.is_empty() {
                w.emit_fmt(format_args!("<{}>", el.tag));
            } else {
                w.emit_fmt(format_args!("<{}", el.tag));
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
                gen_template_node(w, child, fmt);
            }
            w.dedent();

            // Closing tag
            w.emit_fmt(format_args!("</{}>", el.tag));
        }

        ViewTree::Conditional(cond) => {
            let branches = &cond.branches;
            let fallback = &cond.fallback;

            if branches.is_empty() {
                if let Some(ref fb) = fallback {
                    gen_template_node(w, fb, fmt);
                }
                return;
            }

            // Single branch without fallback ->
            //   <template v-if="condition">
            //     ...body...
            //   </template>
            if branches.len() == 1 && fallback.is_none() {
                let cond_str = fmt.format_expr(&branches[0].condition);
                let cond_safe = cond_str.replace('"', "&quot;");
                w.emit_fmt(format_args!("<template v-if=\"{}\">", cond_safe));
                w.indent();
                if let Some(ref body) = branches[0].body {
                    gen_template_node(w, body, fmt);
                }
                w.dedent();
                w.emit("</template>");
                return;
            }

            // Multiple branches or with fallback
            for (i, branch) in branches.iter().enumerate() {
                let cond_str = fmt.format_expr(&branch.condition);
                let cond_safe = cond_str.replace('"', "&quot;");
                if i == 0 {
                    w.emit_fmt(format_args!("<template v-if=\"{}\">", cond_safe));
                } else {
                    w.emit_fmt(format_args!("<template v-else-if=\"{}\">", cond_safe));
                }
                w.indent();
                if let Some(ref body) = branch.body {
                    gen_template_node(w, body, fmt);
                }
                w.dedent();
                w.emit("</template>");
            }

            if let Some(ref fb) = fallback {
                w.emit("<template v-else>");
                w.indent();
                gen_template_node(w, fb, fmt);
                w.dedent();
                w.emit("</template>");
            }
        }

        ViewTree::ListRendering(list) => {
            let iterable_str = fmt.format_expr(&list.iterable);
            let item_name = &list.item_binding.name;
            let has_empty = list.empty_body.is_some();
            if has_empty {
                w.emit_fmt(format_args!(
                    "<template v-if=\"{}.length > 0\">",
                    iterable_str
                ));
                w.indent();
            }
            let key = match &list.key_expr {
                Some(expr) => fmt.format_expr(expr),
                None => "index".to_string(),
            };
            w.emit_fmt(format_args!(
                "<template v-for=\"({}, index) in {}\" :key=\"{}\">",
                item_name, iterable_str, key
            ));
            w.indent();
            if let Some(ref body) = list.body {
                gen_template_node(w, body, fmt);
            }
            w.dedent();
            w.emit("</template>");
            if has_empty {
                w.dedent();
                w.emit("</template>");
                w.emit("<template v-else>");
                w.indent();
                if let Some(ref empty_body) = list.empty_body {
                    gen_template_node(w, empty_body, fmt);
                }
                w.dedent();
                w.emit("</template>");
            }
        }

        ViewTree::Text(text) => {
            if text.starts_with('{') && text.ends_with('}') {
                // Interpolation expression like {name} or {param:xxx}
                // In Vue, use {{ }} for interpolation
                let inner = &text[1..text.len() - 1];
                w.emit_fmt(format_args!("{{{{ {} }}}}", inner));
            } else {
                // Plain text — escape Vue template delimiters and HTML-significant chars
                let safe = text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace("{{", "\\{\\{")
                    .replace("}}", "\\}\\}");
                w.emit(&safe);
            }
        }
        ViewTree::Interpolation(expr) => {
            let formatted = fmt.format_expr(expr);
            w.emit_fmt(format_args!("{{{{ {} }}}}", formatted));
        }
        ViewTree::Slot { name: slot_name, .. } => match slot_name {
            None => w.emit("<slot />"),
            Some(name) => w.emit_fmt(format_args!("<slot name=\"{}\" />", name)),
        },
        ViewTree::Fragment(children) => {
            for child in children {
                gen_template_node(w, child, fmt);
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Build an inline arrow function string from a BlockIR for Vue event handlers.
pub(super) fn format_inline_handler_vue(
    body: &BlockIR,
    diagnostics: &Rc<RefCell<Vec<String>>>,
) -> String {
    let config = InlineHandlerConfig {
        opener: "() => {\n".into(),
        closer: "\n      }".into(),
        empty_handler: "() => {}".into(),
        indent: "        ",
        use_writer: false,
    };
    build_inline_handler(body, &config, |w, stmt| {
        generate_statement_vue(w, stmt, diagnostics)
    })
}

// ═══════════════════════════════════════════════════════════
// Function generation (Vue script setup)
// ═══════════════════════════════════════════════════════════

pub(super) fn generate_function_vue(
    w: &mut CodeWriter,
    func: &FunctionIR,
    diagnostics: &Rc<RefCell<Vec<String>>>,
) {
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = p.name.clone();
        if let Some(ref ty) = p.r#type {
            let ann = JsTypeFormatter.format_type(ty.kind.as_str(), &ty.type_args);
            if !ann.is_empty() {
                let _ = write!(s, ": {}", ann);
            }
        }
        param_strs.push(s);
    }

    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| {
            let ann = JsTypeFormatter.format_type(rt.kind.as_str(), &rt.type_args);
            if ann.is_empty() {
                String::new()
            } else {
                format!(": {}", ann)
            }
        })
        .unwrap_or_default();

    let all_params = param_strs.join(", ");
    let async_kw = if func.is_async { "async " } else { "" };
    w.emit_fmt(format_args!(
        "{}function {}({}){} {{",
        async_kw, func.name, all_params, ret_ann
    ));
    w.indent();

    for stmt in &func.body.statements {
        generate_statement_vue(w, stmt, diagnostics);
    }

    w.dedent();
    w.emit("}");
}
