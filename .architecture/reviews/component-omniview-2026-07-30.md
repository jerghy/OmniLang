# Architectural Review: OmniView Codegen System

**Date**: 2026-07-30
**Review Type**: Full multi-perspective architecture review (11 specialists + Pragmatic Enforcer)
**Scope**: OmniLang compiler's multi-target UI codegen system — pipeline, IR, all codegen backends
**Methodology**: Independent parallel specialist reviews → cross-cut analysis → consolidated report

---

## Executive Summary

OmniView's architecture earns an overall assessment of **Strong (B+/A-)** . The 6-stage compiler pipeline (Lexer→Parser→Semantic→IR Lowering→Codegen→Output) is well-designed with clean phase separation enforced by Rust's type system. The codebase demonstrates exceptional documentation discipline (36 ADRs), rigorous testing (837+ Rust tests, 193+ JS tests), and a mature multi-agent review methodology that has measurably improved code quality over 5+ review cycles.

The architecture's primary strength is its **layered trait architecture** (Codegen → FormatExpr/FormatStmt/FormatType → UiCodegen) that eliminated ~5,000 lines of cross-backend duplication. The ComponentScope pattern (ADR-034) is a textbook application of Rust's ownership model to enforce architectural invariants, eliminating a proven bug class.

However, the review reveals **3 systemic tensions** that span multiple specialist perspectives:

1. **Adoption gap**: The FormatExpr trait architecture is sound, but the two largest backends (OmniView at ~10,700 lines, React at ~7,600 lines) only partially adopt it. OmniView maintains a parallel 417-line `expr_to_js` formatter that duplicates shared logic. This was a known deferred item (ADR-013) that 4 of 11 specialists independently flagged as the highest-leverage structural issue.

2. **Security asymmetry**: The OmniView debug runtime has a thoughtful 3-layer defense-in-depth architecture (compile-time checks + runtime sanitizers + CSP). The React production backend — the one that ships to end users — has zero equivalent sanitization. URL attributes, CSS values, and signal-driven expressions pass through unsanitized in React codegen.

3. **Dual dispatch paths**: OmniView codegen has two entry points — the standard `Codegen` trait adapter (unused in production) and direct `omniview::generate()` calls (used by all 7+ production call sites). This creates confusion and means stdlib import validation (which runs for all other backends) is skipped for the most-used target.

**Top 3 priorities for immediate action:**

| # | Priority | Finding | Specialists |
|---|----------|---------|-------------|
| 1 | **CRITICAL** | React codegen: Missing URL/CSS sanitization for production output | Security |
| 2 | **HIGH** | Inline style `$signal` skips ref-name → `Ref.current` translation | TS/React |
| 3 | **HIGH** | Async `@unmount` lifecycle: missing `body_has_await` check | TS/React |

---

## Review Team

| # | Specialist | Severity Distribution | Key Focus |
|---|-----------|----------------------|-----------|
| 1 | Systems Architect | 3 HIGH, 1 MEDIUM, 1 LOW | System coherence, dual dispatch paths, OmniViewGenerator coupling |
| 2 | Domain Expert | 2 HIGH, 3 MEDIUM, 2 LOW | IR domain model fidelity, stringly-typed fields, signal modeling |
| 3 | Security Specialist | 3 MEDIUM, 3 LOW | React sanitization gap, CSS CR/LF bypass, debug bridge exposure |
| 4 | Maintainability Expert | 2 HIGH, 5 MEDIUM, 2 LOW | Codegen trait adoption, file sizes, struct field counts |
| 5 | Performance Specialist | 3 MEDIUM, 2 LOW | Double-deep-clone, IIFE bloat, redundant view tree traversals |
| 6 | Implementation Strategist | 2 HIGH, 5 MEDIUM | IR exhaustiveness, CI burden, deferral tracking, Flutter P0 status |
| 7 | AI Engineer | 2 MEDIUM, 3 LOW | Agent reliability tracking, eval gaps, P34 pre-design |
| 8 | Pragmatic Enforcer | 1 HIGH, 3 MEDIUM, 4 LOW | YAGNI: UiCodegen trait, typescript.rs, #[non_exhaustive] misuse |
| 9 | Rust Expert | 3 MEDIUM, 5 LOW | assert! vs debug_assert!, stringly-typed warnings, file sizes |
| 10 | TS/React Specialist | 3 HIGH, 6 MEDIUM, 2 LOW | Ref-name threading, React isolation, motion codegen quality |
| 11 | Compiler Architect | 2 MEDIUM, 3 LOW | Stdlib validation gap, FormatExpr adoption, target_spans |

---

## Cross-Cutting Themes

### Theme 1: FormatExpr Adoption Gap (4 specialists)

**Flagged by**: Systems Architect (HIGH), Maintainability Expert (HIGH), Compiler Architect (MEDIUM), Rust Expert (LOW)

OmniView's `expr_to_js()` (417 lines in `omnilang/src/codegen/omniview/expr.rs`) independently implements expression formatting that duplicates the shared `FormatExpr` trait. Flutter proves full migration is possible: its expression formatter is 15 lines delegating to `DartFormatExpr`. The OmniView formatter has unique semantics (signal `.value` appending, `@for` state routing, store rename lookups) that need to be extracted as trait override methods rather than replacing the entire dispatch.

**Impact**: Each new `ExprIR` variant requires manual handling in both the shared trait AND OmniView's copy. Bug fixes to one don't automatically fix the other. This is the highest-leverage structural refactoring available.

**Recommendation**: Short-term — create `OmniViewFormatExpr` implementing the `FormatExpr` trait, with OmniView-specific logic in `format_identifier`/`format_call` overrides. Effort: M (2-3 days). Requires snapshot-identical output.

### Theme 2: React Security Sanitization Gap (2 specialists, high-impact)

**Flagged by**: Security Specialist (MEDIUM C1, C2), TS/React Specialist (related findings)

The OmniView debug runtime has compile-time `is_dangerous_url()` checks, runtime `__omni_sanitizeUrl`/`__omni_sanitizeCss` sanitizers, and CSP meta tags. The React production backend — which generates code that ships to end users — has **none** of these. Static URL attributes (`href`, `src`, `srcset`) with `javascript:` schemes pass through unsanitized. CSS inline style values are emitted verbatim without `sanitize_css_value()` calls. This is a supply-chain vulnerability: a malicious third-party `.omni` component's dangerous URLs would pass through React codegen unchecked.

**Recommendation**: Immediate — port `is_dangerous_url()` check to React codegen for static URL attributes, and add `sanitize_css_value()` to inline style formatting. Both are one-function-import fixes. Effort: S (collectively <1 hour).

### Theme 3: Dual Dispatch Paths (3 specialists)

**Flagged by**: Systems Architect (HIGH), Maintainability Expert (HIGH), Compiler Architect (MEDIUM)

`OmniViewCodegen::generate()` implements the standard `Codegen` trait, but all 7+ production call sites in `lib.rs` and `omnilang-wasm` call `codegen::omniview::generate()` directly. This means:
- The Codegen trait adapter compiles but never executes in production
- Stdlib import validation (`validate_stdlib_imports`) runs for all other backends but is skipped for OmniView
- A developer adding a trait method override to `OmniViewCodegen` would see it compile but never run
- `compile()` with `Target::OmniView` passes hardcoded `editor_mode=false, debug_mode=false`

**Recommendation**: Short-term — unify on a single path. Either thread `editor_mode`/`debug_mode`/`css_only` through the Codegen trait, or remove the adapter and document that OmniView intentionally bypasses the trait. Effort: S-M.

### Theme 4: Deferred Items Accumulating Without Central Tracking (3 specialists)

**Flagged by**: Implementation Strategist (MEDIUM), AI Engineer (MEDIUM), Compiler Architect (noted)

The `deferrals.md` file states "no active deferrals" but ~10 active Phase 2 items are scattered across ADR-036, ADR-038, ADR-021, ADR-037, and session memory files. Examples: `@ref` inside `@for` (ADR-038 D2), Flutter GlobalKey codegen (ADR-036), ref_names in store codegen (ADR-036), signal batching (ADR-037 profiling gate), TypeKind recursive validation (ADR-001 Sprint 4), source map pipeline (ADR-001 Sprint 2-3), Codegen trait adoption (ADR-013), IR `#[non_exhaustive]` migration (ADR-001).

**Recommendation**: Immediate — update `deferrals.md` with all active items, their triggers, and consequences of never addressing. Effort: S (30 minutes).

### Theme 5: Motion Codegen Isolation (2 specialists)

**Flagged by**: TS/React Specialist (HIGH C3), Performance Specialist (MEDIUM C5)

All React motion effects use `document.querySelectorAll('[data-motion-*]')` — global DOM queries unscoped to the component subtree. Two components with `@enter` animations double-activate each other's elements. Additionally, hover/press effects are emitted unconditionally even when the component only uses `@enter`, generating ~50 lines of dead JS per component.

**Recommendation**: Short-term — scope queries to component root via `useRef` + `querySelector` on root element. Medium-term — split motion effect emission by trigger type, only emitting what's actually used. Effort: M-L.

---

## Trade-offs and Disagreements

### Disagreement 1: #[non_exhaustive] on IR Enums

- **Implementation Strategist**: ADD `#[non_exhaustive]` to `ExprIR`, `StatementIR`, `ViewTree`, `MutationIntent` — to prevent cascading compile errors across 10 backends when variants are added
- **Pragmatic Enforcer**: REMOVE `#[non_exhaustive]` from `TypeKind`, `EventDataKind` — because they have zero external consumers and the attribute reduces Rust's exhaustiveness checking
- **Domain Expert**: The `ViewTree` `#[serde(untagged)]` is a related concern — variant ordering determines deserialization, creating fragility at the WASM/FFI boundary

**Resolution**: Both positions are correct for different enums. The IS recommendation applies to IR enums consumed by 10 backends (blast radius protection). The PE recommendation applies to internal enums with zero external consumers (restore compiler exhaustiveness). They are complementary, not contradictory. Apply both.

### Disagreement 2: Paused Backends Strategy

- **Implementation Strategist**: Reduce CI burden by moving to compile-check-only in main CI
- **Pragmatic Enforcer**: Consider removing entirely, restore from git history if needed
- **Compiler Architect**: Current feature-flag approach is "pragmatic" — backends compile but don't slow the default CI path

**Resolution**: The current approach is acceptable. The `test-all-backends` CI job is non-blocking (not a required check for PRs). The real cost is IR-evolution friction — each IR change risks breaking 6 dormant backends. No action required now, but if IR changes become frequent, extract paused backends to a separate version-pinned crate.

---

## Prioritized Recommendations

### Immediate (this session — effort: hours)

| # | Priority | Action | Effort | Specialists |
|---|----------|--------|--------|-------------|
| R1 | **CRITICAL** | Add `is_dangerous_url()` compile-time check to React codegen for static URL attributes | S (15 min) | Security |
| R2 | **CRITICAL** | Add `sanitize_css_value()` to React codegen's `format_style_value_expr` for static strings | S (10 min) | Security |
| R3 | **CRITICAL** | Add CR/LF/TAB normalization to `__omni_sanitizeCss` (one-line fix deferred across 2 ADRs) | XS (1 line) | Security |
| R4 | **HIGH** | Add `ref_names` parameter to `format_style_value_expr` — fix `$signal` → `Ref.current` translation | S (15 min) | TS/React |
| R5 | **HIGH** | Add `body_has_await` check to `@unmount` lifecycle codegen path | S (10 min) | TS/React |
| R6 | **HIGH** | Add factory code validation in editor bridge `__omni_replaceComponent`: `typeof e.data.js === 'string'` guard | XS (1 line) | Security |
| R7 | MEDIUM | Update `deferrals.md` with all ~10 active deferred items, triggers, and consequences | S (30 min) | Impl Strategist, AI Engineer |
| R8 | MEDIUM | Fix `any(|_| true)` in `view_has_motion_or_effects` — filter out `@timeline`/`@animate` triggers | S (1 line) | Performance |
| R9 | MEDIUM | Fix `assert!` vs `debug_assert!` doc mismatch in `push_ref_nid` | S (1 line) | Rust Expert |
| R10 | LOW | Fix `unnecessary_unwrap` in `validate_refs.rs:159` (use `if let`) | S (2 min) | Rust Expert |
| R11 | LOW | Use destructured `event_data` instead of `handler.event_data()` in flutter/view.rs:237 | S (2 min) | Rust Expert |
| R12 | LOW | Add `#[ts(skip)]` to affected fields to fix ts-rs parse warnings | S (15 min) | Rust Expert |
| R13 | LOW | Add `console.warn` to `wrap_debug_html` output about missing CSP | XS (1 line) | Security |
| R14 | LOW | Remove `UiCodegen` trait — zero consumers, 103 lines dead | S (30 min) | Pragmatic Enforcer |
| R15 | LOW | Gate `typescript.rs` (351 lines) behind `#[cfg(test)]` | S (10 min) | Pragmatic Enforcer |
| R16 | LOW | Derive `Default` for `OmniViewGenerator` and `ComponentScope`; replace `test_new()` | S (30 min) | Pragmatic Enforcer |

### Short-term (this sprint — effort: days)

| # | Priority | Action | Effort | Specialists |
|---|----------|--------|--------|-------------|
| R17 | HIGH | Resolve OmniView dual dispatch: unify on single codegen entry path | S-M (half day) | Systems, Maintainability |
| R18 | HIGH | Add `#[non_exhaustive]` to `ExprIR`, `StatementIR`, `ViewTree`, `MutationIntent` — or implement the ADR-001 planned migration | S (<1 hour) | Impl Strategist |
| R19 | HIGH | Remove `#[non_exhaustive]` from `TypeKind`, `EventDataKind` (internal enums, zero external consumers) | S (10 min) | Pragmatic Enforcer |
| R20 | HIGH | Scope React motion `document.querySelectorAll` to component root element | M (1 day) | TS/React |
| R21 | MEDIUM | Create `OmniViewFormatExpr` implementing `FormatExpr` trait — migrate `expr_to_js` to delegate to shared trait | M (2-3 days) | Systems, Maintainability, Compiler |
| R22 | MEDIUM | Thread `ref_names` through React store codegen paths (16 call sites) | M (2-4 hours) | Impl Strategist, TS/React |
| R23 | MEDIUM | Convert `warnings: Vec<String>` to `Vec<Diagnostic>` with spans for IDE integration | M (2-3 hours) | Rust Expert |
| R24 | MEDIUM | Split motion trigger emission by type — only emit hover/press effects when actually used | M (4 hours) | Performance, TS/React |
| R25 | MEDIUM | Add stdlib import validation to OmniView path (match React/Flutter behavior) | S (15 lines) | Compiler Architect |
| R26 | MEDIUM | Add structured evidence requirements to agent dispatch prompts | S (30 min) | AI Engineer |
| R27 | MEDIUM | Create `review-accuracy.md` with aggregated false-positive trend tracking | S (30 min) | AI Engineer |
| R28 | MEDIUM | Replace `StyleRuleIR.output_mode: String` with a `StyleOutputMode` enum | S (1 hour) | Domain Expert |
| R29 | MEDIUM | Replace `EventHandlerIR.used_fields: Option<Vec<String>>` with `UsedEventFields` enum | S (30 min) | Domain Expert |
| R30 | MEDIUM | Eliminate double-deep-clone in `inject_slot_children` (single-pass borrowed approach) | M (4-6 hours) | Performance |
| R31 | MEDIUM | Reduce ViewTree traversals in React codegen from 12 to ~4 via combined analysis pass | M-L (1-2 days) | Performance |
| R32 | MEDIUM | Split React `view.rs` (948 lines) into sub-modules following OmniView `view/` pattern | M (1-2 days) | Maintainability |
| R33 | LOW | Extract `generate_component` into named phase methods | S (half day) | Maintainability |
| R34 | LOW | Add eval-harness smoke test to CI to detect broken evals | S (30 min) | AI Engineer |
| R35 | LOW | Create P34 pre-design document capturing known constraints | S (1-2 hours) | AI Engineer |
| R36 | LOW | Consolidate motion trigger IIFEs into data-attribute-driven registration loops | M (1 day) | Performance |
| R37 | LOW | Merge `motion.rs` (25 lines) into `motion_config.rs` | S (15 min) | Pragmatic Enforcer |
| R38 | LOW | Merge `editor.rs` and `debug.rs` into `bridge.rs` | S (30 min) | Pragmatic Enforcer |
| R39 | LOW | Use `_i` index as fallback key in `@for` list rendering (instead of object identity) | S (1 line) | TS/React |

### Long-term (next sprint or deferred)

| # | Priority | Action | Effort | Specialists |
|---|----------|--------|--------|-------------|
| R40 | MEDIUM | Add `ExprIR::SignalRef(SignalRefIR)` variant for structural signal identification in IR | M | Domain Expert |
| R41 | MEDIUM | Add generated React .tsx type-checking to eval suite (tsc pass) | M (2-4 hours) | AI Engineer, TS/React |
| R42 | MEDIUM | Document Flutter feature gap matrix; decide P0 vs P1 status | S-M | Impl Strategist |
| R43 | LOW | Evaluate `ConstructionKind` on `ElementIR` for possible removal | M | Domain Expert |
| R44 | LOW | Decompose OmniViewGenerator into phase-specific structs (JsPhase → HtmlPhase → ...) | L (4-5 days) | Systems, Maintainability |
| R45 | LOW | Implement TypeKind recursive validation for `Named` types | M | Domain Expert, Compiler |
| R46 | LOW | Populate target_spans for React/Flutter backends (minimal source mapping) | L | Compiler Architect |
| R47 | LOW | Add cross-target generated code quality eval for all 3 P0 backends | L (1 week) | AI Engineer |

---

## Risks If Unaddressed

### Risk 1: Supply-chain XSS via React production output (R1, R2, R3)

A malicious third-party `.omni` component with `href="javascript:..."` or `style="background: url(javascript:...)"` passes through React codegen unsanitized. **Likelihood**: Medium. **Impact**: High (XSS in production React apps). The fix is a one-function import.

### Risk 2: Silent runtime crashes from ref + inline style combination (R4)

Any component using both `@ref` and inline styles that reference the ref via `$signal` meta-objects produces code where `box.offsetWidth` resolves to `undefined.offsetWidth` → runtime crash. **Likelihood**: Medium (natural usage pattern). **Impact**: High (crash).

### Risk 3: Corrupted React cleanup from async @unmount (R5)

Writing `await` in `@unmount` generates a broken async function inside a React cleanup that React does not await. Cleanup timing guarantees are violated. **Likelihood**: Low (async in unmount is rare). **Impact**: High (racy teardown bugs).

### Risk 4: IR enum refactoring silently breaks 6 paused backends (R18)

Adding a variant to `ExprIR` without `#[non_exhaustive]` causes compile errors across all 10 backends. The `test-all-backends` CI job catches this but is non-blocking — failures are discovered after merge. **Likelihood**: High (every IR addition). **Impact**: Medium (post-merge fix commits).

### Risk 5: Motion animation collisions in multi-component pages (R20)

`document.querySelectorAll('[data-motion-*]')` scans the entire document. Two components with `@enter` double-activate each other's elements. **Likelihood**: Medium (any multi-component page). **Impact**: Medium (animation glitches, not crashes).

### Risk 6: Deferral amnesia (R7)

With ~10 active deferred items tracked only in ADR text (not in `deferrals.md`), items will be forgotten or rediscovered during reviews. Each rediscovery costs ~1 specialist-equivalent in review time. **Likelihood**: High (already happened — see ADR-028's 14 tradeoffs that were re-discovered). **Impact**: Low per item, accumulating.

---

## Success Metrics and Follow-Up

### Quality Baseline (pre-review)

| Metric | Value |
|--------|-------|
| Rust tests | 837+ |
| JS/TS tests | 193+ |
| ADRs | 36 (+2 in this review) |
| Clippy warnings | 0 (default), 1 (`-D warnings`) |
| Codegen backends (P0) | 3 active, 6 paused |
| P0 codegen lines | ~22,400 (OmniView 10,700 + React 7,600 + Flutter 4,100) |
| Review cycles completed | 5+ in past month |

### Review Statistics

| Metric | This Review |
|--------|-------------|
| Specialists | 11 + Pragmatic Enforcer |
| Total findings | ~65 |
| CRITICAL | 0 |
| HIGH | 11 (across 4 specialists) |
| MEDIUM | 28 (across all specialists) |
| LOW | 26 (across all specialists) |
| Cross-cutting themes | 5 |
| Trade-off disagreements | 2 |
| Actionable recommendations | 47 |

### False-positive estimate

Based on prior review cycles (Layer 1 → Layer 2 progression from ~33% → ~21% → 12.5%), the expected verification rate for this review is ~85-90% of HIGH/MEDIUM findings being confirmed real. Layer 2 deep verification (adversarial checking of each finding against ADRs and code) is recommended before implementing R17-R33.

### Follow-up plan

1. **This session**: Implement R1-R16 (immediate fixes, ~2 hours total)
2. **This sprint**: Implement R17-R32 (structural improvements, ~3-5 days)
3. **Next sprint**: Evaluate R40-R47 based on deferral triggers and profiling data
4. **ADR creation**: `create-adr` for the FormatExpr migration plan (R21) and React sanitization architecture (R1-R3)
5. **Layer 2 verification**: Run adversarial verification on the 11 HIGH findings before implementing

---

## Individual Specialist Reviews

<details>
<summary>1. Systems Architect — 3 HIGH, 1 MEDIUM, 1 LOW</summary>

### Perspective
Focuses on how components work together as a cohesive system, analyzing big-picture architectural coupling patterns across the codegen layers.

### Key Findings

**HIGH: OmniView expression formatting bypasses shared FormatExpr trait**
- `omnilang/src/codegen/omniview/expr.rs` (417 lines) independently implements expression formatting
- React and Flutter both use the shared `FormatExpr` trait; OmniView has its own parallel implementation
- Each new `ExprIR` variant requires manual handling in 3 places instead of 2
- Fix: Create `OmniViewFormatExpr` implementing the trait, with OmniView-specific overrides

**HIGH: OmniView codegen dual dispatch paths**
- `OmniViewCodegen::generate()` (mod.rs:136-175) implements Codegen trait — but zero production callers
- All 7+ production call sites in `lib.rs` and `omnilang-wasm` call `codegen::omniview::generate()` directly
- Stdlib validation runs for all other backends but is skipped for OmniView
- Fix: Unify on single path — thread `editor_mode`/`debug_mode` through Codegen trait

**HIGH: OmniViewGenerator 38-field stage coupling**
- The generator struct couples JS emission, HTML generation, CSS collection, element trees, and source maps
- `view/mod.rs:49-77` uses fragile `std::mem::swap(&mut self.js, &mut self.deferred_js)` for execution ordering
- Fix: Wrap deferred JS swap in type-safe abstraction with validation assertions

**MEDIUM: css_only mode runs full pipeline**
- `resolve_styles()` creates a full 38-field generator then discards JS output
- For 20+ component editor sessions, this adds measurable latency
- Fix: Extract `generate_css_and_mappings()` for the css_only path

**LOW: Flutter codegen wildcard re-exports**
- `flutter/mod.rs:14-21` uses `pub(crate) use expr::*` — hides actual public API
- Fix: Convert to explicit name lists

</details>

<details>
<summary>2. Domain Expert — 2 HIGH, 3 MEDIUM, 2 LOW</summary>

### Perspective
Evaluates how well the architecture represents the problem domain — whether the domain model (signals, stores, components, views, animations) faithfully captures UI development concepts.

### Key Findings

**HIGH: Action concept lost in IR — merged into generic `fns`**
- `ComponentIR.fns` merges component actions and plain functions into a single `Vec<FunctionIR>`
- At the AST level, `ComponentDef` has both `fns` and `actions` — the distinction is lost during IR lowering
- Backends that need to distinguish actions (Vue methods vs actions, SolidJS signal wrappers) must use heuristics
- Fix: Keep `actions: Vec<FunctionIR>` as separate field, or add `kind` field to `FunctionIR`

**HIGH: `StyleRuleIR.output_mode: String` is stringly-typed with no validation**
- Accepts any string value; currently hardcoded to `"css"` in lowering, `"static"` in test fixtures
- No active codegen backend reads this field in production code
- A typo like `"nativ"` would compile silently and produce wrong output
- Fix: Replace with `StyleOutputMode` enum

**MEDIUM: Signal reference not modeled in Expression IR**
- `ExprIR::IdentifierExpr` covers both plain variables and signal references
- Codegen backends receive separate `signal_names: &FxHashSet<String>` to distinguish them
- When serialized to JSON (WASM/FFI), signal references are indistinguishable from plain identifiers
- Fix: Add `ExprIR::SignalRef(SignalRefIR)` variant for structural identification

**MEDIUM: `EventHandlerIR.used_fields: Option<Vec<String>>` encoding is fragile**
- `None` = "all fields" and `None` = "not yet analyzed" — conflation of different semantics
- The two-pass lowering transitions `None → Some` but assertion only fires in debug builds
- Fix: Replace with `UsedEventFields` enum (`All` | `Some(Vec<String>)`)

**MEDIUM: ViewTree untagged enum serialization fragility**
- `#[serde(untagged)]` means variant ordering determines deserialization
- Reordering variants silently corrupts WASM/FFI JSON bridge data
- Fix: Document ordering dependency with explicit comment

**LOW: Lifecycle/Effect split causes conceptual friction** — `unreachable!()` in the mapping
**LOW: ComponentIR accumulating god-object tendencies** — 15 fields across 6 concerns

</details>

<details>
<summary>3. Security Specialist — 3 MEDIUM, 3 LOW</summary>

### Perspective
Examines the OmniView multi-target codegen system from a security-first perspective — XSS prevention, URL/CSS injection, unsafe DOM API exposure, and supply-chain attack surface.

### Key Findings

**MEDIUM: React codegen — URL attribute sanitization missing**
- React `view.rs:146-153` emits static and dynamic URL attributes without `is_dangerous_url()` check
- OmniView backend has compile-time URL checks; React production backend has none
- `javascript:` URLs in `href`/`src` pass through React codegen unsanitized
- Fix: Port `is_dangerous_url()` from `element.rs:24-35` to React codegen (S)

**MEDIUM: React codegen — CSS inline style value sanitization missing**
- `view.rs:834` emits static string style values verbatim — no `sanitize_css_value()` call
- The OmniView backend wraps both static and dynamic CSS values with sanitizers
- Fix: Call `sanitize_css_value()` in `format_style_value_expr` for static string values (S)

**MEDIUM: CSS sanitizer CR/LF/TAB bypass not fixed (deferred across 2 ADRs)**
- `__omni_sanitizeUrl` strips `[\r\n\t]` but `__omni_sanitizeCss` does not
- CSS spec discards CRLF before tokenization — browser normalizes and processes injected patterns
- Fix: One line — add `replace(/[\r\n\t]/g, '')` to the CSS sanitizer (XS)

**LOW: Debug bridge exposes reactive runtime on `window`**
- `debug.rs:27-30` exposes `__omni_signal`, `__omni_computed`, `__omni_effect`, `__omni_el`
- Any script in the debug window can read/write all signals and access any DOM element

**LOW: `wrap_debug_html` CSP omission creates misuse risk** — no runtime guard preventing production deployment
**LOW: `(0, eval)` in editor bridge — no input validation on factory code** — `typeof e.data.js !== 'string'` guard missing

### Strengths
- Three-layer defense-in-depth (OmniView): compile-time checks + runtime sanitizers + CSP
- `String()` coercion prevents `toString()` bypass in sanitizers
- CSS escape normalization (`\6a` → `j`) before pattern matching
- `postMessage` origin check correctly enforced
- Sanitizer regression tests exist (9 assertions URL, 8 CSS)

</details>

<details>
<summary>4. Maintainability Expert — 2 HIGH, 5 MEDIUM, 2 LOW</summary>

### Perspective
Evaluates code quality, technical debt, refactoring needs, and how well the architecture facilitates long-term maintenance and evolution.

### Key Findings

**HIGH: Dual entry path for OmniView codegen**
- `OmniViewCodegen::generate()` implements Codegen trait but is unused in production
- All 7+ production call sites use direct `codegen::omniview::generate()` calls
- A developer adding a trait method override sees it compile but never execute
- Fix: Unify on single path

**HIGH: OmniView `expr_to_js` independent of FormatExpr trait**
- 417 lines of expression formatting that duplicates the shared trait
- Flutter proves full migration possible (15-line thin wrapper)
- Adding a new `ExprIR` variant requires matching in 3 places instead of 1
- Fix: Create `OmniViewFormatExpr` implementing the trait

**MEDIUM: React `format_expr_ir_tsx` only partially uses FormatExpr**
- 3 variants handled directly; rest delegated to trait — two code paths for expressions
- Fix: Remove direct handlers, delegate entirely to `ReactFormatExpr`

**MEDIUM: React `view.rs` at 948 lines exceeds 800-line guideline**
- Handles attributes, events, motion, CSS, inline styles, ARIA in one file
- OmniView splits this into `view/element.rs`, `view/events.rs`, `view/style.rs`, `view/condition.rs`
- Fix: Extract sub-modules following OmniView pattern

**MEDIUM: OmniViewGenerator struct — 47 fields across 9 concerns**
- 42-line `test_new()` constructor; 300-line `generate_component` method touches 20+ fields
- Fix: Extract `EditorState` and `OutputState` sub-structs

**MEDIUM: `generate_component` is a single ~300-line method**
- 6 distinct phases with ordering dependencies but no structural separation
- Fix: Split into named phase methods

**MEDIUM: `typescript.rs` — 311 lines of dead code since ADR-031 D6**
- Explicit "zero production consumers" comment; still compiled in every build
- Fix: Move behind feature flag or `#[cfg(test)]`

**LOW: IR `types.rs` at 854 lines with 40+ declarations** — central coupling surface
**LOW: Complete FormatExpr migration for OmniView and React** — structural fix for the coupling

</details>

<details>
<summary>5. Performance Specialist — 3 MEDIUM, 2 LOW</summary>

### Perspective
Evaluates performance across two domains: compile-time codegen performance (Rust/WASM) and generated runtime performance (browser JS/React).

### Key Findings

**MEDIUM: Double-deep-clone in `inject_slot_children`**
- `component_html.rs:126-132` — `child_view.clone()` + `inject_slot_children(c.clone(), ...)` = two full deep copies
- For 50-element view trees, 100 allocations instead of 50; compounds with nesting depth
- Fast path at line 126 skips this for slot-free components (common case)
- Fix: Single-pass borrowed approach — only clone at slot nodes (M)

**MEDIUM: Per-element IIFE motion trigger JS bloat**
- Each `@enter`/`@hover`/`@press` trigger generates a 10-15 line IIFE wrapper per element
- 30 elements × 2 triggers = 600 lines of JS setup code
- Fix: Emit data-attribute-driven registration loops (follows React backend pattern) (M)

**MEDIUM: `any(|_| true)` defeats `useEffect` import optimization**
- `effect.rs:307` — checks for motion triggers but `any(|_| true)` always returns true
- Comment says `@timeline`/`@animate` don't need useEffect, but code doesn't filter
- Components using only `@timeline` import `useEffect` unnecessarily
- Fix: Filter trigger kinds — exclude Timeline/Animate (S, 1 line)

**LOW: 12 redundant view tree traversals in React codegen**
- Same structural traversal repeated for signal collection, store refs, style classes, motion refs
- Fix: Combine into single `ViewTreeAnalysis` pass (M-L)

**LOW: `@for` runtime JS generation size**
- Each `@for` generates 80-100 lines of runtime JS (key tracking, nid assignment, event delegation)
- Explicitly documented as intentional debug runtime trade-off (ADR-028)

### Strengths
- `project_uses_motion` guard conditionally embeds 46KB animation runtime
- Shared scroll handler — single `addEventListener('scroll')` instead of per-element
- ComponentScope save/restore is zero-cost (struct move, not clone)
- Fast path for slot-free component inlining (common case)
- `FxHashMap`/`FxHashSet` throughout for compiler-internal hashing

</details>

<details>
<summary>6. Implementation Strategist — 2 HIGH, 5 MEDIUM</summary>

### Perspective
Evaluates HOW and WHEN changes should be implemented — blast radius, reversibility, technical readiness, team capability, and timing.

### Key Findings

**HIGH: IR enums lack `#[non_exhaustive]` protection**
- `ExprIR`, `StatementIR`, `ViewTree`, `MutationIntent` have no protection
- Adding a variant cascades to all 10 backends
- Non-P0 backends produce `panic!` or errors caught only in non-blocking CI
- Fix: Add `#[non_exhaustive]` to core IR enums — ADR-001 planned work, never done (S)

**HIGH: Non-active backends CI burden higher than ADR-003 intended**
- `test-all-backends` runs in CI despite ADR-003 decision to remove it
- ~9,400 lines across 6 dormant targets incur IR-evolution tax
- Fix: Move to compile-check-only in main CI; separate optional nightly job (S)

**MEDIUM: `ref_names` threading incomplete in React store codegen**
- `mod.rs` — 16 call sites pass `&FxHashSet::default()` instead of actual ref_names
- Store action bodies referencing `boxRef` produce `boxRef` instead of `boxRef.current`
- Deferred to Phase 2 (ADR-036) but causes silent runtime errors
- Fix: Thread `ref_names` through store codegen paths (M, ~50 lines)

**MEDIUM: Flutter P0 status inconsistent with feature gaps**
- No `@ref`/`GlobalKey` support; `class_map_bindings` uses placeholder
- Flutter is 39% the size of OmniView in codegen lines (4,128 vs 10,691)
- Fix: Document feature gap matrix; decide P0 vs P1 (S for docs, L for implementation)

**MEDIUM: Deferred Phase 2 items accumulating without centralized tracking**
- `deferrals.md` says "no active deferrals" — actually ~10 items across ADRs
- Fix: Update `deferrals.md` with all items, triggers, consequences (S)

**MEDIUM: `UiCodegen` trait adoption is confusing**
- Defined with 3 implementors but zero polymorphic consumers
- New backend developers unclear whether to implement it
- Fix: Audit and either adopt fully or remove

### Strengths
- Excellent ADR discipline — every significant decision documented with blast radius/reversibility
- ADR-037 (signal batching) is model deferred decision — profiling gate, clear trigger
- P35 crate decomposition successfully validated (772 tests, zero clippy)
- Cross-target consistency baseline (ADR-021) with semantic equivalence principle
- Overall readiness: B+ — "good discipline but reaching limits of active maintenance"

</details>

<details>
<summary>7. AI Engineer — 2 MEDIUM, 3 LOW</summary>

### Perspective
Evaluates architecture from an AI integration perspective — how AI-assisted development patterns have shaped the system, observability of AI-generated code quality, and effectiveness of review methodology.

### Key Findings

**MEDIUM: Subagent reliability data documented but not systematically addressed in tooling**
- 33% false-positive rate on verification, 67% failure on implementation — known but reactive mitigations
- Agent prompts are not hardened against documented failure modes
- Fix: Add structured evidence requirements to agent dispatch prompts (S)

**MEDIUM: No aggregated false-positive trend tracking across reviews**
- 50+ review documents; false-positive rates mentioned ad-hoc
- No single source of truth tracking methodology improvement over time
- Fix: Create `review-accuracy.md` with per-review metrics (S)

**LOW: Eval harness has no generated code quality metrics** — tests compiler correctness, not output quality
**LOW: No automated meta-eval to detect broken evals** — both eval scripts were broken silently before
**LOW: P34 AI code completion has no pre-design** — zero design work, placeholder only

### Strengths
- Meta-awareness of AI agent limitations — actively measures error rates, documents failure modes
- ADR system as persistent context for AI reviewers — elegant solution to context-window limitation
- Pragmatic Enforcer counterbalances AI over-engineering tendency — replicable pattern
- Three-layer review methodology creates AI-quality assurance pipeline
- Measured improvement: false-positive rate 33% → 21% → 12.5% across cycles

</details>

<details>
<summary>8. Pragmatic Enforcer — 1 HIGH, 3 MEDIUM, 4 LOW</summary>

### Perspective
Rigorously questions whether proposed solutions, abstractions, and features are actually needed right now. Applies YAGNI principles with Necessity/Complexity/Ratio scoring.

### Key Findings

**HIGH: UiCodegen trait — zero-consumer abstraction**
- Necessity: 0/10 — zero polymorphic call sites confirmed via codebase-wide grep
- Complexity: ~103 lines across definition + 2 impl blocks
- Fix: Remove trait and impl blocks; concrete methods remain (S)

**MEDIUM: `typescript.rs` — compiled but never invoked**
- Necessity: 1/10 — production call commented out with "zero production consumers"
- Complexity: 351 lines unconditionally compiled in every OmniView build
- Fix: Gate behind `#[cfg(test)]` (S)

**MEDIUM: `#[non_exhaustive]` on internal enums with zero external consumers**
- Necessity: 2/10 — `omnilang-core` is workspace-internal; zero consumers outside workspace
- Actively reduces safety by disabling Rust exhaustiveness checking
- Fix: Remove from `TypeKind` and `EventDataKind` (S)

**MEDIUM: 6 paused backends — 3,900 lines unshipped production code**
- Clean feature-flag gating is good; strategic tradeoff, not strict YAGNI violation
- Fix: Consider extracting to version-pinned crate if IR changes become frequent

**LOW: `motion.rs` (25 lines) unnecessarily split from `motion_config.rs`** — merge
**LOW: `ComponentOutput.html` always empty, reserved for future** — remove field
**LOW: `test_new()` duplicates struct initialization** — derive Default
**LOW: `editor.rs` and `debug.rs` could merge** — single `bridge.rs`

</details>

<details>
<summary>9. Rust Expert — 3 MEDIUM, 5 LOW</summary>

### Perspective
Evaluates memory safety, ownership patterns, type system usage, error handling, unsafe code, and Rust-idiomatic design.

### Key Findings

**MEDIUM: `assert!` vs `debug_assert!` doc mismatch**
- `mod.rs:617` — doc says "Panics (debug_assert)" but code uses `assert!` (panics in release)
- If semantic validator has a bug, compiler crashes in production instead of producing diagnostic
- Fix: Either use `debug_assert!` or fix the doc (S)

**MEDIUM: Stringly-typed warnings collection**
- `warnings: Vec<String>` instead of `Vec<Diagnostic>` — no spans, no severity, no suggestions
- IDE (Poiesis) cannot position warnings in source editor
- Fix: Convert to `Vec<Diagnostic>` with `Severity::Warning` (M)

**MEDIUM: Large files exceeding 800-line guideline**
- `tests.rs` (1,849 lines), `react/view.rs` (948 lines), `view/element.rs` (744 lines)
- Fix: Split following existing `tests_regression/` pattern

**LOW: Unnecessary unwrap after `is_some()` check** — `validate_refs.rs:159`
**LOW: Guarded unwrap uses accessor instead of destructured variable** — `flutter/view.rs:237`
**LOW: OmniView `expr_to_js` doesn't use shared `FormatExpr` trait** — cross-cut with Theme 1
**LOW: ts-rs attribute warnings in build output** — add `#[ts(skip)]`
**LOW: `Arc<ComponentIR>` in single-threaded context** — keep for ADR-009 future-proofing

### Strengths
- Zero unsound code — all 8 `unsafe` blocks have detailed `// SAFETY:` comments
- ComponentScope pattern: ownership-based isolation of per-component state
- FormatExpr trait hierarchy with language-family implementations
- Consistent use of `FxHashMap`/`FxHashSet` for compiler-internal hashing
- Typed diagnostics with builder pattern and `thiserror::Error`

</details>

<details>
<summary>10. TypeScript & React Specialist — 3 HIGH, 6 MEDIUM, 2 LOW</summary>

### Perspective
Evaluates React component architecture, TypeScript type safety, state management patterns, and generated code quality from the React codegen target.

### Key Findings

**HIGH: Inline style `$signal` meta-objects skip ref-name translation**
- `view.rs:846` — `format_style_value_expr` returns bare signal name without checking `ref_names`
- `box.offsetWidth` in inline style emits as `box` instead of `boxRef.current` → runtime crash
- Every other expression formatter accepts `ref_names`; this is the sole exception
- Fix: Add `ref_names` parameter and apply check in `$signal` branch (S, ~10 lines)

**HIGH: Async `@unmount` lifecycle missing `body_has_await` check**
- `effect.rs:112-129` — `@mount` and `@effect` both check for async; `@unmount` does not
- `await` in unmount produces broken async function inside React cleanup (not awaited)
- Fix: Add `body_has_await` detection with async IIFE wrapper, matching `@mount`/`@effect` pattern (S)

**HIGH: Motion `document.querySelectorAll` breaks component isolation**
- All 6 motion codegen paths use global `document.querySelectorAll('[data-motion-*]')`
- Two components with `@enter` double-activate each other's elements
- Fix: Scope queries to component root via `useRef` + scoped `querySelector` (M-L)

**MEDIUM: `_M` motion constant recreated every render** — should use `useMemo(() => ({...}), [])`
**MEDIUM: Motion hover/press effects always emitted even when unused** — ~50 lines dead JS per component
**MEDIUM: Inline event handler formatting has duplicated code paths** — 3 nearly-identical blocks
**MEDIUM: List rendering key strategy fails for object items** — `_i` index available but unused
**MEDIUM: `escape_js_ident` misses TypeScript keywords** — `type`, `as`, `any`, `never`, `unknown`, etc.
**MEDIUM: Generated code formatting inconsistencies** — mixed arrow spacing, destructuring styles

**LOW: Hooks-mode store causes full re-render on any signal change** — documented tradeoff
**LOW: Pipeline-level formatting** — recommend adding Prettier pass to codegen output

### Strengths
- Precise hook selection — only imports hooks actually needed (no dead imports)
- Zustand store mode with correct immutable patterns (`set((state) => ({...}))`)
- Element type mapping for useRef (HTMLInputElement, HTMLCanvasElement, SVGElement, HTMLElement fallback)
- CSS module scoping with proper class merging
- Store mode abstraction (Hooks vs Zustand) with clean API separation

</details>

<details>
<summary>11. Compiler & Language Architect — 2 MEDIUM, 3 LOW</summary>

### Perspective
Evaluates language design decisions, pipeline correctness, multi-target codegen consistency, IR design, and the semantic gap between source language and compilation targets.

### Key Findings

**MEDIUM: OmniView multi-file path skips stdlib import validation**
- `lib.rs:400-418` — OmniView path bypasses `codegen::compile()` → skips `validate_stdlib_imports()`
- React and Flutter get compile-time stdlib errors; OmniView gets runtime failures
- Fix: Add `validate_stdlib_imports(ir, Target::OmniView)` call (S, ~15 lines)

**MEDIUM: OmniView expression formatter duplicates shared FormatExpr trait**
- Same finding as Systems Architect, Maintainability Expert (cross-cut Theme 1)
- Fix: Migrate to trait-based delegation

**LOW: TypeKind::Named is opaque string with no recursive validation** — ADR-001 deferred item
**LOW: Non-OmniView codegen `target_spans` are empty** — no source mapping for React/Flutter
**LOW: React view tree traversal functions duplicated** — 4 independent walkers with identical structure

### Strengths
- Clean pipeline separation — 6 stages with typed input/output contracts
- FormatExpr/FormatStmt/FormatType language-family trait architecture — excellent DRY
- Comprehensive IR coverage — ExprIR (22 variants), StatementIR (14 variants)
- Feature-gated backends with traceable ADR-003 prioritization
- Well-tested lowering — 75+ test cases across 1500+ lines

</details>

---

## Appendix: Finding Severity Distribution

```
CRITICAL:  0  █
HIGH:     11  ███████████
MEDIUM:   28  ████████████████████████████
LOW:      26  ██████████████████████████
```

### HIGH findings by specialist:
- Systems Architect: 3 (FormatExpr bypass, dual dispatch, OmniViewGenerator coupling)
- Domain Expert: 2 (action/function merge, stringly-typed output_mode)
- TS/React Specialist: 3 (ref-name in inline style, async unmount, motion isolation)
- Implementation Strategist: 2 (IR exhaustiveness, CI burden)
- Maintainability Expert: 2 (dual dispatch, expr formatter bypass — overlapped)
- Pragmatic Enforcer: 1 (UiCodegen trait — but this is a removal, not a bug)

Note: The two highest-impact findings are actually MEDIUM in their specialists' ratings but have CRITICAL implications for production security — React URL/CSS sanitization (Security C1, C2). These are elevated to CRITICAL priority in the consolidated recommendations based on cross-specialist impact analysis.

---

*Review compiled 2026-07-30 by the AI Software Architect framework v1.6.0*
*11 independent specialist reviews → cross-cut analysis → 47 prioritized recommendations*
