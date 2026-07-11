//! Debug bridge emission for standalone debug window.
//! When debug_mode is enabled, emits JS that communicates with
//! the main Poiesis window via Tauri events (window.__TAURI_INTERNALS__).

use super::OmniViewGenerator;

impl OmniViewGenerator {
    /// Emit debug bridge code for the standalone debug window.
    /// Uses Tauri IPC (window.__TAURI_INTERNALS__) instead of parent.postMessage.
    /// Called after emit_editor_bridge(), before finalize_js().
    pub(super) fn emit_debug_bridge(&mut self) {
        if !self.debug_mode {
            return;
        }
        self.js.push_str(
            r#"/// ── Debug Bridge ──
(function() {
  // Expose signal registry for the debug panel UI
  window.__omni_subs = __omni_subs;

  // Override click handling: emit Tauri event instead of parent.postMessage
  document.addEventListener('click', function(e) {
    if (!window.__omni_debug || !window.__omni_debug._active) return;
    var el = e.target.closest('[data-nid]');
    if (!el) return;
    e.stopPropagation();
    try {
      window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
        event: 'debug:element-click',
        payload: { nid: el.dataset.nid, tag: el.tagName.toLowerCase() }
      });
    } catch(_) {}
  }, true);

  // Listen for highlight commands from main window
  window.__omni_debug = {
    _active: false,
    _highlightedNid: null,

    toggle: function() {
      this._active = !this._active;
      if (!this._active) this.clearHighlight();
      return this._active;
    },

    isActive: function() { return this._active; },

    getSignals: function() {
      var result = {};
      if (window.__omni_subs) {
        window.__omni_subs.forEach(function(signal, name) {
          try {
            result[name] = { value: signal.peek(), name: name };
          } catch(e) {
            result[name] = { value: '<error>', name: name };
          }
        });
      }
      return result;
    },

    highlight: function(nid) {
      this.clearHighlight();
      if (!nid) return;
      var el = document.querySelector('[data-nid="' + nid + '"]');
      if (el) {
        el.style.outline = '2px solid #6366f1';
        el.style.outlineOffset = '2px';
        this._highlightedNid = nid;
      }
    },

    clearHighlight: function() {
      if (this._highlightedNid) {
        var el = document.querySelector('[data-nid="' + this._highlightedNid + '"]');
        if (el) { el.style.outline = ''; el.style.outlineOffset = ''; }
        this._highlightedNid = null;
      }
    },

    getNidMap: function() {
      return (typeof __omni_buildNidMap === 'function')
        ? __omni_buildNidMap() : {};
    }
  };

  // Listen for highlight events from main window
  if (window.__TAURI_INTERNALS__) {
    window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
      event: 'debug:highlight',
      handler: window.__TAURI_INTERNALS__.transformCallback(function(e) {
        var payload = typeof e.payload === 'string' ? JSON.parse(e.payload) : e.payload;
        if (payload && payload.nid) {
          window.__omni_debug.highlight(payload.nid);
        } else {
          window.__omni_debug.clearHighlight();
        }
      })
    }).catch(function() {});

    // Notify main window that debug bridge is ready
    window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
      event: 'debug:bridge-ready',
      payload: { nidMap: window.__omni_debug.getNidMap() }
    }).catch(function() {});
  }
})();
"#,
        );
    }
}
