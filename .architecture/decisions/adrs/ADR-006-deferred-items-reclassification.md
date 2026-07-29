# ADR-006: Deferred Items Reclassification — Immediate Execution

## Status

Accepted

## Context

The 2026-07-26 architecture review identified 5 items as deferred (recorded in `.architecture/deferrals.md`):

- **P35**: Crate splitting (deferred: "YAGNI, 63K lines no pain")
- **P36**: Codegen parallelization (deferred: "no profiling data")
- **P38b**: Strip TAURI_INTERNALS (deferred: "defensive premature, wait for sandbox")
- **P39**: IR optimization passes (deferred: "needs motion 95% branch coverage first")
- **P34**: AI code completion (removed from plan: "new feature, not debt")

The 2026-07-27 follow-up review (11-expert re-review + 6-expert deep technical review) reassessed each item against current codebase state and user direction.

## Decision Drivers

1. **Compiler scale reached**: 63K lines in single crate; IR changes cascade to all backends. Deferred to 100K threshold is accumulating debt.
2. **Codegen single-threaded**: rayon parallelism exists in parse/semantic but zero in codegen. Growing projects will hit latency walls.
3. **Sandbox validated**: P38 iframe sandbox (`sandbox="allow-scripts"`, no `allow-same-origin`) is functional. Defense hardening can proceed.
4. **Motion test baseline exists**: 13 regression tests in place, covering ~50-60% branches. Test gap to 95% is quantified (7 specific tests).
5. **User direction**: "推迟的项目现在就要完成" (deferred items must be completed now). P34 stays deferred: "AI 代码补全先不做."

## Decision

**P35, P36, P38b, P39 are reclassified from DEFERRED to ACTIVE.** Each has a concrete implementation plan:

| Item | Approach | Phase | Effort |
|------|----------|-------|--------|
| P35 | Phase 1: `omnilang-core` + `omnilang-codegen-omniview` + facade. 7 paused backends remain feature-gated. | Week 2-3 | 1-2 days |
| P36 | Cross-backend parallelization via `compile_multi_target` + rayon. Zero backend changes. | Week 2-3 | ~50 lines |
> **⚠️ 2026-07-29 更新**：P36 的 `compile_multi_target` 方案已在 ADR-009 中实施，后因 YAGNI 在 commit 9d30834 中移除。单后端 WASM 编译路径才是实际瓶颈；多目标并行化无 profiling 数据支撑。
| P38b | Eliminate XSS chain (innerHTML→textContent + origin validation + capability tightening) instead of stripping TAURI_INTERNALS. | Week 1 | 1-2 hours |
| P39 | MVP: dead computed elimination (~200 lines) after IR merge. Full optimization deferred until motion 95% coverage. | Week 2-4 | 2-3 days |

**P34 remains DEFERRED.** No timeline change.

**Architectural Components Affected:**
- `omnilang` crate → split into `omnilang-core` + per-backend crates
- `omnilang/src/codegen/mod.rs` → new `compile_multi_target` entry point
- `omnilang-core/src/ir/` → new `optimize.rs` module
- `poiesis/src-tauri/debug-window/index.html` → innerHTML→textContent + origin validation
- `poiesis/src-tauri/capabilities/debug-window.json` → atomic permissions

## Consequences

### Positive

- **P35**: Each codegen backend independently buildable/testable. IR change blast radius reduced from 9 backends to 1 core crate.
- **P36**: 1.5x-2x codegen speedup for multi-target CI/toolchain builds.
- **P38b**: XSS upgrade chain eliminated. Debug window capability reduced from 9 bundles to 2 atomic permissions.
- **P39**: Unused computed properties eliminated at IR level, reducing runtime overhead.

### Negative

- **P35**: ~15-20 `pub(crate)` items promoted to `pub`. Facade crate adds maintenance layer. Phase 2 (remaining backends) deferred indefinitely.
- **P36**: Only benefits multi-target scenarios. Single-backend editor path unaffected.
- **P38b**: Cannot fully strip TAURI_INTERNALS (needed for debug window IPC transport).
- **P39**: Signal elimination deferred until motion coverage reaches 95%. `ComputedIntent.deps` tracking remains unimplemented.

### Neutral

- P34 stays deferred — no AI features added.
- P39 dead computed elimination is ~200 lines; full optimization pass is a separate Phase 2 effort.

## Implementation Strategy

### Blast Radius

**Impact Scope**: These 4 items touch Rust compiler core, codegen pipeline, Tauri security, and CI. All are reversible independently.

**Affected Components**:
- `omnilang/` crate structure (P35) — high blast radius (file moves + visibility changes)
- `omnilang/src/codegen/` (P36) — low blast radius (new function, no existing code changed)
- `poiesis/src-tauri/debug-window/` (P38b) — low blast radius (single HTML file)
- `omnilang-core/src/ir/` + `analyse.rs` (P39) — medium blast radius (new module + 1 call site)

### Sequencing

```
Week 1: P38b (security — P0) → Phase 2 Tasks 2.1-2.3
Week 2-3: P35 + P36 + P39-MVP → Phase 3 Tasks 3.1-3.3
Week 4: P39 motion tests → Phase 4 Task 4.1
```

### Reversibility

- P35: Reversible — merge facade crate back into single crate if split causes issues.
- P36: Reversible — remove `compile_multi_target` entry point.
- P38b: Reversible — revert to innerHTML if textContent causes display issues (unlikely).
- P39: Reversible — remove optimization pass; dead computed elimination is pure removal, no semantic change.

## Alternatives Considered

### Keep all items deferred
**Rejected**: User directive. Compiler scale and security audit findings demand action now.

### Full TAURI_INTERNALS strip (original P38b scope)
**Rejected**: Debug window requires event IPC for communication with main app. Sandbox already prevents user code from accessing it. XSS chain fix is more effective defense.

### Per-component parallelization (P36 alternative)
**Rejected**: Requires changes to every backend's codegen internals. Cross-backend parallelization achieves similar speedup with zero backend changes.

### Full signal elimination (P39 original scope)
**Rejected**: Requires motion 95% branch coverage and dependency tracking infrastructure. Dead computed elimination is a safer, smaller first step.

## Pragmatic Enforcer Analysis

**Mode**: Balanced

| Item | Necessity | Complexity | Ratio | Verdict |
|------|-----------|------------|-------|---------|
| P35 Crate split | 7/10 | 6/10 | 0.86 | ✅ Approved — scale justifies split |
| P36 Parallelization | 5/10 | 2/10 | 0.40 | ✅ Approved — low complexity, tangible benefit |
| P38b Security | 8/10 | 2/10 | 0.25 | ✅ Approved — P0 security, trivial fix |
| P39 MVP | 6/10 | 3/10 | 0.50 | ✅ Approved — ~200 lines, dead code removal only |
| P34 AI completion | 2/10 | 7/10 | 3.50 | 🔴 Deferred — high complexity, low necessity now |

**Overall**: All 4 reclassified items have favorable ratio (<1.5). P34 correctly deferred (ratio 3.50).

## Validation

**Acceptance Criteria**:
- [ ] P35: `cargo build --workspace` succeeds with split crate structure
- [ ] P36: `cargo test` passes with `compile_multi_target` producing correct output for 3 P0 targets
- [ ] P38b: XSS chain eliminated (innerHTML→textContent, origin validation, atomic capabilities, postMessage origin)
- [ ] P39: Dead computed test passes; existing tests unaffected
- [ ] P34: No AI completion code added

## References

- [Full Workspace Re-Review 2026-07-27](../reviews/full-workspace-review-2026-07-27.md)
- [Deep Technical Review 2026-07-27](../reviews/deep-review-2026-07-27.md)
- [Implementation Plan](../../../docs/superpowers/plans/2026-07-27-architecture-remediation.md)
- [deferrals.md](../../deferrals.md)
- [[ADR-004]] — Original review conclusions (deferred items list)
- [[ADR-003]] — Codegen target prioritization
