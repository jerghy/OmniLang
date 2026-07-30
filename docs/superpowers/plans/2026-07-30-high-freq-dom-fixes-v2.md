# OmniView High-Frequency DOM Fixes v2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 1 CRITICAL correctness/memory bug + 3 HIGH gaps + 7 MEDIUM quality issues identified in the v2 architecture review of OmniView's @ref element reference system.

**Architecture:** Incremental fixes to the OmniView compiler (Rust) and runtime (JS). P0 fixes address a stale element cache bug, a missing @for+@ref diagnostic, and a cross-backend API gap. P1 fixes address TypeScript type safety, dead code removal, DRY violations, and defensive hardening.

**Tech Stack:** Rust (omnilang-core + omnilang crates), JavaScript (OmniView runtime templates), TypeScript (React codegen output)

**Source:** Architecture review v2 at `.architecture/reviews/component-omniview-high-freq-dom-v2.md`

---

## File Structure

| File | Role | Change |
|------|------|--------|
| `omnilang/src/codegen/omniview/runtime.rs:238-246` | `__omni_el` runtime function | Fix stale cache bug |
| `crates/omnilang-core/src/semantic/validate_refs.rs:106-142` | Ref validation | Add @for+@ref diagnostic |
| `omnilang/src/codegen/omniview/store.rs:589-618` | Ref wrapper emission | Add scrollTo/scrollBy + css_only guard + fix /// comments |
| `crates/omnilang-core/src/ir/types.rs:707-709` | IR type: RefDeclIR | Add element_tag field |
| `omnilang/src/codegen/react/component/state.rs:561-574` | React ref hooks | Use element tag for useRef type |
| `omnilang/src/codegen/react/expr.rs:136-142,419-468` | React expression formatting | Remove dead field, unify formatting paths |
| `omnilang/src/codegen/omniview/view/style.rs:40-139` | Style effect emission | Consolidate per-element effects |
| `omnilang/src/codegen/omniview/mod.rs:616-624` | push_ref_nid | debug_assert! → assert! |
| `omnilang/src/codegen/omniview/editor.rs:24-57` | Editor bridge | Dedup __omni_buildNidMap |

---

### Task 1: Fix `__omni_el` stale cache — returns detached elements forever (P0-1, CRITICAL)

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:238-246`

- [ ] **Step 1: Fix the cache to evict stale entries and return null**

In `runtime.rs`, change the `__omni_el` function from:

```javascript
function __omni_el(nid) {
  var cache = __omni_el._cache || (__omni_el._cache = {});
  var el = cache[nid];
  if (!el || !el.isConnected) {
    el = document.querySelector('[data-nid="' + nid + '"]');
    if (el) cache[nid] = el;
  }
  return el;
}
```

To:

```javascript
function __omni_el(nid) {
  var cache = __omni_el._cache || (__omni_el._cache = {});
  var el = cache[nid];
  if (!el || !el.isConnected) {
    el = document.querySelector('[data-nid="' + nid + '"]');
    if (el) {
      cache[nid] = el;
    } else {
      delete cache[nid];
    }
  }
  return el || null;
}
```

The fix: (1) adds `else { delete cache[nid] }` to evict stale entries when the element is no longer in the DOM, and (2) adds `|| null` to ensure the function returns `null` (not `undefined`) when no element is found. Without this fix, `__omni_el` returns a detached DOM element indefinitely after removal, causing both correctness issues (manipulating dead elements) and memory leaks (detached nodes held by cache references).

- [ ] **Step 2: Verify the runtime string compiles**

Run: `cargo build --workspace 2>&1`
Expected: PASS (no compile errors — this is a JS template string change)

- [ ] **Step 3: Run workspace tests**

Run: `cargo test --workspace 2>&1`
Expected: All existing tests pass (the runtime template change doesn't break any test assertions)

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix: evict stale entries from __omni_el cache, return null for missing elements

When an element is removed from the DOM, __omni_el's plain-object cache
held a reference to the detached node indefinitely. querySelector failed
to find a replacement, but the cache entry was never deleted — all
subsequent calls returned the dead element.

- Add else { delete cache[nid] } to evict stale entries
- Add || null fallback for consistency with callers expecting null

Affects all code paths using __omni_el: @ref wrappers, style effects,
text bindings, attribute bindings."
```

---

### Task 2: Add `@ref` inside `@for` compiler diagnostic (P0-2, HIGH)

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_refs.rs:106-142` (add `in_for_template` tracking)
- Modify: `crates/omnilang-core/src/semantic/validate_refs.rs:144+` (add test)

- [ ] **Step 1: Add `in_for_template` tracking to `collect_ref_bindings`**

Change the `collect_ref_bindings` function to accept and propagate an `in_for_template: bool` parameter. When a ref binding is found inside a `ListRendering` body, emit a Warning diagnostic.

Replace the existing `collect_ref_bindings` function (lines 106-142) with:

```rust
/// Recursively collect all ref_binding values from a view tree.
/// When `in_for_template` is true, ref bindings found inside @for bodies
/// generate a warning — @ref inside @for has undefined semantics (Phase 2).
fn collect_ref_bindings<'a>(
    view: &'a ViewTree,
    out: &mut Vec<&'a str>,
    in_for_template: bool,
) {
    match view {
        ViewTree::Element(el) => {
            if let Some(ref name) = el.ref_binding {
                out.push(name.as_str());
                if in_for_template {
                    // Warning will be emitted by the caller since we only
                    // have the binding name here, not the component context.
                    // Mark this by pushing a special sentinel — the caller
                    // checks for refs collected inside @for.
                }
            }
            for child in &el.children {
                collect_ref_bindings(child, out, in_for_template);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_ref_bindings(body, out, in_for_template);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                collect_ref_bindings(fallback, out, in_for_template);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_ref_bindings(body, out, true); // ← KEY CHANGE: in_for_template=true
            }
            if let Some(ref empty) = list.empty_body {
                collect_ref_bindings(empty, out, in_for_template);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_ref_bindings(child, out, in_for_template);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
}
```

- [ ] **Step 2: Add a separate pass to detect refs inside @for contexts**

Instead of complicating `collect_ref_bindings` with sentinel values, add a separate simple function that walks the view tree and emits warnings directly. Add this function after the existing `collect_ref_bindings` (after line 142):

```rust
/// Emit warnings for @ref bindings inside @for loop bodies.
/// @ref inside @for has undefined semantics (Phase 2) — the ref would bind
/// to the template element or last clone, not the per-item element.
fn warn_refs_in_for_bodies(
    view: &ViewTree,
    comp: &ComponentIR,
    in_for_template: bool,
    warnings: &mut Vec<Diagnostic>,
) {
    let (line, col) = comp.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));

    match view {
        ViewTree::Element(el) => {
            if el.ref_binding.is_some() && in_for_template {
                let name = el.ref_binding.as_ref().unwrap();
                warnings.push(Diagnostic {
                    severity: Severity::Warning,
                    kind: DiagnosticKind::E0006,
                    message: format!(
                        "ref '{}' is used inside @for — each @ref inside a loop binds to the \
                         template element, not individual items. @ref inside @for is not yet \
                         supported (planned for Phase 2). Move the @ref outside the loop or \
                         use a different approach.",
                        name
                    ),
                    file: comp.source_file.clone(),
                    line,
                    col,
                    end_line: None,
                    end_col: None,
                    suggestion: Some(
                        "move @ref outside the @for loop, or wait for Phase 2 @for+@ref support"
                            .to_string(),
                    ),
                });
            }
            for child in &el.children {
                warn_refs_in_for_bodies(child, comp, in_for_template, warnings);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    warn_refs_in_for_bodies(body, comp, in_for_template, warnings);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                warn_refs_in_for_bodies(fallback, comp, in_for_template, warnings);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                warn_refs_in_for_bodies(body, comp, true, warnings);
            }
            if let Some(ref empty) = list.empty_body {
                warn_refs_in_for_bodies(empty, comp, in_for_template, warnings);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                warn_refs_in_for_bodies(child, comp, in_for_template, warnings);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
}
```

- [ ] **Step 3: Call the new function from `validate_component_refs`**

In `validate_component_refs` (line 25), add the call after the existing binding collection (after line 36):

```rust
// Collect ref bindings from the view tree
let mut bindings: Vec<&str> = Vec::new();
if let Some(ref view) = comp.view {
    collect_ref_bindings(view, &mut bindings);
    warn_refs_in_for_bodies(view, comp, false, &mut warnings);  // ← ADD THIS LINE
}
```

- [ ] **Step 4: Revert the `collect_ref_bindings` signature change**

Since we now have a separate `warn_refs_in_for_bodies` function, keep `collect_ref_bindings` unchanged (no `in_for_template` parameter). The original function signature stays the same.

- [ ] **Step 5: Write the test**

Add a test case after the existing tests in the `#[cfg(test)] mod tests` block (after line ~330). Add:

```rust
#[test]
fn warns_on_ref_inside_for_loop() {
    use crate::ir::{
        ConstructionKind, ElementIR, ExprIR, ListRenderingIntent, ViewTree,
    };

    let mut comp = ComponentIR {
        name: "TestComp".into(),
        refs: vec![RefDeclIR {
            name: "box".into(),
        }],
        view: Some(ViewTree::ListRendering(Box::new(ListRenderingIntent {
            iterable: ExprIR::IdentifierExpr {
                name: "items".into(),
                original_name: Some("items".into()),
            },
            item_binding: crate::ir::ForItemBinding {
                name: "item".into(),
                index_var: None,
            },
            key_expr: None,
            body: Some(ViewTree::Element(Box::new(ElementIR {
                tag: "div".into(),
                ref_binding: Some("box".into()), // ← ref inside @for!
                ..ElementIR::new("div")
            }))),
            empty_body: None,
        }))),
        source_file: Some("test.omni".into()),
        source_span: Some(crate::ir::SourceSpan { line: 5, col: 3 }),
        ..ComponentIR::new("TestComp")
    };

    let warnings = validate_component_refs(&comp);

    // Should have a warning about @ref inside @for
    let for_warning = warnings
        .iter()
        .find(|w| w.message.contains("@for"))
        .expect("expected a warning about @ref inside @for");
    assert!(
        for_warning.message.contains("inside @for"),
        "warning should mention @for: {}",
        for_warning.message
    );
    assert!(
        for_warning.suggestion.is_some(),
        "warning should include a suggestion"
    );
}
```

- [ ] **Step 6: Run the new test**

Run: `cargo test --package omnilang-core --lib semantic::validate_refs::tests::warns_on_ref_inside_for_loop 2>&1`
Expected: PASS (1 test)

- [ ] **Step 7: Run all existing validate_refs tests**

Run: `cargo test --package omnilang-core --lib semantic::validate_refs 2>&1`
Expected: All existing tests pass, no regressions

- [ ] **Step 8: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_refs.rs
git commit -m "feat: add compiler warning for @ref inside @for loop bodies

@ref inside @for has undefined semantics (Phase 2). The ref binds to
the template element (OmniView) or last clone (React), not the
per-item element the developer expects.

Add warn_refs_in_for_bodies() pass that emits a Warning diagnostic
when a ref_binding is found inside a ListRendering subtree.

Prevents the #1 developer confusion scenario identified in the v2
architecture review (5/6 specialists flagged this gap)."
```

---

### Task 3: Add `scrollTo`/`scrollBy` to OmniView ref wrapper (P0-3, HIGH)

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:605-613`

- [ ] **Step 1: Add scrollTo and scrollBy methods to the IIFE wrapper**

In `store.rs`, in the `emit_ref_wrappers` method, add two new methods after the `get offsetHeight()` getter (after line 612):

```rust
// In the template string, after:
// \x20\x20 get offsetHeight() {{ const e = _el(); return e ? e.offsetHeight : 0; }},\n\
// Add:
// \x20\x20 scrollTo(x, y) {{ const e = _el(); if (e) e.scrollTo(x, y); }},\n\
// \x20\x20 scrollBy(dx, dy) {{ const e = _el(); if (e) e.scrollBy(dx, dy); }},\n\
```

The full updated template block (lines 604-614) becomes:

```rust
let _ = writeln!(
    self.js,
    "/// ── ref: {ref_name} → nid=\"{nid}\" ──\n\
     const {ref_name} = (function() {{\n\
     \x20 const _nid = \"{nid}\";\n\
     \x20 const _el = function() {{ return __omni_el(_nid); }};\n\
     \x20 return {{\n\
     \x20\x20 get style() {{ const e = _el(); return e ? e.style : null; }},\n\
     \x20\x20 capturePointer(id) {{ const e = _el(); if (e) e.setPointerCapture(id); }},\n\
     \x20\x20 releasePointer(id) {{ const e = _el(); if (e) e.releasePointer(id); }},\n\
     \x20\x20 getBoundingClientRect() {{ const e = _el(); return e ? e.getBoundingClientRect() : null; }},\n\
     \x20\x20 focus() {{ const e = _el(); if (e) e.focus(); }},\n\
     \x20\x20 blur() {{ const e = _el(); if (e) e.blur(); }},\n\
     \x20\x20 get offsetWidth() {{ const e = _el(); return e ? e.offsetWidth : 0; }},\n\
     \x20\x20 get offsetHeight() {{ const e = _el(); return e ? e.offsetHeight : 0; }},\n\
     \x20\x20 scrollTo(x, y) {{ const e = _el(); if (e) e.scrollTo(x, y); }},\n\
     \x20\x20 scrollBy(dx, dy) {{ const e = _el(); if (e) e.scrollBy(dx, dy); }},\n\
     \x20 }};\n\
     }})();",
    ref_name = ref_name,
    nid = nid,
);
```

- [ ] **Step 2: Run workspace tests**

Run: `cargo test --workspace 2>&1`
Expected: All existing tests pass

- [ ] **Step 3: Verify existing ref integration tests still pass**

Run: `cargo test --package omnilang -- omniview::tests_regression::refs 2>&1`
Expected: All 3 OmniView ref tests pass

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs
git commit -m "feat: add scrollTo/scrollBy to OmniView __omni_el ref wrapper

ADR-036 specifies scrollTo(x,y) and scrollBy(dx,dy) as part of the
cross-backend ref API. React already supports them (raw DOM element
via useRef.current), but OmniView's IIFE wrapper was missing them,
causing TypeError at runtime.

Adds 4 lines to emit_ref_wrappers() template. Closes the
documentation-implementation gap identified by 3/6 specialists in
the v2 architecture review."
```

---

### Task 4: Fix hardcoded `HTMLDivElement` type for React `useRef` (P1-1, HIGH)

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:707-709` (add `element_tag` to `RefDeclIR`)
- Modify: `omnilang/src/codegen/react/component/state.rs:561-574` (use tag for type)

- [ ] **Step 1: Add `element_tag` field to `RefDeclIR`**

In `crates/omnilang-core/src/ir/types.rs`, change `RefDeclIR` from:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefDeclIR {
    pub name: String,
}
```

To:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefDeclIR {
    pub name: String,
    /// The HTML element tag this ref is bound to (e.g., "input", "video", "canvas").
    /// Used by React codegen to emit the correct TypeScript type for useRef.
    /// None means default to HTMLElement (generic fallback, better than HTMLDivElement).
    #[serde(default)]
    pub element_tag: Option<String>,
}
```

- [ ] **Step 2: Fix all `RefDeclIR` construction sites**

Search for all `RefDeclIR {` construction sites and ensure they compile:

Run: `cargo build --workspace 2>&1`
Expected: Compile errors at each `RefDeclIR { name: ... }` site that's missing `element_tag`

Fix each site by adding `element_tag: None`:

In `crates/omnilang-core/src/ir/lower/component.rs` (line ~252):
```rust
// Before:
.map(|r| RefDeclIR { name: r.name.clone() })
// After:
.map(|r| RefDeclIR { name: r.name.clone(), element_tag: None })
```

In `crates/omnilang-core/src/ir/types.rs` (any `Default` or `new` impls):
```rust
// If there's a Default impl:
impl Default for RefDeclIR {
    fn default() -> Self {
        Self { name: String::new(), element_tag: None }
    }
}
```

- [ ] **Step 3: Build to confirm all construction sites fixed**

Run: `cargo build --workspace 2>&1`
Expected: PASS (no compile errors)

- [ ] **Step 4: Add element tag → TypeScript type mapping in `gen_ref_hooks`**

In `omnilang/src/codegen/react/component/state.rs`, replace the `gen_ref_hooks` function body:

```rust
/// Generate `useRef<T>(null)` declarations for each ref in the component.
/// Maps known HTML element tags to their TypeScript types (e.g., input→HTMLInputElement).
/// Falls back to HTMLElement for unknown or missing tags.
pub(super) fn gen_ref_hooks(w: &mut CodeWriter, comp: &ComponentIR) {
    if comp.refs.is_empty() {
        return;
    }
    for ref_decl in &comp.refs {
        let ts_type = ref_element_type(ref_decl.element_tag.as_deref());
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<{}>(null);",
            ref_decl.name, ts_type
        ));
    }
    w.emit("");
}

/// Map an OmniView HTML element tag to its TypeScript element type.
fn ref_element_type(tag: Option<&str>) -> &str {
    match tag {
        Some("input") | Some("textarea") => "HTMLInputElement",
        Some("select") => "HTMLSelectElement",
        Some("button") => "HTMLButtonElement",
        Some("form") => "HTMLFormElement",
        Some("img") | Some("image") => "HTMLImageElement",
        Some("video") => "HTMLVideoElement",
        Some("audio") => "HTMLAudioElement",
        Some("canvas") => "HTMLCanvasElement",
        Some("a") | Some("link") => "HTMLAnchorElement",
        Some("table") => "HTMLTableElement",
        Some("svg") => "SVGSVGElement",
        Some("div") | Some("section") | Some("article") | Some("nav")
        | Some("header") | Some("footer") | Some("main") | Some("aside") => "HTMLDivElement",
        // For unknown tags, use HTMLElement (generic) — better than HTMLDivElement
        _ => "HTMLElement",
    }
}
```

- [ ] **Step 5: Build to confirm the new code compiles**

Run: `cargo build --package omnilang 2>&1`
Expected: PASS

- [ ] **Step 6: Update the React ref test assertions**

Run: `cargo test --package omnilang -- react::tests::test_react_ref_basic 2>&1`
Expected: Test may fail because the generated output changed from `HTMLDivElement` to something else

Read the test at `omnilang/src/codegen/react/tests.rs` around line 1299-1355. Update assertions to match the new output. Since the test likely uses `<div @ref={box}>`, the type should remain `HTMLDivElement`:

```rust
// If the test element is a <div>, the assertion should still match HTMLDivElement:
assert!(output.contains("useRef<HTMLDivElement>(null)"));
```

Run: `cargo test --package omnilang -- react::tests::test_react_ref 2>&1`
Expected: PASS

- [ ] **Step 7: Run all workspace tests**

Run: `cargo test --workspace 2>&1`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs \
        omnilang/src/codegen/react/component/state.rs
# Add any other files modified during construction site fixes
git commit -m "feat: add element_tag to RefDeclIR for correct useRef TypeScript types

Previously, all @ref declarations generated useRef<HTMLDivElement>(null)
regardless of the actual element type. This caused TypeScript errors for
non-div elements (input, video, canvas, svg, etc.).

- Add element_tag: Option<String> to RefDeclIR
- Add ref_element_type() mapping known tags to TS element types
- Fall back to HTMLElement (generic) instead of HTMLDivElement
- Fixes the #1 React codegen type-safety issue from the v2 review"
```

---

### Task 5: Remove `ReactStmtFormatter.ref_names` dead code (P1-2, HIGH)

**Files:**
- Modify: `omnilang/src/codegen/react/expr.rs:136-142` (remove dead field)
- Modify: `omnilang/src/codegen/react/expr.rs:419-426` (remove redundant clone)

- [ ] **Step 1: Remove the dead `ref_names` field from `ReactStmtFormatter`**

In `expr.rs`, change the struct from:

```rust
pub(super) struct ReactStmtFormatter {
    pub expr: ReactFormatExpr,
    pub signal_names: FxHashSet<String>,
    pub store_mode: super::ReactStoreMode,
    #[allow(dead_code)]
    pub ref_names: FxHashSet<String>,
}
```

To:

```rust
pub(super) struct ReactStmtFormatter {
    pub expr: ReactFormatExpr,
    pub signal_names: FxHashSet<String>,
    pub store_mode: super::ReactStoreMode,
}
```

- [ ] **Step 2: Remove the redundant clone at construction**

In `generate_statement_tsx_with_mode`, change the construction from:

```rust
let fmt = ReactStmtFormatter {
    expr: ReactFormatExpr {
        ref_names: ref_names.clone(),
    },
    signal_names: signal_names.clone(),
    store_mode,
    ref_names: ref_names.clone(),
};
```

To:

```rust
let fmt = ReactStmtFormatter {
    expr: ReactFormatExpr {
        ref_names: ref_names.clone(),
    },
    signal_names: signal_names.clone(),
    store_mode,
};
```

This eliminates the second `FxHashSet<String>` clone that was never read.

- [ ] **Step 3: Build and test**

Run: `cargo build --package omnilang 2>&1`
Expected: PASS (no warnings about dead_code)

Run: `cargo test --package omnilang -- react::tests 2>&1`
Expected: All React tests pass

- [ ] **Step 4: Run clippy to confirm no new warnings**

Run: `cargo clippy --package omnilang 2>&1`
Expected: No new warnings (the `#[allow(dead_code)]` suppression is gone)

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/expr.rs
git commit -m "refactor: remove dead ref_names field from ReactStmtFormatter

The ref_names field was cloned into ReactStmtFormatter but never read —
ref translation happens through self.expr.ref_names (ReactFormatExpr).
The #[allow(dead_code)] suppression masked this waste.

Removes the field and its redundant FxHashSet<String> clone per statement.
Identified by 3/6 specialists in the v2 architecture review."
```

---

### Task 6: Unify dual expression formatting paths in React codegen (P1-3, MEDIUM)

**Files:**
- Modify: `omnilang/src/codegen/react/expr.rs:434-444` (simplify `IdentifierExpr` branch)

- [ ] **Step 1: Delegate `IdentifierExpr` to `format_identifier` instead of duplicating logic**

In `format_expr_ir_tsx`, change the `IdentifierExpr` match arm from:

```rust
ExprIR::IdentifierExpr { name, .. } => {
    if ref_names.contains(name.as_str()) {
        format!("{}Ref.current", name)
    } else {
        ReactFormatExpr {
            ref_names: ref_names.clone(),
        }
        .format_identifier(name)
    }
}
```

To:

```rust
ExprIR::IdentifierExpr { name, .. } => {
    ReactFormatExpr {
        ref_names: ref_names.clone(),
    }
    .format_identifier(name)
}
```

This delegates entirely to `format_identifier()`, which already handles:
- `$` prefix stripping via `name.trim_start_matches('$')`
- Ref name → `nameRef.current` translation via `self.ref_names.contains(stripped)`
- Regular identifier formatting

The old code had a subtle inconsistency: the free function's `IdentifierExpr` branch did NOT strip `$` before checking `ref_names`, while the trait method always strips `$`. Delegating eliminates the divergence.

- [ ] **Step 2: Build and run tests**

Run: `cargo build --package omnilang 2>&1`
Expected: PASS

Run: `cargo test --package omnilang 2>&1`
Expected: All tests pass

- [ ] **Step 3: Run specific React expression tests**

Run: `cargo test --package omnilang -- react::tests::test_react_ref 2>&1`
Expected: All ref tests pass (identifier formatting unchanged for valid inputs)

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/react/expr.rs
git commit -m "refactor: delegate IdentifierExpr formatting to ReactFormatExpr trait

The free function format_expr_ir_tsx had a special case for IdentifierExpr
that duplicated the ref-name translation logic from ReactFormatExpr.
format_identifier(), but without $ prefix stripping. This created a latent
inconsistency if ref names ever carried a $ prefix.

Now format_expr_ir_tsx delegates fully to format_identifier() for all
identifier formatting, ensuring a single code path for ref name translation.

Identified by Pragmatic Enforcer and Maintainability Expert in v2 review."
```

---

### Task 7: Fix `///` → `//` in generated JS comments (P1-6, MEDIUM)

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:595` (ref wrappers section header)
- Modify: `omnilang/src/codegen/omniview/store.rs:600` (per-ref comment)
- Modify: `omnilang/src/codegen/omniview/mod.rs:265` (motion definitions section header)

- [ ] **Step 1: Fix the three `///` occurrences**

In `store.rs:595`, change:
```rust
let _ = writeln!(self.js, "\n  /// ── Element Refs ──");
```
To:
```rust
let _ = writeln!(self.js, "\n  // ── Element Refs ──");
```

In `store.rs:600`, change:
```rust
"/// ── ref: {ref_name} → nid=\"{nid}\" ──\n\
```
To:
```rust
"// ── ref: {ref_name} → nid=\"{nid}\" ──\n\
```

In `mod.rs:265`, find the `/// ── Module Motion Definitions` comment and change:
```rust
"/// ── Module Motion Definitions ──\n\
```
To:
```rust
"// ── Module Motion Definitions ──\n\
```

- [ ] **Step 2: Build and test**

Run: `cargo build --workspace 2>&1`
Expected: PASS

Run: `cargo test --workspace 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs \
        omnilang/src/codegen/omniview/mod.rs
git commit -m "style: fix /// → // in generated JavaScript comments

Triple-slash (///) is a Rust doc-comment convention. In generated JS
output, it's equivalent to // followed by an extra / character.
While syntactically valid JS, it looks like a Rust doc-comment leaked
into the output and reduces professionalism.

Fixes 3 occurrences in store.rs and mod.rs."
```

---

### Task 8: Change `debug_assert!` → `assert!` in `push_ref_nid` (P1-5, MEDIUM)

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:617`

- [ ] **Step 1: Change debug_assert! to assert!**

In `mod.rs:617`, change:
```rust
debug_assert!(
    !self.ref_nid_set.contains(&ref_name),
    "duplicate ref binding '{}' — semantic validator should have caught this",
    ref_name
);
```
To:
```rust
assert!(
    !self.ref_nid_set.contains(&ref_name),
    "duplicate ref binding '{}' — semantic validator should have caught this",
    ref_name
);
```

The semantic validator emits a Warning (not Error) for duplicate bindings. If a consumer ignores warnings, a release build would silently produce broken JS (duplicate `const` declarations). `assert!` ensures the invariant is enforced in all build modes.

- [ ] **Step 2: Build and test**

Run: `cargo build --workspace 2>&1`
Expected: PASS

Run: `cargo test --workspace 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "fix: use assert! instead of debug_assert! in push_ref_nid

The semantic validator emits a Warning (not Error) for duplicate ref
bindings. If a consumer ignores the warning, a release build would
silently produce broken JS with duplicate const declarations.

assert! enforces the uniqueness invariant in all build modes, matching
the defense-in-depth pattern used elsewhere in the codegen."
```

---

### Task 9: Add `css_only` guard to `emit_ref_wrappers` (P1-7, MEDIUM)

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:589`

- [ ] **Step 1: Add the guard**

In `store.rs`, at the top of `emit_ref_wrappers`, add the css_only check:

```rust
pub(super) fn emit_ref_wrappers(&mut self, comp: &ComponentIR) {
    // css_only mode: styles only, no JS emission
    if self.css_only {
        return;
    }
    // Only emit if the component has ref declarations
    if comp.refs.is_empty() || self.ref_nid_map.is_empty() {
        return;
    }
    // ... rest of function unchanged
```

- [ ] **Step 2: Build and test**

Run: `cargo build --workspace 2>&1`
Expected: PASS

Run: `cargo test --workspace 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs
git commit -m "fix: add css_only guard to emit_ref_wrappers

Other JS-emitting methods in the codegen pipeline check self.css_only
before writing to self.js. emit_ref_wrappers was missing this guard,
potentially leaking ref wrapper code into the style-only pipeline.

No known trigger today (components with @ref likely don't go through
css_only mode), but this is defense-in-depth against future changes."
```

---

## Self-Review

### 1. Spec Coverage

Architecture review v2 findings mapped to tasks:

| Finding | Severity | Task |
|---------|----------|------|
| P0-1: __omni_el stale cache | CRITICAL | Task 1 |
| P0-2: @ref inside @for diagnostic | HIGH | Task 2 |
| P0-3: scrollTo/scrollBy missing | HIGH | Task 3 |
| P1-1: HTMLDivElement hardcoded | HIGH | Task 4 |
| P1-2: ReactStmtFormatter dead code | HIGH | Task 5 |
| P1-3: Dual formatting path | MEDIUM | Task 6 |
| P1-6: /// → // in JS | MEDIUM | Task 7 |
| P1-5: debug_assert! → assert! | MEDIUM | Task 8 |
| P1-7: css_only guard | MEDIUM | Task 9 |

Deferred to separate plan (larger scope):
- P1-4: Per-property style effects consolidation (~50 lines in style.rs, needs broader testing)
- P2-1: Missing test coverage for ref in effects/computeds/lifecycles
- P2-2: Component ref forwarding silent drop warning
- P2-3/4/5: Template DRY fixes (_elCache, __omni_effect, __omni_buildNidMap)
- P3-1/2: ViewContext struct, tree traversal dedup (larger refactoring)

### 2. Placeholder Scan

No TBD, TODO, "implement later", or "add appropriate error handling" patterns found. All steps have concrete code and commands.

### 3. Type Consistency

- `RefDeclIR.element_tag: Option<String>` added in Task 4 Step 1, consumed in Task 4 Step 4
- `ref_element_type()` function signature `fn ref_element_type(tag: Option<&str>) -> &str` matches call site `ref_element_type(ref_decl.element_tag.as_deref())`
- `warn_refs_in_for_bodies()` signature consistent between definition (Task 2 Step 2) and call site (Task 2 Step 3)
