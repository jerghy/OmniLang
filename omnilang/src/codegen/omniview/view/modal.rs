//! `@modal` shell emission — a compiler-synthesized, self-drawn overlay
//! (spec 2026-08-09 §4.1 OmniView row, verbatim requirements):
//!
//! - self-drawn mask + dialog container (no native `<dialog>` API)
//! - `tabindex="-1"` dialog focused on open + a compiler-generated focus
//!   trap cycling Tab / Shift+Tab within the dialog (the only target of the
//!   three that must hand-write focus cycling logic)
//! - Escape and mask click set the open signal to `false`
//! - `role="dialog"` / `aria-modal="true"` / conditional `aria-labelledby`
//!   pointing at the first Heading-class element, which receives the
//!   compiler-generated `id="omni-modal-title"`
//! - named slot children project in document order; anonymous children fall
//!   back into the body slot (spec §2.1 rule 3)
//!
//! The open signal is registered as a signal read (F5 lesson: the visibility
//! effect must read the signal reactively — a one-shot static `display:none`
//! would freeze the dialog closed forever).
//!
//! All dialog JS is emitted into the deferred buffer (`self.js` during view
//! rendering → `deferred_js` → `all_deferred_js`), which runs only after the
//! DOM is in place, so `__omni_el(nid)` resolves. Top-level deferred JS
//! accumulates across the whole document, so listeners are wrapped in IIFEs
//! to avoid `const mask`/`const dialog` name collisions between modals.

use super::super::OmniViewGenerator;
use crate::ir::*;
use std::fmt::Write;

/// Heading-class tags that qualify as a modal title target. Mirrors
/// `validate_feedback_modal.rs`'s V13 collection and the React target's
/// `MODAL_HEADING_TAGS` (react/view.rs).
pub(super) const MODAL_HEADING_TAGS: &[&str] =
    &["Heading", "Title", "h1", "h2", "h3", "h4", "h5", "h6"];

/// Minimal positioning so the self-drawn mask actually overlays the viewport
/// and the dialog stacks above it (spec §4.1: 自绘遮罩). Emitted once per
/// document, guarded by `modal_css_emitted`.
const MODAL_CSS: &str = "\
.omni-modal { position: fixed; inset: 0; z-index: 1000; }\n\
.omni-modal-mask { position: absolute; inset: 0; background: rgba(0, 0, 0, 0.5); }\n\
.omni-modal-dialog { position: relative; max-width: 480px; margin: 10vh auto;\n\
    background: #fff; border-radius: 8px; padding: 16px; }\n";

/// Whether a subtree (slot fallbacks included) contains a heading element.
/// Mirrors react/view.rs's `subtree_has_heading` exactly (same ViewTree
/// variants, same reachable set).
fn subtree_has_heading(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            MODAL_HEADING_TAGS.contains(&el.tag.as_str())
                || el.children.iter().any(subtree_has_heading)
        }
        ViewTree::Slot { fallback, .. } => fallback.iter().any(subtree_has_heading),
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(subtree_has_heading))
                || cond.fallback.as_ref().is_some_and(subtree_has_heading)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(subtree_has_heading)
                || list.empty_body.as_ref().is_some_and(subtree_has_heading)
        }
        ViewTree::Fragment(children) => children.iter().any(subtree_has_heading),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

impl OmniViewGenerator {
    /// Emit the `@modal` shell. The caller (element_to_html) has already
    /// pushed the element node onto the tree and inserted element_tags /
    /// nid_map; this method pops the tree (the shell is the element's real
    /// wrapper) and emits the deferred JS listeners.
    ///
    /// Note: takes `open_signal`/`slots` rather than the `ModalIntentIR`
    /// struct itself — the core `ir` module does not re-export that type, and
    /// `types` is private to the core crate.
    pub(super) fn emit_modal_shell(
        &mut self,
        el: &ElementIR,
        open_signal: &str,
        slots: &[SlotDeclIR],
        indent: usize,
        nid: &str,
    ) -> String {
        let pad = "  ".repeat(indent);

        // Resolve the open signal like expr.rs does for identifier reads
        // (component-prefixed rename, e.g. `isOpen` → `App_isOpen`).
        let resolved = self
            .name_resolver
            .name_renames
            .get(open_signal)
            .or_else(|| self.name_resolver.global_renames.get(open_signal))
            .cloned()
            .unwrap_or_else(|| open_signal.to_string());
        let read = if self.editor_mode {
            format!("__omni_subs.get(\"{}\").value", resolved)
        } else {
            format!("{}.value", resolved)
        };

        // F1 (spec §2.1 rule 5): only with a Heading in the dialog content is
        // `aria-labelledby` emitted — pointing at the synthesized title id the
        // first Heading receives via pending_modal_title_id (injected in
        // element_to_html). Mirrors react/view.rs's has_heading formula.
        let has_heading = slots
            .iter()
            .any(|slot| slot.fallback.iter().any(subtree_has_heading))
            || el.children.iter().any(subtree_has_heading);
        if has_heading {
            self.pending_modal_title_id = true;
        }

        // ── HTML shell ──
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}<div class=\"omni-modal\" data-nid=\"{}\" style=\"display:none\">",
            pad, nid
        );
        let _ = writeln!(
            out,
            "{}  <div class=\"omni-modal-mask\" data-nid=\"{}-mask\"></div>",
            pad, nid
        );
        let labelledby = if has_heading {
            " aria-labelledby=\"omni-modal-title\""
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "{}  <div class=\"omni-modal-dialog\" role=\"dialog\" aria-modal=\"true\" \
             tabindex=\"-1\" data-nid=\"{}-dialog\"{}>",
            pad, nid, labelledby
        );

        // Named-slot content projects in document order; each slot's fallback
        // content lands directly in the dialog body (spec §4.1: 内容为命名槽
        // 投影，匿名子内容兜底进 body 槽).
        for slot in slots {
            for child in &slot.fallback {
                out.push_str(&self.view_to_html(child, indent + 2));
            }
        }
        // F4 (spec §2.1 rule 3): anonymous children fall back into the body
        // slot — render everything that is NOT a `ViewTree::Slot` node (those
        // were just projected above and must not render twice).
        for child in &el.children {
            if matches!(child, ViewTree::Slot { .. }) {
                continue;
            }
            out.push_str(&self.view_to_html(child, indent + 2));
        }
        // The flag is consumed by the first heading emitted; reset
        // defensively so it cannot leak onto siblings outside the dialog.
        self.pending_modal_title_id = false;

        let _ = writeln!(out, "{}  </div>", pad);
        let _ = writeln!(out, "{}</div>", pad);

        // ── Modal CSS, once per document ──
        if !self.modal_css_emitted {
            self.modal_css_emitted = true;
            self.css.push_str(MODAL_CSS);
        }

        // ── Deferred JS (runs after DOM insertion; __omni_el available) ──
        let mut js = String::new();
        // Visibility is driven by the open signal (reactive read, F5); on
        // open, focus lands inside the dialog (tabindex="-1" makes it
        // programmatically focusable without adding a tab stop).
        js.push_str(&format!(
            "__omni_effect(() => {{ const _shell = __omni_el(\"{nid}\"); const _dialog = __omni_el(\"{nid}-dialog\"); if (_shell) _shell.style.display = ({read}) ? \"block\" : \"none\"; if (({read}) && _dialog) _dialog.focus(); }});\n",
            nid = nid,
            read = read,
        ));
        // Mask click closes (only when the click target IS the mask, so
        // clicks inside the dialog never propagate to close).
        js.push_str(&format!(
            "(function() {{ const mask = __omni_el(\"{nid}-mask\"); if (mask) mask.addEventListener(\"click\", function(e) {{ if (e.target === mask) __omni_subs.get(\"{resolved}\").set(false); }}); }})();\n",
            nid = nid,
            resolved = resolved,
        ));
        // Focus trap on the dialog: Escape closes; Tab / Shift+Tab cycle
        // between the first and last focusable (no-op when nothing focusable
        // exists inside the dialog).
        js.push_str(&format!(
            "(function() {{ const dialog = __omni_el(\"{nid}-dialog\"); if (!dialog) return; dialog.addEventListener(\"keydown\", function(e) {{ if (e.key === \"Escape\") {{ e.preventDefault(); __omni_subs.get(\"{resolved}\").set(false); return; }} if (e.key === \"Tab\") {{ const focusables = dialog.querySelectorAll(\"a[href],button:not([disabled]),textarea:not([disabled]),input:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex=\\\"-1\\\"])\"); if (focusables.length === 0) return; const first = focusables[0]; const last = focusables[focusables.length - 1]; if (e.shiftKey && document.activeElement === first) {{ e.preventDefault(); last.focus(); }} else if (!e.shiftKey && document.activeElement === last) {{ e.preventDefault(); first.focus(); }} }} }}); }})();\n",
            nid = nid,
            resolved = resolved,
        ));
        self.js.push_str(&js);

        self.tree_pop(); // balance the tree_push in element_to_html
        out
    }
}
