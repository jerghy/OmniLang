# ADR-040: IR Enum `#[non_exhaustive]` Strategy

## Status

Implemented

## Context

The OmniLang compiler generates code for 10 backends (React, Flutter, OmniView, Vue, Dioxus, Dart, Python, JavaScript, Rust, and JSON). All backends pattern-match on shared IR enum types defined in `crates/omnilang-core/src/ir/`:

- `ExprIR` (22 variants) — expression IR nodes
- `StatementIR` (13 variants) — statement IR nodes
- `ViewTree` (7 variants) — view tree nodes
- `MutationIntent` (3 variants) — signal mutation operations

Adding a single new variant to any of these enums causes cascading compilation failures across all 10 backend crates because each backend exhaustively matches on these types. The `test-all-backends` CI job is non-blocking, meaning these failures are discovered post-merge, requiring multi-hour coordination across all backends.

Conversely, two enums — `TypeKind` and `EventDataKind` — had `#[non_exhaustive]` applied despite having zero external API consumers (all consumers are within the same workspace crate).

The 2026-07-30 architecture v2 review (11 specialists) identified this as the single highest blast-radiation risk in the codebase, with 5 of 11 specialists independently flagging the gap.

## Decision Drivers

* **Blast radius reduction**: Prevent a single variant addition from silently breaking 10 backends. Adding `#[non_exhaustive]` converts hard compilation errors into manageable `_` catch-all arms, giving each backend time to implement new variant handling independently.
* **Consumer profile alignment**: Apply `#[non_exhaustive]` only where there are external consumers. Remove it from enums that have only internal (same-crate) consumers, where Rust's exhaustiveness checking is more valuable than forward-compatibility.
* **CI safety**: The `test-all-backends` CI job is non-blocking. Without `#[non_exhaustive]`, variant additions break the build post-merge; with it, backend maintainers can update their catch-all arms at their own pace.
* **Exhaustiveness preservation for internal types**: For enums with no external consumers, `#[non_exhaustive]` removes the compiler's ability to enforce exhaustive matches without providing API stability benefits.

## Decision

Apply `#[non_exhaustive]` to IR enums that have **cross-crate consumers** (backends in separate crates), and remove it from IR enums that have **only same-crate consumers**.

**`#[non_exhaustive]` ADDED to:**

| Enum | Variants | External Consumers | Rationale |
|------|----------|-------------------|-----------|
| `ExprIR` | 22 | 10 backends | Protects all backends from new expression variant breakage |
| `StatementIR` | 13 | 10 backends | Protects all backends from new statement variant breakage |
| `ViewTree` | 7 | 10 backends | Protects all backends from new view tree variant breakage |
| `MutationIntent` | 3 | All backends that process signals | Protects signal processing backends from new mutation variant breakage |

**`#[non_exhaustive]` REMOVED from:**

| Enum | Rationale |
|------|-----------|
| `TypeKind` | Zero external consumers; all match sites are in `omnilang-core` crate. Exhaustiveness checking is more valuable here. |
| `EventDataKind` | Zero external consumers; all match sites are internal. Exhaustiveness checking is more valuable here. |

**Architectural Components Affected:**
* `crates/omnilang-core/src/ir/types.rs` — `ExprIR`, `StatementIR`, `ViewTree`, `TypeKind`
* `crates/omnilang-core/src/ir/signals.rs` — `MutationIntent`
* `crates/omnilang-core/src/ir/events.rs` — `EventDataKind`
* All 10 backend codegen crates — catch-all arms in match statements (33 total match sites)

**Interface Changes:**
* Backends must now include `#[allow(unreachable_patterns)] _ => unreachable!("Unknown IR variant — update codegen for new variant")` in their `ExprIR`, `StatementIR`, and `ViewTree` match statements.
* This is a **compile-time-only** change. Serialization format (`#[serde(tag = "_kind")]`) is unaffected.

## Consequences

### Positive

* **Variant additions no longer break all backends simultaneously.** A new `ExprIR` variant compiles everywhere; backends that don't handle it will hit the `unreachachable!()` at runtime during testing, not at compile time during CI.
* **Backend maintainers can update independently.** The Flutter maintainer can add support for a new variant without coordinating with the React maintainer.
* **6 paused backends (Vue, Dioxus, Dart, Rust, Python, JavaScript) are protected.** These ~7,500 lines of feature-gated code won't break when the IR evolves, reducing maintenance friction.
* **Internal enums regain exhaustiveness checking.** Removing `#[non_exhaustive]` from `TypeKind` and `EventDataKind` means adding a variant to these enums produces compile errors at all match sites — the Rust compiler enforces completeness for internal types.

### Negative

* **Runtime panics replace compile-time errors for unhandled variants.** If a backend's catch-all `unreachachable!()` is hit at runtime, the failure is later and less visible than a compile error. Mitigation: the `unreachachable!()` message includes the enum name and a clear instruction to update the codegen. Future work could add a CI job that scans for `unreachable!()` calls triggered in tests.
* **33 catch-all arms added across 10 backends.** Each adds a `#[allow(unreachable_patterns)]` line that will need updating when variants are added. However, this is a one-time cost and the pattern is mechanically consistent.
* **`unreachable!()` arms are not tested by default CI.** The feature-gated backends (Vue, Dioxus, etc.) are not compiled in default CI. Catch-all arms in these backends could silently break. Mitigation: `cargo check --all-features` verified during this implementation.

### Neutral

* Serialization behavior is unchanged (`#[serde(tag = "...")]` attributes are unaffected by `#[non_exhaustive]`).
* The `#[allow(unreachable_patterns)]` attribute is needed because the compiler sees the wildcard as unreachable when all current variants are already covered — this is semantically correct (the wildcard is for *future* variants, not current ones).

## Implementation Strategy

### Blast Radius

**Impact Scope**: Moderate. 4 enum definitions changed + 33 catch-all arms added across 10 backend crates. Zero behavioral change to generated code.

**Affected Components**:
- `crates/omnilang-core/src/ir/` — 3 files, 6 enums (4 add, 2 remove)
- `omnilang/src/codegen/` — 10 backend directories, 22 files, 33 new catch-all arms
- `omnilang/tests/` — No test changes needed (existing tests cover all current variants)

**Risk Mitigation**:
- All 852+ workspace tests pass with zero regressions
- `cargo check --all-features` verified for non-default backends (Vue, Dioxus)
- `cargo clippy --all-features` verified clean
- Snapshots unchanged (generated code identical for all current variants)

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**:
- Fully reversible: remove `#[non_exhaustive]` attributes and catch-all arms in a single commit
- No data migration, no serialization changes, no behavioral changes
- Rollback cost: ~30 minutes (mechanical undo of 6 attribute changes + 33 line deletions)

**Options Preserved**:
- Future decision: migrate back to exhaustive-only matching if all backends move back into the same crate
- Per-enum granularity: can add/remove `#[non_exhaustive]` on individual enums independently

**Commitments Made**:
- New IR variant additions must still be announced (so backend maintainers can update their catch-all arms)
- Catch-all `unreachable!()` arms create runtime panics on unhandled variants — acceptable trade-off

### Sequencing & Timing

**Prerequisites**:
- [x] 11-specialist architecture review completed (2026-07-30 v2)
- [x] All 852+ workspace tests passing
- [x] `cargo check --all-features` passing for feature-gated backends

**System Readiness**: Ready. Change is purely compile-time; no runtime behavior affected.

**Team Readiness**: High. Pattern (`#[allow(unreachable_patterns)] _ => unreachable!(...)`) is mechanically consistent across all backends and documented in this ADR.

### Social Cost

**Learning Curve**: Low. `#[non_exhaustive]` is a standard Rust attribute. The catch-all pattern is mechanically identical across all 33 match sites.

**Cognitive Load**: Minor increase. Each match statement now has one extra `#[allow(unreachable_patterns)]` arm. The arm is self-documenting ("Unknown IR variant — update codegen for new variant").

## Alternatives Considered

### Alternative 1: Do Nothing (No `#[non_exhaustive]`)

Keep the current state where all backends exhaustively match on every IR enum variant.

**Pros:**
* Compile-time correctness — the compiler enforces that every backend handles every variant
* No catch-all arms, no `unreachable!()` calls
* Simpler mental model (one source of truth)

**Cons:**
* Every variant addition breaks all 10 backends simultaneously
* 6 paused backends incur the same compilation tax as 3 active ones
* Multi-hour coordination burden for variant additions
* `test-all-backends` CI job is non-blocking — failures discovered post-merge

### Alternative 2: Migrate All Backends Into the Same Crate

Move all backend codegen into `omnilang-core` so that match exhaustiveness is enforced across the entire workspace without `#[non_exhaustive]`.

**Pros:**
* Compile-time exhaustiveness preserved
* No `#[non_exhaustive]` or catch-all arms needed
* All backends evolve together

**Cons:**
* Massive crate restructuring (~22,000 lines of codegen moving between crates)
* Defeats P35 crate split (which was specifically done to reduce coupling)
* Compile times increase for all backends
* WASM binary size increases (currently only OmniView is compiled for WASM)

### Alternative 3: CI-Only Enforcement

Keep exhaustive matching but make `test-all-backends` a blocking CI job.

**Pros:**
* Compile-time correctness preserved
* No code changes needed

**Cons:**
* Every PR that adds an IR variant must also update all 10 backends in the same PR
* 6 paused backends add ~7,500 lines of required changes per variant addition
* Doesn't address the coordination burden — just makes the failure blocking instead of non-blocking

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Overall Decision Complexity Assessment**:
This is a minimal, targeted application of an existing Rust language feature to solve a demonstrated, recurring problem. The change is 6 attributes and 33 catch-all arms — no new abstractions, no new dependencies, no behavioral changes. It represents appropriate engineering for a real maintenance burden, not speculative complexity.

**Necessity Assessment**: 8/10
- **Current need**: The 11-specialist review identified this as the highest blast-radiation risk in the codebase. Five specialists independently flagged it. Two prior variant additions (BlockStmtIR to StatementIR, a new ViewTree variant) caused multi-hour coordination across backends.
- **Future need**: IR development is ongoing (new expression types for pattern matching, async support). Each new variant triggers this problem.
- **Cost of waiting**: Low today (no variant addition in progress), but the risk is constant. Each variant addition costs hours of coordination. The fix is 30 minutes of work.
- **Evidence of need**: Documented in the 2026-07-30 architecture v2 review (`component-omniview-2026-07-30-v2.md`, Cross-Cutting Theme 1). Five specialists confirmed.

**Complexity Assessment**: 2/10
- **Added complexity**: Six attributes. Thirty-three mechanically identical catch-all arms. Zero new abstractions.
- **Maintenance burden**: When a new variant is added, the `unreachable!()` arm in the relevant backend will fire — the developer sees the error message and adds the variant. This is marginally more work than a compile error but eliminates the coordination burden.
- **Learning curve**: Standard Rust attribute. No new patterns or concepts.
- **Dependencies introduced**: None.

**Alternative Analysis**:
The alternatives (do nothing, crate merge, CI-only enforcement) were all considered and rejected for concrete reasons. Each has a higher complexity-to-necessity ratio than the chosen approach.

**Simpler Alternative Proposal**:
The chosen approach IS the simplest alternative. "Do nothing" is simpler in code but more complex in process (multi-hour coordination per variant addition). The `#[non_exhaustive]` addition is the minimal change that solves the demonstrated problem.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 2/10
- **Ratio**: 0.25 (Target: <1.5)

**Overall Assessment**:
Strongly appropriate. This is textbook pragmatism: a 30-minute change using a standard language feature to solve a demonstrated, recurring, multi-hour coordination problem. The change has zero behavioral impact, zero new dependencies, and is trivially reversible.

## Validation

**Acceptance Criteria:**
- [x] `cargo test --workspace` passes (852+ tests, 0 failures)
- [x] `cargo check --all-features` passes (all feature-gated backends compile)
- [x] `cargo clippy --all-features -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] All existing snapshots unchanged (generated code identical for all current variants)
- [x] Catch-all arms added to all 33 cross-crate match sites across 10 backend crates

**Testing Approach:**
* Existing test suite (852+ tests) validates that all current IR variants are handled correctly
* `cargo check --all-features` validates that catch-all arms exist in all feature-gated backends
* Future: add a CI job that verifies `#[non_exhaustive]` coverage on all IR enums with external consumers

## References

* [Architecture Review v2 — Cross-Cutting Theme 1](../reviews/component-omniview-2026-07-30-v2.md) — 5-specialist finding
* [Implementation Plan — Task 1](../../../docs/superpowers/plans/2026-07-30-omniview-architecture-v2-fixes.md) — Task 1 details
* [Commit 612cedae](https://github.com/omnilang/omnilang/commit/612cedae) — Implementation
* [Commit 1f6fa970](https://github.com/omnilang/omnilang/commit/1f6fa970) — Fix wave (unreachable wildcard + Vue/Dioxus test type mismatch)
* [P35 Crate Split ADR](./ADR-001-compiler-pipeline-architecture.md) — Motivation for crate boundaries that make `#[non_exhaustive]` necessary
