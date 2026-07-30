//! Event binding emission: addEventListener JS calls and @for template
//! data-event-* attributes.
//!
//! Contains `emit_element_events` which processes `ElementIR` event handlers
//! and writes either:
//! - `addEventListener(...)` calls (normal mode), or
//! - `data-event-*` attributes with delegated handler registration (inside @for).

use std::fmt::Write;

use super::super::utils::{escape_html, map_event};
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
        for (event_name, handler) in &el.events {
            let js_event = map_event(event_name);
            if self.scope.in_for_template {
                // Inside @for template: emit data-event-* attributes for delegation.
                // The @for container gets a single delegated listener that reads these.
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
                        for stmt in &body.statements {
                            self.emit_statement(stmt);
                        }
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
                            for stmt in &body.statements {
                                self.emit_statement(stmt);
                            }
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
