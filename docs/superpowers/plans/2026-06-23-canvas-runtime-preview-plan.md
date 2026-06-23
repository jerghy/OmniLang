# Canvas Runtime Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the editor canvas mock-DOM rendering with live OmniView runtime preview via WASM-compiled output in an iframe, with component-level compilation and hot-replace.

**Architecture:** Editor generates `.omni` source via omni-gen → WASM compiler (in-browser) produces OmniView JS/HTML/CSS → loaded into iframe via `srcdoc` → postMessage bridge for click-to-select and component hot-replace. Component tree panel becomes the primary drag-and-drop surface for structural edits.

**Tech Stack:** Rust (omni-compiler, wasm-bindgen), TypeScript/React (editor client), postMessage (iframe bridge)

## Global Constraints

- `editor_mode: false` must produce byte-identical output to before changes
- All 463 existing omni-compiler tests must continue passing
- All existing omni-gen tests must continue passing
- Only modify `omni-compiler/src/codegen/omniview.rs`; do not touch other codegen targets
- wasm-bindgen does not support `bool` parameters — use JSON options strings
- nid_map is built at runtime from DOM `id` attributes, not from codegen

---

### Task 1: Merge WASM bindings into main branch

**Files:**
- Modify: `omni-compiler/Cargo.toml`
- Modify: `omni-compiler/src/lib.rs`
- Modify: `omni-compiler/src/codegen/omniview.rs:30-30` (generate signature)

**Interfaces:**
- Consumes: (nothing — first task)
- Produces: `pub fn compile_to_ir(source: &str) -> String` (WASM export)
- Produces: `pub fn version() -> String` (WASM export)

- [ ] **Step 1: Add WASM dependencies to Cargo.toml**

```toml
# In [dependencies] section, add after serde_json:
# WASM support
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }

# In [features] section, replace with:
[features]
default = []
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
native = []
```

- [ ] **Step 2: Add WASM exports to lib.rs**

Add after existing `use` statements at line 19:
```rust
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
```

Add after `compile()` function (before line 90):
```rust
/// Compile .omni source to IR JSON. Returns JSON string.
/// This is the WASM entry point and also used by native FFI.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String {
    let result = compile(source);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"ir":null,"errors":[{{"message":"{}"}}]}}"#,
            e
        )
    })
}

/// Return compiler version.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
```

- [ ] **Step 3: Verify native build still works**

```bash
cd omni-compiler && cargo build 2>&1
```
Expected: builds cleanly (WASM deps are optional, not compiled without `--features wasm`).

- [ ] **Step 4: Run existing tests to verify no regressions**

```bash
cd omni-compiler && cargo test 2>&1
```
Expected: all 463 tests pass.

- [ ] **Step 5: Build WASM with wasm-pack**

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm 2>&1
```
Expected: generates `omni_compiler.js`, `omni_compiler_bg.wasm`, `omni_compiler.d.ts`, `omni_compiler_bg.wasm.d.ts` in `editor/public/wasm/`.

- [ ] **Step 6: Verify WASM exports include compile_to_ir and version**

```bash
grep -E "compile_to_ir|version" editor/public/wasm/omni_compiler.d.ts
```
Expected: both functions appear in declarations.

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/Cargo.toml omni-compiler/src/lib.rs editor/public/wasm/
git commit -m "feat(wasm): add WASM bindings for compile_to_ir and version"
```

---

### Task 2: Add compile_to_omniview WASM export

**Files:**
- Modify: `omni-compiler/src/lib.rs` (add WASM export + extend OmniViewResult)
- Modify: `omni-compiler/src/codegen/omniview.rs:18-27` (extend OmniViewOutput)
- Modify: `omni-compiler/src/codegen/omniview.rs:30-30` (generate signature)

**Interfaces:**
- Consumes: `compile_omniview(source) -> OmniViewResult` from Task 1
- Produces: `compile_to_omniview(source: &str, options_json: &str) -> String` (WASM export)
- Produces: `OmniViewOutput { ..., components: HashMap<String, ComponentOutput> }`
- Produces: `ComponentOutput { js, html, css }`

- [ ] **Step 1: Add ComponentOutput struct and extend OmniViewOutput in omniview.rs**

After `OmniViewOutput` struct (line 27), add:
```rust
/// Per-component compiled output for hot-replace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOutput {
    pub js: String,
    pub html: String,
    pub css: String,
}
```

Add `Serialize, Deserialize` derive to `OmniViewOutput` if not present, and add `components` field:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniViewOutput {
    pub js: String,
    pub html: String,
    pub css: String,
    pub source_map: Vec<SourceMapEntry>,
    #[serde(default)]
    pub components: HashMap<String, ComponentOutput>,
}
```

Also add the `use` import for Serialize/Deserialize/Serialize at the top of omniview.rs:
```rust
use serde::{Deserialize, Serialize};
```

And import HashMap:
```rust
use std::collections::HashMap;
```
(Check if already imported; if not, add.)

- [ ] **Step 2: Extend OmniViewResult with components field in lib.rs**

Add `components` to `OmniViewResult`:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<HashMap<String, ComponentOutput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}
```

Import `ComponentOutput` from codegen and `HashMap`:
```rust
use std::collections::HashMap;
use crate::codegen::omniview::ComponentOutput;
```

Populate `components` in `compile_omniview()`:
```rust
    OmniViewResult {
        success: true,
        files: Some(OmniViewFiles { ... }),
        components: Some(output.components),
        source_map: Some(output.source_map),
        errors,
    }
```

- [ ] **Step 3: Update generate() to populate empty components map**

In `generate()` at the return statement (line 106-111), add `components: HashMap::new()`:
```rust
OmniViewOutput {
    js: gen.js,
    html: gen.html,
    css: gen.css,
    source_map: gen.source_map,
    components: HashMap::new(),  // populated in Task 3
}
```

- [ ] **Step 3: Add editor_mode field to OmniViewGenerator struct**

After `mount_js: String,` (line 53 in struct, around line 153 in file):
```rust
    mount_js: String,
    editor_mode: bool,
```

In `generate()` initialization (line 31-54), add default:
```rust
    mount_js: String::new(),
    editor_mode: false,
```

- [ ] **Step 4: Accept editor_mode in generate() signature**

Change line 30:
```rust
pub fn generate(ir: &IRProgram, editor_mode: bool) -> OmniViewOutput {
```

And set it in the struct init:
```rust
    mount_js: String::new(),
    editor_mode,
```

- [ ] **Step 5: Update compile_omniview() in lib.rs to accept and pass editor_mode**

Change `compile_omniview` signature at line 121:
```rust
pub fn compile_omniview(source: &str, editor_mode: bool) -> OmniViewResult {
```

And update the call at line 159:
```rust
    let output = codegen::omniview::generate(&ir_program, editor_mode);
```

- [ ] **Step 6: Add compile_to_omniview WASM export to lib.rs**

After the existing `compile_to_omniview` WASM function, add:
```rust
/// Compile .omni source to OmniView (JS + HTML + CSS). Returns JSON string.
/// options_json: {"editor_mode": true}
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, options_json: &str) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = compile_omniview(source, opts.editor_mode);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":null,"source_map":null,"errors":[{{"message":"{}"}}]}}"#,
            e
        )
    })
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct CompileOptions {
    #[serde(default)]
    editor_mode: bool,
}
```

- [ ] **Step 7: Run tests and rebuild WASM**

```bash
cd omni-compiler && cargo test 2>&1
```
Expected: all tests pass (generate() calls need `false` arg if used in tests — check codegen_tests.rs).

Fix test calls if needed: `generate(&ir)` → `generate(&ir, false)`.

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm 2>&1
```
Expected: builds successfully.

- [ ] **Step 8: Commit**

```bash
git add omni-compiler/src/codegen/omniview.rs omni-compiler/src/lib.rs editor/public/wasm/
git commit -m "feat(wasm): add compile_to_omniview WASM export with editor_mode option"
```

---

### Task 3: Add component_js capture + compile_omniview_component export

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs` (capture per-component JS)
- Modify: `omni-compiler/src/lib.rs` (add compile_omniview_component export)

**Interfaces:**
- Consumes: `OmniViewGenerator` with `editor_mode` and `component_js` fields from Task 2
- Produces: `compile_omniview_component(source, component_name, options_json) -> String`
- Produces: `OmniViewGenerator.component_js: HashMap<String, String>` populated per component

- [ ] **Step 1: Add component_js and current_component_js fields to OmniViewGenerator**

In the struct (after `editor_mode`):
```rust
    editor_mode: bool,
    component_js: HashMap<String, String>,
    current_component_js: String,
```

In `generate()` initialization:
```rust
    editor_mode,
    component_js: HashMap::new(),
    current_component_js: String::new(),
```

- [ ] **Step 2: Capture per-component JS in generate_component() using delta tracking**

The `generate_component()` method (around line 373) currently writes directly to `self.js`. Track the delta to capture component-specific JS without disrupting existing flow.

At the start of `generate_component()`, record current JS length:
```rust
fn generate_component(&mut self, name: &str, comp: &ComponentIR, ir: &IRProgram, is_root: bool) {
    let js_before = self.js.len();
    // ... existing component generation code (unchanged) ...
```

At the end of `generate_component()`, extract the delta and store it:
```rust
    // After all component code is emitted, capture the delta
    let component_js = self.js[js_before..].to_string();
    self.component_js.insert(name.to_string(), component_js);
}
```

This approach is minimal — it doesn't change how `generate_component()` writes to `self.js`, just records what was written.

- [ ] **Step 3: Populate components in OmniViewOutput from component_js map**

In `generate()`, after all components are generated, build the components map. Since we only have `js` per component (not html/css split), for now set html and css as empty:

After the for loop at line 97-99:
```rust
// Build per-component output map
let mut components: HashMap<String, ComponentOutput> = HashMap::new();
for (name, js) in &gen.component_js {
    components.insert(name.clone(), ComponentOutput {
        js: js.clone(),
        html: String::new(),
        css: String::new(),
    });
}
```

Then in the return:
```rust
OmniViewOutput {
    js: gen.js,
    html: gen.html,
    css: gen.css,
    source_map: gen.source_map,
    components,
}
```

- [ ] **Step 4: Add compile_omniview_component WASM export to lib.rs**

```rust
/// Compile a single .omni component to OmniView output. Returns JSON string.
/// The source should contain a single component definition.
/// options_json: {"editor_mode": true}
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compile_omniview_component(source: &str, component_name: &str, options_json: &str) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = compile_omniview(source, opts.editor_mode);
    
    #[derive(Serialize)]
    struct ComponentResult {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        component: Option<ComponentOutput>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        errors: Vec<crate::error::Diagnostic>,
    }
    
    let component = result.components
        .as_ref()
        .and_then(|comps| comps.get(component_name).cloned());
    
    let comp_result = ComponentResult {
        success: result.success && component.is_some(),
        component,
        errors: result.errors,
    };
    
    serde_json::to_string(&comp_result).unwrap_or_else(|e| {
        format!(r#"{{"success":false,"component":null,"errors":[{{"message":"{}"}}]}}"#, e)
    })
}
```

The TS side receives `{ success, component: { js, html, css }, errors }`.

- [ ] **Step 5: Run tests and rebuild WASM**

```bash
cd omni-compiler && cargo test 2>&1
```
Expected: all tests pass.

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm 2>&1
```
Expected: builds.

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/codegen/omniview.rs omni-compiler/src/lib.rs editor/public/wasm/
git commit -m "feat(wasm): add per-component JS capture and compile_omniview_component export"
```

---

### Task 4: Update rustCompiler.ts for new WASM API

**Files:**
- Modify: `editor/client/src/api/rustCompiler.ts`

**Interfaces:**
- Consumes: WASM exports `compile_to_omniview(source, options_json)` and `compile_omniview_component(source, name, options_json)` from Task 3
- Produces: `rustCompiler.compileToOmniView(source, editorMode)` returns `OmniViewResult`
- Produces: `rustCompiler.compileOmniViewComponent(source, componentName, editorMode)` returns `OmniViewResult`

- [ ] **Step 1: Read current rustCompiler.ts**

```bash
cat editor/client/src/api/rustCompiler.ts
```

- [ ] **Step 2: Update WASM interface and methods**

Add to the private `wasm` interface:
```typescript
private wasm: {
    compile_to_ir: (source: string) => string;
    compile_to_omniview: (source: string, options_json: string) => string;
    compile_omniview_component?: (source: string, component_name: string, options_json: string) => string;
    version: () => string;
} | null = null;
```

Update `compileToOmniView` method:
```typescript
compileToOmniView(source: string, editorMode: boolean = false): OmniViewResult {
    if (!this.wasm || !this.wasm.compile_to_omniview) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: 'OmniView codegen not available', line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      const options = JSON.stringify({ editor_mode: editorMode });
      const json = this.wasm.compile_to_omniview(source, options);
      return JSON.parse(json);
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `OmniView compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
}
```

Add new `compileOmniViewComponent` method (returns single-component shape):
```typescript
type ComponentCompileResult = {
  success: boolean;
  component?: { js: string; html: string; css: string };
  errors?: Diagnostic[];
};

compileOmniViewComponent(source: string, componentName: string, editorMode: boolean = false): ComponentCompileResult {
    if (!this.wasm || !this.wasm.compile_omniview_component) {
      // Fallback: use full compile and extract on TS side
      const full = this.compileToOmniView(source, editorMode);
      if (!full.success || !full.files) {
        return { success: false, errors: full.errors };
      }
      return {
        success: true,
        component: {
          js: full.files.js,
          html: full.files.html,
          css: full.files.css,
        },
      };
    }
    try {
      const options = JSON.stringify({ editor_mode: editorMode });
      const json = this.wasm.compile_omniview_component(source, componentName, options);
      return JSON.parse(json) as ComponentCompileResult;
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `Component compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
}
```

- [ ] **Step 3: Run TypeScript check**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -20
```
Expected: no new type errors (there may be pre-existing ones unrelated to our change).

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/api/rustCompiler.ts
git commit -m "feat(editor): update rustCompiler.ts for new WASM compile_to_omniview API"
```

---

### Task 5: Generate editor bridge code in omniview.rs

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs` (add emit_editor_bridge method)

**Interfaces:**
- Consumes: `OmniViewGenerator.editor_mode` from Task 2
- Produces: `emit_editor_bridge()` method — emits postMessage bridge + factory function wrapper

- [ ] **Step 1: Write test for editor bridge output**

Create `omni-compiler/tests/editor_bridge_tests.rs`:
```rust
use omni_compiler::compile_omniview;

#[test]
fn editor_mode_false_produces_no_bridge_code() {
    let source = r#"
component App {
    view {
        <div id="App.root.0">Hello</div>
    }
}
"#;
    let normal = compile_omniview(source, false);
    let editor = compile_omniview(source, true);
    assert!(normal.success && editor.success);
    let normal_js = &normal.files.unwrap().js;
    let editor_js = &editor.files.unwrap().js;
    // Normal mode has no bridge code
    assert!(!normal_js.contains("window.__omni_signals"));
    // Editor mode has bridge code
    assert!(editor_js.contains("window.__omni_signals"));
}

#[test]
fn editor_mode_true_injects_bridge_code() {
    let source = r#"
component App {
    view {
        <div id="App.root.0">Hello</div>
    }
}
"#;
    let result = compile_omniview(source, true);
    assert!(result.success);
    let js = &result.files.unwrap().js;
    assert!(js.contains("window.__omni_signals"));
    assert!(js.contains("window.__omni_components"));
    assert!(js.contains("__omni_buildNidMap"));
    assert!(js.contains("postMessage"));
    assert!(js.contains("omni-select"));
    assert!(js.contains("omni-rendered"));
    assert!(js.contains("omni-replace-component"));
}

#[test]
fn editor_mode_true_uses_factory_function_pattern() {
    let source = r#"
component App {
    signal count: number = 0;
    view { <div id="App.root.0"></div> }
}
"#;
    let result = compile_omniview(source, true);
    assert!(result.success);
    let js = &result.files.unwrap().js;
    // Factory wraps component in IIFE
    assert!(js.contains("window.__omni_components[\"App\"] = (function()"));
    assert!(js.contains("teardown"));
}
```

- [ ] **Step 2: Add emit_editor_bridge() method to omniview.rs**

After `emit_dom_and_bindings()` and before `finalize_js()`, add:

```rust
fn emit_editor_bridge(&mut self) {
    if !self.editor_mode {
        return;
    }
    
    // Signal registry
    self.js.push_str(r#"
// === Editor Bridge ===
window.__omni_signals = {};
window.__omni_components = {};

function __omni_buildNidMap() {
  var map = {};
  document.querySelectorAll('[data-nid]').forEach(function(el) {
    map[el.dataset.nid] = {
      elementId: el.id || '',
      tag: el.tagName.toLowerCase(),
      component: (el.closest('[data-component]') || {}).dataset?.component || '',
    };
  });
  return map;
}

document.addEventListener('click', function(e) {
  var el = e.target.closest('[data-nid]');
  if (el) {
    e.stopPropagation();
    parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, '*');
  }
});

window.addEventListener('message', function(e) {
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
  // factoryCode is the IIFE text: "(function() { ... return {signals, render, teardown}; })()"
  var oldComp = window.__omni_components[name];
  var savedSignals = {};
  if (oldComp && oldComp.signals) {
    Object.keys(oldComp.signals).forEach(function(k) {
      savedSignals[k] = oldComp.signals[k].peek();
    });
    if (oldComp.teardown) oldComp.teardown();
  }
  // Execute factory IIFE and assign result
  window.__omni_components[name] = (0, eval)(factoryCode);
  var newComp = window.__omni_components[name];
  if (newComp && newComp.signals) {
    Object.keys(savedSignals).forEach(function(k) {
      if (newComp.signals[k]) newComp.signals[k].set(savedSignals[k]);
    });
  }
  parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, '*');
}

parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, '*');
"#);
}
```

- [ ] **Step 3: Call emit_editor_bridge() in generate()**

In `generate()`, after `gen.emit_dom_and_bindings()` and before `gen.finalize_js()`:
```rust
    gen.emit_dom_and_bindings();
    gen.emit_editor_bridge();   // NEW
    gen.finalize_js();
```

- [ ] **Step 4: Run tests**

```bash
cd omni-compiler && cargo test editor_bridge 2>&1
cd omni-compiler && cargo test 2>&1 | tail -5
```
Expected: editor_bridge tests pass; all 463+ tests pass.

- [ ] **Step 5: Rebuild WASM**

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm 2>&1
```

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/codegen/omniview.rs omni-compiler/tests/editor_bridge_tests.rs editor/public/wasm/
git commit -m "feat(omniview): add editor bridge injection (postMessage, factory, nid_map)"
```

---

### Task 6: Emit components as factory functions in editor mode

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs` (modify generate_component wrapping)

**Interfaces:**
- Consumes: `editor_mode` field from Task 2
- Produces: Each component emitted as `window.__omni_components["Name"] = (function() { ... })();` when editor_mode=true

- [ ] **Step 1: Write test for factory function output**

Add to `editor_bridge_tests.rs`:
```rust
#[test]
fn editor_mode_emits_factory_not_bare_const() {
    let source = r#"
component Counter {
    signal count: number = 0;
    view { <button id="Counter.btn.0" @click={inc}>Count</button> }
    fn inc() { count = count + 1; }
}
"#;
    let normal = compile_omniview(source, false);
    let editor = compile_omniview(source, true);
    
    let normal_js = &normal.files.unwrap().js;
    let editor_js = &editor.files.unwrap().js;
    
    // Normal mode uses bare const
    assert!(normal_js.contains("const Counter_count"));
    // Editor mode wraps in factory
    assert!(editor_js.contains("window.__omni_components[\"Counter\"] = (function()"));
}
```

- [ ] **Step 2: Modify generate_component() to conditionally wrap in factory**

In `generate_component()`, at the beginning (before signal declarations), when `editor_mode` is true:
```rust
if self.editor_mode {
    self.js.push_str(&format!(
        "window.__omni_components[\"{}\"] = (function() {{\n",
        name
    ));
}
```

At the end of `generate_component()`, when `editor_mode` is true, emit the factory return value:
```rust
if self.editor_mode {
    // Collect all signal names for this component for the factory return
    let signal_registrations = comp.states.iter()
        .map(|s| format!("{}", prefixed_name))
        .collect::<Vec<_>>()
        .join(", ");
    
    self.js.push_str(&format!(r#"
  return {{
    signals: {{ {} }},
    teardown: function() {{
      var root = document.querySelector('[data-component="{}"]');
      if (root) root.remove();
    }}
  }};
}})();
"#, signal_registrations, name));
}
```

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test editor_bridge 2>&1
cd omni-compiler && cargo test 2>&1 | tail -3
```
Expected: all pass.

- [ ] **Step 4: Rebuild WASM and commit**

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm 2>&1
git add omni-compiler/ editor/public/wasm/
git commit -m "feat(omniview): emit components as factory functions in editor mode"
```

---

### Task 7: Create CanvasPreview component

**Files:**
- Create: `editor/client/src/canvas/CanvasPreview.tsx`

**Interfaces:**
- Consumes: `OmniViewResult` from `rustCompiler.compileToOmniView()` (Task 4)
- Produces: `<CanvasPreview>` React component with iframe + postMessage bridge
- Produces: `onSelect(nid: string): void` callback
- Produces: `onRendered(nidMap: Record<string, NidEntry>): void` callback

- [ ] **Step 1: Create CanvasPreview.tsx**

```typescript
import React, { useRef, useEffect, useCallback } from 'react';

interface NidEntry {
  elementId: string;
  tag: string;
  component: string;
}

interface CanvasPreviewProps {
  srcdoc: string;
  onSelect?: (nid: string, entry: NidEntry) => void;
  onRendered?: (nidMap: Record<string, NidEntry>) => void;
  onHover?: (nid: string) => void;
  style?: React.CSSProperties;
  className?: string;
}

export function CanvasPreview({
  srcdoc,
  onSelect,
  onRendered,
  onHover,
  style,
  className,
}: CanvasPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const nidMapRef = useRef<Record<string, NidEntry>>({});

  useEffect(() => {
    const iframe = iframeRef.current;
    if (!iframe || !srcdoc) return;

    iframe.srcdoc = srcdoc;

    const handler = (e: MessageEvent) => {
      if (e.source !== iframe.contentWindow) return;

      switch (e.data?.type) {
        case 'omni-select': {
          const nid = e.data.nid as string;
          const entry = nidMapRef.current[nid];
          onSelect?.(nid, entry || { elementId: '', tag: '', component: '' });
          // Also highlight in iframe
          iframe.contentWindow?.postMessage({ type: 'omni-highlight', nid }, '*');
          break;
        }
        case 'omni-hover': {
          onHover?.(e.data.nid as string);
          break;
        }
        case 'omni-rendered': {
          nidMapRef.current = (e.data.nidMap || {}) as Record<string, NidEntry>;
          onRendered?.(nidMapRef.current);
          break;
        }
        case 'omni-signal-changed':
        case 'omni-event-fired':
          // Debug messages — log to console for now
          console.debug('[CanvasPreview]', e.data.type, e.data);
          break;
      }
    };

    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [srcdoc, onSelect, onRendered, onHover]);

  const postMessage = useCallback((msg: unknown) => {
    iframeRef.current?.contentWindow?.postMessage(msg, '*');
  }, []);

  return (
    <iframe
      ref={iframeRef}
      style={{
        width: '100%',
        height: '100%',
        border: 'none',
        backgroundColor: '#fff',
        ...style,
      }}
      className={className}
      sandbox="allow-scripts allow-same-origin"
      title="OmniView Preview"
    />
  );
}

// Expose postMessage capability for parent to call
export type CanvasPreviewHandle = {
  postMessage: (msg: unknown) => void;
  highlight: (nid: string) => void;
  clearHighlight: () => void;
  replaceComponent: (name: string, factoryCode: string) => void;
};
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -20
```
Expected: no errors in CanvasPreview.tsx.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/canvas/CanvasPreview.tsx
git commit -m "feat(editor): create CanvasPreview iframe component with postMessage bridge"
```

---

### Task 8: Create OmniCompilerCache

**Files:**
- Create: `editor/client/src/omni-compiler-cache.ts`

**Interfaces:**
- Consumes: `rustCompiler` from Task 4
- Produces: `OmniCompilerCache` class with compile/recompile/invalidate methods

- [ ] **Step 1: Create omni-compiler-cache.ts**

```typescript
import { rustCompiler, type OmniViewResult } from './api/rustCompiler';
import { generateOmniSource } from './omni-gen/pipeline';
// NOTE: import { generateOmniFragment } from './omni-gen/pipeline';  // Added in Task 9
import type { ElementData } from './store/editor';

interface ComponentCacheEntry {
  omniFragment: string;
  compiledJS: string;
  compiledHTML: string;
  compiledCSS: string;
  signalNames: string[];
  lastCompiled: number;
}

interface CompiledOutput {
  srcdoc: string;
  components: Map<string, { js: string; html: string; css: string }>;
}

class OmniCompilerCache {
  private components = new Map<string, ComponentCacheEntry>();
  private runtimeJS = '';
  private lastFullCompile = 0;

  compileProject(elements: ElementData[], editorMode: boolean = true): CompiledOutput | null {
    const result = generateOmniSource(elements);
    if (result.errors?.length) {
      console.error('[OmniCompilerCache] omni-gen errors:', result.errors);
      return null;
    }

    const omniSource = result.value || '';
    const wasmResult = rustCompiler.compileToOmniView(omniSource, editorMode);
    
    if (!wasmResult.success || !wasmResult.files) {
      console.error('[OmniCompilerCache] WASM errors:', wasmResult.errors);
      return null;
    }

    // Cache per-component output
    const components = new Map<string, { js: string; html: string; css: string }>();
    if (wasmResult.components) {
      for (const [name, comp] of Object.entries(wasmResult.components)) {
        components.set(name, comp);
      }
    }

    // Build srcdoc
    const srcdoc = buildSrcdoc(wasmResult.files.js, wasmResult.files.html, wasmResult.files.css);
    this.lastFullCompile = Date.now();

    return { srcdoc, components };
  }

  recompileComponent(
    name: string,
    elements: ElementData[],
    editorMode: boolean = true
  ): { js: string } | null {
    // Use generateOmniSource for now; Task 9 adds generateOmniFragment for incremental
    const result = generateOmniSource(elements);
    if (result.errors?.length) {
      console.error('[OmniCompilerCache] omni-gen errors:', result.errors);
      return null;
    }

    const fragment = result.value || '';
    const cached = this.components.get(name);
    
    // Skip if fragment unchanged
    if (cached && cached.omniFragment === fragment) {
      return { js: cached.compiledJS };
    }

    const wasmResult = rustCompiler.compileOmniViewComponent(fragment, name, editorMode);
    if (!wasmResult.success || !wasmResult.component) {
      console.error('[OmniCompilerCache] Component compile errors:', wasmResult.errors);
      return null;
    }

    const componentJS = wasmResult.component.js;
    this.components.set(name, {
      omniFragment: fragment,
      compiledJS: componentJS,
      compiledHTML: wasmResult.component.html || '',
      compiledCSS: wasmResult.component.css || '',
      signalNames: extractSignalNames(componentJS),
      lastCompiled: Date.now(),
    });

    return { js: componentJS };
  }

  invalidate(name: string): void {
    this.components.delete(name);
  }

  invalidateAll(): void {
    this.components.clear();
    this.runtimeJS = '';
    this.lastFullCompile = 0;
  }
}

function buildSrcdoc(js: string, html: string, css: string): string {
  return `<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"><style>${css}</style></head>
<body>
<div id="root">${html}</div>
<script>${js}<\/script>
</body>
</html>`;
}

function extractSignalNames(js: string): string[] {
  const names: string[] = [];
  const re = /__omni_signal\("([^"]+)"/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(js)) !== null) {
    names.push(m[1]);
  }
  return names;
}

export const compilerCache = new OmniCompilerCache();
export type { CompiledOutput };
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/omni-compiler-cache.ts
git commit -m "feat(editor): add OmniCompilerCache with incremental compilation"
```

---

### Task 9: Add generateOmniFragment to pipeline

**Files:**
- Modify: `editor/client/src/omni-gen/pipeline.ts`

**Interfaces:**
- Consumes: `generateOmniSource(elements)` existing function
- Produces: `generateOmniFragment(componentName, elements) -> StageResult<string>` — generates .omni for a single component

- [ ] **Step 1: Read current pipeline.ts to understand generateOmniSource signature**

```bash
grep -n "export function generateOmniSource\|export function generateOmniFiles" editor/client/src/omni-gen/pipeline.ts
```

- [ ] **Step 2: Add generateOmniFragment function**

```typescript
/**
 * Generate .omni source fragment for a single component.
 * Used for incremental compilation — only re-generates the changed component's source.
 */
export function generateOmniFragment(
  componentName: string,
  elements: ElementData[]
): StageResult<string> {
  // Filter elements belonging to this component
  // For the root component, include all elements without a componentRef
  // For custom components, include elements whose componentRef matches
  const componentElements = elements.filter((el) => {
    if (componentName === 'App' || componentName === 'main') {
      return !el.componentRef || el.componentRef === componentName;
    }
    return el.componentRef === componentName;
  });

  // Run the full pipeline but only for these elements
  const treeResult = buildViewTree(componentElements);
  if (!treeResult.value) {
    return { value: null, errors: treeResult.errors };
  }

  // Generate source for just this component
  const emitInput = buildEmitInput(componentElements, treeResult.value, componentName);
  const source = emitOmniSource(emitInput);

  return { value: source, errors: [] };
}
```

**Note:** This is a simplified approach. In practice, `generateOmniFragment` may need to call a sub-pipeline that only handles one component. For Phase 1, the simplest approach is to call `generateOmniSource` on filtered elements.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/omni-gen/pipeline.ts
git commit -m "feat(omni-gen): add generateOmniFragment for incremental component generation"
```

---

### Task 10: Add right-click context menu to ComponentTree

**Files:**
- Modify: `editor/client/src/panels/ComponentTree.tsx`

**Interfaces:**
- Produces: Right-click context menu with: Add child, Copy, Paste, Move Up, Move Down, Delete, Edit Component

- [ ] **Step 1: Read current ComponentTree.tsx**

Key things to understand: current tree rendering, selection handling, drag implementation.

- [ ] **Step 2: Add context menu state and handlers**

Add state at the top of ComponentTree component:
```typescript
const [contextMenu, setContextMenu] = useState<{
  x: number;
  y: number;
  elementId: string;
} | null>(null);
```

Add right-click handler on tree nodes:
```typescript
const handleContextMenu = useCallback((e: React.MouseEvent, elementId: string) => {
  e.preventDefault();
  e.stopPropagation();
  setContextMenu({ x: e.clientX, y: e.clientY, elementId });
}, []);
```

- [ ] **Step 3: Add context menu JSX**

After the tree rendering, add:
```tsx
{contextMenu && (
  <div
    className="omni-context-menu"
    style={{
      position: 'fixed',
      left: contextMenu.x,
      top: contextMenu.y,
      zIndex: 1000,
    }}
    onMouseLeave={() => setContextMenu(null)}
  >
    <ContextMenuBody
      elementId={contextMenu.elementId}
      onClose={() => setContextMenu(null)}
    />
  </div>
)}
```

- [ ] **Step 4: Create ContextMenuBody component**

In the same file or a new `ComponentTreeContextMenu.tsx`:
```tsx
function ContextMenuBody({ elementId, onClose }: { elementId: string; onClose: () => void }) {
  const store = useEditorStore();
  const execute = useUndoStore((s) => s.execute);

  const items: ContextMenuItem[] = [
    {
      label: 'Add Child ▸',
      submenu: [
        { label: 'Layout ▸', submenu: layoutComponents() },
        { label: 'Typography ▸', submenu: typographyComponents() },
        { label: 'Actions ▸', submenu: actionComponents() },
        { label: 'Forms ▸', submenu: formComponents() },
      ],
    },
    { label: 'Copy', shortcut: 'Ctrl+C', action: () => handleCopy(elementId) },
    { label: 'Paste', shortcut: 'Ctrl+V', action: () => handlePaste(elementId) },
    { type: 'separator' },
    { label: 'Move Up', shortcut: 'Ctrl+↑', action: () => handleMoveUp(elementId) },
    { label: 'Move Down', shortcut: 'Ctrl+↓', action: () => handleMoveDown(elementId) },
    { type: 'separator' },
    { label: 'Delete', shortcut: 'Delete', action: () => handleDelete(elementId) },
  ];

  // Add "Edit Component" for custom components
  const element = store.elements.find((el) => el.id === elementId);
  if (element?.componentRef) {
    items.push(
      { type: 'separator' },
      { label: 'Edit Component', action: () => handleEditComponent(element.componentRef!) }
    );
  }

  return (
    <div className="context-menu-list">
      {items.map((item, i) => (
        <ContextMenuRow key={i} item={item} onClose={onClose} />
      ))}
    </div>
  );
}
```

- [ ] **Step 5: Implement action handlers**

```typescript
function handleDelete(elementId: string) {
  const cmd = new RemoveElement(elementId);
  execute(cmd);
  onClose();
  // Trigger recompile after state update
  triggerCompile();
}

function handleMoveUp(elementId: string) {
  const elements = useEditorStore.getState().elements;
  const el = elements.find((e) => e.id === elementId);
  if (!el || !el.parentId) return;
  const parent = elements.find((e) => e.id === el.parentId);
  if (!parent) return;
  const idx = parent.children.indexOf(elementId);
  if (idx <= 0) return;
  const cmd = new ReorderElement(elementId, el.parentId, idx - 1);
  execute(cmd);
  onClose();
  triggerCompile();
}

function handleMoveDown(elementId: string) {
  const elements = useEditorStore.getState().elements;
  const el = elements.find((e) => e.id === elementId);
  if (!el || !el.parentId) return;
  const parent = elements.find((e) => e.id === el.parentId);
  if (!parent) return;
  const idx = parent.children.indexOf(elementId);
  if (idx < 0 || idx >= parent.children.length - 1) return;
  const cmd = new ReorderElement(elementId, el.parentId, idx + 1);
  execute(cmd);
  onClose();
  triggerCompile();
}
```

- [ ] **Step 6: Add CSS styles**

In `editor/client/src/panels/ComponentTree.css` or equivalent:
```css
.omni-context-menu {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.12);
  min-width: 200px;
  padding: 4px 0;
}

.context-menu-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
  color: #1e293b;
}

.context-menu-item:hover {
  background: #f1f5f9;
}

.context-menu-separator {
  height: 1px;
  background: #e2e8f0;
  margin: 4px 0;
}

.context-menu-shortcut {
  color: #94a3b8;
  font-size: 11px;
  margin-left: 24px;
}
```

- [ ] **Step 7: Commit**

```bash
git add editor/client/src/panels/ComponentTree.tsx
git commit -m "feat(editor): add right-click context menu to component tree"
```

---

### Task 11: Add keyboard shortcuts to ComponentTree

**Files:**
- Modify: `editor/client/src/panels/ComponentTree.tsx`

- [ ] **Step 1: Add keyboard event listener**

In the ComponentTree component, add a `useEffect` for keyboard handling:
```typescript
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    const selected = useEditorStore.getState().selectedIds;
    if (!selected || selected.size === 0) return;
    const elementId = [...selected][0];

    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      const cmd = new RemoveElement(elementId);
      useUndoStore.getState().execute(cmd);
      triggerCompile();
      return;
    }

    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case 'c':
          e.preventDefault();
          handleCopy(elementId);
          break;
        case 'v':
          e.preventDefault();
          handlePaste(elementId);
          break;
        case 'ArrowUp':
          e.preventDefault();
          handleMoveUp(elementId);
          break;
        case 'ArrowDown':
          e.preventDefault();
          handleMoveDown(elementId);
          break;
      }
    }
  };

  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}, []);
```

- [ ] **Step 2: Implement copy/paste with clipboard**

```typescript
let clipboard: { element: ElementData; children: ElementData[] } | null = null;

function handleCopy(elementId: string) {
  const elements = useEditorStore.getState().elements;
  const el = elements.find((e) => e.id === elementId);
  if (!el) return;
  
  // Deep copy the element and all descendants
  const children = collectDescendants(elementId, elements);
  clipboard = {
    element: { ...el },
    children: children.map((c) => ({ ...c })),
  };
}

function handlePaste(targetId: string) {
  if (!clipboard) return;
  // Create new IDs for pasted elements
  const idMap = new Map<string, string>();
  idMap.set(clipboard.element.id, uid());
  
  const newElement = { ...clipboard.element, id: idMap.get(clipboard.element.id)!, parentId: targetId };
  const cmd = new AddElement(newElement, targetId);
  useUndoStore.getState().execute(cmd);
  
  // Also add children with remapped IDs
  for (const child of clipboard.children) {
    const newId = uid();
    idMap.set(child.id, newId);
    const newChild = {
      ...child,
      id: newId,
      parentId: idMap.get(child.parentId || '') || targetId,
    };
    const childCmd = new AddElement(newChild, newChild.parentId);
    useUndoStore.getState().execute(childCmd);
  }
  
  triggerCompile();
}
```

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/panels/ComponentTree.tsx
git commit -m "feat(editor): add keyboard shortcuts to component tree (Delete, Ctrl+C/V, Ctrl+↑↓)"
```

---

### Task 12: Enhance tree drag with visual indicators

**Files:**
- Modify: `editor/client/src/panels/ComponentTree.tsx`

- [ ] **Step 1: Add drop indicator state**

```typescript
const [dropIndicator, setDropIndicator] = useState<{
  targetId: string;
  position: 'before' | 'after' | 'inside';
} | null>(null);
```

- [ ] **Step 2: Update drag handlers to show indicators**

In the existing `onDragOver` handler, add mouse position calculation:
```typescript
const handleDragOver = (e: React.DragEvent, elementId: string) => {
  e.preventDefault();
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const y = e.clientY - rect.top;
  const h = rect.height;

  if (y < h * 0.25) {
    setDropIndicator({ targetId: elementId, position: 'before' });
  } else if (y > h * 0.75) {
    setDropIndicator({ targetId: elementId, position: 'after' });
  } else {
    setDropIndicator({ targetId: elementId, position: 'inside' });
  }
};
```

- [ ] **Step 3: Render drop indicator**

```tsx
{dropIndicator && (
  <div
    className={`drop-indicator drop-${dropIndicator.position}`}
    style={{
      /* position based on dropIndicator.targetId */
    }}
  />
)}
```

CSS:
```css
.drop-indicator.drop-before {
  border-top: 2px solid #4F46E5;
}
.drop-indicator.drop-after {
  border-bottom: 2px solid #4F46E5;
}
.drop-indicator.drop-inside {
  outline: 2px solid #4F46E5;
  outline-offset: -2px;
}
```

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/panels/ComponentTree.tsx
git commit -m "feat(editor): add visual drop indicators to component tree drag"
```

---

### Task 13: Replace LayoutView mock-DOM with CanvasPreview iframe

**Files:**
- Modify: `editor/client/src/canvas/LayoutView.tsx` (replace render section, remove DnD)

- [ ] **Step 1: Read current LayoutView.tsx to understand the render path**

Identify the `RenderElement` component and the DnD wrapper.

- [ ] **Step 2: Replace mock-DOM render with CanvasPreview**

In `LayoutView`, replace the `RenderElement(...)` call tree with:
```tsx
import { CanvasPreview } from './CanvasPreview';
import { compilerCache } from '../omni-compiler-cache';

// In the component:
const [srcdoc, setSrcdoc] = useState<string>('');
const [nidMap, setNidMap] = useState<Record<string, NidEntry>>({});

// Compile on initial load
useEffect(() => {
  const elements = useEditorStore.getState().elements;
  const output = compilerCache.compileProject(elements, true);
  if (output) {
    setSrcdoc(output.srcdoc);
  }
}, []);

// Recompile on element changes
useEffect(() => {
  const unsub = useEditorStore.subscribe((state) => {
    // Debounce: recompile after 300ms of inactivity
    debouncedCompile(state.elements);
  });
  return unsub;
}, []);

const handleSelect = useCallback((nid: string, entry: NidEntry) => {
  // Select element in tree + property panel
  if (entry.elementId) {
    useEditorStore.getState().setSelectedIds(new Set([entry.elementId]));
  }
  // Clear previous highlight and highlight new
  iframeRef.current?.postMessage({ type: 'omni-highlight', nid }, '*');
}, []);

const handleRendered = useCallback((map: Record<string, NidEntry>) => {
  setNidMap(map);
}, []);

// In JSX:
<CanvasPreview
  srcdoc={srcdoc}
  onSelect={handleSelect}
  onRendered={handleRendered}
  style={{ flex: 1, minHeight: 0 }}
/>
```

- [ ] **Step 3: Add drag overlay for palette drops**

Above the iframe, add a transparent overlay that appears during drag:
```tsx
const [isDragOver, setIsDragOver] = useState(false);

// In JSX, between the iframe and the outer div:
{isDragOver && (
  <div
    className="canvas-drag-overlay"
    onDragOver={(e) => e.preventDefault()}
    onDrop={(e) => {
      setIsDragOver(false);
      const tag = e.dataTransfer.getData('application/omni-component');
      if (tag) {
        const newEl = createDefaultElement(tag);
        const cmd = new AddElement(newEl, null);
        useUndoStore.getState().execute(cmd);
        triggerCompile();
      }
    }}
    onDragLeave={() => setIsDragOver(false)}
    style={{
      position: 'absolute',
      inset: 0,
      zIndex: 10,
      pointerEvents: 'auto',
    }}
  />
)}
```

- [ ] **Step 4: Remove dnd-kit DragHandle/MoveHandleWrapper from LayoutView**

Remove:
- `DndContext` and `SortableContext` imports and usage
- `DragHandle` component (`⋮⋮` icon)
- `MoveHandleWrapper` component (`⇄` icon)
- `dragOpMap` and associated collision detection logic

Keep only:
- The iframe wrapper rendering
- The drag overlay
- Keyboard undo/redo shortcuts

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/canvas/LayoutView.tsx
git commit -m "feat(editor): replace mock-DOM canvas with CanvasPreview iframe"
```

---

### Task 14: Connect property panel to compile cycle

**Files:**
- Modify: `editor/client/src/panels/PropertyPanel.tsx`

- [ ] **Step 1: Add compile trigger on prop change**

In the `SetProp` command execution path, add:
```typescript
// After SetProp command executes:
const compileAndUpdate = () => {
  const elements = useEditorStore.getState().elements;
  const selectedIds = useEditorStore.getState().selectedIds;
  
  // Determine which component changed
  const changedElement = elements.find((el) => selectedIds.has(el.id));
  if (!changedElement) return;
  
  const componentName = findOwningComponent(changedElement, elements);
  
  // Recompile only this component
  const result = compilerCache.recompileComponent(componentName, elements, true);
  if (result) {
    // Send to iframe
    iframeRef.current?.contentWindow?.postMessage({
      type: 'omni-replace-component',
      name: componentName,
      js: result.js,
    }, '*');
  }
};
```

- [ ] **Step 2: Add findOwningComponent helper**

```typescript
function findOwningComponent(element: ElementData, allElements: ElementData[]): string {
  // Walk up parent chain to find the root component
  let current = element;
  while (current.parentId) {
    const parent = allElements.find((e) => e.id === current.parentId);
    if (!parent) break;
    current = parent;
  }
  // If it has a componentRef, that's the component
  if (current.componentRef) return current.componentRef;
  // Otherwise it's the root component
  return 'App';
}
```

- [ ] **Step 3: Debounce the compile for slider/input continuous edits**

```typescript
import { useDebouncedCallback } from 'use-debounce'; // or implement inline

const debouncedRecompile = useDebouncedCallback(
  (componentName: string, elements: ElementData[]) => {
    const result = compilerCache.recompileComponent(componentName, elements, true);
    if (result) {
      iframeRef.current?.contentWindow?.postMessage({
        type: 'omni-replace-component',
        name: componentName,
        js: result.js,
      }, '*');
    }
  },
  300
);
```

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/panels/PropertyPanel.tsx
git commit -m "feat(editor): connect property panel edits to compile cycle"
```

---

### Task 15: Wire full compile cycle in App.tsx

**Files:**
- Modify: `editor/client/src/App.tsx`

- [ ] **Step 1: Add recompile function**

```typescript
import { compilerCache } from './omni-compiler-cache';

const recompileAndUpdate = useCallback((elements: ElementData[]) => {
  const output = compilerCache.compileProject(elements, true);
  if (output) {
    setSrcdoc(output.srcdoc);
  } else {
    // Compile failed — keep previous srcdoc
    console.warn('[App] Compile failed, keeping previous output');
  }
}, []);
```

- [ ] **Step 2: Hook into state changes**

```typescript
// Subscribe to element changes
useEffect(() => {
  const unsub = useEditorStore.subscribe((state, prevState) => {
    if (state.elements !== prevState.elements || state.selectedIds !== prevState.selectedIds) {
      // For structural changes (elements array different), do full recompile
      // For prop changes (same elements, different props), recompile single component
      recompileAndUpdate(state.elements);
    }
  });
  return unsub;
}, [recompileAndUpdate]);
```

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/App.tsx
git commit -m "feat(editor): wire full compile cycle in App.tsx"
```

---

### Task 16: Clean up — remove obsolete canvas files

**Files:**
- Remove: `editor/client/src/canvas/CustomComponentRenderer.tsx`
- Remove: `editor/client/src/canvas/WireOverlay.tsx`
- Modify: Remove imports of above files

- [ ] **Step 1: Remove files and update imports**

```bash
git rm editor/client/src/canvas/CustomComponentRenderer.tsx
git rm editor/client/src/canvas/WireOverlay.tsx
```

Search for and remove imports:
```bash
grep -r "CustomComponentRenderer\|WireOverlay" editor/client/src/ --include="*.tsx" --include="*.ts"
```

Remove any remaining references.

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add editor/client/
git commit -m "refactor(editor): remove obsolete CustomComponentRenderer and WireOverlay"
```

---

### Task 17: Final verification — run all tests

- [ ] **Step 1: Run Rust compiler tests**

```bash
cd omni-compiler && cargo test 2>&1
```
Expected: all 463+ tests pass.

- [ ] **Step 2: Run omni-gen tests**

```bash
cd editor/client && npx vitest run --reporter=verbose 2>&1 | tail -30
```
Expected: all existing omni-gen tests pass.

- [ ] **Step 3: Verify editor TypeScript build**

```bash
cd editor/client && npx tsc --noEmit --pretty false 2>&1
```
Expected: no errors (or only pre-existing errors not caused by our changes).

- [ ] **Step 4: Rebuild WASM (final)**

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm 2>&1
```

- [ ] **Step 5: Verify WASM loads in browser**

Start the editor server and check browser console:
```
[RustCompiler] Loaded omni-compiler v0.1.0
```

- [ ] **Step 6: Save session checkpoint**

```bash
git add -A
git status
```

---

### Task 18: Polish — error handling, debounce, IR cache validation

**Files:**
- Modify: `editor/client/src/omni-compiler-cache.ts`
- Modify: `editor/client/src/canvas/CanvasPreview.tsx`
- Modify: `editor/client/src/App.tsx`

**Interfaces:**
- Consumes: `compilerCache`, `CanvasPreview` from Tasks 7-8
- Produces: Compile error recovery, debounced recompile, IR cache validation

- [ ] **Step 1: Add compile error recovery to OmniCompilerCache**

In `compileProject()` and `recompileComponent()`, add rollback:
```typescript
compileProject(elements: ElementData[], editorMode: boolean = true): CompiledOutput | null {
  const previousOutput = this.lastOutput;  // NEW: save previous
  
  const result = generateOmniSource(elements);
  if (result.errors?.length) {
    console.error('[OmniCompilerCache] omni-gen errors:', result.errors);
    return previousOutput;  // Return stale output instead of null
  }
  
  // ... rest of compile ...
  
  // Save successful output
  this.lastOutput = { srcdoc, components };
  return this.lastOutput;
}
```

- [ ] **Step 2: Add IR cache validation**

```typescript
private validateCache(): void {
  const now = Date.now();
  const MAX_AGE = 5 * 60 * 1000; // 5 minutes
  for (const [name, entry] of this.components) {
    if (now - entry.lastCompiled > MAX_AGE) {
      this.components.delete(name);
    }
  }
}
```

Call `validateCache()` at start of `compileProject()`.

- [ ] **Step 3: Add error boundary in CanvasPreview**

```tsx
// In CanvasPreview, handle srcdoc errors
const [loadError, setLoadError] = useState(false);

useEffect(() => {
  setLoadError(false);
  const iframe = iframeRef.current;
  if (!iframe || !srcdoc) return;
  
  try {
    iframe.srcdoc = srcdoc;
  } catch (e) {
    console.error('[CanvasPreview] Failed to load srcdoc:', e);
    setLoadError(true);
  }
}, [srcdoc]);
```

Show a fallback UI when `loadError` is true:
```tsx
{loadError && (
  <div style={{ padding: 20, color: '#ef4444', textAlign: 'center' }}>
    Compilation error — check the .omni source for syntax errors
  </div>
)}
```

- [ ] **Step 4: Add debounce to App.tsx recompile**

```typescript
const debouncedRecompile = useMemo(() => {
  let timer: ReturnType<typeof setTimeout>;
  return (elements: ElementData[]) => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const output = compilerCache.compileProject(elements, true);
      if (output) {
        setSrcdoc(output.srcdoc);
      }
    }, 300);
  };
}, []);
```

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/omni-compiler-cache.ts editor/client/src/canvas/CanvasPreview.tsx editor/client/src/App.tsx
git commit -m "feat(editor): add error recovery, debounce, and cache validation"
```

---

## Task Summary

| Task | Component | Files Changed | Key Deliverable |
|------|-----------|---------------|-----------------|
| 1 | WASM bindings merge | Cargo.toml, lib.rs | WASM compiles, 463 tests pass |
| 2 | compile_to_omniview export | lib.rs, omniview.rs | WASM export with editor_mode |
| 3 | component_js capture | omniview.rs, lib.rs | Per-component JS captured |
| 4 | rustCompiler.ts update | rustCompiler.ts | TS API matches new WASM |
| 5 | editor bridge injection | omniview.rs | postMessage + nid_map + __omni_signals |
| 6 | factory function pattern | omniview.rs | Components emit as replaceable factories |
| 7 | CanvasPreview component | CanvasPreview.tsx (new) | iframe + postMessage bridge |
| 8 | OmniCompilerCache | omni-compiler-cache.ts (new) | Incremental compilation cache |
| 9 | generateOmniFragment | pipeline.ts | Single-component omni-gen |
| 10 | Tree context menu | ComponentTree.tsx | Right-click menu |
| 11 | Tree keyboard shortcuts | ComponentTree.tsx | Delete, Ctrl+C/V, Ctrl+↑↓ |
| 12 | Tree drag indicators | ComponentTree.tsx | Visual drop feedback |
| 13 | Canvas → iframe | LayoutView.tsx, App.tsx | Mock-DOM replaced by iframe |
| 14 | Property panel → compile | PropertyPanel.tsx | Prop edits trigger recompile |
| 15 | Compile cycle wiring | App.tsx | End-to-end pipeline |
| 16 | Cleanup obsolete files | CustomComponentRenderer, WireOverlay | Removed |
| 17 | Final verification | All | All tests pass, WASM loads |
| 18 | Polish & error handling | Cache, CanvasPreview, App.tsx | Error recovery, debounce, cache validation |
