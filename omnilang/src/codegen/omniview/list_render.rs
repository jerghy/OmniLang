//! List rendering: DOM generation and event delegation for @for loops.
//!
//! This module contains the iterative/cloning/binding portion of the @for
//! render effect. The state initialization (template HTML, JS _effect header,
//! key/empty-state setup) is in list.rs.
use super::utils::collect_for_events;
use super::OmniViewGenerator;
use crate::codegen::emit_event_object;
use crate::ir::*;

impl OmniViewGenerator {
    /// DOM generation and event delegation portion of @for rendering.
    /// Called after emit_list_state_init to emit the iteration/cloning/binding JS.
    /// Depends on JS-scoped variables (`data`, `tpl`, `ctr`, `parent`, etc.)
    /// set up by emit_list_state_init in the same _effect IIFE.
    pub(super) fn emit_list_dom(&mut self, list: &ListRenderingIntent) {
        let has_key = list.key_expr.is_some();

        // Collect event types from the @for body for delegated event listener setup
        let mut for_event_types: Vec<String> = Vec::new();
        let mut for_event_kinds: Vec<(String, EventDataKind)> = Vec::new();
        if let Some(ref body) = list.body {
            collect_for_events(body, &mut for_event_types, &mut for_event_kinds);
        }

        // FLIP: record element positions before re-render (for @for with key)
        if has_key {
            self.js
                .push_str("  Omni.flipBefore(parent);\n");
        }

        // Iterate and clone
        self.js.push_str(&format!(
            "  data.forEach(({}, _idx) => {{\n",
            &list.item_binding.name
        ));

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

        // Pre-compiled text bindings (no new Function)
        if !self.for_text_bindings.is_empty() {
            self.js.push_str("    // Pre-compiled text bindings\n");
            self.js.push_str("    (function() {\n");
            for (nid, js_expr) in &self.for_text_bindings {
                self.js.push_str(&format!(
                    "      var _el = clone.querySelector('[data-nid=\"{}\"]');\n",
                    nid
                ));
                self.js.push_str(&format!(
                    "      if (_el) try {{ _el.textContent = String({}); }} catch(e) {{ __omni_reportError(\"for:text\", e); }}\n",
                    js_expr
                ));
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
        if !self.for_class_bindings.is_empty() {
            self.js.push_str("    // Pre-compiled class bindings\n");
            self.js.push_str("    (function() {\n");
            for (nid, class_name, js_expr) in &self.for_class_bindings {
                self.js.push_str(&format!(
                    "      var _el = clone.querySelector('[data-nid=\"{}\"]');\n",
                    nid
                ));
                self.js.push_str(&format!(
                    "      if (_el) try {{ _el.classList.toggle(\"{}\", !!({})); }} catch(e) {{ __omni_reportError(\"for:class\", e); }}\n",
                    class_name, js_expr
                ));
            }
            self.js.push_str("    })();\n");
        }

        // Pre-compiled attribute bindings (no new Function)
        if !self.for_attr_bindings.is_empty() {
            use super::AttrPart;
            self.js.push_str("    // Pre-compiled attribute bindings\n");
            self.js.push_str("    (function() {\n");
            for (nid, attr_name, parts) in &self.for_attr_bindings {
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
                self.js.push_str(&format!(
                    "      var _el = clone.querySelector('[data-nid=\"{}\"]'); if (_el) try {{ _el.setAttribute(\"{}\", {}); }} catch(e) {{ __omni_reportError(\"for:attr\", e); }}\n",
                    nid, attr_name, resolved
                ));
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
            self.js
                .push_str("          const player = __omni_animations.get(leavingNid + '_leave');\n");
            self.js.push_str("          if (player) {\n");
            self.js.push_str(
                "            player.play().then(() => { el.remove(); _forKeys.delete(key); });\n",
            );
            self.js.push_str("          } else {\n");
            self.js
                .push_str("            el.remove(); _forKeys.delete(key);\n");
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
            self.js
                .push_str("  Omni.flipAfter(parent);\n");
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
                self.js.push_str(&format!(
                    "      '{}': '{}',\n",
                    js_event,
                    event_data_kind_label(kind)
                ));
            }
            self.js.push_str("    };\n");

            // Emit _extractEvent switch function
            self.js.push_str("    function _extractEvent(kind, e) {\n");
            self.js.push_str("      switch (kind) {\n");
            for (_, kind) in &for_event_kinds {
                let label = event_data_kind_label(kind);
                self.js.push_str(&format!("        case '{}':\n", label));
                self.js.push_str(&format!(
                    "          return {};\n",
                    emit_event_object(&Some(kind.clone()), &None, "e")
                ));
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
            self.js
                .push_str("      if (e.type === 'submit') e.preventDefault();\n");
            self.js.push_str("      if (attr.startsWith('fn:')) {\n");
            self.js
                .push_str("        const fn = __omni_forHandlers[attr.slice(3)];\n");
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
            if let Some(ref rebind_fn) = self.for_rebind_fn {
                self.js.push_str(&format!(
                    "          if (typeof {} === 'function') {}(_forComp);\n",
                    rebind_fn, rebind_fn
                ));
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
                self.js.push_str(&format!("'{}'", evt));
            }
            self.js.push_str("].forEach(function(evt) {\n");
            self.js
                .push_str("      parent.addEventListener(evt, _forHandle);\n");
            self.js.push_str("    });\n");

            self.js.push_str("  }\n");
        }

        // Emit pre-compiled rebind function for text bindings (no new Function)
        if !self.for_text_bindings.is_empty() {
            if let Some(ref rebind_fn) = self.for_rebind_fn {
                self.js.push_str(&format!(
                    "  // Pre-compiled rebind function: {}\n",
                    rebind_fn
                ));
                self.js
                    .push_str(&format!("  function {}(comp) {{\n", rebind_fn));
                for (nid, js_expr) in &self.for_text_bindings {
                    self.js.push_str(&format!(
                        "    var _el = comp.querySelector('[data-nid=\"{}\"]');\n",
                        nid
                    ));
                    self.js.push_str(&format!(
                        "    if (_el) try {{ _el.textContent = String({}); }} catch(e) {{ __omni_reportError(\"for:rebind\", e); }}\n",
                        js_expr
                    ));
                }
                self.js.push_str("  }\n");
            }
        }

        self.js.push_str("});\n");
    }
}
