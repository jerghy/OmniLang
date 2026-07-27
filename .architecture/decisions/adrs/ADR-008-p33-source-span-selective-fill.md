# ADR-008: P33 source_span Population Strategy — Selective Fill

## Status

Accepted

## Context

P33 added `source_span: Option<Span>` to 37 IR struct definitions across `types.rs`, `logic.rs`, `design.rs`, and `types_motion.rs`. The original 2026-07-26 review found `IRProgram.source_map` was always empty (CRITICAL) — this was fixed by P4a (lowering now inserts 5 entity-type entries). P33 extended the data model but did not complete the propagation.

The 2026-07-27 deep review (Compiler Architect code-level audit) found:

- **1 struct populated**: `ElementIR` (`view.rs:599` — `source_span: Some(el.span)`)
- **6 sites can be immediately populated** from available AST spans (ComponentIR, StoreIR, 3×FunctionIR, HeroIntent)
- **2 structs cannot be populated**: ModuleIR and IRProgram — their AST types (`Module`, `Program`) lack `span` fields
- **29 expression-oriented structs** have no current consumer for `source_span`

Additionally, a separate P4 pipeline bug was discovered: `codegen/mod.rs:117` **overwrites** (not extends) the codegen-computed per-element source_map with the IR-level source_map.

## Decision Drivers

1. **Existing AST spans are trivially available**: 6 construction sites already have `comp.span`/`store.span`/`f.span`/`a.span`/`h.span` in scope.
2. **ModuleIR/IRProgram physically cannot be populated**: Their AST types (`Module` at `parser/ast/mod.rs:467`, `Program` at `parser/ast/mod.rs:484`) lack `span` fields — this is an AST design limitation.
3. **No consumers exist for 29 structs**: Only `ElementIR.source_span` is read (3 consumers: 2 semantic lints + 1 codegen source_map writer).
4. **`IRProgram.source_map` already works end-to-end** for entity-level mapping (Component/Store/Function/Motion/Timeline).
5. **Removing fields would be premature**: The `Option<Span>` overhead is negligible; fields serve as honest signal of "not yet populated."

## Decision

**Populate `source_span` on the 6 sites where AST spans are immediately available. Leave 29 expression-oriented structs as `None` until concrete consumers emerge.**

### Phase 1 (Immediate — 6 sites, ~6 lines):

| File:Line | IR Struct | AST Span Source |
|-----------|-----------|-----------------|
| `component.rs:289` | ComponentIR | `comp.span` |
| `component.rs:634` | StoreIR | `store.span` |
| `component.rs:107` | FunctionIR (comp fn) | `f.span` |
| `component.rs:130` | FunctionIR (action) | `a.span` |
| `component.rs:620` | FunctionIR (store action) | `action.span` |
| `component.rs:260` | HeroIntent | `h.span` |

### Deferred (until consumer need):
- 29 expression-oriented structs in `logic.rs`, `design.rs`, `types_motion.rs` — all `source_span: None`
- ModuleIR, IRProgram — cannot be populated (AST limitation)

### Also fixed (same phase):
- `codegen/mod.rs:117`: `=` → `extend` to preserve per-element source_map from OmniView codegen

**Architectural Components Affected:**
- `omnilang/src/ir/lower/component.rs` — 6 construction sites
- `omnilang/src/codegen/mod.rs:117` — source_map merge logic

## Consequences

### Positive

- ComponentIR and StoreIR now carry source location for future diagnostic consumers.
- Per-element (NID-level) source_map preserved end-to-end (overwrite bug fixed).
- 29 unpopulated fields serve as documentation: "this struct's source location is not yet tracked."
- Zero risk — these are `Option` fields, defaulting to `None` via `#[serde(default)]`.

### Negative

- ModuleIR and IRProgram remain without source location (AST limitation — requires parser change).
- 29 `source_span: None` sites are visual noise in construction code.

### Neutral

- The `source_span` (per-struct, byte-precise) and `source_map` (per-node, line/col) systems remain overlapping but distinct. Unification is a separate decision.

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low — 7 lines changed. No API changes. No test expectations affected.

**Affected Components**:
- `ir/lower/component.rs` — 6 source_span assignments
- `codegen/mod.rs` — 1 source_map merge fix

### Sequencing

Phase 1 Task 1.1 (source_map bug) + Task 1.2 (source_span 6 sites) in the implementation plan. Both are S-effort, Day 1.

### Reversibility

Fully reversible — revert `Some(span)` to `None` if a populated span causes issues. The `source_map` merge fix is a one-character revert (`extend` → `=`).

## Alternatives Considered

### Populate all 37 structs
**Rejected**: 29 expression-oriented structs have zero consumers. Threading AST spans through ~200 lines of view-lowering code for structures built from extracted attribute values (not direct AST nodes) would be high-effort with no current benefit.

### Remove source_span from unpopulated structs
**Rejected**: Premature removal. The field is harmless (`Option` overhead negligible) and serves as honest signal. Re-adding later if a consumer emerges would be more costly than keeping.

### Populate only 2 core structs
**Rejected**: FunctionIR and HeroIntent spans are equally trivially available. Including them costs 4 extra lines with identical risk profile.

## Pragmatic Enforcer Analysis

**Necessity**: 5/10 — ComponentIR/StoreIR source_span has future diagnostic value. Per-element source_map preservation has immediate tooling value.

**Complexity**: 1/10 — 7 lines changed. No new code. No test changes.

**Ratio**: 0.20 ✅

**Recommendation**: ✅ Approved. Minimum viable fix — fill what's trivially available, defer the rest.

## Validation

**Acceptance Criteria**:
- [ ] 6 source_span sites changed from `None` to `Some(span)`
- [ ] `codegen/mod.rs:117` uses `extend` instead of `=`
- [ ] `cargo build -p omnilang --lib` passes
- [ ] `cargo test -p omnilang --lib` passes (656/0 unchanged)

## References

- [Deep Technical Review 2026-07-27](../reviews/deep-review-2026-07-27.md) — Finding #1 and #2
- [Source Map Implementation](../../../C:/Users/XHT/.claude/projects/D--space-OmniLang/memory/source-map-implementation.md)
- [[ADR-004]] — Original review highlighting source_map as CRITICAL
