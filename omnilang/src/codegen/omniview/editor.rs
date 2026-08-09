//! Editor bridge emission and conditional HTML generation.
use super::OmniViewGenerator;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    /// Inject editor bridge code when `editor_mode` is enabled.
    /// Provides:
    ///   - window.__omni_signals / __omni_components for external access
    ///   - __omni_buildNidMap() — scans DOM for [data-nid] elements
    ///   - Click → postMessage omni-select
    ///   - postMessage handlers: omni-replace-component, omni-highlight, omni-clear-hl
    ///   - window.__omni_editor — highlight/clearHighlight helpers
    ///   - __omni_replaceComponent(name, factoryCode) — snapshot signals, teardown, restore
    ///   - parent.postMessage omni-rendered on load
    ///
    /// Bridge code below is ES5-compatible (no arrow functions, no template literals).
    pub(super) fn emit_editor_bridge(&mut self) {
        if !self.editor_mode {
            return;
        }
        // When debug_mode is active, the Debug Bridge handles click/communication
        // via Tauri IPC. Only emit __omni_buildNidMap() which getNidMap() relies on.
        if self.debug_mode {
            self.js.push_str(
                "\n/// ── Editor Bridge (debug: nidMap only) ──\n\
function __omni_buildNidMap() {\n\
  var map = {};\n\
  document.querySelectorAll('[data-nid]').forEach(function(el) {\n\
    map[el.dataset.nid] = {\n\
      elementId: el.id || '',\n\
      tag: el.tagName.toLowerCase(),\n\
      component: (el.closest('[data-component]') || {}).dataset ? el.closest('[data-component]').dataset.component : '',\n\
    };\n\
  });\n\
  return map;\n\
}\n",
            );
            return;
        }
        self.js.push_str(include_str!("bridges/editor_bridge.js"));
    }

    /// Generate HTML + JS for @if/@else-if/@else conditional rendering.
    ///
    /// Strategy:
    ///   - Generate each branch's HTML in a hidden wrapper div.
    ///   - Generate a JS _effect() that evaluates conditions and toggles display.
    pub(super) fn emit_conditional_html(
        &mut self,
        cond: &ConditionalIntent,
        indent: usize,
    ) -> String {
        let pad = "  ".repeat(indent);
        let nid = self.next_nid("if");
        let mut out = String::new();
        let mut branch_nids: Vec<String> = Vec::new();

        // Generate HTML for each branch (all hidden by default)
        for (i, branch) in cond.branches.iter().enumerate() {
            let branch_nid = format!("{}-b{}", nid, i);
            let display = "none";
            let _ = writeln!(
                out,
                "{}<div data-nid=\"{}\" style=\"display:{}\">",
                pad, branch_nid, display
            );
            if let Some(ref body) = branch.body {
                out.push_str(&self.view_to_html(body, indent + 1));
            }
            let _ = writeln!(out, "{}</div>", pad);
            branch_nids.push(branch_nid);
        }

        // Fallback branch (else)
        let fallback_nid = if cond.fallback.is_some() {
            let fb_nid = format!("{}-else", nid);
            let _ = writeln!(
                out,
                "{}<div data-nid=\"{}\" style=\"display:none\">",
                pad, fb_nid
            );
            if let Some(ref fallback) = cond.fallback {
                out.push_str(&self.view_to_html(fallback, indent + 1));
            }
            let _ = writeln!(out, "{}</div>", pad);
            Some(fb_nid)
        } else {
            None
        };

        // Generate the reactive JS _effect()
        let mut conditions_js = Vec::new();
        for (i, branch) in cond.branches.iter().enumerate() {
            let cond_js = self.expr_to_js(&branch.condition);
            conditions_js.push((cond_js, branch_nids[i].clone()));
        }

        let _ = writeln!(self.js, "\n/// ── @if conditional {} ──", nid);

        // `mode: "wait"` would wait for a leave animation before swapping
        // branches, but OmniLang has no @leave mechanism — nothing registers
        // `_leave` entries (the old `__omni_animations.get(nid + "_leave")`
        // lookup could never match). Degrade honestly: instant swap (same
        // timing as mode: "none"), stated in the generated JS so it is never
        // a silent no-op.
        if cond.transition_mode.as_deref() == Some("wait") {
            let _ = writeln!(
                self.js,
                "// @if mode: \"wait\" — no leave animation exists in OmniLang; degrades to an instant swap (same as mode: \"none\")"
            );
        }

        // Emit all NIDs as a JS array for efficient bulk toggling
        let nid_list: Vec<String> = branch_nids
            .iter()
            .chain(fallback_nid.iter())
            .map(|n| format!("\"{}\"", n))
            .collect();
        let nid_array = format!("[{}]", nid_list.join(", "));

        // Sync mode (default): instant branch switching, @enter/@leave coexist in DOM
        self.js.push_str("__omni_effect(() => {\n");
        let _ = writeln!(self.js, "  const _nids = {};", nid_array);
        self.js.push_str("  const _show = (nid) => { _nids.forEach(id => { const e = __omni_el(id); if (e) { e.style.display = (id === nid) ? \"block\" : \"none\"; if (id === nid) { e.querySelectorAll('[data-nid]').forEach(function(c) { var a = __omni_animations.get(c.getAttribute('data-nid') + '_enter'); if (a) { a.cancel(); a.play(); } __omni_checkInView(c.getAttribute('data-nid')); }); } else { e.querySelectorAll('[data-nid]').forEach(function(c) { __omni_cancelElement(c.getAttribute('data-nid')); }); } } }); };\n");

        // For each branch, build the if/else if chain
        for (i, (cond_js, branch_nid)) in conditions_js.iter().enumerate() {
            let if_word = if i == 0 { "if" } else { "else if" };
            let _ = writeln!(
                self.js,
                "  {} ({}) {{ _show(\"{}\"); }}",
                if_word, cond_js, branch_nid
            );
        }

        // Else clause (fallback or hide-all)
        if let Some(ref fb_nid) = fallback_nid {
            let _ = writeln!(self.js, "  else {{ _show(\"{}\"); }}", fb_nid);
        } else {
            // No @else — hide all branches when no condition matches
            self.js.push_str("  else { _nids.forEach(id => { const e = __omni_el(id); if (e) e.style.display = \"none\"; }); }\n");
        }

        self.js.push_str("});\n");

        out
    }
}
