# OmniView High-Frequency Element Manipulation Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the 3 P0 regressions in OmniView's high-frequency element manipulation: (1) React codegen ref expression dead code causing `ReferenceError`, (2) FLIP `flipBefore` ordering bug, (3) `@for` nid collision breaking list-item motion triggers.

**Architecture:** Thread `ref_names: FxHashSet<String>` through all React codegen expression paths — from `gen_component_tsx` builder, through `gen_view_node_tsx`, `gen_functions`, `gen_effects`, `gen_computed_values`, `format_inline_handler`, into `ReactFormatExpr.format_identifier` and `format_expr_ir_tsx`. Fix FLIP by reordering 2 lines in list.rs. Fix nid collision by generating index-qualified nids at clone time.

**Tech Stack:** Rust (omnilang crate), OmniLang IR, JS/TSX codegen

## Global Constraints

- All 837+ workspace tests must pass with zero regressions
- Clippy clean, rustfmt clean
- Run `cargo test -p omnilang` before each commit
- Commit after each task with `feat:` prefix

---

## File Structure

| File | Responsibility | Change |
|------|---------------|--------|
| `omnilang/src/codegen/react/component/mod.rs` | Entry point — builds `ref_names` set | Modify: add `ref_names` construction, thread to all callers |
| `omnilang/src/codegen/react/expr.rs` | `ReactFormatExpr`, `ReactStmtFormatter`, `format_expr_ir_tsx`, `generate_statement_tsx*` | Modify: add `ref_names` field/params |
| `omnilang/src/codegen/react/view.rs` | `gen_view_node_tsx`, `format_inline_handler` | Modify: add `ref_names` param, thread through |
| `omnilang/src/codegen/react/component/function.rs` | `gen_functions` | Modify: add `ref_names` param |
| `omnilang/src/codegen/react/component/effect.rs` | `gen_effects` | Modify: add `ref_names` param |
| `omnilang/src/codegen/react/component/computed.rs` | `gen_computed_values` | Modify: add `ref_names` param |
| `omnilang/src/codegen/react/component/jsx.rs` | `gen_jsx_return` | Modify: add `ref_names` param, thread to `gen_view_node_tsx` |
| `omnilang/src/codegen/react/tests.rs` | Ref tests | Modify: update assertions |
| `omnilang/src/codegen/omniview/list.rs` | `@for` rendering, FLIP | Modify: reorder 2 lines, add nid indexing |
| `omnilang/src/codegen/omniview/tests_regression/list.rs` | List rendering tests | Modify: update FLIP/nid assertions |

---

### Task 1: Thread `ref_names` through React codegen expression pipeline

**Files:**
- Modify: `omnilang/src/codegen/react/expr.rs:19-31` (ReactFormatExpr struct)
- Modify: `omnilang/src/codegen/react/expr.rs:127-131` (ReactStmtFormatter struct)
- Modify: `omnilang/src/codegen/react/expr.rs:354-406` (generate_statement_tsx / generate_statement_tsx_with_mode)
- Modify: `omnilang/src/codegen/react/component/mod.rs:98-101, 146-350` (gen_component_tsx)
- Modify: `omnilang/src/codegen/react/view.rs:16-22` (gen_view_node_tsx signature)
- Modify: `omnilang/src/codegen/react/view.rs:808-818` (format_inline_handler)
- Modify: `omnilang/src/codegen/react/component/function.rs:16-21` (gen_functions)
- Modify: `omnilang/src/codegen/react/component/effect.rs:15-21` (gen_effects)
- Modify: `omnilang/src/codegen/react/component/computed.rs:13-28` (gen_computed_values)
- Modify: `omnilang/src/codegen/react/component/jsx.rs:16-26` (gen_jsx_return)
- Modify: `omnilang/src/codegen/react/tests.rs:1286-1333` (test_react_ref_in_handler)

**Interfaces:**
- Consumes: `ComponentIR.refs: Vec<RefDeclIR>` (existing)
- Produces: `ref_names: FxHashSet<String>` — set of ref names built from `comp.refs.iter().map(|r| r.name.clone()).collect()`
- Produces: `ReactFormatExpr { ref_names: FxHashSet<String> }` — new struct field
- Produces: `format_expr_ir_tsx(expr, ref_names)` — existing signature, now populated

- [ ] **Step 1: Add `ref_names` field to `ReactFormatExpr`**

In `omnilang/src/codegen/react/expr.rs`, replace the unit struct:

```rust
// BEFORE (line 19):
pub(crate) struct ReactFormatExpr;

// AFTER:
pub(crate) struct ReactFormatExpr {
    pub ref_names: FxHashSet<String>,
}
```

- [ ] **Step 2: Update `format_identifier` to translate ref names**

In `omnilang/src/codegen/react/expr.rs`, modify the `format_identifier` method (line 29):

```rust
// BEFORE:
fn format_identifier(&self, name: &str) -> String {
    name.trim_start_matches('$').to_string()
}

// AFTER:
fn format_identifier(&self, name: &str) -> String {
    let stripped = name.trim_start_matches('$');
    if self.ref_names.contains(stripped) {
        format!("{}Ref.current", stripped)
    } else {
        stripped.to_string()
    }
}
```

- [ ] **Step 3: Add `ref_names` to `ReactStmtFormatter`**

In `omnilang/src/codegen/react/expr.rs`, modify the struct (line 127):

```rust
// BEFORE:
pub(super) struct ReactStmtFormatter {
    pub expr: ReactFormatExpr,
    pub signal_names: FxHashSet<String>,
    pub store_mode: super::ReactStoreMode,
}

// AFTER:
pub(super) struct ReactStmtFormatter {
    pub expr: ReactFormatExpr,
    pub signal_names: FxHashSet<String>,
    pub store_mode: super::ReactStoreMode,
    pub ref_names: FxHashSet<String>,
}
```

- [ ] **Step 4: Add `ref_names` param to `generate_statement_tsx` and `generate_statement_tsx_with_mode`**

In `omnilang/src/codegen/react/expr.rs`, modify both function signatures (lines 354, 362):

```rust
// BEFORE:
pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
) {
    generate_statement_tsx_with_mode(w, stmt, signal_names, super::ReactStoreMode::Hooks);
}

pub(super) fn generate_statement_tsx_with_mode(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
    store_mode: super::ReactStoreMode,
) {

// AFTER:
pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
) {
    generate_statement_tsx_with_mode(w, stmt, signal_names, super::ReactStoreMode::Hooks, ref_names);
}

pub(super) fn generate_statement_tsx_with_mode(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
    store_mode: super::ReactStoreMode,
    ref_names: &FxHashSet<String>,
) {
```

- [ ] **Step 5: Update `array_push`/`array_pop` intercept to pass `ref_names`**

In `omnilang/src/codegen/react/expr.rs`, line 385, update the `format_expr_ir_tsx` call:

```rust
// BEFORE:
.map(|e| format_expr_ir_tsx(e, &FxHashSet::default()))

// AFTER:
.map(|e| format_expr_ir_tsx(e, ref_names))
```

- [ ] **Step 6: Update `ReactStmtFormatter` construction to include `ref_names`**

In `omnilang/src/codegen/react/expr.rs`, lines 400-405:

```rust
// BEFORE:
let fmt = ReactStmtFormatter {
    expr: ReactFormatExpr,
    signal_names: signal_names.clone(),
    store_mode,
};

// AFTER:
let fmt = ReactStmtFormatter {
    expr: ReactFormatExpr {
        ref_names: ref_names.clone(),
    },
    signal_names: signal_names.clone(),
    store_mode,
    ref_names: ref_names.clone(),
};
```

- [ ] **Step 7: Build `ref_names` set in `gen_component_tsx`**

In `omnilang/src/codegen/react/component/mod.rs`, after line 147 (after `signal_names` construction), add:

```rust
// Build ref_names set for expression translation (ADR-036 Phase 2)
let ref_names: FxHashSet<String> = comp
    .refs
    .iter()
    .map(|r| r.name.clone())
    .collect();
```

- [ ] **Step 8: Thread `ref_names` through `gen_functions`**

In `omnilang/src/codegen/react/component/function.rs`, modify the signature (line 16):

```rust
// BEFORE:
pub(super) fn gen_functions(
    &self,
    w: &mut CodeWriter,
    comp: &ComponentIR,
    called_fns: &FxHashSet<String>,
    signal_names: &FxHashSet<String>,
    fn_names: &FxHashSet<String>,
) {

// AFTER:
pub(super) fn gen_functions(
    &self,
    w: &mut CodeWriter,
    comp: &ComponentIR,
    called_fns: &FxHashSet<String>,
    signal_names: &FxHashSet<String>,
    fn_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
) {
```

Find all calls to `generate_statement_tsx` inside `gen_functions` and add `ref_names` as the new argument. In `function.rs`, look for `generate_statement_tsx(&mut w, stmt, signal_names)` and change to `generate_statement_tsx(&mut w, stmt, signal_names, ref_names)`.

- [ ] **Step 9: Thread `ref_names` through `gen_effects`**

In `omnilang/src/codegen/react/component/effect.rs`, modify the signature (line 15):

```rust
// BEFORE:
pub(super) fn gen_effects(
    &self,
    w: &mut CodeWriter,
    comp: &ComponentIR,
    store_signal_map: &FxHashMap<String, String>,
    signal_names: &FxHashSet<String>,
) {

// AFTER:
pub(super) fn gen_effects(
    &self,
    w: &mut CodeWriter,
    comp: &ComponentIR,
    store_signal_map: &FxHashMap<String, String>,
    signal_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
) {
```

Find all calls to `generate_statement_tsx` inside `gen_effects` and add `ref_names` as the new argument.

- [ ] **Step 10: Thread `ref_names` through `gen_computed_values`**

In `omnilang/src/codegen/react/component/computed.rs`, modify the signature (line 13):

```rust
// BEFORE:
pub(super) fn gen_computed_values(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    known_signal_names: &[String],
    signal_names: &FxHashSet<String>,
    store_mode: super::ReactStoreMode,
) {

// AFTER:
pub(super) fn gen_computed_values(
    w: &mut CodeWriter,
    comp: &ComponentIR,
    known_signal_names: &[String],
    signal_names: &FxHashSet<String>,
    store_mode: super::ReactStoreMode,
    ref_names: &FxHashSet<String>,
) {
```

Find all calls to `format_expr_ir_tsx` inside `gen_computed_values` and replace `&FxHashSet::default()` with `ref_names`.

- [ ] **Step 11: Thread `ref_names` through `gen_view_node_tsx`**

In `omnilang/src/codegen/react/view.rs`, modify the signature (line 16):

```rust
// BEFORE:
pub(super) fn gen_view_node_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    signal_names: &FxHashSet<String>,
    motion_names: &FxHashSet<String>,
) {

// AFTER:
pub(super) fn gen_view_node_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    signal_names: &FxHashSet<String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
) {
```

Replace ALL calls to `format_expr_ir_tsx(..., &FxHashSet::default())` inside `gen_view_node_tsx` with `format_expr_ir_tsx(..., ref_names)`. There are approximately 12 call sites (lines 44, 135, 175, 442, 568, 589, 601, 633, 636, 708 — verify exact count after edits).

- [ ] **Step 12: Thread `ref_names` through `format_inline_handler`**

In `omnilang/src/codegen/react/view.rs`, modify the function (line 808):

```rust
// BEFORE:
fn format_inline_handler(
    body: &BlockIR,
    signal_names: &FxHashSet<String>,
    params: &[String],
) -> String {

// AFTER:
fn format_inline_handler(
    body: &BlockIR,
    signal_names: &FxHashSet<String>,
    params: &[String],
    ref_names: &FxHashSet<String>,
) -> String {
```

Update the `generate_statement_tsx` call inside (line 816):

```rust
// BEFORE:
generate_statement_tsx(&mut body_w, stmt, signal_names);

// AFTER:
generate_statement_tsx(&mut body_w, stmt, signal_names, ref_names);
```

- [ ] **Step 13: Thread `ref_names` through `gen_jsx_return`**

In `omnilang/src/codegen/react/component/jsx.rs`, modify the signature (line 16):

```rust
// BEFORE:
pub(super) fn gen_jsx_return(
    &self,
    w: &mut CodeWriter,
    comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    signal_names: &FxHashSet<String>,
    motion_names: &FxHashSet<String>,
    name: &str,
) {

// AFTER:
pub(super) fn gen_jsx_return(
    &self,
    w: &mut CodeWriter,
    comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    signal_names: &FxHashSet<String>,
    motion_names: &FxHashSet<String>,
    name: &str,
    ref_names: &FxHashSet<String>,
) {
```

Thread `ref_names` through all internal calls to `gen_view_node_tsx`. Search for `gen_view_node_tsx(w, node, component_map, signal_names, motion_names)` and add `ref_names`.

- [ ] **Step 14: Update all call sites in `gen_component_tsx`**

In `omnilang/src/codegen/react/component/mod.rs`, update every caller:

```rust
// Lines 314-332, update each invocation:

// BEFORE:
self.gen_effects(&mut w, comp, &store_signal_map, &signal_names);
// AFTER:
self.gen_effects(&mut w, comp, &store_signal_map, &signal_names, &ref_names);

// BEFORE:
computed::gen_computed_values(&mut w, comp, &known_signal_names, &signal_names, self.store_mode);
// AFTER:
computed::gen_computed_values(&mut w, comp, &known_signal_names, &signal_names, self.store_mode, &ref_names);

// BEFORE:
self.gen_functions(&mut w, comp, &called_fns, &signal_names, &fn_names);
// AFTER:
self.gen_functions(&mut w, comp, &called_fns, &signal_names, &fn_names, &ref_names);

// BEFORE:
self.gen_jsx_return(&mut w, comp, component_map, &signal_names, &motion_names, name);
// AFTER:
self.gen_jsx_return(&mut w, comp, component_map, &signal_names, &motion_names, name, &ref_names);
```

- [ ] **Step 15: Update `format_inline_handler` call sites in `gen_view_node_tsx`**

In `omnilang/src/codegen/react/view.rs`, find all calls to `format_inline_handler(body, signal_names, &params)` and add `ref_names`:

```rust
// BEFORE:
format_inline_handler(body, signal_names, &params)

// AFTER:
format_inline_handler(body, signal_names, &params, ref_names)
```

Search for `format_inline_handler(` in view.rs to find all call sites (approximately 3-4).

- [ ] **Step 16: Fix remaining `FxHashSet::default()` call sites**

Run a grep across the entire react codegen directory to ensure no `format_expr_ir_tsx` call still uses `FxHashSet::default()`:

```bash
cd D:/space/OmniLang && grep -rn "FxHashSet::default()" omnilang/src/codegen/react/ --include="*.rs"
```

Each remaining instance must be replaced with `ref_names`. If a function doesn't have `ref_names` in scope, add it as a parameter and thread it from its caller.

- [ ] **Step 17: Build and verify compilation**

Run: `cargo check -p omnilang 2>&1`
Expected: Compilation succeeds with zero errors

- [ ] **Step 18: Update React ref handler test assertion**

In `omnilang/src/codegen/react/tests.rs`, line 1329, update the assertion from broken to correct:

```rust
// BEFORE (line 1324-1332):
// NOTE: Inside function handler bodies, ref names are NOT auto-translated
// to Ref.current (codegen limitation - expressions in fn bodies use
// ReactFormatExpr which doesn't apply ref name resolution).
// The handler references the ref by its original name.
assert!(
    content.contains("box.style.transform"),
    "Handler body should reference 'box'. Got:\n{}",
    content
);

// AFTER:
// Ref names in function bodies are now translated to nameRef.current (ADR-036 Phase 2)
assert!(
    content.contains("boxRef.current.style.transform"),
    "Handler body should reference 'boxRef.current'. Got:\n{}",
    content
);
```

Also remove the "codegen limitation" comment above and replace with the corrected assertion comment.

- [ ] **Step 19: Run full test suite**

```bash
cargo test -p omnilang 2>&1
```

Expected: ALL tests pass, including `test_react_ref_in_handler`

- [ ] **Step 20: Commit**

```bash
git add omnilang/src/codegen/react/
git commit -m "feat: thread ref_names through React codegen expression pipeline

- Add ref_names field to ReactFormatExpr and ReactStmtFormatter
- Add ref_names param to generate_statement_tsx, gen_view_node_tsx,
  gen_functions, gen_effects, gen_computed_values, gen_jsx_return,
  format_inline_handler
- Build ref_names set from comp.refs in gen_component_tsx
- Replace all FxHashSet::default() call sites with threaded ref_names
- Update test_react_ref_in_handler to assert boxRef.current translation

Fixes P0-1 from architecture review: React ref expression dead code
causing ReferenceError in function handler bodies."
```

---

### Task 2: Fix FLIP `flipBefore` ordering

**Files:**
- Modify: `omnilang/src/codegen/omniview/list.rs:116-189` (reorder 2 lines)
- Modify: `omnilang/src/codegen/omniview/tests_regression/list.rs:203-210` (update FLIP assertion)

**Interfaces:**
- Consumes: `Omni.flipBefore(parent)` (existing JS runtime function)
- Produces: correct FLIP capture — `flipBefore` runs BEFORE item removal

- [ ] **Step 1: Move `flipBefore` before the clearing loop**

In `omnilang/src/codegen/omniview/list.rs`, lines 117-122 and 188-189, reorder:

```rust
// BEFORE (current code, lines 117-122 + 188-189):
self.js.push_str("  // Clear existing rendered items\n");
self.js.push_str("  while (tpl.nextElementSibling && tpl.nextElementSibling !== ctr) {\n");
self.js.push_str("    parent.removeChild(tpl.nextElementSibling);\n");
self.js.push_str("  }\n");
// ... ~65 lines later ...
if has_key {
    self.js.push_str("  Omni.flipBefore(parent);\n");  // LINE 189 — TOO LATE
}

// AFTER (reorder: flipBefore BEFORE clear):
// FLIP: record element positions BEFORE clearing (must run before removal)
if has_key {
    self.js.push_str("  Omni.flipBefore(parent);\n");
}
// Clear existing rendered items (now runs AFTER flipBefore captures positions)
self.js.push_str("  // Clear existing rendered items\n");
self.js.push_str("  while (tpl.nextElementSibling && tpl.nextElementSibling !== ctr) {\n");
self.js.push_str("    parent.removeChild(tpl.nextElementSibling);\n");
self.js.push_str("  }\n");
```

And remove the original `Omni.flipBefore(parent)` line at line 189.

- [ ] **Step 2: Update FLIP regression test assertions**

In `omnilang/src/codegen/omniview/tests_regression/list.rs`, lines 203-210, update so the test verifies `flipBefore` appears BEFORE the clearing loop:

```rust
// BEFORE (line 203-210):
// FLIP animation recording
assert!(
    gen.js.contains("Omni.flipBefore(parent)"),
    "should record FLIP positions before"
);
assert!(
    gen.js.contains("Omni.flipAfter(parent)"),
    "should animate FLIP positions after"
);

// AFTER: Verify flipBefore runs BEFORE the while-remove loop
let flip_before_pos = gen.js.find("Omni.flipBefore(parent)").unwrap();
let clear_pos = gen.js.find("while (tpl.nextElementSibling").unwrap();
assert!(
    flip_before_pos < clear_pos,
    "flipBefore should run BEFORE clearing loop, got flipBefore at {} and clear at {}",
    flip_before_pos, clear_pos
);
assert!(
    gen.js.contains("Omni.flipAfter(parent)"),
    "should animate FLIP positions after"
);
```

- [ ] **Step 3: Verify existing FLIP tests still pass**

```bash
cargo test -p omnilang test_flip 2>&1
cargo test -p omnilang test_keyed 2>&1
```

Expected: All FLIP-related tests pass

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/list.rs omnilang/src/codegen/omniview/tests_regression/list.rs
git commit -m "fix: move Omni.flipBefore before clearing loop in @for re-render

flipBefore was called after the while-remove loop which had already
deleted all rendered items, always capturing zero elements. Move it
before the removal so it captures item positions for FLIP animation.

Fixes P0-2 from architecture review: FLIP before-snapshot ordering bug."
```

---

### Task 3: Fix `@for` template nid collision for motion triggers

**Files:**
- Modify: `omnilang/src/codegen/omniview/list.rs:232-247` (add nid indexing at clone time)
- Modify: `omnilang/src/codegen/omniview/tests_regression/list.rs` (add multi-item motion test)

**Interfaces:**
- Consumes: `__omni_el(nid)` (existing runtime)
- Produces: index-qualified nids `{nid}_{idx}` on cloned elements

- [ ] **Step 1: Add nid index suffix at clone time**

In `omnilang/src/codegen/omniview/list.rs`, after line 231 (the `clone` call), add nid reassignment:

```rust
// AFTER line 231-233, INSERT:
// Assign unique nids per clone to avoid collisions with motion triggers
// Without this, all clones share the template nid, and __omni_el()
// (via querySelector) only finds the first clone.
self.js.push_str("    // Assign unique nids per clone for motion trigger isolation\n");
self.js.push_str("    (function assignUniqueNids(el, idx) {\n");
self.js.push_str("      var nid = el.getAttribute('data-nid');\n");
self.js.push_str("      if (nid) {\n");
self.js.push_str("        var uniqueNid = nid + '_' + idx;\n");
self.js.push_str("        el.setAttribute('data-nid', uniqueNid);\n");
self.js.push_str("      }\n");
self.js.push_str("      el.querySelectorAll('[data-nid]').forEach(function(child) {\n");
self.js.push_str("        var childNid = child.getAttribute('data-nid');\n");
self.js.push_str("        if (childNid) {\n");
self.js.push_str("          child.setAttribute('data-nid', childNid + '_' + idx);\n");
self.js.push_str("        }\n");
self.js.push_str("      });\n");
self.js.push_str("    })(clone, _idx);\n");
```

This must be inserted AFTER the clone line (`const clone = ...`) and AFTER the `data-for-item` attribute is set on lines 232-234, but BEFORE the `_elCache` population on line 237. The exact insertion point is between lines 234 and 236.

- [ ] **Step 2: Update `_elCache` population to use indexed nids**

The `cacheElements` function on lines 238-247 currently uses `el.getAttribute('data-nid')` directly. After Step 1, the nids already have the `_idx` suffix appended, so no change is needed — the cache will store the indexed nids.

But verify: check that `cacheElements` runs AFTER `assignUniqueNids`. If the code order is:
1. Set `data-for-item` attribute
2. Run `assignUniqueNids` (makes nids unique)
3. Run `cacheElements` (populates cache with unique nids)

Then the cache will correctly map indexed nids to their clone elements. This is correct.

- [ ] **Step 3: Add multi-item motion trigger regression test**

In `omnilang/src/codegen/omniview/tests_regression/list.rs`, add a new test after the existing FLIP tests:

```rust
/// Test: @for with motion triggers generates unique nids per clone,
/// ensuring motion triggers fire on all list items, not just the first.
#[test]
fn test_for_motion_unique_nids_per_clone() {
    let source = r#"
component ListMotion {
  signal items: List<String> = ["a", "b", "c"]
  view {
    <ul>
      <li @for={item in items}
          @onEnter={fadeIn}>
        {item}
      </li>
    </ul>
  }
}
"#;
    let ir = compile_to_ir(source).unwrap();
    let gen = generate_omniview_with_ir(&ir);
    let js = &gen.js;

    // Verify nid indexing is generated at clone time
    assert!(
        js.contains("assignUniqueNids"),
        "Should generate assignUniqueNids function. Got:\n{}",
        js
    );
    // Verify index-qualified nids are used
    assert!(
        js.contains("nid + '_' + idx"),
        "Should append index to nid for uniqueness. Got:\n{}",
        js
    );
    // Verify no duplicate nid usage (template nid without index)
    let nid_matches: Vec<&str> = js
        .lines()
        .filter(|l| l.contains("__omni_el("))
        .collect();
    assert!(
        nid_matches.len() >= 2,
        "Should have at least 2 __omni_el calls (template + container). Got: {}",
        nid_matches.len()
    );
}
```

- [ ] **Step 4: Run test suite**

```bash
cargo test -p omnilang test_for_motion_unique_nids 2>&1
cargo test -p omnilang 2>&1
```

Expected: New test passes, all existing tests pass with zero regressions

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/list.rs omnilang/src/codegen/omniview/tests_regression/list.rs
git commit -m "fix: generate unique nids per clone in @for list rendering

Add assignUniqueNids function that appends _idx suffix to data-nid
attributes on each cloned element. This prevents nid collisions where
all clones shared the same static nid from the template, causing
__omni_el(nid) to only ever find the first clone via querySelector.

Fixes P0-3 from architecture review: @for nid collision breaking
motion triggers on non-first list items."
```

---

### Task 4: Add `push_ref_nid` helper with uniqueness enforcement

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:529` (ref_nid_map field, add FxHashSet guard)
- Modify: `omnilang/src/codegen/omniview/view/element.rs:646` (ref_nid_map push call site)
- Modify: `omnilang/src/codegen/omniview/store.rs:152` (ref_nid_map push call site)

**Interfaces:**
- Produces: `push_ref_nid(name, nid)` — panics on duplicate via `debug_assert!`

- [ ] **Step 1: Add `ref_nid_set` field to generator struct**

In `omnilang/src/codegen/omniview/mod.rs`, near line 529, add a companion set:

```rust
// BEFORE (line 529):
pub ref_nid_map: Vec<(String, String)>,

// AFTER:
pub ref_nid_map: Vec<(String, String)>,
/// Enforce uniqueness — duplicate ref_binding names would produce
/// duplicate const declarations in generated JS (SyntaxError).
ref_nid_set: FxHashSet<String>,
```

- [ ] **Step 2: Add `push_ref_nid` helper method**

In `omnilang/src/codegen/omniview/mod.rs`, add the method near the existing field declarations:

```rust
/// Register a ref→nid binding with uniqueness enforcement.
/// Panics (debug_assert) on duplicates — the semantic validator
/// should catch duplicates before codegen, but this is defense-in-depth.
pub(super) fn push_ref_nid(&mut self, ref_name: String, nid: String) {
    debug_assert!(
        !self.ref_nid_set.contains(&ref_name),
        "duplicate ref binding '{ref_name}' — semantic validator should have caught this"
    );
    self.ref_nid_set.insert(ref_name.clone());
    self.ref_nid_map.push((ref_name, nid));
}
```

- [ ] **Step 3: Replace direct `push` calls with `push_ref_nid`**

In `omnilang/src/codegen/omniview/view/element.rs`, find all `self.ref_nid_map.push((ref_name, nid))` calls and replace:

```rust
// BEFORE:
self.ref_nid_map.push((ref_name.clone(), nid.clone()));

// AFTER:
self.push_ref_nid(ref_name.clone(), nid.clone());
```

Similarly in `omnilang/src/codegen/omniview/store.rs`.

- [ ] **Step 4: Initialize `ref_nid_set` in the generator `Default` impl**

Find the `Default` impl or new() for the generator struct and ensure `ref_nid_set: FxHashSet::default()` is included.

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p omnilang 2>&1
```

Expected: Compiles cleanly

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/
git commit -m "feat: add push_ref_nid with uniqueness guard for ref_nid_map

Add FxHashSet companion set with debug_assert on duplicate insert.
Replaces bare Vec::push calls with push_ref_nid helper. Prevents
silent generation of duplicate const declarations (which produce JS
SyntaxError) if a future code path bypasses semantic validation.

Fixes P1-1 from architecture review."
```

---

### Task 5: Fix unnecessary `el.children.clone()` in `try_inline_component`

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs:122-131`

- [ ] **Step 1: Move clone into the else branch**

In `omnilang/src/codegen/omniview/component_html.rs`, lines 122-131:

```rust
// BEFORE:
let slot_children: Vec<ViewTree> = el.children.clone();
let child_html = if slot_children.is_empty() {
    self.view_to_html(child_view, indent + 1)
} else {
    let child_view_owned = inject_slot_children(child_view.clone(), &slot_children);
    self.view_to_html(&child_view_owned, indent + 1)
};

// AFTER:
let child_html = if el.children.is_empty() {
    self.view_to_html(child_view, indent + 1)
} else {
    let slot_children: Vec<ViewTree> = el.children.clone();
    let child_view_owned = inject_slot_children(child_view.clone(), &slot_children);
    self.view_to_html(&child_view_owned, indent + 1)
};
```

- [ ] **Step 2: Verify compilation and tests**

```bash
cargo check -p omnilang 2>&1
cargo test -p omnilang 2>&1
```

Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/component_html.rs
git commit -m "perf: defer el.children clone in try_inline_component

Only clone children vector when slots are present. For the common
case (no <slot> directives), the allocation is entirely wasted.

Fixes P1-2 from architecture review."
```

---
