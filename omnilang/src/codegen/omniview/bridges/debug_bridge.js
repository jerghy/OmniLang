/// ── Debug Bridge ──
(function() {
  // Always update signal registry reference — new IIFE has fresh __omni_subs
  window.__omni_subs = __omni_subs;
  // Component registry for hot-replace
  window.__omni_components = window.__omni_components || {};
  // Expose runtime functions globally so (0, eval) in __omni_replaceComponent
  // (defined in the static debug-window/index.html shell) can access them.
  // These are the functions referenced by per-component factory code during
  // hot-replace. Updated on every compile so hot-replaced components use the
  // latest runtime from the current IIFE.
  window.__omni_signal = __omni_signal;
  window.__omni_computed = __omni_computed;
  window.__omni_effect = __omni_effect;
  window.__omni_el = __omni_el;

  // Guard against re-registration on reload:
  // the Tauri event listeners and click handler only need to be set up once.
  if (window.__omni_debug && window.__omni_debug._initialized) {
    return;
  }

  // Click → Tauri IPC element selection (capturing phase, before editor bridge)
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

  window.__omni_debug = {
    _active: false,
    _initialized: true,
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

  // Listen for highlight commands from main window (register once)
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
