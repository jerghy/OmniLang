//! View emission: DOM insertion, event/reactive bindings, CSS styles.
use super::OmniViewGenerator;
use crate::codegen::camel_to_kebab;
use crate::ir::*;

impl OmniViewGenerator {
    /// Emit the root component's view HTML into self.html, temporarily swapping
    /// self.js ↔ self.deferred_js so event/effect code goes into the deferred buffer.
    pub(super) fn emit_view(&mut self, view: &ViewTree) {
        self.current_nid_path.clear();

        // Swap js with deferred_js so view_to_html event/effect code
        // goes into the deferred buffer (emitted after DOM insertion)
        std::mem::swap(&mut self.js, &mut self.deferred_js);

        // Generate initial DOM HTML (events/effects go to deferred_js)
        self.html.push_str(&format!(
            "<div id=\"root\" data-component=\"{}\">\n",
            self.current_component
        ));
        let html = self.view_to_html(view, 1);
        self.html.push_str(&html);
        self.html.push_str("</div>\n");

        // Swap back — events/effects are now in deferred_js
        std::mem::swap(&mut self.js, &mut self.deferred_js);

        // Accumulate deferred_js — emit only once after ALL components
        if !self.deferred_js.is_empty() {
            self.all_deferred_js.push_str(&self.deferred_js);
            self.deferred_js.clear();
        }
    }

    /// Emit ONE innerHTML insertion + ALL accumulated deferred bindings/effects.
    /// Called once after all components have been generated.
    pub(super) fn emit_dom_and_bindings(&mut self) {
        if self.dom_emitted || self.html.is_empty() {
            return;
        }
        self.dom_emitted = true;

        // Emit DOM insertion script
        self.js.push_str("\n/// ── Initial DOM ──\n");
        self.js
            .push_str("document.getElementById(\"root\").innerHTML = `");
        let escaped = self
            .html
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("$", "\\$");
        self.js.push_str(&escaped);
        self.js.push_str("`;\n\n");

        // Emit ALL accumulated deferred event bindings and reactive effects
        if !self.all_deferred_js.is_empty() {
            self.js
                .push_str("\n/// ── Event Bindings & Reactive Effects ──\n");
            self.js.push_str(&self.all_deferred_js);
        }

        // Emit mount lifecycle hooks — run after DOM is inserted
        if !self.mount_js.is_empty() {
            self.js.push_str("\n/// ── Mount Lifecycle ──\n");
            self.js.push_str(&self.mount_js);
        }
    }

    /// Emit a CSS style rule scoped to the current component.
    pub(super) fn emit_style(&mut self, style: &StyleRuleIR) {
        let selector_prefix = if style.is_tag_selector {
            String::new()
        } else {
            ".".to_string()
        };
        self.css.push_str(&format!(
            "[data-component=\"{}\"] {}{} {{\n",
            self.current_component, selector_prefix, style.class_name
        ));
        for (prop, value) in &style.properties {
            let css_val = self.expr_to_css(value);
            let css_prop = camel_to_kebab(prop);
            self.css
                .push_str(&format!("  {}: {};\n", css_prop, css_val));
        }
        self.css.push_str("}\n\n");
    }
}
