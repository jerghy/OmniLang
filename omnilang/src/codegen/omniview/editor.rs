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
        self.js.push_str(
            r#"
/// ── Editor Bridge ──
window.__omni_signals = {};
window.__omni_components = {};

function __omni_buildNidMap() {
  var map = {};
  document.querySelectorAll('[data-nid]').forEach(function(el) {
    map[el.dataset.nid] = {
      elementId: el.id || '',
      tag: el.tagName.toLowerCase(),
      component: (el.closest('[data-component]') || {}).dataset ? el.closest('[data-component]').dataset.component : '',
    };
  });
  return map;
}

document.addEventListener('click', function(e) {
  var el = e.target.closest('[data-nid]');
  if (el) {
    e.stopPropagation();
    parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, window.location.origin);
  }
});

window.addEventListener('message', function(e) {
  if (e.origin !== window.location.origin) return;
  switch (e.data.type) {
    case 'omni-replace-component':
      __omni_replaceComponent(e.data.name, e.data.js);
      break;
    case 'omni-highlight':
      __omni_editor.highlight(e.data.nid);
      break;
    case 'omni-clear-hl':
      __omni_editor.clearHighlight();
      break;
  }
});

window.__omni_editor = {
  highlight: function(nid) {
    var el = document.querySelector('[data-nid="' + nid + '"]');
    if (el) { el.style.outline = '2px solid #4F46E5'; }
  },
  clearHighlight: function() {
    document.querySelectorAll('[data-nid]').forEach(function(el) {
      el.style.outline = '';
    });
  },
};

function __omni_replaceComponent(name, factoryCode) {
  // Defense-in-depth: validate factoryCode is a string before eval.
  // The primary protection is the postMessage origin check, but this
  // guards against same-origin XSS in the parent editor page sending
  // a crafted message with non-string js payload.
  if (typeof factoryCode !== 'string' || factoryCode.length === 0) {
    return;
  }
  var oldComp = window.__omni_components[name];
  var savedSignals = {};
  if (oldComp && oldComp.signals) {
    Object.keys(oldComp.signals).forEach(function(k) {
      savedSignals[k] = oldComp.signals[k].peek();
    });
  }
  try {
    window.__omni_components[name] = (0, eval)(factoryCode);
  } catch(e) {
    try { parent.postMessage({ type: 'omni-replace-error', name: name, error: String(e) }, window.location.origin); } catch(_) {}
    if (oldComp) window.__omni_components[name] = oldComp;
    return;
  }
  if (oldComp && oldComp.teardown) oldComp.teardown();
  if (__omni_el._cache) __omni_el._cache = {};
  var newComp = window.__omni_components[name];
  if (newComp && newComp.signals) {
    Object.keys(savedSignals).forEach(function(k) {
      if (newComp.signals[k]) newComp.signals[k].set(savedSignals[k]);
    });
  }
  parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);
}

parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);
"#,
        );
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

        let is_wait_mode = cond.transition_mode.as_deref() == Some("wait");

        let _ = writeln!(self.js, "\n/// ── @if conditional {} ──", nid);

        // Emit all NIDs as a JS array for efficient bulk toggling
        let nid_list: Vec<String> = branch_nids
            .iter()
            .chain(fallback_nid.iter())
            .map(|n| format!("\"{}\"", n))
            .collect();
        let nid_array = format!("[{}]", nid_list.join(", "));

        if is_wait_mode {
            // Wait mode: play @leave on old branch, wait for completion, then show new branch
            let _ = writeln!(self.js, "var _current_{nid} = null;\n\
                 __omni_effect(() => {{\n\
                 \x20 const _nids = {nids};\n\
                 \x20 const _hideAll = (except) => {{ _nids.forEach(id => {{ const e = __omni_el(id); if (e && id !== except) e.style.display = \"none\"; }}); }};\n\
                 \x20 const _showNow = (nid) => {{ const e = __omni_el(nid); if (e) e.style.display = \"block\"; _hideAll(nid); _current_{nid} = nid; }};\n\
                 \x20 const _transition = (newNid) => {{\n\
                 \x20\x20 const oldNid = _current_{nid};\n\
                 \x20\x20 if (oldNid === newNid) return;\n\
                 \x20\x20 if (!oldNid) {{ _showNow(newNid); return; }}\n\
                 \x20\x20 // Play @leave on old branch, then show new\n\
                 \x20\x20 const leaveAnim = __omni_animations.get(oldNid + \"_leave\");\n\
                 \x20\x20 if (leaveAnim) {{\n\
                 \x20\x20\x20 leaveAnim.play();\n\
                 \x20\x20\x20 // Wait for leave animation to complete\n\
                 \x20\x20\x20 function _onLeaveDone() {{\n\
                 \x20\x20\x20\x20 // Cleanup old branch animations\n\
                 \x20\x20\x20\x20 const oldEl = __omni_el(oldNid);\n\
                 \x20\x20\x20\x20 if (oldEl) {{\n\
                 \x20\x20\x20\x20\x20 oldEl.querySelectorAll('[data-nid]').forEach(function(c) {{\n\
                 \x20\x20\x20\x20\x20\x20 __omni_cleanupElement(c.getAttribute('data-nid'));\n\
                 \x20\x20\x20\x20\x20 }});\n\
                 \x20\x20\x20\x20 }}\n\
                 \x20\x20\x20\x20 _showNow(newNid);\n\
                 \x20\x20\x20\x20 // Trigger @enter on new branch\n\
                 \x20\x20\x20\x20 const enterAnim = __omni_animations.get(newNid + \"_enter\");\n\
                 \x20\x20\x20\x20 if (enterAnim && enterAnim.playState !== \"paused\") enterAnim.play();\n\
                 \x20\x20\x20 }}\n\
                 \x20\x20\x20 if (leaveAnim.finished) {{\n\
                 \x20\x20\x20\x20 leaveAnim.finished.then(_onLeaveDone);\n\
                 \x20\x20\x20 }} else {{\n\
                 \x20\x20\x20\x20 const checkComplete = setInterval(() => {{\n\
                 \x20\x20\x20\x20\x20 if (leaveAnim.playState === \"finished\" || leaveAnim.playState === \"idle\") {{\n\
                 \x20\x20\x20\x20\x20\x20 clearInterval(checkComplete);\n\
                 \x20\x20\x20\x20\x20\x20 _onLeaveDone();\n\
                 \x20\x20\x20\x20\x20 }}\n\
                 \x20\x20\x20\x20 }}, 16);\n\
                 \x20\x20\x20 }}\n\
                 \x20\x20 }} else {{\n\
                 \x20\x20\x20 // Cleanup old branch animations\n\
                 \x20\x20\x20 const oldEl = __omni_el(oldNid);\n\
                 \x20\x20\x20 if (oldEl) {{\n\
                 \x20\x20\x20\x20 oldEl.querySelectorAll('[data-nid]').forEach(function(c) {{\n\
                 \x20\x20\x20\x20\x20 __omni_cleanupElement(c.getAttribute('data-nid'));\n\
                 \x20\x20\x20\x20 }});\n\
                 \x20\x20\x20 }}\n\
                 \x20\x20\x20 _showNow(newNid);\n\
                 \x20\x20 }}\n\
                 \x20 }};", nid = nid,
                nids = nid_array);
        } else {
            // Sync mode (default): instant branch switching, @enter/@leave coexist in DOM
            self.js.push_str("__omni_effect(() => {\n");
            let _ = writeln!(self.js, "  const _nids = {};", nid_array);
            self.js.push_str("  const _show = (nid) => { _nids.forEach(id => { const e = __omni_el(id); if (e) { e.style.display = (id === nid) ? \"block\" : \"none\"; if (id === nid) { e.querySelectorAll('[data-nid]').forEach(function(c) { var a = __omni_animations.get(c.getAttribute('data-nid') + '_enter'); if (a) { a.cancel(); a.play(); } }); } else { e.querySelectorAll('[data-nid]').forEach(function(c) { __omni_cleanupElement(c.getAttribute('data-nid')); }); } } }); };\n");
        }

        // For each branch, build the if/else if chain
        for (i, (cond_js, branch_nid)) in conditions_js.iter().enumerate() {
            let if_word = if i == 0 { "if" } else { "else if" };
            if is_wait_mode {
                let _ = writeln!(
                    self.js,
                    "  {} ({}) {{ _transition(\"{}\"); }}",
                    if_word, cond_js, branch_nid
                );
            } else {
                let _ = writeln!(
                    self.js,
                    "  {} ({}) {{ _show(\"{}\"); }}",
                    if_word, cond_js, branch_nid
                );
            }
        }

        // Else clause (fallback or hide-all)
        if let Some(ref fb_nid) = fallback_nid {
            if is_wait_mode {
                let _ = writeln!(self.js, "  else {{ _transition(\"{}\"); }}", fb_nid);
            } else {
                let _ = writeln!(self.js, "  else {{ _show(\"{}\"); }}", fb_nid);
            }
        } else {
            // No @else — hide all branches when no condition matches
            if is_wait_mode {
                self.js
                    .push_str("  else { _hideAll(null); _current_{nid} = null; }\n");
            } else {
                self.js.push_str("  else { _nids.forEach(id => { const e = __omni_el(id); if (e) e.style.display = \"none\"; }); }\n");
            }
        }

        self.js.push_str("});\n");

        out
    }
}
