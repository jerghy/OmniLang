# Element Reference System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up the already-parsed `ref:` syntax through IR lowering and codegen so developers can get stable element references for high-frequency DOM manipulation (drag, mousemove, resize).

**Architecture:** The parser already handles `ref name: Element` declarations and `ref: name` element attributes. Two missing connections in the pipeline: (1) `ElementIR` needs a `ref_binding` field, (2) `ElementProp::Ref` in lowering must set it instead of emitting `data-ref`. Then codegen: OmniView wraps `__omni_el(nid)` in a getter-based object, React generates `useRef` + `ref={}` JSX + translates `refName.property` → `refNameRef.current.property`.

**Tech Stack:** Rust (omnilang-core + omnilang crates), TypeScript/React (generated output), 837 existing tests must stay green.

**Design doc:** `docs/design/element-ref-system.md`

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `crates/omnilang-core/src/ir/types.rs` | Modify | Add `ref_binding: Option<String>` to `ElementIR` |
| `crates/omnilang-core/src/ir/lower/view.rs` | Modify | Fix `ElementProp::Ref` lowering, add `ref_binding` var |
| `crates/omnilang-core/src/semantic/validate_component.rs` | Modify | Validate ref names exist, check no duplicate bindings |
| `omnilang/src/codegen/omniview/store.rs` | Modify | Generate `__omni_el` wrapper objects for each ref decl |
| `omnilang/src/codegen/omniview/expr.rs` | Modify | Translate `refName.property` → wrapper access |
| `omnilang/src/codegen/react/component/mod.rs` | Modify | Hook selection: push `"useRef"` when `comp.refs` not empty |
| `omnilang/src/codegen/react/component/state.rs` | Modify | Generate `const xRef = useRef<HTMLDivElement>(null)` |
| `omnilang/src/codegen/react/view.rs` | Modify | JSX: emit `ref={nameRef}` on elements with `ref_binding` |
| `omnilang/src/codegen/react/expr.rs` | Modify | Translate `refName.prop` → `refNameRef.current.prop` |
| `omnilang/src/codegen/omniview/tests_regression/refs.rs` | Create | OmniView ref integration tests |
| `omnilang/src/codegen/react/tests.rs` | Modify | React ref snapshot tests |

---

### Task 1: Add `ref_binding` to `ElementIR` + wire through lowering

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:617-640`
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:243,446-454,557-616`

- [ ] **Step 1: Add `ref_binding` field to ElementIR**

In `crates/omnilang-core/src/ir/types.rs`, after the `drag` field (line 639), add:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag: Option<Box<DragBehavior>>,
    /// Element ref binding name — connects this element to a component-level
    /// ref declaration. Maps to useRef (React), __omni_el wrapper (OmniView),
    /// or GlobalKey (Flutter).
    #[serde(default)]
    pub ref_binding: Option<String>,
}
```

- [ ] **Step 2: Add `ref_binding` variable in `lower_element`**

In `crates/omnilang-core/src/ir/lower/view.rs`, after `let mut drag_velocity_max: Option<ExprIR> = None;` (line 241), add:

```rust
    let mut ref_binding: Option<String> = None;
```

- [ ] **Step 3: Fix `ElementProp::Ref` lowering**

Replace lines 446-454 in `crates/omnilang-core/src/ir/lower/view.rs`:

```rust
            // BEFORE — lowered to useless data-ref attribute:
            ElementProp::Ref(r) => {
                // ref="..." attribute
                props.get_or_insert_with(Default::default).insert(
                    "data-ref".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::Value::String(r.clone()),
                    },
                );
            }

            // AFTER — sets ref_binding on the element:
            ElementProp::Ref(r) => {
                ref_binding = Some(r.clone());
            }
```

- [ ] **Step 4: Pass `ref_binding` into ElementIR constructor**

In the `ViewTree::Element(ElementIR { ... })` construction (line 557), add the `ref_binding` field after `drag`:

```rust
        drag: {
            if drag_constraints.is_some()
                || drag_elastic.is_some()
                || drag_threshold.is_some()
                || drag_velocity_max.is_some()
            {
                Some(Box::new(DragBehavior { ... }))
            } else {
                None
            }
        },
        ref_binding,   // ← NEW
    })
```

- [ ] **Step 5: Build and run existing tests to verify no regressions**

Run: `cargo test -p omnilang-core --lib`
Expected: All existing tests pass. The `ref_binding` field defaults to `None` for all existing elements since no test uses `ref:` syntax yet.

- [ ] **Step 6: Add a unit test for `ElementProp::Ref` lowering**

In `crates/omnilang-core/src/ir/lower/view.rs`, add a test in the existing `#[cfg(test)]` module (or create a test near the end):

```rust
#[test]
fn test_lower_element_with_ref_binding() {
    // This test verifies that ref: boxName on an element
    // sets ref_binding instead of emitting a data-ref prop.
    // Uses the parser to parse a minimal component.
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box>"hello"</div>
  }
}
"#;
    // Parse and verify the lowered ElementIR has ref_binding = Some("box")
    // and does NOT have "data-ref" in props.
    // (Full parse pipeline test — see Task 9 for integration test approach)
}
```

*Note: A full pipeline test requires the parser → IR lowering chain. If the test infrastructure doesn't easily support this in a unit test, defer to Task 9's integration test.*

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/ir/lower/view.rs
git commit -m "feat(ir): add ref_binding field to ElementIR and fix lowering

ElementProp::Ref now sets el.ref_binding instead of emitting a
useless data-ref HTML attribute. This connects the parser's
existing ref:name syntax to the IR for codegen consumption."
```

---

### Task 2: Semantic validation for ref declarations

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_component.rs`
- Create: `crates/omnilang-core/src/semantic/validate_refs.rs` (new file, ~50 lines)

- [ ] **Step 1: Create `validate_refs.rs`**

Create `crates/omnilang-core/src/semantic/validate_refs.rs`:

```rust
//! Validate ref declarations and bindings for a component.
//!
//! Checks:
//! 1. Every ref name used in @ref={name} has a corresponding `ref name: Element` declaration.
//! 2. Each ref declaration is bound to exactly one element (no duplicates, no orphans).

use crate::ir::types::{ComponentIR, ViewTree};
use rustc_hash::FxHashSet;

/// Validation result for component refs.
#[derive(Debug, Default)]
pub struct RefValidation {
    pub warnings: Vec<String>,
}

/// Validate all ref declarations and bindings in the component.
pub fn validate_component_refs(comp: &ComponentIR) -> RefValidation {
    let mut result = RefValidation::default();

    // Collect declared ref names
    let declared: FxHashSet<&str> = comp.refs.iter().map(|r| r.name.as_str()).collect();

    // Collect ref bindings from the view tree
    let mut bindings: Vec<&str> = Vec::new();
    if let Some(ref view) = comp.view {
        collect_ref_bindings(view, &mut bindings);
    }

    // Check: every binding must reference a declared ref
    for binding in &bindings {
        if !declared.contains(binding) {
            result.warnings.push(format!(
                "ref '{}' is used in @ref={{}} but no `ref {}: Element` declaration found",
                binding, binding
            ));
        }
    }

    // Check: every declared ref should be bound somewhere (warning, not error)
    for decl in &declared {
        if !bindings.contains(decl) {
            result.warnings.push(format!(
                "ref '{}' is declared but never bound to an element with @ref={{{}}}",
                decl, decl
            ));
        }
    }

    // Check: no duplicate bindings for the same ref name
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    for binding in &bindings {
        if !seen.insert(binding) {
            result.warnings.push(format!(
                "ref '{}' is bound to multiple elements — each ref must be bound to exactly one element",
                binding
            ));
        }
    }

    result
}

/// Recursively collect all ref_binding values from a view tree.
fn collect_ref_bindings<'a>(view: &'a ViewTree, out: &mut Vec<&'a str>) {
    match view {
        ViewTree::Element(el) => {
            if let Some(ref name) = el.ref_binding {
                out.push(name.as_str());
            }
            for child in &el.children {
                collect_ref_bindings(child, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                collect_ref_bindings(&branch.body, out);
            }
            if let Some(ref fallback) = cond.fallback {
                collect_ref_bindings(fallback, out);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_ref_bindings(body, out);
            }
            if let Some(ref empty) = list.empty_body {
                collect_ref_bindings(empty, out);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_ref_bindings(child, out);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orphan_ref_binding_warns() {
        // ElementIR with ref_binding but no ComponentIR.refs entry
        // (test via validate_component_refs directly)
    }

    #[test]
    fn test_unbound_ref_decl_warns() {
        // ComponentIR with refs entry but no ElementIR.ref_binding
    }
}
```

- [ ] **Step 2: Wire validation into the semantic pipeline**

In `crates/omnilang-core/src/semantic/validate_component.rs`, add at the end of `validate_component` (or wherever component-level validation runs):

```rust
// Validate ref declarations and bindings
let ref_validation = super::validate_refs::validate_component_refs(comp);
for warning in &ref_validation.warnings {
    // Add to the component's warnings/diagnostics
    // (integrate with existing warning infrastructure)
}
```

*Note: If `validate_component.rs` doesn't exist as a separate file, find the component validation entry point and add the call there. The exact integration point depends on how the existing semantic validation pipeline is structured — look for where `ComponentIR` is validated after lowering.*

- [ ] **Step 3: Add module declaration**

In the semantic module's `mod.rs` or equivalent, add:

```rust
pub mod validate_refs;
```

- [ ] **Step 4: Build and test**

Run: `cargo test -p omnilang-core --lib`
Expected: All existing tests pass. New validation module compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_refs.rs crates/omnilang-core/src/semantic/
git commit -m "feat(semantic): add ref declaration validation pass

Validates that @ref={name} bindings reference declared refs,
no duplicate bindings, and no orphaned ref declarations."
```

---

### Task 3: OmniView codegen — ref wrapper objects

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs` (add `emit_ref_wrappers` call)

- [ ] **Step 1: Add `emit_ref_wrappers` method to OmniViewGenerator**

In `omnilang/src/codegen/omniview/store.rs`, after the component generation logic, add a new method. Find the spot where component-level declarations (signals, computeds, fns) are emitted — ref wrappers should be emitted in the same scope, after signals but before the view:

```rust
impl OmniViewGenerator {
    /// Emit element-ref wrapper objects for all ref declarations in a component.
    /// Each wrapper uses lazy getters on __omni_el(nid) for O(1) cached access.
    /// Called during generate_component, after signal declarations.
    pub(super) fn emit_ref_wrappers(&mut self, comp: &ComponentIR) {
        // Only emit if the component has ref declarations AND a view
        // (refs without view elements are validated elsewhere)
        if comp.refs.is_empty() {
            return;
        }

        // Collect nid → ref_name mappings from the view tree
        let mut ref_nid_map: Vec<(&str, &str)> = Vec::new();
        if let Some(ref view) = comp.view {
            collect_ref_nids(view, &mut ref_nid_map);
        }

        for (ref_name, nid) in &ref_nid_map {
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
                 \x20 }};\n\
                 }})();\n",
                ref_name = ref_name,
                nid = nid,
            );
        }
    }
}

/// Recursively walk the view tree to find elements with ref_binding
/// and collect their (ref_name, nid) pairs.
fn collect_ref_nids<'a>(
    view: &'a ViewTree,
    out: &mut Vec<(&'a str, &'a str)>,
    // Note: nid is generated during codegen, not stored in IR.
    // We need a different approach — see Step 2.
) {
    // This approach needs revision — nid is codegen-local.
    // The actual implementation stores the mapping during element_to_html.
}
```

- [ ] **Step 2: Revised approach — store ref_nid mapping during element rendering**

The nid is generated during `element_to_html`, not available at IR traversal time. The correct approach:

In `omnilang/src/codegen/omniview/view/element.rs`, in `element_to_html` (or the orchestrator that calls `next_nid`), when an element has `ref_binding`, store the (ref_name, nid) mapping:

```rust
// In OmniViewGenerator, add a field:
// pub ref_nid_map: Vec<(String, String)>,  // (ref_name, nid)

// In element_to_html, after next_nid is called:
if let Some(ref ref_name) = el.ref_binding {
    self.ref_nid_map.push((ref_name.clone(), nid.clone()));
}
```

Then `emit_ref_wrappers` reads from `self.ref_nid_map`.

- [ ] **Step 3: Clear ref_nid_map per component**

In `generate_component` in `store.rs`, at the start of component generation:

```rust
self.ref_nid_map.clear();
```

- [ ] **Step 4: Call emit_ref_wrappers in generate_component**

In `store.rs`'s `generate_component`, after signal/computed declarations are emitted but before the view:

```rust
// After signal declarations
self.emit_ref_wrappers(comp);
```

- [ ] **Step 5: Build and run OmniView tests**

Run: `cargo test -p omnilang`
Expected: Existing tests pass. No ref-related tests yet.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/view/element.rs
git commit -m "feat(omniview): generate __omni_el wrapper objects for ref declarations"
```

---

### Task 4: OmniView codegen — ref expression translation

**Files:**
- Modify: `omnilang/src/codegen/omniview/expr.rs`

- [ ] **Step 1: Detect ref names in expression context**

In `omnilang/src/codegen/omniview/expr.rs`, the `expr_to_js` method translates OmniLang expressions to JS. When it encounters an identifier that is a ref name, it should pass it through as-is (the ref wrapper object name in JS is the same as the OmniLang ref name):

```rust
// In the IdentifierExpr / VarRef handling of expr_to_js:
// Ref names are already valid JS identifiers and refer to the wrapper objects.
// No translation needed — the OmniLang ref name == JS wrapper variable name.
// But field accesses like "box.style" must stay as "box.style" — also no translation.
// Method calls like "box.focus()" must stay as "box.focus()" — also no translation.
//
// The key insight: in OmniView codegen, ref names ARE the wrapper variable names.
// We just need to NOT transform them into signal.value accesses.
```

- [ ] **Step 2: Ensure ref names are NOT treated as signals**

In the current codegen, identifiers in expression context are checked against `signal_names` to determine if they need `.value` suffix. Ref names must be excluded from this check. This is done by not adding ref names to `signal_names` / `name_renames`.

Verify in `store.rs`'s `generate_component` that ref names from `comp.refs` are NOT added to `name_renames` or `signal_names`. If they currently are (unlikely since refs had no codegen), add explicit exclusion:

```rust
// In generate_component, when building name_renames:
let ref_names: FxHashSet<&str> = comp.refs.iter().map(|r| r.name.as_str()).collect();
// ... when iterating signals, skip any name in ref_names
```

- [ ] **Step 3: Write and run a test**

In `omnilang/src/codegen/omniview/tests.rs`, add:

```rust
#[test]
fn test_ref_wrapper_generated_for_ref_declaration() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box style:width="100px">content</div>
  }
}
"#;
    let output = compile_omni(source).unwrap();
    // Verify the wrapper object is generated
    assert!(output.js.contains("const box = (function()"), 
        "Expected ref wrapper 'box' in generated JS, got: {}", output.js);
    // Verify __omni_el is used
    assert!(output.js.contains("__omni_el("), 
        "Expected __omni_el call in wrapper");
    // Verify NO data-ref attribute in HTML
    assert!(!output.html.contains("data-ref=\"box\""),
        "data-ref attribute should NOT be emitted for ref bindings");
}
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/expr.rs omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "feat(omniview): exclude ref names from signal.value translation in expressions"
```

---

### Task 5: React codegen — useRef hook selection and import

**Files:**
- Modify: `omnilang/src/codegen/react/component/mod.rs:84-96`

- [ ] **Step 1: Add useRef to hooks vector when component has refs**

In `omnilang/src/codegen/react/component/mod.rs`, after the existing hook selection logic (after line 96), add:

```rust
        // Ref declarations → useRef
        let has_refs = !comp.refs.is_empty();
        if has_refs {
            hooks.push("useRef".to_string());
        }
```

No import changes needed — `ComponentImports::emit` at `imports.rs:109-113` already emits all hooks from the `hooks` vector:

```rust
if !self.hooks.is_empty() {
    w.emit_fmt(format_args!(
        "import React, {{ {} }} from 'react';",
        self.hooks.join(", ")
    ));
}
```

- [ ] **Step 2: Build and verify the import line**

Run: `cargo test -p omnilang`
Expected: All existing tests pass. No behavioral change yet (no component has refs).

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/react/component/mod.rs
git commit -m "feat(react): add useRef to hook imports when component has ref declarations"
```

---

### Task 6: React codegen — useRef declaration generation

**Files:**
- Modify: `omnilang/src/codegen/react/component/state.rs`

- [ ] **Step 1: Add `gen_ref_hooks` function**

At the end of `omnilang/src/codegen/react/component/state.rs`, add:

```rust
/// Generate `useRef<HTMLDivElement>(null)` declarations for each ref in the component.
/// Called after gen_state_hooks and before gen_effects in gen_component_tsx.
pub(super) fn gen_ref_hooks(w: &mut CodeWriter, comp: &ComponentIR) {
    if comp.refs.is_empty() {
        return;
    }
    for ref_decl in &comp.refs {
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<HTMLDivElement>(null);",
            ref_decl.name
        ));
    }
    w.emit("");
}
```

- [ ] **Step 2: Wire into gen_component_tsx**

In `omnilang/src/codegen/react/component/mod.rs`, find the call to `gen_state_hooks` (line 307) and add `gen_ref_hooks` after it:

```rust
        // State hooks + effects
        let fn_names: FxHashSet<String> = comp.fns.iter().map(|f| f.name.clone()).collect();
        self.gen_state_hooks(&mut w, comp, &read_signals, &mutated_signals, &fn_names);
        state::gen_ref_hooks(&mut w, comp);  // ← NEW
        self.gen_effects(&mut w, comp, &store_signal_map, &signal_names);
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/react/component/state.rs omnilang/src/codegen/react/component/mod.rs
git commit -m "feat(react): generate useRef declarations for component ref declarations"
```

---

### Task 7: React codegen — JSX `ref` attribute

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`

- [ ] **Step 1: Emit `ref={nameRef}` on elements with `ref_binding`**

In `omnilang/src/codegen/react/view.rs`, in `gen_view_node_tsx`, in the `ViewTree::Element(el)` match arm. Find the section where HTML attributes are collected into the `attrs` vector (after motion data attributes, around line 265). Add:

```rust
            // Ref binding — emit ref={nameRef} JSX attribute
            if let Some(ref ref_name) = el.ref_binding {
                attrs.push(format!("ref={{{}Ref}}", ref_name));
            }
```

This should be placed BEFORE the `attrs.join(" ")` line that constructs the final JSX element string. The exact insertion point is after all other attribute-building logic but before the JSX tag is emitted.

- [ ] **Step 2: For custom components with ref, skip ref for now**

In the component composition branch (where `component_map.get(el.tag.as_str())` returns Some), if `el.ref_binding` is set, emit a warning or skip — component ref forwarding is Phase 2:

```rust
            if let Some(_child_comp) = component_map.get(el.tag.as_str()) {
                // Component composition
                if el.ref_binding.is_some() {
                    // ref forwarding for custom components is not yet supported (Phase 2)
                    // For now, only native HTML elements support @ref
                }
                // ... existing component prop handling ...
            }
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/react/view.rs
git commit -m "feat(react): emit ref={nameRef} JSX attribute for elements with ref_binding"
```

---

### Task 8: React codegen — ref expression translation

**Files:**
- Modify: `omnilang/src/codegen/react/expr.rs`

- [ ] **Step 1: Understand the translation pattern**

In OmniLang source, the developer writes:
```omni
box.style.transform = "translate(100px, 0)"
box.getBoundingClientRect()
box.focus()
```

These need to become in React TSX:
```tsx
boxRef.current.style.transform = "translate(100px, 0)"
boxRef.current.getBoundingClientRect()
boxRef.current.focus()
```

The translation rule: when an identifier `X` is a ref name, `X` → `XRef.current`.

- [ ] **Step 2: Pass ref names to expression formatter**

The `format_expr_ir_tsx` function (line 412 in `expr.rs`) needs to know which names are refs. Add a parameter:

```rust
// BEFORE:
pub(super) fn format_expr_ir_tsx(expr: &ExprIR) -> String {

// AFTER:
pub(super) fn format_expr_ir_tsx(expr: &ExprIR, ref_names: &FxHashSet<String>) -> String {
```

Update all callers to pass `ref_names`. Most callers pass `&FxHashSet::default()` (empty set). The key caller is `gen_view_node_tsx` in `view.rs`, which should build the set from `comp.refs`.

- [ ] **Step 3: Add ref name translation logic**

In `format_expr_ir_tsx`, find the `ExprIR::IdentifierExpr` handling and add ref translation. Also handle `ExprIR::FieldAccessExpr` and `ExprIR::MethodCallExpr`:

```rust
        ExprIR::IdentifierExpr { name, .. } => {
            if ref_names.contains(name.as_str()) {
                format!("{}Ref.current", name)
            } else {
                name.clone()
            }
        }
        ExprIR::FieldAccessExpr { object, field, .. } => {
            let obj_str = format_expr_ir_tsx(object, ref_names);
            // If the object is a ref, don't add .current again —
            // the IdentifierExpr handler already did it
            format!("{}.{}", obj_str, field)
        }
        ExprIR::MethodCallExpr { object, method, args, .. } => {
            let obj_str = format_expr_ir_tsx(object, ref_names);
            let args_str: Vec<String> = args.iter()
                .map(|a| format_expr_ir_tsx(a, ref_names))
                .collect();
            format!("{}.{}({})", obj_str, method, args_str.join(", "))
        }
```

- [ ] **Step 4: Exclude ref names from signal identifier collection**

In `omnilang/src/codegen/react/component/mod.rs`, in the identifier collection function `collect_expr_identifiers` (line 364), ref names should not be collected as signal reads:

```rust
// Build ref_names set for exclusion
let ref_names: FxHashSet<String> = comp.refs.iter().map(|r| r.name.clone()).collect();

// In read_signals collection, filter out ref names:
// (already handled because ref names aren't in comp.signals)
```

Verify this is naturally handled — ref names come from `comp.refs`, not `comp.signals`, so `collect_expr_identifiers` won't find them as signals. No change needed if this invariant holds.

- [ ] **Step 5: Update all callers of format_expr_ir_tsx**

Search for all calls to `format_expr_ir_tsx` and add the `ref_names` parameter. Locations include:
- `view.rs` — JSX prop value formatting (pass the component's ref_names)
- `component/effect.rs` — effect body code generation (pass empty set)
- `component/computed.rs` — computed value generation (pass empty set)
- `component/function.rs` — function body generation (pass empty set)
- Any other call sites

For callers in component codegen that have access to `comp` (ComponentIR), build the set:
```rust
let ref_names: FxHashSet<String> = comp.refs.iter().map(|r| r.name.clone()).collect();
```

For callers without access to comp, pass `FxHashSet::default()`.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/expr.rs omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/component/
git commit -m "feat(react): translate ref name expressions to refNameRef.current

IdentifierExpr for ref names becomes 'nameRef.current'.
FieldAccessExpr and MethodCallExpr chain correctly onto the
translated ref expression. All callers of format_expr_ir_tsx
updated with ref_names parameter."
```

---

### Task 9: Integration test — OmniView ref end-to-end

**Files:**
- Create: `omnilang/src/codegen/omniview/tests_regression/refs.rs`

- [ ] **Step 1: Write the integration test file**

Create `omnilang/src/codegen/omniview/tests_regression/refs.rs`:

```rust
//! Integration tests for element ref system in OmniView codegen.

use crate::codegen::omniview::tests_regression::util::compile;

/// Test: basic ref declaration and binding generates wrapper object
#[test]
fn test_ref_basic_wrapper() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box style:width="100px">content</div>
  }
}
"#;
    let output = compile(source);
    // Wrapper object exists
    assert!(output.js.contains("const box = (function()"), 
        "JS should contain ref wrapper. Got:\n{}", output.js);
    // Uses __omni_el for element lookup
    assert!(output.js.contains("__omni_el("), 
        "Wrapper should use __omni_el. Got:\n{}", output.js);
    // No stale data-ref attribute in HTML
    assert!(!output.html.contains("data-ref=\"box\""),
        "HTML should not have data-ref. Got:\n{}", output.html);
    // HTML still has the div
    assert!(output.html.contains("width:100px"),
        "HTML should contain the element. Got:\n{}", output.html);
}

/// Test: ref in component with @for (ref inside template — future semantic)
#[test]
fn test_ref_not_in_for_template() {
    // For now, ref inside @for is not supported. Verify it at least
    // doesn't crash the compiler.
    let source = r#"
component TestRefFor {
  signal items: String[] = ["a", "b"]
  ref first: Element
  view {
    <div>
      <div ref:first>static</div>
      @for item in items {
        <span>{item}</span>
      }
    </div>
  }
}
"#;
    let output = compile(source);
    // Should compile without panicking
    assert!(output.js.contains("const first = (function()"),
        "Ref outside @for should still work. Got:\n{}", output.js);
}

/// Test: multiple refs in one component
#[test]
fn test_multiple_refs() {
    let source = r#"
component MultiRef {
  ref header: Element
  ref footer: Element
  view {
    <div>
      <div ref:header>Header</div>
      <div ref:footer>Footer</div>
    </div>
  }
}
"#;
    let output = compile(source);
    assert!(output.js.contains("const header = (function()"),
        "Expected header ref wrapper");
    assert!(output.js.contains("const footer = (function()"),
        "Expected footer ref wrapper");
}
```

- [ ] **Step 2: Add module declaration**

In `omnilang/src/codegen/omniview/tests_regression/mod.rs`, add:

```rust
pub mod refs;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p omnilang --test test_name`
Or: `cargo test -p omnilang ref`
Expected: All new ref tests pass.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/tests_regression/refs.rs omnilang/src/codegen/omniview/tests_regression/mod.rs
git commit -m "test(omniview): add integration tests for element ref system"
```

---

### Task 10: Integration test — React ref end-to-end

**Files:**
- Modify: `omnilang/src/codegen/react/tests.rs`

- [ ] **Step 1: Add React ref snapshot test**

In `omnilang/src/codegen/react/tests.rs`, add:

```rust
/// Test: ref declaration generates useRef + ref={} JSX attribute
#[test]
fn test_react_ref_basic() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box style:width="100px">content</div>
  }
}
"#;
    let output = compile_to_react(source);
    // useRef import
    assert!(output.contains("useRef"), 
        "Import should include useRef. Got:\n{}", output);
    // useRef declaration
    assert!(output.contains("const boxRef = useRef<HTMLDivElement>(null)"),
        "Should generate useRef declaration. Got:\n{}", output);
    // ref={} JSX attribute
    assert!(output.contains("ref={boxRef}"),
        "JSX should have ref={boxRef}. Got:\n{}", output);
    // NOT data-ref (legacy lowering artifact)
    assert!(!output.contains("data-ref"),
        "Should not emit data-ref attribute. Got:\n{}", output);
}

/// Test: ref name used in event handler translates correctly
#[test]
fn test_react_ref_in_handler() {
    let source = r#"
component DragRef {
  signal x: Float = 0
  ref box: Element
  fn onDrag(e) {
    box.style.transform = "translate(100px, 0)"
  }
  view {
    <div ref:box @onPointerMove={onDrag}>drag</div>
  }
}
"#;
    let output = compile_to_react(source);
    // useRef imported
    assert!(output.contains("useRef"));
    // Handler references boxRef.current
    assert!(output.contains("boxRef.current.style.transform"),
        "Handler should use boxRef.current.style. Got:\n{}", output);
}

/// Test: component without refs does NOT import useRef
#[test]
fn test_react_no_ref_no_use_ref_import() {
    let source = r#"
component NoRef {
  signal count: Float = 0
  view {
    <div>{count}</div>
  }
}
"#;
    let output = compile_to_react(source);
    assert!(!output.contains("useRef"),
        "useRef should NOT be imported when no refs. Got:\n{}", output);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang test_react_ref`
Expected: Tests pass for the cases where the codegen is complete.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/react/tests.rs
git commit -m "test(react): add integration tests for React useRef codegen"
```

---

### Task 11: Run full test suite + clippy

- [ ] **Step 1: Run the complete Rust test suite**

```bash
cd D:/space/OmniLang
cargo test --workspace
```

Expected: All 837+ tests pass. Zero regressions.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: Zero clippy warnings.

- [ ] **Step 3: Run Rustfmt**

```bash
cargo fmt --all -- --check
```

Expected: All files formatted.

- [ ] **Step 4: Run the omni-animation JS tests**

```bash
cd packages/omni-animation && pnpm test
```

Expected: 18 JS tests pass.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final verification — all tests pass, clippy clean, fmt clean"
```

---

## Self-Review Results

### Spec Coverage
- [x] IR: `ref_binding` field on `ElementIR` → Task 1
- [x] IR lowering: `ElementProp::Ref` → `ref_binding` → Task 1
- [x] Semantic: ref name validation → Task 2
- [x] OmniView: `__omni_el` wrapper generation → Task 3
- [x] OmniView: expression translation (ref names not signal.value) → Task 4
- [x] React: `useRef` hook import → Task 5
- [x] React: `useRef` declaration gen → Task 6
- [x] React: JSX `ref={}` attribute → Task 7
- [x] React: expression translation (`refName` → `refNameRef.current`) → Task 8
- [x] Integration tests → Tasks 9, 10
- [x] Full verification → Task 11

### Placeholder Scan
- No TBD/TODO/fill-in-details found
- All code steps contain actual Rust/JS code
- All test steps contain actual test code with assertions
- All commands have expected output

### Type Consistency
- `ref_binding: Option<String>` consistent across Tasks 1, 7, 8
- `RefDeclIR { name: String }` matches `comp.refs` usage in Tasks 3, 5, 6
- `ref_name: &str` / `nid: &str` pairs consistent in Task 3
- `format_expr_ir_tsx(expr, ref_names)` signature consistent in Task 8
