# Poiesis Debug Window — Design Spec

**Date:** 2026-07-11
**Status:** Approved
**Goal:** Add a standalone debug window to Poiesis for live OmniView preview + runtime inspection.

## Motivation

Poiesis currently has no dedicated debugging facility. The canvas preview is embedded in DesignPage and tightly coupled to the editor. A separate debug window provides:

1. Full-screen preview of compiled OmniView output (HTML/CSS/JS)
2. Bidirectional element highlight (click in debug window → selects in editor, and vice versa)
3. Runtime Store/signal inspection (live values, action execution traces)

## User Flow

1. User clicks **🐛 "Open Debug Window"** button in Poiesis toolbar
2. Tauri creates a new `debug` webview window (full-screen, 1400×900 default)
3. Main window's WASM compiles current project → sends `{ js, html, css }` to debug window via Tauri events
4. Debug window renders the OmniView app, with a **floating green ball** 🟢 in the top-right corner
5. Clicking the ball toggles **debug mode ON**:
   - Ball expands into a **draggable inspector panel** showing Store signals and actions
   - Clicking any element in the preview emits its nid to the main window (highlights in editor)
   - Main window can send highlight commands (element selected in editor → highlighted in debug window)
6. Closing the debug window destroys the webview
7. Hot update: subsequent edits in the editor re-compile and push updates to the debug window in real-time (150ms debounce)

## Architecture

```
┌─ Poiesis 主窗口 (main) ──────────────┐     ┌─ 调试窗口 (debug) ─────────────────┐
│                                       │     │                                   │
│  Toolbar [🐛 打开调试]                 │     │  ┌─ OmniView 渲染 (全屏) ──────┐  │
│    ↓ Tauri: create_window("debug")    │     │  │ data-nid="el-btn-001"         │  │
│                                       │     │  │   ↓ click → emit event       │  │
│  useWasmCompile (150ms debounce)      │     │  └──────────────────────────────┘  │
│    ↓ { js, html, css }                │     │                              🟢     │
│                                       │     │  emit_to←         emit→  ┌──────┐ │
│  app.emit_to("debug",        ────────→│     │  "debug:          "debug: │📊    │ │
│    "debug:compile-result")            │     │  highlight"       element │signals│ │
│                                       │     │                   -click" │actions│ │
│  listen("debug:element-click") ←──────│─────│                          └──────┘ │
│    → selectElementInEditor(pid)       │     │                                   │
└───────────────────────────────────────┘     └───────────────────────────────────┘
```

## Three-Layer Implementation

### Layer 1: Rust Codegen (`omnilang/src/codegen/omniview/`)

Add a `debug_mode` flag to the WASM code generator. When enabled, emit additional JS bridge code after the existing `editor_mode` bridge.

**Changes:**

- `OmniViewGenerator` struct: add `debug_mode: bool`
- `generate()` function signature: accept `debug_mode: bool` parameter
- New file `debug.rs`: `emit_debug_bridge()` method (~80 lines)

**What `emit_debug_bridge()` emits (conceptual):**

```javascript
/// ── Debug Bridge ──
(function() {
  // Override click handler: use Tauri events instead of parent.postMessage
  document.addEventListener('click', function(e) {
    var el = e.target.closest('[data-nid]');
    if (!el || !window.__omni_debug || !window.__omni_debug._active) return;
    e.stopPropagation();
    window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
      event: 'debug:element-click',
      payload: { nid: el.dataset.nid, tag: el.tagName.toLowerCase() }
    });
  });

  // Listen for highlight commands from main window
  window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
    event: 'debug:highlight',
    handler: window.__TAURI_INTERNALS__.transformCallback(function(e) {
      var payload = JSON.parse(e.payload);
      // clear previous highlight
      document.querySelectorAll('[data-nid]').forEach(function(el) {
        el.style.outline = '';
      });
      // set new highlight
      if (payload.nid) {
        var el = document.querySelector('[data-nid="' + payload.nid + '"]');
        if (el) el.style.outline = '2px solid #4F46E5';
      }
    })
  });

  // Expose debug inspection API
  window.__omni_debug = {
    _active: false,
    toggle: function() {
      this._active = !this._active;
      // signal postMessage to panel UI
      window.dispatchEvent(new CustomEvent('debug:toggle', { detail: this._active }));
    },
    isActive: function() { return this._active; },
    getSignals: function() {
      var result = {};
      if (window.__omni_signals) {
        Object.keys(window.__omni_signals).forEach(function(k) {
          var s = window.__omni_signals[k];
          result[k] = { value: s.peek ? s.peek() : s, name: k };
        });
      }
      return result;
    },
    getComponents: function() {
      return Object.keys(window.__omni_components || {});
    },
    getNidMap: function() {
      return (typeof __omni_buildNidMap === 'function')
        ? __omni_buildNidMap() : {};
    }
  };

  // Report ready
  window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
    event: 'debug:ready',
    payload: { nidMap: window.__omni_debug.getNidMap() }
  });
})();
```

**WASM function signature change:**

```rust
// Before
pub fn generate(ir: &IRProgram, editor_mode: bool, css_only: bool) -> OmniViewOutput

// After
pub fn generate(ir: &IRProgram, editor_mode: bool, css_only: bool, debug_mode: bool) -> OmniViewOutput
```

`editor_mode` must be `true` when `debug_mode` is `true` (debug mode requires `data-nid` attributes and the signal registry).

### Layer 2: Tauri Backend (`poiesis/src-tauri/src/lib.rs`)

**New commands:**

```rust
#[tauri::command]
fn open_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    let debug_html = include_str!("../debug-window/index.html");
    WebviewWindowBuilder::new(&app, "debug",
        tauri::WebviewUrl::App("debug-window/index.html".into()))
        .title("Poiesis Debug")
        .inner_size(1400.0, 900.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn close_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

**CSP update** (`tauri.conf.json`): The debug window needs to allow inline scripts (the compiled OmniView JS is inline). Consider using a separate CSP for the debug window or adding `'unsafe-inline'` for `script-src`.

**Capabilities**: Ensure `core:event:default` includes `allow-emit`, `allow-emit-to`, `allow-listen`.

### Layer 3: Poiesis Frontend (`poiesis/src/`)

**New files:**

| File | Purpose |
|------|---------|
| `src/features/debug/useDebugWindow.ts` | Hook: manage debug window lifecycle + compilation push |
| `src/features/debug/DebugWindowButton.tsx` | Toolbar button component |

**Static asset:**

| File | Purpose |
|------|---------|
| `poiesis/src-tauri/debug-window/index.html` | Debug window shell (vanilla JS float ball + panel UI) |

**`useDebugWindow.ts` logic:**

```
openDebugWindow():
  1. invoke("open_debug_window")
  2. compile current project → { js, html, css }
  3. emit_to("debug", "debug:compile-result", result)
  4. set isOpen = true

onCompileChange (triggered by useWasmCompile):
  if isOpen:
    emit_to("debug", "debug:compile-result", latestCompileOutput)

onElementClick (listener):
  listen("debug:element-click") → deselectAll, selectElement by pid match

closeDebugWindow():
  invoke("close_debug_window")
  set isOpen = false
```

**`DebugWindowButton.tsx`:**

Toolbar button, toggles debug window. Shows active state when window is open.

**`debug-window/index.html` structure:**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Poiesis Debug</title>
  <style>
    /* Reset + debug ball + panel styles (~120 lines) */
    .debug-ball { position: fixed; top: 16px; right: 16px;
      width: 40px; height: 40px; border-radius: 50%;
      background: #22c55e; cursor: pointer; z-index: 99999;
      box-shadow: 0 2px 8px rgba(0,0,0,0.2); display: flex;
      align-items: center; justify-content: center; font-size: 18px; }
    .debug-ball.active { background: #6366f1; }
    .debug-panel { position: fixed; top: 64px; right: 16px;
      width: 280px; max-height: 500px; background: #1e1e2e; color: #cdd6f4;
      border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.4);
      z-index: 99998; overflow: hidden; display: none; }
    .debug-panel.visible { display: block; }
    .panel-header { padding: 8px 12px; font-size: 12px;
      font-weight: 600; text-transform: uppercase; color: #a6adc8;
      border-bottom: 1px solid #313244; letter-spacing: 0.5px; }
    .panel-drag-handle { cursor: grab; user-select: none; }
    .signal-row { padding: 6px 12px; font-size: 13px;
      font-family: 'JetBrains Mono', monospace; display: flex;
      justify-content: space-between; align-items: center; }
    .signal-row:hover { background: #313244; }
    .signal-name { color: #89b4fa; }
    .signal-value { color: #a6e3a1; max-width: 140px;
      overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .action-row { padding: 4px 12px; font-size: 12px;
      color: #f9e2af; }
  </style>
</head>
<body>
  <div id="root"></div>

  <!-- Debug Ball -->
  <div id="debug-ball" class="debug-ball" title="Toggle debug mode">🔍</div>

  <!-- Debug Panel -->
  <div id="debug-panel" class="debug-panel">
    <div class="panel-header panel-drag-handle" id="panel-drag-handle">
      📊 Store Inspector
    </div>
    <div id="signal-list"></div>
    <div class="panel-header">📋 Actions</div>
    <div id="action-list"></div>
    <div class="panel-header">🔄 Compile Log</div>
    <div id="compile-log" style="font-size:11px; padding:4px 12px; color:#6c7086;"></div>
  </div>

  <script>
    // ═══ Float Ball + Panel UI (~200 lines vanilla JS) ═══
    (function() {
      var ball = document.getElementById('debug-ball');
      var panel = document.getElementById('debug-panel');
      var signalList = document.getElementById('signal-list');
      var actionList = document.getElementById('action-list');
      var compileLog = document.getElementById('compile-log');
      var active = false;

      // Toggle panel
      ball.addEventListener('click', function() {
        active = !active;
        ball.classList.toggle('active', active);
        panel.classList.toggle('visible', active);
        if (active) {
          refreshSignals();
          // Notify compiled JS that debug mode is active
          window.dispatchEvent(new CustomEvent('debug:toggle', { detail: true }));
        } else {
          window.dispatchEvent(new CustomEvent('debug:toggle', { detail: false }));
        }
      });

      // Panel drag
      var dragHandle = document.getElementById('panel-drag-handle');
      var dragging = false, offsetX, offsetY;
      dragHandle.addEventListener('mousedown', function(e) {
        dragging = true;
        offsetX = e.clientX - panel.offsetLeft;
        offsetY = e.clientY - panel.offsetTop;
        dragHandle.style.cursor = 'grabbing';
      });
      document.addEventListener('mousemove', function(e) {
        if (!dragging) return;
        panel.style.left = (e.clientX - offsetX) + 'px';
        panel.style.top = (e.clientY - offsetY) + 'px';
        panel.style.right = 'auto';
      });
      document.addEventListener('mouseup', function() {
        dragging = false;
        dragHandle.style.cursor = 'grab';
      });

      // Signal refresh
      function refreshSignals() {
        if (!active || !window.__omni_debug) return;
        var signals = window.__omni_debug.getSignals();
        var html = '';
        Object.keys(signals).forEach(function(k) {
          var v = signals[k].value;
          var display = typeof v === 'object' ? JSON.stringify(v).substring(0, 80) : String(v);
          html += '<div class="signal-row">' +
            '<span class="signal-name">' + k + '</span>' +
            '<span class="signal-value">' + display + '</span>' +
            '</div>';
        });
        signalList.innerHTML = html || '<div style="padding:6px 12px;color:#6c7086;">(no signals)</div>';

        // Also update action list from window.__omni_signals (actions are functions)
        var allKeys = Object.keys(window.__omni_signals || {});
        var actionHtml = '';
        var storeNames = new Set();
        allKeys.forEach(function(k) {
          var parts = k.split('_');
          if (parts.length >= 2) storeNames.add(parts[0]);
        });
        storeNames.forEach(function(sn) {
          actionHtml += '<div class="action-row">📦 ' + sn + '</div>';
        });
        actionList.innerHTML = actionHtml || '<div style="padding:4px 12px;color:#6c7086;">(no actions)</div>';

        scheduleRefresh();
      }

      function scheduleRefresh() {
        if (active) setTimeout(refreshSignals, 500);
      }

      // Listen for compile results from main window
      if (window.__TAURI_INTERNALS__) {
        window.__TAURI_INTERNALS__.invoke('plugin:event|listen', {
          event: 'debug:compile-result',
          handler: window.__TAURI_INTERNALS__.transformCallback(function(e) {
            var payload = JSON.parse(e.payload);
            compileLog.textContent = 'Last compile: ' + new Date().toLocaleTimeString();
            // Clear previous app
            document.getElementById('root').innerHTML = '';
            // Apply CSS
            var existingStyle = document.getElementById('omni-style');
            if (!existingStyle) {
              existingStyle = document.createElement('style');
              existingStyle.id = 'omni-style';
              document.head.appendChild(existingStyle);
            }
            existingStyle.textContent = payload.css;
            // Insert HTML
            document.getElementById('root').innerHTML = payload.html;
            // Execute JS (must re-create script context)
            var oldScript = document.getElementById('omni-script');
            if (oldScript) oldScript.remove();
            var script = document.createElement('script');
            script.id = 'omni-script';
            script.textContent = payload.js;
            document.body.appendChild(script);
            // Refresh panel after new app is loaded
            setTimeout(refreshSignals, 100);
          })
        });
        // Notify main window we're ready
        window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
          event: 'debug:window-ready', payload: {}
        });
      }
    })();
  </script>

  <!-- Compiled OmniView JS injected here on first compile-result -->
</body>
</html>
```

## Communication Protocol

All events go through Tauri's built-in event bus.

| Event | Direction | Payload | When |
|-------|-----------|---------|------|
| `debug:window-ready` | debug → main | `{}` | Debug window loaded |
| `debug:compile-result` | main → debug | `{ js, html, css }` | After compilation (initial + hot updates) |
| `debug:element-click` | debug → main | `{ nid, tag }` | User clicks element in debug mode |
| `debug:highlight` | main → debug | `{ nid }` | Editor selects element (null nid = clear) |
| `debug:store-snapshot` | main → debug | `{ signals: Record<string, any> }` | Store state changed (future phase) |

## Implementation Phases

### Phase 1: Core Pipeline (this spec)
1. Add `debug_mode` to WASM `generate()` + `emit_debug_bridge()` in codegen
2. Tauri `open_debug_window` / `close_debug_window` commands
3. Debug window static `index.html` with ball + panel UI
4. Toolbar button + `useDebugWindow` hook
5. Compile → push → render loop

### Phase 2: Bidirectional Highlight
6. `debug:element-click` → editor selection
7. `debug:highlight` ← editor selection

### Phase 3: Inspection Polish (future)
8. Real-time signal polling panel
9. Action execution traces
10. Breakpoint support (TBD)

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Vanilla JS for panel UI | Panel is ~200 lines; no React bundle needed; fast startup; isolated failures |
| `debug_mode` is a codegen flag (not runtime patch) | Clean separation; generated JS is self-contained; easy to test |
| Reuse `editor_mode` infrastructure | `data-nid` + signal registry already work; only need to swap communication layer |
| Tauri events over postMessage | Native IPC, more reliable, structured data, survives component rerenders |
| `window.__TAURI_INTERNALS__` over `@tauri-apps/api` | No bundler needed in debug window; always available in Tauri v2 webviews |
| Hot update via re-compilation (not hot-replace) | Simpler first version; hot-replace (`__omni_replaceComponent`) can be added later |

## Implementation Notes

### Signal Access Pattern

The current runtime stores signals in a closure-private `__omni_subs` Map (see `runtime.rs` L21). The debug panel needs access to signal values. **Fix:** In `emit_signal_runtime()`, when `debug_mode=true`, expose the registry:

```javascript
// Add to the last line of emit_signal_runtime():
window.__omni_signal = __omni_signal;
window.__omni_computed = __omni_computed;
window.__omni_effect = __omni_effect;
// In debug mode, also expose the signal registry:
window.__omni_subs = __omni_subs;    // ← added for debug
```

Then `getSignals()` can read `window.__omni_subs` (Map) directly:

```javascript
getSignals: function() {
  var result = {};
  if (window.__omni_subs) {
    window.__omni_subs.forEach(function(signal, name) {
      result[name] = { value: signal.peek(), name: name };
    });
  }
  return result;
}
```

### CSS Scoping

The existing `editor_mode` design CSS emission uses unscoped selectors (line 307 of mod.rs). Debug window needs the same unscoped CSS because there's no `data-design` attribute on the root element. This is already handled by `editor_mode=true`.

### editor_mode + debug_mode Requirement

`debug_mode` requires `editor_mode=true` (asserted at the `generate()` call site). Without `editor_mode`, there are no `window.__omni_signals` globals or `data-nid` attributes, and the debug bridge has nothing to work with.

### Dev-only static file

`poiesis/src-tauri/debug-window/index.html` is a development-time static asset. It is NOT generated by the WASM compiler — only the inner `<script>` payload is. The shell HTML (float ball, panel CSS/JS) is hand-maintained.

### Phase 2 Highlight: nid → PID mapping

When `debug:element-click` sends `{ nid: "el-button-003" }`, the main window needs to map nid → design element ID to select the correct element in the editor. The existing `nid_map` (returned in `OmniViewOutput`) provides this mapping. It should be sent alongside the compile result or cached in the `useDebugWindow` hook.

## Non-Goals (for this spec)
- Breakpoint/step-through debugging
- Monaco code editor in debug window
- Performance profiling
- Network request inspection
- Multi-target debugging (debugging React/Flutter output)
