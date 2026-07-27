# ADR-009: P36 Codegen Parallelization — Cross-Backend Strategy

> **Superseded in remediation cleanup (commit 9d30834).** Rationale: single-backend WASM compilation path is the production bottleneck; multi-target parallelization via rayon is YAGNI until profiling data shows codegen is the bottleneck.
>
> **Superseded by**: Remediation cleanup (2026-07-27) — `compile_multi_target` removed as unused

## Status

Superseded

## Context

P36 (codegen parallelization) was deferred in the 2026-07-26 review: "no profiling data showing codegen is a bottleneck." The 2026-07-27 deep review reclassified it to immediate execution per user directive.

The current codegen pipeline (`omnilang/src/codegen/mod.rs:64-120`) compiles to a single `Target` per call via a sequential `match` block. rayon is used in `analyse.rs` (4 call sites for parse/semantic parallelism) but zero in codegen.

## Decision Drivers

1. **Thread safety verified**: All codegen structs carry `Send + Sync`. Zero `RefCell`/`Cell`/`Mutex` in entire `codegen/` directory.
2. **Pure functions**: `.generate(&self, &IRProgram, &FxHashMap) -> CompileOutput` — no shared mutable state.
3. **Backends are independent**: No cross-backend imports. Each backend only depends on shared codegen infrastructure (`codegen/shared/`, `codegen/traits.rs`).
4. **Current API limitation**: `compile()` accepts a single `Target`. Multi-target toolchains/CI must call sequentially.
5. **Per-component parallelization** (Option B) requires changing every backend's internals — higher risk, similar benefit.

## Decision

**Implement cross-backend parallelization via `compile_multi_target` entry point using rayon. Do not implement per-component parallelization at this time.**

### Approach

```rust
pub fn compile_multi_target(
    sources: FxHashMap<String, String>,
    targets: Vec<Target>,
    ...
) -> FxHashMap<Target, InternalCompileResult> {
    // Phase 1-3: Parse + analyse (already parallel internally)
    let analysis = analyse_multi(&sources, &path_mapping);
    
    // Phase 4: Codegen in parallel per target
    targets.par_iter().map(|&target| {
        (target, compile_single_target(&ir, &sources, target, ...))
    }).collect()
}
```

**Expected speedup**: 1.5x-2x for 3-target builds (OmniView + React + Flutter). Single-target editor path unaffected.

**Architectural Components Affected:**
- `omnilang-core/src/lib.rs` (or `omnilang/src/lib.rs`) — new `compile_multi_target` function
- `omnilang/src/lib.rs` — re-export in facade crate

## Consequences

### Positive

- **Zero backend changes**: All codegen implementations unchanged.
- **Progressive deployment**: Existing single-target API preserved. Multi-target is additive.
- **CI benefit**: `cargo test --test cross_target` compiles 51 fixtures × 3 targets = 153 compilations. Parallel codegen reduces this phase proportionally.
- **Verified safe**: `Send + Sync` at trait boundary. No locks needed.

### Negative

- **Single-target editor path unaffected**: P36 provides no benefit for the WASM editor preview use case (always OmniView only).
- **New API surface**: `compile_multi_target` must be maintained alongside existing `compile`/`compile_multi` functions.
- **Rayon overhead**: Thread pool initialization adds ~1ms for very small projects. Negligible vs. parse/semantic costs.

### Neutral

- Per-component parallelization (Option B) remains available as a future enhancement if profiling shows component-level bottlenecks within a single backend.
- CodegenCache (for incremental compilation) is orthogonal — addresses editor re-compile, not build throughput.

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low — new function added. No existing code paths changed.

**Affected Components**:
- `omnilang-core/src/lib.rs` — ~50 lines new function
- `omnilang/src/lib.rs` — 1 line re-export

### Sequencing

Phase 3 Task 3.2 in the implementation plan. Requires P35 Phase 1 (crate split) to be completed first so the new function lands in `omnilang-core`.

### Reversibility

Fully reversible — remove `compile_multi_target` function. No downstream code depends on it yet.

## Alternatives Considered

### Per-component parallelization (Option B)
**Rejected for now**: Requires threading `par_iter()` through `generate_per_component` in every backend. `build_component_map` clone overhead would need elimination first. Higher risk, similar benefit for multi-target cases. May revisit if single-backend codegen becomes a bottleneck.

### Async/streaming parallelization
**Rejected**: rayon is already a dependency (used in `analyse.rs`). Adding tokio for codegen parallelism would introduce a second async runtime. Rayon's work-stealing thread pool is a better fit for CPU-bound codegen.

### Keep sequential (status quo)
**Rejected**: User directive to execute P36 now. Thread safety verification confirms this is low-risk.

## Pragmatic Enforcer Analysis

**Necessity**: 5/10 — No current user complaint about codegen speed. But CI test cycle time is a developer experience metric. Low-cost investment for measurable improvement.

**Complexity**: 2/10 — ~50 lines of new code. No existing code changes. Rayon already a dependency.

**Ratio**: 0.40 ✅

**Recommendation**: ✅ Approved. Low complexity, tangible benefit, fully reversible.

## Validation

**Acceptance Criteria**:
- [ ] `compile_multi_target` produces correct output for all 3 P0 targets
- [ ] Output matches sequential `compile()` for each target individually
- [ ] `cargo test -p omnilang-core --lib` passes
- [ ] Cross-target test uses parallel path and passes

## References

- [Deep Technical Review 2026-07-27](../reviews/deep-review-2026-07-27.md) — Finding #5
- [Implementation Plan](../../../docs/superpowers/plans/2026-07-27-architecture-remediation.md) — Task 3.2
- [[ADR-006]] — Deferred items reclassification
