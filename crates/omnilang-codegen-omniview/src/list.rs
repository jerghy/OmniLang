//! List rendering: state initialization and thin dispatcher for @for loops.
use super::OmniViewGenerator;
use omnilang_core::ir::*;

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
            self.for_text_bindings.clear();
            self.for_class_bindings.clear();
            self.for_attr_bindings.clear();
            // Set rebind function name for this @for loop
            let rebind_name = format!("__omni_rebind_{}_{}", self.current_component, item_var);
            self.for_rebind_fn = Some(rebind_name);
            self.in_for_template = true;
            let html = self.view_to_html(body, indent + 1);
            self.in_for_template = false;
            html
        } else {
            format!("{}  <!-- empty item -->\n", pad)
        };

        let mut out = String::new();
        out.push_str(&format!(
            "{}<template data-nid=\"{}\">\n{}</template>\n",
            pad, template_nid, item_html
        ));

        // Container marker: <template> element used as insertion point.
        // The template tag is valid inside <tbody>, <tr>, etc. (unlike <div>).
        out.push_str(&format!(
            "{}<template data-nid=\"{}\" data-for-container></template>\n",
            pad, container_nid
        ));

        // Empty state template (cloned into parent when data is empty)
        if let Some(ref empty_body) = list.empty_body {
            self.in_for_template = true;
            let empty_html = self.view_to_html(empty_body, indent + 1);
            self.in_for_template = false;
            out.push_str(&format!(
                "{}<template data-nid=\"{}\">\n{}</template>\n",
                pad, empty_nid, empty_html
            ));
        }

        // Generate the reactive JS _effect()
        self.js.push_str(&format!(
            "\n/// ── @for {item} in ... ──\n",
            item = item_var
        ));
        self.js.push_str("__omni_effect(() => {\n");
        self.js
            .push_str(&format!("  const data = {};\n", iterable_js));
        self.js
            .push_str(&format!("  const tpl = __omni_el(\"{}\");\n", template_nid));
        self.js.push_str(&format!(
            "  const ctr = __omni_el(\"{}\");\n",
            container_nid
        ));
        self.js.push_str("  if (!tpl || !ctr) return;\n");
        self.js.push_str("  const parent = tpl.parentNode;\n");
        self.js.push_str("  if (!parent) return;\n");

        // Re-render loop
        // If key_expr is present, use key-based identity tracking for enter/leave animations.
        // Otherwise, clear and re-render all items.
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
                "); } catch(e) { __omni_reportError(\"list:key\", e); return null; } };\n",
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
            self.js.push_str(&format!(
                "    const emptyEl = __omni_el(\"{}\");\n",
                empty_nid
            ));
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
}
