# ADR-019: OmniView Event System Remediation — Phase 2 (Deferred Items)

## Status

Accepted

## Context

ADR-018 documented the first phase of event system fixes (D1-D5: unified `strip_prefix("on")`, proper `emit` lowering, event param name threading, CustomEvent dispatch model, opt-in preventDefault). A follow-up dual-specialist review (2026-07-28) identified 4 additional items that were deferred from Phase 1:

1. **Custom event payload silently lost** on React/Dioxus/Flutter backends (returns `{}` instead of `e.detail`)
2. **InlineFn event data extraction computed then discarded** on Vue/Dioxus/Flutter/React (4 backends)
3. **`@for` delegation drops `.prevent`/`.stop` modifiers** in OmniView
4. **Missing `onTap` mappings** on Dioxus (evaluated false-positive for Vue)
5. **React zero-arg `emit` no-op** (`onCustomEvent;` instead of `onCustomEvent();`)
6. **@for event extraction always allocates all fields** (performance)
7. **React event prop types are always `(value: any) => void`**
8. **Multi-arg events lose args beyond `args[0]`** in CustomEvent detail
9. **`field_extract_*` ~80% duplicated** across 4 backends
10. **Dioxus/Flutter InlineFn FieldAccess not redirected** to local extraction bindings

Items 1-8 were fixed in Phase 2 (11 commits). Items 9-10 were also addressed (9 partially, 10 fully) via the `FormatExpr` trait extension. Item 9 (`field_extract_*` dedup) was re-evaluated and deferred as the backends have legitimate differences in DOM access patterns.

**Reference:** Compiler & Language Architect + TypeScript & React Specialist joint review of OmniView Event System (2026-07-28). Phase 1 fixes documented in ADR-018.

## Decision Drivers

1. **Cross-backend correctness**: All 5 codegen targets must handle custom events, InlineFn field extraction, and event modifiers consistently
2. **Minimal invasiveness**: Prefer targeted codegen fixes over IR/lowering pipeline changes
3. **Type safety**: React TypeScript output should reflect declared event parameter types
4. **Performance**: @for loops should not allocate unnecessary event field objects
5. **Backward compatibility**: All existing .omni source and 807+ tests must continue to pass

## Decision

### D6: Custom event payload via `e.detail` in all backends

The `None` branch in `emit_event_object_react`/`emit_event_object_dioxus`/`emit_event_object_flutter` returned `{}`/`FxHashMap::default()` instead of the actual `CustomEvent.detail` payload. The shared `emit_event_object` in omnilang-core already handled this correctly.

**Fix:** Changed `None => return "{}".to_string()` to `None => return format!("{}.detail", raw_var)` in all three backends.

**Pattern:**
```rust
// React/Flutter:
None => return format!("{}.detail", raw_var)  // was: "{}".to_string()
// Dioxus (Rust):
None => return format!("{}.detail()", raw_var)  // was: "FxHashMap::default()"
```

### D7: InlineFn event extraction via `Object.assign` (JS backends) / local bindings (Rust/Dart backends)

The InlineFn handler path in Vue/React/Dioxus/Flutter computed event field extraction but discarded the result. The fix depends on the target language:

**JS-family (Vue, React):** `Object.assign(e, eventObj)` augments the event parameter with extracted fields before the body executes. Body references like `e.value` resolve to the extracted value.

**Rust-family (Dioxus):** Individual `let field = extract;` bindings for each used field, using `field_extract_dioxus` for correct web_sys API calls.

**Dart-family (Flutter):** Individual `final field = extract;` bindings, using `field_extract_flutter`.

### D8: `event_param_name` in `FormatExpr` trait for FieldAccess redirection

**This is the key architectural decision of Phase 2.**

When an InlineFn handler body references `e.value`, the codegen emits `e.value()` (Dioxus) or `e.value` (Flutter). After D7 emits `let value = extract;`, the body needs to reference the local `value` binding, not `e.value`.

**Solution:** Added `event_param_name: Option<String>` to the `FormatExpr` trait with a default `None`. When set, the `FieldAccessExpr` handler in `format_expr` redirects:

```rust
ExprIR::FieldAccessExpr { object, field } => {
    if let Some(param) = self.event_param_name() {
        if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
            if name == param {
                return self.format_identifier(field); // emit bare field name
            }
        }
    }
    format!("{}.{field}", self.format_expr(object))
}
```

**Struct changes:**
- `RustFormatExpr`: added `event_param_name: Option<String>` field
- `DartFormatExpr`: converted from unit struct to struct with `event_param_name: Option<String>` field

**Trait interface:** `fn event_param_name(&self) -> Option<&str> { None }` — zero-impact default for all other backends.

### D9: Multi-arg CustomEvent detail

Changed `detail: args[0]` to `detail: args.length <= 1 ? args[0] : args` in the OmniView store event stub. Single-arg events pass the value directly (backward compatible); multi-arg events pass an array.

### D10: `@for` data-event flag encoding

Extended the `data-event-*` attribute format from `fn:handlerName` to `fn:handlerName|pd|sp` where `pd` = `needs_prevent_default`, `sp` = `needs_stop_propagation`. The `_forHandle` function parses flags and calls `e.preventDefault()`/`e.stopPropagation()` dynamically, replacing the hardcoded `e.type === 'submit'` check.

### D11: React typed event props

Added `event_param_types: Vec<String>` field to `ComponentIR` (parallel to `event_names`). Populated during lowering via `type_ann_to_ts()` which converts `TypeAnnotation` to TypeScript type strings (e.g., `string` → `"string"`, `i32` → `"number"`, `Optional<T>` → `"T | undefined"`).

React component prop interfaces now emit typed callbacks:
```typescript
// Before:
onUpdate?: (value: any) => void;

// After:
onUpdate?: (value: string) => void;
```

**Architectural Components Affected:**
- `crates/omnilang-core/src/codegen/shared/expr.rs` — `FormatExpr` trait (D8)
- `crates/omnilang-core/src/ir/types.rs` — `ComponentIR.event_param_types` (D11)
- `crates/omnilang-core/src/ir/lower/component.rs` — `type_ann_to_ts()` (D11)
- `omnilang/src/codegen/react/utils.rs` — `emit_event_object_react` (D6)
- `omnilang/src/codegen/react/view.rs` — InlineFn handler + Object.assign (D7)
- `omnilang/src/codegen/react/component/mod.rs` — typed props (D11)
- `omnilang/src/codegen/react/expr.rs` — zero-arg emit fix
- `omnilang/src/codegen/vue/template.rs` — InlineFn Object.assign (D7)
- `omnilang/src/codegen/dioxus/expr.rs` — event payload + tap/press mapping (D6)
- `omnilang/src/codegen/dioxus/view.rs` — InlineFn let bindings (D7)
- `omnilang/src/codegen/dioxus/mod.rs` — `DioxusStmtFormatter` visibility
- `omnilang/src/codegen/flutter/view.rs` — InlineFn final bindings (D7)
- `omnilang/src/codegen/flutter/motion_widgets.rs` — event payload (D6)
- `omnilang/src/codegen/flutter/stmt.rs` — `FlutterStmtFormatter` visibility
- `omnilang/src/codegen/omniview/store.rs` — multi-arg CustomEvent (D9)
- `omnilang/src/codegen/omniview/view/element.rs` — @for flag encoding (D10)
- `omnilang/src/codegen/omniview/list_render.rs` — @for flag decoding + used_fields (D10)
- `omnilang/src/codegen/omniview/utils.rs` — `collect_for_events` used_fields
- `omnilang/src/codegen/dart/expr.rs`, `dart/mod.rs` — DartFormatExpr construction site updates
- `omnilang/src/codegen/rust/expr.rs`, `rust/mod.rs` — RustFormatExpr construction site updates

**Interface Changes:**
- `FormatExpr` trait: new `event_param_name()` method with default `None`
- `RustFormatExpr`: new `event_param_name: Option<String>` field
- `DartFormatExpr`: converted from unit struct; new `event_param_name: Option<String>` field
- `ComponentIR`: new `event_param_types: Vec<String>` field (serde default)
- `FlutterStmtFormatter.expr`: field visibility changed to `pub(crate)`
- `DioxusStmtFormatter.expr`: field visibility changed to `pub(super)`

## Consequences

### Positive

- All 5 codegen targets now produce correct custom event payloads (was silently `{}` on 3/5)
- InlineFn handlers with event field access (`e.value`, `e.x`, `e.y`) now work on all 5 backends
- `@for` loops respect `.prevent` and `.stop` modifiers (was silently ignored)
- React TypeScript output has typed event callbacks instead of `any`
- Multi-argument event emits pass all arguments (not just `args[0]`)
- `@for` event extraction only allocates used fields (was 11 fields per PointerEvent trigger)
- Zero-arg `emit` is now a function call (was a no-op on React)
- `@onTap` works on all backends (was producing non-standard `ontap` on Dioxus)
- Zero test regressions (807+ tests pass)
- `event_param_name` default `None` means zero impact on non-affected code paths

### Negative

- `DartFormatExpr` converted from unit struct to struct with field — all construction sites (6 files) must include `event_param_name: None`
- `event_param_types` added to `ComponentIR` — 50+ test construction sites needed updating
- `FlutterStmtFormatter` and `DioxusStmtFormatter` field visibility relaxed for InlineFn access — minor encapsulation trade-off
- `event_param_name` redirection is conditional on exact identifier match (`name == param`) — if a handler uses a different parameter name, redirection won't activate (this is the correct behavior, but worth noting)

### Neutral

- `SemanticResult.event_param_name` (from ADR-018 D3) is used in lowering but the format-level `event_param_name` is an independent mechanism at the codegen layer
- The `type_ann_to_ts()` function handles only common OmniLang types; unrecognized types pass through as-is (e.g., custom user types)

## Implementation Strategy

### Blast Radius

**Impact Scope**: Codegen layer only — no user-facing .omni language changes. All fixes produce improved codegen output from existing source.

**Affected Components**: 25 files across 44 changed files (remaining 19 files are test construction sites with the new `event_param_types` field).

**User Impact**: Positive only. React users get typed event callbacks. @for users get working .prevent/.stop. All users get correct custom event payloads.

**Risk Mitigation**:
- All 807+ existing tests pass without modification (except 2 snapshot updates for expected output changes)
- The `FormatExpr::event_param_name()` default returns `None` — safe default for all non-InlineFn paths
- Flag format `fn:handlerName|pd|sp` is backward compatible: `|` is not valid in JS function names

### Reversibility

**Reversibility Level**: High — all changes are internal compiler refactoring. Rolling back restores previous behavior.

### Sequencing

**Prerequisites**: ADR-018 Phase 1 fixes (already merged). No other dependencies.

**Readiness Assessment**: Changes implemented and merged. 11 commits on main.

## Alternatives Considered

### Alternative 1: Convert InlineFn to synthetic FnRef in lowering (instead of codegen-level FieldAccess redirection)

**Pros:** Single fix point; all backends benefit automatically
**Cons:** Requires new IR nodes; breaks the semantic distinction between inline and referenced handlers; 129 call-site changes in lowering

**Decision**: Rejected. The codegen-level approach (D8) is more surgical and avoids IR changes.

### Alternative 2: Thread `event_param_name` through all statement/expression formatter functions

**Pros:** No struct field changes
**Cons:** 50+ function signature changes across all backends; high merge conflict risk; every call site is identical mechanical threading

**Decision**: Rejected. The `FormatExpr` trait field approach achieves the same correctness with zero call-site changes (except construction sites).

### Alternative 3: Use string replacement on generated code instead of FieldAccess redirection

**Pros:** No struct/trait changes at all
**Cons:** Fragile; breaks on complex expressions; can't distinguish `e.value` (field access) from `evaluate()` (function call)

**Decision**: Rejected. String-level manipulation is too fragile for production codegen.

### Alternative 4: Drop `emit` keyword, use only function-call syntax (revisited from ADR-018)

**Pros:** Removes a syntax variant
**Cons:** Breaking change; `emit` carries semantic intent

**Decision**: Already rejected in ADR-018 (Alternative 2). Reconfirmed here.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: Phase 2 fixes are surgical corrections of confirmed bugs from the dual-specialist review. The only new structural element is the `FormatExpr::event_param_name()` trait method, which is a minimal, opt-in mechanism. No new abstractions, dependencies, or patterns are introduced.

### Decision Challenge

**Proposed Decision**: Fix 10 remaining event system issues through targeted codegen changes including a new `event_param_name` mechanism in the `FormatExpr` trait.

**Necessity Assessment**: 9/10
- **Current need**: 3 CRITICAL/HIGH bugs (custom payload loss, InlineFn extraction discard, @for modifier loss) confirmed by dual-specialist review with concrete reproduction scenarios
- **Future need**: Enables correct multi-backend InlineFn handler behavior
- **Cost of waiting**: Each release ships a compiler that silently produces wrong event handler output on 4/5 backends
- **Evidence of need**: 20 event categories in `EventDataKind` enum; only OmniView backend correctly threaded event data through InlineFn handlers

**Complexity Assessment**: 4/10
- **Added complexity**: The `event_param_name` trait method is the only new structural element; all other changes are field additions or 1-line fixes
- **Maintenance burden**: Low. The `event_param_name` default `None` means zero impact on existing code
- **Learning curve**: Minimal. The pattern is self-documenting: "if this is set, FieldAccess on this identifier emits bare field name"
- **Dependencies introduced**: Zero. No new crates, libraries, or abstractions.

**Alternative Analysis**: Three alternatives were considered and rejected for good reasons. Each would either increase fragility (string replacement), churn (signature threading), or complexity (synthetic FnRef IR node).

**Simpler Alternative Proposal**: None needed. The implementation is the minimal fix for each finding:
- D6: 1 line per backend
- D7: 3-5 lines per backend (emit Object.assign or let/final bindings)
- D8: 1 trait method + 1 field + 1 if-check (the only structural addition)
- D9-D11: 1 line, 5 lines, and ~15 lines respectively

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 4/10
- **Ratio**: 0.44 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: This is appropriate engineering — fixing confirmed bugs that silently corrupt compiler output on 4 of 5 backends. The `event_param_name` mechanism is the minimal abstraction needed to solve a real cross-backend problem. No over-engineering detected.

## Validation

**Acceptance Criteria:**
- [x] All 807+ tests pass (cargo test --workspace)
- [x] Clippy clean (no new warnings)
- [x] rustfmt applied
- [x] Custom event payload returns `e.detail` on all 5 backends
- [x] InlineFn handlers with event field access work on all 5 backends
- [x] `@for` .prevent/.stop modifiers work correctly
- [x] React TypeScript output has typed event callbacks
- [x] Multi-arg emit passes all arguments
- [x] Zero-arg emit produces function call (not bare reference)
- [x] Snapshot tests updated where event output changed (2 snapshots)

**Testing Approach:**
- All 5 codegen backends have dedicated test suites (`cargo test -p omnilang -- <backend>`)
- Parameterized cross-backend tests verify consistent event handling
- Snapshot-based regression tests catch output regressions
- `event_param_types: Vec<String>` has `#[serde(default)]` for backward-compatible deserialization

## References

- [ADR-018: OmniView Event System Architecture](ADR-018-omniview-event-system-architecture.md) — Phase 1 fixes
- [Event System Remediation Plan](docs/superpowers/plans/2026-07-28-omniview-event-system-remediation.md) — Phase 2 plan
- [Event System Remediation Follow-Up Plan](docs/superpowers/plans/2026-07-28-omniview-event-system-remediation-followup.md) — Deferred items plan
- [Compiler & Language Architect Review](.architecture/reviews/) — Dr. Lin Wei, 2026-07-28
- [TypeScript & React Specialist Review](.architecture/reviews/) — Emma Rodriguez, 2026-07-28
- [FormatExpr trait](crates/omnilang-core/src/codegen/shared/expr.rs:26) — `event_param_name()` addition
- [Event data schema](crates/omnilang-core/src/ir/events.rs) — 20 EventDataKind variants
- [ADR-013: Codegen Modularization](ADR-013-codegen-modularization-format-traits.md) — FormatExpr trait architecture
