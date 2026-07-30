//! List rendering: state initialization, DOM generation and event delegation for @for loops.
//!
//! The `emit_list_rendering_html` method orchestrates two phases:
//! 1. State initialization (template HTML, JS _effect header, key/empty-state setup)
//! 2. DOM generation (iteration/cloning/binding JS, event delegation)
use super::utils::collect_for_events;
use super::OmniViewGenerator;
use crate::codegen::emit_event_object;
use crate::ir::*;
use rustc_hash::FxHashMap;
use std::fmt::Write;

impl OmniViewGenerator {
    /// Generate HTML + JS for @for list rendering.
    ///
    /// Strategy:
    ///   - Generate a `<template>` element containing one item's HTML.
    ///   - Generate a `<template>` container marker (valid in any HTML context,
    ///     unlike `<div>` which is invalid inside `<tbody>`, `<tr>`, etc.).
    ///   - Runtime inserts cloned items between the source template and the marker.
    ///   - Empty state uses a `<template>` that gets cloned when data is empty.
    ///   - Generate a JS _effect() that iterates the data and clones the template.
    pub(super) fn emit_list_rendering_html(
        &mut self,
        list: &ListRenderingIntent,
        indent: usize,
    ) -> String {
        let out = self.emit_list_state_init(list, indent);
        self.emit_list_dom(list);
        out
    }

    /// State initialization portion of @for rendering: template HTML, JS _effect
    /// header, data/template/container lookups, key handling setup, empty state handling.
    /// Returns the HTML string for the template elements.
    pub(super) fn emit_list_state_init(
        &mut self,
        list: &ListRenderingIntent,
        indent: usize,
    ) -> String {
        let pad = "  ".repeat(indent);
        let nid = self.next_nid("for");
        let template_nid = format!("{}-tpl", nid);
        let container_nid = format!("{}-ctr", nid);
        let empty_nid = format!("{}-empty", nid);
        let item_var = &list.item_binding.name;
        let iterable_js = self.expr_to_js(&list.iterable);

        // Generate template for one item (with in_for_template flag)
        let item_html = if let Some(ref body) = list.body {
            // Clear per-@for-loop binding vectors so each loop gets its own fresh bindings
            self.scope.for_text_bindings.clear();
            self.scope.for_class_bindings.clear();
            self.scope.for_attr_bindings.clear();
            self.for_state_names.clear();
            // Set rebind function name for this @for loop
            let rebind_name = format!(
                "__omni_rebind_{}_{}",
                self.scope.current_component, item_var
            );
            self.scope.for_rebind_fn = Some(rebind_name);
            // Save outer @for template flag to support direct nesting.
            // Inner @for loops set in_for_template = true → false on exit,
            // which would corrupt the outer loop's template generation for
            // any content after the inner loop. Restoring the saved value
            // ensures the outer loop continues in the correct mode.
            let outer_in_for = self.scope.in_for_template;
            self.scope.in_for_template = true;
            let html = self.view_to_html(body, indent + 1);
            self.scope.in_for_template = outer_in_for;
            html
        } else {
            format!("{}  <!-- empty item -->\n", pad)
        };

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}<template data-nid=\"{}\">\n{}</template>",
            pad, template_nid, item_html
        );

        // Container marker: <template> element used as insertion point.
        // The template tag is valid inside <tbody>, <tr>, etc. (unlike <div>).
        let _ = writeln!(
            out,
            "{}<template data-nid=\"{}\" data-for-container></template>",
            pad, container_nid
        );

        // Empty state template (cloned into parent when data is empty)
        if let Some(ref empty_body) = list.empty_body {
            self.scope.in_for_template = true;
            let empty_html = self.view_to_html(empty_body, indent + 1);
            self.scope.in_for_template = false;
            let _ = writeln!(
                out,
                "{}<template data-nid=\"{}\">\n{}</template>",
                pad, empty_nid, empty_html
            );
        }

        // Generate the reactive JS _effect()
        let _ = writeln!(self.js, "\n/// ── @for {item} in ... ──", item = item_var);
        self.js.push_str("__omni_effect(() => {\n");
        let _ = writeln!(self.js, "  const data = {};", iterable_js);
        let _ = writeln!(self.js, "  const tpl = __omni_el(\"{}\");", template_nid);
        let _ = writeln!(self.js, "  const ctr = __omni_el(\"{}\");", container_nid);
        self.js.push_str("  if (!tpl || !ctr) return;\n");
        self.js.push_str("  const parent = tpl.parentNode;\n");
        self.js.push_str("  if (!parent) return;\n");

        // Re-render loop
        // If key_expr is present, use key-based identity tracking for enter/leave animations.
        // Otherwise, clear and re-render all items.
        // FLIP: record element positions BEFORE clearing (must run before removal)
        if list.key_expr.is_some() {
            self.js.push_str("  Omni.flipBefore(parent);\n");
        }
        // Clear existing rendered items (elements between tpl template and container marker)
        self.js.push_str("  // Clear existing rendered items\n");
        self.js
            .push_str("  while (tpl.nextElementSibling && tpl.nextElementSibling !== ctr) {\n");
        self.js
            .push_str("    parent.removeChild(tpl.nextElementSibling);\n");
        self.js.push_str("  }\n");

        // Key-based map
        if let Some(ref key_expr) = list.key_expr {
            let key_js = self.expr_to_js(key_expr);
            self.js
                .push_str("  const _forKeys = ctr._forKeys || (ctr._forKeys = new Map());\n");
            self.js
                .push_str("  const keyFn = (item) => { try { return String(");
            self.js.push_str(&key_js);
            self.js.push_str(
                "); } catch(e) { __omni_reportError(\"error:list:key\", e); return null; } };\n",
            );
            self.js
                .push_str("  const oldKeys = new Set(_forKeys.keys());\n");
            self.js.push_str("  const newKeys = new Set();\n");
        }

        // Handle empty state: clone empty template and insert before container marker
        if list.empty_body.is_some() {
            self.js.push_str("  if (!data || data.length === 0) {\n");
            self.js
                .push_str("    // Clone empty state template and insert before container marker\n");
            let _ = writeln!(self.js, "    const emptyEl = __omni_el(\"{}\");", empty_nid);
            self.js.push_str(
                "    if (emptyEl && emptyEl.content && emptyEl.content.firstElementChild) {\n",
            );
            self.js.push_str(
                "      const emptyClone = emptyEl.content.firstElementChild.cloneNode(true);\n",
            );
            self.js
                .push_str("      emptyClone.setAttribute('data-for-empty', 'true');\n");
            self.js
                .push_str("      parent.insertBefore(emptyClone, ctr);\n");
            self.js.push_str("    }\n");
            self.js.push_str("    return;\n");
            self.js.push_str("  }\n");
        } else {
            self.js
                .push_str("  if (!data || data.length === 0) return;\n");
        }

        out
    }

    /// DOM generation and event delegation portion of @for rendering.
    /// Called after emit_list_state_init to emit the iteration/cloning/binding JS.
    /// Depends on JS-scoped variables (`data`, `tpl`, `ctr`, `parent`, etc.)
    /// set up by emit_list_state_init in the same _effect IIFE.
    pub(super) fn emit_list_dom(&mut self, list: &ListRenderingIntent) {
        let has_key = list.key_expr.is_some();

        // Collect event types from the @for body for delegated event listener setup
        let mut for_event_types: Vec<String> = Vec::new();
        let mut for_event_kinds: Vec<(String, EventDataKind)> = Vec::new();
        let mut used_fields_by_kind: FxHashMap<String, Option<Vec<String>>> = FxHashMap::default();
        if let Some(ref body) = list.body {
            collect_for_events(
                body,
                &mut for_event_types,
                &mut for_event_kinds,
                &mut used_fields_by_kind,
            );
        }

        // Iterate and clone
        let _ = writeln!(
            self.js,
            "  data.forEach(({}, _idx) => {{",
            &list.item_binding.name
        );

        // Helper: clone template content, unwrapping <fragment> wrapper if present.
        self.js
            .push_str("    var __root = tpl.content.firstElementChild;\n");
        self.js.push_str("    var __cloneItem = function() {\n");
        self.js.push_str("      if (!__root) return null;\n");
        self.js
            .push_str("      if (__root.tagName === 'FRAGMENT') {\n");
        self.js
            .push_str("        var __f = document.createDocumentFragment();\n");
        self.js.push_str("        for (var __i = 0; __i < __root.children.length; __i++) __f.appendChild(__root.children[__i].cloneNode(true));\n");
        self.js.push_str("        return __f;\n");
        self.js.push_str("      }\n");
        self.js.push_str("      return __root.cloneNode(true);\n");
        self.js.push_str("    };\n");

        self.js.push_str("    var clone = null;\n");
        if has_key {
            self.js.push_str("    const _key = keyFn(item);\n");
            self.js
                .push_str("    if (_key != null) newKeys.add(_key);\n");
            self.js
                .push_str("    clone = _key != null ? _forKeys.get(_key) : null;\n");
            self.js.push_str("    const isNew = !clone;\n");
            self.js.push_str("    if (isNew) {\n");
            self.js.push_str("      clone = __cloneItem();\n");
            self.js.push_str("      if (clone && _key != null) { _forKeys.set(_key, clone); if (clone.nodeType === 1) clone.setAttribute('data-for-key', _key); }\n");
            self.js.push_str("    }\n");
        } else {
            self.js.push_str("    clone = __cloneItem();\n");
        }
        self.js.push_str("    if (!clone) return;\n");
        self.js
            .push_str("    // Mark first element with data-for-item\n");
        self.js.push_str("    if (clone.nodeType === 11) { if (clone.firstElementChild) clone.firstElementChild.setAttribute('data-for-item', _idx); }\n");
        self.js
            .push_str("    else { clone.setAttribute('data-for-item', _idx); }\n");

        // Cache element references for O(1) lookup in bindings
        self.js.push_str("    var _elCache = new Map();\n");
        self.js.push_str("    (function cacheElements(el) {\n");
        self.js
            .push_str("      var nid = el.getAttribute('data-nid');\n");
        self.js.push_str("      if (nid) _elCache.set(nid, el);\n");
        self.js
            .push_str("      el.querySelectorAll('[data-nid]').forEach(function(child) {\n");
        self.js
            .push_str("        _elCache.set(child.getAttribute('data-nid'), child);\n");
        self.js.push_str("      });\n");
        self.js.push_str("    })(clone);\n");

        // Pre-compiled text bindings (no new Function)
        if !self.scope.for_text_bindings.is_empty() {
            self.js.push_str("    // Pre-compiled text bindings\n");
            self.js.push_str("    (function() {\n");
            for (nid, js_expr) in &self.scope.for_text_bindings {
                let _ = writeln!(
                    self.js,
                    "      var _el = _elCache.get(\"{}\"); if (!_el) {{ _el = clone.querySelector('[data-nid=\"{}\"]'); if (_el) _elCache.set(\"{}\", _el); }}",
                    nid, nid, nid
                );
                let _ = writeln!(self.js, "      if (_el) try {{ _el.textContent = String({}); }} catch(e) {{ __omni_reportError(\"warn:for:text\", e); }}", js_expr);
            }
            self.js.push_str("    })();\n");
        }

        // Initialize per-instance state for @for component instances
        self.js
            .push_str("    // Initialize per-instance state for @for component instances\n");
        self.js.push_str("    var _stateEls = [];\n");
        self.js.push_str("    if (clone.matches && clone.matches('[data-for-state-names]')) _stateEls.push(clone);\n");
        self.js.push_str("    clone.querySelectorAll('[data-for-state-names]').forEach(function(el) { _stateEls.push(el); });\n");
        self.js.push_str("    _stateEls.forEach(function(_fc) {\n");
        self.js.push_str("      if (!_fc.__omniState) {\n");
        self.js.push_str("        _fc.__omniState = {};\n");
        self.js
            .push_str("        var _snStr = _fc.getAttribute('data-for-state-names');\n");
        self.js.push_str("        if (_snStr) {\n");
        self.js
            .push_str("          _snStr.split(',').forEach(function(_sn) {\n");
        self.js.push_str("            if (_sn) {\n");
        self.js
            .push_str("              var _cn = _fc.getAttribute('data-component');\n");
        self.js
            .push_str("              var _sig = __omni_subs.get(_cn + '_' + _sn);\n");
        self.js.push_str("              if (_sig) _fc.__omniState[_sn] = _sig._default != null ? _sig._default : _sig.peek();\n");
        self.js.push_str("            }\n");
        self.js.push_str("          });\n");
        self.js.push_str("        }\n");
        self.js.push_str("      }\n");
        self.js.push_str("    });\n");

        // Pre-compiled class bindings (no new Function)
        if !self.scope.for_class_bindings.is_empty() {
            self.js.push_str("    // Pre-compiled class bindings\n");
            self.js.push_str("    (function() {\n");
            for (nid, class_name, js_expr) in &self.scope.for_class_bindings {
                let _ = writeln!(
                    self.js,
                    "      var _el = _elCache.get(\"{}\"); if (!_el) {{ _el = clone.querySelector('[data-nid=\"{}\"]'); if (_el) _elCache.set(\"{}\", _el); }}",
                    nid, nid, nid
                );
                let _ = writeln!(self.js, "      if (_el) try {{ _el.classList.toggle(\"{}\", !!({})); }} catch(e) {{ __omni_reportError(\"warn:for:class\", e); }}", class_name, js_expr);
            }
            self.js.push_str("    })();\n");
        }

        // Pre-compiled attribute bindings (no new Function)
        if !self.scope.for_attr_bindings.is_empty() {
            use super::AttrPart;
            self.js.push_str("    // Pre-compiled attribute bindings\n");
            self.js.push_str("    (function() {\n");
            for (nid, attr_name, parts) in &self.scope.for_attr_bindings {
                let mut val_parts: Vec<String> = Vec::new();
                for part in parts {
                    match part {
                        AttrPart::Static(s) => {
                            val_parts.push(format!("\"{}\"", s));
                        }
                        AttrPart::Dynamic(expr) => {
                            val_parts.push(format!("String({})", expr));
                        }
                    }
                }
                let resolved = if val_parts.len() == 1 {
                    val_parts[0].clone()
                } else {
                    format!("({})", val_parts.join(" + "))
                };
                let _ = writeln!(self.js, "      var _el = _elCache.get(\"{}\"); if (!_el) {{ _el = clone.querySelector('[data-nid=\"{}\"]'); if (_el) _elCache.set(\"{}\", _el); }} if (_el) try {{ _el.setAttribute(\"{}\", {}); }} catch(e) {{ __omni_reportError(\"warn:for:attr\", e); }}", nid, nid, nid, attr_name, resolved);
            }
            self.js.push_str("    })();\n");
        }

        if has_key {
            self.js
                .push_str("    if (isNew) { parent.insertBefore(clone, ctr); }\n");
        } else {
            self.js.push_str("    parent.insertBefore(clone, ctr);\n");
        }
        self.js.push_str("  });\n");

        // Key-based cleanup: remove items whose keys disappeared
        if has_key {
            self.js
                .push_str("  // Remove items whose keys disappeared (play @leave then remove)\n");
            self.js.push_str("  oldKeys.forEach(key => {\n");
            self.js.push_str("    if (!newKeys.has(key)) {\n");
            self.js.push_str("      const el = _forKeys.get(key);\n");
            self.js.push_str("      if (el) {\n");
            self.js
                .push_str("        const leavingNid = el.getAttribute('data-nid');\n");
            self.js.push_str("        if (leavingNid) {\n");
            self.js.push_str(
                "          const player = __omni_animations.get(leavingNid + '_leave');\n",
            );
            self.js.push_str("          if (player) {\n");
            self.js.push_str(
                "            player.play().then(() => { __omni_cleanupElement(leavingNid); el.remove(); _forKeys.delete(key); });\n",
            );
            self.js.push_str("          } else {\n");
            self.js
                .push_str("            __omni_cleanupElement(leavingNid); el.remove(); _forKeys.delete(key);\n");
            self.js.push_str("          }\n");
            self.js.push_str("        } else {\n");
            self.js
                .push_str("          el.remove(); _forKeys.delete(key);\n");
            self.js.push_str("        }\n");
            self.js.push_str("      }\n");
            self.js.push_str("    }\n");
            self.js.push_str("  });\n");
        }

        // FLIP after: animate position changes for surviving items
        if has_key {
            self.js.push_str("  Omni.flipAfter(parent);\n");
        }

        // Attach delegated event listeners on the parent for @for item events
        if !for_event_types.is_empty() {
            self.js
                .push_str("  if (!ctr.dataset.forEventsAttached) {\n");
            self.js
                .push_str("    ctr.dataset.forEventsAttached = 'true';\n");

            // Emit event-kind lookup map
            self.js.push_str("    const _eventKinds = {\n");
            for (js_event, kind) in &for_event_kinds {
                let _ = writeln!(
                    self.js,
                    "      '{}': '{}',",
                    js_event,
                    event_data_kind_label(kind)
                );
            }
            self.js.push_str("    };\n");

            // Emit _extractEvent switch function
            self.js.push_str("    function _extractEvent(kind, e) {\n");
            self.js.push_str("      switch (kind) {\n");
            for (_, kind) in &for_event_kinds {
                let label = event_data_kind_label(kind);
                let used = used_fields_by_kind.get(label).unwrap_or(&None);
                let _ = writeln!(self.js, "        case '{}':", label);
                let _ = writeln!(
                    self.js,
                    "          return {};",
                    emit_event_object(&Some(kind.clone()), used, "e")
                );
            }
            self.js.push_str("        default: return {};\n");
            self.js.push_str("      }\n");
            self.js.push_str("    }\n");

            self.js.push_str("    const _forHandle = (e) => {\n");
            self.js
                .push_str("      const t = e.target.closest('[data-event-' + e.type + ']');\n");
            self.js.push_str("      if (!t) return;\n");
            self.js
                .push_str("      const attr = t.getAttribute('data-event-' + e.type);\n");
            self.js.push_str("      if (!attr) return;\n");
            self.js.push_str(
                "      const _flags = attr.includes('|') ? attr.split('|').slice(1) : [];\n",
            );
            self.js
                .push_str("      if (_flags.includes('pd')) e.preventDefault();\n");
            self.js
                .push_str("      if (_flags.includes('sp')) e.stopPropagation();\n");
            self.js.push_str("      if (attr.startsWith('fn:')) {\n");
            self.js.push_str(
                "        const fnKey = attr.includes('|') ? attr.slice(3, attr.indexOf('|')) : attr.slice(3);\n",
            );
            self.js
                .push_str("        const fn = __omni_forHandlers[fnKey];\n");
            self.js.push_str(
                "        // Before calling fn, restore per-instance state to global signal\n",
            );
            self.js.push_str(
                "        // so the fn handler operates on the correct per-instance value\n",
            );
            self.js
                .push_str("        const _forComp = t.closest('[data-for-state-names]');\n");
            self.js
                .push_str("        if (_forComp && _forComp.__omniState) {\n");
            self.js.push_str(
                "          var _stateNames = _forComp.getAttribute('data-for-state-names');\n",
            );
            self.js.push_str("          if (_stateNames) {\n");
            self.js
                .push_str("            _stateNames.split(',').forEach(function(_sname) {\n");
            self.js
                .push_str("              if (_sname && _sname in _forComp.__omniState) {\n");
            self.js.push_str(
                "                var _compName = _forComp.getAttribute('data-component');\n",
            );
            self.js.push_str(
                "                var _sig = __omni_subs.get(_compName + '_' + _sname);\n",
            );
            self.js
                .push_str("                if (_sig) _sig.set(_forComp.__omniState[_sname]);\n");
            self.js.push_str("              }\n");
            self.js.push_str("            });\n");
            self.js.push_str("          }\n");
            self.js.push_str("        }\n");
            self.js.push_str("        if (typeof fn === 'function') fn(_extractEvent(_eventKinds[e.type] || '', e));\n");
            self.js.push_str("        // Sync global signals back to element-scoped state for @for component instances\n");
            self.js.push_str("        if (_forComp) {\n");
            self.js
                .push_str("          if (!_forComp.__omniState) _forComp.__omniState = {};\n");
            self.js.push_str(
                "          var _stateNames = _forComp.getAttribute('data-for-state-names');\n",
            );
            self.js.push_str("          if (_stateNames) {\n");
            self.js
                .push_str("            _stateNames.split(',').forEach(function(_sname) {\n");
            self.js.push_str("              if (_sname) {\n");
            self.js.push_str(
                "                var _compName = _forComp.getAttribute('data-component');\n",
            );
            self.js.push_str(
                "                var _sig = __omni_subs.get(_compName + '_' + _sname);\n",
            );
            self.js.push_str(
                "                if (_sig) _forComp.__omniState[_sname] = _sig.peek();\n",
            );
            self.js.push_str("              }\n");
            self.js.push_str("            });\n");
            self.js.push_str("          }\n");
            self.js.push_str(
                "          // Re-evaluate text bindings from element-scoped state (pre-compiled)\n",
            );
            if let Some(ref rebind_fn) = self.scope.for_rebind_fn {
                let _ = writeln!(
                    self.js,
                    "          if (typeof {} === 'function') {}(_forComp);",
                    rebind_fn, rebind_fn
                );
            }
            self.js.push_str("        }\n");
            self.js
                .push_str("      } else if (attr.startsWith('store:')) {\n");
            self.js
                .push_str("        const fn = __omni_forHandlers[attr.slice(6)];\n");
            self.js
                .push_str("        if (typeof fn === 'function') fn();\n");
            self.js.push_str("      }\n");
            self.js.push_str("    };\n");

            // Dynamic subscription: iterate over collected event types
            self.js.push_str("    [");
            for (i, evt) in for_event_types.iter().enumerate() {
                if i > 0 {
                    self.js.push_str(", ");
                }
                let _ = write!(self.js, "'{}'", evt);
            }
            self.js.push_str("].forEach(function(evt) {\n");
            self.js
                .push_str("      parent.addEventListener(evt, _forHandle);\n");
            self.js.push_str("    });\n");

            self.js.push_str("  }\n");
        }

        // Emit pre-compiled rebind function for text bindings (no new Function)
        if !self.scope.for_text_bindings.is_empty() {
            if let Some(ref rebind_fn) = self.scope.for_rebind_fn {
                let _ = writeln!(self.js, "  // Pre-compiled rebind function: {}", rebind_fn);
                let _ = writeln!(self.js, "  function {}(comp) {{", rebind_fn);
                for (nid, js_expr) in &self.scope.for_text_bindings {
                    let _ = writeln!(
                        self.js,
                        "    var _el = comp.querySelector('[data-nid=\"{}\"]');",
                        nid
                    );
                    let _ = writeln!(self.js, "    if (_el) try {{ _el.textContent = String({}); }} catch(e) {{ __omni_reportError(\"warn:for:rebind\", e); }}", js_expr);
                }
                self.js.push_str("  }\n");
            }
        }

        self.js.push_str("});\n");
    }
}
