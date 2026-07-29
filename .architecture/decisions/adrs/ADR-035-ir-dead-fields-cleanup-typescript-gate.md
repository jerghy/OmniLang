# ADR-035: Remove Speculative IR Fields and Gating TypeScript Declarations

## Status

Implemented

## Context

The v4 10-perspective architecture review (2026-07-29) identified three items of accumulated debt in the OmniLang IR and codegen pipeline:

1. **`generic_params: Vec<String>` on `ComponentIR`** — zero consumers across all 9 codegen backends. No parser grammar exists for generic component parameters. Pure speculative generality, present since initial IR design.

2. **`if_mode_default: Option<String>` on `ComponentIR`** — parsed, lowered to IR, serialized, but **never read by any codegen backend**. Vestigial from animation features removed by ADR-032, but the field was not cleaned up.

3. **TypeScript declaration generation running unconditionally** — `mod.rs:395` called `generate_typescript_declarations()` on every codegen invocation, cloning all `ComponentIR`s into a fresh `Vec`, despite ADR-031 D6 explicitly deciding to skip generation ("zero production consumers"). The 298-line `typescript.rs` module was preserved but its output was never consumed.

Additionally, the review found that `event_names: Vec<String>` and `event_param_types: Vec<String>` on `ComponentIR` used a **parallel array pattern** — two vectors sharing an implicit index relationship. While lowering guaranteed synchronization, any manual IR construction (tests, future tooling) could silently break it.

**All four items were flagged by 3+ independent specialists** in the review (Pragmatic Enforcer, Domain Expert, Compiler Architect).

## Decision Drivers

* YAGNI: speculative features without consumers add noise and maintenance burden
* Type safety: parallel arrays are a well-known anti-pattern — structural types prevent desync
* ADR compliance: ADR-031 D6 was an accepted decision that was never implemented
* Backward compatibility: all changes use `#[serde(default)]` for serialized IR

## Decision

### 1. Remove `generic_params` and `if_mode_default` from the full pipeline

**Affected layers:**
- IR type (`ComponentIR`), parser AST (`ComponentDef`), parser dispatch, lowering (`lower_component()`)
- ~70 test construction sites across all backends

Both fields have `#[serde(default)]` — serialized IRs containing them ignore the fields on deserialization, ensuring backward compatibility.

### 2. Gate TypeScript declaration generation per ADR-031 D6

Comment out the generation call in `mod.rs:393-395`, set `types = String::new()`. Preserve the 298-line `typescript.rs` module code for future use when a consumer materializes (e.g., Poiesis IDE intellisense, npm package publishing). Update 4 test assertions to expect empty `types`.

### 3. Replace parallel arrays with `EventDeclIR` struct

```rust
pub struct EventDeclIR {
    pub name: String,
    #[serde(default)]
    pub param_type: String,
}
```

Replace `event_names: Vec<String>` + `event_param_types: Vec<String>` with `events: Vec<EventDeclIR>` on `ComponentIR`. Update lowering and all consumers (React imports, React mod, OmniView typescript/store/mod).

### 4. Change untyped event params from `any` to `unknown`

As a bonus type-safety improvement in generated `.d.ts` and `.tsx` output: event handler parameter types default to `unknown` instead of `any`, requiring consumers to narrow the type before use.

### Architectural Components Affected

* `crates/omnilang-core/src/ir/types.rs` — `ComponentIR` (2 fields removed, 1 field restructured)
* `crates/omnilang-core/src/parser/ast/view.rs` — `ComponentDef` (1 field removed)
* `crates/omnilang-core/src/parser/top_level/component.rs` — `if_mode` parsing arm removed
* `crates/omnilang-core/src/ir/lower/component.rs` — lowering updated
* `omnilang/src/codegen/omniview/mod.rs` — TypeScript gen gated
* `omnilang/src/codegen/omniview/typescript.rs` — consumer updated
* `omnilang/src/codegen/react/component/imports.rs` — consumer updated, `any`→`unknown`
* `omnilang/src/codegen/react/component/mod.rs` — consumer updated
* `omnilang/src/codegen/omniview/store.rs` — consumer updated

## Consequences

### Positive
* **IR noise reduction**: 2 dead fields removed, ~140 lines eliminated from test fixtures
* **Structural type safety**: `EventDeclIR` makes the name→type relationship enforced by the type system, not convention
* **ADR compliance**: ADR-031 D6 is now actually implemented
* **Performance**: No more cloning all `ComponentIR`s on every codegen invocation
* **TypeScript quality**: `unknown` forces consumers to narrow types, preventing `any` propagation
* **Backward compatible**: `#[serde(default)]` on all changed/removed fields

### Negative
* **28 files touched** for EventDeclIR migration — wide blast radius, but compiler-enforced (exhaustive match)
* **TypeScript module preserved but dead**: 298 lines of code exist but produce no output. Risk of bit-rot if not maintained.

### Neutral
* `any`→`unknown` change in generated output may require downstream TypeScript consumers to add type assertions — this is a correctness improvement, not a regression

## Alternatives Considered

### A. Keep `generic_params` for future use
Rejected: no parser grammar exists, no codegen backend handles it, and there is no spec for generic components. Re-adding is trivial if a real implementation plan emerges.

### B. Keep TypeScript generation enabled, find a consumer
Rejected: ADR-031 D6 already decided this. If a consumer materializes, uncommenting 2 lines and updating 3 test assertions is trivial.

### C. Keep parallel arrays, add assertion in lowering
Rejected: assertions prevent desync in production but don't help with test/manual IR construction. A struct makes the invariant structural rather than enforced.

## Validation

* `cargo build --workspace` — clean
* `cargo test --workspace` — 830+ tests pass, 0 failures
* `cargo clippy --workspace -- -D warnings` — clean
* `rg 'generic_params|if_mode_default' --type rust` — zero results (confirmed complete removal)
* `rg 'event_names|event_param_types' --type rust` — zero field-access results (confirmed complete migration)

## References

* ADR-031 D6 — original decision to gate TypeScript generation
* ADR-032 — animation scoping that made `if_mode_default` vestigial
* `.architecture/reviews/component-omniview-2026-07-29-v4.md` — v4 architecture review, cross-cutting theme #4 (dead/speculative fields)

## Pragmatic Enforcer Analysis

**Necessity Assessment (8/10):** Dead fields add noise to every test fixture and every new backend implementation. The TypeScript generation wastes cycles on every codegen invocation against an explicit ADR decision. The parallel array pattern is fragile. All three items have zero or near-zero consumer value today.

**Complexity Assessment (2/10):** All changes are mechanical — remove fields, restructure one type, comment out two lines. Compiler errors guide every change.

**Ratio: 2/8 = 0.25** — well within the <1.5 threshold.

**Recommendation: Approve** — Textbook YAGNI enforcement. Each change eliminates code that carries cost without delivering value.
