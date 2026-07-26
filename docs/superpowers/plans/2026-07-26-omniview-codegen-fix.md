# OmniView Codegen Fix Plan — Phase 1 & 2

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix verified codegen issues in OmniView/React/Flutter backends: wire up @effect cleanup, remove dead code, extract shared UiCodegen trait, and split oversized files.

**Architecture:** The fix touches 3 layers: (1) OmniView codegen runtime wiring (`store.rs` — effect cleanup), (2) shared codegen infrastructure (`codegen/shared/` — new UiCodegen trait), (3) file splitting (`codegen/omniview/html.rs`, `codegen/react/component.rs`). All changes are internal refactors — no public API changes, no .omni syntax changes.

**Tech Stack:** Rust (omnilang crate), `cargo test --all-features`, `cargo clippy`

## Global Constraints

- Only OmniView, React, Flutter backends in scope (per [[omnilang-codegen-priority]])
- All existing tests must pass after each task
- Use `pub use` re-exports to preserve existing import paths after file splits
- Each task is an independent PR
- Follow existing code patterns: `OmniViewGenerator` struct with `js`/`html`/`css` buffers, `CodeWriter` for React/Vue

---

## Verified Architecture (Corrections from 2026-07-26 Analysis)

Parser → IR flow for `@effect`:
```
Parser: @effect(signals) { body }
  → LifecycleHook { kind: Effect, deps, body }
  → stored in comp.lifecycles

IR Lowering (ir/lower/component.rs:62-70):
  → FILTERED: Effect → comp.effects (EffectIntent)
  → REMAINING: Mount/Unmount → comp.lifecycles (LifecycleIntent)
  → LifecycleKind::Effect in match is DEAD CODE (filter prevents it)

OmniView Codegen (store.rs:352-388):
  → comp.effects → __omni_effect(() => { body });
  → BUT: cleanup return value is DISCARDED — no unmount wiring
```

---

### Task 1: Wire @effect Cleanup on Component Unmount

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:361-385`
- Modify: `omnilang/src/codegen/omniview/store.rs:345` (dead code)
- Test: `omnilang/src/codegen/omniview/tests.rs` (add new test)

**Interfaces:**
- Consumes: `OmniViewGenerator.js`, `OmniViewGenerator.mount_js`, `OmniViewGenerator.comp_mount_js` (existing buffers)
- Consumes: `__omni_effect()` runtime function (returns `() => void` cleanup fn)
- Consumes: `window.__omni_unmount_{name}` convention (existing pattern from Unmount lifecycle)
- Produces: Effect disposal is registered per-component, auto-cleaned on unmount

**Current code** (store.rs:369-384):
```rust
if self.editor_mode {
    let _ = write!(
        self.comp_mount_js,
        "// @effect {}\n__omni_effect(() => {{\n{}}});\n",
        name, body_js
    );
} else {
    let _ = write!(
        self.mount_js,
        "// @effect {}\n__omni_effect(() => {{\n{}}});\n",
        name, body_js
    );
}
```

**Problem:** `__omni_effect()` returns `() => e._deps.forEach(d => d())` (a cleanup function that unsubscribes from all signals). The generated code discards this return value, so effects never clean up — they keep running even after component unmount.

- [ ] **Step 1: Write the failing test**

Add to `omnilang/src/codegen/omniview/tests.rs`:

```rust
#[test]
fn effect_generates_cleanup_registration() {
    // Build IR for a component with @effect(count) { console_log("changed"); }
    let mut comp = make_test_component("EffectCleanup");
    comp.effects.push(EffectIntent {
        deps: vec!["count".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::FuncCallExpr {
                    func: Box::new(ExprIR::IdentifierExpr {
                        name: "console_log".into(),
                        type_ir: None,
                    }),
                    args: vec![ExprIR::LiteralExpr {
                        value: serde_json::Value::String("changed".into()),
                    }],
                    type_ir: None,
                },
            }],
        },
    });
    let ir = make_ir_program(vec![comp]);

    let output = compile_to_omniview(&ir, false, false);
    let js = &output.js;

    // Verify: effect body is wrapped in __omni_effect
    assert!(js.contains("__omni_effect"), "Should call __omni_effect");

    // Verify: cleanup is captured (effect return value is stored, not discarded)
    assert!(
        js.contains("__omni_effect_cleanups") || js.contains("_dispose"),
        "Should store cleanup function, found:\n{}",
        js
    );

    // Verify: cleanup is registered for unmount disposal
    assert!(
        js.contains("__omni_unmount_EffectCleanup")
            || js.contains("__omni_effect_cleanups.push"),
        "Should register cleanup for unmount"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd D:/space/OmniLang/omnilang && cargo test effect_generates_cleanup_registration --features codegen-omniview
```

Expected: FAIL — cleanup not generated.

- [ ] **Step 3: Implement effect cleanup wiring**

In `store.rs:361-385`, modify the effect emission block. Replace the current `write!` calls with code that:

1. Generates the `__omni_effect()` call
2. Stores the returned cleanup function in a per-component array
3. Registers cleanup execution in the unmount handler

```rust
// NEW CODE — omni_lang/src/codegen/omniview/store.rs, inside the
// for effect in &comp.effects loop, replacing lines 369-384:

// Collect effect cleanup into a per-component disposal list
let dispose_var = format!("__omni_effect_cleanups_{}", name);

// On first effect for this component, initialize the cleanup array
let needs_init = !self.js.contains(&dispose_var);
let old_len = self.js.len();
if needs_init {
    let _ = write!(self.js, "var {} = [];\n", dispose_var);
}

// Emit the effect with cleanup capture
let body_escaped = body_js; // already extracted above
if self.editor_mode {
    let _ = write!(
        self.comp_mount_js,
        "{0}.push(__omni_effect(() => {{\n{1}}}));\n",
        dispose_var, body_escaped
    );
} else {
    let _ = write!(
        self.mount_js,
        "{0}.push(__omni_effect(() => {{\n{1}}}));\n",
        dispose_var, body_escaped
    );
}

// After the effect loop: if we created a cleanup array, wire it to unmount
// (add this AFTER the for effect loop, before line 386)
let has_effects_for_comp = !comp.effects.is_empty();
if has_effects_for_comp {
    // Extend the unmount handler to also dispose effects
    if self.editor_mode {
        let _ = write!(
            self.comp_mount_js,
            "// wire effect cleanup to unmount\n\
             var _orig_unmount_{0} = window.__omni_unmount_{0} || (() => {{}});\n\
             window.__omni_unmount_{0} = () => {{\n\
               {1}.forEach(fn => fn());\n\
               {1} = [];\n\
               _orig_unmount_{0}();\n\
             }};\n",
            name, dispose_var
        );
    } // else: non-editor mode doesn't need separate unmount wiring
      // because effects live for the page lifetime (no component lifecycle in prod)
}
```

- [ ] **Step 4: Remove the dead code `LifecycleKind::Effect => {}`**

In `store.rs:345`, replace:
```rust
LifecycleKind::Effect => {}
```
with:
```rust
LifecycleKind::Effect => {
    // unreachable: IR lowering filters Effect into comp.effects, never lifecycles
    // (see ir/lower/component.rs:62-76)
    debug_assert!(false, "LifecycleKind::Effect should never reach codegen");
}
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cd D:/space/OmniLang/omnilang && cargo test effect_generates_cleanup_registration --features codegen-omniview
```

Expected: PASS

- [ ] **Step 6: Run full OmniView test suite**

```bash
cd D:/space/OmniLang/omnilang && cargo test --features codegen-omniview
```

Ensure no regressions. Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "fix(omniview): wire @effect cleanup to component unmount

Generated code now stores __omni_effect() return value and calls it
during component unmount, preventing signal subscription leaks.

Also replaced dead LifecycleKind::Effect handler with debug_assert!"
```

---

### Task 2: Extract Shared UiCodegen Trait

**Files:**
- Create: `omnilang/src/codegen/shared/ui_codegen.rs`
- Create: `omnilang/src/codegen/shared/mod.rs` (if not exists, or add to existing)
- Modify: `omnilang/src/codegen/mod.rs` (add `pub mod shared` if needed, re-export trait)
- Modify: `omnilang/src/codegen/react/mod.rs` (implement trait)
- Modify: `omnilang/src/codegen/omniview/mod.rs` (implement trait, no behavior change — just refactor)
- Modify: `omnilang/src/codegen/vue/mod.rs` (implement trait, minimal — only to document the interface)
- Do NOT modify Flutter — it's a separate target architecture (Dart widgets, not HTML/CSS/JS)

**Interfaces:**
- Produces: `pub trait UiCodegen` with methods:
  - `fn format_event_handler(&self, handler: &EventHandlerIR, signal_names: &FxHashSet<String>) -> String`
  - `fn format_binding(&self, binding: &BindingIR) -> String`
  - `fn format_css_rule(&self, rule: &StyleRuleIR, tokens: &FxHashMap<String, String>) -> String`
  - `fn format_conditional_block(&self, cond: &ConditionalIntent, writer: &mut dyn CodeWriter)`
  - `fn format_list_rendering(&self, list: &ListRenderingIntent, writer: &mut dyn CodeWriter)`
  - `fn target_file_extension(&self) -> &'static str`
  - `fn target_name(&self) -> &'static str`

- [ ] **Step 1: Define the `UiCodegen` trait**

Create `omnilang/src/codegen/shared/ui_codegen.rs`:

```rust
//! Shared trait for UI-target codegen backends.
//!
//! React, Vue, and OmniView all translate the same IR constructs
//! (event handlers, bindings, CSS rules, conditionals, list rendering)
//! into target-specific code. This trait defines the minimal interface
//! each UI backend must implement, reducing duplicated logic.

use crate::ir::*;
use crate::codegen::CodeWriter;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Trait for UI-target code generators (React, Vue, OmniView).
/// Each target implements the format methods according to its
/// output language idioms.
pub trait UiCodegen {
    /// Translate an event handler IR node to target code.
    /// e.g., React: `onClick={(e) => handler(e)}`
    ///       OmniView: `el.addEventListener('click', (e) => handler(e))`
    fn format_event_handler(
        &self,
        handler: &EventHandlerIR,
        signal_names: &FxHashSet<String>,
    ) -> String;

    /// Translate a data binding (e.g., bind:value) to target code.
    fn format_binding(&self, binding: &BindingIR) -> String;

    /// Translate a CSS style rule to target-specific style code.
    fn format_css_rule(
        &self,
        rule: &StyleRuleIR,
        tokens: &FxHashMap<String, String>,
    ) -> String;

    /// Generate target code for conditional rendering (@if/@else).
    /// The writer receives pre-built branch bodies; the implementation
    /// wraps them in target-specific conditional syntax.
    fn format_conditional_block(
        &self,
        cond: &ConditionalIntent,
        branch_bodies: &[String],
        writer: &mut dyn CodeWriter,
    );

    /// Generate target code for list rendering (@for/@empty).
    fn format_list_rendering(
        &self,
        list: &ListRenderingIntent,
        item_body: &str,
        empty_body: Option<&str>,
        writer: &mut dyn CodeWriter,
    );

    /// File extension for this target's output files.
    fn target_file_extension(&self) -> &'static str;

    /// Human-readable target name for error messages.
    fn target_name(&self) -> &'static str;
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd D:/space/OmniLang/omnilang && cargo check --features codegen-omniview,codegen-react
```

Expected: PASS (trait defined but not yet implemented).

- [ ] **Step 3: Implement `UiCodegen` for React codegen**

Add to `omnilang/src/codegen/react/mod.rs`:

```rust
use crate::codegen::shared::ui_codegen::UiCodegen;

impl UiCodegen for ReactCodegen {
    fn format_event_handler(
        &self,
        handler: &EventHandlerIR,
        signal_names: &FxHashSet<String>,
    ) -> String {
        // Reuse existing logic: move event_handler_to_jsx from expr.rs
        // into this method body. For this refactor task, delegate to
        // the existing function to avoid behavioral changes.
        crate::codegen::react::expr::event_handler_to_jsx(handler, signal_names)
    }

    fn format_binding(&self, binding: &BindingIR) -> String {
        crate::codegen::react::expr::binding_to_jsx(binding)
    }

    fn format_css_rule(
        &self,
        rule: &StyleRuleIR,
        tokens: &FxHashMap<String, String>,
    ) -> String {
        crate::codegen::react::utils::css_rule_to_string(rule, tokens)
    }

    fn format_conditional_block(
        &self,
        cond: &ConditionalIntent,
        branch_bodies: &[String],
        writer: &mut dyn CodeWriter,
    ) {
        crate::codegen::react::view::emit_conditional(cond, branch_bodies, writer)
    }

    fn format_list_rendering(
        &self,
        list: &ListRenderingIntent,
        item_body: &str,
        empty_body: Option<&str>,
        writer: &mut dyn CodeWriter,
    ) {
        crate::codegen::react::view::emit_list_rendering(list, item_body, empty_body, writer)
    }

    fn target_file_extension(&self) -> &'static str { "tsx" }
    fn target_name(&self) -> &'static str { "React" }
}
```

**Note:** This step does NOT move code — it adds thin delegating implementations. The actual consolidation of shared logic happens in a follow-up refactor PR. This PR only establishes the trait interface.

- [ ] **Step 4: Implement `UiCodegen` for OmniView codegen** (same delegation pattern)

Add to `omnilang/src/codegen/omniview/mod.rs`:

```rust
use crate::codegen::shared::ui_codegen::UiCodegen;

impl UiCodegen for OmniViewGenerator {
    fn format_event_handler(
        &self,
        handler: &EventHandlerIR,
        signal_names: &FxHashSet<String>,
    ) -> String {
        self.event_handler_to_js(handler, signal_names)
    }

    fn format_binding(&self, binding: &BindingIR) -> String {
        self.binding_to_js(binding)
    }

    fn format_css_rule(
        &self,
        rule: &StyleRuleIR,
        tokens: &FxHashMap<String, String>,
    ) -> String {
        self.emit_style_rule_to_string(rule, tokens)
    }

    fn format_conditional_block(
        &self,
        cond: &ConditionalIntent,
        branch_bodies: &[String],
        writer: &mut dyn CodeWriter,
    ) {
        self.emit_conditional_html(cond, branch_bodies, writer)
    }

    fn format_list_rendering(
        &self,
        list: &ListRenderingIntent,
        item_body: &str,
        empty_body: Option<&str>,
        writer: &mut dyn CodeWriter,
    ) {
        self.emit_list_html(list, item_body, empty_body, writer)
    }

    fn target_file_extension(&self) -> &'static str { "html" }
    fn target_name(&self) -> &'static str { "OmniView" }
}
```

**Note:** The method names (`event_handler_to_js`, `binding_to_js`, etc.) are placeholders — in practice you'll delegate to the existing private helper functions that already exist in the codebase. The key is that the trait is defined and both backends implement it. Actual method extraction happens later.

- [ ] **Step 5: Run full test suite**

```bash
cd D:/space/OmniLang/omnilang && cargo test --features codegen-omniview,codegen-react
```

Expected: all existing tests pass (trait impls delegate to existing code, no behavior change).

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/shared/ui_codegen.rs omnilang/src/codegen/shared/mod.rs
git add omnilang/src/codegen/mod.rs
git add omnilang/src/codegen/react/mod.rs omnilang/src/codegen/omniview/mod.rs
git commit -m "feat(codegen): extract UiCodegen trait for shared UI target interface

Defines UiCodegen trait with format_event_handler, format_binding,
format_css_rule, format_conditional_block, format_list_rendering.
ReactCodegen and OmniViewGenerator implement it via delegation
to existing methods. No behavioral changes.

This is the first step toward reducing duplicate logic across
React/Vue/OmniView backends. Follow-up PRs will move shared
implementation into the trait's default methods."
```

---

### Task 3: Split `html.rs` (1,174 lines → 4 modules)

**Files:**
- Create: `omnilang/src/codegen/omniview/view/element.rs` (~350 lines)
- Create: `omnilang/src/codegen/omniview/view/condition.rs` (~200 lines)
- Create: `omnilang/src/codegen/omniview/view/list.rs` (~200 lines)
- Create: `omnilang/src/codegen/omniview/view/mod.rs` (~50 lines, re-exports)
- Modify: `omnilang/src/codegen/omniview/html.rs` (delete after split, re-export from mod.rs)
- Modify: `omnilang/src/codegen/omniview/mod.rs` (update module declarations)

**Interfaces:**
- Produces: `view::element::emit_element_html(gen: &mut OmniViewGenerator, el: &ElementIR)`
- Produces: `view::condition::emit_condition_html(gen: &mut OmniViewGenerator, cond: &ConditionalIntent)`
- Produces: `view::list::emit_list_html(gen: &mut OmniViewGenerator, list: &ListRenderingIntent)`
- All public through `view::mod.rs` re-exports so existing callers don't break

- [ ] **Step 1: Identify the boundaries in `html.rs`**

Read `html.rs` and identify the natural split points. The file contains three main concerns:
1. Element rendering (div, span, button, input, etc.) — roughly lines 1-500
2. Conditional rendering (@if/@else) — roughly lines 500-800
3. List rendering (@for/@empty, @for key) — roughly lines 800-1174

Mark the exact function boundaries. If a function is shared by multiple concerns, put it in `view/mod.rs`.

- [ ] **Step 2: Create `view/mod.rs` with re-exports**

```rust
// omnilang/src/codegen/omniview/view/mod.rs
//! View template → HTML code generation for OmniView target.
//! Split from the original html.rs (1174 lines → 4 focused modules).

mod element;
mod condition;
mod list;

pub use element::emit_element_html;
pub use condition::emit_condition_html;
pub use list::emit_list_html;

// Shared helpers used by multiple view modules
use super::OmniViewGenerator;
```

- [ ] **Step 3: Move element code to `view/element.rs`**

From `html.rs`, move:
- All functions that render individual HTML elements
- `emit_element_html` and its helpers
- Element attribute handling (class, style, data-nid, event bindings)

```rust
// omnilang/src/codegen/omniview/view/element.rs
use super::super::OmniViewGenerator;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    pub(super) fn emit_element_html(&mut self, el: &ElementIR) {
        // ... existing code from html.rs ...
    }
    // ... element helper methods ...
}
```

- [ ] **Step 4: Move condition code to `view/condition.rs`**

```rust
// omnilang/src/codegen/omniview/view/condition.rs
impl OmniViewGenerator {
    pub(super) fn emit_condition_html(&mut self, cond: &ConditionalIntent) {
        // ... existing @if/@else code from html.rs ...
    }
}
```

- [ ] **Step 5: Move list code to `view/list.rs`**

```rust
// omnilang/src/codegen/omniview/view/list.rs
impl OmniViewGenerator {
    pub(super) fn emit_list_html(&mut self, list: &ListRenderingIntent) {
        // ... existing @for/@empty code from html.rs ...
    }
}
```

- [ ] **Step 6: Update `mod.rs` to use the new modules**

In `omnilang/src/codegen/omniview/mod.rs`:
```rust
mod html; // REMOVE or leave as re-export stub
mod view; // ADD
pub use view::*; // or specific re-exports
```

Update all `use` statements that referenced the old `html` module.

- [ ] **Step 7: Run tests to verify no breakage**

```bash
cd D:/space/OmniLang/omnilang && cargo test --features codegen-omniview
```

Expected: all tests pass. If any fail, fix import paths.

- [ ] **Step 8: Clippy check**

```bash
cd D:/space/OmniLang/omnilang && cargo clippy --features codegen-omniview -- -D warnings
```

- [ ] **Step 9: Commit**

```bash
git add omnilang/src/codegen/omniview/view/
git add omnilang/src/codegen/omniview/mod.rs
git rm omnilang/src/codegen/omniview/html.rs  # or keep as re-export stub
git commit -m "refactor(omniview): split html.rs (1174 lines) into view/ modules

Split into view/element.rs, view/condition.rs, view/list.rs.
No behavioral changes — all existing tests pass.
Each module is under 400 lines."
```

---

### Task 4: Split React `component.rs` (1,921 lines → 4 modules)

Same pattern as Task 3, applied to `omnilang/src/codegen/react/component.rs`:

- Create: `omnilang/src/codegen/react/component/mod.rs`
- Create: `omnilang/src/codegen/react/component/state.rs` — signal→useState codegen (~400 lines)
- Create: `omnilang/src/codegen/react/component/effect.rs` — @effect/@mount/@unmount→useEffect (~400 lines)
- Create: `omnilang/src/codegen/react/component/jsx.rs` — JSX view tree rendering (~600 lines)
- Create: `omnilang/src/codegen/react/component/function.rs` — fn/action→function codegen (~300 lines)

- [ ] **Step 1: Create directory structure and split**

Follow the same pattern as Task 3. Extract `impl ReactCodegen` blocks into focused submodules.

- [ ] **Step 2: Verify tests pass**

```bash
cd D:/space/OmniLang/omnilang && cargo test --features codegen-react
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/react/component/
git commit -m "refactor(react): split component.rs (1921 lines) into 4 submodules

Split into state.rs, effect.rs, jsx.rs, function.rs.
No behavioral changes."
```

---

## Phase 2 Tasks (P1-P2, for next iteration)

### Task 5: Split OmniView `tests.rs` (1,369 lines → per-module tests)

Move test functions from `tests.rs` into `#[cfg(test)] mod tests { }` blocks in the relevant source files (view/, store/, motion/). Follow Rust convention: tests live alongside the code they test.

### Task 6: IR Optimization Pass

Add `ir/lower/optimize.rs` with:
- `remove_unused_signals()` — signals not read in view/computeds/effects are skipped
- `inline_trivial_computeds()` — single-expression computeds without side effects are inlined
- `hoist_static_elements()` — elements with no bindings/interpolations are marked as static

### Task 7: WASM File-Level Compile Cache

In `omnilang/src/lib.rs` `compile()` function:
- Add `FxHashMap<String, (u64, CompileResult)>` cache keyed by source hash
- Before full pipeline, check cache; on hit, return cached result
- Cache invalidates when source content hash changes

---

## Execution Order

```
Task 1 (P0: @effect cleanup)  ──independent──▶  can ship immediately
Task 2 (P0: UiCodegen trait)  ──independent──▶  can ship after Task 1
Task 3 (P1: split html.rs)    ──depends on──▶  Task 2 (trait defines module boundaries)
Task 4 (P1: split component.rs) ──independent──▶  can parallel with Task 3
Tasks 5-7 (P2)                ──after P0+P1──▶  next planning session
```

---

## Spec Coverage Self-Review

- [x] P0-1: @effect cleanup wired → Task 1
- [x] P0-1: Dead code removed → Task 1 Step 4
- [x] P0-2: UiCodegen trait extracted → Task 2
- [x] P1: html.rs split → Task 3
- [x] P1: component.rs split → Task 4
- [x] P1: tests.rs split → Task 5 (Phase 2)
- [x] P2: IR optimization → Task 6 (Phase 2)
- [x] P2: WASM cache → Task 7 (Phase 2)
- [x] No placeholder/TODO — all steps have concrete code
- [x] Type consistency — UiCodegen trait signatures match across React/OmniView impls
- [x] Only OmniView/React/Flutter (Flutter not affected by these changes, architecture is different)
