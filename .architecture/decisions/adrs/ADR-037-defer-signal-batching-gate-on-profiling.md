# ADR-037: Defer Signal Batching — Gate on Profiling Evidence

**Date**: 2026-07-30
**Author**: Architecture Review (6-expert panel)

---

## Status

Accepted

## Context

The `.architecture/reviews/component-omniview-high-freq-dom.md` architecture review identified that OmniView's debug runtime uses synchronous, unbatchted signal propagation: each `signal.value = v` immediately iterates all subscribers and invokes each effect's `run()`. This was a deliberate design choice documented in ADR-028 D3 (2026-07-29).

The Performance Specialist raised a MEDIUM concern: at 60fps with 3+ animated properties, a single event handler setting multiple related signals causes N×M redundant effect executions (where N = signals set, M = effects per signal). Each execution writes to the DOM, potentially triggering layout recalculation.

The trade-off space:

| Approach | Pros | Cons |
|----------|------|------|
| Synchronous (current) | Predictable, debuggable, simple ~200-line runtime | Redundant DOM writes at high frequency |
| Microtask batching | Correct for animation-heavy UIs | Adds timing complexity, ~50-80 lines to runtime |
| requestAnimationFrame batching | Aligns with frame budget | Introduces 1-frame latency on all updates |

## Decision Drivers

- **ADR-028 D3**: Synchronous notification is the established baseline — any change must preserve the debug runtime's simplicity budget
- **No production use case**: OmniView is a development/debug target; production apps use React/Flutter backends with their own scheduling
- **Profiling-first principle**: Optimize only when measured, not on speculation
- **Measurability**: The cost of batching (added complexity) is known; the benefit (reduced jank) is unmeasured

## Decision

**Defer signal batching. Maintain synchronous subscriber notification as per ADR-028 D3.**

Gate implementation on concrete profiling evidence:
1. A real OmniView application (not a synthetic benchmark)
2. Measurable frame drops below 60fps attributable to redundant effect execution
3. Confirmation that the frame drops persist after optimizing other causes (layout thrashing, GC pressure)

When the gating conditions are met, implement microtask-based batching:

```js
let __omni_batchDepth = 0;
let __omni_pendingEffects = null;

function __omni_batch(fn) {
  __omni_batchDepth++;
  try { fn(); } finally {
    __omni_batchDepth--;
    if (__omni_batchDepth === 0 && __omni_pendingEffects) {
      const effects = __omni_pendingEffects;
      __omni_pendingEffects = null;
      effects.forEach(e => e.run());
    }
  }
}
```

This is a gated future decision — not an active implementation plan.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/runtime.rs:36-46` — synchronous `signal.set()` subscriber loop
- Event handler codegen — would need `__omni_batch()` wrapping on all event handler invocations

**Interface Changes:**
- None in current state
- If implemented: `__omni_batch(fn)` becomes public API, event handler codegen wraps handlers in batch calls

## Consequences

### Positive

- **Preserves simplicity**: The ~200-line runtime budget (ADR-028) remains intact; no scheduling complexity
- **Debuggability maintained**: `signal.set()` produces immediate, synchronous DOM results — developers can inspect the DOM after any state change
- **No premature optimization**: Resources are directed at measured problems, not speculative ones

### Negative

- **Animation-heavy components may jank**: At 60fps with 3+ animated properties, redundant effect executions compound. The cost is O(signals_set × effects_per_signal) per event handler
- **Eventual implementation cost**: When batching is needed, it requires changes to runtime, codegen, and all event handler emission paths

### Neutral

- **Existing per-property style effects** (style.rs:47/81/125) independently subscribe to signals — batching would reduce their overhead but is not required for correctness

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low. Deferring batching has zero code impact. Implementing it later would affect:
- Runtime: ~50 lines in `runtime.rs`
- Codegen: wrapping in `omniview/view/events.rs` (~10 call sites)
- All existing behavior preserved through backward-compatible API

**Risk Mitigation**:
- The gating conditions prevent implementation without evidence
- If never triggered (e.g., React backend becomes dominant), the complexity is never introduced

### Reversibility

**Reversibility Level**: High

- Current decision (defer) is trivially reversible: implement batching when triggered
- If batching is implemented later, the synchronous path can be preserved behind a flag
- Migration: additive API, no breaking changes

### Sequencing & Timing

**Prerequisites**:
- [ ] Real OmniView application with animation-heavy components
- [ ] Chrome DevTools Performance panel recording showing frame drops
- [ ] Flame chart confirming root cause is redundant effect execution (not GC, layout, or paint)
- [ ] Attempted optimization of other causes (per-property style effect consolidation per P1-3)

**Readiness Assessment**: Not ready — profiling data required before any implementation.

### Confidence Assessment

**Model Correctness Confidence**: High

- ADR-028 D3 was validated by the Pragmatic Enforcer (Necessity 10/10)
- The Performance Specialist's concern is well-founded but unmeasured
- The gating conditions ensure we only optimize when there's a real problem

**Assumptions**:
1. Production React/Flutter backends handle their own scheduling — **Validation**: confirmed, React uses fiber scheduling, Flutter uses widget rebuild optimization
2. OmniView remains primarily a debug/development target — **Validation**: confirmed in architecture review and codegen priority document

## Alternatives Considered

### A. Implement Microtask Batching Now

Implement `__omni_batch()` and wrap all event handlers immediately.

**Pros**: Would reduce redundant effect executions for all users
**Cons**: Violates "optimize only when measured" principle. Adds complexity to a deliberately simple runtime. No evidence that current performance is insufficient.

### B. Implement requestAnimationFrame Batching

Batch all signal updates to fire once per animation frame.

**Pros**: Aligns with frame budget, optimal for animations
**Cons**: Introduces 1-frame latency on ALL updates (not just animations). Breaks the synchronous debugging contract of ADR-028 D3.

### C. Per-Property Style Effect Consolidation (Complementary)

Merge per-element dynamic style properties into a single `__omni_effect()` per nid (P1-3 from architecture review).

**Pros**: Reduces redundant effect executions without changing the signal propagation model. Measurable benefit at codegen time.
**Cons**: Only addresses style properties, not text/class/attr bindings. Complementary to batching, not a replacement.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This is a textbook YAGNI decision. The signal batching optimization is a well-understood pattern (used by Vue, Solid, Preact), but OmniView's runtime is deliberately minimal (~200 lines). Implementing batching now would add ~25% to the runtime size for a benefit that has not been measured in any real application.

**Proposed Decision**: "Defer signal batching. Gate implementation on profiling evidence from a real OmniView application showing frame drops attributable to redundant effect execution."

**Necessity Assessment**: 3/10
- **Current need**: No known application experiencing jank from synchronous notification
- **Future need**: Moderate likelihood for animation-heavy apps, but timeframe is speculative
- **Cost of waiting**: Zero — batching is backward-compatible, can be added at any time
- **Evidence of need**: Performance Specialist identified theoretical concern; no profiling data exists

**Complexity Assessment**: 4/10
- **Added complexity**: ~50 lines of runtime code, ~10 codegen call site changes
- **Maintenance burden**: Low once implemented; batch/sync duality adds cognitive load
- **Learning curve**: Team must understand microtask timing vs synchronous execution
- **Dependencies introduced**: None — pure JS, no external libraries

**Alternative Analysis**:
- Alternative A (implement now) is premature optimization
- Alternative B (rAF batching) breaks the debugging contract
- Alternative C (per-property consolidation) should be done first regardless

**Recommendation**: ⏸️ Defer until triggered

**Justification**: The synchronous notification model serves OmniView's debug purpose correctly. The theoretical performance concern is real but unmeasured. Adding batching infrastructure now risks complexity for an unvalidated benefit. The gating conditions ensure implementation only when there is concrete evidence.

**Trigger conditions**:
1. A real OmniView application with 3+ animated properties updating at 60fps
2. Chrome DevTools flame chart showing redundant effect execution as the bottleneck
3. Frame drops below 60fps confirmed to be caused by signal propagation, not layout/paint/GC

**Pragmatic Score**:
- **Necessity**: 3/10
- **Complexity**: 4/10
- **Ratio**: 1.33 (within 1.5 threshold, but necessity is too low to justify)

**Overall Assessment**: Appropriate deferral. The gating conditions provide a clear path to implementation when needed, while protecting the runtime's deliberate simplicity.

## Validation

**Acceptance Criteria:**
- [ ] No batching code added to runtime unless gating conditions are met
- [ ] Per-property style effect consolidation (P1-3) explored before batching
- [ ] Profiling data collected from at least one real OmniView application before reopening

**Testing Approach:**
- When triggered: add `__omni_batch()` unit tests verifying deduplication
- Verify no regression in event handler timing (synchronous flush at end of microtask)
- Integration test: component with 3 animated properties, verify effect execution count

## References

- ADR-028 (OmniView Debug Runtime Trade-offs — D3: synchronous subscriber notification)
- `.architecture/reviews/component-omniview-high-freq-dom.md` — 2026-07-30 architecture review
- `omnilang/src/codegen/omniview/runtime.rs:36-46` — current synchronous signal.set() implementation
- `omnilang/src/codegen/omniview/view/style.rs:47/81/125` — per-property style effects
