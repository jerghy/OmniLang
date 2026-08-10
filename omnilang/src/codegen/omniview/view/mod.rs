//! View emission: DOM insertion, event/reactive bindings, CSS styles.
//! Entry point for HTML/JS/CSS generation from view trees.
//!
//! ## JS Execution Order Guarantee
//!
//! The generated JS executes in this exact order to ensure DOM elements exist
//! before event/reactive bindings are attached:
//!
//! 1. **Runtime definitions** — `__omni_signal`, `__omni_computed`, `__omni_effect`,
//!    `__omni_el`, `__omni_reportError`, `__omni_forGetState` etc.
//! 2. **Store declarations** — global `__omni_signal` calls for store fields
//! 3. **Component definitions** — signal/computed/prop/function/motion declarations
//!    prefixed by component name
//! 4. **DOM insertion** — `innerHTML` assignment (all static HTML rendered at once)
//! 5. **Deferred event bindings + reactive effects** — `addEventListener` calls,
//!    `__omni_effect()` registrations, animation triggers (all deferred via
//!    the `self.js ↔ self.deferred_js` swap in `emit_view()`)
//! 6. **Mount lifecycle hooks** — `__omni_unmount_*` handlers, `@enter` replay,
//!    `@effect` cleanup chaining
//!
//! This ordering is critical: reordering these phases will cause silent runtime
//! failures (events bound to non-existent elements, effects running before DOM).
//!
//! Sub-modules:
//! - `element`: Element HTML generation (`element_to_html`, tree helpers)
//! - `condition`: Conditional rendering (css_only mode + canvas helpers)
//! - `events`: Event binding emission (addEventListener, @for delegation)
//! - `style`: Inline style emission (static + dynamic signal-driven)

mod condition;
pub(super) mod element;
mod events;
mod modal;
pub(super) mod style;

// Re-exports so sibling modules in `codegen::omniview` can still access
// these functions by path if needed.
// Note: methods on `impl OmniViewGenerator` are callable via `self.xxx()`
// from anywhere in the crate — no re-export required.

use super::utils::escape_html;
use super::OmniViewGenerator;
use crate::codegen::camel_to_kebab;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    /// Emit the root component's view HTML into self.html, temporarily swapping
    /// self.js ↔ self.deferred_js so event/effect code goes into the deferred buffer.
    ///
    /// When hash routing is active the root container is a route page itself
    /// (`data-omni-route="/"`, hidden by default — the router shows it when the
    /// path "/" or an unmatched path is current).
    pub(super) fn emit_view(&mut self, view: &ViewTree) {
        let route_attr = if self.router_active {
            " data-omni-route=\"/\" style=\"display:none\""
        } else {
            ""
        };
        let open_tag = format!(
            "<div id=\"root\" data-component=\"{}\"{}{}>\n",
            self.scope.current_component,
            self.design_attr(),
            route_attr
        );
        self.emit_view_body(view, &open_tag);
    }

    /// Emit a routed non-root component's view into a hidden route container
    /// (`<div data-omni-route="{pattern}" ... style="display:none">`) appended
    /// after the root container. The hash router toggles visibility and moves
    /// the container into its owning layout's `@slot` at runtime (Task 8).
    pub(super) fn emit_route_view(&mut self, view: &ViewTree, pattern: &str) {
        let open_tag = format!(
            "<div data-omni-route=\"{}\" data-component=\"{}\"{} style=\"display:none\">\n",
            pattern,
            self.scope.current_component,
            self.design_attr()
        );
        self.emit_view_body(view, &open_tag);
    }

    /// Shared body of view emission: swap js ↔ deferred_js so event/effect code
    /// goes into the deferred buffer, write the open tag + tree + close tag,
    /// then restore buffers and accumulate deferred code (flushed once after
    /// ALL components).
    fn emit_view_body(&mut self, view: &ViewTree, open_tag: &str) {
        std::mem::swap(&mut self.js, &mut self.deferred_js);
        self.html.push_str(open_tag);
        let html = self.view_to_html(view, 1);
        self.html.push_str(&html);
        self.html.push_str("</div>\n");
        std::mem::swap(&mut self.js, &mut self.deferred_js);

        // Accumulate deferred_js — emit only once after ALL components
        if !self.deferred_js.is_empty() {
            self.all_deferred_js.push_str(&self.deferred_js);
            self.deferred_js.clear();
        }
    }

    /// ` data-design="..."` attribute string for the current component (empty
    /// when no design names are in scope).
    fn design_attr(&self) -> String {
        if self.design_names.is_empty() {
            String::new()
        } else {
            format!(" data-design=\"{}\"", self.design_names.join(" "))
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
        let _ = writeln!(
            self.css,
            "[data-component=\"{}\"] {}{} {{",
            self.scope.current_component, selector_prefix, style.class_name
        );
        for (prop, value) in &style.properties {
            let css_val = self.expr_to_css(value);
            let css_prop = camel_to_kebab(prop);
            let _ = writeln!(self.css, "  {}: {};", css_prop, css_val);
        }
        self.css.push_str("}\n\n");
    }

    // ══════════════════════════════════════════════════════
    // View → HTML Generation (dispatch)
    // ══════════════════════════════════════════════════════

    /// Main dispatch: convert a ViewTree into its HTML representation,
    /// collecting deferred JS bindings along the way.
    ///
    /// Delegates to `element`, `condition` and `list` sub-module methods,
    /// plus the `editor`-module `emit_conditional_html` for live mode.
    pub(super) fn view_to_html(&mut self, view: &ViewTree, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        match view {
            ViewTree::Element(el) => self.element_to_html(el, indent),
            ViewTree::Text(text) => {
                if text.trim().is_empty() {
                    String::new()
                } else {
                    let tnid = self.next_nid("text");
                    self.tree_text(tnid, escape_html(text));
                    format!("{}{}\n", pad, escape_html(text))
                }
            }
            ViewTree::Interpolation(expr) => {
                // Determine display text for the element tree (editor canvas preview).
                // Prefer the signal's default value, fall back to the expression label.
                let expr_label = self.expr_label(expr);
                let tree_text_content = self
                    .signal_defaults
                    .get(&expr_label)
                    .cloned()
                    .unwrap_or_else(|| expr_label.clone());

                if self.scope.in_for_template {
                    // Inside @for template: collect pre-compiled binding
                    let nid = self.next_nid("interp");
                    let js_expr = self.expr_to_js(expr);
                    self.scope.for_text_bindings.push((nid.clone(), js_expr));
                    // Add to element tree: span with value/default text
                    let text_nid = self.next_nid("text");
                    self.tree_push(nid.clone(), "span".to_string());
                    self.tree_text(text_nid, tree_text_content);
                    self.tree_pop();
                    format!("{}<span data-nid=\"{}\"></span>\n", pad, nid)
                } else {
                    // Emit placeholder span + deferred _effect() binding
                    let nid = self.next_nid("interp");
                    let js_expr = self.expr_to_js(expr);
                    // Strip trailing () for signal/computed name lookup
                    let js_expr_base = js_expr.trim_end_matches("()");
                    let value_access = if self.name_resolver.signal_names.contains(js_expr_base) {
                        format!("{}.value", js_expr_base)
                    } else {
                        js_expr
                    };
                    // Write to self.js (which is the deferred buffer after emit_view swap)
                    let _ = writeln!(self.js,
                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.textContent = String({}); }} catch(e) {{ __omni_reportError(\"warn:interpolation:text\", e); }} }});",
                        nid, value_access
                    );
                    // Add to element tree: span with value/default text
                    let text_nid = self.next_nid("text");
                    self.tree_push(nid.clone(), "span".to_string());
                    self.tree_text(text_nid, tree_text_content);
                    self.tree_pop();
                    format!("{}<span data-nid=\"{}\"></span>\n", pad, nid)
                }
            }
            ViewTree::Conditional(cond) => {
                if self.css_only {
                    self.emit_conditional_css_only(cond, indent)
                } else {
                    self.emit_conditional_html(cond, indent)
                }
            }
            ViewTree::ListRendering(list) => self.emit_list_rendering_html(list, indent),
            ViewTree::Slot { name, fallback, .. } => {
                let slot_attr = match name {
                    Some(ref n) => format!(" name=\"{}\"", n),
                    None => String::new(),
                };
                if fallback.is_empty() {
                    // No fallback — the slot stays empty; childrenMap injection
                    // (slot projection) fills it with parent-provided content.
                    format!("{}<slot{}></slot>\n", pad, slot_attr)
                } else {
                    // Fallback content renders inside the <slot> element and is
                    // replaced by the projected children when the parent
                    // provides content for this slot (browser slot semantics).
                    let mut out = format!("{}<slot{}>\n", pad, slot_attr);
                    for child in fallback {
                        out.push_str(&self.view_to_html(child, indent + 1));
                    }
                    out.push_str(&format!("{}</slot>\n", pad));
                    out
                }
            }
            ViewTree::Fragment(children) => {
                let mut out = String::new();
                for child in children {
                    out.push_str(&self.view_to_html(child, indent));
                }
                out
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
        }
    }
}
