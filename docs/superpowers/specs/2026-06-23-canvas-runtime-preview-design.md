# Canvas Runtime Preview — Design Spec

**Date:** 2026-06-23
**Status:** Draft
**Goal:** Replace the editor canvas mock-DOM rendering with live OmniView runtime preview via WASM-compiled output in an iframe.

---

## 1. Motivation

Currently, the visual editor's canvas (`LayoutView`) renders a simulated DOM using React components that approximate what the compiled output would look like. This has several problems:

- **Behavioral drift**: The canvas simulation doesn't execute signals, events, or animations — it's a static approximation
- **Double maintenance**: Style resolution, tag mapping, and component rendering logic exist in both the canvas renderer and the compiler codegen
- **No true WYSIWYG**: Users can't see actual runtime behavior (hover states, input bindings, `@for` rendering, `@if` toggling, animations)

The goal is to make the canvas render the **actual compiled OmniView runtime output** in an iframe, while retaining interactive editing capabilities.

## 2. Architecture Overview

```
┌─ Editor (React SPA) ───────────────────────────────────────────┐
│                                                                │
│  ActivityBar │  CanvasPanel (iframe)                           │
│              │  ┌──────────────────────────────────────────┐   │
│  Palette ────│──│→ drop creates element (retained)         │   │
│              │  │                                          │   │
│              │  │  OmniView Runtime                        │   │
│              │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐    │   │
│  Component   │  │  │ Header  │ │ Content │ │ Footer  │    │   │
│  Tree        │  │  │ nid=001 │ │ nid=002 │ │ nid=003 │    │   │
│  ────────    │  │  └─────────┘ └─────────┘ └─────────┘    │   │
│  reorder     │  │                                          │   │
│  reparent    │  │  ↕ postMessage bridge                   │   │
│  add/del     │  └──────────────────────────────────────────┘   │
│  right-click │                                                │
│              │  PropertyPanel ──→ postMessage ──→ runtime     │
│              │                                                │
│  ElementData[]  ──→ omni-gen ──→ WASM ──→ iframe srcdoc     │
│  (Zustand)         (TS pipe)     (in-browser)  (HTML+JS+CSS)  │
│                                                                │
│  OmniCompilerCache (TS)                                       │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  per-component: .omni fragment │ IR cache │ compiled JS  │ │
│  │  incremental: only recompile changed components          │ │
│  └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### Key principles

- **All changes go through omni-gen → WASM compile → iframe** — unified pipeline, single API
- **Component-level compilation** — change one component, recompile only that component
- **State preservation** — signal values snapshotted before hot-replace, restored after
- **Incremental caching** — TS-side cache for IR and compiled output per component

## 3. Canvas Preview: iframe + postMessage Bridge

### 3.1 Embedding method

The compiled OmniView output (JS + HTML + CSS) is merged into a single HTML string and loaded via `iframe.srcdoc`. No Vite dev server needed during editing.

### 3.2 postMessage protocol

```
Editor → Runtime:
─────────────────
omni-replace-component  { name, js }            Hot-replace a single component
omni-highlight          { nid }                 Highlight element
omni-clear-hl           {}                      Clear highlight

Runtime → Editor:
─────────────────
omni-select             { nid }                 User clicked element (→ editor resolves nid to elementId)
omni-hover              { nid }                 User hovering element
omni-rendered           { nidMap }              Runtime ready + nid→elementId mapping
omni-signal-changed     { signal, value }       Signal value changed (debug)
omni-event-fired        { event, payload }      Event fired (debug)
```

Note: `omni-rendered` carries the nid_map built by scanning the rendered DOM at runtime (see §3.3). No direct property/style bridge — all edits go through the compile pipeline (§6.1).

### 3.3 Runtime injection (editor mode only)

When `editor_mode: true` is passed to the WASM compiler, the generated code includes:

```js
// Signal registry for editor access
window.__omni_signals = {};
// Component registry for hot-replace
window.__omni_components = {};

// Build nid → elementId mapping from rendered DOM (id attr comes from omni-gen)
function __omni_buildNidMap() {
  const map = {};
  document.querySelectorAll('[data-nid]').forEach(el => {
    map[el.dataset.nid] = {
      elementId: el.id || '',
      tag: el.tagName.toLowerCase(),
      component: el.closest('[data-component]')?.dataset.component || '',
    };
  });
  return map;
}

// Click-to-select
document.addEventListener('click', (e) => {
  const el = e.target.closest('[data-nid]');
  if (el) {
    e.stopPropagation();
    parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, '*');
  }
});

// Receive editor commands
window.addEventListener('message', (e) => {
  switch (e.data.type) {
    case 'omni-replace-component': __omni_replaceComponent(e.data.name, e.data.js); break;
    case 'omni-highlight': __omni_editor.highlight(e.data.nid); break;
    case 'omni-clear-hl': __omni_editor.clearHighlight(); break;
  }
});

// Signal ready with nid_map
parent.postMessage({
  type: 'omni-rendered',
  nidMap: __omni_buildNidMap()
}, '*');

// Hot-replace a single component (detail in §7)
function __omni_replaceComponent(name, factoryCode) {
  // 1. Snapshot current signal values
  // 2. Teardown old: comp.teardown() removes DOM + clears effects
  // 3. Execute new factory IIFE: window.__omni_components[name] = (function(){...})();
  // 4. Restore signal values where names match
  // 5. Rebuild nid_map and notify editor
  // 6. Effects re-run, DOM updates
}
```

**nid_map rationale:** The HTML already has stable `id` attributes from omni-gen (e.g., `<div id="App.header.0" data-nid="el-div-001">`). Building the nid→elementId mapping at runtime avoids duplicating this information in the WASM codegen output, keeping the WASM surface simpler.

### 3.4 Canvas interaction

- **Click element in iframe** → postMessage `omni-select` → editor selects in tree + property panel
- **Property panel edit** → omni-gen → WASM compile component → postMessage `omni-replace-component`
- **Tree drag/reorder** → omni-gen → WASM compile affected components → postMessage replace
- **Palette drop onto canvas** → During drag, a transparent overlay div is placed on top of the iframe with `pointer-events: auto` to capture `dragover`/`drop` events (the iframe would otherwise consume them). The overlay reads `dataTransfer.getData("application/omni-component")`, creates element via `AddElement` command, triggers compile, then removes itself.

## 4. Component-Level Compilation

### 4.1 OmniView codegen changes (`omni-compiler/src/codegen/omniview.rs`)

**New struct fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `editor_mode` | `bool` | When true, inject editor bridge code |
| `component_js` | `HashMap<String, String>` | Per-component JS output (captured separately) |
| `current_component_js` | `String` | Buffer for current component being generated |

**`generate()` signature change:**
```rust
pub fn generate(ir: &IRProgram, editor_mode: bool) -> OmniViewOutput
```

**`OmniViewOutput` extension:**
```rust
pub struct OmniViewOutput {
    pub js: String,
    pub html: String,
    pub css: String,
    pub source_map: Vec<SourceMapEntry>,
    pub components: HashMap<String, ComponentOutput>,  // NEW
}

pub struct ComponentOutput {
    pub js: String,     // Component-specific JS (signals, view renderer, styles)
    pub html: String,   // Component HTML fragment
    pub css: String,    // Component-scoped CSS
}
```

Note: nid_map is NOT generated by the compiler. It is built at runtime by scanning `data-nid` elements and reading their `id` attributes (see §3.3). This avoids duplicating element ID information in the WASM output.

**Per-component JS capture:**
In `generate_component()`, swap `self.js` with `self.current_component_js` before emitting component code, then swap back and store result in `self.component_js`.

**Editor bridge injection:**
After `emit_dom_and_bindings()`, conditionally emit:
1. `window.__omni_signals` registration (map signal names to signal objects)
2. `window.__omni_components` registry (for factory function pattern, see §7.1)
3. postMessage listener for editor commands
4. `__omni_buildNidMap()` helper (scans DOM, returns nid→elementId mapping)
5. Click-to-select event listener
6. `omni-rendered` notification with nid_map

**nid_map: built at runtime, not in codegen.**
See §3.3 — the iframe runtime scans `[data-nid]` elements and reads their `id` attributes to build the mapping. No compiler changes needed for this.

### 4.2 WASM API changes

**New exports (in `lib.rs`):**
```rust
// Full project compile
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, options_json: &str) -> String;
// options_json: {"editor_mode": true}
// Returns JSON: { success, files: {js,html,css}, components: { Name: {js,html,css} }, source_map, errors }

// Single-component compile (incremental)
#[wasm_bindgen]
pub fn compile_omniview_component(source: &str, component_name: &str, options_json: &str) -> String;
// Returns JSON: { success, component: {js,html,css}, errors }
// source is the .omni fragment for a single component
```

**Existing export stays:**
```rust
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String;
// Unchanged — returns IR JSON for validation and caching
```

Note: `wasm-bindgen` does not support `bool` parameters natively. We use a JSON options string for extensibility. Future options may include `debug_mode`, `source_map_detail`, etc.

### 4.3 TS-side compiler cache

```ts
interface ComponentCacheEntry {
  omniFragment: string;       // The .omni source fragment for this component
  ir: CompileResult;          // WASM compile_to_ir() result (cached)
  compiledJS: string;         // WASM compile_to_omniview() output for this component
  signalNames: string[];      // List of signal names in this component (for state preservation)
  lastCompiled: number;       // Timestamp
}

class OmniCompilerCache {
  private components: Map<string, ComponentCacheEntry>;
  private runtimeJS: string;  // Signal runtime + animation runtime + stores (rarely changes)
  private nidMap: Record<string, { elementId: string; tag: string; component: string }>;  // From iframe omni-rendered

  async compileProject(elements: ElementData[]): Promise<CompiledOutput>;
  async recompileComponent(name: string, elements: ElementData[]): Promise<ComponentOutput>;
  updateNidMap(map: Record<string, unknown>): void;  // Called on omni-rendered
  lookupElementId(nid: string): string | null;
  invalidate(name: string): void;
  invalidateAll(): void;
}
```

**Incremental strategy:**
- Full compile on initial load or when store/module-level things change
- Component recompile when a single component's elements/props change
- `compile_to_ir()` result cached per component — if the .omni fragment hasn't changed, skip WASM entirely
- State preservation: before hot-replace, read signal values via postMessage; after replace, restore them

## 5. Component Tree Panel Redesign

### 5.1 Current state

`ComponentTree` (`editor/client/src/panels/ComponentTree.tsx`) has:
- Hierarchical tree view with expand/collapse
- Click to select element
- HTML5 native drag-and-drop for reorder/reparent

### 5.2 New capabilities

| Feature | Implementation | Triggers compile? |
|---------|---------------|-------------------|
| **Drag reorder** | Same-level reorder via `ReorderElement` command | Yes (structural) |
| **Drag reparent** | Move to another container via `MoveElement` command | Yes (structural) |
| **Right-click menu** | Context menu with add/delete/copy/paste/move/edit | — |
| **Add child component** | Right-click → submenu of component categories → `AddElement` | Yes (structural) |
| **Delete** | Right-click or Delete key → `RemoveElement` | Yes (structural) |
| **Copy/Paste** | Ctrl+C/V with recursive subtree copy | Yes (structural) |
| **Move up/down** | Ctrl+↑/↓ for sibling reorder | Yes (structural) |
| **Click select** | Selects element, highlights in iframe, opens property panel | No (postMessage) |
| **Edit custom component** | Double-click or right-click → opens sub-canvas via CanvasPanel | Yes (switch canvas) |

### 5.3 Right-click context menu

```
┌─────────────────────┐
│ 添加子组件 ▸        │  → submenu: Layout | Typography | Actions | Forms | Data | ...
│ 复制        Ctrl+C  │
│ 粘贴        Ctrl+V  │
│ 上移        Ctrl+↑  │
│ 下移        Ctrl+↓  │
│ 删除        Delete  │
│ ─────────────────── │
│ 编辑组件             │  (only for custom components)
└─────────────────────┘
```

### 5.4 Drag behavior change

**Removed from canvas:**
- DragHandle (`⋮⋮` — sort handle) on canvas elements
- MoveHandleWrapper (`⇄` — move handle) on canvas elements
- dnd-kit `DndContext` / `SortableContext` wrapping in `LayoutView`

**Moved to component tree (enhanced):**
- Drag within tree for reorder/reparent
- Visual drop indicators (before/after/inside)

**Retained on canvas wrapper:**
- HTML5 native DnD for palette → canvas drop (creating new components)

## 6. Data Flow

### 6.1 Full compile cycle

```
User action → Command (AddElement/RemoveElement/ReorderElement/MoveElement/SetProp)
  → Zustand state update
  → omni-gen pipeline (view-tree-builder → wire-analyzer → ... → source-emitter)
  → .omni source string
  → OmniCompilerCache.recompileComponent(name, source)
     → WASM compile_to_ir(fragment)          [skip if fragment unchanged]
     → WASM compile_omniview_component(...)   [skip if IR unchanged]
  → postMessage({ type: 'omni-replace-component', name, js })
  → iframe runtime: __omni_replaceComponent(name, js)
     → snapshot signal values
     → replace component DOM + logic
     → restore signal values
```

### 6.2 Future optimization: direct bridge

If compile latency becomes an issue for high-frequency operations (e.g., typing in a text field), a direct postMessage bridge can bypass the compile cycle:

```
Property panel: typing text
  → postMessage({ type: 'omni-set-text', nid, text })
  → iframe: el.textContent = text; signal.value = text;
```

This is NOT implemented in Phase 1-7. All edits go through the unified compile pipeline. Direct bridge is noted here as a potential future optimization path if needed.

## 7. Hot-Replace Mechanism

### 7.1 Code evaluation strategy

The component JS uses `const` declarations (e.g., `const App_count = __omni_signal(...)`). A naive `eval()` on hot-replace would throw `SyntaxError: Identifier 'App_count' has already been declared`.

**Solution: factory function pattern (editor mode only)**

When `editor_mode: true`, each component is emitted as an assignable factory instead of bare declarations:

```js
// Normal mode:
//   const App_count = __omni_signal("App_count", 0);
//   function App_render() { ... }

// Editor mode:
window.__omni_components["App"] = (function() {
  const App_count = __omni_signal("App_count", 0);
  function App_render() { ... }
  return {
    signals: { App_count },
    render: App_render,
    teardown() { /* remove DOM, clear effects */ },
  };
})();
```

This avoids `eval()` entirely, provides clean encapsulation, and makes `__omni_replaceComponent()` straightforward.

### 7.2 State preservation flow

When `__omni_replaceComponent(name, factoryCode)` is called:

1. **Snapshot**: Iterate `window.__omni_signals`, filter by component prefix, save `{ name: signal.peek() }`
2. **Teardown old**: Call old component's `teardown()` — removes DOM subtree, clears effects
3. **Execute new factory**: The factory code is an IIFE that self-executes: `window.__omni_components[name] = (function() { ... })();`. The editor sends the full factory source as a `<script>` injection or via `new Function()` for scoped execution. No `eval()` of arbitrary JS — the factory pattern is self-contained.
4. **Restore**: For each saved signal name that exists in new `comp.signals`, `signal.set(savedValue)`
5. **Re-render**: Effects re-run, DOM updates
6. **Rebuild nid_map**: Scan new DOM, send `omni-rendered` with updated mapping to editor

Edge cases:
- New signals added: keep default value
- Signals removed: discard saved value
- Signal renamed: cannot preserve (treated as delete + add)
- Component added/removed: managed by root component hot-replace

## 8. omni-compiler Changes (scope: omniview only)

### 8.1 Files to modify

| File | Changes |
|------|---------|
| `omni-compiler/Cargo.toml` | Add `wasm-bindgen`, `serde-wasm-bindgen` optional deps; add `wasm` feature |
| `omni-compiler/src/lib.rs` | Add `#[wasm_bindgen]` exports: `compile_to_ir`, `compile_to_omniview`, `compile_omniview_component` |
| `omni-compiler/src/codegen/omniview.rs` | Main changes (see §4.1) |

### 8.2 Not modified

- All other codegen targets (react, vue, flutter, dart, python, rust, javascript, dioxus)
- Parser, lexer, semantic analysis
- IR types (no changes needed)
- CLI (`omnilang/`)

## 9. Editor Changes

### 9.1 Files to modify

| File | Changes |
|------|---------|
| `editor/client/src/canvas/LayoutView.tsx` | Replace mock-DOM render with iframe wrapper; remove dnd-kit sort/move |
| `editor/client/src/canvas/CanvasPreview.tsx` | **NEW** — iframe wrapper with srcdoc management + postMessage bridge |
| `editor/client/src/panels/ComponentTree.tsx` | Add right-click menu, keyboard shortcuts, enhanced drag |
| `editor/client/src/panels/PropertyPanel.tsx` | Connect to postMessage bridge; signal-based prop editing |
| `editor/client/src/api/rustCompiler.ts` | Update WASM API to call `compile_to_omniview(source, editor_mode)` |
| `editor/client/src/omni-gen/pipeline.ts` | Add `generateOmniFragment(name, elements)` for single-component |
| `editor/client/src/omni-compiler-cache.ts` | **NEW** — TS-side compilation cache |
| `editor/client/src/store/editor.ts` | Ensure element → component mapping is accessible |
| `editor/client/src/App.tsx` | Wire new compile cycle |

### 9.2 Files to remove/archive

| File | Reason |
|------|--------|
| `editor/client/src/canvas/CustomComponentRenderer.tsx` | No longer needed (runtime handles component rendering) |
| `editor/client/src/canvas/WireOverlay.tsx` | Wires rendered as source annotations, not canvas overlay |

## 10. Implementation Phases

### Phase 1: WASM Foundation
- Merge WASM bindings from `learn-omniview-phase-1` worktree into main
- Add `compile_to_omniview(source, options_json)` WASM export
- Add `compile_omniview_component(source, name, options_json)` WASM export
- Build and verify WASM loads in editor
- Add `OmniViewOutput` extensions (components map)
- **Gate:** 463 existing compiler tests still pass; WASM loads in browser

### Phase 2: Editor Bridge Injection (omniview.rs)
- Add `editor_mode` parameter to `OmniViewGenerator`
- Generate components as factory functions (not bare declarations) when editor_mode=true
- Generate postMessage listener + `__omni_signals` registry
- Generate `__omni_replaceComponent()` function
- Verify bridge works with a standalone test iframe
- **Gate:** `editor_mode=true` output valid JS; `editor_mode=false` output unchanged

### Phase 3: Component Tree Redesign
- Add right-click context menu (add child / copy / paste / move up / move down / delete / edit component)
- Add keyboard shortcuts (Delete, Ctrl+C/V, Ctrl+↑↓)
- Enhance drag for sort (same-level reorder) and reparent (move to another container)
- Visual drop indicators (before/after/inside)
- **Gate:** All tree operations update Zustand correctly via Command pattern

### Phase 4: Canvas → iframe
- Create `CanvasPreview` component (iframe wrapper with srcdoc management)
- Build transparent drag overlay for palette drops
- Replace `LayoutView` mock-DOM render with iframe + overlay
- Wire omni-gen → WASM → srcdoc pipeline
- Remove dnd-kit DragHandle/MoveHandleWrapper from canvas
- Implement click-to-select via postMessage (iframe → editor)
- **Gate:** Palette drop still works; click select works; compile output renders in iframe

### Phase 5: Component-Level Compilation
- Capture per-component JS in `OmniViewGenerator` (component_js map)
- Build `OmniCompilerCache` in TS
- Implement incremental omni-gen (`generateOmniFragment`)
- Implement hot-replace with state preservation (factory function pattern)
- **Gate:** Modifying one component's property only recompiles that component; signal values preserved

### Phase 6: Property Panel → Compile Bridge
- Connect property edits to compile cycle (SetProp → omni-gen → WASM → iframe)
- nid→elementId lookup from nid_map for selection sync
- Style editing via compile
- **Gate:** Changing a prop in property panel updates the iframe rendering

### Phase 7: Polish & Performance
- DOM diff for component replacement (reduce visual flicker)
- IR caching validation (skip WASM when .omni fragment unchanged)
- Debounce compile (300ms) for slider/text-input continuous edits
- Error handling: compile failures → show in editor toast, don't crash iframe
- Restore existing iframe state on compile error (keep previous working output)
- Performance profiling for 50+ component projects

## 11. Testing Strategy

### Existing tests must continue passing
- All 463 omni-compiler tests (codegen, parser, lexer, semantic, IR)
- `editor_mode: false` must produce byte-identical output to before the changes
- All existing omni-gen tests

### Unit tests (Rust)
- `omniview.rs`: editor_mode on/off output comparison
- `omniview.rs`: factory function wrapping in editor mode
- `omniview.rs`: per-component JS capture correctness
- `omniview.rs`: `__omni_replaceComponent()` helper generation
- `lib.rs`: WASM exports return valid JSON

### Unit tests (TypeScript)
- `OmniCompilerCache`: cache hit/miss, invalidation
- postMessage protocol serialization/deserialization
- omni-gen incremental fragment generation

### Integration tests
- Full cycle: ElementData[] → omni-gen → WASM → srcdoc → iframe load
- Hot-replace: modify one component → recompile → replace → verify state preserved
- Editor bridge: click in iframe → selection in tree/property panel

### E2E tests (existing Playwright)
- Verify all 19 test components still render correctly
- Verify editor interactions (click select, property edit, tree reorder)

## 12. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| WASM binary too large | Slow load | Already 296KB, acceptable; lazy-load on first edit |
| Compile latency on every keystroke | Sluggish editing | Debounce compile (300ms); direct bridge for text input |
| Signal state loss on hot-replace | UX frustration | Snapshot/restore mechanism; test thoroughly |
| iframe sandbox restrictions | postMessage blocked | Same-origin srcdoc iframe has no sandbox by default |
| omni-gen generates different .omni on re-run | Cache invalidated unnecessarily | Deterministic element ordering; stable path generation |
