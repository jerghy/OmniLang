use super::expr::is_fn_type;
use super::scope::ComponentScope;
use super::utils::map_event;
use super::OmniViewGenerator;
use crate::ir::*;
use rustc_hash::FxHashMap;
use std::fmt::Write;
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
        if let Some(child_comp) = self
            .name_resolver
            .component_map
            .get(&el.tag)
            .map(Arc::clone)
        {
            if let Some(ref child_view) = child_comp.view {
                // Save and update current component context
                let saved_scope = ComponentScope::take(self);
                // Preserve @for template context — take() resets in_for_template
                // to false, but child components inside @for need it true so that
                // interpolations (e.g. {text}) generate data-bind-text markers
                // and the @for prop→expression mapping (below) works correctly.
                // for_item_var/rewrite_for_item must follow the same rule: the
                // inlined child's event handler bodies still compile to top-level
                // JS where the outer loop variable is out of scope.
                self.scope.in_for_template = saved_scope.in_for_template;
                self.scope.for_item_var = saved_scope.for_item_var.clone();
                self.scope.rewrite_for_item = saved_scope.rewrite_for_item;
                self.scope.current_component = child_comp.name.clone();
                self.scope.form_intent = child_comp.form_intent.clone();
                self.scope.load_intent = child_comp.load_intent.clone();
                // @load/@form builtin reserved names (loading/error/data/retry,
                // formErrors/formValid/formSubmitting) must resolve to the
                // child's prefixed signals in the inlined view — the same
                // registration generate_component does for top-level
                // components. Without it, `@if loading` / `{error}` /
                // `@for x in data` in a nested @load component emit bare
                // identifiers and throw ReferenceError at runtime.
                self.register_builtin_intent_names(child_comp.as_ref());

                if self.scope.in_for_template {
                    // fn-typed props of the child must be callable in the
                    // inlined view ({transform("hello")}) — mark them so
                    // CallExpr emission derefs .value before invoking.
                    for (prop_name, ty) in &child_comp.props {
                        if is_fn_type(ty) {
                            self.name_resolver.fn_prop_names.insert(prop_name.clone());
                        }
                    }
                    // Inside @for template: map component prop names to their
                    // value expressions so data-bind-text markers resolve to
                    // the @for loop variable (e.g. text={item} → "text" → "item").
                    // States are NOT added to signal_names — instead they go into
                    // for_state_names so expr_to_js emits element-scoped expressions
                    // (__omni_forGetState) rather than .value reads on global signals.
                    for state in &child_comp.signals {
                        let prefixed = format!("{}_{}", child_comp.name, state.name);
                        let default_js = self.expr_to_js(&state.initial_value);
                        self.name_resolver
                            .name_renames
                            .insert(state.name.clone(), prefixed.clone());
                        self.name_resolver
                            .for_state_names
                            .insert(prefixed, (state.name.clone(), default_js));
                    }
                    for c in &child_comp.computeds {
                        let prefixed = format!("{}_{}", child_comp.name, c.name);
                        self.name_resolver
                            .name_renames
                            .insert(c.name.clone(), prefixed.clone());
                        self.name_resolver.signal_names.insert(prefixed);
                    }
                    // Add child component functions so class: map and call
                    // expressions in the inlined view resolve to the correct
                    // prefixed name (e.g. isActive -> TabPanel_isActive).
                    for f in &child_comp.fns {
                        let prefixed = format!("{}_{}", child_comp.name, f.name);
                        self.name_resolver
                            .name_renames
                            .insert(f.name.clone(), prefixed);
                    }
                    // Props: map to their value expression (which references
                    // the @for loop variable), not to prefixed signal names.
                    // Do NOT add to signal_names — the resolved name (e.g. "item")
                    // is not a signal, and adding the prop name would cause
                    // expr_to_js to emit "text.value" instead of the mapped "item".
                    for (prop_name, prop_value) in &el.props {
                        let js_value = self.expr_to_js(prop_value);
                        self.name_resolver
                            .name_renames
                            .insert(prop_name.clone(), js_value);
                    }
                    // Default props that aren't passed: use prefixed names.
                    for prop_name in child_comp.prop_defaults.keys() {
                        if !el.props.contains_key(prop_name) {
                            let prefixed = format!("{}_{}", child_comp.name, prop_name);
                            self.name_resolver
                                .name_renames
                                .insert(prop_name.clone(), prefixed.clone());
                            self.name_resolver.signal_names.insert(prefixed);
                        }
                    }
                } else {
                    // Normal (non-@for) mode: use prefixed signal names.
                    for (prop_name, ty) in &child_comp.props {
                        if is_fn_type(ty) {
                            self.name_resolver.fn_prop_names.insert(prop_name.clone());
                        }
                    }
                    for state in &child_comp.signals {
                        let prefixed = format!("{}_{}", child_comp.name, state.name);
                        self.name_resolver
                            .name_renames
                            .insert(state.name.clone(), prefixed.clone());
                        self.name_resolver.signal_names.insert(prefixed);
                    }
                    for c in &child_comp.computeds {
                        let prefixed = format!("{}_{}", child_comp.name, c.name);
                        self.name_resolver
                            .name_renames
                            .insert(c.name.clone(), prefixed.clone());
                        self.name_resolver.signal_names.insert(prefixed);
                    }
                    // Add store signal/getter names for .value access when the
                    // child component references stores (e.g. {TodoStore.totalCount}).
                    // store_signal_names is populated globally during emit_store.
                    for store_name in &child_comp.store_refs {
                        let prefix = format!("{}_", store_name);
                        for name in &self.name_resolver.store_signal_names {
                            if name.starts_with(&prefix) {
                                self.name_resolver.signal_names.insert(name.clone());
                            }
                        }
                    }
                    // Add child component functions so class: map and call
                    // expressions in the inlined view resolve to the correct
                    // prefixed name (e.g. isActive -> TabPanel_isActive).
                    for f in &child_comp.fns {
                        let prefixed = format!("{}_{}", child_comp.name, f.name);
                        self.name_resolver
                            .name_renames
                            .insert(f.name.clone(), prefixed);
                    }
                    for (prop_name, default_value) in &child_comp.prop_defaults {
                        // Use per-instance signal names (based on nid) so that
                        // multiple instances of the same component each have
                        // their own prop storage. Without this, all instances
                        // share one module-level signal and only the last
                        // instance's value survives (the Overview page's 6
                        // CapabilityCard bug).
                        //
                        // Emit the const declaration BEFORE view_to_html so that
                        // the child's view effects (which reference e.g.
                        // Comp_prop_nid.value) don't hit the JS temporal dead zone.
                        let prefixed = format!(
                            "{}_{}_{}",
                            child_comp.name,
                            prop_name,
                            nid.replace("-", "_")
                        );
                        let default_js = self.expr_to_js(default_value);
                        let _ = writeln!(
                            self.js,
                            "const {} = __omni_signal(\"{}\", {});",
                            prefixed, prefixed, default_js
                        );
                        self.name_resolver
                            .name_renames
                            .insert(prop_name.clone(), prefixed.clone());
                        self.name_resolver.signal_names.insert(prefixed);
                    }
                }

                // Merge parent renames into child scope so slot content
                // referencing parent signals (e.g. {count}) resolves to the
                // parent's prefixed signal name (SignalPatternsDemo_count.value)
                // rather than a bare `count` identifier.
                for (name, prefixed) in &saved_scope.name_renames {
                    if !self.name_resolver.name_renames.contains_key(name.as_str()) {
                        self.name_resolver
                            .name_renames
                            .insert(name.clone(), prefixed.clone());
                        if saved_scope.signal_names.contains(prefixed.as_str()) {
                            self.name_resolver.signal_names.insert(prefixed.clone());
                        }
                    }
                }

                // Extract owned data from child_comp before the mutable borrow
                // in view_to_html, releasing the borrow on self.name_resolver.component_map.
                let child_comp_name = child_comp.name.clone();
                let for_states_attr =
                    if self.scope.in_for_template && !child_comp.signals.is_empty() {
                        let names: Vec<&str> =
                            child_comp.signals.iter().map(|s| s.name.as_str()).collect();
                        format!(" data-for-state-names=\"{}\"", names.join(","))
                    } else if self.scope.in_for_template {
                        " data-for-state-names=\"\"".to_string()
                    } else {
                        String::new()
                    };
                // ADR-053 D5: body-level `@slot` declarations surface in the
                // view as `{name}` interpolations — materialize them into
                // `ViewTree::Slot` nodes (carrying the declared fallback) so
                // slot projection and fallback rendering below work
                // uniformly with view-level slots. Fast path: children with
                // no body slots keep the original view (no tree clone).
                let child_html = if el.children.is_empty() {
                    // Fast path: no slot children — render the view directly.
                    let view: &ViewTree = if child_comp.slots.is_empty() {
                        child_view
                    } else {
                        &materialize_view_slots(
                            child_view,
                            &child_comp,
                            &slot_collision_names(&child_comp),
                        )
                    };
                    self.view_to_html(view, indent + 1)
                } else {
                    let slot_children: Vec<ViewTree> = el.children.clone();
                    let child_view_owned = if child_comp.slots.is_empty() {
                        child_view.clone()
                    } else {
                        materialize_view_slots(
                            child_view,
                            &child_comp,
                            &slot_collision_names(&child_comp),
                        )
                    };
                    let child_view_injected = inject_slot_children(
                        child_view_owned,
                        &slot_children,
                        &self.name_resolver.component_map,
                    );
                    self.view_to_html(&child_view_injected, indent + 1)
                };

                // Save child's for-loop bindings before restoring parent scope,
                // then merge them back so @for template bindings accumulated by
                // the child component (e.g. {text} → item.text inside TodoItem)
                // are not lost when the parent scope replaces the child's.
                let child_text_bindings = std::mem::take(&mut self.scope.for_text_bindings);
                let child_class_bindings = std::mem::take(&mut self.scope.for_class_bindings);
                let child_attr_bindings = std::mem::take(&mut self.scope.for_attr_bindings);
                // Same for the delegated-event collection: events inside the
                // inlined child (e.g. the TodoItem ✕ button's click) must be
                // merged into the outer @for's listener set, otherwise the
                // click is never delegated and the button is dead.
                let child_event_types = std::mem::take(&mut self.scope.for_event_types);
                let child_event_kinds = std::mem::take(&mut self.scope.for_event_kinds);
                let child_used_fields = std::mem::take(&mut self.scope.used_fields_by_kind);

                // Restore all saved state (for-loop state + parent context)
                saved_scope.restore(self);

                // Merge child's for-loop bindings back into parent scope
                if self.scope.in_for_template {
                    self.scope.for_text_bindings.extend(child_text_bindings);
                    self.scope.for_class_bindings.extend(child_class_bindings);
                    self.scope.for_attr_bindings.extend(child_attr_bindings);
                    for evt in child_event_types {
                        if !self.scope.for_event_types.contains(&evt) {
                            self.scope.for_event_types.push(evt);
                        }
                    }
                    self.scope.for_event_kinds.extend(child_event_kinds);
                    for (label, fields) in child_used_fields {
                        match fields {
                            Some(fields) => {
                                let entry = self
                                    .scope
                                    .used_fields_by_kind
                                    .entry(label)
                                    .or_insert_with(|| Some(Vec::new()));
                                if let Some(accumulated) = entry.as_mut() {
                                    for field in fields {
                                        if !accumulated.contains(&field) {
                                            accumulated.push(field);
                                        }
                                    }
                                }
                            }
                            None => {
                                self.scope.used_fields_by_kind.insert(label, None);
                            }
                        }
                    }
                }

                // Inside @for, emit events dispatched by the child (its
                // onXxx → CustomEvent on this root div) need to reach the
                // loop's delegated listener on the parent. Encode each event
                // binding as data-emit-{js_event}="fn:{parent}_{handler}" so
                // _forHandle can route the bubbling CustomEvent back to the
                // parent's handler (e.g. TodoItem's 'delete' → TodoList_handleDelete).
                // Note: runs after saved_scope.restore(self), so
                // current_component is the PARENT component's name here.
                let mut emit_attrs = String::new();
                if self.scope.in_for_template && !el.events.is_empty() {
                    let parent_comp = self.scope.current_component.clone();
                    for (event_name, handler) in &el.events {
                        let js_event = map_event(event_name);
                        if let EventHandlerIR::FnRef { r#ref, .. } = handler {
                            let fn_key = format!("{}_{}", parent_comp, r#ref);
                            let _ = write!(emit_attrs, " data-emit-{}=\"fn:{}\"", js_event, fn_key);
                            // The component-instance event binding is consumed
                            // here (try_inline_component never reaches
                            // emit_element_events), so the handler must be
                            // registered in the delegated lookup explicitly —
                            // and the event type added to the delegated
                            // listener set (a bubbling CustomEvent from the
                            // child root must be caught by the parent loop).
                            if !self.scope.for_event_types.contains(&js_event) {
                                self.scope.for_event_types.push(js_event.clone());
                            }
                            let _ = writeln!(
                                self.js,
                                "__omni_forHandlers[\"{}\"] = {};",
                                fn_key, fn_key
                            );
                        }
                    }
                }

                let mut out = String::new();
                let _ = writeln!(
                    out,
                    "{}<div data-nid=\"{}\" data-component=\"{}\"{for_states}{emit_attrs}>",
                    pad,
                    nid,
                    el.tag,
                    for_states = for_states_attr,
                    emit_attrs = emit_attrs
                );
                out.push_str(&child_html);
                let _ = writeln!(out, "{}</div>", pad);

                if !self.scope.in_for_template {
                    // Wire parent props to child signals via reactive effects.
                    // Use per-instance signal names (based on element nid) so
                    // that multiple component instances each have their own
                    // prop storage instead of all sharing one module-level signal.
                    // Signal const declarations were already emitted by the
                    // prop_defaults mapping above, BEFORE view_to_html.
                    let instance_suffix = format!("_{}", nid.replace("-", "_"));
                    for (prop_name, prop_value) in &el.props {
                        let js_value = self.expr_to_js(prop_value);
                        let child_prefixed =
                            format!("{}_{}{}", child_comp_name, prop_name, instance_suffix);
                        let _ = writeln!(
                            self.js,
                            "__omni_effect(() => {{ __omni_subs.get(\"{0}\").set({1}); }});",
                            child_prefixed, js_value
                        );
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

    /// Register the @load/@form builtin reserved names of a child component
    /// (loading/error/data/retry, formErrors/formValid/formSubmitting) into
    /// `name_renames`/`signal_names` so inlined-view expressions resolve them
    /// to the child's prefixed signals (`Child_loading.value`), mirroring the
    /// registration `generate_component` performs for top-level components.
    ///
    /// These names live in the semantic reserved-name table (validate_form_load)
    /// and are NOT part of `ComponentIR.signals`, so the generic signal loop
    /// above would silently skip them — leaving `@if loading` to lower to a
    /// bare `loading` identifier that throws ReferenceError at runtime.
    fn register_builtin_intent_names(&mut self, comp: &ComponentIR) {
        if comp.load_intent.is_some() {
            for (bare, kind) in [
                ("loading", IntentReservedKind::Signal),
                ("error", IntentReservedKind::Signal),
                ("data", IntentReservedKind::Signal),
                ("retry", IntentReservedKind::Fn),
            ] {
                let prefixed = format!("{}_{}", comp.name, bare);
                self.name_resolver
                    .name_renames
                    .insert(bare.to_string(), prefixed.clone());
                if matches!(kind, IntentReservedKind::Signal) {
                    self.name_resolver.signal_names.insert(prefixed);
                }
            }
        }
        if comp.form_intent.is_some() {
            for bare in ["formErrors", "formSubmitting", "formValid"] {
                let prefixed = format!("{}_{}", comp.name, bare);
                self.name_resolver
                    .name_renames
                    .insert(bare.to_string(), prefixed.clone());
                self.name_resolver.signal_names.insert(prefixed);
            }
        }
    }
}

/// Kind of an intent-reserved builtin name — signals read via `.value`,
/// fns invoked as plain identifiers.
enum IntentReservedKind {
    Signal,
    Fn,
}

/// Recursively maps parent children into <slot> elements in the view tree.
/// Top-level children go to the default (unnamed) slot; children with
/// slot="name" attributes — or consumer-position `@slot name { ... }` /
/// `name={<.../>}` projections (both `ViewTree::Slot` children) — go to
/// named slots. Children of a nested component instantiation are that
/// component's own projections, not slot positions of this view — their
/// injection happens when the nested component is inlined.
fn inject_slot_children(
    view: ViewTree,
    slot_children: &[ViewTree],
    component_map: &FxHashMap<String, Arc<ComponentIR>>,
) -> ViewTree {
    match view {
        ViewTree::Slot {
            name: ref slot_name,
            ..
        } => {
            if let Some(name) = slot_name.as_deref() {
                let named: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| has_slot_attr(c, name))
                    .map(strip_slot_prop)
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
                    .map(strip_slot_prop)
                    .collect();
                if default.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(default)
                }
            }
        }
        ViewTree::Element(el) => {
            if component_map.contains_key(el.tag.as_str()) {
                // Children of a nested component instantiation are ITS slot
                // projections, handled when that component is inlined.
                return ViewTree::Element(el);
            }
            let new_children: Vec<ViewTree> = el
                .children
                .iter()
                .map(|c| inject_slot_children(c.clone(), slot_children, component_map))
                .collect();
            ViewTree::Element(Box::new(ElementIR {
                children: new_children,
                ..*el
            }))
        }
        ViewTree::Conditional(cond) => {
            let new_branches = cond
                .branches
                .into_iter()
                .map(|b| ConditionalBranch {
                    body: b
                        .body
                        .map(|v| inject_slot_children(v, slot_children, component_map)),
                    ..b
                })
                .collect();
            let fallback = cond
                .fallback
                .map(|v| inject_slot_children(v, slot_children, component_map));
            ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: new_branches,
                fallback,
                transition_mode: cond.transition_mode,
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
        // Consumer-position `@slot name { ... }` / `name={<.../>}`
        // projections (ADR-053 D5) are ViewTree::Slot children.
        ViewTree::Slot { name, .. } => name.as_deref() == Some(slot_name),
        _ => false,
    }
}

fn has_any_slot_attr(child: &ViewTree) -> bool {
    match child {
        ViewTree::Element(el) => el.props.contains_key("slot"),
        ViewTree::Slot { name: Some(_), .. } => true,
        _ => false,
    }
}

/// Remove the `slot` prop from an element ViewTree so browsers don't
/// interpret it as a Web Components slot assignment attribute.
fn strip_slot_prop(view: &ViewTree) -> ViewTree {
    match view {
        ViewTree::Element(el) => {
            if el.props.contains_key("slot") {
                let mut props = el.props.clone();
                props.remove("slot");
                ViewTree::Element(Box::new(ElementIR {
                    props,
                    ..*el.clone()
                }))
            } else {
                view.clone()
            }
        }
        // A consumer-position `@slot name { ... }` / `name={<.../>}`
        // projection (ADR-053 D5) is a ViewTree::Slot whose fallback IS the
        // projected content — unwrap it so the content renders directly in
        // the slot position (mirrors the React/Flutter backends).
        ViewTree::Slot { fallback, .. } => ViewTree::Fragment(fallback.clone()),
        other => other.clone(),
    }
}
