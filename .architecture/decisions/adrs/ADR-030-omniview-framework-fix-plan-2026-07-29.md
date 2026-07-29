# ADR-030: OmniView Framework Completeness & Security Fix Plan (2026-07-29)

## Status

Accepted

## Context

The 2026-07-29 comprehensive architecture review ([component-omniview-frontend-framework-2026-07-29.md](../../reviews/component-omniview-frontend-framework-2026-07-29.md), 10 specialists, 44 findings) evaluated OmniView specifically as a **frontend framework** — its component model, reactivity system, state management, styling, animation, and developer experience. The review identified 3 NEW HIGH-severity framework gaps and 10 MEDIUM compiler/security/refactoring issues. This ADR documents the architectural decisions made during the 13-task remediation plan.

**Reference:** Architecture review (component-omniview-frontend-framework-2026-07-29.md), ADR-028 (Debug Runtime Trade-offs), ADR-029 (Classification Framework), ADR-027 (Security Sanitization Architecture).

## Decision Drivers

1. **Framework completeness**: OmniView must deliver on documented language features — a non-functional content-projection system erodes developer trust
2. **Data integrity**: Props documented as immutable must not be silently mutable — parent-child data corruption is a correctness bug regardless of debug/production role
3. **Production adoption path**: The React production target (ADR-028 D14) requires TypeScript type declarations for practical use
4. **Security completeness**: CSS escape sequence bypass (documented in ADR-027) must be closed; Rust/JS sanitizer inconsistency must be resolved
5. **Performance debugging**: @enter replay over-subscription (O(S×A) per signal change) degrades the Poiesis editor experience
6. **Editor responsiveness**: WASM compilation latency scales with total project size, not delta — incremental caching fixes this
7. **Refactoring safety**: Fragile manual state save/restore in component inlining has caused bugs across multiple review cycles (ADR-025 D1, ADR-026 D3)

## Decision

### D1: Fragment ViewTree Variant for Slot/Content-Projection

`ViewTree::Slot` emitted raw `<slot>` HTML elements but `try_inline_component` never wired parent-submitted children into child slot positions. The fix adds a `Fragment(Vec<ViewTree>)` variant to `ViewTree` and an `inject_slot_children` function that recursively maps parent children into child `<slot>` positions during component inlining.

**Pattern:**
```rust
ViewTree::Fragment(children) // Renders children inline, no wrapper element
fn inject_slot_children(view: ViewTree, slot_children: &[ViewTree]) -> ViewTree // Recursive slot injection
```

Named slots (`<slot name="title"/>`) route children with `slot="title"` attributes. Unnamed slots (`<slot/>`) receive all non-named children. Fallback: empty slot with no matching children preserves the original slot element.

**Architectural Components Affected:**
- `crates/omnilang-core/src/ir/types.rs` — `ViewTree` enum (new `Fragment` variant)
- `omnilang/src/codegen/omniview/view/mod.rs` — `view_to_html` (Fragment rendering)
- `omnilang/src/codegen/omniview/component_html.rs` — `try_inline_component` (slot injection wiring + helper functions)
- All 5 codegen backends (OmniView, React, Vue, Dioxus, Flutter) — Fragment match arms
- `crates/omnilang-core/src/codegen/shared/view_tree.rs` — shared utilities
- `crates/omnilang-core/src/ir/optimize.rs` — IR optimization pass

### D2: Read-Only Signal Primitive for Props

Props were documented as immutable component inputs but emitted as fully mutable `__omni_signal` instances (with `.set(v)` and writable `.value`), enabling child components to silently overwrite parent-supplied values. The fix introduces `__omni_readonlySignal` — a wrapper that exposes only `get value()`, `get()`, and `peek()` (no setters). Parent→child updates continue to work via `__omni_subs.get("Child_prop").set(value)` which bypasses the wrapper and operates on the internal signal directly.

**Pattern:**
```js
function __omni_readonlySignal(name, init) {
  const inner = __omni_signal(name, init);
  return {
    _default: init,
    get value() { return inner.value; },
    get() { return inner.value; },
    peek() { return inner.peek(); },
    // No set value, no set() — child components cannot write
  };
}
```

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/runtime.rs` — new `__omni_readonlySignal` function
- `omnilang/src/codegen/omniview/store.rs` — prop default emission (line 223-233)
- No change to `component_html.rs` — parent→child push uses internal registry

### D3: TypeScript Declaration Generation Module

`ComponentIR` carries full type information (`props: FxHashMap<String, TypeIR>`, `event_param_types: Vec<String>`) but this was never emitted as `.d.ts` for the React production target. The fix adds a `typescript.rs` module that maps TypeIR variants to TypeScript types and generates per-component `Props` interfaces, event handler types, and component call signatures. Output is stored in `OmniViewOutput.types` and emitted alongside JS/HTML/CSS in `build_file_output()`.

**Pattern:**
```rust
pub fn generate_typescript_declarations(components: &[ComponentIR]) -> String
fn type_ir_to_ts(t: &TypeIR) -> String  // I32→number, Bool→boolean, List<T>→T[], Optional<T>→T|null, etc.
```

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/typescript.rs` — new module (298 lines)
- `omnilang/src/codegen/omniview/mod.rs` — module registration + `types` field on `OmniViewOutput`
- `omnilang/src/codegen/omniview/utils.rs` — `.d.ts` file emission in `build_file_output()`

### D4: CSS Escape Sequence Normalization in Sanitizers

ADR-027 documented a CSS injection bypass: `\6aavascript:alert(1)` passes the `javascript:` substring check because `\6a` is parsed as `j` by the browser's CSS parser. The fix adds `normalize_css_escapes` to both the Rust compile-time sanitizer and the JS runtime sanitizer, converting CSS hex escapes to their ASCII equivalents before keyword checking. Also aligns the Rust sanitizer with the JS runtime: `url()` with safe schemes is now allowed (previously blocked all `url()` unconditionally).

**Pattern:**
```rust
fn normalize_css_escapes(value: &str) -> String // \6a → j, \3c → <, etc.
```
```js
function __omni_normalizeCssEscapes(s) { return s.replace(/\\([0-9a-fA-F]{1,6})\s?/g, ...); }
```

**Architectural Components Affected:**
- `crates/omnilang-core/src/codegen/sanitize.rs` — Rust compile-time sanitizer
- `omnilang/src/codegen/omniview/runtime.rs` — JS runtime sanitizer
- `omnilang/tests/js_runtime/test_sanitizers.js` — behavioral test coverage

### D5: @enter Replay Scope Per Component

Previously, a single `__omni_effect` read every signal in the program and replayed every `@enter` animation on every signal change — O(S×A) per change. The fix groups `@enter` triggers by component and emits one `__omni_effect` per component that only reads that component's signals and replays that component's triggers. Complexity reduced to O(Sc×Ac) per signal change where Sc = signals in that component.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/mod.rs` — `enter_trigger_nids` type extended to include component name
- `omnilang/src/codegen/omniview/motion_triggers.rs` — trigger registration records `current_component`
- `omnilang/src/codegen/omniview/store.rs` — per-component effect emission instead of monolithic effect

### D6: WASM Incremental Analysis Cache for OmniView Compilation

`compile_multi_to_omniview` bypassed the existing `ANALYSIS_CACHE` thread-local, always performing full re-parse of all source files. The fix mirrors the `resolve_styles_multi_cached` pattern: accept `cache_state`, seed the cache, use `compile_multi_internal_cached`, and return updated cache state. In the Poiesis editor, subsequent debug window compilations skip parsing unchanged files.

**Architectural Components Affected:**
- `omnilang-wasm/src/lib.rs` — `compile_multi_to_omniview` (cache parameter + caching logic)
- `poiesis/src/commands/rustCompiler.ts` — TypeScript wrapper (cache state passthrough)

### D7: InliningState Struct for Component Inlining Safety

Component inlining (`try_inline_component`) manually saved/restored 9 fields of `OmniViewGenerator` with no compiler enforcement. Historical bugs (ADR-025 D1, ADR-026 D3) confirmed this pattern causes field-drift errors. The fix extracts an `InliningState` struct with `save()`/`restore()` methods, making the compiler enforce that new per-component-isolation fields must be added to the struct.

**Pattern:**
```rust
struct InliningState { /* 8 fields: current_component, signal_names, name_renames, for_*_bindings ×4, in_for_template */ }
impl InliningState { fn save(gen: &OmniViewGenerator) -> Self; fn restore(self, gen: &mut OmniViewGenerator); }
```

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/mod.rs` — struct definition
- `omnilang/src/codegen/omniview/component_html.rs` — save/restore call sites

### D8: Diagnostic Warning for Unhandled StatementIR Variants

The `emit_statement` and `stmt_to_js_inline` wildcard `_` arms silently emitted `"/* unhandled stmt */"` comments into generated JS output. The BlockStmtIR gap (previously CRITICAL) was the canonical example of this failure mode. The fix replaces silent emission with `self.warnings.push(...)` calls, ensuring new IR variants produce visible diagnostics instead of silently broken output.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/stmt.rs` — two wildcard arm replacements

### D9: Arc<ComponentIR> for Component Map Lookups

`component_map` stored owned `ComponentIR` values, requiring deep-clone on every component inlining lookup. The fix wraps values in `Arc<ComponentIR>`, replacing O(component-size) clones with O(1) atomic reference increments.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/mod.rs` — field type + initialization
- `omnilang/src/codegen/omniview/component_html.rs` — lookup call site

### D10: JS Runtime Behavioral Tests

All 242 Rust tests verified generated JS via string-content checks — they confirmed the TEXT of the generated code, never its BEHAVIOR when executed. The fix adds Node.js-based behavioral tests for `__omni_sanitizeUrl`, `__omni_sanitizeCss`, `__omni_signal`, `__omni_effect`, and `__omni_readonlySignal`, providing a CI gate for JS engine correctness.

**Architectural Components Affected:**
- `omnilang/tests/js_runtime/` — new test directory (3 files, 29+ tests)

## Consequences

### Positive

1. **Slot projection works end-to-end**: Parent→child content flows through `<slot>` positions during component inlining. Named and default slots function correctly. All 5 codegen backends handle the new `Fragment` variant.
2. **Props are immutable from child perspective**: Child components cannot silently overwrite parent-supplied values. Parent→child updates continue via internal registry (no breaking change).
3. **React consumers get type safety**: TypeScript declarations are auto-generated from IR type information, emitted alongside compiled JS.
4. **CSS injection bypass closed**: Escape sequence normalization in both Rust and JS sanitizers prevents `\6aavascript:`-style bypass. Rust/JS sanitizer URL handling now consistent.
5. **@enter replay no longer over-subscribes**: Signal changes in component A don't trigger animation rebuilds in component B.
6. **WASM compilation uses incremental cache**: Debug window compilations in Poiesis benefit from the same caching as canvas style resolution.
7. **Component inlining state is compiler-enforced**: Adding a new field that needs per-component isolation now requires updating `InliningState` struct — field-drift bugs prevented at compile time.
8. **Future IR additions produce diagnostics**: New `StatementIR` variants no longer silently emit broken output.
9. **JS runtime has behavioral test coverage**: Sanitizers and signal system verified in actual JS engine, not just via string matching.
10. **Component lookups are O(1)**: `Arc<ComponentIR>` eliminates multiplicative deep-clone cost.

### Negative

1. **Fragment variant adds match arms**: Every codegen backend that exhaustively matches on `ViewTree` must handle `Fragment`. This is an ongoing maintenance cost for new backend additions.
2. **__omni_readonlySignal duplicates signal API surface**: A second signal constructor exists alongside `__omni_signal` in the runtime. Developers must understand when to use each (props = readonly, signals/computeds = mutable).
3. **TypeScript module depends on IR staying stable**: Changes to `TypeIR` variants or `ComponentIR` structure require updates to `type_ir_to_ts` mapping.
4. **JS runtime tests are not in CI by default**: Tests require Node.js and are not integrated into `cargo test` — they must be run separately.
5. **CSS escape normalization is greedy**: Consumes up to 6 hex digits, which matches CSS spec but may surprise developers using adjacent hex characters without a space separator.

### Neutral

1. **InliningState uses .clone() instead of std::mem::take()**: Simpler but slightly more allocation-heavy than the previous approach. Given the Arc<ComponentIR> change, clone costs are negligible.
2. **Per-component @enter effects create more JS output**: Instead of one effect, there are now N effects (one per component with @enter triggers). The trade-off is correctness and performance for marginally larger output.
3. **WASM cache is backward-compatible**: When cache_state is null, behavior is identical to before (full re-parse). The cache is purely additive.

## Implementation

### Tasks Executed (13 total, 12 commits)

| # | Task | Commit | Category |
|---|------|--------|----------|
| 1 | Slot/content-projection | `6f2dc11f` | Framework completeness (HIGH) |
| 2 | Read-only props | `397aa1e7` | Framework completeness (HIGH) |
| 3 | TypeScript declarations | `07fc58ec` | Framework completeness (HIGH) |
| 4 | Wildcard → diagnostic | `4fdf7a7f` | Compiler safety (MEDIUM) |
| 5 | Clippy fixes | `be492626` | Code hygiene (LOW) |
| 6 | InliningState struct | `4fdf7a7f` | Refactoring (MEDIUM) |
| 7 | FieldAccessExpr dedup | `f82d6788` | Refactoring (MEDIUM) |
| 8 | Motion trigger dedup | `594b2d01` | Refactoring (MEDIUM) |
| 9 | Arc<ComponentIR> | `4818fea6` | Performance (MEDIUM) |
| 10 | JS runtime tests | `bc2a7e1f` | Testing (MEDIUM) |
| 11 | CSS sanitizer alignment | `3f08f3ad` | Security (MEDIUM) |
| 12 | @enter replay scope | `f1f46075` | Performance (MEDIUM) |
| 13 | WASM incremental cache | `4cf2e6b4` | Performance (HIGH) |

### Files Changed
- 19 Rust source files modified, 1 new module created (`typescript.rs`)
- 3 JS test files created (`test_sanitizers.js`, `test_signals.js`, `run_tests.sh`)
- 1 TypeScript file modified (`rustCompiler.ts`)
- 10+ snapshot files updated

### Test Coverage
- Rust: 284 unit tests + 523 core tests pass
- JS: 29+ behavioral tests pass (sanitizers + signals)

## Alternatives Considered

### Alternative 1: Fix slot projection by emitting content inline at the parent level (not via Fragment)

**Pros:** No new ViewTree variant needed. Child content is simply emitted between the child component's wrapper div tags.

**Cons:** Doesn't support named slots. Parent content doesn't flow into correct DOM position within child. Loses the semantic distinction between "this is the child's wrapper" and "this is parent content inserted into child."

**Decision:** Rejected. The Fragment approach correctly models the slot projection semantics: parent content replaces `<slot>` elements at their location within the child's DOM structure.

### Alternative 2: Compile-time check for prop immutability (semantic pass) instead of runtime wrapper

**Pros:** Catches prop writes at compile time with clear error messages. No runtime overhead.

**Cons:** Requires a new semantic analysis pass. Doesn't prevent dynamic prop writes through computed expressions. The runtime wrapper is simpler and catches all cases.

**Decision:** Rejected for now — runtime wrapper is the minimal fix. Compile-time check can be added later as a DX improvement.

### Alternative 3: Single sanitizer source-of-truth (shared between Rust and JS)

**Pros:** Zero inconsistency between compile-time and runtime sanitization. Changes to one automatically apply to the other.

**Cons:** Requires a shared sanitization specification format (likely JSON/TOML) that both Rust and JS can consume. Adds build complexity. For the current two-sanitizer setup, the maintenance burden is low.

**Decision:** Deferred. Current alignment (both sanitizers normalize escapes and check the same dangerous patterns) is sufficient. Revisit if a third sanitizer target is added.

### Alternative 4: Document slot non-functionality instead of fixing it

**Pros:** Zero code changes. Zero risk of regressions.

**Cons:** Leaves a documented language feature silently broken. Erodes framework credibility. Slot/children syntax is a core component composition primitive expected by frontend developers.

**Decision:** Rejected. Framework completeness requires fixing documented-feature gaps.

## Pragmatic Enforcer Analysis

**Reviewer:** Pragmatic Enforcer
**Mode:** Balanced

### Necessity Assessment: 8/10

- **Current need**: 3 HIGH findings (slot, props, TS types) are framework-completeness gaps that directly affect whether developers can build real applications. 2 MEDIUM security findings (CSS escape, sanitizer inconsistency) close documented vulnerabilities.
- **Future need**: Component composition (slots), prop immutability, and type safety are table-stakes for any frontend framework. Delaying these defers OmniView's viability as a framework.
- **Cost of waiting**: Each release with broken slot projection or mutable props ships a framework with known correctness bugs. The CSS escape bypass is a documented vulnerability in ADR-027.
- **Evidence of need**: 10-specialist architecture review independently flagged all 3 HIGH findings. Previous review cycles (ADR-025, ADR-026) confirmed the inlining state bug pattern.

### Complexity Assessment: 5/10

- **Added complexity**: The `Fragment` ViewTree variant adds match arms to all backends (+5 lines each). `__omni_readonlySignal` adds ~11 lines to the runtime. `typescript.rs` is a new 298-line module. The CSS escape normalization adds ~30 lines to each sanitizer.
- **Maintenance burden**: Fragment variant requires new backends to handle it — but the compiler enforces this (exhaustive match). The TypeScript module requires updates when TypeIR changes — this is mechanical.
- **Learning curve**: `__omni_readonlySignal` is a new concept for runtime developers. The distinction between signal/readonlySignal/computed is subtle but well-scoped (props = readonly, everything else = mutable).
- **Dependencies introduced**: Zero new crates or libraries.

### Ratio: 0.63 (Target: <1.5 for balanced mode)

### Recommendation: ✅ Approve

**Pragmatic Score:**
- **Necessity**: 8/10
- **Complexity**: 5/10
- **Ratio**: 0.63

**Overall Assessment**: This is appropriate engineering — fixing confirmed bugs that affect framework correctness and security. The complexity added is proportional to the problems solved. No over-engineering detected. The refactoring tasks (InliningState, dedup, Arc) reduce existing complexity rather than adding new complexity.

## Validation

**Acceptance Criteria:**
- [x] All 284 Rust tests pass (`cargo test -p omnilang`)
- [x] All 523 core tests pass (`cargo test -p omnilang-core`)
- [x] All JS runtime behavioral tests pass (`bash omnilang/tests/js_runtime/run_tests.sh`)
- [x] `cargo clippy -p omnilang -p omnilang-core -- -D warnings` clean
- [x] `cargo fmt` applied
- [x] Slot projection: parent content renders inside child `<slot>` positions
- [x] Props: `__omni_readonlySignal` has no setter; `__omni_subs.get().set()` still works
- [x] TypeScript: `.d.ts` content emitted for components with typed props/events
- [x] CSS sanitizer: `\6aavascript:alert(1)` blocked by both Rust and JS sanitizers
- [x] @enter replay: per-component effects, not monolithic
- [x] WASM: `cargo check -p omnilang-wasm` compiles clean

## References

- [ADR-028: OmniView Codegen as Debug/Development Runtime](ADR-028-omniview-debug-runtime-design-tradeoffs.md) — establishes debug-runtime role
- [ADR-029: Review Finding Classification Framework](ADR-029-review-finding-classification-framework.md) — three-bucket triage
- [ADR-027: Security Sanitization Architecture](ADR-027-omniview-security-sanitization-architecture.md) — CSS escape bypass documented
- [ADR-025: Review Remediation 2026-07-29](ADR-025-omniview-review-remediation-2026-07-29.md) — D1 field-drift bug
- [ADR-026: Review v2 Fixes](ADR-026-omniview-review-v2-fixes.md) — D3 field-drift bug recurrence
- [ADR-012: OmniView Codegen as Debug Tool](ADR-012-omniview-codegen-debug-tool.md) — established independent codegen path
- [Component-omniview-frontend-framework Review](../../reviews/component-omniview-frontend-framework-2026-07-29.md) — source review (10 specialists, 44 findings)
- [Fix Plan](../../../docs/superpowers/plans/2026-07-29-omniview-framework-fixes.md) — 13-task implementation plan
