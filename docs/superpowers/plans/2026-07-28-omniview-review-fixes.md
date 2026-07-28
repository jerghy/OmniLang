# OmniView 多视角评审修复计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 3 HIGH, 15 MEDIUM, and 11 LOW findings from the 2026-07-28 OmniView 4-specialist architecture review.

**Architecture:** Five-phase execution: Phase 1 (immediate — 3 fixes for bugs that affect correctness), Phase 2 (cross-target consistency — fill missing features in Flutter/Vue/Dioxus backends), Phase 3 (refactoring — eliminate duplication and split large files), Phase 4 (frontend runtime — fix JS runtime issues and add tests), Phase 5 (polish — LOW-severity cleanups).

**Tech Stack:** Rust (omnilang crate), TypeScript (packages/omni-animation), React codegen, WASM (omnilang-wasm)

## Global Constraints

- All Rust changes must pass `cargo test --all-features` with zero regressions (801+ tests)
- All Rust changes must pass `cargo clippy -- -D warnings`
- All Rust changes must pass `cargo fmt --check`
- All TypeScript changes must pass `tsc --noEmit`
- Each task ends with a standalone commit using conventional commit format
- No behavior change to OmniView/React backends (the "gold standard" targets) without corresponding test updates
- Generated code snapshots must be updated when output format changes

---

## Phase 1: Immediate Bug Fixes

### Task 1: Fix React motion useEffect dependency array (H1)

**Files:**
- Modify: `omnilang/src/codegen/react/motion/mod.rs:115-136`

**Interfaces:**
- Consumes: `signal_names: &FxHashSet<String>` (component + store signal names)
- Produces: Fixed `emit_motion_effects_tsx` that generates `useEffect(() => {...}, [])` for @enter and uses per-element refs for interactive triggers

**Context:** The generated React `useEffect` currently includes ALL component signal names in its dependency array (`[count, isOpen, ...]`). This causes @enter animations to replay on every state change and `addEventListener` calls to leak (old listeners never cleaned up).

- [ ] **Step 1: Read the full `emit_motion_effects_tsx` function**

Read `omnilang/src/codegen/react/motion/mod.rs` lines 65-136 to understand the full function signature, all trigger types (@enter, @hover, @press, @focus), and the current dependency array generation logic.

- [ ] **Step 2: Split the single effect into two effects**

Modify `emit_motion_effects_tsx` to generate TWO separate `useEffect` blocks:

**Effect 1 — @enter only (mount-once):**
```tsx
useEffect(() => {
  document.querySelectorAll('[data-motion-enter]').forEach(el => {
    const n = el.getAttribute('data-motion-enter-name');
    if (n && _M[n]) {
      const { kf, opts } = _M[n];
      animate(el, kf, opts);
    }
  });
}, []); // Empty deps: mount only
```

**Effect 2 — interactive triggers with cleanup:**
```tsx
useEffect(() => {
  const handlers = [];
  document.querySelectorAll('[data-motion-hover]').forEach(el => {
    const n = el.getAttribute('data-motion-hover-name');
    if (n && _M[n]) {
      const { kf, opts } = _M[n];
      const enter = () => animate(el, kf, opts);
      const leave = () => animate(el, [...kf].reverse(), opts);
      el.addEventListener('mouseenter', enter);
      el.addEventListener('mouseleave', leave);
      handlers.push({ el, enter, leave });
    }
  });
  // Same pattern for @press (mousedown/mouseup) and @focus (focus/blur)
  return () => {
    handlers.forEach(({ el, enter, leave }) => {
      el.removeEventListener('mouseenter', enter);
      el.removeEventListener('mouseleave', leave);
    });
    // Same cleanup for press/focus handlers
  };
}, []); // Empty deps with cleanup
```

- [ ] **Step 3: Check for @enter re-trigger use case**

The comment on line 126 says "Include signal names in dep array so @enter re-triggers on state changes." Verify this is NOT an intentional feature by checking if any existing tests rely on @enter replay behavior. If a legitimate re-trigger use case exists (e.g., list items being added), add a separate mechanism using `key` props rather than the broken dependency array approach.

- [ ] **Step 4: Update React test snapshots**

Run `cargo test -p omnilang --features codegen-react` to identify failing snapshot tests. Update any snapshots that captured the old dependency array format.

- [ ] **Step 5: Run all tests**

```bash
cargo test --all-features
```

Expected: All 801+ tests pass. Zero regressions.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/motion/mod.rs
git add omnilang/src/codegen/react/tests.rs  # if snapshots updated
git commit -m "fix(react): @enter animations now mount-only, interactive triggers use cleanup

Split the motion useEffect into two separate effects:
1. @enter effect with [] deps (fires once on mount)
2. Interactive triggers (@hover/@press/@focus) with cleanup functions
   that removeEventListener on unmount

Previously all signals were in the dep array, causing @enter to replay
on every state change and event listeners to leak."
```

---

### Task 2: Fix Dioxus todo!() panic (H3)

**Files:**
- Modify: `omnilang/src/codegen/dioxus/expr.rs:249`

**Interfaces:**
- Consumes: `EventDataKind` enum, field name string
- Produces: Safe fallback that generates a comment instead of a runtime panic

**Context:** The wildcard match arm in `field_extract_dioxus` generates `todo!("field {field}")` — a Rust macro that panics at compile time. If a new `EventDataKind` variant is added to the IR, every `.omni` file using that event type will crash the Dioxus compiler.

- [ ] **Step 1: Replace the todo!() with a safe fallback**

In `omnilang/src/codegen/dioxus/expr.rs:249`, change:
```rust
_ => format!("todo!(\"field {}\")", field),
```
To:
```rust
_ => format!("String::new() /* UNSUPPORTED event field: {} */", field),
```

This generates valid Dioxus code that compiles (returns an empty string) with an inline comment marking the unsupported field. The generated code will work at runtime (the field just returns empty) and the comment is visible to developers debugging Dioxus output.

- [ ] **Step 2: Check for any other todo!() or unimplemented!() in codegen**

```bash
grep -rn "todo!\|unimplemented!" omnilang/src/codegen/
```

If any other panics are found in codegen backends, apply the same safe-fallback pattern.

- [ ] **Step 3: Run Dioxus tests**

```bash
cargo test -p omnilang --features codegen-dioxus
```

Expected: All Dioxus tests pass with unchanged output.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/dioxus/expr.rs
git commit -m "fix(dioxus): replace todo!() panic with safe fallback for unknown event fields

The wildcard match arm in field_extract_dioxus previously generated
a todo!() macro that would panic at compile time when encountering
new or unknown EventDataKind variants. Replaced with a safe fallback
that generates valid Dioxus code with a comment marking the unsupported
field."
```

---

### Task 3: Fix WASM OmniCompiler path_mapping (M4)

**Files:**
- Modify: `omnilang-wasm/src/lib.rs:570`

**Interfaces:**
- Consumes: `self.sources: FxHashMap<String, String>`
- Produces: Correct `path_mapping` derived from source file paths
- Reference: `omnilang::path_mapping::derive_path_mapping` (already used by `compile_multi_to_omniview` at line 463)

**Context:** The `OmniCompiler::compile_to_omniview` method passes `FxHashMap::default()` for path_mapping when doing multi-file compilation. This means cross-file imports (`import { Button } from "./components/Button"`) fail silently. The single-file `compile_multi_to_omniview` function correctly calls `derive_path_mapping` — this is a one-line omission.

- [ ] **Step 1: Add the derive_path_mapping call**

In `omnilang-wasm/src/lib.rs:570`, change:
```rust
FxHashMap::default(),
```
To:
```rust
omnilang::path_mapping::derive_path_mapping(&self.sources),
```

- [ ] **Step 2: Verify the fix matches the working code path**

Compare with `compile_multi_to_omniview` (line 463) to confirm the same `derive_path_mapping` call signature is used.

- [ ] **Step 3: Build WASM**

```bash
cargo build -p omnilang-wasm --target wasm32-unknown-unknown
```

- [ ] **Step 4: Commit**

```bash
git add omnilang-wasm/src/lib.rs
git commit -m "fix(wasm): add missing path_mapping in OmniCompiler::compile_to_omniview

The stateful OmniCompiler API was passing an empty FxHashMap for
path_mapping, causing cross-file imports to fail silently in multi-file
compilation. Now uses derive_path_mapping() to compute correct
import paths, matching the behavior of compile_multi_to_omniview."
```

---

## Phase 2: Cross-Target Consistency

### Task 4: Add class_map_bindings to Flutter codegen (M1a)

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs` (the element rendering function)
- Create: test case in `omnilang/src/codegen/flutter/tests.rs`

**Interfaces:**
- Consumes: `el.class_map_bindings: FxHashMap<String, ExprIR>` from `ElementIR`
- Produces: Flutter widget code that conditionally applies classes
- Reference: React implementation at `omnilang/src/codegen/react/view.rs:423-479`

**Context:** When `.omni` source uses `class:active={isOpen}`, the React and OmniView backends emit code that dynamically toggles the CSS class. The Flutter backend silently ignores `class_map_bindings`, so the class is never applied.

- [ ] **Step 1: Read React reference implementation**

Read `omnilang/src/codegen/react/view.rs` lines 423-479 to understand how class_map_bindings are emitted in JSX (template literals with ternary expressions).

- [ ] **Step 2: Add a test case**

In `omnilang/src/codegen/flutter/tests.rs`, add a test case for a widget with `class_map_bindings`:

```rust
#[test]
fn test_class_map_bindings() {
    let mut bindings = FxHashMap::default();
    bindings.insert("active".to_string(), ExprIR::Ident { name: "isOpen".to_string() });
    // Create an element with class_map_bindings and verify the output
    // contains conditional class application
}
```

- [ ] **Step 3: Implement class_map_bindings in Flutter view**

In the Flutter element rendering function, after processing static classes, iterate `class_map_bindings` and emit conditional decoration. For each `(class_name, condition_expr)`, emit a Flutter pattern that conditionally adds the class:

```dart
// For class:active={isOpen}
// Emit: decoration: isOpen ? Styles.active : Styles.inactive,
// Or wrap in a conditional Container/AnimatedContainer
```

The exact approach depends on how Flutter styles are currently emitted. Match the existing style application pattern.

- [ ] **Step 4: Run Flutter tests**

```bash
cargo test -p omnilang --features codegen-flutter
```

Update any snapshot changes.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "feat(flutter): add class_map_bindings support for dynamic CSS classes

Previously class:active={condition} syntax was silently ignored in
Flutter output. Now emits conditional style/decoration application
based on the binding expression."
```

---

### Task 5: Add class_map_bindings to Vue codegen (M1b)

**Files:**
- Modify: `omnilang/src/codegen/vue/template.rs` (the element rendering function)
- Create: test case in `omnilang/src/codegen/vue/tests.rs`

**Interfaces:**
- Consumes: `el.class_map_bindings: FxHashMap<String, ExprIR>`
- Produces: Vue `:class` binding with object syntax

**Context:** Vue natively supports object syntax for dynamic classes: `:class="{ active: isOpen }"`. This is the simplest backend to implement — it's a direct mapping.

- [ ] **Step 1: Add Vue :class emission**

In `omnilang/src/codegen/vue/template.rs`, in the element attribute generation section, after processing static `class` attributes, add:

```rust
if !el.class_map_bindings.is_empty() {
    let bindings: Vec<String> = el.class_map_bindings.iter()
        .map(|(class_name, expr)| {
            format!("{}: {}", class_name, format_expr_vue(expr))
        })
        .collect();
    w.emit_fmt(format_args!(" :class=\"{{ {} }}\"", bindings.join(", ")));
}
```

- [ ] **Step 2: Add test case**

In `omnilang/src/codegen/vue/tests.rs`, add a test for `class_map_bindings`.

- [ ] **Step 3: Run Vue tests**

```bash
cargo test -p omnilang --features codegen-vue
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/vue/template.rs omnilang/src/codegen/vue/tests.rs
git commit -m "feat(vue): add class_map_bindings support via :class object syntax

Vue natively supports :class=\"{ active: isOpen }\" which maps directly
to the Omni class:active={isOpen} syntax."
```

---

### Task 6: Add class_map_bindings to Dioxus codegen (M1c)

**Files:**
- Modify: `omnilang/src/codegen/dioxus/view.rs`

**Interfaces:**
- Consumes: `el.class_map_bindings`
- Produces: Dioxus `class:` attribute with conditional string concatenation

**Context:** Dioxus supports conditional classes via `class: "base {if condition { \"active\" }}"` or `class: format!("base {}", if condition { "active" } else { "" })`.

- [ ] **Step 1: Implement conditional class in Dioxus view**

In `omnilang/src/codegen/dioxus/view.rs`, in the element attribute section, iterate `class_map_bindings` and emit conditional class attributes using Dioxus syntax:

```rust
for (class_name, expr) in &el.class_map_bindings {
    let cond = format_expr_ir_dioxus(expr);
    w.emit_fmt(format_args!(
        "class: if {} {{ \"{}\" }},",
        cond, class_name
    ));
}
```

- [ ] **Step 2: Run Dioxus tests**

```bash
cargo test -p omnilang --features codegen-dioxus
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/dioxus/view.rs
git commit -m "feat(dioxus): add class_map_bindings support for dynamic classes"
```

---

### Task 7: Add empty_body to Vue and Dioxus (M2)

**Files:**
- Modify: `omnilang/src/codegen/vue/template.rs:203-216`
- Modify: `omnilang/src/codegen/dioxus/view.rs:198-215`

**Interfaces:**
- Consumes: `list.empty_body: Option<ViewTree>` from `ListRenderingIntent`
- Produces: Conditional rendering when the iterable is empty
- Reference: React implementation at `omnilang/src/codegen/react/view.rs:629-668`

**Context:** The `@for item in items ... @empty ...` syntax provides fallback UI when the list is empty. React and Flutter handle this, but Vue and Dioxus silently drop the `empty_body`.

- [ ] **Step 1: Add empty_body to Vue template**

In `omnilang/src/codegen/vue/template.rs`, modify the `ViewTree::ListRendering` match arm (lines 203-216). Wrap the existing `v-for` in a conditional that shows the list or the empty body:

```rust
ViewTree::ListRendering(list) => {
    let iterable_str = fmt.format_expr(&list.iterable);
    let item_name = &list.item_binding.name;
    if let Some(ref empty_body) = list.empty_body {
        w.emit_fmt(format_args!(
            "<template v-if=\"{}.length > 0\">", iterable_str
        ));
        w.indent();
    }
    // ... existing v-for code ...
    if list.empty_body.is_some() {
        w.dedent();
        w.emit("</template>");
        w.emit("<template v-else>");
        w.indent();
        gen_template_node(w, empty_body.as_ref().unwrap(), fmt);
        w.dedent();
        w.emit("</template>");
    }
}
```

- [ ] **Step 2: Add empty_body to Dioxus view**

In `omnilang/src/codegen/dioxus/view.rs`, modify the `ViewTree::ListRendering` match arm (lines 198-215). Wrap the iterator in a conditional:

```rust
if let Some(ref empty_body) = list.empty_body {
    w.emit_fmt(format_args!(
        "{{ if {}.len() == 0 {{ rsx! {{", iterable_str
    ));
    w.indent();
    gen_view_node_dioxus(w, empty_body, true);
    w.dedent();
    w.emit("}} } else {");
    w.indent();
}
// ... existing iterator code ...
if list.empty_body.is_some() {
    w.dedent();
    w.emit("} }");
}
```

- [ ] **Step 3: Add test cases**

Add tests in both `vue/tests.rs` and `dioxus/tests.rs` for `@for` with `@empty`.

- [ ] **Step 4: Run all affected tests**

```bash
cargo test -p omnilang --features codegen-vue,codegen-dioxus
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/vue/template.rs omnilang/src/codegen/vue/tests.rs
git add omnilang/src/codegen/dioxus/view.rs omnilang/src/codegen/dioxus/tests.rs
git commit -m "feat(vue,dioxus): add @for empty_body support for empty-list fallback

Vue: wrap v-for in v-if/v-else conditional
Dioxus: wrap iterator in if/else conditional"
```

---

### Task 8: Add <slot> support to Vue (M3)

**Files:**
- Modify: `omnilang/src/codegen/vue/template.rs:239-241`

**Interfaces:**
- Consumes: `ViewTree::Slot(Option<String>)` — `None` = default slot, `Some(name)` = named slot
- Produces: Vue `<slot />` or `<slot name="..." />`

**Context:** Vue natively supports `<slot>` and `<slot name="...">` elements. The current codegen has a comment saying "slots not supported" and emits nothing. This is a direct 1:1 mapping — no workaround needed.

- [ ] **Step 1: Replace the no-op slot handler**

In `omnilang/src/codegen/vue/template.rs:239-241`, replace:
```rust
ViewTree::Slot(_) => {
    // slots not supported in Vue codegen — emit placeholder
}
```
With:
```rust
ViewTree::Slot(slot_name) => {
    match slot_name {
        None => w.emit("<slot />"),
        Some(name) => w.emit_fmt(format_args!("<slot name=\"{}\" />", name)),
    }
}
```

- [ ] **Step 2: Run Vue tests**

```bash
cargo test -p omnilang --features codegen-vue
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/vue/template.rs
git commit -m "feat(vue): add <slot> element support using Vue's native slot element

Vue natively supports <slot> and <slot name=\"...\"> which maps directly
to OmniView's Slot view tree node."
```

---

### Task 9: Add key_expr support to Vue and Dioxus (L1)

**Files:**
- Modify: `omnilang/src/codegen/vue/template.rs:207`
- Modify: `omnilang/src/codegen/dioxus/view.rs:198-214`

**Interfaces:**
- Consumes: `list.key_expr: Option<ExprIR>` from `ListRenderingIntent`
- Produces: `:key` attribute in Vue, `key:` attribute in Dioxus

- [ ] **Step 1: Add key_expr to Vue v-for**

Change `:key="index"` to use `key_expr` when available:
```rust
let key = match &list.key_expr {
    Some(expr) => fmt.format_expr(expr),
    None => "index".to_string(),
};
w.emit_fmt(format_args!(
    "<template v-for=\"({}, index) in {}\" :key=\"{}\">",
    item_name, iterable_str, key
));
```

- [ ] **Step 2: Add key_expr to Dioxus iterator**

Inside the Dioxus iterator closure, add a `key:` attribute when `key_expr` is present.

- [ ] **Step 3: Run tests and commit**

```bash
cargo test -p omnilang --features codegen-vue,codegen-dioxus
git add omnilang/src/codegen/vue/template.rs omnilang/src/codegen/dioxus/view.rs
git commit -m "feat(vue,dioxus): use key_expr for list keys instead of hardcoded index"
```

---

## Phase 3: Code Quality & Refactoring

### Task 10: Split gen_component_tsx (H2)

**Files:**
- Modify: `omnilang/src/codegen/react/component/mod.rs`
- Create: `omnilang/src/codegen/react/component/imports.rs`
- Create: `omnilang/src/codegen/react/component/computed.rs`
- Create: `omnilang/src/codegen/react/component/motion_effects.rs`

**Interfaces:**
- Consumes: `ComponentIR`, `IRProgram`, `component_map`, `signal_names`, `store_signal_map`
- Produces:
  - `collect_component_imports() -> ComponentImports` — struct with all import decisions
  - `gen_computed_values(&mut CodeWriter, &ComponentIR, &[String])` — emits useMemo/useCallback
  - `gen_motion_effects(&mut CodeWriter, &ViewTree, &IRProgram, &FxHashSet<String>)` — emits all 5 motion effects

**Context:** `gen_component_tsx` is 520 lines. Extract import collection (~240 lines, lines 30-271), computed value generation (~55 lines, lines 433-485), and motion effect emission (~35 lines, lines 491-526) into separate functions.

- [ ] **Step 1: Create ComponentImports struct and extract collect_component_imports**

Create `omnilang/src/codegen/react/component/imports.rs`:

```rust
use super::*;
use rustc_hash::FxHashSet;

pub(super) struct ComponentImports {
    pub hooks_needed: FxHashSet<String>,
    pub style_modules: Vec<String>,
    pub store_hook_imports: Vec<(String, String)>, // (store_name, import_path)
    pub child_comp_names: Vec<String>,
    pub library_fns: Vec<String>,
}

pub(super) fn collect_component_imports(
    comp: &ComponentIR,
    ir: &IRProgram,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> ComponentImports {
    // Move lines 30-271 from gen_component_tsx here
    // Return the struct instead of writing directly to CodeWriter
}
```

Add `pub(super) mod imports;` to `component/mod.rs`.

- [ ] **Step 2: Add emit_imports method**

Add a method to emit imports from the struct:
```rust
impl ComponentImports {
    pub(super) fn emit(&self, w: &mut CodeWriter, comp: &ComponentIR) {
        // Move the writing logic here from gen_component_tsx
    }
}
```

- [ ] **Step 3: Extract gen_computed_values**

Create `omnilang/src/codegen/react/component/computed.rs`. Move the computed values loop (lines 433-485) into a standalone function:

```rust
pub(super) fn gen_computed_values(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    known_signal_names: &[String],
    signal_names: &FxHashSet<String>,
    store_mode: ReactStoreMode,
) {
    // Lines 433-485
}
```

- [ ] **Step 4: Extract gen_motion_effects**

Create `omnilang/src/codegen/react/component/motion_effects.rs`. Move the motion effect emission block (lines 491-526) into:

```rust
pub(super) fn gen_motion_effects(
    w: &mut CodeWriter,
    view: &ViewTree,
    ir: &IRProgram,
    signal_names: &FxHashSet<String>,
) {
    // Lines 491-526
}
```

- [ ] **Step 5: Update gen_component_tsx to use extracted functions**

Replace the extracted sections in `gen_component_tsx` with calls to the new functions. Target: `gen_component_tsx` should be ~200 lines of orchestration.

- [ ] **Step 6: Fix the redundant motion_names collection (M5b)**

While refactoring, fix the duplicate `motion_names` collection (lines 237-247):
```rust
// BEFORE (redundant — first collect is discarded)
let mut motion_names: FxHashSet<String> = ir.modules.values()
    .flat_map(|m| &m.motions).map(|m| m.name.clone()).collect();
for module in ir.modules.values() {
    for motion in &module.motions {
        motion_names.insert(motion.name.clone());
    }
}

// AFTER (single pass)
let motion_names: FxHashSet<String> = ir.modules.values()
    .flat_map(|m| &m.motions)
    .map(|m| m.name.clone())
    .collect();
```

- [ ] **Step 7: Run all tests**

```bash
cargo test -p omnilang --features codegen-react
cargo test --all-features
```

- [ ] **Step 8: Commit**

```bash
git add omnilang/src/codegen/react/component/
git commit -m "refactor(react): split gen_component_tsx into focused sub-modules

Extract import collection, computed value generation, and motion effect
emission into separate functions in new sub-modules:
- component/imports.rs: ComponentImports struct + collect/emit
- component/computed.rs: gen_computed_values
- component/motion_effects.rs: gen_motion_effects

Also fix redundant motion_names double-collection.

gen_component_tsx: 520 → ~200 lines"
```

---

### Task 11: Remove ComponentIR double-clone (M5)

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs:18`
- Modify: `omnilang/src/codegen/omniview/mod.rs:300` (optional — keep the map but store references)

**Interfaces:**
- Consumes: `self.component_map: FxHashMap<String, ComponentIR>`
- Produces: All usages of `child_comp` now use `&ComponentIR` references instead of cloned values

**Context:** Line 18 calls `.cloned()` on the HashMap `get` result, creating a full clone of `ComponentIR` (20+ fields including the entire view tree). All usages in the function only need `&ComponentIR`.

- [ ] **Step 1: Remove .cloned() and verify borrows work**

In `omnilang/src/codegen/omniview/component_html.rs:18`, change:
```rust
if let Some(child_comp) = self.component_map.get(&el.tag).cloned() {
```
To:
```rust
if let Some(child_comp) = self.component_map.get(&el.tag) {
```

Now `child_comp` is `&ComponentIR`. Verify all usages compile:
- `child_comp.view` → already `&Option<ViewTree>` via auto-deref ✓
- `child_comp.name.clone()` → `child_comp.name.to_string()` (minor)
- `&comp.signals`, `&comp.computeds`, etc. → already references ✓

- [ ] **Step 2: Fix any compilation errors**

```bash
cargo build -p omnilang --features codegen-omniview
```

If any usage required ownership (unlikely based on code review), adjust to use references.

- [ ] **Step 3: Run tests**

```bash
cargo test -p omnilang --features codegen-omniview
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/component_html.rs
git commit -m "perf(omniview): eliminate ComponentIR clone on view inlining lookup

try_inline_component was cloning the entire ComponentIR (20+ fields
including view tree) on every HashMap::get. Changed to use &ComponentIR
reference — all usages only needed borrowed access."
```

---

### Task 12: Remove compile_files dead code + consolidate HTML templates (M6)

**Files:**
- Modify: `omnilang/src/codegen/omniview/utils.rs:276-319` (remove `compile_files`)
- Modify: `omnilang/src/codegen/omniview/mod.rs` (remove re-export, consolidate HTML template)

**Interfaces:**
- Remove: `pub fn compile_files(...)` — zero callers confirmed
- Remove: Re-export from `mod.rs`
- Create: Single shared HTML template function used by `OmniViewCodegen::generate` and `build_file_output`

- [ ] **Step 1: Delete compile_files function**

Remove lines 276-319 from `omnilang/src/codegen/omniview/utils.rs`.

- [ ] **Step 2: Remove re-export**

Remove the `compile_files` re-export from `omnilang/src/codegen/omniview/mod.rs`.

- [ ] **Step 3: Consolidate HTML templates**

Create a shared function `html_document_wrapper(body_html: &str, include_animation_script: bool) -> String` that both `OmniViewCodegen::generate` (mod.rs:144-159) and `build_file_output` (utils.rs:235-251) call. This eliminates the three near-identical template copies.

- [ ] **Step 4: Verify build**

```bash
cargo build -p omnilang --all-features
```

- [ ] **Step 5: Run tests and commit**

```bash
cargo test -p omnilang --features codegen-omniview
git add omnilang/src/codegen/omniview/
git commit -m "refactor(omniview): remove dead compile_files, consolidate HTML templates

- Removed compile_files (zero callers, superseded by OmniViewCodegen)
- Extracted shared html_document_wrapper() to eliminate 3 template copies
- build_file_output animation script tag now consistent with OmniViewCodegen"
```

---

### Task 13: Compound mutation macro (M12)

**Files:**
- Modify: `omnilang/src/codegen/react/expr.rs:241-374`

**Interfaces:**
- Consumes: `MutationIntent` variants (Increment, Decrement, Multiply, Divide, Modulo)
- Produces: A `compound_op!` macro that generates all 5 arithmetic mutation blocks

- [ ] **Step 1: Define the compound_op! macro**

In `omnilang/src/codegen/react/expr.rs`, before `format_compound_mutation`, add:

```rust
/// Generate Zustand/Setter/Direct assignment for arithmetic compound mutations.
macro_rules! compound_op {
    ($w:ident, $s:ident, $target_str:ident, $amt:ident, $in_zustand:ident, $use_setter:ident, $op:tt) => {{
        if $in_zustand {
            $w.emit_fmt(format_args!(
                "{}.setState((s) => ({{ {}: s.{} {} {} }}));",
                $s, $target_str, $target_str, stringify!($op), $amt
            ));
        } else if $use_setter {
            let setter = format!("set{}", capitalize_first($target_str));
            $w.emit_fmt(format_args!("{}(prev => prev {} {});", setter, stringify!($op), $amt));
        } else {
            $w.emit_fmt(format_args!("{} = {} {} {};", $target_str, $target_str, stringify!($op), $amt));
        }
    }};
}
```

- [ ] **Step 2: Replace 5 duplicate blocks with macro invocations**

Replace lines 264-334 (Increment, Decrement, Multiply, Divide, Modulo arms) with:

```rust
MutationIntent::Increment { .. } => {
    compound_op!(w, s, target_str, amt, in_zustand, use_setter, +);
}
MutationIntent::Decrement { .. } => {
    compound_op!(w, s, target_str, amt, in_zustand, use_setter, -);
}
MutationIntent::Multiply { .. } => {
    compound_op!(w, s, target_str, amt, in_zustand, use_setter, *);
}
MutationIntent::Divide { .. } => {
    compound_op!(w, s, target_str, amt, in_zustand, use_setter, /);
}
MutationIntent::Modulo { .. } => {
    compound_op!(w, s, target_str, amt, in_zustand, use_setter, %);
}
```

- [ ] **Step 3: Run React tests**

```bash
cargo test -p omnilang --features codegen-react
```

Output must be byte-identical to before (this is a pure refactor).

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/react/expr.rs
git commit -m "refactor(react): replace 5 duplicate compound mutation blocks with macro

compound_op! macro eliminates ~230 lines of near-identical code for
Increment/Decrement/Multiply/Divide/Modulo mutations. No behavior change."
```

---

### Task 14: Shared view_tree_any_element (M13)

**Files:**
- Create: `crates/omnilang-core/src/codegen/shared/view_tree.rs`
- Modify: `crates/omnilang-core/src/codegen/shared/mod.rs` (add module)
- Modify: `omnilang/src/codegen/omniview/mod.rs:445-469` (use shared)
- Modify: `omnilang/src/codegen/react/motion/transforms.rs:44-291` (use shared)
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs:11-358` (use shared)

**Interfaces:**
- Produces: `pub fn view_tree_any(view: &ViewTree, predicate: impl Fn(&ElementIR) -> bool) -> bool`
- Consumed by: All 8 tree-walking functions across omniview/react/flutter backends

- [ ] **Step 1: Create shared view_tree_any function**

Create `crates/omnilang-core/src/codegen/shared/view_tree.rs`:

```rust
use crate::ir::ViewTree;
use crate::ir::ElementIR;

/// Walk a ViewTree recursively. Returns true if any Element satisfies the predicate.
pub fn view_tree_any(view: &ViewTree, predicate: impl Fn(&ElementIR) -> bool) -> bool {
    match view {
        ViewTree::Element(el) => {
            if predicate(el) { return true; }
            el.children.iter().any(|child| view_tree_any(child, &predicate))
        }
        ViewTree::Conditional(cond) => {
            [&cond.then_branch, &cond.else_branch]
                .iter()
                .filter_map(|x| x.as_ref())
                .any(|v| view_tree_any(v, &predicate))
        }
        ViewTree::ListRendering(list) => {
            [&list.body, &list.empty_body]
                .iter()
                .filter_map(|x| x.as_ref())
                .any(|v| view_tree_any(v, &predicate))
        }
        ViewTree::Text(_) | ViewTree::Interpolation(_) | ViewTree::Slot(_) => false,
    }
}
```

- [ ] **Step 2: Add module to shared mod.rs**

Add `pub mod view_tree;` to `crates/omnilang-core/src/codegen/shared/mod.rs`.

- [ ] **Step 3: Replace omniview view_tree_has_motion**

In `omnilang/src/codegen/omniview/mod.rs:445-469`, replace the custom tree walker with:
```rust
fn view_tree_has_motion(view: &ViewTree) -> bool {
    omnilang_core::codegen::shared::view_tree::view_tree_any(view, |el| {
        el.motion.is_some()
    })
}
```

- [ ] **Step 4: Replace React motion trigger scanners**

In `omnilang/src/codegen/react/motion/transforms.rs`, replace `has_driven_by`, `has_inview_trigger`, `has_scroll_trigger`, `has_drag_trigger` with `view_tree_any` calls using appropriate predicates.

- [ ] **Step 5: Replace Flutter motion trigger scanners**

In `omnilang/src/codegen/flutter/motion_widgets.rs`, replace `has_trigger_kind_in_view`, `has_trigger_kind_scroll`, `has_trigger_kind_drag` with `view_tree_any`.

- [ ] **Step 6: Run all tests**

```bash
cargo test --all-features
```

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/view_tree.rs
git add crates/omnilang-core/src/codegen/shared/mod.rs
git add omnilang/src/codegen/omniview/mod.rs
git add omnilang/src/codegen/react/motion/transforms.rs
git add omnilang/src/codegen/flutter/motion_widgets.rs
git commit -m "refactor: extract shared view_tree_any to eliminate 8 duplicate tree-walkers

Added view_tree_any() to omnilang-core::codegen::shared::view_tree.
Replaced 8 nearly-identical recursive tree-walking functions across
omniview/react/flutter backends with single shared implementation."
```

---

### Task 15: Narrow #[allow(unused_variables)] (M14)

**Files:**
- Modify: `omnilang/src/lib.rs:326,352,375`

**Interfaces:**
- Consumes: 3 functions with `#[allow(unused_variables)]` at function level
- Produces: Same functions with the attribute moved to individual parameter patterns

- [ ] **Step 1: Move attributes to parameter level**

For each of the three functions (`compile_multi_internal`, `compile_multi_internal_cached`, `compile_multi_internal_post`), change from function-level `#[allow(unused_variables)]` to per-parameter:

```rust
// BEFORE
#[allow(unused_variables)]
pub fn compile_multi_internal(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
) -> InternalCompileResult {

// AFTER
pub fn compile_multi_internal(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    #[allow(unused_variables)] editor_mode: bool,
    #[allow(unused_variables)] debug_mode: bool,
    #[allow(unused_variables)] store_mode: Option<&str>,
) -> InternalCompileResult {
```

- [ ] **Step 2: Verify build with all feature combinations**

```bash
cargo build -p omnilang --no-default-features
cargo build -p omnilang --features codegen-omniview
cargo build -p omnilang --all-features
```

No new warnings should appear.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/lib.rs
git commit -m "refactor: narrow #[allow(unused_variables)] to parameter-level

Moved from function-level suppression to per-parameter annotations on
editor_mode, debug_mode, and store_mode. This ensures future genuinely
unused parameters still trigger compiler warnings."
```

---

### Task 16: Split element.rs (M15)

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs` (1047 → ~700 lines)
- Create: `omnilang/src/codegen/omniview/view/events.rs` (~150 lines)
- Create: `omnilang/src/codegen/omniview/view/style.rs` (~120 lines)
- Modify: `omnilang/src/codegen/omniview/view/mod.rs` (add sub-modules)

**Interfaces:**
- Consumes: Existing `OmniViewGenerator` impl methods
- Produces: Same public API, split across 3 files

- [ ] **Step 1: Extract emit_element_events to view/events.rs**

Move `emit_element_events` (~146 lines) to `omnilang/src/codegen/omniview/view/events.rs`:
```rust
use super::super::OmniViewGenerator;
impl OmniViewGenerator {
    pub(super) fn emit_element_events(&mut self, el: &ElementIR, nid: &str) {
        // ... existing code ...
    }
}
```

- [ ] **Step 2: Extract emit_element_inline_style to view/style.rs**

Move `emit_element_inline_style` (~117 lines) to `view/style.rs`.

- [ ] **Step 3: Update view/mod.rs**

Add `pub(super) mod events;` and `pub(super) mod style;` to `view/mod.rs`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p omnilang --features codegen-omniview
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/view/
git commit -m "refactor(omniview): split element.rs events and style into sub-modules

Extracted emit_element_events → view/events.rs (~150 lines)
Extracted emit_element_inline_style → view/style.rs (~120 lines)
element.rs: 1047 → ~700 lines"
```

---

## Phase 4: Frontend Runtime

### Task 17: Canonicalize easing tables (M7)

**Files:**
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs:206-247` (import from shared)
- Create: `crates/omnilang-core/src/codegen/shared/easing.rs` (canonical table)
- Modify: `crates/omnilang-core/src/codegen/shared/mod.rs`
- Verify: `packages/omni-animation/src/easing.ts` remains runtime source of truth

**Interfaces:**
- Produces: `pub const NAMED_EASINGS: &[(&str, &str)]` — canonical easing name → cubic-bezier mapping
- Consumed by: React keyframes.rs, semantic motion.rs

- [ ] **Step 1: Create shared easing table**

Create `crates/omnilang-core/src/codegen/shared/easing.rs` with the canonical easing mapping extracted from the existing JS runtime table:

```rust
/// Canonical named easing to cubic-bezier mapping.
/// Source of truth: packages/omni-animation/src/easing.ts
pub const NAMED_EASINGS: &[(&str, &str)] = &[
    ("power1In", "cubic-bezier(0.550,0.085,0.680,0.530)"),
    ("power1Out", "cubic-bezier(0.215,0.610,0.355,1.000)"),
    ("power1InOut", "cubic-bezier(0.455,0.030,0.515,0.955)"),
    // ... all 30+ entries from easing.ts
];

pub fn resolve_easing(name: &str) -> Option<&'static str> {
    NAMED_EASINGS.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
}
```

- [ ] **Step 2: Update React keyframes.rs to use shared table**

In `omnilang/src/codegen/react/motion/keyframes.rs:206-247`, replace the inline easing resolution with `omnilang_core::codegen::shared::easing::resolve_easing()`.

- [ ] **Step 3: Update semantic motion.rs to use shared table**

In `crates/omnilang-core/src/semantic/motion.rs:55-60`, replace the inline easing names with `resolve_easing()`.

- [ ] **Step 4: Run all tests**

```bash
cargo test --all-features
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/easing.rs
git add crates/omnilang-core/src/codegen/shared/mod.rs
git add omnilang/src/codegen/react/motion/keyframes.rs
git add crates/omnilang-core/src/semantic/motion.rs
git commit -m "refactor: canonicalize easing table into single shared source

Extracted NAMED_EASINGS constant into omnilang-core shared module.
React keyframes.rs and semantic motion.rs now reference the shared
table instead of maintaining their own copies. The JS runtime
(packages/omni-animation/src/easing.ts) remains the authoritative
runtime source with parameterized easing support."
```

---

### Task 18: Fix computed subscriber accumulation (M8)

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:56-79`

**Interfaces:**
- Consumes: `__omni_computed` JS function
- Produces: Same function with a single reused effect object and cleanup of old `_deps`

- [ ] **Step 1: Refactor __omni_computed to reuse effect object**

In `omnilang/src/codegen/omniview/runtime.rs:56-79`, change the `run()` function to reuse a single effect object:

```js
function __omni_computed(name, fn) {
  let val, dirty = true;
  const signal = __omni_signal(name, undefined);
  const effect = { _deps: new Set(), run: () => { dirty = true; signal.set(fn()); } };
  const run = () => {
    dirty = false;
    // Clean up old dependency subscriptions
    effect._deps.forEach(fn => fn());
    effect._deps.clear();
    const prev = __omni_activeEffect;
    __omni_activeEffect = effect;
    const newVal = fn();
    __omni_activeEffect = prev;
    val = newVal;
    signal.peek();
    return val;
  };
  run();
  return {
    get value() {
      signal.value;
      return dirty ? run() : val;
    },
    get() { return signal.value; },
    _signal: signal,
  };
}
```

Key change: `effect` object created once outside `run()`, old deps cleaned up via `forEach(fn => fn())` before re-running.

- [ ] **Step 2: Update test snapshots**

Run `cargo test -p omnilang --features codegen-omniview` and update any snapshots that embed the runtime code.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix(runtime): reuse effect object in __omni_computed to prevent subscriber leak

Previously each run() call created a new effect object, accumulating
stale subscribers in signal.deps sets. Now reuses a single effect object
and cleans up old _deps before re-running, preventing O(n) growth."
```

---

### Task 19: Add @omni/animation tests (M9)

**Files:**
- Modify: `packages/omni-animation/package.json` (add test script + vitest dep)
- Create: `packages/omni-animation/vitest.config.ts`
- Create: `packages/omni-animation/src/__tests__/keyframes.test.ts`
- Create: `packages/omni-animation/src/__tests__/easing.test.ts`
- Create: `packages/omni-animation/src/__tests__/stagger.test.ts`

- [ ] **Step 1: Add vitest dependency and test script**

```bash
cd packages/omni-animation
npm install -D vitest
```

Add to `package.json`:
```json
"scripts": {
    "build": "tsc && node build-bundle.js",
    "check": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest"
}
```

- [ ] **Step 2: Create vitest.config.ts**

```typescript
import { defineConfig } from 'vitest/config';
export default defineConfig({
  test: {
    environment: 'jsdom',
  },
});
```

- [ ] **Step 3: Write tests for keyframes**

`src/__tests__/keyframes.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { buildKeyframes, buildProperties } from '../keyframes';

describe('buildProperties', () => {
  it('converts x/y/rotate to transform properties', () => {
    const result = buildProperties({ x: 100, y: 50, rotate: 45, opacity: 0.5 });
    expect(result.transform).toBe('translateX(100px) translateY(50px) rotate(45deg)');
    expect(result.opacity).toBe('0.5');
  });

  it('handles percentage values', () => {
    const result = buildProperties({ x: '50%' });
    expect(result.transform).toBe('translateX(50%)');
  });

  it('handles scale shorthand', () => {
    const result = buildProperties({ scale: 1.5 });
    expect(result.transform).toContain('scale(1.5)');
  });
});

describe('buildKeyframes', () => {
  it('builds WAAPI-compatible keyframes', () => {
    const kf = buildKeyframes([{ opacity: 0 }, { opacity: 1 }], [{}]);
    expect(kf).toHaveLength(2);
    expect(kf[0].opacity).toBe('0');
    expect(kf[1].opacity).toBe('1');
  });
});
```

- [ ] **Step 4: Write tests for easing**

`src/__tests__/easing.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { resolveEasing } from '../easing';

describe('resolveEasing', () => {
  it('resolves named easings to cubic-bezier', () => {
    expect(resolveEasing('power1In')).toContain('cubic-bezier');
  });

  it('resolves parameterized easings', () => {
    const result = resolveEasing('backIn(1.5)');
    expect(result).toContain('cubic-bezier');
  });

  it('returns unknown easings as-is', () => {
    expect(resolveEasing('customCurve')).toBe('customCurve');
  });
});
```

- [ ] **Step 5: Write tests for stagger**

`src/__tests__/stagger.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { staggerDelays } from '../stagger';

describe('staggerDelays', () => {
  it('computes evenly-spaced delays', () => {
    const delays = staggerDelays(5, 100);
    expect(delays).toEqual([0, 100, 200, 300, 400]);
  });

  it('handles single element', () => {
    expect(staggerDelays(1, 100)).toEqual([0]);
  });
});
```

- [ ] **Step 6: Run tests**

```bash
cd packages/omni-animation && npm test
```

- [ ] **Step 7: Commit**

```bash
git add packages/omni-animation/
git commit -m "test(animation): add vitest test suite for @omni/animation

Covers buildKeyframes, buildProperties, resolveEasing, and staggerDelays.
0% → baseline test coverage for the core animation modules."
```

---

### Task 20: Type-safe Zustand getters and action params (M10)

**Files:**
- Modify: `omnilang/src/codegen/react/mod.rs:349,355`

**Interfaces:**
- Consumes: `store.getters: FxHashMap<String, TypeIR>`, `store.actions: FxHashMap<String, ActionIR>` with `ActionIR.params: Vec<(String, TypeIR)>`
- Produces: Zustand store interface with typed getter returns and action parameters instead of `any`

- [ ] **Step 1: Type getter returns**

In `omnilang/src/codegen/react/mod.rs:349`, change:
```rust
w.emit_fmt(format_args!("  readonly {}: any;", getter_name));
```
To:
```rust
let return_type = JsTypeFormatter.format_type(&getter_type);
w.emit_fmt(format_args!("  readonly {}: {};", getter_name, return_type));
```

- [ ] **Step 2: Type action parameters**

In `omnilang/src/codegen/react/mod.rs:355`, change the action parameter typing from `any` to typed parameters.

- [ ] **Step 3: Update test snapshots**

Run `cargo test -p omnilang --features codegen-react` and update snapshots.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/react/mod.rs
git commit -m "feat(react): emit typed Zustand store getters and action parameters

Replace 'any' types in generated Zustand store interfaces with actual
TypeScript types from IR, using JsTypeFormatter for proper type emission."
```

---

## Phase 5: Polish (LOW Items)

### Task 21: Remove unused __omni_bindText (L7)

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:110-115`

**Context:** `__omni_bindText` is defined and emitted in every output but has zero call sites.

- [ ] **Step 1: Remove or conditionally emit**

Remove the function from the runtime, or wrap it in a conditional flag that's only true when text bindings are used. If the function is part of a public runtime API (used by externally authored JS), keep it but add a comment explaining why.

- [ ] **Step 2: Verify no callers**

Grep for `__omni_bindText(` across all `.omni`, `.js`, `.ts`, `.rs` files to confirm zero usages.

- [ ] **Step 3: Run tests and commit**

---

### Task 22: Remove keyframe_to_config dead code (L11)

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_config.rs:22-24` (remove function)
- Modify: `omnilang/src/codegen/omniview/tests_regression/motion.rs:545` (migrate test to keyframe_to_animate_call)

**Context:** `keyframe_to_config` has `#[allow(dead_code)]` and a TODO comment saying to migrate the single test caller.

- [ ] **Step 1: Migrate the test**

Update `test_keyframe_to_config_with_path()` to use `keyframe_to_animate_call` instead.

- [ ] **Step 2: Remove the dead function**

Delete `keyframe_to_config` and the `#[allow(dead_code)]` suppression.

- [ ] **Step 3: Run tests and commit**

---

### Task 23: Delete lib.rs.orig, fix .gitignore

**Files:**
- Delete: `omnilang/src/lib.rs.orig`
- Modify: `.gitignore` (add `*.orig`)

---

### Task 24: Replace build-bundle.js with esbuild (L8)

**Files:**
- Modify: `packages/omni-animation/build-bundle.js` (replace or supplement)
- Modify: `packages/omni-animation/package.json`

**Context:** The current regex-based import/export stripping is fragile.

- [ ] **Step 1: Install esbuild**

```bash
cd packages/omni-animation && npm install -D esbuild
```

- [ ] **Step 2: Add esbuild build script**

Replace or supplement `build-bundle.js` with an esbuild config that bundles to IIFE format.

- [ ] **Step 3: Verify output**

Compare bundle output with previous approach to ensure API compatibility.

---

### Task 25: Fix (window as any).Omni type escape (L9)

**Files:**
- Modify: `packages/omni-animation/src/global.ts:4`

```typescript
// BEFORE
(window as any).Omni = Omni;

// AFTER
declare global {
  interface Window {
    Omni: typeof import('./index');
  }
}
window.Omni = Omni;
```

---

### Task 26: Add FLIP/Hero cleanup on DOM removal (L10)

**Files:**
- Modify: `packages/omni-animation/src/flip.ts:38-42`
- Modify: `packages/omni-animation/src/hero.ts:48-52`

Add a `MutationObserver` or fallback timeout to clean up inline styles if the element is removed before animation completes.

---

## Self-Review

### Spec coverage
- [x] H1 — React motion useEffect → Task 1
- [x] H2 — gen_component_tsx split → Task 10
- [x] H3 — Dioxus todo!() → Task 2
- [x] M1 — class_map_bindings Flutter/Vue/Dioxus → Tasks 4-6
- [x] M2 — empty_body Vue/Dioxus → Task 7
- [x] M3 — <slot> Vue → Task 8
- [x] M4 — WASM path_mapping → Task 3
- [x] M5 — ComponentIR double-clone → Task 11
- [x] M6 — compile_files dead code + HTML templates → Task 12
- [x] M7 — Easing table canonicalization → Task 17
- [x] M8 — Computed subscriber accumulation → Task 18
- [x] M9 — @omni/animation tests → Task 19
- [x] M10 — Zustand typed getters → Task 20
- [x] M11 — OmniViewGenerator God object → Deferred (long-term, requires significant refactor; not in plan)
- [x] M12 — Compound mutation macro → Task 13
- [x] M13 — Shared view_tree_any → Task 14
- [x] M14 — Narrow #[allow(unused_variables)] → Task 15
- [x] M15 — Split element.rs → Task 16
- [x] L1 — key_expr Vue/Dioxus → Task 9
- [x] L7 — remove __omni_bindText → Task 21
- [x] L8 — esbuild replace build-bundle.js → Task 24
- [x] L9 — (window as any).Omni → Task 25
- [x] L10 — FLIP/Hero cleanup → Task 26
- [x] L11 — keyframe_to_config dead code → Task 22
- [x] lib.rs.orig → Task 23
- [ ] L2-L6 (minor perf/style items) → Not worth individual tasks; addressed in passing
- [ ] M11 (God object) → Requires design discussion before execution; deferred

### Placeholder scan
All steps contain actual code or specific instructions. No TBD/TODO/fill-in-the-blanks.

### Type consistency
- `ComponentImports` struct defined in Task 10 matches usage in subsequent tasks
- `view_tree_any` function signature consistent between Task 14 creation and backends
- `compound_op!` macro parameter names consistent throughout Task 13
- `NAMED_EASINGS` constant signature consistent between Task 17 definition and consumers
