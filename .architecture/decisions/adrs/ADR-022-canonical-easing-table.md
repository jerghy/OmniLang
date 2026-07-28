# ADR-022: Canonical Easing Table as Single Source of Truth

## Status

Implemented

## Context

The 2026-07-28 architecture review found that the same 30+ named easing-to-cubic-bezier mappings existed in three separate locations:

1. `packages/omni-animation/src/easing.ts` — JS runtime easing resolution
2. `omnilang/src/codegen/react/motion/keyframes.rs` — Rust codegen easing resolution
3. `crates/omnilang-core/src/semantic/motion.rs` — Semantic validation of easing names

Each copy had identical 30+ entries. Adding a new easing required synchronized updates across all three files. The JS runtime also had parameterized easing handling (`spring(t,f)`, `backIn(1.5)`) that the Rust codegen duplicated via `EasingIR::Spring`/`EasingIR::Back` variants with their own math.

## Decision Drivers

- **Maintenance burden**: Three copies of the same data, any change requiring triple-synchronized edits
- **Drift risk**: A developer adding a new easing in one location but not others produces hard-to-diagnose bugs (works in one target, silently broken in another)
- **Single source of truth**: The JS runtime (`@omni/animation`) is the authoritative source since it executes at runtime

## Decision

**The canonical easing table lives in `crates/omnilang-core/src/codegen/shared/easing.rs` as `NAMED_EASINGS` and `resolve_easing()`.** The JS runtime (`packages/omni-animation/src/easing.ts`) remains the authoritative source for parameterized easing (spring, backIn with parameters) which cannot be represented as static cubic-bezier values.

**Architectural Components Affected:**
- `crates/omnilang-core/src/codegen/shared/easing.rs` — new canonical table (26 entries)
- `omnilang/src/codegen/react/motion/keyframes.rs` — replaced 42-line `translate_named_easing` with delegate to shared
- `crates/omnilang-core/src/semantic/motion.rs` — replaced 30-line `matches!` macro with delegate to shared

**Interface Changes:**
- New public API: `omnilang_core::codegen::shared::easing::resolve_easing(name: &str) -> Option<&'static str>`
- New public constant: `omnilang_core::codegen::shared::easing::NAMED_EASINGS: &[(&str, &str)]`

## Consequences

### Positive

- Single source of truth for named easing mappings
- Adding a new easing requires only one edit
- Shared module guarantees consistency across semantic checker and all codegen backends
- Parameterized easing remains in JS runtime (appropriate — it requires runtime computation)

### Negative

- The shared table is a static array of 26 entries — not huge, but adds compile-time data
- `resolve_easing()` returns `Option<&str>` — consumers must handle the `None` case (passthrough unknown easings)

### Neutral

- The JS runtime retains its own `resolveEasing()` for parameterized easing — this is correct since `spring(t,f)` cannot be a static cubic-bezier value

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low. This is a data deduplication. The public API surface is a single function and a constant array.

**Affected Components**:
- Semantic validation — now uses shared table
- React codegen — now uses shared table
- Future backends — should use shared table

**Risk Mitigation**:
- Existing tests (`test_easing_all_variants`) confirm behavioral equivalence
- `NAMED_EASINGS` values were copy-pasted from the JS runtime (the authoritative source)
- kebab-case → camelCase normalization added in the React keyframes wrapper (JS uses camelCase identifiers, CSS easing names are kebab-case)

### Reversibility

**Reversibility Level**: High. If the shared table approach proves problematic, each consumer can re-inline its own copy. The shared API is a simple function call.

## Alternatives Considered

### Keep three copies, add CI check for consistency

Add a CI script that verifies all three easing tables are identical.

**Pros:**
- No refactoring needed
- Catches drift automatically

**Cons:**
- Still requires triple-synchronized edits
- CI failure doesn't tell the developer WHICH table is correct
- Doesn't scale — 4th backend = 4 copies, 5th = 5 copies

### Delegate easing resolution entirely to the JS runtime

Have the Rust codegen emit `resolveEasing("power1In")` calls instead of resolving at compile time.

**Pros:**
- Single source of truth (the JS runtime)
- Zero Rust-side duplication

**Cons:**
- Ties codegen output to runtime library (coupling)
- Not applicable for non-JS targets (Flutter, Dioxus, Rust, Python)
- Semantic checker needs runtime-unaware validation

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This is a data deduplication — the simplest possible fix. No new abstractions, no new dependencies.

**Proposed Decision**: "The canonical easing table lives in a shared Rust module."

**Necessity Assessment**: 6/10
- **Current need**: Moderate — the 3 copies were identical and stable. Drift risk is real but hasn't materialized yet.
- **Future need**: New backends (Rust, Python, Dart) will need easing resolution. Having it shared prevents per-backend copies.
- **Cost of waiting**: If a new easing is added to only 2 of 3 locations, users get inconsistent behavior across targets.
- **Evidence of need**: 3 identical copies confirmed in code audit.

**Complexity Assessment**: 1/10
- **Added complexity**: Near-zero — a `const &[(&str, &str)]` array and a 5-line function.
- **Maintenance burden**: Lower than before (1 edit vs 3).
- **Learning curve**: None — `resolve_easing("power1In")` is self-documenting.
- **Dependencies introduced**: None (already within `omnilang-core`).

**Alternative Analysis**: The "keep copies + CI check" alternative is simpler to implement but more complex to maintain. The chosen approach is simpler overall.

**Simpler Alternative Proposal**: None simpler than a single const array. This is the minimal viable deduplication.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 6/10
- **Complexity**: 1/10
- **Ratio**: 0.17 (Target: <1.5)

**Overall Assessment**: Textbook data deduplication — reduces maintenance burden with near-zero complexity cost.

## Validation

**Acceptance Criteria:**
- [x] `NAMED_EASINGS` contains all 26 entries from JS runtime
- [x] React `keyframes.rs` delegates to shared `resolve_easing()`
- [x] Semantic `motion.rs` delegates to shared `resolve_easing()`
- [x] All 423 backend tests pass
- [x] `test_easing_all_variants` passes
- [ ] Future: JS runtime and shared table stay synchronized (process: update JS runtime first, then sync to Rust)

**Testing Approach:**
- Unit test: `test_easing_all_variants` validates all 26 named easings produce correct cubic-bezier values
- Regression: all existing motion/animation tests pass unchanged

## References

- [OmniView Multi-Perspective Review 2026-07-28](../../reviews/omniview-multi-perspective-review-2026-07-28.md) — M7 finding
- [OmniLang Animation](../../../memory/omnilang-animation.md) — animation package architecture
