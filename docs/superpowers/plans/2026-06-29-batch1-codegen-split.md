# Batch 1: Codegen File Splitting — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split 8 oversized codegen `.rs` files into sub-module directories, each file ≤800 lines of code. Zero behavior changes — all 375 tests must pass after every task.

**Architecture:** Each `src/codegen/<target>.rs` becomes `src/codegen/<target>/` with a `mod.rs` plus concern-specific sub-modules (`expr.rs`, `view.rs`, `utils.rs`, etc.). The `#[cfg(test)] mod tests` block moves to `tests.rs`. Cross-module private functions become `pub(super)`. The parent `codegen/mod.rs` changes `pub mod <target>;` from a file reference to a directory reference (no syntax change needed — `pub mod foo;` resolves to either `foo.rs` or `foo/mod.rs`).

**Tech Stack:** Rust 2021, cargo test, cargo fmt, cargo clippy

**Safety net:** `cargo test --lib` must return "375 passed; 0 failed" after every commit.

**Key mechanics:**

1. **`mv`, not `cp`:** Use `mv old.rs newdir/mod.rs` to avoid having both `old.rs` and `old/mod.rs` exist simultaneously (Rust rejects duplicate module definitions). Git detects the rename automatically — no `git rm` needed.

2. **Re-export pattern:** Items that were `pub fn` in the original file (e.g., `compile_files`, `dart_type`, `js_type`) must remain accessible from their original module path after moving to a sub-module. In the new `mod.rs`, add:
   ```rust
   mod submodule;
   pub(crate) use submodule::*;  // re-export everything at the parent module level
   ```
   This keeps external callers (like `codegen/mod.rs` → `omniview::compile_files`) working without changes.

3. **Test imports:** When tests move to `tests.rs`, they become `mod tests;` inside the directory. Test code that used `use super::*;` should change to `use super::super::*;` if tests reference items from sibling sub-modules. Simpler: add `use super::*;` in `tests.rs` to pull in everything from `mod.rs`, which should re-export from sub-modules.

4. **`pub mod` declarations in `codegen/mod.rs` need NO changes.** `pub mod rust;` resolves to `rust/mod.rs` automatically when `rust.rs` is removed.

---

### Task 1: Delete dead lsp.rs

**Files:**
- Delete: `src/lsp.rs`

- [ ] **Step 1: Delete lsp.rs**

```bash
rm src/lsp.rs
```

- [ ] **Step 2: Verify build and tests**

Run: `cd omnilang && cargo test --lib 2>&1`
Expected: "375 passed; 0 failed" — lsp.rs was never declared as a module, so removing it has no effect.

- [ ] **Step 3: Commit**

```bash
git add src/lsp.rs
git commit -m "chore: remove dead lsp.rs (never declared as module)"
```

---

### Task 2: Split rust.rs — test extraction only

**Files:**
- Create: `src/codegen/rust/mod.rs`
- Create: `src/codegen/rust/expr.rs`
- Create: `src/codegen/rust/tests.rs`
- Delete: `src/codegen/rust.rs`

Code is already 787 lines (<800). Only need to extract tests. But since we're moving to a directory, also split out the expression/statement helpers for consistency.

- [ ] **Step 1: Create directory and move the file**

Use `mv` (not `cp`) to avoid having both `rust.rs` and `rust/mod.rs` exist simultaneously, which would cause a "duplicate module" error.

```bash
mkdir -p omnilang/src/codegen/rust
mv omnilang/src/codegen/rust.rs omnilang/src/codegen/rust/mod.rs
```

- [ ] **Step 2: Extract expr.rs**

Functions to move from `mod.rs` to `expr.rs` (lines ~443–737): `format_block_as_rust_expr`, `format_expr_ir`, `format_json_value`, `binary_op_to_rust`, `rs_type`, `format_type_ir_rust`, `generate_struct`, `generate_enum`, `generate_type_alias`, `generate_standalone_function`.

Edit `mod.rs` — replace the moved functions with `mod expr;` and `use expr::*;` (or `pub(super) use expr::*;`). In `expr.rs`, all moved `fn` items become `pub(super) fn`.

Create `src/codegen/rust/expr.rs`:
```rust
//! Expression, statement, and type formatting for Rust codegen.
use super::CodeWriter;
use crate::ir::*;
use serde;

pub(super) fn format_block_as_rust_expr(block: &BlockIR) -> String {
    // ... (moved from mod.rs)
}

pub(super) fn format_expr_ir(expr: &ExprIR) -> String {
    // ... (moved from mod.rs)
}

pub(super) fn format_json_value(value: &serde_json::Value) -> String {
    // ... (moved from mod.rs)
}

pub(super) fn binary_op_to_rust(op: &str) -> &str {
    // ... (moved from mod.rs)
}

pub fn rs_type(kind: &str, type_args: &[TypeIR]) -> String {
    // ... (moved from mod.rs, keep pub since it's called from super::mod.rs)
}

pub(super) fn format_type_ir_rust(ty: &TypeIR) -> String {
    // ... (moved from mod.rs)
}

pub(super) fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR) {
    // ... (moved from mod.rs)
}

pub(super) fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR) {
    // ... (moved from mod.rs)
}

pub(super) fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR) {
    // ... (moved from mod.rs)
}

pub(super) fn generate_standalone_function(w: &mut CodeWriter, func: &FunctionIR) {
    // ... (moved from mod.rs)
}
```

- [ ] **Step 3: Extract tests.rs**

Move the `#[cfg(test)] mod tests { ... }` block from the end of `mod.rs` to `tests.rs`. The tests use `use super::*;` which still works from `tests.rs`.

Create `src/codegen/rust/tests.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::super::*;  // access items from mod.rs
    // ... (all test functions moved verbatim)
}
```

- [ ] **Step 4: Update mod.rs**

At the top of `src/codegen/rust/mod.rs`, add:
```rust
mod expr;
mod tests;

use expr::*;
```

Remove the extracted function bodies and the test module. Keep: `RustCodegen` struct, `impl Codegen for RustCodegen`, `generate_component`, `generate_store`, `generate_function`, `generate_statement`.

- [ ] **Step 5: Verify build and tests**

Run: `cd omnilang && cargo test --lib 2>&1`
Expected: "375 passed; 0 failed"

- [ ] **Step 6: Format**

```bash
cd omnilang && cargo fmt
```

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/rust/
git commit -m "refactor: split rust codegen into sub-modules (787 lines → dir)"
```

---

### Task 3: Split dart.rs — test extraction only

**Files:**
- Create: `src/codegen/dart/mod.rs`
- Create: `src/codegen/dart/expr.rs`
- Create: `src/codegen/dart/tests.rs`
- Delete: `src/codegen/dart.rs`

Same pattern as Task 2. Code is 769 lines (<800).

- [ ] **Step 1: Create directory and move the file**

```bash
mkdir -p omnilang/src/codegen/dart
mv omnilang/src/codegen/dart.rs omnilang/src/codegen/dart/mod.rs
```

- [ ] **Step 2: Extract expr.rs**

Functions to move (lines ~409–768): `format_block_as_dart_expr`, `format_expr_ir`, `format_json_value`, `binary_op_to_dart`, `dart_type` (keep `pub`), `format_type_ir_dart`, `generate_struct`, `generate_enum`, `generate_type_alias`, `generate_standalone_function`.

All become `pub(super) fn` except `dart_type` which stays `pub fn` (called from `flutter.rs`).

- [ ] **Step 3: Extract tests.rs**

Move `#[cfg(test)] mod tests { ... }` to `tests.rs`.

- [ ] **Step 4: Update mod.rs**

Add `mod expr; mod tests;` at top. Remove moved functions and test module.

- [ ] **Step 5: Verify and commit**

Run: `cd omnilang && cargo test --lib 2>&1`
Expected: "375 passed; 0 failed"

```bash
cd omnilang && cargo fmt
git add omnilang/src/codegen/dart/
git rm omnilang/src/codegen/dart.rs
git commit -m "refactor: split dart codegen into sub-modules (769 lines → dir)"
```

---

### Task 4: Split javascript.rs

**Files:**
- Create: `src/codegen/javascript/mod.rs`
- Create: `src/codegen/javascript/expr.rs`
- Create: `src/codegen/javascript/utils.rs`
- Create: `src/codegen/javascript/tests.rs`
- Delete: `src/codegen/javascript.rs`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p omnilang/src/codegen/javascript
cp omnilang/src/codegen/javascript.rs omnilang/src/codegen/javascript/mod.rs
```

- [ ] **Step 2: Extract expr.rs** (~350 lines)

Functions to move (lines ~472–777):
- `format_expr_ir`, `format_block_as_js_expr`, `format_stmt_ir`, `format_json_value`, `binary_op_to_js`

All become `pub(super) fn`.

- [ ] **Step 3: Extract utils.rs** (~261 lines)

Functions to move (lines ~724–911):
- `js_type` (keep `pub fn` — called from `mod.rs`)
- `format_type_ir_js`, `generate_struct`, `generate_enum`, `generate_type_alias`, `generate_standalone_function`

`pub(super) fn` except `js_type` which stays `pub fn`.

- [ ] **Step 4: Extract tests.rs**

Move `#[cfg(test)] mod tests { ... }` (lines ~912–2220).

- [ ] **Step 5: Update mod.rs**

Keep: `JavaScriptCodegen` struct, `impl JavaScriptCodegen`, `impl Codegen for JavaScriptCodegen`, `generate_component`, `generate_store`, `generate_function`, `generate_statement`. Remove everything else.

Add at top:
```rust
mod expr;
mod utils;
mod tests;
use expr::*;
use utils::*;
```

- [ ] **Step 6: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1  # Expected: 375 passed
cargo fmt
git add omnilang/src/codegen/javascript/
git rm omnilang/src/codegen/javascript.rs
git commit -m "refactor: split javascript codegen into sub-modules (911→mod+expr+utils+tests)"
```

---

### Task 5: Split dioxus.rs

**Files:**
- Create: `src/codegen/dioxus/mod.rs`
- Create: `src/codegen/dioxus/view.rs`
- Create: `src/codegen/dioxus/expr.rs`
- Create: `src/codegen/dioxus/tests.rs`
- Delete: `src/codegen/dioxus.rs`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p omnilang/src/codegen/dioxus
cp omnilang/src/codegen/dioxus.rs omnilang/src/codegen/dioxus/mod.rs
```

- [ ] **Step 2: Extract view.rs** (~350 lines)

Functions to move (lines ~245–1059):
- `gen_view_node_dioxus`, `format_inline_handler_dioxus`

Become `pub(super) fn`.

- [ ] **Step 3: Extract expr.rs** (~259 lines)

Functions to move (lines ~613–1059):
- `format_block_as_dioxus_expr`, `format_expr_ir_dioxus`, `format_json_value_dioxus`, `binary_op_to_dioxus`, `to_snake_case`, `to_dioxus_attr`, `to_dioxus_event`, `field_extract_dioxus`, `emit_event_object_dioxus`

All `pub(super) fn`.

- [ ] **Step 4: Extract tests.rs** and update mod.rs

Move test module. Keep `DioxusCodegen`, `impl Codegen for DioxusCodegen`, and the `generate()` method's component/store generation in `mod.rs`.

- [ ] **Step 5: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/codegen/dioxus/
git rm omnilang/src/codegen/dioxus.rs
git commit -m "refactor: split dioxus codegen into sub-modules (1059→mod+view+expr+tests)"
```

---

### Task 6: Split vue.rs

**Files:**
- Create: `src/codegen/vue/mod.rs`
- Create: `src/codegen/vue/template.rs`
- Create: `src/codegen/vue/expr.rs`
- Create: `src/codegen/vue/tests.rs`
- Delete: `src/codegen/vue.rs`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p omnilang/src/codegen/vue
cp omnilang/src/codegen/vue.rs omnilang/src/codegen/vue/mod.rs
```

- [ ] **Step 2: Extract template.rs** (~352 lines)

Functions to move (lines ~297–1002):
- `gen_template_node`, `format_inline_handler_vue`, `generate_function_vue`

All `pub(super) fn`.

- [ ] **Step 3: Extract expr.rs** (~200 lines)

Functions to move (lines ~753–1002):
- `format_expr_vue`, `generate_statement_vue`, `format_block_as_vue_expr`, `capitalize_first`, `format_json_value_vue`, `binary_op_to_vue`, `format_css_value`, `to_css_prop`, `to_vue_attr`, `to_vue_event`

All `pub(super) fn`.

- [ ] **Step 4: Extract tests.rs** and update mod.rs

Move test module. Keep `VueCodegen` struct, `impl Codegen for VueCodegen`, `impl VueCodegen` with `generate()` method in `mod.rs`.

- [ ] **Step 5: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/codegen/vue/
git rm omnilang/src/codegen/vue.rs
git commit -m "refactor: split vue codegen into sub-modules (1002→mod+template+expr+tests)"
```

---

### Task 7: Split react.rs

**Files:**
- Create: `src/codegen/react/mod.rs`
- Create: `src/codegen/react/view.rs`
- Create: `src/codegen/react/expr.rs`
- Create: `src/codegen/react/utils.rs`
- Create: `src/codegen/react/tests.rs`
- Delete: `src/codegen/react.rs`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p omnilang/src/codegen/react
cp omnilang/src/codegen/react.rs omnilang/src/codegen/react/mod.rs
```

- [ ] **Step 2: Extract view.rs** (~700 lines)

Functions to move (lines ~361–1273):
- `emit_motion_effects_tsx`, `has_enter_trigger`, `emit_driven_by_effects_tsx`, `has_driven_by`, `emit_inview_effects_tsx`, `has_inview_trigger`, `emit_scroll_effects_tsx`, `has_scroll_trigger`, `has_drag_trigger`, `emit_drag_effects_tsx`, `gen_view_node_tsx`, `format_inline_handler`

All `pub(super) fn`.

- [ ] **Step 3: Extract expr.rs** (~350 lines)

Functions to move (lines ~1296–1627):
- `generate_statement_tsx`, `format_block_as_tsx_expr`, `format_expr_ir_tsx`

All `pub(super) fn`.

- [ ] **Step 4: Extract utils.rs** (~355 lines)

Functions to move (lines ~1628–1955):
- `capitalize_first`, `format_json_value_tsx`, `binary_op_to_tsx`, `format_css_value`, `to_css_prop`, `to_jsx_attr`, `to_react_event`, `field_extract_react`, `emit_event_object_react`, `getter_expr_contains_ident` (standalone fn at line 15)

`getter_expr_contains_ident` becomes `pub(super) fn`. Others become `pub(super) fn`.

- [ ] **Step 5: Extract tests.rs** and update mod.rs

Move test module. Keep `ReactCodegen` struct, `impl Codegen for ReactCodegen`, `impl ReactCodegen` with `generate()`, `gen_component_tsx`, `gen_component_css`, `gen_store_hook` in `mod.rs`.

- [ ] **Step 6: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/codegen/react/
git rm omnilang/src/codegen/react.rs
git commit -m "refactor: split react codegen into sub-modules (1955→mod+view+expr+utils+tests)"
```

---

### Task 8: Split flutter.rs

**Files:**
- Create: `src/codegen/flutter/mod.rs`
- Create: `src/codegen/flutter/view.rs`
- Create: `src/codegen/flutter/expr.rs`
- Create: `src/codegen/flutter/stmt.rs`
- Create: `src/codegen/flutter/motion.rs`
- Create: `src/codegen/flutter/utils.rs`
- Create: `src/codegen/flutter/tests.rs`
- Delete: `src/codegen/flutter.rs`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p omnilang/src/codegen/flutter
cp omnilang/src/codegen/flutter.rs omnilang/src/codegen/flutter/mod.rs
```

- [ ] **Step 2: Extract view.rs** (~520 lines)

Functions to move (lines ~449–868):
- `format_view_tree`, `format_element`, `format_conditional`, `format_list_rendering`, `format_inline_handler_flutter`

All `pub(super) fn`.

- [ ] **Step 3: Extract expr.rs** (~400 lines)

Functions to move (lines ~1130–1317):
- `format_block_as_flutter_expr`, `format_expr_ir_flutter`, `format_json_value_flutter`, `binary_op_to_flutter`

All `pub(super) fn`.

- [ ] **Step 4: Extract stmt.rs** (~200 lines)

Functions to move (lines ~888–1129):
- `generate_statement_flutter`, `generate_function_flutter`

All `pub(super) fn`.

- [ ] **Step 5: Extract motion.rs** (~700 lines)

Functions to move (lines ~1464–2520):
- `comp_has_spring_motion`, `comp_has_path_motion`, `motion_has_path`, `extract_keyframe_properties`, `extract_tween_begin`, `extract_tween_end`, `extract_duration_ms`, `format_easing_flutter`, `format_named_curve`, `build_animated_builder_body`, `wrap_with_motion_widgets`, `wrap_flutter_states`, `parse_dart_duration_ms`, `find_enter_motions`, `find_enter_inner`, `has_trigger_kind_in_view`, `has_trigger_kind_scroll`, `emit_inview_widget_class`, `emit_scroll_widget_class`, `has_trigger_kind_drag`, `emit_drag_widget_class`, `field_extract_flutter`, `emit_event_object_flutter`

All `pub(super) fn`.

- [ ] **Step 6: Extract utils.rs** (~195 lines)

Functions to move (lines ~1348–1463):
- `tag_to_flutter_widget`, `event_to_flutter_callback`, `prop_to_flutter_param_name`, `binding_to_flutter_param`, `capitalize_first`

All `pub(super) fn`.

- [ ] **Step 7: Extract tests.rs** and update mod.rs

Move test module. Keep `FlutterCodegen` struct, `impl Codegen for FlutterCodegen`, `impl FlutterCodegen` with `generate()`, `gen_component_dart`, `gen_store_dart` in `mod.rs`.

- [ ] **Step 8: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/codegen/flutter/
git rm omnilang/src/codegen/flutter.rs
git commit -m "refactor: split flutter codegen into sub-modules (2515→mod+view+expr+stmt+motion+utils+tests)"
```

---

### Task 9: Split omniview.rs (Part 1 — runtime + store + stmt + utils)

This is the hardest file (5065 code lines). Split into two subtasks to keep commits manageable.

**Files (Part 1):**
- Create: `src/codegen/omniview/mod.rs`
- Create: `src/codegen/omniview/runtime.rs`
- Create: `src/codegen/omniview/store.rs`
- Create: `src/codegen/omniview/stmt.rs`
- Create: `src/codegen/omniview/utils.rs`
- Delete: `src/codegen/omniview.rs`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p omnilang/src/codegen/omniview
cp omnilang/src/codegen/omniview.rs omnilang/src/codegen/omniview/mod.rs
```

- [ ] **Step 2: Extract runtime.rs** (~140 lines)

Functions to move (lines ~251–383 in mod.rs):
- `emit_js_header`, `emit_signal_runtime`, `emit_animation_runtime`

These are methods on `OmniViewGenerator`. Create as a separate `impl OmniViewGenerator` block in `runtime.rs`:

```rust
//! Inline JS runtime emission: signal system, animation runtime.
use super::OmniViewGenerator;

impl OmniViewGenerator {
    pub(super) fn emit_js_header(&mut self) {
        // ... moved verbatim from mod.rs
    }
    pub(super) fn emit_signal_runtime(&mut self) {
        // ... moved verbatim from mod.rs
    }
    pub(super) fn emit_animation_runtime(&mut self) {
        // ... moved verbatim from mod.rs
    }
}
```

- [ ] **Step 3: Extract store.rs** (~500 lines)

Methods to move (lines ~384–881):
- `emit_store`, `generate_component`, `emit_style_assignment`, `emit_signal_decl`, `emit_computed_decl`, `emit_function`, `emit_function_named`

Same pattern — `impl OmniViewGenerator { pub(super) fn ... }` block.

- [ ] **Step 4: Extract stmt.rs** (~290 lines)

Methods to move (lines ~882–1171):
- `emit_statement`, `emit_mutation`

- [ ] **Step 5: Extract utils.rs** (~200 lines)

Standalone functions to move (lines ~4623–5084):
- `serde_value_to_js`, `map_tag`, `map_event`, `emit_event_object`, `collect_for_events`, `map_binary_op`, `to_kebab`, `to_css_property`, `escape_html`, `wrap_debug_html`, `compile_files`

`map_event`, `emit_event_object`, `wrap_debug_html`, `compile_files` keep `pub` or `pub(crate)` as they are today. Others become `pub(super)`.

- [ ] **Step 6: Update mod.rs**

Add module declarations at top:
```rust
mod runtime;
mod store;
mod stmt;
mod utils;
use runtime::*;
use store::*;
use stmt::*;
use utils::*;
```

Remove the extracted function bodies. Keep: `OmniViewOutput`, `ComponentOutput`, `generate()`, `OmniViewGenerator` struct definition, and all remaining methods (`emit_view`, `emit_dom_and_bindings`, `emit_editor_bridge`, `emit_conditional_html`, `emit_list_rendering_html`, `emit_motion_js`, `format_transform_value`, `emit_states_js`, `emit_motion_triggers_for_element`, `emit_timeline_js`, `emit_style`).

- [ ] **Step 7: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1  # Expected: 375 passed
cargo fmt
git add omnilang/src/codegen/omniview/
git rm omnilang/src/codegen/omniview.rs
git commit -m "refactor: split omniview codegen part 1 — runtime, store, stmt, utils"
```

---

### Task 10: Split omniview.rs (Part 2 — view + editor + list + motion + timeline)

**Files:**
- Create: `src/codegen/omniview/view.rs`
- Create: `src/codegen/omniview/editor.rs`
- Create: `src/codegen/omniview/list.rs`
- Create: `src/codegen/omniview/list_render.rs`
- Create: `src/codegen/omniview/motion.rs`
- Create: `src/codegen/omniview/motion_triggers.rs`
- Create: `src/codegen/omniview/timeline.rs`
- Modify: `src/codegen/omniview/mod.rs`

- [ ] **Step 1: Extract view.rs** (~120 lines)

Move `emit_view`, `emit_dom_and_bindings`, `emit_style` to `impl OmniViewGenerator { pub(super) fn ... }` in `view.rs`.

- [ ] **Step 2: Extract editor.rs** (~290 lines)

Move `emit_editor_bridge`, `emit_conditional_html` to `editor.rs`.

- [ ] **Step 3: Extract list.rs and list_render.rs** (~1214 lines total)

This is the hard one. `emit_list_rendering_html` is ~1214 lines. Split at the natural boundary between "state initialization" and "DOM generation":

`list.rs` — `emit_list_rendering_html` left as a thin dispatcher:
```rust
impl OmniViewGenerator {
    pub(super) fn emit_list_rendering_html(&mut self, list: &ListRenderingIntent, indent: usize) -> String {
        self.emit_list_state_init(list);
        self.emit_list_dom(list, indent)
    }

    pub(super) fn emit_list_state_init(&mut self, list: &ListRenderingIntent) {
        // ... first ~600 lines of the original function
    }
}
```

`list_render.rs`:
```rust
impl OmniViewGenerator {
    pub(super) fn emit_list_dom(&mut self, list: &ListRenderingIntent, indent: usize) -> String {
        // ... second ~614 lines of the original function
    }
}
```

The exact split point must be determined by reading the function body — find where state variable setup ends and DOM string building begins.

- [ ] **Step 4: Extract motion.rs** (~460 lines)

Move `emit_motion_js`, `format_transform_value`, `emit_states_js` to `motion.rs`.

- [ ] **Step 5: Extract motion_triggers.rs** (~658 lines)

Move `emit_motion_triggers_for_element` to `motion_triggers.rs`.

- [ ] **Step 6: Extract timeline.rs** (~718 lines)

Move `emit_timeline_js` to `timeline.rs`.

- [ ] **Step 7: Update mod.rs**

Add remaining module declarations:
```rust
mod view;
mod editor;
mod list;
mod list_render;
mod motion;
mod motion_triggers;
mod timeline;
```

`mod.rs` now contains only: type definitions (`OmniViewOutput`, `ComponentOutput`, `OmniViewGenerator` struct fields), `generate()`, and module declarations. ~300 lines.

- [ ] **Step 8: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1  # Expected: 375 passed
cargo fmt
cargo clippy 2>&1 | grep "warning:" | wc -l  # Should be ≤220
git add omnilang/src/codegen/omniview/
git commit -m "refactor: split omniview codegen part 2 — view, editor, list, motion, timeline"
```

---

### Task 11: Final verification — full eval harness

- [ ] **Step 1: Run all tests**

```bash
cd omnilang && cargo test 2>&1
```
Expected: "375 passed; 0 failed; 0 ignored"

- [ ] **Step 2: Run clippy**

```bash
cd omnilang && cargo clippy 2>&1 | grep -c "warning:"
```
Expected: ≤220 (baseline)

- [ ] **Step 3: Run release build**

```bash
cd omnilang && cargo build --release 2>&1
```
Expected: exit 0

- [ ] **Step 4: Run the full eval harness**

```bash
bash .claude/evals/eval-run-omnilang.sh
```
Expected: all C1–C5 and R1–R4 pass or warn, zero failures.

- [ ] **Step 5: Verify file sizes**

```bash
cd omnilang && find src/codegen -name "*.rs" -exec wc -l {} \; | sort -rn | awk '$1 > 800 {print $1, $2}'
```
Expected: zero files over 800 lines in `src/codegen/`.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: final verification — all 8 codegen targets split, 375 tests green"
```
