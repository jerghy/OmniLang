use super::utils::map_event;
use super::OmniViewGenerator;
use crate::codegen::emit_event_object;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

impl OmniViewGenerator {
    /// Try to inline a known component's view HTML.
    /// Returns Some(html) if the element is a component, None otherwise.
    pub(super) fn try_inline_component(
        &mut self,
        el: &ElementIR,
        indent: usize,
        nid: &str,
        pad: &str,
    ) -> Option<String> {
        if let Some(child_comp) = self.component_map.get(&el.tag).cloned() {
            if let Some(ref child_view) = child_comp.view {
                // Save and update current component context
                let saved_component = self.current_component.clone();
                self.current_component = child_comp.name.clone();
                let saved_signal_names: FxHashSet<String> =
                    self.signal_names.iter().cloned().collect();
                let saved_renames_child: FxHashMap<String, String> = self
                    .name_renames
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                if self.in_for_template {
                    // Inside @for template: map component prop names to their
                    // value expressions so data-bind-text markers resolve to
                    // the @for loop variable (e.g. text={item} → "text" → "item").
                    // States are NOT added to signal_names — instead they go into
                    // for_state_names so expr_to_js emits element-scoped expressions
                    // (__omni_forGetState) rather than .value reads on global signals.
                    for state in &child_comp.states {
                        let prefixed = format!("{}_{}", child_comp.name, state.name);
                        let default_js = self.expr_to_js(&state.initial_value);
                        self.name_renames
                            .insert(state.name.clone(), prefixed.clone());
                        self.for_state_names
                            .insert(prefixed, (state.name.clone(), default_js));
                    }
                    for c in &child_comp.computeds {
                        let prefixed = format!("{}_{}", child_comp.name, c.name);
                        self.name_renames.insert(c.name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                    // Add child component functions so class: map and call
                    // expressions in the inlined view resolve to the correct
                    // prefixed name (e.g. isActive -> TabPanel_isActive).
                    for f in &child_comp.fns {
                        let prefixed = format!("{}_{}", child_comp.name, f.name);
                        self.name_renames.insert(f.name.clone(), prefixed);
                    }
                    // Props: map to their value expression (which references
                    // the @for loop variable), not to prefixed signal names.
                    // Do NOT add to signal_names — the resolved name (e.g. "item")
                    // is not a signal, and adding the prop name would cause
                    // expr_to_js to emit "text.value" instead of the mapped "item".
                    for (prop_name, prop_value) in &el.props {
                        let js_value = self.expr_to_js(prop_value);
                        self.name_renames.insert(prop_name.clone(), js_value);
                    }
                    // Default props that aren't passed: use prefixed names.
                    for prop_name in child_comp.prop_defaults.keys() {
                        if !el.props.contains_key(prop_name) {
                            let prefixed = format!("{}_{}", child_comp.name, prop_name);
                            self.name_renames
                                .insert(prop_name.clone(), prefixed.clone());
                            self.signal_names.insert(prefixed);
                        }
                    }
                } else {
                    // Normal (non-@for) mode: use prefixed signal names.
                    for state in &child_comp.states {
                        let prefixed = format!("{}_{}", child_comp.name, state.name);
                        self.name_renames
                            .insert(state.name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                    for c in &child_comp.computeds {
                        let prefixed = format!("{}_{}", child_comp.name, c.name);
                        self.name_renames.insert(c.name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                    // Add child component functions so class: map and call
                    // expressions in the inlined view resolve to the correct
                    // prefixed name (e.g. isActive -> TabPanel_isActive).
                    for f in &child_comp.fns {
                        let prefixed = format!("{}_{}", child_comp.name, f.name);
                        self.name_renames.insert(f.name.clone(), prefixed);
                    }
                    for prop_name in child_comp.prop_defaults.keys() {
                        let prefixed = format!("{}_{}", child_comp.name, prop_name);
                        self.name_renames
                            .insert(prop_name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                }

                // Generate child HTML inline
                let child_html = self.view_to_html(child_view, indent + 1);

                // Restore parent context
                self.current_component = saved_component;
                self.signal_names = saved_signal_names;
                self.name_renames = saved_renames_child;

                // Emit wrapper div with nid for event bindings on this component
                // For @for, add data-for-state-names for per-instance state sync
                let for_states_attr = if self.in_for_template && !child_comp.states.is_empty() {
                    let names: Vec<&str> =
                        child_comp.states.iter().map(|s| s.name.as_str()).collect();
                    format!(" data-for-state-names=\"{}\"", names.join(","))
                } else if self.in_for_template {
                    " data-for-state-names=\"\"".to_string()
                } else {
                    String::new()
                };
                let mut out = String::new();
                out.push_str(&format!(
                    "{}<div data-nid=\"{}\" data-component=\"{}\"{for_states}>\n",
                    pad,
                    nid,
                    el.tag,
                    for_states = for_states_attr
                ));
                out.push_str(&child_html);
                out.push_str(&format!("{}</div>\n", pad));

                if !self.in_for_template {
                    // Wire parent props to child signals via reactive effects.
                    // Register the child signal + emit its declaration in the parent.
                    for (prop_name, prop_value) in &el.props {
                        let js_value = self.expr_to_js(prop_value);
                        let child_prefixed = format!("{}_{}", child_comp.name, prop_name);
                        // The signal_names were just restored to the parent's set,
                        // so inserting here persists correctly.
                        if !self.signal_names.contains(&child_prefixed) {
                        // When the child component already declares this signal
                        // (via its own prop), skip the const declaration to avoid
                        // "Identifier has already been declared" errors.
                        // The child's signal already exists; we just need the effect.
                        let is_child_prop = child_comp.props.contains_key(prop_name);
                        if !is_child_prop {
                            self.signal_names.insert(child_prefixed.clone());
                            self.js.push_str(&format!(
                                "const {} = __omni_signal(\"{}\", undefined);\n",
                                child_prefixed, child_prefixed
                            ));
                        }
                        self.name_renames
                            .entry(prop_name.clone())
                            .or_insert_with(|| child_prefixed.clone());
                    }
                        self.js.push_str(&format!(
                            "__omni_effect(() => {{ __omni_subs.get(\"{0}\").set({1}); }});\n",
                            child_prefixed, js_value
                        ));
                    }

                    // Emit event bindings for the component element
                    for (event_name, handler) in &el.events {
                        let js_event = map_event(event_name);
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
                        match handler {
                            EventHandlerIR::FnRef { r#ref, .. } => {
                                if self.motion_names.contains(r#ref.as_str()) {
                                    // Motion reference: call factory and play on event
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ if (el.offsetParent !== null) {{ var p = __omni_motion_{motion}(el, {{}}); __omni_animations.set(\"{nid}_{js_event}\", p); p.play(); }} }}); }}\n",
                                        nid = nid,
                                        js_event = js_event,
                                        motion = r#ref,
                                    ));
                                } else {
                                    let prefixed = format!("{}_{}", self.current_component, r#ref);
                                    let event_obj = emit_event_object(
                                        handler.event_data(),
                                        handler.used_fields(),
                                        "e",
                                    );
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {prefixed}({event_obj}); }}); }}\n",
                                        nid = nid, js_event = js_event, sp = sp, pd = pd, prefixed = prefixed, event_obj = event_obj
                                    ));
                                }
                            }
                            EventHandlerIR::StoreAction { r#ref, .. } => {
                                let var_name = r#ref.replace('.', "_");
                                self.js.push_str(&format!(
                                    "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ {} {} {}(); }}); }}\n",
                                    nid, js_event, sp, pd, var_name
                                ));
                            }
                            EventHandlerIR::InlineFn { body, params, .. } => {
                                let wrapper_name =
                                    format!("__on{}_{}", event_name, nid.replace('-', "_"));
                                let params_str = params.join(", ");
                                self.js.push_str(&format!(
                                    "function {}({}) {{\n",
                                    wrapper_name, params_str
                                ));
                                for stmt in &body.statements {
                                    self.emit_statement(stmt);
                                }
                                self.js.push_str("}\n");
                                let event_obj = emit_event_object(
                                    handler.event_data(),
                                    handler.used_fields(),
                                    "e",
                                );
                                self.js.push_str(&format!(
                                    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {wrapper_name}({event_obj}); }}); }}\n",
                                    nid = nid, js_event = js_event, sp = sp, pd = pd, wrapper_name = wrapper_name, event_obj = event_obj
                                ));
                            }
                        }
                    }
                }

                return Some(out);
            }
        }
        None
    }
}
