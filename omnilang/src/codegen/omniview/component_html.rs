use super::utils::map_event;
use super::InliningState;
use super::OmniViewGenerator;
use crate::ir::*;
use rustc_hash::FxHashSet;
use std::sync::Arc;

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
        self.assert_can_emit_js();
        if let Some(child_comp) = self.component_map.get(&el.tag).map(Arc::clone) {
            if let Some(ref child_view) = child_comp.view {
                // Save and update current component context
                let saved_state = InliningState::save(self);
                self.current_component = child_comp.name.clone();

                if self.in_for_template {
                    // Inside @for template: map component prop names to their
                    // value expressions so data-bind-text markers resolve to
                    // the @for loop variable (e.g. text={item} → "text" → "item").
                    // States are NOT added to signal_names — instead they go into
                    // for_state_names so expr_to_js emits element-scoped expressions
                    // (__omni_forGetState) rather than .value reads on global signals.
                    for state in &child_comp.signals {
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
                    for state in &child_comp.signals {
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

                // Extract owned data from child_comp before the mutable borrow
                // in view_to_html, releasing the borrow on self.component_map.
                let child_comp_name = child_comp.name.clone();
                let child_prop_keys: FxHashSet<String> = child_comp.props.keys().cloned().collect();
                let for_states_attr = if self.in_for_template && !child_comp.signals.is_empty() {
                    let names: Vec<&str> =
                        child_comp.signals.iter().map(|s| s.name.as_str()).collect();
                    format!(" data-for-state-names=\"{}\"", names.join(","))
                } else if self.in_for_template {
                    " data-for-state-names=\"\"".to_string()
                } else {
                    String::new()
                };
                // Extract parent-submitted children for slot projection
                let slot_children: Vec<ViewTree> = el.children.clone();

                // Clone the view tree to break the borrow on self
                // Apply slot injection if parent passed children
                let child_view_owned = if slot_children.is_empty() {
                    child_view.clone()
                } else {
                    inject_slot_children(child_view.clone(), &slot_children)
                };

                // Generate child HTML inline
                let child_html = self.view_to_html(&child_view_owned, indent + 1);

                // Restore all saved state (for-loop state + parent context)
                saved_state.restore(self);

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
                        let child_prefixed = format!("{}_{}", child_comp_name, prop_name);
                        // The signal_names were just restored to the parent's set,
                        // so inserting here persists correctly.
                        if !self.signal_names.contains(&child_prefixed) {
                            // When the child component already declares this signal
                            // (via its own prop), skip the const declaration to avoid
                            // "Identifier has already been declared" errors.
                            // The child's signal already exists; we just need the effect.
                            let is_child_prop = child_prop_keys.contains(prop_name);
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
                        self.emit_event_listener(nid, &js_event, event_name, handler, sp, pd);
                    }
                }

                return Some(out);
            }
        }
        None
    }
}

/// Recursively maps parent children into <slot> elements in the view tree.
/// Top-level children go to the default (unnamed) slot; children with
/// slot="name" attributes go to named slots.
fn inject_slot_children(view: ViewTree, slot_children: &[ViewTree]) -> ViewTree {
    match view {
        ViewTree::Slot(ref slot_name) => {
            if let Some(name) = slot_name.as_deref() {
                let named: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| has_slot_attr(c, name))
                    .cloned()
                    .collect();
                if named.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(named)
                }
            } else {
                let default: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| !has_any_slot_attr(c))
                    .cloned()
                    .collect();
                if default.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(default)
                }
            }
        }
        ViewTree::Element(el) => {
            let new_children: Vec<ViewTree> = el
                .children
                .iter()
                .map(|c| inject_slot_children(c.clone(), slot_children))
                .collect();
            ViewTree::Element(ElementIR {
                children: new_children,
                ..el
            })
        }
        ViewTree::Conditional(cond) => {
            let new_branches = cond
                .branches
                .into_iter()
                .map(|b| ConditionalBranch {
                    body: b.body.map(|v| inject_slot_children(v, slot_children)),
                    ..b
                })
                .collect();
            ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: new_branches,
                ..*cond
            }))
        }
        ViewTree::ListRendering(..) => view, // @for templates: slots stay as-is
        other => other,
    }
}

fn has_slot_attr(child: &ViewTree, slot_name: &str) -> bool {
    match child {
        ViewTree::Element(el) => el
            .props
            .get("slot")
            .and_then(|v| match v {
                ExprIR::LiteralExpr { value } => value.as_str(),
                _ => None,
            })
            .map(|s| s == slot_name)
            .unwrap_or(false),
        _ => false,
    }
}

fn has_any_slot_attr(child: &ViewTree) -> bool {
    match child {
        ViewTree::Element(el) => el.props.contains_key("slot"),
        _ => false,
    }
}
