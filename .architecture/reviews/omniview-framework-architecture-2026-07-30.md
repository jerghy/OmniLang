# Architecture Review: OmniView Frontend Framework

**Date**: 2026-07-30
**Review Type**: Component — Frontend Framework Architecture
**Reviewers**: 11 architecture specialists (Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert, TypeScript & React Specialist, Compiler & Language Architect)

---

## Executive Summary

OmniView is a **Language-First frontend framework** with a compiler-driven architecture — `.omni` DSL source files compile through a 6-stage Rust pipeline (Lexer → Parser → Semantic → IR → Codegen → Output) to generate HTML/JS/CSS. The architecture's defining strength is its elevation of UI concepts (`component`, `signal`, `store`, `design`, `@motion`) into language keywords rather than library abstractions — a design choice validated by industry trends toward signal-based reactivity (Svelte 5 runes, SolidJS, Preact signals).

The codebase exhibits strong engineering discipline: 853+ tests, zero clippy warnings, zero panics in production paths, and 46 well-structured ADRs. However, the review surface 50+ findings across 11 perspectives reveals **four cross-cutting structural concerns** that span 3+ specialist domains each:

1. **OmniViewGenerator accretion** (5/11 reviewers): The 40-field generator struct has caused documented bugs and continues growing with each feature.
2. **Dead code & unused IR fields** (4/11): Six non-default backends (~9,700 lines), dead TypeScript module (343 lines), and 3 unused IR fields create maintenance drag.
3. **Signal batching gap** (3/11): Synchronous signal propagation without batching will become a performance ceiling as component complexity grows.
4. **Runtime integration testing blind spot** (3/11): The primary OmniView target lacks CI-level runtime verification, while recent CRITICAL bugs were all found via manual browser testing.

**Overall Assessment**: **Adequate** — The architecture is sound in its fundamentals, but structural debt (generator accretion, dead code) and verification gaps (runtime CI, signal batching) are accumulating faster than they are being resolved.

**Top priorities**:
1. Extract `NameResolver` from OmniViewGenerator (Phase 1 decomposition) — addresses 5/11 reviews
2. Populate diagnostic `suggestion` fields and return all parse errors — unblocks AI-assisted workflows
3. Add OmniView runtime smoke tests to CI — closes the primary verification blind spot
4. Remove dead IR fields (`is_copy`, `owner`, `StoreAction.event_data`) — reduces cognitive overhead

---

## System Overview

OmniView is the primary codegen target of the OmniLang compiler. It compiles `.omni` source files to self-contained HTML/JS/CSS bundles using a compiler-driven, runtime-light strategy similar to Svelte.

| Dimension | Detail |
|-----------|--------|
| **Language** | `.omni` DSL with 18 first-class keywords |
| **Compiler** | Rust, ~16,000 lines (core + OmniView codegen) |
| **Pipeline** | Lexer → Parser → Semantic Analysis → IR Lowering → Codegen → Output |
| **Runtime** | ~200 lines JS signals + ~30KB conditional animation library |
| **Active targets** | OmniView (debug), React, Flutter |
| **Paused targets** | Vue, Dioxus, JavaScript, Python, Rust, Dart (feature-flagged) |
| **Test suite** | 853+ Rust tests, cross-target golden compilation, manual Browser MCP |
| **ADR count** | 46 (ADR-001 through ADR-046) |
| **Codebase** | `omnilang/src/codegen/omniview/` — 18 files, ~10,700 lines |

**Key architectural decisions**:
- **Compile-time component inlining**: Child components expand into parent HTML; no runtime component tree.
- **Deferred JS swap**: Events/effects buffered during HTML generation, executed after DOM insertion.
- **Design system as language feature**: `design` blocks → CSS variables + semantic classes + multi-dimensional variants.
- **14 documented debug-runtime tradeoffs** (ADR-028): synchronous signals, string-based codegen, no batching, `innerHTML` injection.

---

## Cross-Cutting Themes

### Theme 1: OmniViewGenerator Structural Accretion (5/11 reviewers)

**Reviewers**: Systems Architect, Maintainability Expert, Rust Expert, Implementation Strategist, Pragmatic Enforcer

The `OmniViewGenerator` struct has grown to **40 global fields + 9-field ComponentScope**, distributed across 18 `impl` blocks in 27 source files. This is not a theoretical concern — it has caused **two documented bugs** (ADR-025 D1, ADR-026 D3) where fields were not saved/restored across component inlining boundaries.

**Contributing factors**:
- Each new feature adds 1-3 fields (Element Refs added `ref_nid_map`/`ref_nid_set`, Debug mode added `debug_mode`, Scroll triggers added `scroll_handler_emitted`)
- 6 mutable string buffers managed via `std::mem::swap` with no type-level enforcement of phase ordering
- `generate_component()` at 421 lines is the single largest method
- `expr_to_js()` at 278 lines duplicates `FormatExpr` trait logic used by 8 other backends

**Consensus recommendation**: Begin phased extraction:
1. Phase 1: Extract `NameResolver` (9 fields: signal_names, name_renames, global_renames, component_map, store_signal_names, store_field_names, store_names, motion_names, for_state_names) — S effort, purely mechanical
2. Phase 2: Formalize 6-buffer phase state machine with `OutputPhase` enum and guarded write methods — M effort
3. Phase 3: Extract motion/animation, design/CSS, and editor bridge as separate concerns — M effort across phases

### Theme 2: Dead Code & Unused IR Fields (4/11 reviewers)

**Reviewers**: Compiler Architect, Pragmatic Enforcer, Maintainability Expert, Domain Expert

The codebase carries several categories of unused code that create maintenance drag:

| Dead component | Lines | Status | Recommendation |
|---------------|-------|--------|----------------|
| 6 non-default backends | ~9,700 | Feature-flagged, CI tested | Remove from CI or archive to separate repo |
| `typescript.rs` | 343 | ADR-031 disabled, compiled every build | Gate behind `#[cfg(feature = "codegen-dts")]` |
| `SignalIntent.is_copy` | 21 occurrences | Always `false`, never read | Remove from IR |
| `SignalRefIR.owner` | 14 occurrences | Always `None`, never read | Remove from IR |
| `StoreAction.event_data` | 1 field | Always `None` at lowering, never read | Remove from variant |
| `ComponentOutput.html`/`.css` | 2 fields | Always empty strings | Remove — add back when needed |
| `MotionBodyIR` enum | 1 variant | Only `Keyframe` exists, ADR-032 closed other paths | Inline to `KeyframeIR` directly |

**Consensus recommendation**: The IR field removals are trivially reversible (git history preserves them) and immediately reduce cognitive load. The non-default backend question has two defensible positions: remove from CI (Pragmatic Enforcer, Implementation Strategist) vs. keep as IR stability constraint (Systems Architect). The middle ground — remove from CI, keep source in tree — satisfies both concerns.

### Theme 3: Signal Batching Gap (3/11 reviewers)

**Reviewers**: Performance Specialist, TypeScript & React Specialist, Implementation Strategist

The signal runtime propagates changes **synchronously** — each `set()` immediately notifies all subscribers, which perform DOM writes. Multiple signal sets in one synchronous block cause cascading, unbatched DOM updates. This is acknowledged in ADR-037 (deferred with Pragmatic Score 3/10).

**Current impact**: Manageable for the learn-omniview demo (~322 effects). **Projected impact**: At ~500+ effects with frequent updates (sliders, real-time data), layout thrashing will exceed the 16ms frame budget.

**Consensus recommendation**: Implement synchronous batching with a `__omni_batchDepth` counter (SolidJS pattern). When `batchDepth > 0`, defer subscriber notification; flush when returning to 0. ~30 lines in `runtime.rs`. Effort: S.

### Theme 4: Runtime Integration Testing Blind Spot (3/11 reviewers)

**Reviewers**: Implementation Strategist, TypeScript & React Specialist, AI Engineer

The OmniView runtime (primary development target) has **no CI-level runtime verification**. Recent CRITICAL/HIGH bugs were all found via manual Browser MCP testing:
- Prop signal per-instance sharing (CRITICAL, commit 88520e5e)
- Ref wrapper value/textContent getters (HIGH, commit 54f83dff)
- Auto-bind guard for @onInput (HIGH, commit 54f83dff)
- Drag motion mouse position (HIGH, commit 2b7b3cc0)

**Consensus recommendation**: Add a lightweight Node.js-based smoke test (execute generated JS via `vm` module + `jsdom`, verify no runtime errors and expected DOM structure). Effort: M (1-2 days). This does not require a browser and can run in CI.

---

## Trade-offs and Disagreements

### Trade-off 1: Non-default backends — keep or archive?

| Position | Proponents | Argument |
|----------|-----------|----------|
| **Archive/remove from CI** | Pragmatic Enforcer, Implementation Strategist | 9,700 lines of untested code with no consumers; CI testing them is maintenance tax |
| **Keep source, remove CI** | Systems Architect, Compiler Architect | Source serves as IR stability constraint; removing CI reduces cost without losing reference code |
| **Keep as-is** | (none) | No reviewer advocated for current state |

**Resolution**: Middle ground — **remove non-default backends from CI, retain source code in tree**. This reduces immediate maintenance cost while preserving reference implementations for future consumers.

### Trade-off 2: OmniView `expr_to_js` vs. shared `FormatExpr`

| Position | Proponents | Argument |
|----------|-----------|----------|
| **Unify** | Systems Architect | Duplicate `ExprIR` match dispatch is doubled maintenance for primary backend |
| **Don't unify** | Compiler Architect (implicit) | OmniView's signal/scope resolution is genuinely unique; forcing it into FormatExpr would over-parameterize the trait |

**Resolution**: **Structural alignment, not full unification**. Audit `expr_to_js()` to identify match arms producing identical output to `FormatExpr::JsFormatExpr` (literals, binary ops, method calls, etc.) and delegate those to the shared trait. Keep OmniView-specific arms (signal resolution, @for templates, editor mode) in `expr_to_js`. This eliminates the purely mechanical duplication without forcing OmniView concepts into the generic trait.

### Trade-off 3: Component `action` keyword — preserve or remove?

| Position | Proponents | Argument |
|----------|-----------|----------|
| **Preserve in IR** | Domain Expert | Future backends may need to distinguish actions from functions (e.g., React `useCallback` strategies) |
| **Remove from DSL** | Domain Expert (alternative) | If no semantic difference exists, the keyword is misleading |

**Resolution**: **Preserve `action` in parser/AST; add `ActionIR` to IR only when a backend needs it**. The DSL keyword costs nothing to keep, and the distinction may become valuable for React/Flutter codegen. Do not pre-emptively add IR types.

---

## Prioritized Recommendations

### Critical (0-2 weeks)

| # | Recommendation | Effort | Reviews Addressed |
|---|---------------|--------|-------------------|
| C1 | **Extract `NameResolver` from OmniViewGenerator** (Phase 1 decomposition): 9 ident/rename fields into dedicated struct with save/restore | S (2-4h) | Systems, Maintainability, Rust, Implementation, Pragmatic |
| C2 | **Return all parse errors** instead of truncating to first: change `parse()` return type to `Result<Program, Vec<ParseError>>` | S (half-day) | AI Engineer |
| C3 | **Populate `suggestion` field** on type mismatch, undefined reference, and missing return diagnostics | M (2-3 days) | AI Engineer |
| C4 | **Remove dead IR fields**: `is_copy`, `owner`, `StoreAction.event_data`, `ComponentOutput.html`/`.css` | S (1h) | Compiler Architect, Pragmatic Enforcer, Domain Expert |
| C5 | **Fix `component_html.rs:126` unwrap**: iterate `.iter()` instead of `.keys()` + `.get()` | S (10min) | Rust Expert |

### High (2-8 weeks)

| # | Recommendation | Effort | Reviews Addressed |
|---|---------------|--------|-------------------|
| H1 | **Add OmniView runtime smoke tests to CI**: Node.js + jsdom execution of generated output | M (1-2 days) | Implementation, TypeScript/React, AI Engineer |
| H2 | **Implement signal batching**: `__omni_batchDepth` counter with deferred subscriber flush | S (~30 lines) | Performance, TypeScript/React |
| H3 | **Fix React motion effects for conditional rendering**: replace `[]` deps with MutationObserver or render-count signal | M (3-5 days) | TypeScript/React |
| H4 | **Gate `typescript.rs` behind feature flag**: `#[cfg(feature = "codegen-dts")]` | S (5min) | Maintainability, Pragmatic Enforcer |
| H5 | **Remove non-default backends from CI**: keep source, stop testing in `all-backends` | S (15min) | Pragmatic Enforcer, Implementation |
| H6 | **Fix @for list fragment batching**: accumulate clones into `DocumentFragment` before single `insertBefore` | S (one line) | Performance |
| H7 | **Add content validation before `(0, eval)` in editor bridge**: reject non-function strings | S (2 lines) | Security |
| H8 | **Add restrictive CSP to editor mode**: at minimum `connect-src 'self'; frame-src 'none'` | S (5 lines) | Security |

### Medium (2-6 months)

| # | Recommendation | Effort | Reviews Addressed |
|---|---------------|--------|-------------------|
| M1 | **Split `generate_component` (421 lines) into 8-10 named methods** | M (3-5 days) | Maintainability |
| M2 | **Split `emit_list_dom` (390 lines) into 4-5 methods** | M (1-2 days) | Maintainability |
| M3 | **Box `ViewTree::Element(ElementIR)`**: reduce enum from ~376 to ~32 bytes | S (30min) | Rust Expert |
| M4 | **Extract shared `walk_expr`/`walk_stmt`/`walk_view` utilities**: eliminate 3 duplicate recursive walkers | M (2-4h) | Rust Expert |
| M5 | **Build reverse source map index**: `(generated_file, line, col) → omni_span` for runtime error tracing | S (half-day) | AI Engineer |
| M6 | **Build motion name → MotionIntent index on IRProgram**: O(n*m) → O(1) lookup | S (1-2h) | Rust Expert |
| M7 | **Add `debug_assert!` to `format_block_expr` default**: catch un-overridden backends | S (1 line) | Compiler Architect |
| M8 | **Fix @for key fallback for objects**: generate `{item?.id ?? _i}` instead of `{item}` | S (one line) | TypeScript/React |
| M9 | **Wrap store hooks return in `useMemo`**: prevent unnecessary re-renders | S (5 lines) | TypeScript/React |
| M10 | **Fix `push_str(&format!(...))` occurrences** (17 locations): use `write!()` instead | S (30min) | Rust Expert |
| M11 | **Move design.rs test block to separate file**: 279 lines of `#[cfg(test)]` at 50% of file | S (1h) | Maintainability |
| M12 | **Extract raw JS bridge code into `include_str!()` files**: editor.rs, debug.rs | S (1h) | Maintainability |
| M13 | **Convert `TypeKind::from(&str)` to avoid `to_lowercase()` allocation** | S (15min) | Rust Expert |

### Nice-to-Have (2-6 months)

| # | Recommendation | Effort | Reviews Addressed |
|---|---------------|--------|-------------------|
| N1 | **Formalize 6-buffer phase machine**: `OutputPhase` enum + guarded write methods | M (3-5 days) | Systems Architect |
| N2 | **Move ViewTree transformations to shared infra**: `inject_slot_children`, `collect_for_events` | S (1 day) | Systems Architect |
| N3 | **Re-enable TypeScript declaration generation** as opt-in feature | S (1h) | AI Engineer, TypeScript/React |
| N4 | **Refactor React motion to ref-based approach**: use `useRef` instead of `querySelectorAll` | L (1-2 weeks) | TypeScript/React |
| N5 | **Add view-aware error recovery**: track nesting depth for view tree parsing | M (2-3 days) | AI Engineer |
| N6 | **Rename `design::VariantIR` to `StyleVariantIR`**: disambiguate from `logic::VariantIR` | S (30min) | Domain Expert |
| N7 | **Inline `MotionBodyIR`**: single-variant enum → direct `KeyframeIR` usage | S (30min) | Pragmatic Enforcer, Compiler Architect |
| N8 | **Remove Flutter from default Cargo features**: no production consumer exists | S (1 line) | Pragmatic Enforcer |
| N9 | **Add `__mergeTouches` usage check**: mirror `project_uses_motion()` pattern | S (~15 lines) | Pragmatic Enforcer |

---

## Risks If Unaddressed

| Risk | Trigger | Impact | Timeframe |
|------|---------|--------|-----------|
| **OmniViewGenerator exceeds maintainability threshold** | 2-3 more features add 6-9 fields without decomposition | Non-linear complexity increase; new features become disproportionately harder | 2-4 weeks |
| **Runtime bugs ship undetected** | Signal wiring, ref binding, or event handling regression not caught by unit tests | Production-like breakage in primary development target; developer time wasted on manual debugging | Immediate |
| **Signal cascade causes visible jank** | App grows to 500+ effects with frequent updates | 60fps budget exceeded; layout thrashing in animation frames | 2-4 weeks |
| **AI-assisted tooling blocked on single-error iteration** | AI generates multi-error .omni source | N compile cycles for N errors; poor developer experience for AI workflows | When AI tooling is built |
| **Dead code accumulation normalizes waste** | Each new feature adds a deferred item without removing old ones | Codebase signal-to-noise ratio degrades; new contributors spend time understanding unused code | 1-3 months |
| **`format_block_expr` default silently drops statements** | Lowering emits BlockExpr with side-effect statements | Generated code silently incorrect; no compiler warning | Unknown |

---

## Follow-Up Plan

1. **ADR-047**: Create ADR for OmniViewGenerator decomposition plan (Phase 1: NameResolver extraction)
2. **ADR-048**: Create ADR for signal batching implementation (activate ADR-037's deferred plan)
3. **ADR-049**: Create ADR for non-default backend CI policy (remove from CI, retain source)
4. **Specialist deep-dives**: Use `specialist-review` for:
   - Performance: `@for` list rendering optimization (fragment batching + key default)
   - Security: Editor bridge eval hardening
   - AI: Diagnostic suggestion field population
5. **Deferral cleanup**: Review all 33 deferred items; close items with expired trigger conditions; schedule active items

---

## Appendix: Individual Member Reviews

Full verbatim reviews are preserved in the agent output files at:
`C:\Users\XHT\AppData\Local\Temp\claude\D--space-OmniLang\63447a8e-f8b7-43de-ae79-cdd7657fe022\tasks\`

### Summary of Findings by Reviewer

| Reviewer | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Systems Architect | 0 | 1 | 2 | 1 |
| Domain Expert | 0 | 0 | 2 | 3 |
| Security Specialist | 0 | 0 | 3 | 3 |
| Maintainability Expert | 0 | 0 | 4 | 3 |
| Performance Specialist | 0 | 1 | 3 | 2 |
| Implementation Strategist | 0 | 1 | 2 | 1 |
| AI Engineer | 1 | 2 | 3 | 1 |
| Pragmatic Enforcer | 0 | 2 | 3 | 2 |
| Rust Expert | 0 | 0 | 3 | 4 |
| TypeScript & React Specialist | 0 | 2 | 4 | 3 |
| Compiler & Language Architect | 0 | 0 | 4 | 2 |
| **Total** | **1** | **9** | **33** | **25** |

**Total distinct findings**: 68 (after deduplication of overlapping observations across reviewers)

---

*Generated by 11-member architecture team review. Pragmatic mode: enabled (balanced intensity).*
