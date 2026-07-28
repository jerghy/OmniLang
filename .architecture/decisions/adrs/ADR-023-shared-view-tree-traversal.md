# ADR-023: Shared ViewTree Traversal Utility

## Status

Implemented

## Context

The 2026-07-28 architecture review identified 8 nearly-identical recursive tree-walking functions across 3 codegen backends:

| Backend | Functions | Purpose |
|---------|-----------|---------|
| OmniView | `view_tree_has_motion` | Check if any element has motion |
| React | `has_driven_by`, `has_inview_trigger`, `has_scroll_trigger`, `has_drag_trigger` | Check for specific motion triggers |
| Flutter | `has_trigger_kind_in_view`, `has_trigger_kind_scroll`, `has_trigger_kind_drag` | Check for specific trigger types |

All 8 functions shared identical tree-walking structure:
1. Match `ViewTree::Element(el)` → apply predicate, recurse into children
2. Match `ViewTree::Conditional(cond)` → recurse into branches
3. Match `ViewTree::ListRendering(list)` → recurse into body/empty_body
4. Base cases: `Text`, `Interpolation`, `Slot` return false

The only difference was the predicate applied to each `ElementIR`.

## Decision Drivers

- **DRY violation**: 8 copies of the same recursive tree-walking logic
- **Drift risk**: A new `ViewTree` variant added in one backend but missed in another
- **Consistency**: Each backend should traverse the tree identically — no backend-specific shortcuts
- **Testability**: A shared implementation can be tested once instead of per-backend

## Decision

**A single `view_tree_any()` function in `omnilang_core::codegen::shared::view_tree` replaces all 8 duplicate tree-walkers.** The function accepts a predicate closure: `pub fn view_tree_any(view: &ViewTree, predicate: impl Fn(&ElementIR) -> bool) -> bool`.

**Architectural Components Affected:**
- `crates/omnilang-core/src/codegen/shared/view_tree.rs` — new shared function
- `omnilang/src/codegen/omniview/mod.rs` — replaced `view_tree_has_motion` (24 → 4 lines)
- `omnilang/src/codegen/react/motion/transforms.rs` — replaced 4 functions (~100 → 4 lines)
- `omnilang/src/codegen/flutter/motion_widgets.rs` — replaced 3 functions (~90 → 3 lines)

**Interface Changes:**
- New public API: `omnilang_core::codegen::shared::view_tree::view_tree_any(view, predicate) -> bool`
- Internal `view_tree_any_dyn` helper for type erasure (avoids Rust recursive `impl Fn` type explosion)

## Consequences

### Positive

- Net -119 lines (8 functions → 1 shared + 4 thin wrappers)
- Single point of truth for ViewTree traversal logic
- New `ViewTree` variant only needs one update
- All backends traverse identically

### Negative

- Generic predicate creates deeply nested `impl Fn` types through recursion — requires internal `dyn Fn` dispatch layer
- Slight runtime overhead from `dyn` dispatch (negligible — tree traversal is not a hot path)

### Neutral

- Backends still have thin wrapper functions for readability (e.g., `view_tree_has_motion` → `view_tree_any(view, |el| el.motion.is_some())`)

## Implementation Strategy

### Blast Radius

**Impact Scope**: Moderate (3 backends, 1 shared module). Changes are localized to motion-scanning functions.

**Risk Mitigation**:
- Existing tests for all 3 backends verify behavioral equivalence
- `view_tree_any_dyn` internal helper prevents Rust type recursion issues
- The abstracted pattern (recursive walk + predicate) is well-understood

### Reversibility

**Reversibility Level**: High. If the shared function proves problematic, each backend can re-inline its tree-walker. The shared API surface is minimal.

## Alternatives Considered

### Keep separate functions, add doc comment referencing each other

Accept the duplication and use cross-references to keep them in sync.

**Pros:**
- No refactoring risk
- Backend-specific optimizations possible

**Cons:**
- Drift risk remains
- 8 copies to update for each `ViewTree` change
- Doc comments don't enforce consistency

### Use a macro instead of a generic function

Define `view_tree_any!` as a declarative macro that expands at each call site.

**Pros:**
- No `dyn Fn` overhead
- Compile-time expansion

**Cons:**
- Harder to read/debug
- Still requires `impl Fn` passing between macro invocations
- Macro hygiene issues with recursive calls

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This is a code deduplication — the standard Extract Method refactoring applied at crate scale. Minimal complexity increase.

**Proposed Decision**: "A single view_tree_any() in omnilang-core replaces 8 duplicate tree-walkers."

**Necessity Assessment**: 5/10
- **Current need**: Moderate — the 8 functions were stable and correct. Drift risk is theoretical.
- **Future need**: New `ViewTree` variants will need tree-walking. Single update point is valuable.
- **Cost of waiting**: If a new variant is added (e.g., `ViewTree::Portal`), 3 backends would need updating.
- **Evidence of need**: 8 duplicate functions confirmed in audit.

**Complexity Assessment**: 3/10
- **Added complexity**: One generic function with internal `dyn Fn` layer. Somewhat unintuitive but well-documented.
- **Maintenance burden**: Lower than before (1 edit vs 3-4 edits for new ViewTree variants).
- **Learning curve**: Low — the predicate pattern is standard Rust.
- **Dependencies introduced**: None (within existing `omnilang-core`).

**Alternative Analysis**: The "keep copies" alternative defers the maintenance cost to future `ViewTree` changes. The macro alternative would have similar complexity with worse readability. The chosen approach balances maintainability with minimal complexity.

**Simpler Alternative Proposal**: None simpler than a single generic function. The `dyn Fn` internal layer is a Rust language constraint, not architectural complexity.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 5/10
- **Complexity**: 3/10
- **Ratio**: 0.60 (Target: <1.5)

**Overall Assessment**: Good extract-method-at-scale. The benefit-to-cost ratio is favorable given the multi-backend architecture.

## Validation

**Acceptance Criteria:**
- [x] All 8 original tree-walking functions replaced with shared `view_tree_any`
- [x] All 423 backend tests pass (behavioral equivalence confirmed)
- [x] React motion trigger detection unchanged
- [x] Flutter motion trigger detection unchanged
- [x] OmniView motion detection unchanged
- [ ] Future: new `ViewTree` variant added → only `view_tree_any` needs updating

**Testing Approach:**
- Regression tests for all 3 backends' motion systems verify behavioral equivalence
- New `ViewTree` variants should add a test case to `view_tree_any` tests

## References

- [OmniView Multi-Perspective Review 2026-07-28](../../reviews/omniview-multi-perspective-review-2026-07-28.md) — M13 finding
- [OmniView Compiler Internals](../../../memory/omniview-compiler-internals.md) — ViewTree architecture
