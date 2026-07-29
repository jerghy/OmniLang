# Architecture Review: OmniView Frontend Framework v3

**Date**: 2026-07-29
**Review Type**: Component (Comprehensive Multi-Perspective)
**Reviewers**: Compiler & Language Architect, TypeScript & React Specialist, Security Specialist, Rust Expert, Maintainability Expert, Performance Specialist, Domain Expert, Systems Architect, Implementation Strategist, AI Engineer, Pragmatic Enforcer
**Previous Reviews**: [component-omniview.md](component-omniview.md) (v1, 46 findings), [component-omniview-frontend-framework-2026-07-29.md](component-omniview-frontend-framework-2026-07-29.md) (v2, 44 findings)
**Key ADRs**: ADR-028 (Debug Runtime Trade-offs), ADR-029 (Classification Framework), ADR-030 (Fix Plan)

---

## Executive Summary

OmniView has completed its third major remediation wave (ADR-030, 13 tasks across 12+ commits). The three HIGH framework gaps from the previous review (slot/content-projection, readonly props, TypeScript declarations) have been addressed. The framework is now feature-complete for documented language capabilities.

However, this review discovered **3 NEW HIGH-severity bugs** introduced by the remediation wave itself, plus **1 HIGH YAGNI concern** and **6 MEDIUM security/correctness issues**:

1. **Props emitted after computeds causes ReferenceError** — when a computed references a prop with a default, the prop signal doesn't exist yet when the computed eagerly evaluates
2. **`has_slot_attr` uses Debug-format substring matching** — slot names that are substrings of each other (e.g., `"header"` vs `"headerExtra"`) cause false-positive slot routing
3. **Event handlers missing from generated `.d.ts` Props interface** — React consumers get TypeScript errors on every event handler prop
4. **TypeScript declaration module has zero consumers** — 298 lines added for a feature no project, pipeline, or consumer uses (Pragmatic Enforcer: **REMOVE**)
5. **URL sanitizer bypass via newlines/tabs** — `java\nscript:alert(1)` passes both Rust and JS sanitizers
6. **`inject_slot_children` skips `Conditional.fallback`** — slots inside `else` branches silently fail
7. **Effect exception handling permanently kills effects** — if `fn()` throws, subscriptions are cleared and never re-registered
8. **`String.fromCodePoint` throws on out-of-range CSS hex escapes** — diverges from Rust behavior, crashes JS runtime
9. **learn-omniview `TodoPage.omni` has a real data-corruption bug** — array index passed as entity ID, deletes wrong items
10. **JS runtime behavioral tests not integrated into CI** — 431 lines of tests with zero automated execution

**Overall Assessment**: Adequate → Strengthening. Framework completeness is achieved, but the remediation wave introduced regressions that need immediate attention. The 10 findings above are distributed across 6 NEW bugs (items 1-6), 2 existing deferred issues (items 7, 10), 1 security gap (item 8), and 1 example-app bug (item 9).

---

## What's Changed Since v2 Review

13 tasks from ADR-030 were executed across 12+ commits:

| Commit | Task | Status |
|--------|------|--------|
| `6f2dc11f` | Slot/content-projection (Fragment ViewTree) | ✅ Implemented |
| `07fc58ec` | Readonly signal primitive for props | ✅ Implemented |
| `397aa1e7` | TypeScript declaration generation | ✅ Implemented |
| `3f08f3ad` | CSS sanitizer alignment + escape normalization | ✅ Implemented |
| `bc2a7e1f` | JS engine behavioral tests | ✅ Implemented |
| `4818fea6` | Arc\<ComponentIR\> in component_map | ✅ Implemented |
| `f1f46075` | @enter replay scope per component | ✅ Implemented |
| `4fdf7a7f` | InliningState save/restore struct | ✅ Implemented |
| `594b2d01` | format_simple_trigger_js helper | ✅ Implemented |
| `f82d6788` | resolve_store_field_access helper | ✅ Implemented |
| `22a602b3` | emit_class_map_effects helper | ✅ Implemented |
| `0fbcc243` | emit_element_css_only_capture helper | ✅ Implemented |
| `be492626` | Clippy warning fixes | ✅ Implemented |

**Net assessment**: All ADR-030 tasks completed. The framework is functionally complete for documented language features. However, the remediation wave introduced regressions (see findings below).

---

## Cross-Cutting Themes

### Theme 1: `has_slot_attr` Debug-String Matching (4 specialists independently flagged)

**Specialists**: Rust Expert (C1), Performance Specialist (C6), Implementation Strategist (C1), Compiler Architect (H2)

`component_html.rs:259` uses `format!("{:?}", v).contains(slot_name)` to match slot attributes. This means:
- A slot named `"value"` matches ALL `LiteralExpr` values (Debug output always contains `value:`)
- A slot named `"header"` matches `"headerExtra"` (substring match)
- A change to `ExprIR`'s Debug format silently changes behavior

**Consensus**: HIGH severity. Replace with direct `ExprIR::LiteralExpr` value extraction.

### Theme 2: JS Runtime Tests Have Zero Automated Execution (3 specialists)

**Specialists**: AI Engineer (C1), Implementation Strategist (C2), Pragmatic Enforcer (Item 4)

431 lines of behavioral tests exist in `omnilang/tests/js_runtime/` but run only via manual `bash run_tests.sh`. Not in `cargo test`, not in CI, not in any build pipeline. The Pragmatic Enforcer's verdict: "Cost paid, value zero."

**Consensus**: HIGH priority. Either integrate into `cargo test` (5-line change) or delete the test infrastructure.

### Theme 3: God Object Stable But Not Improving (6 specialists)

**Specialists**: Systems Architect (C1), Maintainability Expert (C1, C4, C7), Implementation Strategist, Rust Expert, Performance Specialist

`OmniViewGenerator` remains at 45 fields. The InliningState extraction was a step forward but doesn't enforce save/restore completeness at compile time. `store.rs` has its own ad-hoc save/restore duplicating the pattern.

**Consensus**: MEDIUM severity. Monitor. Begin with `NameResolver` extraction (9 fields, S effort). Full decomposition only if field count crosses 55.

### Theme 4: TypeScript Declaration Module Has No Consumer (Pragmatic Enforcer)

`typescript.rs` (298 lines) generates `.d.ts` files. Zero production consumers exist. No project, build pipeline, or consumer reads `output.types`. The React production path uses its own separate codegen backend with its own type generation. The OmniView iframe debug runtime doesn't use TypeScript.

**Consensus**: HIGH (YAGNI). Remove the module; re-add when a real consumer materializes.

### Theme 5: Cross-Target Slot/Content-Projection Gap

**Specialists**: Compiler Architect (M3), Systems Architect, Domain Expert

Slot resolution via `inject_slot_children` only runs in OmniView codegen. React emits no-op comments; Flutter emits `SizedBox.shrink()`; Dioxus emits no-ops. A user iterating in OmniView debug sees correct slot projection; deploying to React silently loses it.

**Consensus**: MEDIUM. Document the limitation. Cross-target slot resolution is L effort and not justified for a debug runtime.

---

## Findings Summary

### NEW HIGH-Severity Findings

| # | Finding | Specialist | File:Line |
|---|---------|-----------|-----------|
| H1 | Props emitted AFTER computeds — ReferenceError when computed references prop | Compiler Architect | `store.rs:216-234` |
| H2 | `has_slot_attr` Debug-substring false-positive slot matching | 4 specialists | `component_html.rs:259` |
| H3 | Event handlers omitted from Props interface in `.d.ts` | TypeScript/React | `typescript.rs:46-60` |
| H4 | TypeScript declarations have zero consumers — YAGNI violation | Pragmatic Enforcer | `typescript.rs` (entire module) |

### NEW MEDIUM-Severity Findings

| # | Finding | Specialist | File:Line |
|---|---------|-----------|-----------|
| M1 | URL sanitizer bypass via `java\nscript:` (newlines not stripped) | Security | `runtime.rs:145-158`, `sanitize.rs:24-27` |
| M2 | `inject_slot_children` doesn't recurse into `Conditional.fallback` | Compiler Architect | `component_html.rs:235-248` |
| M3 | Effect exception leaves effect permanently dead (N6 from v2, unfixed) | Implementation Strategist | `runtime.rs:96-107` |
| M4 | `String.fromCodePoint` throws on out-of-range CSS hex escapes | Security | `runtime.rs:180-183` |
| M5 | Unnecessary ViewTree clone when no slot children exist | Performance | `component_html.rs:112-120` |
| M6 | `slot` attribute not stripped from DOM after slot routing | Implementation Strategist | `component_html.rs:265-269` |
| M7 | `typekind_to_ts` catch-all panic defeats exhaustiveness checking | Maintainability + TS/React | `typescript.rs:127-137` |
| M8 | `store.rs` has ad-hoc save/restore duplicating InliningState pattern | Maintainability | `store.rs:49-51, 107-114` |
| M9 | Event param types use string mapping, losing structural type info | TypeScript/React | `typescript.rs:196-206` |
| M10 | `__omni_computed` `get()` returns internal signal value, bypassing dirty check | TypeScript/React | `runtime.rs:91` |
| M11 | Dead `signal.peek()` statement in `__omni_computed` | TypeScript/React | `runtime.rs:82` |

### NEW LOW-Severity / Example-App Findings

| # | Finding | Specialist | File:Line |
|---|---------|-----------|-----------|
| L1 | `TodoPage.omni` data corruption: index passed as entity ID | Domain Expert | `test/.../TodoPage.omni:56` |
| L2 | `TodoList.omni` dead `filterStatus` — filter declared but unused | Domain Expert | `test/.../TodoStore.omni` |
| L3 | JSX import missing from generated `.d.ts` | TypeScript/React | `typescript.rs:106-109` |
| L4 | Editor/debug bridge highlight color divergence: `#4F46E5` vs `#6366f1` | Systems Architect | `editor.rs:82`, `debug.rs:84` |
| L5 | Named/inline motion trigger duplication (250 lines of parallel match arms) | Maintainability | `motion_triggers.rs:396-647` |
| L6 | 10+ querySelector sites interpolate nid without `CSS.escape()` | Implementation Strategist | Multiple files |
| L7 | `__mergeTouches` unconditionally emitted (~400 bytes) | Compiler Architect | `mod.rs:343-354` |

### Deferred from v2 Review (Still Open)

| # | Finding | Status |
|---|---------|--------|
| D1 | Effect exception handling (was N6) | **Still open** — M3 in this review |
| D2 | JS runtime tests not in CI | **Still open** — Theme 2 |
| D3 | NameResolver extraction (God Object Phase 1) | **Still deferred** |
| D4 | learn-omniview full-pipeline test | **Still deferred** |
| D5 | nid querySelector escaping | **Still deferred** |
| D6 | editor_mode/debug_mode → ViewMode enum | **Still deferred** |
| D7 | Conditional DOM optimization | **Still deferred** |
| D8 | Remove 222 frozen tests | **Still deferred** |

---

## Individual Member Reviews (Verbatim Summaries)

### Compiler & Language Architect — Dr. Lin Wei

**Key finding**: H1 — Props emitted after computeds. Emission order is (1) signals, (2) computeds, (3) prop defaults. If a computed references a prop, the prop signal doesn't exist when `__omni_computed` eagerly evaluates its function. This produces a `ReferenceError` at runtime.

**Also found**: H2, M2. **Plus**: Slot projection is OmniView-only, creating cross-target semantic gap. `inject_slot_children` doesn't recurse into Conditional.fallback or ListRendering.

**Recommendations**: Fix emission order (S, immediate). Fix slot matching (S, immediate). Add fallback recursion (S, immediate).

### TypeScript & React Specialist — Emma Rodriguez

**Key finding**: H3 — Event handlers not included in generated Props interface. React consumers get TypeScript errors on every event-bound component. The React codegen correctly includes event handlers in Props; OmniView's `typescript.rs` emits them as orphaned types.

**Also found**: M7, M9, M10, M11, L3. **Strengths**: Readonly signal pattern is sound. Slot projection is correctly implemented. React codegen is idiomatic.

**Recommendations**: Add event handlers to Props interface (S, immediate). Fix panic to exhaustive match (S, immediate). Add JSX import (S, immediate).

### Security Specialist

**Key finding**: M1 — URL sanitizer bypass. `java\nscript:alert(1)` passes both Rust and JS sanitizers because newlines/tabs aren't stripped before `javascript:` substring check. Chrome's URL parser strips these characters before navigation.

**Also found**: M4 — `String.fromCodePoint` throws RangeError on CSS hex escapes > 0x10FFFF, diverging from Rust's graceful fallback. L: `has_slot_attr` (shared finding). L: Readonly signal bypassable via `__omni_subs` (documentation gap).

**Recommendations**: Strip CR/LF/TAB before scheme matching (S, immediate). Add fromCodePoint range check (S, immediate). Add bypass test cases (S, immediate).

### Rust Expert — Katya Volkov

**Key finding**: C1 — `has_slot_attr` Debug-string matching (shared finding). The `InliningState` extraction is correct but lacks compile-time enforcement.

**Strengths**: No unsafe code in omniview codegen. All unwrap/expect calls are in test code or infallible writes. `Arc<ComponentIR>` correctly applied. `format_simple_trigger_js` eliminated ~200 lines of IIFE duplication.

**Recommendations**: Fix slot matching (S, immediate). Change typekind_to_ts panic to fallback (S, immediate). Add InliningState round-trip test (S, short-term). Cache all_components() result (M, long-term).

### Performance Specialist

**Key finding**: M5 — Unnecessary ViewTree clone in slot-projection when no slot children exist. Component rendered 20 times clones its view tree 20 times even when no slot injection is needed. ~300KB per compilation for 20-instance components.

**Also found**: Effect cleanup return values discarded for attribute/class/interpolation bindings — orphaned effects accumulate in signal subscriber sets across hot-reload cycles. Missing computed signal behavioral tests. `has_slot_attr` (shared finding).

**Recommendations**: Avoid clone when slot_children is empty (S, immediate). Add computed signal + diamond dependency tests (S, short-term). Add effect cleanup registration (M, long-term).

### Maintainability Expert

**Key finding**: InliningState is a step forward but doesn't enforce save/restore completeness. `store.rs` has ad-hoc save/restore duplicating the pattern. Generator struct at 45 fields, mod.rs at 787 lines approaching the 800-line limit. Motion trigger named/inline duplication at ~250 lines.

**Strengths**: InliningState centralization. format_simple_trigger_js extraction. Debug assertions on state transitions. JS runtime test infrastructure exists.

**Recommendations**: Convert typekind_to_ts to exhaustive match (S, immediate). Add InliningState symmetry test (M, immediate). Extract StoreEmissionState struct (S, short-term). Decompose Generator into sub-structs (L, long-term).

### Domain Expert

**Key finding**: L1 — `TodoPage.omni:56` has a real data-corruption bug. The `@onDelete` handler receives the `@for` loop index but passes it as an entity ID to `TodoStore.remove(id: i32)`. The parameter is never actually passed to the store call — it always deletes ID 0 (JS undefined→0 coercion).

**Also found**: `@for` binding domain model fragmented across three parallel vectors (`for_text_bindings`, `for_class_bindings`, `for_attr_bindings`). Missing Context/Provider domain pattern makes `@store` the only state injection strategy. `serde_json::Value` infrastructure leaks into IR domain model via `$signal` magic keys.

**Strengths**: Ubiquitous language is cohesive and well-proven by 80+ .omni files. `design` block is a genuine domain innovation — language-level design system with compile-time token resolution. css_only mode correctly serves the editor domain.

**Recommendations**: Fix TodoPage.omni bug (S, immediate). Fix TodoList filter (S, short-term). Introduce ForBinding enum (M, short-term). Consider ContextIR for scoped state (L, long-term).

### Systems Architect

**Key finding**: Dual bridge implementation duplication — `editor.rs` and `debug.rs` implement the same concepts (highlight, nidMap, replaceComponent) through different channels. Already diverged: highlight color `#4F46E5` vs `#6366f1`.

**Also found**: OmniViewOutput carries 69% editor-only metadata fields. ADR process at ~10 ADRs/day — documentation-driven development with diminishing returns. Animation runtime dual-persistence with version skew risk.

**Strengths**: Clean IR-centric architecture proven by Fragment expansion. Well-documented intentional trade-offs (ADR-028). Effective WASM FFI serialization contract. Per-component @enter replay scoping.

**Recommendations**: Merge editor/debug bridge logic (S, immediate). Add round-trip serde test for OmniViewOutput (S, immediate). Extract OutputAccumulator struct (M, short-term). Define two-tier ADR process (S, short-term).

### Implementation Strategist

**Key finding**: Confirmed H2 (slot matching). Confirmed M3 (effect exception handling — N6 from v2, still unfixed).

**Also found**: M6 — `slot` attribute not stripped after routing. Fragment blast radius fully enumerated across 19 files. ReadonlySignal is trivially reversible. 10+ querySelector sites interpolate nid without CSS.escape().

**Strengths**: Sequencing correct (framework gaps first). Reversibility of readonlySignal is exemplary. InliningState prevents known bug class.

**Recommendations**: Fix slot matching (S, immediate). Strip slot attribute (S, immediate). Integrate JS tests into CI (S, short-term). Fix effect exception handling (S, short-term).

### AI Engineer

**Key finding**: Theme 2 — JS runtime tests not in CI (HIGH). Meta-process stack (ADR-029 + Pragmatic Enforcer) is the project's most innovative asset — a self-improving review methodology.

**Also found**: Knowledge base lacks topical index (66 memory files, temporally organized). `__omni_reportError` is well-designed observability. No performance metrics in runtime (effect run counts, signal propagation counts). No developer-facing `onError` API.

**Strengths**: Meta-process maturity. Runtime error observability. JS behavioral test infrastructure. InliningState struct pattern. Warning collection infrastructure.

**Recommendations**: Integrate JS tests into cargo test (S, immediate). Add computed signal tests (S, immediate). Create topical index in MEMORY.md (S, immediate). Pre-load ADR-028 trade-offs into review prompts (S, short-term).

### Pragmatic Enforcer — YAGNI Guardian

**Key finding**: H4 — TypeScript declarations have zero consumers. 298 lines for a feature no project uses. **Recommendation: REMOVE**.

**Also found**: JS runtime tests: "cost paid, value zero" without CI integration. 9-backend strategy: 5 of 9 are maintenance-only burden (ratio 2.0). ADR process: 29 ADRs is borderline sustainable. Complexity trend: net growth of ~200-500 lines per session.

**Complexity/Necessity ratios**: TypeScript decls 2.0 (remove), Slots 0.86 (keep), ReadonlySignals 0.33 (keep), JS tests 1.0 (simplify), Arc\<ComponentIR\> 0.20 (keep), 9-backend 2.0 (simplify), ADRs 0.83 (simplify), Complexity trend 1.14 (simplify).

**Recommendations**: Remove typescript.rs (HIGH). CI-integrate or delete JS tests (HIGH). Mark non-default backends as experimental (MEDIUM). Introduce code budget guideline (MEDIUM).

---

## Trade-offs and Disagreements

### TypeScript Declarations: Keep vs Remove

- **Typescript/React Specialist**: Keep and fix (event handlers, JSX import, exhaustive matches) — it's close to correct and valuable for React consumers
- **Pragmatic Enforcer**: Remove immediately — zero consumers, YAGNI violation
- **Systems Architect**: Keep with guard — it's a new output path, evaluate after a React project actually consumes it
- **Resolution**: Keep the module but gate it behind a `--typescript` CLI flag (off by default). This eliminates the maintenance burden for non-consumers while preserving the code for future use. **Compromise**: Remove the `.types` field from `OmniViewOutput`; generate TS only when explicitly requested.

### God Object Decomposition Priority

- **Systems Architect**: Extract OutputAccumulator first (accumulator fields)
- **Maintainability Expert**: Extract NameResolver first (9 fields, highest impact-per-effort)
- **Rust Expert**: Don't decompose yet — "Monitor. Don't refactor until concrete bug or field count crosses 55"
- **Resolution**: Follow Maintainability Expert's plan (NameResolver first, S effort). This removes the highest-risk field group (save/restore sensitive) with the least code motion.

### 9-Backend Strategy

- **Pragmatic Enforcer**: Reduce to 3-4 active backends
- **Systems Architect**: Mark experimental, don't remove — shared IR changes keep them alive at low cost
- **Domain Expert**: The 9-target promise is a domain value proposition; removing backends undermines the "write once, target anywhere" story
- **Resolution**: Mark 5 non-default backends as `experimental` in docs. Don't remove code. Accept that shared IR changes will keep them compiling. This preserves the domain promise while being honest about maintenance reality.

---

## Prioritized Recommendations

### CRITICAL — Fix Immediately (This Session)

| # | Action | Effort | Specialists | Files |
|---|--------|--------|-------------|-------|
| C1 | **Fix prop emission order**: Move prop defaults BEFORE computed declarations | S (2 lines) | Compiler Architect | `store.rs:216-234` |
| C2 | **Fix `has_slot_attr`**: Replace Debug format matching with direct `ExprIR::LiteralExpr` extraction | S (8 lines) | 4 specialists | `component_html.rs:254-263` |
| C3 | **Fix URL sanitizer newline bypass**: Strip `\r\n\t` before scheme matching in both Rust and JS | S (10 lines) | Security | `runtime.rs:145`, `sanitize.rs:24`, `element.rs:24` |
| C4 | **Fix `fromCodePoint` range check**: Add 0x10FFFF + surrogate bounds check to JS CSS escape normalizer | S (5 lines) | Security | `runtime.rs:180-183` |

### HIGH — Fix Within 1-2 Sessions

| # | Action | Effort | Specialists | Files |
|---|--------|--------|-------------|-------|
| H1 | **Add event handlers to Props interface** in `.d.ts` generation | S (15 min) | TypeScript/React | `typescript.rs:46-60` |
| H2 | **Gate TypeScript declarations behind CLI flag** (or remove if no consumer within 1 week) | S (30 min) | Pragmatic Enforcer | `typescript.rs`, `mod.rs` |
| H3 | **Integrate JS runtime tests into `cargo test`** | S (30 min) | AI Engineer, Impl Strategist | Add Rust `#[test]` shelling to node |
| H4 | **Fix effect exception handling**: Wrap `fn()` in try/catch inside `e.run()` | S (5 min) | Impl Strategist | `runtime.rs:96-107` |
| H5 | **Recurse into `Conditional.fallback`** in `inject_slot_children` | S (10 lines) | Compiler Architect | `component_html.rs:235-248` |
| H6 | **Strip `slot` attribute after routing** to prevent browser Web Component interference | S (5 lines) | Impl Strategist | `component_html.rs:265-269` |
| H7 | **Fix learn-omniview `TodoPage.omni` data-corruption bug** | S (5 min) | Domain Expert | `test/.../TodoPage.omni:56` |

### MEDIUM — This Sprint

| # | Action | Effort | Files |
|---|--------|--------|-------|
| M1 | Avoid unnecessary `child_view.clone()` when `slot_children.is_empty()` | S | `component_html.rs:112-120` |
| M2 | Add computed signal behavioral tests (diamond deps, lazy eval, dirty tracking) | S | `tests/js_runtime/test_signals.js` |
| M3 | Convert `typekind_to_ts` catch-all panic to exhaustive match | S | `typescript.rs:127-137` |
| M4 | Add InliningState symmetry test (save/restore round-trip assertion) | M | `mod.rs` (test module) |
| M5 | Extract `StoreEmissionState` struct for store.rs ad-hoc save/restore | S | `store.rs:49-114` |
| M6 | Merge editor/debug bridge shared JS (eliminate highlight color divergence) | S | `editor.rs`, `debug.rs` |
| M7 | Add CSS escape bypass test cases to JS sanitizer tests | S | `tests/js_runtime/test_sanitizers.js` |
| M8 | Create topical index in MEMORY.md | S | `memory/MEMORY.md` |
| M9 | Add `// NOTE:` comments linking runtime.rs functions to JS test files | S | `runtime.rs` |
| M10 | Replace three parallel `@for` binding vectors with `ForBinding` enum | M | `mod.rs:511-517`, multiple |

### LONG-TERM — Next 2-3 Sprints

| # | Action | Effort |
|---|--------|--------|
| L1 | Extract NameResolver struct (God Object Phase 1 — 9 fields) | S |
| L2 | Add learn-omniview full-pipeline compilation test | M |
| L3 | Consolidate editor_mode/debug_mode into ViewMode enum | S |
| L4 | Add Context/Provider domain pattern (ContextIR) | L |
| L5 | Decompose Generator struct into sub-structs (JsBuffer, ScopeState, ForTemplateState) | L |
| L6 | Mark non-default backends as experimental in docs | S |
| L7 | Define two-tier ADR process (full vs lightweight decision records) | S |
| L8 | Introduce code budget guideline: remove 50 lines per 100 added | Process |
| L9 | Pre-load ADR-028 trade-off list into review agent prompts | S |

---

## Risks If Unaddressed

| Risk | Severity | Trigger | Impact |
|------|----------|---------|--------|
| Computed referencing prop → ReferenceError | **CRITICAL** | Any component with `computed` referencing a `prop` | Silent runtime crash with no compiler warning |
| Named slot false-positive routing | **HIGH** | Slot names that are substrings of each other | Content appears in wrong slot, broken layout |
| URL sanitizer newline bypass | **HIGH** | User input bound to `href`/`src` attribute | XSS via `javascript:` URL execution |
| Event handlers missing from `.d.ts` | **HIGH** | React consumer using generated types | TypeScript errors on every event-handling component |
| JS runtime regression passes CI undetected | **HIGH** | Any change to signal/sanitizer runtime | Broken debug runtime ships to Poiesis consumers |
| Effect permanently dead on exception | **MEDIUM** | Any signal effect that throws once | UI silently freezes with no error recovery |
| CSS hex escape RangeError crash | **MEDIUM** | `\110000` in signal-bound CSS value | JS runtime crash during sanitization |
| `Conditional.fallback` slot projection broken | **MEDIUM** | Slot inside `else` branch | Content silently missing |
| Cross-target slot gap surprises users | **MEDIUM** | Debug (OmniView) → Production (React) deploy | Slot-projected content disappears |
| `TodoPage.omni` data corruption ships to learners | **MEDIUM** | Users copy example code | Deployed apps delete wrong items |

---

## Follow-Up Plan

1. **Immediate** (this session): Fix C1-C4 (4 CRITICAL bugs, ~1 hour total)
2. **Today**: Fix H1-H7 (7 HIGH items, ~2 hours total)
3. **This sprint**: Address M1-M10 (10 MEDIUM items, ~1 day total)
4. **Next sprint**: Begin L1-L3 (3 quick long-term items)
5. **Backlog**: L4-L8 (strategic improvements)

### Verification Gate

Before the next architecture review:
- [ ] All 4 CRITICAL fixes committed and tested
- [ ] All 7 HIGH fixes committed and tested
- [ ] JS runtime tests integrated into CI
- [ ] Regression test added for computed-referencing-prop pattern
- [ ] learn-omniview full-pipeline test passes with corrected TodoPage.omni
- [ ] TypeScript declarations either removed or gated behind CLI flag

---

*Generated by 11-specialist parallel architecture review using the AI Software Architect framework v1.6.0*
