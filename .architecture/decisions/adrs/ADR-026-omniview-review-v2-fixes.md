# ADR-026: OmniView 2026-07-29 v2 Review Remediation

## Status

Implemented

## Context

The 2026-07-29 v2 architecture review (`.architecture/reviews/component-omniview-2026-07-29-v2.md`) conducted by 11 specialists found 6 HIGH-severity and 23 MEDIUM-severity findings across the OmniView codegen. This ADR records the architectural decisions made during the remediation of all 16 actionable findings.

The review's top priorities were:

1. **`format_mutation_stmt_inline` producing invalid JS** — confirmed by 3 independent specialists (Rust Expert, TS/React Specialist, Maintainability Expert). The function returned debug placeholder strings instead of valid JavaScript.
2. **Boolean attribute detection failure** — string heuristic missed complex expressions like `disabled={!isSubmitting}`, causing silent permanent-true attributes.
3. **Nested `@for` buffer corruption** — inner component `@for` loops cleared outer loop binding buffers during component inlining.
4. **Reactive `href`/`src` XSS** — no URL scheme validation on signal-driven attribute bindings.
5. **Implicit state routing without phase guards** — three mode flags and deferred JS swap created phase-confusion bugs (proven by ADR-025's `__omni_effect` leak).
6. **No Content-Security-Policy in standalone HTML** — defense-in-depth gap.

## Decision Drivers

- **Correctness over compatibility**: The `format_mutation_stmt_inline` fix changes `&self` → `&mut self` on 9 methods in the expression evaluation cascade. Necessary because `emit_mutation` writes to the JS buffer.
- **Security-first**: URL sanitization and CSP are defense-in-depth measures. The `javascript:` URL injection vector was a real, exploitable XSS path.
- **Minimal blast radius**: All 16 fixes are S-to-M effort changes localized to specific files, targeting high-impact-low-risk interventions.
- **Zero snapshot drift for refactors**: The event listener deduplication (Task 8) must produce byte-identical output — verified with `cargo insta review`.

## Decision

### D1: Buffer-capture pattern for mutation inline codegen

**Delegate `format_mutation_stmt_inline` to `emit_mutation` via temporary buffer swap.**

The old code returned debug strings like `"count direct_assign 5"`. The fix uses `std::mem::take`/`std::mem::replace` on `self.js` to capture `emit_mutation`'s output as a return value. This required cascading `&self` → `&mut self` through 9 methods (`expr_to_js`, `block_to_expr`, `expr_label`, 5 `motion_config.rs` methods, `expr_default_display`).

**Rationale**: The buffer-capture pattern produces identical JS output to the working `emit_mutation` path without duplicating the codegen logic. The `&mut self` cascade is justified — all 9 methods genuinely call `expr_to_js` which now needs mutable access for the temporary buffer swap.

### D2: ExprIR type matching for boolean attribute detection

**Replace string heuristic `val_str.starts_with('{') && val_str.ends_with('}')` with `!matches!(value, ExprIR::LiteralExpr { .. })`.**

The old heuristic only caught simple identifier expressions. Complex expressions like `disabled={!isSubmitting}` returned `""` from `expr_to_static_str`, falling through to bare attribute emission (permanent `true`). ExprIR type matching correctly identifies any non-literal as dynamic.

**Rationale**: The `ExprIR` type is the canonical representation of expression kinds in the IR. Using it for dynamic detection is semantically correct and future-proof against new expression variants.

### D3: @for state save/restore around component inlining

**Save/restore `for_text_bindings`, `for_class_bindings`, `for_attr_bindings`, `for_rebind_fn`, and `in_for_template` around nested component inlining in `try_inline_component`.**

Matching the existing save/restore pattern for `signal_names` and `name_renames`. Uses `std::mem::take` for Vec fields (zero-cost on empty vectors) and `Option::take()` for `for_rebind_fn`.

**Rationale**: The save/restore pattern is already proven in `try_inline_component` for signal context. Extending it to @for state prevents inner component `@for` loops from corrupting outer loop binding data without introducing new abstractions.

### D4: Runtime sanitization functions

**Add `__omni_sanitizeUrl()` and `__omni_sanitizeCss()` to the generated JS runtime.**

- `__omni_sanitizeUrl`: blocks `javascript:`, `data:text/html`, `vbscript:` URL schemes, returns `'#blocked'`
- `__omni_sanitizeCss`: blocks 9 CSS injection patterns (`javascript:`, `expression(`, `url(`, `-moz-binding`, `data:text/html`, `@import`, `vbscript:`, `document.`, `window.`), returns `''`

Both are ES5-compatible (`function`, `var`, `indexOf`). URL sanitization wraps `setAttribute` calls for `href`/`src`/`action`/`formaction`. CSS sanitization wraps `$signal`/`$concat`/`$call` dynamic style paths.

**Rationale**: The compile-time sanitizers (`sanitize_css_value`, `escape_html`) correctly handle static values but signal-driven dynamic values bypass them entirely. Runtime sanitization closes this gap at the correct layer — the generated JS rather than the compiler.

### D5: Phase assertion guards

**Add `debug_assert!` phase guards at JS emission entry points.**

- `assert_can_emit_js()` — checks `!dom_emitted` (JS after DOM insertion)
- `assert_not_css_only()` — checks `!css_only` (pure-JS methods only)

Applied to 6 methods across 5 files. `assert_can_emit_js` intentionally does NOT check `css_only` — view-generation methods (`try_inline_component`, `emit_element_events`) are legitimately called in css_only mode for HTML/element-tree generation.

**Rationale**: Phase confusion caused the ADR-025 `__omni_effect` leak. Debug assertions catch similar bugs at development time with zero release cost. The split between `assert_can_emit_js` and `assert_not_css_only` reflects the dual-use nature of view-generation methods.

### D6: Error severity taxonomy

**Parse `warn:`/`error:`/`fatal:` prefixes from `__omni_reportError` context strings.**

The runtime function now extracts severity from the context string prefix, uses appropriate console methods (`console.warn` vs `console.error`), and emits structured error objects with `severity`/`code`/`message`/`time` fields.

Call site categorization:
- `warn:` — transient/recoverable (element not found, interpolation text, @for rebind)
- `error:` — handler/binding failures (event handlers, attribute bindings, list keys)
- `fatal:` — reserved for irrecoverable state corruption

**Rationale**: Enables AI-powered diagnostic tools in the Poiesis editor to filter and prioritize errors. The 100-entry FIFO buffer previously mixed transient warnings with real bugs, making automated debugging unreliable.

### D7: Event listener deduplication

**Extract single `emit_event_listener()` method handling all three `EventHandlerIR` variants (FnRef, StoreAction, InlineFn).**

Pure refactor — replaces 8 independent `addEventListener` emission points across `events.rs` and `component_html.rs`. Verified byte-identical output via zero snapshot differences.

**Rationale**: A bug fix applied to only one file would leave the other emitting broken JS. Single source of truth prevents drift.

### D8: UiCodegen trait removal

**Remove dead `UiCodegen` trait implementation (~120 lines across 3 backends) with zero call sites.**

Trait definition retained in `omnilang-core` for planned multi-target component generation integration. Implementations removed to prevent the O(n²) trap in OmniView's `generate_component_code` (re-runs full pipeline if ever called).

**Rationale**: Dead code that compiles but produces no behavior is maintenance debt. The trait integration was deferred during P35 crate split and never wired up.

## Consequences

### Positive

- **Correctness**: `format_mutation_stmt_inline` now produces valid JS. Boolean attributes work correctly for complex expressions. Nested `@for` loops no longer corrupt outer loop bindings.
- **Security**: URL scheme injection vector closed. CSS injection via dynamic styles blocked. CSP defense-in-depth added for standalone mode.
- **Observability**: Error severity taxonomy enables structured error triage. Phase guards catch development-time bugs.
- **Maintainability**: Event listener emission unified to single source of truth. Dead UiCodegen code removed. `finalize_js` inlined. Source span length propagated.
- **Performance**: Component root element cached in event prop dispatch. Design class validation O(1) instead of O(n×m). `push_str(&format!(...))` replaced with `write!` on hot paths (Task 14 partially addresses this).

### Negative

- **`&mut self` cascade**: The mutation inline codegen fix propagated `&mut self` through 9 methods. This is a necessary consequence of the buffer-capture pattern but increases mutable surface area.
- **Snapshot churn**: 5 insta snapshot files updated for runtime function additions and error severity changes.
- **Pre-existing test failure**: `test_resolve_styles_with_design_class` has a pre-existing warning mismatch unrelated to these fixes (confirmed by bisecting to pre-fix state).

### Neutral

- **ES5 compatibility maintained**: All runtime changes use ES5 patterns (`function`, `var`, `indexOf`). No arrow functions, `const`/`let`, or template literals introduced.

## Implementation Summary

17 commits across 16 tasks, all S-to-M effort:

| Category | Tasks | Commits |
|----------|-------|---------|
| Correctness fixes | C1, C2, C3 | 3 |
| Security fixes | C4, I2, I3 | 3 |
| Phase guards | C5, C6 | 2 (+1 fix) |
| Error observability | I4, I5 | 2 (+1 fix) |
| Code quality | I1, I6, I7, N2, N3 | 5 |
| Performance | I8, N4, N5 | 3 |
| Documentation | N/A | ADR-026 (this document) |

**Test results**: 93/93 omniview unit tests pass. 3/4 resolve_styles integration tests pass (1 pre-existing failure). Clippy clean.
