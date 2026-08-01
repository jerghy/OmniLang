# ADR-024: React Motion useEffect Lifecycle Split

## Status

Superseded by [ADR-052](ADR-052-semantic-level-dsl-cross-target-codegen.md)（2026-08-01，见 [ADR-054](ADR-054-omniview-semantic-dsl-era-design-philosophy.md)）

> React 动画重写（native refs + WAAPI、8 触发器、无 DOM 扫描）取代了 useEffect 依赖数组机制；其「避免依赖膨胀」的意图保留在新实现中。原文保留作历史参考。

## Context

The 2026-07-28 architecture review found a bug in the React codegen's motion animation system. The generated `useEffect` for `@enter`/`@hover`/`@press`/`@focus` triggers included ALL component signal names in its dependency array:

```tsx
// Buggy: re-runs on every state change
useEffect(() => {
  document.querySelectorAll('[data-motion-enter]').forEach(el => { animate(el, kf, opts); });
  document.querySelectorAll('[data-motion-hover]').forEach(el => { el.addEventListener('mouseenter', ...); });
}, [count, isOpen]); // ALL signals as deps
```

This caused two problems:
1. `@enter` animations replayed on every state change (should be mount-only)
2. `addEventListener` calls leaked — each re-render added new listeners without removing old ones

The source was a deliberate comment in the code: "Include signal names in dep array so @enter re-triggers on state changes" — which created the very bugs it described.

## Decision Drivers

- **Correctness**: `@enter` animations should fire once on mount. Interactive triggers need cleanup.
- **Memory leak**: Event listener accumulation on every render cycle.
- **UX**: Visual flicker from replaying entrance animations on unrelated state changes.
- **React conventions**: `useEffect` with empty deps for mount-only effects; cleanup functions for subscriptions.

## Decision

**Split the single motion effect into two separate `useEffect` blocks:**

1. **@enter effect** with `[]` deps — fires once on mount, no cleanup needed
2. **Interactive triggers (@hover, @press, @focus) with `[]` deps** — uses named handler references stored in per-trigger-type arrays, with a cleanup function that calls `removeEventListener` on unmount

If a legitimate `@enter` re-trigger use case exists (e.g., list items being added dynamically), it should be handled through `key` props or a dedicated mechanism — not by including all state in the dependency array.

**Architectural Components Affected:**
- `omnilang/src/codegen/react/motion/mod.rs` — `emit_motion_effects_tsx` refactored (60 insertions, 28 deletions)
- `omnilang/src/codegen/react/component/mod.rs` — caller updated (1 line: removed `signal_names` argument)

**Interface Changes:**
- `emit_motion_effects_tsx` signature: removed `signal_names: &FxHashSet<String>` parameter

## Consequences

### Positive

- @enter animations fire exactly once (correct behavior)
- No event listener leaks — cleanup function removes all listeners
- Generated code follows React best practices
- Separation of mount-only vs. interactive concerns

### Negative

- Two `useEffect` blocks instead of one (marginally more code in generated output)
- If a future use case genuinely needs state-dependent @enter replay, a new mechanism is required

### Neutral

- Existing tests didn't cover motion output (all test fixtures use `motion: None`) — no test gap was created, but the gap was exposed

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low. Only affects generated React component output for components that use motion triggers.

**Affected Components**:
- React codegen motion module — changed how effects are structured
- React codegen component module — removed one unused parameter

**Risk Mitigation**:
- 235 React codegen tests pass with unchanged output (no motion in test fixtures)
- Generated code is valid React — no runtime framework changes
- The two-effect structure is a well-established React pattern

### Reversibility

**Reversibility Level**: High. Reverting to the single-effect approach is a codegen change, not an architectural commitment.

## Alternatives Considered

### Keep single effect but use refs for state-dependent triggers

Use `useRef` for the mounted state and only run @enter on first render.

**Pros:**
- Single `useEffect` block
- Can handle state-dependent re-triggers if needed

**Cons:**
- More complex generated code (refs + conditional logic)
- Doesn't solve the listener leak — still needs cleanup
- Over-engineered for the mount-only use case

### Keep the old behavior, document as "intentional"

Accept that @enter replays on every state change.

**Pros:**
- No code change

**Cons:**
- Clearly wrong — @enter means "enter the DOM", not "state changed"
- Listener leak is a real memory problem
- Violates React's rules of hooks
- The original comment justifying it was contradicted by the bug symptoms

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This is a bug fix, not an architectural change. The fix uses standard React patterns (empty deps + cleanup) — no new abstractions.

**Proposed Decision**: "Split motion useEffect into two separate effects: mount-only @enter and interactive triggers with cleanup."

**Necessity Assessment**: 10/10
- **Current need**: Bug causing visual flicker and memory leaks. Confirmed by code audit.
- **Future need**: Every future component with motion triggers would inherit the same bugs.
- **Cost of waiting**: Production components replay animations on every interaction + leak event listeners.
- **Evidence of need**: Code audit confirmed the incorrect dependency array pattern and missing cleanup.

**Complexity Assessment**: 1/10
- **Added complexity**: None — the fix uses standard React patterns. The codegen output is simpler (two clean effects instead of one messy one).
- **Maintenance burden**: Lower — the two-effect split is self-documenting.
- **Learning curve**: None — standard React.
- **Dependencies introduced**: None.

**Alternative Analysis**: The "keep old behavior" alternative is unacceptable (known memory leak). The "refs" alternative is over-engineered for the mount-only case.

**Simpler Alternative Proposal**: None simpler than `[]` deps + cleanup function. This is the minimal fix.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 1/10
- **Ratio**: 0.10 (Target: <1.5)

**Overall Assessment**: Textbook bug fix. The "fix" is actually simpler than the "bug" — one messy effect with all-signals deps becomes two clean effects with empty deps.

## Validation

**Acceptance Criteria:**
- [x] @enter effect uses `[]` deps (mount-only)
- [x] Interactive triggers use cleanup function with `removeEventListener`
- [x] No `signal_names` parameter dependency in motion effect generation
- [x] All 235 React codegen tests pass
- [x] No new snapshot diffs (no motion tests existed)
- [ ] Future: add motion-specific integration tests (not in scope for this fix)

**Testing Approach:**
- Regression: all 235 React codegen tests pass
- Manual verification: inspect generated TSX for correct useEffect structure
- Gap: no automated test that verifies the generated motion effect output (test fixtures use `motion: None`)

## References

- [OmniView Multi-Perspective Review 2026-07-28](../../reviews/omniview-multi-perspective-review-2026-07-28.md) — H1 finding
- [OmniLang Animation](../../../memory/omnilang-animation.md) — animation system architecture
