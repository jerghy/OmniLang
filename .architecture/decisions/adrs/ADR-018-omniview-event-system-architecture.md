# ADR-018: OmniView Event System Architecture

## Status

Accepted

## Context

The OmniView event system handles DOM event bindings (`@onPress`, `@onChange`), component-to-component events (`emit onCustomEvent`), two-way bindings (`bind:value`), and event data field extraction. A comprehensive Compiler & Language Architect review identified structural issues across three layers: IR lowering, codegen mapping, and event field extraction. This ADR documents the architectural decisions made during remediation.

**Reference:** Compiler & Language Architect review of OmniView Event System (2026-07-28). Review found maturity level B- with 2 HIGH and 4 MEDIUM findings.

## Decision Drivers

1. **Multi-backend consistency**: 5 codegen targets (OmniView, React, Flutter, Dioxus, Vue) must handle events uniformly
2. **Pipeline completeness**: AST → IR → codegen data flow must not drop or silently degrade event semantics
3. **Type safety**: Event data fields should be validated against typed schemas at compile time
4. **Backward compatibility**: Existing `.omni` source and generated output should not break silently
5. **Simplicity**: Avoid adding new abstraction layers; thread data through existing lowering infrastructure

## Decision

### D1: Unified `strip_prefix("on")` normalization across all backends

All event mapping functions (`to_react_event`, `to_dioxus_event`, `to_vue_event`, `event_to_flutter_callback`) now strip the `"on"` prefix from IR event names and lowercase before matching. This normalizes IR event names like `"onPress"`, `"onClick"` which originate from the `@onEventName` syntax.

**Pattern:**
```rust
let stem = event_name.strip_prefix("on").unwrap_or(event_name).to_lowercase();
match stem.as_str() {
    "click" | "press" | "tap" => platform_event_handler,
    // ...
    _ => fallback,
}
```

### D2: Proper lowering of `emit` statements

Previously, `Stmt::EventEmit` (the AST node for `emit onCustomEvent(args)`) fell through to a wildcard catch-all in lower/stmt.rs, producing `null;` — a complete no-op. The IR node `StatementIR::EventEmitStmtIR` existed but was never populated.

**Fix:** Added explicit `Stmt::EventEmit` lowering to `StatementIR::EventEmitStmtIR` with properly lowered arguments. The shared codegen default `format_event_emit_stmt` was changed from emitting a comment to emitting a real function call. The OmniView backend resolves event names through the `name_renames` map set up by store.rs for component event dispatch functions.

### D3: Event handler parameter name threading

Previously, event field detection hardcoded parameter name `"e"` in three places:
- `walk_expr_event_fields` (IR-level, FnRef path)
- `scan_expr_for_event_methods` (AST-level, inline handler path)
- `lower_expr::FieldAccess` (AST-level, inline handler path)

**Fix:**
- FnRef path: Parameterized `walk_expr_event_fields` and `collect_ir_event_fields` with `param_name: &str`
- InlineFn AST scan: Added `event_param: &str` to `scan_event_methods` / `scan_stmt_for_event_methods` / `scan_expr_for_event_methods`
- InlineFn lowering: Added `event_param_name: Option<String>` field to `SemanticResult`, set/restore around `lower_block` calls, checked in `lower_expr::FieldAccess`

The SemanticResult field approach was chosen over threading through `lower_expr` → `lower_block` → `lower_stmt` (touching 129 call sites) to minimize surface area.

### D4: CustomEvent dispatch model for component events

Component events (`event onXxx: Type`) use `CustomEvent.dispatchEvent` with `detail: args[0]` for dispatch, and `e.detail` for extraction. This model is consistent across all JS-based targets. React handles custom events as function calls (no CustomEvent), which is correct for React's component prop model.

### D5: Default preventDefault is opt-in

Removed the unconditional `needs_pd = true` for `onSubmit` events. The `scan_event_methods` pass detects explicit `e.preventDefault()` calls in handler bodies. Forms that need native submission behavior can now omit `e.preventDefault()`.

**Architectural Components Affected:**
- IR lowering pipeline (lower/stmt.rs, lower/view.rs, lower/expr.rs)
- Semantic analysis (semantic/mod.rs — `SemanticResult` struct)
- Codegen backends (dioxus/expr.rs, vue/expr.rs, flutter/utils.rs, shared/stmt.rs, omniview/stmt.rs)

**Interface Changes:**
- `walk_expr_event_fields` signature: added `param_name: &str` parameter
- `collect_ir_event_fields` signature: added `param_name: &str` parameter
- `scan_event_methods` / `scan_stmt_for_event_methods` / `scan_expr_for_event_methods`: added `event_param: &str`
- `SemanticResult`: new field `event_param_name: Option<String>`
- `to_vue_event` return type: `&str` → `String` (now produces owned strings for custom events)

## Consequences

### Positive

- All 5 codegen targets now produce correct event handler bindings from identical .omni source
- `emit` keyword now works end-to-end: parser → semantic → lowering → codegen → runtime dispatch
- Event handler parameter names other than `"e"` are now correctly detected for field extraction
- `onSubmit` forms can optionally use native form submission
- Zero test regressions (800+ tests pass across workspace)

### Negative

- Two remaining hardcoded `"e"` checks persist at low-level AST paths (Expr::Block lowering) — these are very rare edge cases
- `to_vue_event` return type change from `&str` to `String` is technically a breaking API change within the compiler crate
- Flutter `event_to_flutter_callback` returns original `event_name` for unmatched events (not stripped stem) — intentional to preserve existing behavior for custom Flutter widget params

### Neutral

- `SemanticResult` now carries transient lowering state (`event_param_name`) alongside its core analysis data — this blurs the boundary between semantic analysis and lowering, but avoids 129 call-site changes
- `format_event_emit_stmt` shared default changed from comment to call — backends that don't override it now get the new behavior automatically

## Implementation Strategy

### Blast Radius

**Impact Scope**: Compiler internals only — no user-facing `.omni` language changes. All fixes produce the same or improved codegen output from existing source.

**Affected Components**:
- `crates/omnilang-core` (IR + semantic + lowering): 7 files
- `omnilang` (codegen backends): 7 files
- Test snapshots: 1 Flutter snapshot updated

**User Impact**: Minimal. The onSubmit preventDefault removal is the only behavioral change — forms that relied on implicit preventDefault will now submit natively unless `e.preventDefault()` is called explicitly.

**Risk Mitigation**:
- All 800+ existing tests pass without modification
- Regex-free, mechanical changes
- Each fix is independently verifiable via its backend's test suite

### Reversibility

**Reversibility Level**: High — all changes are purely internal compiler refactoring. Rolling back reverts the compiler to previous behavior without affecting source compatibility.

### Sequencing & Timing

**Prerequisites**: None. All changes are self-contained.

**Readiness Assessment**: Changes implemented and merged. 12 commits on main branch.

## Alternatives Considered

### Alternative 1: Thread `event_param` through all 129 `lower_expr`/`lower_block`/`lower_stmt` call sites

**Pros:**
- Explicit, type-safe data flow
- No mutable state on shared struct

**Cons:**
- Touches ~5 files with ~129 mechanical call-site updates
- High risk of merge conflicts
- Each call site is identical mechanical threading — high churn for low value

**Decision**: Rejected. The `SemanticResult` field approach achieves the same correctness with 0 call-site changes.

### Alternative 2: Drop `emit` keyword entirely, use only function-call syntax

**Pros:**
- Removes a syntax variant
- Simplifies parser

**Cons:**
- Breaking change to existing .omni source (EmitKeywordDemo, EmitEventDemo test fixtures)
- `emit` keyword carries semantic intent ("this is an event dispatch") that direct function calls don't
- Documentation already describes both forms

**Decision**: Rejected. Keep `emit` keyword and fix its lowering.

### Alternative 3: Convert `emit` to a call expression in the semantic pass (instead of fixing lowering)

**Pros:**
- Avoids introducing `EventEmitStmtIR` node handling in codegen

**Cons:**
- Would hide the semantic distinction at IR level
- Would prevent backends from generating platform-specific emit dispatch code
- Semantic pass already handles `Stmt::EventEmit` — transforming it there would break other analyses

**Decision**: Rejected. The IR node exists for a reason — keeping the distinction through lowering allows backends to specialize.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: These fixes are surgical corrections, not architectural redesigns. Each fix addresses a specific, confirmed bug with minimal code changes. No new abstraction layers, dependencies, or patterns are introduced.

### Decision Challenge

**Proposed Decision**: Fix event system bugs across 5 codegen backends through targeted corrections to lowering, semantic analysis, and codegen mapping functions.

**Necessity Assessment**: 9/10
- **Current need**: 2 HIGH bugs (H1: silent event failures on 3/5 targets; H2: `emit` keyword is a no-op). These are concrete, user-visible defects, not theoretical improvements.
- **Future need**: The fixes also enable future backend work (Flutter, Dioxus, Vue) that was blocked by the prefix bug.
- **Cost of waiting**: Each release with these bugs ships a compiler that silently produces non-functional event handlers. Cost of fixing later is identical — zero incremental complexity.
- **Evidence of need**: 20 event categories exist in `EventDataKind` enum. 3/5 backends were ignoring them. The `emit` reserved word was parsing successfully but producing `null;`.

**Complexity Assessment**: 3/10
- **Added complexity**: The `SemanticResult.event_param_name` field is the only new structural element. All other changes are parameter additions to existing functions.
- **Maintenance burden**: Low. The `strip_prefix("on")` pattern is now uniform across backends — easier to maintain than 4 subtly different approaches.
- **Learning curve**: None. The fixes follow existing patterns (React's `to_react_event` was already correct; Dioxus/Vue/Flutter now mirror it).
- **Dependencies introduced**: Zero. No new crates, libraries, or abstractions.

**Alternative Analysis**: The alternatives (drop `emit` keyword, thread through 129 call sites, covert emit in semantic pass) were all considered and rejected for good reasons. Each would either increase churn or reduce capability without proportional benefit.

**Simpler Alternative Proposal**: None needed. The implementation is already the minimal fix for each finding:
- H1: 1 line per backend (`strip_prefix`)
- H2: 1 case arm + 1 function body change
- M1-M3: 1 line, 2 lines, and `SemanticResult` field respectively

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 3/10
- **Ratio**: 0.33 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: This is appropriate engineering — fixing confirmed bugs that silently corrupt compiler output. No over-engineering detected.

## Validation

**Acceptance Criteria:**
- [x] All 800+ tests pass (cargo test --workspace)
- [x] Clippy clean on modified files
- [x] rustfmt applied
- [x] Dioxus/Vue/Flutter test suites produce correct event handler output
- [x] OmniView `emit` generates real dispatch calls, not comments
- [x] Snapshot tests updated where event output changed

**Testing Approach:**
- All 5 codegen backends have dedicated test suites (`cargo test -p omnilang -- <backend>`)
- Regression test fixtures cover `emit` keyword (EmitKeywordDemo, EmitEventDemo)
- Snapshot-based testing catches output changes (one Flutter snapshot updated)
- Parameterized cross-backend tests verify consistent event handling

## References

- [Compiler & Language Architect Review of OmniView Event System](.architecture/reviews/) — 2026-07-28
- [Implementation Plan](docs/superpowers/plans/2026-07-28-omniview-event-system-fixes.md)
- [Event data schema](crates/omnilang-core/src/ir/events.rs) — 20 EventDataKind variants
- [Event handler IR types](crates/omnilang-core/src/ir/types.rs:417-505) — EventHandlerIR enum
- [OmniView Events Documentation](docs/omniview/07-events-and-bindings.md)
- [ADR-001: Compiler Pipeline Architecture](ADR-001-compiler-pipeline-architecture.md) — original IR design
- [ADR-013: Codegen Modularization](ADR-013-codegen-modularization-format-traits.md) — format trait pattern used for event codegen
