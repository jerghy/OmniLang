//! Event binding emission: addEventListener JS calls and @for template
//! data-event-* attributes.
//!
//! Contains `emit_element_events` which processes `ElementIR` event handlers
//! and writes either:
//! - `addEventListener(...)` calls (normal mode), or
//! - `data-event-*` attributes with delegated handler registration (inside @for).

use std::fmt::Write;

use super::super::utils::{escape_html, event_missing_js_mapping, map_event, map_tag};
use super::super::OmniViewGenerator;
use crate::codegen::emit_event_object;
use crate::ir::*;

impl OmniViewGenerator {
    /// Build a pipe-delimited suffix encoding preventDefault/stopPropagation flags
    /// for data-event-* attribute values inside @for templates.
    fn flag_suffix(pd: bool, sp: bool) -> &'static str {
        match (pd, sp) {
            (true, true) => "|pd|sp",
            (true, false) => "|pd",
            (false, true) => "|sp",
            (false, false) => "",
        }
    }

    /// Emit event bindings: for @for templates, emit data-event-* attributes;
    /// for normal mode, generate addEventListener JS calls.
    pub(super) fn emit_element_events(&mut self, el: &ElementIR, nid: &str, attrs: &mut String) {
        self.assert_can_emit_js();
        // 归一化后的 HTML 标签（map_tag），供 toggle 的 details 元素条件使用
        // （task 6.1b）。
        let html_tag = map_tag(&el.tag);
        for (event_name, handler) in &el.events {
            // v4 domain HIGH-1: hover/toggle/longpress 无 DOM 事件——
            // map_event 直通发射产生永不被触发的死监听器。warning + 跳过
            // （Flutter 正确映射 onLongPress/onHover/onChanged）。toggle 例外：
            // `<details>` 上是真实 DOM 事件（task 6.1b），details 不跳过。
            if let Some(w) = event_missing_js_mapping(event_name, &html_tag) {
                self.warnings.push(w);
                continue;
            }
            let js_event = map_event(event_name);
            if self.scope.in_for_template {
                // Inside @for template: emit data-event-* attributes for delegation.
                // The @for container gets a single delegated listener that reads these.
                // Collect the JS event name here (not via AST walk in list.rs):
                // this code path also sees events inside inlined child components,
                // which the AST-based collector misses (the TodoItem ✕ button bug).
                if !self.scope.for_event_types.contains(&js_event) {
                    self.scope.for_event_types.push(js_event.clone());
                    if let Some(kind) = handler.event_data() {
                        self.scope
                            .for_event_kinds
                            .push((js_event.clone(), kind.clone()));
                    }
                }
                // Union used_fields across handlers of the same EventDataKind label.
                if let Some(kind) = handler.event_data() {
                    let label = event_data_kind_label(kind);
                    let entry = self
                        .scope
                        .used_fields_by_kind
                        .entry(label.to_string())
                        .or_insert_with(|| Some(Vec::new()));
                    match handler.used_fields() {
                        Some(used) => {
                            if let Some(accumulated) = entry.as_mut() {
                                for field in used {
                                    if !accumulated.contains(field) {
                                        accumulated.push(field.clone());
                                    }
                                }
                            }
                        }
                        None => *entry = None,
                    }
                }
                let val = match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        needs_prevent_default,
                        needs_stop_propagation,
                        ..
                    } => {
                        let prefixed = format!("{}_{}", self.scope.current_component, r#ref);
                        let flags =
                            Self::flag_suffix(*needs_prevent_default, *needs_stop_propagation);
                        format!("fn:{}{}", escape_html(&prefixed), flags)
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        let var_name = r#ref.replace('.', "_");
                        format!("store:{}", escape_html(&var_name))
                    }
                    EventHandlerIR::InlineFn {
                        body,
                        params,
                        needs_prevent_default,
                        needs_stop_propagation,
                        ..
                    } => {
                        let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
                        let params_str = params.join(", ");
                        let _ = writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
                        // Inside @for templates this function is declared at top
                        // level where the loop variable is out of scope; rewrite
                        // references to it (__omni_forItemVar) so the delegated
                        // listener can inject the clicked item at event time.
                        let saved_rewrite = self.scope.rewrite_for_item;
                        self.scope.rewrite_for_item = self.scope.in_for_template;
                        for stmt in &body.statements {
                            self.emit_statement(stmt);
                        }
                        self.scope.rewrite_for_item = saved_rewrite;
                        self.js.push_str("}\n");
                        let flags =
                            Self::flag_suffix(*needs_prevent_default, *needs_stop_propagation);
                        format!("fn:{}{}", escape_html(&wrapper_name), flags)
                    }
                };
                let _ = write!(attrs, " data-event-{}=\"{}\"", js_event, val);
                // Register handler in the global lookup so the delegated
                // listener can find it (functions are IIFE-local, not on
                // window).
                let handler_name = match handler {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        Some(format!("{}_{}", self.scope.current_component, r#ref))
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => Some(r#ref.replace('.', "_")),
                    EventHandlerIR::InlineFn { .. } => {
                        Some(format!("__on{}_{}", event_name, nid.replace('-', "_")))
                    }
                };
                if let Some(ref hname) = handler_name {
                    let _ = writeln!(self.js, "__omni_forHandlers[\"{}\"] = {};", hname, hname);
                }
            } else {
                // ResizeEvent binds to window, not el.addEventListener
                if let Some(EventDataKind::ResizeEvent) = handler.event_data() {
                    let event_obj =
                        emit_event_object(handler.event_data(), handler.used_fields(), "_e");
                    let handler_call = match handler {
                        EventHandlerIR::FnRef { r#ref, .. } => {
                            let prefixed = format!("{}_{}", self.scope.current_component, r#ref);
                            format!("{}({})", prefixed, event_obj)
                        }
                        EventHandlerIR::StoreAction { r#ref, .. } => {
                            let var_name = r#ref.replace('.', "_");
                            format!("{}()", var_name)
                        }
                        EventHandlerIR::InlineFn { body, params, .. } => {
                            let wrapper_name =
                                format!("__on{}_{}", event_name, nid.replace('-', "_"));
                            let params_str = params.join(", ");
                            let _ =
                                writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
                            // Same @for loop-variable rewrite as the template branch.
                            let saved_rewrite = self.scope.rewrite_for_item;
                            self.scope.rewrite_for_item = self.scope.in_for_template;
                            for stmt in &body.statements {
                                self.emit_statement(stmt);
                            }
                            self.scope.rewrite_for_item = saved_rewrite;
                            self.js.push_str("}\n");
                            format!("{}({})", wrapper_name, event_obj)
                        }
                    };
                    let _ = writeln!(
                        self.js,
                        "{{ window.addEventListener(\"resize\", (_e) => {{ {} }}); }}",
                        handler_call
                    );
                    continue;
                }
                let pd = if handler.needs_prevent_default() || js_event == "submit" {
                    " e.preventDefault();"
                } else {
                    ""
                };
                let sp = if handler.needs_stop_propagation() {
                    " e.stopPropagation();"
                } else {
                    ""
                };
                self.emit_event_listener(nid, &js_event, event_name, handler, sp, pd);
            }
        }
    }

    /// Emit addEventListener JS for a single event handler.
    /// Handles all three EventHandlerIR variants (FnRef, StoreAction, InlineFn).
    /// sp/pd are the pre-computed stopPropagation/preventDefault suffix strings.
    pub(crate) fn emit_event_listener(
        &mut self,
        nid: &str,
        js_event: &str,
        event_name: &str,
        handler: &EventHandlerIR,
        sp: &str,
        pd: &str,
    ) {
        match handler {
            EventHandlerIR::FnRef { r#ref, .. } => {
                if self.name_resolver.motion_names.contains(r#ref.as_str()) {
                    // Motion reference: call factory and play on event
                    let _ = writeln!(self.js,
                        "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ try {{ if (el.offsetParent !== null) {{ var p = __omni_motion_{motion}(el, {{}}); __omni_animations.set(\"{nid}_{js_event}\", p); p.play(); }} }} catch(err) {{ __omni_reportError(\"error:event:{js_event}\", err); }} }}); }}",
                        nid = nid,
                        js_event = js_event,
                        motion = r#ref,
                    );
                } else {
                    let prefixed = format!("{}_{}", self.scope.current_component, r#ref);
                    let event_obj =
                        emit_event_object(handler.event_data(), handler.used_fields(), "e");
                    let _ = writeln!(self.js,
                        "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ try {{ {sp} {pd} {prefixed}({event_obj}); }} catch(err) {{ __omni_reportError(\"error:event:{js_event}\", err); }} }}); }}",
                        nid = nid, js_event = js_event, sp = sp, pd = pd, prefixed = prefixed, event_obj = event_obj
                    );
                }
            }
            EventHandlerIR::StoreAction { r#ref, .. } => {
                let var_name = r#ref.replace('.', "_");
                let _ = writeln!(self.js,
                    "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ try {{ {} {} {}(); }} catch(err) {{ __omni_reportError(\"error:event:{}\", err); }} }}); }}",
                    nid, js_event, sp, pd, var_name, js_event
                );
            }
            EventHandlerIR::InlineFn { body, params, .. } => {
                let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
                let params_str = params.join(", ");
                let _ = writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
                for stmt in &body.statements {
                    self.emit_statement(stmt);
                }
                self.js.push_str("}\n");
                let event_obj = emit_event_object(handler.event_data(), handler.used_fields(), "e");
                let _ = writeln!(self.js,
                    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ try {{ {sp} {pd} {wrapper_name}({event_obj}); }} catch(err) {{ __omni_reportError(\"error:event:{js_event}\", err); }} }}); }}",
                    nid = nid, js_event = js_event, sp = sp, pd = pd, wrapper_name = wrapper_name, event_obj = event_obj
                );
            }
        }
    }
}
