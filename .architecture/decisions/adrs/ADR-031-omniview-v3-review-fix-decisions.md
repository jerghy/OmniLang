# ADR-031: OmniView v3 Architecture Review — Fix Decisions (2026-07-29)

## Status

Accepted

## Context

The 2026-07-29 v3 architecture review ([component-omniview-2026-07-29-v3.md](../../reviews/component-omniview-2026-07-29-v3.md), 11 specialists, 30 findings) evaluated OmniView after the ADR-030 remediation wave (slot/content-projection, readonly props, TypeScript declarations). The review discovered that the remediation wave itself introduced regressions: 4 CRITICAL/HIGH bugs, 1 YAGNI violation, and 6 MEDIUM security/correctness issues. This ADR documents the architectural decisions made during the 11-task fix plan.

**Reference:** Architecture review v3, ADR-028 (Debug Runtime Trade-offs), ADR-029 (Classification Framework), ADR-030 (Framework Fix Plan).

## Decision Drivers

1. **Correctness**: The ADR-030 remediation wave introduced regressions — props emitted after computeds causes `ReferenceError`, slot matching uses Debug-format substring comparison, URL sanitizer has newline bypass
2. **Security**: CSS escape normalizer crashes on out-of-range code points (JS/Rust divergence), URL sanitizer allows `java\nscript:` bypass
3. **Framework trust**: Example app (`learn-omniview/TodoPage.omni`) has a real data-corruption bug that learners would copy
4. **CI integrity**: JS runtime behavioral tests (431 lines) exist but have zero automated execution — silent runtime regressions pass all Rust tests
5. **YAGNI discipline**: TypeScript declaration module (298 lines) has zero production consumers

## Decision

### D1: Fix Emission Order — Props Before Computeds

In `store.rs`, prop defaults (`__omni_readonlySignal`) were emitted AFTER computed declarations (`__omni_computed`). Since `__omni_computed` eagerly evaluates its function body at declaration time, a computed referencing a prop with a default causes `ReferenceError` at runtime.

**Fix**: Reorder blocks: Signal declarations → Prop defaults → Computed declarations. Pure block reorder, zero content changes. Added regression test using direct IR construction.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/store.rs` — block reorder (lines 211-234)
- `omnilang/src/codegen/omniview/tests_regression/mod.rs` — regression test

### D2: Fix Slot Name Matching — Direct Value Extraction

`has_slot_attr` in `component_html.rs` used `format!("{:?}", v).contains(slot_name)` where `v` is `ExprIR`. This compared against Rust Debug output (`LiteralExpr { value: "headerText" }`), causing substring false-positives (`"header"` matches `"headerExtra"`) and structural false-positives (`"value"` matches every `LiteralExpr`).

**Fix**: Extract actual string value from `ExprIR::LiteralExpr` via pattern matching, compare with `==` equality. Added regression test for substring false-positive scenario.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/component_html.rs` — `has_slot_attr` function rewrite

### D3: Fix URL Sanitizer — Strip CR/LF/TAB Before Scheme Matching

Browsers (Chromium, WebKit, Gecko) strip `\r`, `\n`, `\t` from URLs before navigation. `java\nscript:alert(1)` passed both Rust and JS sanitizers because `javascript:` substring check operated on the un-stripped value. The browser executes it as `javascript:alert(1)`.

**Fix**: Strip `\r\n\t` before scheme matching in all three sanitizer locations: JS runtime (`__omni_sanitizeUrl`), Rust `element.rs` (`is_dangerous_url`). Added 4 JS behavioral tests + 3 Rust test assertions.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/runtime.rs` — JS: `normalized = trimmed.replace(/[\r\n\t]/g, '')`
- `omnilang/src/codegen/omniview/view/element.rs` — Rust: `chars().filter(|c| !matches!(c, '\n' | '\r' | '\t'))`
- `omnilang/tests/js_runtime/test_sanitizers.js` — 4 new bypass tests

### D4: Fix CSS Escape Normalizer — fromCodePoint Range Check

CSS hex escapes collect up to 6 hex digits (max 0xFFFFFF). `String.fromCodePoint(code)` throws `RangeError` for values > 0x10FFFF or surrogates (0xD800-0xDFFF). The Rust `normalize_css_escapes` already handles this correctly via `char::from_u32(code)` which returns `None` for invalid code points. The JS version crashed.

**Fix**: Add range check `code <= 0x10FFFF && !(code >= 0xD800 && code <= 0xDFFF)` before `fromCodePoint`. Added 3 JS behavioral tests (out-of-range, surrogate, valid regression).

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/runtime.rs` — `__omni_normalizeCssEscapes` range guard
- `omnilang/tests/js_runtime/test_sanitizers.js` — 3 new edge-case tests

### D5: Add Event Handlers to Generated .d.ts Props Interface

`emit_props_interface` in `typescript.rs` only included `prop` declarations. Event handlers were emitted as orphaned `XxxHandler` type aliases, disconnected from the Props interface. React consumers using generated `.d.ts` get TypeScript errors on every event handler prop.

**Fix**: Add event handler fields (always optional, typed as `(detail: T) => void`) to the Props interface after prop fields. Added unit test.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/typescript.rs` — `emit_props_interface` event handler loop

### D6: Gate TypeScript Declarations — Skip Generation (Zero Consumers)

The `typescript.rs` module (298 lines) generates `.d.ts` files but zero production consumers, build pipelines, or projects use the output. The React production path has its own separate codegen backend with its own type generation. The module adds maintenance burden (every `ViewTree` change touches it, every `TypeIR` change requires updates) with no users.

**Decision**: Skip calling `generate_typescript_declarations` in the `generate()` pipeline. Keep the module code preserved for future use. Re-enable by uncommenting the block when a real consumer materializes.

**Alternatives considered**: (a) Remove the module entirely — rejected because the implementation is correct and may be needed soon; (b) Gate behind CLI flag — rejected as premature infra for a zero-consumer feature.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/mod.rs` — commented out generation call, `types = String::new()`
- `omnilang/src/codegen/omniview/tests.rs` — 3 integration tests updated to expect empty types

### D7: Integrate JS Runtime Behavioral Tests into `cargo test`

431 lines of JS behavioral tests (`test_signals.js`, `test_sanitizers.js`) existed but ran only via manual `bash run_tests.sh`. No CI integration. A runtime regression in signal/sanitizer logic would pass all 254 Rust tests undetected.

**Fix**: Create `omnilang/tests/js_runtime_tests.rs` — Rust integration tests that shell out to Node.js. Tests skip gracefully when Node.js is not available (`node_available()` guard). Both JS test files now run as part of `cargo test`.

**Architectural Components Affected:**
- `omnilang/tests/js_runtime_tests.rs` — new file (52 lines)

### D8: Fix Effect Exception Handling — Permanent Death on Exception

`e.run()` in `__omni_effect` cleared `_deps` before calling `fn()`. If `fn()` threw, `_deps` was already empty and the effect had no subscriptions — permanently dead. The `try/finally` at the outer call site only restored `__omni_activeEffect`, not the dependency subscriptions. This was finding N6 from the v2 review, still unfixed.

**Fix**: Wrap `fn()` in `try { fn(); } catch (err) { __omni_reportError("effect:run", err); }` inside `e.run()`. The effect re-subscribes to dependencies on the next signal change (deps are cleared first, then re-tracked during `fn()`). Added behavioral test for effect exception recovery.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/runtime.rs` — `__omni_effect` try/catch
- `omnilang/tests/js_runtime/test_signals.js` — exception recovery test

### D9: Recurse `inject_slot_children` into `Conditional.fallback`

Slot injection recursed into `Conditional.branches[].body` but skipped `fallback`. A `<slot>` inside an `@if`/`@else` fallback branch silently failed.

**Fix**: Add `let fallback = cond.fallback.map(|v| inject_slot_children(v, slot_children))`. Replace `..*cond` spread with explicit `fallback, transition_mode` fields to avoid duplicate field in struct update syntax.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/component_html.rs` — `ViewTree::Conditional` arm

### D10: Strip `slot` Attribute After Slot Routing

After `inject_slot_children` routed children to named slots, the `slot="name"` attribute remained on child elements. Browsers interpret `slot` as a Web Components slot assignment attribute, causing unexpected behavior.

**Fix**: Add `strip_slot_prop` helper that removes the `slot` prop from `ElementIR` props map. Apply via `.map(|c| strip_slot_prop(c))` when collecting named and default slot children.

**Architectural Components Affected:**
- `omnilang/src/codegen/omniview/component_html.rs` — `strip_slot_prop` helper + Slot arm changes

### D11: Fix learn-omniview TodoPage Data Corruption

In `TodoPage.omni`, `@onDelete={(index) => { TodoStore.remove(); }}` received the `@for` loop index but never passed it to `TodoStore.remove(id: i32)`. The `index` parameter was captured but unused; `remove()` received no argument → JS `undefined` → coerced to `0` → always deleted item with ID 0.

**Fix**: Change to `@onDelete={(index) => { TodoStore.remove(TodoStore.items[index].id); }}`. Pass the entity ID (`items[index].id`) instead of the array index.

**Architectural Components Affected:**
- `test/projects/learn-omniview/src/pages/TodoPage.omni` — one-line fix

## Consequences

### Positive

- **All 4 CRITICAL correctness bugs fixed**: emission order, slot matching, URL sanitizer bypass, CSS escape crash
- **JS runtime tests now CI-gated**: `cargo test` runs all signal/sanitizer behavioral tests via Node.js — silent runtime regressions no longer possible
- **TypeScript module preserved but zero-cost**: code kept for future use, zero pipeline overhead
- **Effect exception handling fixed**: N6 from v2 review, deferred across 2 review cycles, now resolved
- **Example app no longer teaches data-corruption**: learn-omniview TodoPage correctly deletes items by entity ID
- **Slot system hardened**: no Debug-format false-positives, no leftover `slot` DOM attributes, fallback branches correctly projected

### Negative

- **TypeScript `.d.ts` generation disabled**: the 298-line module exists but produces no output. If a React consumer materializes, re-enabling requires uncommenting one block and updating 3 integration tests
- **JS runtime tests require Node.js on CI**: graceful skip when unavailable, but CI must have Node.js installed for full coverage
- **CSS sanitizer CR/LF/TAB bypass not yet fixed**: the URL sanitizer fix was not extended to `__omni_sanitizeCss` (MEDIUM follow-up, documented in ADR-031)

### Neutral

- **9 commits added to main**: small, surgical changes — largest diff is ~30 lines
- **3 snapshot files updated**: standard for JS runtime changes
- **`is_dangerous_url` confirmed to exist only in `element.rs`**: plan originally referenced `crates/omnilang-core/src/codegen/sanitize.rs` incorrectly — verified as plan error, no fix needed there

## Implementation Strategy

### Blast Radius

All 11 fixes are surgical — single-function changes or single-block reorders. No API changes, no IR changes, no cross-backend changes. Largest change is `component_html.rs` (~30 lines added for `strip_slot_prop` + 4 lines changed for fallback recursion).

**Affected Components:**
- `runtime.rs` — 4 changes (URL sanitizer, CSS normalizer, effect handling) — JS runtime behavior
- `component_html.rs` — 3 changes (slot matching, fallback recursion, slot attribute stripping) — slot projection
- `store.rs` — 1 change (emission order) — JS output order
- `typescript.rs` — 1 change (event handlers in Props) — .d.ts output format
- `mod.rs` — 1 change (gate TS generation) — pipeline skip
- `tests/js_runtime_tests.rs` — 1 new file (CI integration)

### Reversibility

**Reversibility Level**: High — all changes are trivially reversible.

| Decision | Revert Cost | Revert Method |
|----------|-------------|---------------|
| D1 (emission order) | S | Swap two blocks back |
| D2 (slot matching) | S | Revert one function |
| D3 (URL sanitizer) | S | Remove `normalized` line in 3 locations |
| D4 (fromCodePoint) | S | Remove range check |
| D5 (event handlers) | S | Remove event loop from `emit_props_interface` |
| D6 (gate TS) | S | Uncomment block |
| D7 (JS CI) | S | Delete `js_runtime_tests.rs` |
| D8 (effect fix) | S | Remove try/catch |
| D9 (fallback) | S | Remove fallback line |
| D10 (strip slot) | S | Change `.map()` back to `.cloned()` |
| D11 (TodoPage) | XS | Revert one line |

### Social Cost

**Learning Curve**: Low — all fixes are mechanical (reorder, replace, add guard). No new abstractions, no new patterns, no new APIs.

**Cognitive Load**: Minimal — each fix is self-contained within a single function or block.

## Alternatives Considered

### Alternative 1: Fix TypeScript declarations instead of gating them

Keep generating `.d.ts` files and fix all the issues (event handlers in Props, JSX import, exhaustive type matches). This is what the TypeScript/React specialist recommended.

**Pros:** Complete TypeScript support ready for consumers.  
**Cons:** Maintains 298 lines of code with zero users. Every `TypeIR`/`ViewTree` change requires updating the TS generator. Premature optimization.  
**Decision:** Rejected in favor of D6 (gate, re-enable when consumer exists).

### Alternative 2: Remove TypeScript module entirely

Delete `typescript.rs` and the `.types` field from `OmniViewOutput`. This is what the Pragmatic Enforcer recommended.

**Pros:** Cleanest — zero maintenance burden.  
**Cons:** Loses the implementation. Re-creating it later costs more than preserving it.  
**Decision:** Rejected in favor of D6 (keep code, skip call). Compromise position.

### Alternative 3: Compile-time analysis for prop immutability

Instead of the runtime `__omni_readonlySignal` wrapper (ADR-030 D2), add a compile-time pass that validates child components never write to parent props.

**Pros:** Zero runtime cost, catches errors at build time.  
**Cons:** Complex analysis pass, requires inter-component data flow tracking. Over-engineered for a debug runtime.  
**Decision:** Rejected. ADR-030 D2's runtime wrapper approach is correct and sufficient.

## Pragmatic Enforcer Analysis

**Mode**: Balanced (per `.architecture/config.yml`)

**Overall Decision Complexity Assessment**: This ADR documents 11 independent fix decisions, each of which is minimal and surgical. The aggregate complexity is in the ADR's documentation, not the code — each fix is a single-function change. No new abstractions, no new patterns, no new dependencies.

### Per-Decision Assessment

| Decision | Necessity | Complexity | Ratio | Verdict |
|----------|-----------|------------|-------|---------|
| D1 (emission order) | 10/10 | 1/10 | 0.10 | ✅ Critical bug |
| D2 (slot matching) | 8/10 | 2/10 | 0.25 | ✅ Real bug, 4 specialists flagged |
| D3 (URL sanitizer) | 8/10 | 2/10 | 0.25 | ✅ Security bypass |
| D4 (fromCodePoint) | 6/10 | 2/10 | 0.33 | ✅ JS/Rust divergence |
| D5 (event handlers) | 5/10 | 1/10 | 0.20 | ✅ Fixes broken .d.ts |
| D6 (gate TS) | 6/10 | 1/10 | 0.17 | ✅ Removes unused code path |
| D7 (JS CI) | 7/10 | 2/10 | 0.29 | ✅ Closes CI gap |
| D8 (effect fix) | 6/10 | 2/10 | 0.33 | ✅ N6 from v2, deferred 2 cycles |
| D9 (fallback) | 4/10 | 2/10 | 0.50 | ✅ Edge case fix |
| D10 (strip slot) | 3/10 | 2/10 | 0.67 | ✅ Browser compat |
| D11 (TodoPage) | 5/10 | 1/10 | 0.20 | ✅ Example app correctness |

**All ratios < 1.0** — no over-engineering concerns. The cumulative 11 fixes represent pruning and hardening, not feature addition.

## Validation

**Acceptance Criteria:**
- [x] All 4 CRITICAL/HIGH correctness bugs fixed and tested
- [x] JS runtime behavioral tests run as part of `cargo test`
- [x] `cargo test -p omnilang --lib` — 254 passed, 0 failed
- [x] `cargo test -p omnilang js_runtime` — 2 JS integration tests pass
- [x] JS behavioral tests (`test_signals.js`, `test_sanitizers.js`) — all pass
- [x] `cargo clippy -p omnilang --lib -- -D warnings` — clean
- [x] Example app (`learn-omniview/main.omni`) compiles without new errors
- [ ] CSS sanitizer CR/LF/TAB bypass follow-up (deferred, documented)

**Testing Approach:**
- Rust regression tests for emission order, slot matching
- JS behavioral tests for URL sanitizer bypass, CSS escape normalization, effect exception recovery
- Rust integration tests (`js_runtime_tests.rs`) for CI-gated JS test execution
- Unit test for TypeScript Props interface event handlers

## References

- [Architecture Review v3](../../reviews/component-omniview-2026-07-29-v3.md) — 11-specialist review (30 findings)
- [ADR-028: Debug Runtime Trade-offs](../adrs/ADR-028-omniview-debug-runtime-design-tradeoffs.md) — 14 intentional design decisions
- [ADR-029: Review Finding Classification](../adrs/ADR-029-review-finding-classification-framework.md) — three-bucket framework
- [ADR-030: Framework Fix Plan](../adrs/ADR-030-omniview-framework-fix-plan-2026-07-29.md) — previous remediation wave (13 tasks)
- [Fix Plan](../../../docs/superpowers/plans/2026-07-29-omniview-v3-review-fixes.md) — 11-task implementation plan
