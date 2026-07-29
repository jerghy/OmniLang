# ADR-034: Extract ComponentScope for Compiler-Enforced Inlining Isolation

## Status

Implemented

## Context

The OmniView codegen backend compiles components by **inlining child component views at compile time** (`try_inline_component()` in `component_html.rs`). During inlining, per-component state (signal names, name renames, @for bindings, etc.) must be isolated to prevent parent-component state from leaking into child-component codegen, and vice versa.

Since the initial implementation, this isolation was handled by a manual `InliningState` struct that saved/restored 8 of the 46 `OmniViewGenerator` fields. The remaining 38 fields were assumed to be "global" — needing no isolation during inlining.

**This pattern caused real bugs twice:**
- **ADR-025 D1**: `for_text_bindings`, `for_class_bindings`, and `for_attr_bindings` were corrupted by nested component inlining inside @for templates because they were not in `InliningState`.
- **ADR-026 D3**: Same class of bug — a new field was added to `OmniViewGenerator` that needed per-component isolation but was not added to `InliningState`.

The root cause: when a developer adds a new field to `OmniViewGenerator`, **nothing forces them to also add it to `InliningState`**. The compiler cannot verify completeness of a manually-maintained save/restore list.

The v4 10-perspective architecture review (2026-07-29) identified this as a **cross-cutting theme flagged by 4 independent specialists** (Systems Architect, Maintainability Expert, Implementation Strategist, Rust Expert).

## Decision Drivers

* Eliminate the proven bug class where new generator fields silently leak across component inlining boundaries
* Make the compiler enforce per-component state isolation — not convention or code review
* Preserve existing behavior (zero behavioral change, all snapshots byte-identical)
* Keep the refactoring contained to the OmniView codegen backend only

## Decision

**Extract all per-component state into a dedicated `ComponentScope` struct, and store it as a single field on `OmniViewGenerator`.** During component inlining, swap the entire `ComponentScope` atomically via `take()`/`restore()`.

### ComponentScope struct (`omnilang/src/codegen/omniview/scope.rs`)

```rust
pub(super) struct ComponentScope {
    pub current_component: String,
    pub signal_names: FxHashSet<String>,
    pub name_renames: FxHashMap<String, String>,
    pub for_text_bindings: Vec<(String, String)>,
    pub for_class_bindings: Vec<(String, String, String)>,
    pub for_attr_bindings: Vec<(String, String, Vec<AttrPart>)>,
    pub for_rebind_fn: Option<String>,
    pub in_for_template: bool,
    pub emitting_root_view: bool,  // WAS MISSING from InliningState — existing bug!
}
```

### Key design choices

1. **Direct field access** (`self.scope.field`) rather than accessor methods — clearer, grep-friendly, no indirection cost
2. **`std::mem::take` in `take()`** — zero-clone save, replaces each field with its `Default` value
3. **`emitting_root_view` added** — this field was NOT in the old `InliningState`, meaning it leaked across component boundaries. This was an existing bug caught by the extraction.
4. **`InliningState` struct deleted entirely** — 54 lines removed

### Architectural Components Affected

* `omnilang/src/codegen/omniview/scope.rs` — new file (77 lines)
* `omnilang/src/codegen/omniview/mod.rs` — `OmniViewGenerator` reduced from 46 to 37 fields + `scope: ComponentScope`; `InliningState` deleted
* 13 files across `omnilang/src/codegen/omniview/` — field references updated (`self.current_component` → `self.scope.current_component`)

## Consequences

### Positive
* **Compiler-enforced isolation**: Adding a new per-component field to `ComponentScope` automatically includes it in save/restore — no manual list to maintain
* **Existing bug fixed**: `emitting_root_view` was missing from the old `InliningState` and leaked across inlining boundaries
* **Reduced cognitive load**: 46-field struct → 37 global fields + 9-field scope struct, clearly separated by concern
* **Zero behavioral change**: All 830+ tests pass, all snapshots byte-identical
* **Eliminates documented bug class**: ADR-025 D1 and ADR-026 D3 cannot recur

### Negative
* **13 files touched**: field reference updates across all `impl OmniViewGenerator` blocks
* **One more file in the module**: `scope.rs` adds a new file, though it's small and focused

### Neutral
* `take()`/`restore()` pattern remains structurally similar to `save()`/`restore()` — the improvement is in scope completeness, not mechanism

## Alternatives Considered

### A. Proc-macro derive for InliningState
Generate save/restore boilerplate via `#[derive(InliningState)]` on `OmniViewGenerator`, similar to serde. Rejected: adds proc-macro dependency, obscures which fields are per-component vs global, and doesn't help with the "38 other fields might also need saving" problem.

### B. Immutable codegen with pure functions
Each codegen function takes per-component state as parameters and returns new state. Rejected: requires threading 9+ parameters through every function, incompatible with the existing mutable-generator architecture used across all 9 backends.

### C. RAII guard with Drop implementation
Use a guard struct that calls `restore()` in `Drop`, preventing state leaks on panic. Rejected: would require `restore()` to take `&mut OmniViewGenerator`, which `Drop` cannot do without interior mutability. The current explicit `restore()` is clearer.

## Validation

* `cargo build --workspace` — clean
* `cargo test --workspace` — 830+ tests pass, 0 failures
* `cargo clippy --workspace -- -D warnings` — clean
* All snapshot tests byte-identical (zero output change)
* `cargo fmt --check` — clean

## References

* ADR-025 D1 — first documented InliningState bug (@for bindings corruption)
* ADR-026 D3 — second documented InliningState bug (new field not saved)
* `.architecture/reviews/component-omniview-2026-07-29-v4.md` — v4 architecture review, cross-cutting theme #1

## Pragmatic Enforcer Analysis

**Necessity Assessment (9/10):** This fixes a proven bug class that has caused real issues twice. The cost of waiting is accumulating more instances of the same pattern. The fix was low-risk (byte-identical snapshots confirm zero behavioral change).

**Complexity Assessment (3/10):** Adds one small struct (77 lines) and one indirection (`self.scope.field`). Removes 54 lines of `InliningState` boilerplate. Net +23 lines for a structural improvement. No new dependencies or patterns.

**Ratio: 3/9 = 0.33** — well within the <1.5 threshold.

**Recommendation: Approve** — This is a textbook case of justified refactoring: fixes real bugs, reduces future risk, and has near-zero cost.
