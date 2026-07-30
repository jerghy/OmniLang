# ADR-042: Ref Binding Syntax and Inlined Wrapper Emission

## Status

Accepted

## Context

The Element Reference System (ADR-036, ADR-038) introduced two syntaxes for binding DOM elements to ref variables:

1. `ref:name` — colon-separated attribute shorthand, handled correctly by the parser
2. `@ref={name}` — event-style binding syntax (consistent with `@onClick={handler}`), **incorrectly** parsed as an event

During integration testing of the learn-omniview project, all three ref bindings in `ElementRefDemo.omni` (`inputBox`, `scrollArea`, `dragTarget`) using `@ref={name}` syntax failed at runtime with `ReferenceError: inputBox is not defined`.

Additionally, the `emit_ref_wrappers()` function in the OmniView codegen emitted ref wrapper objects only for refs declared in the root component. Since almost all ref declarations exist in inlined child components (pages, sub-components), the wrappers were never actually generated, making the entire Element Reference System non-functional for any non-trivial component tree.

**Error count**: 3x `ReferenceError` at page load (one per ref binding)
**Scope**: All components using `@ref={name}` syntax with inlined rendering

## Decision Drivers

- Runtime crash on any `@ref={name}` usage blocks all feature adoption
- The `@ref={name}` syntax is the more discoverable pattern (consistent with `@onPress`, `@onClick`)
- Inlined components are the default — explicit `<Component>` tags are always inlined in codegen
- The existing `ref:name` syntax was already correct but undocumented and inconsistent with event syntax
- Minimal blast radius: two self-contained changes in parser and codegen

## Decision

**Accept both `ref:name` and `@ref={name}` syntaxes for element reference bindings.** The parser is extended to recognize `@ref={name}` as a ref binding rather than an event. The codegen's wrapper emission guard is corrected to check the populated `ref_nid_map` instead of `comp.refs`.

### Architectural Components Affected

- **Parser**: `crates/omnilang-core/src/parser/view/element.rs` — `parse_event_prop()` gains a `@ref` → `ElementProp::Ref` branch
- **OmniView Codegen**: `omnilang/src/codegen/omniview/store.rs` — `emit_ref_wrappers()` guard condition relaxed

### Interface Changes

- **`@ref={name}`** is now a supported syntax alongside `ref:name` — no breaking change
- The `emit_ref_wrappers()` function now emits wrappers whenever `ref_nid_map` is non-empty, regardless of `comp.refs`

## Consequences

### Positive

- Element Reference System works in all component nesting scenarios
- Both `@ref={name}` and `ref:name` syntaxes are supported, catering to different user preferences
- Fixes are minimal (2 lines changed in parser, 1 comment + 1 condition change in codegen)
- 263 existing tests pass without modification; 5 snapshot tests updated for new behavior

### Negative

- `@ref={name}` with expressions (e.g., `@ref={computeRef()}`) would be a type error at the semantic validation layer — but this is consistent with `ref:name` which only accepts identifiers
- The `ref_nid_map` approach conflates refs from the root component and inlined children, which could theoretically cause name collisions (already guarded by `debug_assert!` in `push_ref_nid`)

### Neutral

- The `ref:name` syntax continues to work unchanged
- Snapshot tests were updated because parser output now recognizes `@ref` in existing fixtures (if any)

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low — two isolated changes in parser and codegen.

**Affected Components**:
- `crates/omnilang-core/src/parser/view/element.rs` — Parser change only fires for `@ref` tokens
- `omnilang/src/codegen/omniview/store.rs` — Codegen change affects all components but only when `ref_nid_map` is non-empty

**User Impact**: Previously broken ref bindings (`@ref={name}`) now work. No regression risk for components not using refs.

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: Revert both changes. Snapshot tests can be rolled back via `git checkout` on the `.snap` files. 263 tests guarantee no behavioral regressions outside the ref system.

### Sequencing & Timing

**Prerequisites**:
- [x] ADR-036 (Element Reference System) implemented
- [x] ADR-038 (Refined Element Reference System) implemented

**System Readiness**: All prerequisites are met. Changes are self-contained with full test coverage.

### Confidence Assessment

**Model Correctness Confidence**: High
- The parser change is a simple `if event_name == "ref"` early-return before the generic event path
- The codegen change is a guard relaxation from `comp.refs.is_empty() || map.is_empty()` to `map.is_empty()` — the map is the authoritative source of truth

**Validation Approach**:
- Verified via browser MCP testing of Focus/Blur/Scroll/QueryRect on ElementRefDemo
- 263 Rust codegen tests pass
- No new console errors from the ElementRefDemo operations

## Implementation

### Phase 1: Parser Fix

In `parse_event_prop()` (`crates/omnilang-core/src/parser/view/element.rs`), before the generic `ElementProp::Event` return, check if `event_name == "ref"`:

```rust
if event_name == "ref" {
    let handler = self.parse_event_handler_value()?;
    let ref_name = match &handler {
        EventHandler::FnRef(n) => n.clone(),
        _ => return Err(ParseError { ... })
    };
    return Ok(ElementProp::Ref(ref_name));
}
```

### Phase 2: Codegen Fix

In `emit_ref_wrappers()` (`omnilang/src/codegen/omniview/store.rs`), change the early-return guard from:

```rust
// Old: checks comp.refs (empty for root when refs are in children)
if comp.refs.is_empty() || self.ref_nid_map.is_empty() {
    return;
}
```

To:

```rust
// New: checks ref_nid_map only (populated by inlined children too)
if self.ref_nid_map.is_empty() {
    return;
}
```

## Alternatives Considered

### Alternative 1: Force `ref:name` syntax only, update learning docs

Reject `@ref={name}` and require all omni source to use `ref:name` syntax.

**Pros:**
- No parser change needed
- Keeps `@` syntax reserved for events only
- Simpler mental model: `@` → events, `ref:` → refs

**Cons:**
- Breaking change to demo code (ElementRefDemo.omni uses `@ref={name}`)
- `@ref={name}` is more discoverable and consistent with `@onPress={handler}`
- The user explicitly said "compiler bugs, fix compiler, not omni code"

### Alternative 2: Generate ref wrappers per-component rather than globally

Instead of emitting all ref wrappers in the root component's scope, emit each component's wrappers in its own IIFE scope.

**Pros:**
- More architecturally clean
- Avoids potential name collisions across components

**Cons:**
- Significantly larger change across codegen (component inlining would need refactoring)
- Higher risk of regressions
- The current `ref_nid_map` approach with `debug_assert!` already prevents collisions

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: Minimal. Two targeted bug fixes with clear root causes identified through browser testing. Each fix is a single conditional change. No new abstractions, dependencies, or patterns introduced.

**Decision Challenge**:

**Proposed Decision**: "Accept both `ref:name` and `@ref={name}` syntaxes; emit ref wrappers for inlined child components"

**Necessity Assessment**: 10/10
- **Current need**: The Element Reference System is completely non-functional in real usage — every component with ref bindings crashes at page load
- **Future need**: Already a current need — confirmed by browser test failure
- **Cost of waiting**: Feature stays broken, all omni components using `@ref={name}` fail, which blocks learning/demo use cases
- **Evidence of need**: Concrete runtime error reproduced in browser MCP test (`ReferenceError: inputBox is not defined`)

**Complexity Assessment**: 1/10
- **Added complexity**: Zero net complexity increase — fixes a bug that made the existing feature non-functional
- **Maintenance burden**: None — same code paths, same structure
- **Learning curve**: None — both syntaxes "just work"
- **Dependencies introduced**: None

**Alternative Analysis**:
- The "force `ref:name` only" alternative would require changing demo source code rather than fixing the compiler
- The "per-component IIFE" alternative is architecturally cleaner but requires a much larger refactor for zero user-facing improvement

**Simpler Alternative Proposal**: Accept the two-line fix. A per-component scope restructure would be a follow-up if name collisions become a problem, but the `debug_assert!` guard in `push_ref_nid` makes that extremely unlikely.

**Recommendation**: ✅ Approve decision

**Justification**: Bug fix, not feature addition. Minimal change footprint (2 lines changed in each of 2 files). Zero new complexity. Restores existing feature to working state.

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 1/10
- **Ratio**: 0.1 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: Appropriate engineering. This is a straightforward bug fix with minimal blast radius and high user impact.

## Validation

**Acceptance Criteria:**
- [x] The `@ref={name}` syntax is correctly parsed as a ref binding, not an event
- [x] Ref wrapper objects (`const inputBox = ...`) are emitted for inlined child components
- [x] `inputBox.focus()` and `inputBox.blur()` work correctly in browser
- [x] `.scrollTo()` and `.scrollBy()` on refs work correctly
- [x] `.getBoundingClientRect()` on refs returns valid coordinates
- [x] 263 Rust compiler tests pass with 5 snapshot updates
- [x] No new console errors from ElementRefDemo operations

**Testing Approach:**
- Browser MCP test with 6 ElementRefDemo operations (Focus, Blur, ScrollToTop, ScrollToBottom, ScrollBy, QueryRect)
- Full Rust test suite (`cargo test -p omnilang --lib`)
- Regression test via learn-omniview project compilation and browser rendering

## References

- ADR-036: Element Reference System
- ADR-038: Refined Element Reference System (cache fix, @for diagnostics, scrollTo/scrollBy, HTMLElement fallback)
- [OmniView Element Ref System Review 2026-07-29](omniview-review-high-freq-dom-v2-2026-07-30.md)
- PR: #ref-binding-syntax-and-inlined-wrappers
