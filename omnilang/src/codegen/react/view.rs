// ═══════════════════════════════════════════════════════════
// View tree generation (JSX)
// ═══════════════════════════════════════════════════════════

use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::style::{format_style_prop_key, format_style_value_expr};
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::super::capitalize_first;
use super::component::jsx::component_declared_slots;
use super::design::DesignCtx;
use super::expr::{format_expr_ir_tsx, generate_statement_tsx};
use super::utils::{emit_event_object_react, to_jsx_attr, to_react_event, to_react_tag};
use crate::codegen::sanitize::is_dangerous_url;
use std::fmt::Write;

#[allow(clippy::too_many_arguments)]
pub(super) fn gen_view_node_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    design_ctx: &DesignCtx,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
    motion_counter: &mut usize,
) {
    match node {
        ViewTree::Element(el) => {
            // Fragment tag -> React fragment shorthand
            if el.tag == "fragment" {
                if el.children.is_empty() {
                    return;
                }
                for child in &el.children {
                    gen_view_node_tsx(
                        w,
                        child,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        motion_counter,
                    );
                }
                return;
            }

            // Component composition: check if tag is a known component
            if let Some(child_comp) = component_map.get(el.tag.as_str()) {
                // Ref forwarding for custom components is Phase 2 — skip for now
                if el.ref_binding.is_some() {
                    // Only native HTML elements support @ref in the current phase
                }
                // Slots the child component actually declares — children
                // carrying a `slot="name"` attribute, or a consumer-position
                // `@slot name { ... }` / `name={<.../>}` projection (both
                // lower to `ViewTree::Slot` children), project into the
                // matching named prop; anything else flows into the
                // `children` prop. Unknown slot names fall back into
                // `children` (content stays visible and the generated TS
                // never breaks).
                let mut declared_slot_names: FxHashSet<String> = FxHashSet::default();
                for (slot_name, _) in component_declared_slots(child_comp, component_map) {
                    if let Some(n) = slot_name {
                        declared_slot_names.insert(n);
                    }
                }
                let mut comp_attrs: Vec<String> = Vec::new();
                for (prop_name, prop_value) in &el.props {
                    let js_val = format_expr_ir_tsx(prop_value, ref_names);
                    comp_attrs.push(format!("{}={{{}}}", prop_name, js_val));
                }
                for (event_name, handler) in &el.events {
                    let jsx_event = to_react_event(event_name);
                    match handler {
                        EventHandlerIR::FnRef { r#ref, .. } => {
                            comp_attrs.push(format!("{}={{{}}}", jsx_event, r#ref));
                        }
                        EventHandlerIR::InlineFn { body, params, .. } => {
                            let handler_str = format_inline_handler(
                                body,
                                signal_names,
                                store_signal_owners,
                                params,
                                ref_names,
                            );
                            comp_attrs.push(format!("{}={{{}}}", jsx_event, handler_str));
                        }
                        EventHandlerIR::StoreAction { r#ref, .. } => {
                            // Use full qualified ref (e.g. "NotificationStore.clear")
                            // so store methods are called through their hook object
                            comp_attrs.push(format!("{}={{() => {{ {}(); }}}}", jsx_event, r#ref));
                        }
                    }
                }
                // Partition children: elements carrying a literal
                // `slot="name"` attribute, or consumer-position
                // `@slot name { ... }` projections, matching a declared slot
                // project into the named prop; everything else goes to the
                // default `children` prop.
                let mut default_children: Vec<&ViewTree> = Vec::new();
                let mut named_children: Vec<(String, &ViewTree)> = Vec::new();
                for child in &el.children {
                    match child_slot_name(child) {
                        Some(slot_name) if declared_slot_names.contains(&slot_name) => {
                            named_children.push((slot_name, child));
                        }
                        _ => default_children.push(child),
                    }
                }
                for (slot_name, child) in &named_children {
                    // Render the projected content into the named prop and
                    // strip the `slot` attribute so it never leaks into the
                    // DOM (it is only meaningful for web components).
                    let mut child_w = crate::codegen::CodeWriter::new();
                    gen_consumer_child_tsx(
                        &mut child_w,
                        child,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        motion_counter,
                    );
                    let child_jsx = child_w.into_string();
                    let trimmed = child_jsx.trim();
                    if !trimmed.is_empty() {
                        comp_attrs.push(format!("{}={{{}}}", slot_name, trimmed));
                    }
                }
                let attrs_str = comp_attrs.join(" ");
                if default_children.is_empty() {
                    w.emit_fmt(format_args!("<{} {}/>", el.tag, attrs_str));
                } else {
                    w.emit_fmt(format_args!("<{} {}>", el.tag, attrs_str));
                    w.indent();
                    for child in default_children {
                        gen_consumer_child_tsx(
                            w,
                            child,
                            component_map,
                            design_ctx,
                            signal_names,
                            store_signal_owners,
                            motion_names,
                            ref_names,
                            motion_counter,
                        );
                    }
                    w.dedent();
                    w.emit_fmt(format_args!("</{}>", el.tag));
                }
                return; // IMPORTANT: skip HTML element handling
            }
            // Otherwise, fall through to existing HTML element handling
            // Normalize OmniView custom tags (Button→button, Card→div, etc.)
            let html_tag = to_react_tag(&el.tag);
            // Build attribute list from props, bindings, events
            let mut attrs: Vec<String> = Vec::new();

            // Collect binding attr names for dedup with static props
            let bound_attr_names: FxHashSet<String> = el
                .bindings
                .keys()
                .map(|bind_name| to_jsx_attr(bind_name))
                .collect();

            // Collect class_map_binding names so we can exclude them from static className
            let binding_names: FxHashSet<&str> = el
                .class_map_bindings
                .keys()
                .map(|cls| cls.as_str())
                .collect();

            // Static props — skip names that will be populated by bindings
            // `_design_variant` (variant: DesignName.Variant(dim=val)) is
            // expanded into literal global class names after the other class
            // attributes are merged (see below).
            let mut variant_classes: Option<Vec<String>> = None;
            for (prop_name, prop_value) in &el.props {
                // Skip internal IR props (not valid HTML/React attributes)
                if prop_name == "_design_variant" {
                    variant_classes = design_ctx.expand_variant_classes(prop_value);
                    continue;
                }
                let jsx_attr = to_jsx_attr(prop_name);
                // Skip if this attr name will be generated by a binding (e.g., value= vs bind:value)
                if bound_attr_names.contains(&jsx_attr) {
                    continue;
                }
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value.as_str().unwrap_or_default(); // defense-in-depth fallback
                        if jsx_attr == "className" {
                            // Split multi-class names: "browserTab tabActive" → styles['browserTab'] + ' ' + styles['tabActive']
                            // Exclude class names that are handled by class_map_bindings.
                            // Design classes are global CSS in design.css and
                            // are emitted as literal class names (HIGH #1).
                            let classes: Vec<&str> = s
                                .split_whitespace()
                                .filter(|c| !binding_names.contains(c))
                                .collect();
                            if classes.is_empty() {
                                // All classes are handled by bindings — skip static className
                            } else if classes.len() > 1 {
                                let parts: Vec<String> = classes
                                    .iter()
                                    .map(|c| design_ctx.format_class_ref(c))
                                    .collect();
                                attrs.push(format!("className={{{}}}", parts.join(" + ' ' + ")));
                            } else {
                                attrs.push(format!(
                                    "className={{{}}}",
                                    design_ctx.format_class_ref(classes[0])
                                ));
                            }
                        } else {
                            // Check URL attributes for dangerous schemes (javascript:, data:, vbscript:)
                            // Mirrors the shared sanitizer's compile-time is_dangerous_url() defense-in-depth
                            // against supply-chain compromises where a third-party .omni component
                            // contains literal javascript:/data: URLs.
                            let url_attrs: &[&str] =
                                &["href", "src", "srcset", "action", "formaction", "poster"];
                            if url_attrs.contains(&jsx_attr.as_str()) && is_dangerous_url(s) {
                                attrs.push(format!("{}=\"#blocked\"", jsx_attr));
                            } else {
                                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                                attrs.push(format!("{}=\"{}\"", jsx_attr, escaped));
                            }
                        }
                    }
                    _ => {
                        let jsx_val = format_expr_ir_tsx(prop_value, ref_names);
                        attrs.push(format!("{}={{{}}}", jsx_attr, jsx_val));
                    }
                }
            }

            // Two-way bindings: value={x}, collect binding event setters for dedup with user events
            let mut binding_events_map: FxHashMap<String, String> = FxHashMap::default();
            for (bind_name, signal_ref) in &el.bindings {
                let jsx_attr = to_jsx_attr(bind_name);
                let setter = format!("set{}", capitalize_first(&signal_ref.name));
                attrs.push(format!("{}={{{}}}", jsx_attr, signal_ref.name));
                match bind_name.as_str() {
                    "value" => {
                        binding_events_map.insert(
                            "onChange".to_string(),
                            format!("{}(e.target.value)", setter),
                        );
                    }
                    "checked" => {
                        binding_events_map.insert(
                            "onChange".to_string(),
                            format!("{}(e.target.checked)", setter),
                        );
                    }
                    _ => {}
                }
            }

            // Motion triggers — per-element ref + JSX handler wiring. The
            // effects themselves live in motion_effects.rs; this site only:
            // 1. assigns the element's motion ref (user @ref, or synthesized
            //    `__el_N` — see motion::classify_motion_el), and
            // 2. wires @hover/@press to useCallback handlers emitted there.
            let motion_info = super::motion::classify_motion_el(el, component_map, motion_counter);
            if let Some((ref motion_ref_name, motion_idx)) = &motion_info {
                if el.ref_binding.is_none() {
                    // Synthesized refs are declared as `const __el_NRef = useRef(...)`
                    attrs.push(format!("ref={{{}Ref}}", motion_ref_name));
                }
                for trigger in el
                    .motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
                {
                    match trigger.kind {
                        TriggerKind::Hover => {
                            attrs.push(format!("onMouseEnter={{handleEnter_{}}}", motion_idx));
                            attrs.push(format!("onMouseLeave={{handleLeave_{}}}", motion_idx));
                        }
                        TriggerKind::Press => {
                            attrs.push(format!("onPointerDown={{handleDown_{}}}", motion_idx));
                            attrs.push(format!("onPointerUp={{handleUp_{}}}", motion_idx));
                        }
                        _ => {}
                    }
                }
            }

            // Events — with dedup for binding conflicts and motion name filtering
            for (event_name, handler) in &el.events {
                let react_event = to_react_event(event_name);
                match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        // Skip motion names — they're animation references, not functions
                        if motion_names.contains(r#ref.as_str()) {
                            continue;
                        }
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            if event_data.is_some() {
                                let event_obj =
                                    emit_event_object_react(event_data, used_fields, "e");
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}; {}({}); }}}}",
                                    react_event, setter_call, r#ref, event_obj
                                ));
                            } else {
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}; {}(e); }}}}",
                                    react_event, setter_call, r#ref
                                ));
                            }
                        } else if event_data.is_some() {
                            let event_obj = emit_event_object_react(event_data, used_fields, "e");
                            // When no event fields are actually needed (empty object),
                            // skip the (e) wrapper to avoid unused-param warnings.
                            if event_obj == "{}" {
                                attrs.push(format!("{}={{{}}}", react_event, r#ref));
                            } else {
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}({}) }}}}",
                                    react_event, r#ref, event_obj
                                ));
                            }
                        } else {
                            attrs.push(format!("{}={{{}}}", react_event, r#ref));
                        }
                    }
                    EventHandlerIR::InlineFn {
                        body,
                        params,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            // Generate body with binding setter prepended
                            let mut merged_hw = CodeWriter::new();
                            merged_hw.emit_fmt(format_args!("{};", setter_call));
                            // Emit event data extraction for InlineFn with event_data
                            if event_data.is_some() {
                                let event_obj =
                                    emit_event_object_react(event_data, used_fields, "e");
                                merged_hw
                                    .emit_fmt(format_args!("Object.assign(e, {});", event_obj));
                            }
                            for stmt in &body.statements {
                                generate_statement_tsx(
                                    &mut merged_hw,
                                    stmt,
                                    signal_names,
                                    store_signal_owners,
                                    ref_names,
                                );
                            }
                            let content = merged_hw.into_string();
                            let params_str = if params.is_empty() {
                                "e".to_string()
                            } else {
                                params.join(", ")
                            };
                            let mut result = format!("({}) => {{\n", params_str);
                            for line in content.lines() {
                                let _ = write!(result, "        {}\n", line);
                            }
                            result.push_str("      }");
                            attrs.push(format!("{}={{{}}}", react_event, result));
                        } else {
                            // Non-binding InlineFn: add event extraction before body
                            if event_data.is_some() {
                                let event_obj =
                                    emit_event_object_react(event_data, used_fields, "e");
                                let mut merged_hw = CodeWriter::new();
                                merged_hw
                                    .emit_fmt(format_args!("Object.assign(e, {});", event_obj));
                                for stmt in &body.statements {
                                    generate_statement_tsx(
                                        &mut merged_hw,
                                        stmt,
                                        signal_names,
                                        store_signal_owners,
                                        ref_names,
                                    );
                                }
                                let content = merged_hw.into_string();
                                let params_str = if params.is_empty() {
                                    "e".to_string()
                                } else {
                                    params.join(", ")
                                };
                                let mut result = format!("({}) => {{\n", params_str);
                                for line in content.lines() {
                                    let _ = write!(result, "        {}\n", line);
                                }
                                result.push_str("      }");
                                attrs.push(format!("{}={{{}}}", react_event, result));
                            } else {
                                let handler_str = format_inline_handler(
                                    body,
                                    signal_names,
                                    store_signal_owners,
                                    params,
                                    ref_names,
                                );
                                attrs.push(format!("{}={{{}}}", react_event, handler_str));
                            }
                        }
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        // Use full qualified ref (e.g. "NotificationStore.clear")
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            attrs.push(format!(
                                "{}={{(e) => {{ {}; {}(); }}}}",
                                react_event, setter_call, r#ref
                            ));
                        } else {
                            attrs.push(format!(
                                "{react_event}={{() => {{ {action_ref}(); }}}}",
                                react_event = react_event,
                                action_ref = r#ref
                            ));
                        }
                    }
                }
            }

            // Add remaining binding events that weren't merged with user events
            for (event_name, setter_call) in &binding_events_map {
                attrs.push(format!("{}={{(e) => {{ {}; }}}}", event_name, setter_call));
            }

            // Style classes from view — design classes emit literal names
            // (global rules in design.css), component classes stay CSS-module
            // lookups (HIGH #1).
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    let class_parts: Vec<&str> = classes_str.split_whitespace().collect();
                    if class_parts.len() > 1 {
                        let parts: Vec<String> = class_parts
                            .iter()
                            .map(|c| design_ctx.format_class_ref(c))
                            .collect();
                        attrs.push(format!("className={{{}}}", parts.join(" + ' ' + ")));
                    } else {
                        attrs.push(format!(
                            "className={{{}}}",
                            design_ctx.format_class_ref(classes_str)
                        ));
                    }
                }
            }

            // Inline style
            if let Some(ref style_json) = el.inline_style {
                if let Some(obj) = style_json.as_object() {
                    let mut pairs: Vec<String> = Vec::new();
                    for (prop, val) in obj {
                        let js_prop = format_style_prop_key(prop);
                        let js_val = format_style_value_expr(val, ref_names);
                        pairs.push(format!("{}: {}", js_prop, js_val));
                    }
                    if !pairs.is_empty() {
                        attrs.push(format!("style={{{{ {} }}}}", pairs.join(", ")));
                    }
                }
            }

            // Dynamic class bindings: class:active={isOpen} → className using
            // CSS module syntax for component classes, literal names for
            // design classes (global rules in design.css, HIGH #1).
            if !el.class_map_bindings.is_empty() {
                let mut static_classes: Vec<String> = Vec::new();
                let mut dynamic_parts: Vec<String> = Vec::new();
                for (cls, expr) in &el.class_map_bindings {
                    match expr {
                        ExprIR::LiteralExpr { value } if value.as_bool() == Some(true) => {
                            static_classes.push(design_ctx.format_class_ref(cls));
                        }
                        ExprIR::LiteralExpr { value } if value.as_bool() == Some(false) => {
                            // skip — class not applied
                        }
                        _ => {
                            let cond = format_expr_ir_tsx(expr, ref_names);
                            let class_ref = design_ctx.format_class_ref(cls);
                            dynamic_parts.push(format!("${{{cond} ? {class_ref} : ''}}"));
                        }
                    }
                }
                // Merge into existing className attribute, or create a new one
                let mut new_class = String::new();
                if !static_classes.is_empty() {
                    new_class = static_classes.join(" + ' ' + ");
                }
                if !dynamic_parts.is_empty() {
                    if !new_class.is_empty() {
                        new_class =
                            format!("({}) + ' ' + `{}`", new_class, dynamic_parts.join(" "));
                    } else {
                        new_class = format!("`{}`", dynamic_parts.join(" "));
                    }
                }
                if !new_class.is_empty() {
                    if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
                        // Merge: prepend the pre-existing static class name if it exists
                        let old = &attrs[pos];
                        // Extract the value from existing className={...}
                        if let Some(inner) = old.strip_prefix("className={") {
                            if let Some(existing) = inner.strip_suffix("}") {
                                // Prepend existing static class to the new_class
                                let merged = if !static_classes.is_empty() {
                                    // class_map_bindings already provided static classes
                                    format!("{existing} + ' ' + ({new_class})")
                                } else {
                                    // Static class from props + dynamic from bindings
                                    format!("{existing} + ' ' + {new_class}")
                                };
                                attrs[pos] = format!("className={{{merged}}}");
                            } else {
                                attrs[pos] = format!("className={{{new_class}}}");
                            }
                        } else {
                            attrs[pos] = format!("className={{{new_class}}}");
                        }
                    } else {
                        attrs.push(format!("className={{{}}}", new_class));
                    }
                }
            }

            // Design variant binding: `variant: LearnDesign.Button(color=primary)`
            // → literal global class names ("Button primary") matching the
            // combination rules in design.css (HIGH #1).  Merged last so a
            // `class:` on the same element keeps both.
            if let Some(variant_classes) = variant_classes {
                let joined = variant_classes.join(" ");
                append_class_attr(&mut attrs, format!("\"{}\"", joined));
            }

            // Ensure <button> has explicit type to prevent accidental form submission
            if html_tag == "button" && !attrs.iter().any(|a| a.starts_with("type=")) {
                attrs.push("type=\"button\"".into());
            }

            // Ensure form fields with placeholder have an accessible label
            let is_form_field = matches!(html_tag.as_str(), "input" | "textarea" | "select");
            if is_form_field
                && attrs.iter().any(|a| a.starts_with("placeholder="))
                && !attrs
                    .iter()
                    .any(|a| a.starts_with("aria-label=") || a.starts_with("aria-labelledby="))
            {
                // Extract placeholder text to use as aria-label
                if let Some(ph_attr) = attrs.iter().find(|a| a.starts_with("placeholder=")) {
                    // Static placeholder: placeholder="Search..."
                    if let Some(start) = ph_attr.find("=\"") {
                        let val_start = start + 2;
                        if let Some(end) = ph_attr[val_start..].find('"') {
                            let ph_val = &ph_attr[val_start..val_start + end];
                            attrs.push(format!("aria-label=\"{}\"", ph_val));
                        }
                    }
                }
            }

            // Ref binding — emit ref={nameRef} JSX attribute
            if let Some(ref ref_name) = el.ref_binding {
                attrs.push(format!("ref={{{}Ref}}", ref_name));
            }

            // Emit opening tag
            if attrs.is_empty() && el.children.is_empty() {
                // Self-closing tag
                w.emit_fmt(format_args!("<{} />", html_tag));
                return;
            }

            if attrs.is_empty() {
                w.emit_fmt(format_args!("<{}>", html_tag));
            } else {
                w.emit_fmt(format_args!("<{}", html_tag));
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
                gen_view_node_tsx(
                    w,
                    child,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    motion_counter,
                );
            }
            w.dedent();

            // Closing tag
            w.emit_fmt(format_args!("</{}>", html_tag));
        }
        ViewTree::Conditional(cond) => {
            let branches = &cond.branches;
            let fallback = &cond.fallback;

            // mode: "wait" — OmniLang has no leave-animation mechanism to
            // wait for, so the transition degrades to an instant swap (same
            // timing as mode: "none"). The degradation is stated in the
            // output so it is never a silent no-op.
            if cond.transition_mode.as_deref() == Some("wait") {
                w.emit("{/* @if mode: \"wait\" — OmniLang has no leave animation; degrades to an instant swap (same as mode: \"none\"). */}");
            }

            // mode: "fade" — each branch renders inside a wrapper div
            // carrying the CSS-module fade class (opacity 0 → 1, defined in
            // the component's .module.css).
            let is_fade = cond.transition_mode.as_deref() == Some("fade");

            if branches.is_empty() {
                if let Some(ref fb) = fallback {
                    emit_fade_branch_open(w, is_fade, "fb");
                    gen_branch_body_tsx(
                        w,
                        fb,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        motion_counter,
                    );
                    emit_fade_branch_close(w, is_fade);
                }
                return;
            }

            // Single branch without fallback → {condition && (...)}
            if branches.len() == 1 && fallback.is_none() {
                w.emit("{");
                w.indent();
                let cond_str = format_expr_ir_tsx(&branches[0].condition, ref_names);
                w.emit_fmt(format_args!("{} && (", cond_str));
                w.indent();
                emit_fade_branch_open(w, is_fade, "0");
                if let Some(ref body) = branches[0].body {
                    gen_branch_body_tsx(
                        w,
                        body,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        motion_counter,
                    );
                }
                emit_fade_branch_close(w, is_fade);
                w.dedent();
                w.emit(")");
                w.dedent();
                w.emit("}");
                return;
            }

            // Multiple branches or fallback → nested ternary
            // Clean structure: each branch closes `? (` before the next `: (`
            // Remaining `: (` from branches 1..N get closed after fallback/null.
            w.emit("{");
            w.indent();

            // branches.is_empty() returned above, so split_first() cannot
            // fail here — the nested-ternary path always has a first branch.
            let (first_branch, rest) = branches
                .split_first()
                .unwrap_or_else(|| unreachable!("branches is non-empty — the empty case returned above"));

            // First branch: `cond ? ( body )`
            let cond_str = format_expr_ir_tsx(&first_branch.condition, ref_names);
            w.emit_fmt(format_args!("{} ? (", cond_str));
            w.indent();
            emit_fade_branch_open(w, is_fade, "0");
            if let Some(ref body) = first_branch.body {
                gen_branch_body_tsx(
                    w,
                    body,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    motion_counter,
                );
            }
            emit_fade_branch_close(w, is_fade);
            w.dedent();

            // Remaining branches: `) : ( cond ? ( body )`
            for (i, branch) in rest.iter().enumerate() {
                w.emit(") : (");
                w.indent();
                let cond_str = format_expr_ir_tsx(&branch.condition, ref_names);
                w.emit_fmt(format_args!("{} ? (", cond_str));
                w.indent();
                emit_fade_branch_open(w, is_fade, &(i + 1).to_string());
                if let Some(ref body) = branch.body {
                    gen_branch_body_tsx(
                        w,
                        body,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        motion_counter,
                    );
                }
                emit_fade_branch_close(w, is_fade);
                w.dedent();
            }

            // Fallback or null
            if let Some(ref fb) = fallback {
                w.emit(") : (");
                w.indent();
                emit_fade_branch_open(w, is_fade, "fb");
                gen_branch_body_tsx(
                    w,
                    fb,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    motion_counter,
                );
                emit_fade_branch_close(w, is_fade);
                w.dedent();
                w.emit(")");
            } else {
                w.emit(") : null");
            }
            // Close each `: (` from rest branches
            for _ in 0..rest.len() {
                w.emit(")");
            }

            w.dedent();
            w.emit("}");
        }
        ViewTree::ListRendering(list) => {
            let iterable_str = format_expr_ir_tsx(&list.iterable, ref_names);
            let item_name = &list.item_binding.name;
            let key_attr = if let Some(ref ke) = list.key_expr {
                format!(" key={{{}}}", format_expr_ir_tsx(ke, ref_names))
            } else {
                // Use item identity as key to avoid index-as-key anti-pattern.
                // String() wraps so React gets a consistent key for objects
                // rather than silently calling toString() -> "[object Object]"
                format!(" key={{String({})}}", item_name)
            };
            if list.empty_body.is_some() {
                w.emit("{");
                w.indent();
                w.emit_fmt(format_args!(
                    "({} && {}.length === 0) ? (",
                    iterable_str, iterable_str
                ));
                w.indent();
                if let Some(ref empty_body) = list.empty_body {
                    gen_branch_body_tsx(
                        w,
                        empty_body,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        motion_counter,
                    );
                }
                w.dedent();
                w.emit_fmt(format_args!(
                    ") : ({}.map(({}, _i) => (",
                    iterable_str, item_name
                ));
            } else {
                w.emit("{");
                w.indent();
                w.emit_fmt(format_args!(
                    "{}.map(({}, _i) => (",
                    iterable_str, item_name
                ));
            }
            w.indent();
            w.emit_fmt(format_args!("<React.Fragment{}>", key_attr));
            w.indent();
            if let Some(ref body) = list.body {
                gen_view_node_tsx(
                    w,
                    body,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    motion_counter,
                );
            }
            w.dedent();
            w.emit("</React.Fragment>");
            w.dedent();
            w.emit("))");
            w.dedent();
            if list.empty_body.is_some() {
                w.emit(")}");
            } else {
                w.emit("}");
            }
        }
        ViewTree::Text(text) => {
            // Skip empty text nodes and text containing only syntax artifacts from IR lowering
            let text_trimmed = text.trim();
            if text_trimmed.is_empty()
                || text_trimmed
                    .chars()
                    .all(|c| c.is_ascii_punctuation() || c.is_ascii_whitespace())
            {
                return;
            }
            if text.starts_with('{') && text.ends_with('}') {
                // Interpolation expression like {name} or {param:xxx}
                let inner = &text[1..text.len() - 1];
                w.emit_fmt(format_args!("{{{}}}", inner));
            } else {
                // Plain text — escape JSX-significant characters
                let escaped = text
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('&', "&amp;");
                w.emit_fmt(format_args!("{{'{}'}}", escaped));
            }
        }
        ViewTree::Interpolation(expr) => {
            let js_expr = format_expr_ir_tsx(expr, ref_names);
            w.emit_fmt(format_args!("{{{}}}", js_expr));
        }
        ViewTree::Fragment(children) => {
            for child in children {
                gen_view_node_tsx(
                    w,
                    child,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    motion_counter,
                );
            }
        }
        ViewTree::Slot { name, fallback, .. } => {
            // Named slots map to same-named props; the unnamed slot maps to
            // `children`. When the parent provides no content for the slot,
            // the preserved fallback (Task 1.2) is rendered instead:
            //   {header ?? (fallback JSX)}
            // Multi-node fallbacks are wrapped in a fragment by
            // gen_branch_body_tsx so the emitted TSX stays valid.
            let prop_name = name.as_deref().unwrap_or("children");
            if fallback.is_empty() {
                w.emit_fmt(format_args!("{{{}}}", prop_name));
            } else {
                w.emit_fmt(format_args!("{{{} ?? (", prop_name));
                w.indent();
                gen_branch_body_tsx(
                    w,
                    &ViewTree::Fragment(fallback.clone()),
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    motion_counter,
                );
                w.dedent();
                w.emit(")}");
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Emit a conditional/ternary branch body (also used for `@for` empty
/// states). A body with multiple top-level nodes is wrapped in a React
/// fragment (`<>...</>`) so the adjacent JSX children stay valid inside the
/// parenthesized `(body)` expression — e.g. an `@if` body containing a slot
/// plus a sibling element.
///
/// Also used for the component root view (`gen_jsx_return`) so that
/// multi-element roots (e.g. a view with an input + buttons as siblings)
/// emit valid JSX instead of adjacent root expressions.
#[allow(clippy::too_many_arguments)]
pub(super) fn gen_branch_body_tsx(
    w: &mut CodeWriter,
    body: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    design_ctx: &DesignCtx,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
    motion_counter: &mut usize,
) {
    let is_multi = match body {
        // Lowering wraps multi-node bodies in an element with tag "fragment";
        // ViewTree::Fragment is the inline form.
        ViewTree::Fragment(children) => children.len() > 1,
        ViewTree::Element(el) if el.tag == "fragment" => el.children.len() > 1,
        _ => false,
    };
    if is_multi {
        w.emit("<>");
        w.indent();
    }
    gen_view_node_tsx(
        w,
        body,
        component_map,
        design_ctx,
        signal_names,
        store_signal_owners,
        motion_names,
        ref_names,
        motion_counter,
    );
    if is_multi {
        w.dedent();
        w.emit("</>");
    }
}

/// Open/close the `mode: "fade"` wrapper div around a conditional branch.
///
/// `key` identifies the branch inside the conditional (`"0"`, `"1"`, ...;
/// `"fb"` for the fallback). The key is REQUIRED for replay: React
/// reconciliation reuses same-position DOM nodes, so a ternary swap
/// (`@if/@else`) would otherwise keep the old wrapper node — animation
/// already finished, opacity stuck at 1 — and the fade would never replay.
/// The key forces a remount, so the CSS animation replays exactly when the
/// shown branch changes. The class and its keyframes are defined in the
/// component's `.module.css` (see `component::gen_component_css`) —
/// same-module definition + reference so the bundler scopes both
/// consistently.
fn emit_fade_branch_open(w: &mut CodeWriter, is_fade: bool, key: &str) {
    if is_fade {
        w.emit_fmt(format_args!(
            "<div key=\"{}\" className={{styles['omni-if-fade']}}>",
            key
        ));
        w.indent();
    }
}

fn emit_fade_branch_close(w: &mut CodeWriter, is_fade: bool) {
    if is_fade {
        w.dedent();
        w.emit("</div>");
    }
}

/// If the view tree is a consumer-side slot projection, return the slot
/// name. Two forms are recognized:
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

/// Render a child of a component instantiation (consumer-side projection
/// content). Consumer-position `@slot name { ... }` directives render their
/// fallback — that IS the projected content — instead of the slot placeholder.
/// Elements get the `slot` attribute stripped so it never reaches the DOM.
#[allow(clippy::too_many_arguments)]
fn gen_consumer_child_tsx(
    w: &mut CodeWriter,
    child: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    design_ctx: &DesignCtx,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
    motion_counter: &mut usize,
) {
    match child {
        ViewTree::Slot { fallback, .. } => {
            gen_branch_body_tsx(
                w,
                &ViewTree::Fragment(fallback.clone()),
                component_map,
                design_ctx,
                signal_names,
                store_signal_owners,
                motion_names,
                ref_names,
                motion_counter,
            );
        }
        _ => {
            gen_view_node_tsx(
                w,
                &strip_slot_prop(child),
                component_map,
                design_ctx,
                signal_names,
                store_signal_owners,
                motion_names,
                ref_names,
                motion_counter,
            );
        }
    }
}

/// Remove the `slot` prop from an element so it never reaches the DOM as a
/// web-component attribute after projection into a React named prop.
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

/// Build an inline arrow function string from a BlockIR.
/// Uses the declared params from the Omni source (e.g., `(text)` or `(e)`) instead of hardcoding `e`,
/// so that the handler body which references the parameter by name works correctly.
/// Unused params are prefixed with `_` to suppress ESLint no-unused-vars warnings.
fn format_inline_handler(
    body: &BlockIR,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    params: &[String],
    ref_names: &FxHashSet<String>,
) -> String {
    // Generate the body content first to check which params are actually used
    let mut body_w = CodeWriter::new();
    for stmt in &body.statements {
        generate_statement_tsx(
            &mut body_w,
            stmt,
            signal_names,
            store_signal_owners,
            ref_names,
        );
    }
    let body_content = body_w.into_string();

    // Check if params are used in the body; prefix unused ones with _
    let effective_params: Vec<String> = params
        .iter()
        .map(|p| {
            if body_content.contains(p.as_str()) {
                p.clone()
            } else {
                format!("_{}", p)
            }
        })
        .collect();

    let param_str = if effective_params.is_empty() {
        String::new()
    } else {
        effective_params.join(", ")
    };
    let config = InlineHandlerConfig {
        opener: if param_str.is_empty() {
            "() => {\n".to_string()
        } else {
            format!("({}) => {{\n", param_str)
        },
        closer: "\n      }".to_string(),
        empty_handler: "() => {}".to_string(),
        indent: "        ",
        use_writer: false,
    };
    build_inline_handler(body, &config, |w, s| {
        generate_statement_tsx(w, s, signal_names, store_signal_owners, ref_names)
    })
}

/// Append a static class expression to the element's `className` attribute,
/// merging into the existing attribute when present and creating a new one
/// otherwise (used for design variant combination classes).
fn append_class_attr(attrs: &mut Vec<String>, class_expr: String) {
    if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
        let old = attrs[pos].clone();
        if let Some(inner) = old.strip_prefix("className={") {
            if let Some(existing) = inner.strip_suffix('}') {
                attrs[pos] = format!("className={{{} + ' ' + {}}}", existing, class_expr);
                return;
            }
        }
        attrs[pos] = format!("className={{{}}}", class_expr);
    } else {
        attrs.push(format!("className={{{}}}", class_expr));
    }
}
