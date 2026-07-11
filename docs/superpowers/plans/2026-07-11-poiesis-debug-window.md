# Poiesis Debug Window — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a standalone Tauri debug window to Poiesis that renders OmniView output with bidirectional element highlight and runtime Store/signal inspection.

**Architecture:** Add a `debug_mode` flag to the OmniView WASM codegen that emits Tauri event bridge JS. Create a Tauri WebviewWindow that loads a static HTML shell. The main window pushes compile results via Tauri events; the debug window renders them and sends element clicks back.

**Tech Stack:** Rust (omnilang codegen + Tauri backend), TypeScript/React (Poiesis frontend), vanilla JS (debug window panel UI)

## Global Constraints

- `debug_mode` requires `editor_mode=true` (asserted at call site)
- Debug window uses `window.__TAURI_INTERNALS__` (no npm dependency)
- Panel UI is vanilla JS + CSS (~200 lines), no React in debug window
- Hot update shares the existing 150ms debounce from `useWasmCompile`
- CSP: debug window needs inline script support; add `'unsafe-inline'` to `script-src` for the debug window only

---

### Task 1: Codegen — `debug_mode` flag + `debug.rs` bridge emission

**Files:**
- Create: `omnilang/src/codegen/omniview/debug.rs`
- Modify: `omnilang/src/codegen/omniview/mod.rs:28,122-167,393-487`
- Modify: `omnilang/src/codegen/omniview/utils.rs:189-232`

**Interfaces:**
- Produces: `OmniViewGenerator::emit_debug_bridge(&mut self)` — emits Tauri event bridge JS inside the IIFE
- Produces: `pub fn generate(ir: &IRProgram, editor_mode: bool, css_only: bool, debug_mode: bool) -> OmniViewOutput`
- Produces: `pub fn compile_files(ir: &IRProgram, source_files: &FxHashMap<String, String>, editor_mode: bool, debug_mode: bool) -> CompileOutput`
- Consumes: `self.debug_mode: bool` field on `OmniViewGenerator`

- [ ] **Step 1a: Add `mod debug` and `debug_mode` field to generator struct**

In `omnilang/src/codegen/omniview/mod.rs`, after line 28 (`pub use utils::{compile_files, wrap_debug_html};`), add:

```rust
mod debug;
```

In the `OmniViewGenerator` struct (around line 487, before the closing `}`), add after `nid_map`:

```rust
        /// When true, emit debug bridge code for the standalone debug window.
        /// Uses Tauri events instead of parent.postMessage.
        debug_mode: bool,
```

In the `generate()` function, update the struct literal to include `debug_mode`:

```rust
pub fn generate(ir: &IRProgram, editor_mode: bool, css_only: bool, debug_mode: bool) -> OmniViewOutput {
    let mut gen = OmniViewGenerator {
        // ... all existing fields ...
        nid_map: FxHashMap::default(),
        debug_mode,  // ← ADD THIS LINE
    };
```

In the test constructor `test_new()` (approx line 546), add:

```rust
            nid_map: FxHashMap::default(),
            debug_mode: false,  // ← ADD THIS LINE
```

- [ ] **Step 1b: Create `debug.rs` with `emit_debug_bridge()`**

Create `omnilang/src/codegen/omniview/debug.rs`:

```rust
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
```

- [ ] **Step 1c: Call `emit_debug_bridge()` in `generate()`**

In `omnilang/src/codegen/omniview/mod.rs`, in the `generate()` function, after `gen.emit_editor_bridge();` (approx line 322), add:

```rust
        // Inject debug bridge code (Tauri event communication for standalone debug window)
        gen.emit_debug_bridge();
```

- [ ] **Step 1d: Update `compile_files()` to thread `editor_mode` and `debug_mode`**

In `omnilang/src/codegen/omniview/utils.rs`, replace the `compile_files` function signature and the `generate` call:

```rust
/// Wrap `generate()` into the standard `CompileOutput` format.
///
/// Called by `super::compile()` when the target is `Target::OmniView`.
pub fn compile_files(
    ir: &crate::ir::IRProgram,
    _source_files: &FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
) -> CompileOutput {
    let output = super::generate(ir, editor_mode, false, debug_mode);
```

- [ ] **Step 1e: Update `codegen::compile()` to pass `false, false` for normal path**

In `omnilang/src/codegen/mod.rs`, find the OmniView match arm (approx line 78) and update:

```rust
        Target::OmniView => omniview::compile_files(ir, source_files, false, false),
```

- [ ] **Step 1f: Update `lib.rs` callers to pass `debug_mode`**

In `omnilang/src/lib.rs`:

Line 134 — `compile_omniview`:
```rust
            let gen_output = codegen::omniview::generate(&pipeline.ir, editor_mode, false, false);
```

Line 196 — `compile_omniview_full`:
```rust
            let gen_output = codegen::omniview::generate(&pipeline.ir, editor_mode, false, false);
```

Line 440 — `compile_multi_internal_post`, replace the editor_mode block:
```rust
    #[cfg(feature = "codegen-omniview")]
    if editor_mode && matches!(target, codegen::Target::OmniView) {
        // Re-generate with editor_mode + debug_mode for debug window usage.
        // This replaces the previous wrap_debug_html post-processing approach
        // so the HTML is always generated with the correct flags.
        let debug_mode = false; // set by caller; CLI never uses debug mode
        let gen_output = codegen::omniview::compile_files(&merged_ir, sources, editor_mode, debug_mode);
        let mut files_map: FxHashMap<String, String> = gen_output
            .files
            .into_iter()
            .map(|f| (f.path, f.content))
            .collect();
        // No wrap_debug_html needed — compile_files already produces standalone HTML
        return InternalCompileResult {
            success: gen_output.errors.iter().all(|d| d.severity != error::Severity::Error),
            files: files_map,
            errors: diags.into_iter().chain(gen_output.errors).collect(),
            warnings,
        };
    }
```

Wait — the existing code at line 440 does `wrap_debug_html` post-processing. But with the new approach, `compile_files` now takes `editor_mode` and `debug_mode`, so it generates the correct HTML directly. We should call `compile_files` instead of `codegen::compile` for the OmniView+editor_mode case.

Actually, let me keep it simpler. The `compile_multi_internal_post` code path is for CLI usage (omnic). It already calls `codegen::compile()` which now passes `false, false` to `compile_files()`. The `wrap_debug_html` post-processing wraps the output with editor bridge JS inlined. This path doesn't need debug_mode (CLI never uses debug window).

The debug window will use the WASM `OmniCompiler` path or a new WASM function. Let me not modify the CLI path for now — keep the scope minimal.

- [ ] **Step 1g: Build and verify — omni-compiler WASM**

```bash
cd omnilang && cargo build --features codegen-omniview -p omnilang 2>&1
```

Expected: compiles without errors.

- [ ] **Step 1h: Run existing Omniview codegen tests**

```bash
cd omnilang && cargo test -p omnilang -- codegen::omniview 2>&1
```

Expected: all existing tests pass (no regressions from adding `debug_mode: false`).

- [ ] **Step 1i: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs \
        omnilang/src/codegen/omniview/debug.rs \
        omnilang/src/codegen/omniview/utils.rs \
        omnilang/src/codegen/mod.rs \
        omnilang/src/lib.rs
git commit -m "feat: add debug_mode flag to OmniView codegen and emit_debug_bridge()"
```

---

### Task 2: WASM bindings — expose `debug_mode` to JavaScript

**Files:**
- Modify: `omnilang/src/wasm.rs:46-52,59-63,104-110,167-201,237-250`

**Interfaces:**
- Consumes: `OmniViewGenerator::debug_mode` (from Task 1)
- Produces: Updated WASM exports with optional `debug_mode` parameter (default `false`)

- [ ] **Step 2a: Add `debug_mode` parameter to `compile_to_omniview`**

In `omnilang/src/wasm.rs`, update the function signature and call:

```rust
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, editor_mode: bool, debug_mode: bool) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&compile_omniview(source, editor_mode, debug_mode))
}
```

- [ ] **Step 2b: Add `debug_mode` parameter to `compile_omniview_component`**

```rust
#[wasm_bindgen]
pub fn compile_omniview_component(
    source: &str,
    component_name: &str,
    editor_mode: bool,
    debug_mode: bool,
) -> JsValue {
```

And update the internal call to pass `debug_mode`.

- [ ] **Step 2c: Add `debug_mode` parameter to `compile_to_omniview_full`**

```rust
pub fn compile_to_omniview_full(source: &str, editor_mode: bool, debug_mode: bool) -> JsValue {
```

- [ ] **Step 2d: Add `debug_mode` parameter to `compile_multi_to_omniview`**

```rust
pub fn compile_multi_to_omniview(sources: JsValue, editor_mode: bool, debug_mode: bool) -> JsValue {
```

- [ ] **Step 2e: Update `OmniCompiler.compile_to_omniview`**

```rust
pub fn compile_to_omniview(&mut self, editor_mode: bool, debug_mode: bool) -> JsValue {
```

And update internal calls to `compile_multi_internal`.

- [ ] **Step 2f: Update `compile_omniview` in `lib.rs`**

In `omnilang/src/lib.rs`, update the `compile_omniview` function:

```rust
pub fn compile_omniview(source: &str, editor_mode: bool, debug_mode: bool) -> OmniViewResult {
    // ...
    let gen_output = codegen::omniview::generate(&pipeline.ir, editor_mode, false, debug_mode);
```

And `compile_omniview_full`:

```rust
pub fn compile_omniview_full(source: &str, editor_mode: bool, debug_mode: bool) -> OmniViewFullResult {
    // ...
    let gen_output = codegen::omniview::generate(&pipeline.ir, editor_mode, false, debug_mode);
```

- [ ] **Step 2g: Build WASM and verify**

```bash
cd omnilang && wasm-pack build --target web --features codegen-omniview 2>&1 | tail -20
```

Expected: `[INFO]: Done! Your wasm pkg is ready`

- [ ] **Step 2h: Commit**

```bash
git add omnilang/src/wasm.rs omnilang/src/lib.rs
git commit -m "feat: add debug_mode parameter to WASM compile functions"
```

---

### Task 3: Tauri backend — debug window commands + static HTML shell

**Files:**
- Create: `poiesis/src-tauri/debug-window/index.html`
- Modify: `poiesis/src-tauri/src/lib.rs:840-850` (roughly, near the end where `.run()` is)
- Modify: `poiesis/src-tauri/tauri.conf.json:14-18`
- Modify: `poiesis/src-tauri/capabilities/default.json`

**Interfaces:**
- Produces: `open_debug_window` Tauri command
- Produces: `close_debug_window` Tauri command
- Consumes: Tauri `AppHandle` for window creation

- [ ] **Step 3a: Create debug window static HTML shell**

Create `poiesis/src-tauri/debug-window/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Poiesis Debug</title>
<style>
  *, *::before, *::after { box-sizing: border-box; }
  body { margin: 0; padding: 0; font-family: system-ui, sans-serif; overflow: hidden; }
  #root { width: 100vw; height: 100vh; overflow: auto; }

  .debug-ball {
    position: fixed; top: 16px; right: 16px;
    width: 42px; height: 42px; border-radius: 50%;
    background: #22c55e; cursor: pointer; z-index: 99999;
    box-shadow: 0 2px 12px rgba(0,0,0,0.25);
    display: flex; align-items: center; justify-content: center;
    font-size: 20px; user-select: none;
    transition: background 0.2s, transform 0.15s;
    border: none; outline: none;
  }
  .debug-ball:hover { transform: scale(1.08); }
  .debug-ball:active { transform: scale(0.95); }
  .debug-ball.active { background: #6366f1; }

  .debug-panel {
    position: fixed; top: 68px; right: 16px;
    width: 300px; max-height: 520px;
    background: #1e1e2e; color: #cdd6f4;
    border-radius: 12px; box-shadow: 0 8px 40px rgba(0,0,0,0.5);
    z-index: 99998; overflow: hidden; display: none;
    font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
    font-size: 12px;
  }
  .debug-panel.visible { display: block; }

  .panel-header {
    padding: 10px 14px; font-size: 11px; font-weight: 600;
    text-transform: uppercase; color: #a6adc8;
    border-bottom: 1px solid #313244; letter-spacing: 0.5px;
    user-select: none;
  }
  .panel-drag-handle { cursor: grab; }
  .panel-drag-handle:active { cursor: grabbing; }

  .signal-row {
    padding: 6px 14px; font-size: 12px;
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid #181825;
  }
  .signal-row:hover { background: #313244; }
  .signal-name { color: #89b4fa; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex-shrink: 0; max-width: 55%; }
  .signal-value { color: #a6e3a1; max-width: 45%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; }

  .action-row { padding: 4px 14px; font-size: 11px; color: #f9e2af; }
  .compile-log { font-size: 10px; padding: 4px 14px; color: #6c7086; font-family: inherit; }
</style>
</head>
<body>
<div id="root"></div>

<button id="debug-ball" class="debug-ball" title="Toggle debug mode">🔍</button>

<div id="debug-panel" class="debug-panel">
  <div class="panel-header panel-drag-handle" id="panel-drag-handle">📊 Store Signals</div>
  <div id="signal-list"></div>
  <div class="panel-header">📋 Stores</div>
  <div id="action-list"></div>
  <div id="compile-log" class="compile-log"></div>
</div>

<script>
(function() {
  var ball = document.getElementById('debug-ball');
  var panel = document.getElementById('debug-panel');
  var signalList = document.getElementById('signal-list');
  var actionList = document.getElementById('action-list');
  var compileLog = document.getElementById('compile-log');
  var active = false;
  var refreshTimer = null;

  function toggleDebug() {
    active = !active;
    ball.classList.toggle('active', active);
    panel.classList.toggle('visible', active);
    ball.textContent = active ? '✕' : '🔍';
    if (active) { refreshSignals(); }
    else { stopRefresh(); }
    if (window.__omni_debug) window.__omni_debug._active = active;
  }

  ball.addEventListener('click', toggleDebug);

  // Panel drag
  var dragHandle = document.getElementById('panel-drag-handle');
  var dragging = false, startX, startY, startLeft, startTop;
  dragHandle.addEventListener('mousedown', function(e) {
    dragging = true; startX = e.clientX; startY = e.clientY;
    startLeft = panel.offsetLeft; startTop = panel.offsetTop;
    dragHandle.style.cursor = 'grabbing'; e.preventDefault();
  });
  document.addEventListener('mousemove', function(e) {
    if (!dragging) return;
    panel.style.left = (startLeft + e.clientX - startX) + 'px';
    panel.style.top = (startTop + e.clientY - startY) + 'px';
    panel.style.right = 'auto';
  });
  document.addEventListener('mouseup', function() {
    dragging = false; dragHandle.style.cursor = 'grab';
  });

  function refreshSignals() {
    if (!active) return;
    if (window.__omni_debug && window.__omni_debug.getSignals) {
      var signals = window.__omni_debug.getSignals();
      var storeGroups = {};
      Object.keys(signals).forEach(function(k) {
        var parts = k.split('_');
        var store = parts.length > 1 ? parts[0] : '(global)';
        if (!storeGroups[store]) storeGroups[store] = [];
        storeGroups[store].push({ name: k, value: signals[k].value });
      });

      var html = '';
      Object.keys(storeGroups).sort().forEach(function(store) {
        html += '<div style="padding:2px 14px;font-size:10px;color:#6c7086;">' + store + '</div>';
        storeGroups[store].forEach(function(s) {
          var display = s.value === undefined ? 'undefined'
            : s.value === null ? 'null'
            : typeof s.value === 'object' ? JSON.stringify(s.value).substring(0, 100)
            : String(s.value);
          html += '<div class="signal-row"><span class="signal-name">' +
            s.name.replace(store + '_', '') +
            '</span><span class="signal-value">' + display + '</span></div>';
        });
      });
      signalList.innerHTML = html || '<div style="padding:6px 14px;color:#6c7086;">(no signals)</div>';

      // Show store list
      var storeNames = Object.keys(storeGroups);
      var actionHtml = '';
      storeNames.forEach(function(sn) {
        actionHtml += '<div class="action-row">📦 ' + sn + ' (' +
          storeGroups[sn].length + ' signals)</div>';
      });
      actionList.innerHTML = actionHtml || '<div style="padding:4px 14px;color:#6c7086;">(no stores)</div>';
    }
    scheduleRefresh();
  }

  function scheduleRefresh() { refreshTimer = setTimeout(refreshSignals, 500); }
  function stopRefresh() { if (refreshTimer) { clearTimeout(refreshTimer); refreshTimer = null; } }

  // Listen for compile results from main window
  if (window.__TAURI_INTERNALS__) {
    window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
      event: 'debug:compile-result',
      handler: window.__TAURI_INTERNALS__.transformCallback(function(e) {
        var payload = typeof e.payload === 'string' ? JSON.parse(e.payload) : e.payload;
        compileLog.textContent = 'Compiled: ' + new Date().toLocaleTimeString();

        // Apply CSS
        var styleEl = document.getElementById('omni-css');
        if (!styleEl) { styleEl = document.createElement('style'); styleEl.id = 'omni-css'; document.head.appendChild(styleEl); }
        styleEl.textContent = payload.css || '';

        // Replace body HTML
        document.getElementById('root').innerHTML = payload.html || '';

        // Execute JS — remove old script, create new one
        var oldScript = document.getElementById('omni-js');
        if (oldScript) oldScript.remove();
        if (payload.js) {
          var script = document.createElement('script');
          script.id = 'omni-js';
          script.textContent = payload.js;
          document.body.appendChild(script);
        }

        // Clear old highlights
        if (window.__omni_debug) window.__omni_debug.clearHighlight();

        // Refresh panel after JS executes
        setTimeout(function() { if (active) refreshSignals(); }, 200);
      })
    }).catch(function() {});

    // Notify main
    window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
      event: 'debug:window-ready', payload: {}
    }).catch(function() {});
  }
})();
</script>
</body>
</html>
```

- [ ] **Step 3b: Add Tauri commands for debug window management**

In `poiesis/src-tauri/src/lib.rs`, add these commands before the `run()` function (find the existing `#[tauri::command]` functions and add after the last one):

```rust
/// Open the standalone debug window for live OmniView preview.
#[tauri::command]
fn open_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    // If window already exists, just show it
    if let Some(window) = app.get_webview_window("debug") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(
        &app,
        "debug",
        tauri::WebviewUrl::App("debug-window/index.html".into()),
    )
    .title("Poiesis Debug")
    .inner_size(1400.0, 900.0)
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Close the debug window.
#[tauri::command]
fn close_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 3c: Register the new commands**

Find the `tauri::Builder::default()` invocation in `lib.rs` and add the two new commands to the `.invoke_handler()` chain:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    open_debug_window,
    close_debug_window,
])
```

- [ ] **Step 3d: Update `tauri.conf.json` to register the debug window**

In `poiesis/src-tauri/tauri.conf.json`, add a second window entry. The debug window is created programmatically, but we need to ensure the `app` security context allows inline scripts for the debug window. Add a `"security"` override — actually, the debug window inherits the app-level CSP. Update the app-level CSP to allow inline scripts:

```json
"security": {
  "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; form-action 'none'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'"
}
```

(Adding `'unsafe-inline'` to `script-src` and `style-src` is required because the compiled OmniView JS is injected as an inline `<script>` tag and the panel CSS is inline `<style>`.)

- [ ] **Step 3e: Add event permissions to capabilities**

In `poiesis/src-tauri/capabilities/default.json`, add `core:event:default` to the permissions array:

```json
"permissions": [
    "core:default",
    "core:event:default",
    // ... rest stays the same
]
```

- [ ] **Step 3f: Build Tauri app and verify**

```bash
cd poiesis && cargo build 2>&1 | tail -5
```

Expected: `Finished dev [unoptimized + debuginfo]`

- [ ] **Step 3g: Commit**

```bash
git add poiesis/src-tauri/debug-window/index.html \
        poiesis/src-tauri/src/lib.rs \
        poiesis/src-tauri/tauri.conf.json \
        poiesis/src-tauri/capabilities/default.json
git commit -m "feat: add open_debug_window/close_debug_window commands and static HTML shell"
```

---

### Task 4: Frontend — `useDebugWindow` hook + toolbar button

**Files:**
- Create: `poiesis/src/features/debug/useDebugWindow.ts`
- Create: `poiesis/src/features/debug/DebugWindowButton.tsx`
- Modify: `poiesis/src/components/shell/Toolbar.tsx`
- Modify: `poiesis/src/commands/rustCompiler.ts` (optional — add `debugMode` param to TypeScript types)
- Modify: `poiesis/src/commands/index.ts`

**Interfaces:**
- Produces: `useDebugWindow()` hook → `{ isOpen, open, close }`
- Consumes: Tauri `invoke("open_debug_window")`, `invoke("close_debug_window")`
- Consumes: Tauri `emit("debug:compile-result", payload)`, `listen("debug:element-click")`

- [ ] **Step 4a: Add command wrappers in `commands/index.ts`**

Find the `api` export object in `poiesis/src/commands/index.ts` and add:

```typescript
export const api = {
  // ... existing methods ...
  
  /** Open the debug window. Creates it if not already open. */
  openDebugWindow: () => invoke("open_debug_window"),
  
  /** Close the debug window. */
  closeDebugWindow: () => invoke("close_debug_window"),
};
```

- [ ] **Step 4b: Create `useDebugWindow.ts` hook**

Create `poiesis/src/features/debug/useDebugWindow.ts`:

```typescript
import { useState, useCallback, useEffect, useRef } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { api } from "../../commands";

interface CompilePayload {
  js: string;
  html: string;
  css: string;
  nidMap?: Record<string, string>;
}

interface ElementClickPayload {
  nid: string;
  tag: string;
}

export function useDebugWindow() {
  const [isOpen, setIsOpen] = useState(false);
  const nidMapRef = useRef<Record<string, string>>({});

  // Listen for element clicks from debug window
  useEffect(() => {
    const unlistenPromise = listen<ElementClickPayload>("debug:element-click", (event) => {
      const { nid } = event.payload;
      // Look up the design element ID from nid_map
      const designId = nidMapRef.current[nid];
      if (designId) {
        // Emit a custom event that the editor can listen to for element selection
        window.dispatchEvent(new CustomEvent("poiesis:debug-select", { detail: { nid, designId } }));
      }
    });
    return () => { unlistenPromise.then((fn) => fn()); };
  }, []);

  // Listen for window-ready from debug window
  useEffect(() => {
    const unlistenPromise = listen("debug:window-ready", () => {
      // Debug window is ready to receive compile results
      // The next compile cycle will push automatically
    });
    return () => { unlistenPromise.then((fn) => fn()); };
  }, []);

  const open = useCallback(async () => {
    try {
      await api.openDebugWindow();
      setIsOpen(true);
    } catch (e) {
      console.error("[DebugWindow] Failed to open:", e);
    }
  }, []);

  const close = useCallback(async () => {
    try {
      await api.closeDebugWindow();
      setIsOpen(false);
    } catch (e) {
      console.error("[DebugWindow] Failed to close:", e);
    }
  }, []);

  /** Push compiled OmniView output to the debug window. */
  const pushCompileResult = useCallback(async (payload: CompilePayload) => {
    if (!isOpen) return;
    // Cache nid_map for element click → design ID resolution
    if (payload.nidMap) {
      nidMapRef.current = payload.nidMap;
    }
    try {
      await emit("debug:compile-result", payload);
    } catch (e) {
      console.error("[DebugWindow] Failed to push compile result:", e);
    }
  }, [isOpen]);

  /** Send highlight command to debug window. nid=null to clear. */
  const highlight = useCallback(async (nid: string | null) => {
    if (!isOpen) return;
    try {
      await emit("debug:highlight", { nid });
    } catch (e) {
      console.error("[DebugWindow] Failed to highlight:", e);
    }
  }, [isOpen]);

  return { isOpen, open, close, pushCompileResult, highlight };
}
```

- [ ] **Step 4c: Create `DebugWindowButton.tsx`**

Create `poiesis/src/features/debug/DebugWindowButton.tsx`:

```typescript
import { useDebugWindow } from "./useDebugWindow";

export function DebugWindowButton() {
  const { isOpen, open, close } = useDebugWindow();

  return (
    <button
      onClick={isOpen ? close : open}
      title={isOpen ? "Close debug window" : "Open debug window"}
      style={{
        background: isOpen ? "#6366f1" : "transparent",
        color: isOpen ? "#fff" : "inherit",
        border: "1px solid #d4d4d8",
        borderRadius: 6,
        padding: "4px 10px",
        cursor: "pointer",
        fontSize: 14,
        display: "flex",
        alignItems: "center",
        gap: 4,
      }}
    >
      🐛 {isOpen ? "Debug On" : "Debug"}
    </button>
  );
}
```

- [ ] **Step 4d: Integrate button into Toolbar**

In `poiesis/src/components/shell/Toolbar.tsx`, import and add the button. Find the toolbar buttons area and add:

```tsx
import { DebugWindowButton } from "../../features/debug/DebugWindowButton";

// Inside the toolbar JSX, add where other action buttons are:
<DebugWindowButton />
```

- [ ] **Step 4e: Verify TypeScript compilation**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | head -20
```

Expected: no new errors (may show pre-existing errors from other files).

- [ ] **Step 4f: Commit**

```bash
git add poiesis/src/features/debug/useDebugWindow.ts \
        poiesis/src/features/debug/DebugWindowButton.tsx \
        poiesis/src/components/shell/Toolbar.tsx \
        poiesis/src/commands/index.ts
git commit -m "feat: add useDebugWindow hook and DebugWindowButton component"
```

---

### Task 5: Integration — wire compile output → push + element selection

**Files:**
- Modify: `poiesis/src/commands/rustCompiler.ts` (add debug_mode to TypeScript interface)
- Create: `poiesis/src/features/debug/useDebugCompile.ts`
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx:18`
- Modify: `poiesis/src/store/slices/element-crud.ts:95` (add debug highlight call)

**Interfaces:**
- Consumes: `useDebugWindow().pushCompileResult, highlight` from Task 4
- Consumes: WASM `compileMultiToOmniView(sources, editorMode, debugMode)` from Task 2
- Produces: `useDebugCompile` hook — watches elements/wires, compiles full output when debug window is open
- Produces: Automatic push of compile results on 150ms debounce
- Produces: `debug:element-click` → `selectElement(designId)` in editor
- Produces: `selectElement(id)` → `highlight(nid)` in debug window

- [ ] **Step 5a: Update `rustCompiler.ts` to pass `debugMode`**

In `poiesis/src/commands/rustCompiler.ts`, update the `RustCompiler` class. Find the WASM interface type (line 116-126) and update the `compile_multi_to_omniview` signature:

```typescript
  private wasm: {
    // ... existing ...
    compile_multi_to_omniview?: (sources: Record<string, string>, editor_mode: boolean, debug_mode: boolean) => any;
    // ... rest ...
  } | null = null;
```

Update the `compileMultiToOmniView` method (line 286):

```typescript
  compileMultiToOmniView(sources: Record<string, string>, editorMode: boolean = false, debugMode: boolean = false): MultiCompileResult {
    if (!this.wasm || !this.wasm.compile_multi_to_omniview) {
      return {
        success: false,
        files: {},
        errors: [{ severity: 'error' as const, code: 'WASM', message: 'compile_multi_to_omniview not available in WASM', file: null, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      return this.wasm.compile_multi_to_omniview(sources, editorMode, debugMode) as MultiCompileResult;
    } catch (e) {
      // ... error handling stays the same ...
    }
  }
```

Also update the WASM initialization (line 170) to match the new signature:

```typescript
        compile_multi_to_omniview: wasmModule.compile_multi_to_omniview as unknown as (sources: Record<string, string>, editor_mode: boolean, debug_mode: boolean) => any,
```

- [ ] **Step 5b: Add `compileForDebugWindow` method to `RustCompiler`**

In `poiesis/src/commands/rustCompiler.ts`, add a method that uses the multi-source compile pathway with `editorMode=true, debugMode=true`:

```typescript
  /** Compile multiple sources for debug window preview. Returns full OmniView output. */
  compileForDebugWindow(sources: Record<string, string>): { js: string; html: string; css: string } | null {
    const result = this.compileMultiToOmniView(sources, true, true);
    if (!result.success || !result.files) {
      log.warn('Debug window compile failed', result.errors);
      return null;
    }
    // compile_multi_to_omniview returns { files: { "index.js": ..., "index.html": ..., "index.css": ... } }
    const files = result.files;
    // Extract HTML body content (between <body> and </body>)
    const htmlContent = files['body.html'] || '';
    const htmlMatch = (files['index.html'] || '').match(/<body[^>]*>([\s\S]*)<\/body>/i);
    const bodyHtml = htmlMatch ? htmlMatch[1] : '';
    return {
      js: files['index.js'] || '',
      html: bodyHtml || htmlContent,
      css: files['index.css'] || '',
    };
  }
```

Also update the WASM signatures throughout:

- `compile_to_omniview` type: `(source: string, editor_mode: boolean, debug_mode: boolean) => any`
- `compile_to_omniview_full` type: `(source: string, editor_mode: boolean, debug_mode: boolean) => any`
- `compile_multi_to_omniview` type: `(sources: Record<string, string>, editor_mode: boolean, debug_mode: boolean) => any`
- `compileToOmniView(source, editorMode, debugMode)` — add 3rd param defaulting to `false`
- `compileToOmniViewFull(source, editorMode, debugMode)` — same
- `compileMultiToOmniView(sources, editorMode, debugMode)` — same

- [ ] **Step 5c: Create `useDebugCompile` hook**

Create `poiesis/src/features/debug/useDebugCompile.ts`:

```typescript
import { useEffect, useRef } from "react";
import { usePoiesisStore } from "../../store";
import { rustCompiler } from "../../commands/rustCompiler";
import { generateOmniFiles } from "../../services/codegen/pipeline";
import { useDebugWindow } from "./useDebugWindow";
import { useShallow } from "zustand/react/shallow";

/**
 * When debug window is open, re-compiles the full project (editor_mode=true, debug_mode=true)
 * on every canvas change (150ms debounce) and pushes result to the debug window.
 */
export function useDebugCompile() {
  const { isOpen, pushCompileResult } = useDebugWindow();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Watch the same elements/wires/rootOrder as useWasmCompile
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const rootOrder = usePoiesisStore((s) => s.rootOrder);
  const designElements = usePoiesisStore((s) => s.elements);
  const designWires = usePoiesisStore((s) => s.wires);

  useEffect(() => {
    if (!isOpen || !rustCompiler.isReady) return;
    if (elements.length === 0) return;

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      try {
        // Generate .omni sources from current editor state
        const result = generateOmniFiles({
          designElements,
          designWires,
          codeTabs: [],
          componentName: "main",
          rootOrder,
          // Pass any additional state generateOmniFiles needs
        });

        // Collect all .omni source files
        const sources: Record<string, string> = {};
        for (const [path, content] of Object.entries(result.files || {})) {
          sources[path] = content;
        }

        // Compile with debug flags
        const output = rustCompiler.compileForDebugWindow(sources);
        if (output) {
          pushCompileResult(output);
        }
      } catch (e) {
        console.error("[DebugCompile] Compile failed:", e);
      }
    }, 150);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [isOpen, elements, wires, rootOrder, pushCompileResult]);
}
```

- [ ] **Step 5d: Mount `useDebugCompile` in `OmniViewRenderer`**

In `poiesis/src/components/design/OmniViewRenderer.tsx`, add the hook call after the existing `useWasmCompile()`:

```typescript
export function OmniViewRenderer() {
  const { output, isCompiling, errors } = useWasmCompile();
  useDebugCompile();  // ← ADD: pushes to debug window when open
  
  // ... rest of component unchanged
}
```

Also add the import at the top:
```typescript
import { useDebugCompile } from "../../features/debug/useDebugCompile";
```

- [ ] **Step 5e: Handle `debug:element-click` → editor selection**

The `useDebugWindow` hook already listens for `debug:element-click` and dispatches `poiesis:debug-select` custom event with `{ nid, designId }`. Now wire it to the store's `selectElement` action.

In `poiesis/src/components/design/OmniViewRenderer.tsx` (or `DesignPage.tsx`), add:

```typescript
import { usePoiesisStore } from "../../store";

// Inside the component:
const selectElement = usePoiesisStore((s) => s.selectElement);

useEffect(() => {
  const handler = (e: Event) => {
    const { designId } = (e as CustomEvent).detail;
    if (designId) selectElement(designId);
  };
  window.addEventListener("poiesis:debug-select", handler);
  return () => window.removeEventListener("poiesis:debug-select", handler);
}, [selectElement]);
```

- [ ] **Step 5f: Handle editor selection → highlight in debug window**

In `poiesis/src/store/slices/element-crud.ts`, in the `selectElement` action (line 95), after the existing logic, dispatch a custom event so the debug window bridge can pick it up:

```typescript
selectElement: (id: string | null, multi = false) => set((s) => {
  // ... existing selection logic ...
  
  // Notify debug window of selection change
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent("poiesis:editor-select", { 
      detail: { elementId: id } 
    }));
  }
  
  return { selectedElementIds: /* ... */ };
}),
```

Then in `useDebugWindow.ts`, add a listener that maps `elementId` → `nid` and calls `highlight(nid)`:

```typescript
// In useDebugWindow hook, add:
useEffect(() => {
  const handler = (e: Event) => {
    const { elementId } = (e as CustomEvent).detail;
    if (!elementId) {
      highlight(null);  // clear highlight
      return;
    }
    // Reverse-lookup: find nid for the given design element ID
    const nidMap = nidMapRef.current;
    for (const [nid, designId] of Object.entries(nidMap)) {
      if (designId === elementId) {
        highlight(nid);
        return;
      }
    }
    // Not found — clear highlight
    highlight(null);
  };
  window.addEventListener("poiesis:editor-select", handler);
  return () => window.removeEventListener("poiesis:editor-select", handler);
}, [highlight]);
```

- [ ] **Step 5g: Manual test with poiesis running**

```bash
# Terminal 1: Start the dev server
cd poiesis && cargo tauri dev 2>&1 | head -30
```

Manual test steps:
1. Open a project in Poiesis
2. Click 🐛 "Debug" in toolbar → debug window opens
3. Verify OmniView render appears in debug window
4. Click 🔍 ball → verify panel shows Store signals
5. Click an element in debug window → verify it highlights in editor
6. Modify a style in editor → verify debug window updates (hot reload)
7. Close debug window → verify it closes cleanly

- [ ] **Step 5h: Commit**

```bash
git add poiesis/src/commands/rustCompiler.ts \
        poiesis/src/components/design/useWasmCompile.ts
git commit -m "feat: wire debug window compile push and element selection"
```

---

## Verification Checklist

After all tasks are complete, verify end-to-end:

- [ ] `omnilang` WASM builds with `debug_mode` parameter
- [ ] `cargo build` for poiesis succeeds
- [ ] `tsc --noEmit` shows no new TypeScript errors
- [ ] Debug window opens when 🐛 button clicked
- [ ] OmniView app renders correctly in debug window
- [ ] Float ball toggles debug panel
- [ ] Panel shows Store signal names and values (refreshes every 500ms)
- [ ] Clicking element in debug window (when debug mode ON) selects it in editor
- [ ] Selecting element in editor highlights it in debug window
- [ ] Editing a style property updates debug window within ~150ms
- [ ] Closing debug window does not crash Poiesis
- [ ] Re-opening debug window works after close
