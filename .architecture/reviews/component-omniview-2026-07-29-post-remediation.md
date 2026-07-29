# Architecture Review: OmniView Codegen — 2026-07-29 Post-ADR-026 Remediation Assessment

**Date**: 2026-07-29
**Review Type**: Post-Remediation Assessment (follow-up to v2 review and ADR-026)
**Reviewers**: Compiler & Language Architect (Dr. Lin Wei), Security Specialist, Rust Expert (Katya Volkov), Maintainability Expert, Pragmatic Enforcer

> **Context**: The v2 comprehensive review (`component-omniview-2026-07-29-v2.md`, 11 specialists, 44 findings) identified 0 CRITICAL, 6 HIGH, 23 MEDIUM issues. All 16 actionable findings were fixed in 17 commits (documented in ADR-026). This review assesses the post-remediation current state.

---

## Executive Summary

**Overall Assessment: Strong → Stronger**

The ADR-026 remediation wave achieved its goals with **zero net code growth** (+227 / −227 lines across 22 files). All 5 specialists independently verified the correctness of the 16 fixes. The Pragmatic Enforcer confirmed every fix had an **excellent necessity-to-complexity ratio**. No fix introduced regressions.

This post-remediation review found **20 findings** across 5 specialists:
- **0 CRITICAL** — no new exploitable vulnerabilities or data-loss risks
- **0 HIGH** from the fixes themselves — the ADR-026 fixes are correct
- **2 HIGH** pre-existing deferred items (CSS-only block duplication, motion trigger duplication) — stable, not deteriorating
- **9 MEDIUM** — primarily security scope gaps (sanitization coverage, CSP directives) and one correctness bug (direct nested @for)
- **9 LOW** — documentation staleness, dead code, minor Rust idioms

**The single most actionable finding**: The Compiler Architect discovered that direct nested `@for` (without component wrapping) still has an `in_for_template` state leak at `list.rs:60` — a **one-line fix** that was outside the scope of the C1 fix (which targeted the component-inlining case).

**Top-line**: The codegen is solid post-remediation. The remaining findings are scope gaps in the new security layer and deferred code-quality items — none are regressions from the fixes.

---

## Cross-Cutting Themes

### Theme 1: Security Hardening — V1 Completeness (3 specialists)

The ADR-026 security layer (`__omni_sanitizeUrl`, `__omni_sanitizeCss`, CSP meta tag) is correctly implemented and covers the primary attack vectors identified in the v2 review. However, the Security Specialist and Compiler Architect identified scope gaps:

- **URL sanitization**: blocks `javascript:`, `data:text/html`, `vbscript:` — but not `data:text/javascript` or other `data:` subtypes (Security M1)
- **CSS sanitization**: blocks `url()` entirely — overly aggressive, breaks legitimate `background-image: url(...)` via signals (Security M3)
- **CSP directives**: missing `base-uri`, `frame-ancestors` (Security M2, M6)
- **Static attribute path**: compile-time URL attributes bypass sanitization entirely (Security M5)
- **No runtime sanitization tests**: ~20 lines of security-critical JS with zero behavioral tests (Compiler M2)

**Consensus**: These are "v1 completeness" issues, not design flaws. The sanitization architecture (compile-time → runtime layering) is correct; the scope needs expansion.

### Theme 2: Pre-existing Technical Debt — Stable (Maintainability + Pragmatic Enforcer)

Three items deferred from the v2 review (N4: CSS-only block duplication, N5: motion trigger dedup, N6: test_new() constructor) remain unchanged — no deterioration, no new urgency. The Pragmatic Enforcer confirmed these are genuine debt but not actively causing problems.

### Theme 3: Nested @for Correctness Gap (Compiler Architect)

The C1 fix (save/restore around component inlining) handles the component-wrapping case correctly. But the **direct nested @for case** (two `@for` loops nested without a component between them) still has a state leak: `emit_list_state_init` unconditionally sets `in_for_template = false` at `list.rs:60`, corrupting the outer loop's template generation for any content after the inner loop. One-line fix: save and restore `in_for_template` instead of unconditionally setting to `false`.

---

## Trade-offs and Disagreements

### T1: CSS `url()` blocking — security vs. functionality

**Security Specialist**: The blanket `url()` block in `__omni_sanitizeCss` prevents CSS injection via `url(javascript:...)` but also blocks legitimate `url("https://...")` usage, making signal-driven `background-image` impossible.

**Pragmatic Enforcer**: This is the wrong trade-off. Developers will work around the sanitizer (using inline styles, direct DOM manipulation, or removing the sanitizer), weakening the security posture more than a targeted check would.

**Resolution**: Relax the `url()` check to allow `https://`, `/path/`, and `data:image/` schemes while blocking `javascript:` and `vbscript:` inside URLs. This is an S-effort change.

### T2: Error severity categorization (D6) — YAGNI or infrastructure?

**Pragmatic Enforcer**: The error severity taxonomy (`warn:`/`error:`/`fatal:` prefix parsing) is the only ADR-026 fix with a borderline YAGNI profile. The AI diagnostic tools that would consume it don't exist yet.

**AI Engineer (from v2)**: The 100-entry FIFO buffer without severity was a real problem — transient errors evicted real bugs. Structured errors enable future tooling.

**Resolution**: Keep the fix. The runtime cost is zero (error-handling code, executed only on errors), and the benefit for observability is real even without AI tooling — developers debugging in browser console benefit from `console.warn` vs `console.error` distinction.

---

## Prioritized Recommendations

### Critical (0-2 weeks) — 5 items

| # | Finding | Source | Effort | File(s) |
|---|---------|--------|--------|---------|
| **P1-1** | **Fix direct nested @for `in_for_template` leak** — Replace `self.in_for_template = false` with `self.in_for_template = saved_outer_value` at `list.rs:60` | Compiler Architect (MEDIUM) | XS (1 line) | `list.rs:58-60` |
| **P1-2** | **Extend URL sanitization to block all `data:` schemes** — Add `data:text/javascript`, `data:application/javascript`, or block all `data:` prefixes | Security Specialist (MEDIUM) | S | `runtime.rs:136` |
| **P1-3** | **Add `base-uri 'self'` and `frame-ancestors 'self'` to CSP** — Prevent base-tag hijacking and clickjacking | Security Specialist (MEDIUM) | XS | `utils.rs:248` |
| **P1-4** | **Coerce input to string before sanitization** — `var str = String(url)` in `__omni_sanitizeUrl` and `__omni_sanitizeCss` to prevent `toString()` bypass | Security Specialist (MEDIUM) | XS | `runtime.rs:134,143` |
| **P1-5** | **Fix UTF-8 byte boundary slice in `expr_default_display`** — Use `char_indices()` pattern (matching `expr.rs:382-388`) | Rust Expert (MEDIUM) | XS | `store.rs:25` |

### Important (2-8 weeks) — 6 items

| # | Finding | Source | Effort |
|---|---------|--------|--------|
| **P2-1** | **Relax CSS `url()` block** — Allow safe URL schemes (`https://`, `/`, `data:image/`) while blocking `javascript:`/`vbscript:` inside URLs | Security Specialist (MEDIUM) | S |
| **P2-2** | **Add runtime sanitization JS tests** — Standalone JS test file exercising `__omni_sanitizeUrl` and `__omni_sanitizeCss` with edge cases (I10 from v2) | Compiler Architect + Security (MEDIUM) | M |
| **P2-3** | **Add compile-time URL sanitization for static attributes** — Apply protocol check to literal `href`/`src` values (supply-chain defense) | Security Specialist (MEDIUM) | S |
| **P2-4** | **Add `srcset` and `poster` to `is_url_attr`** or document exclusion | Security Specialist (MEDIUM) | XS |
| **P2-5** | **Replace `debug_assert!(false, …)` with `unreachable!()`** for `LifecycleKind::Effect` in `store.rs:333` — catch invariant violations in release builds | Rust Expert (MEDIUM) | XS |
| **P2-6** | **Extract CSS-only capture to shared method** — Eliminate 31-line block duplicated in void/normal tag branches (N4 from v2) | Maintainability Expert (HIGH) | S |

### Nice-to-Have (2-6 months) — 9 items

| # | Finding | Source | Effort |
|---|---------|--------|--------|
| **P3-1** | **Move ResizeEvent special case into `emit_event_listener`** — Consolidate `window.addEventListener` handling | Compiler Architect (LOW) | S |
| **P3-2** | **Remove React + Vue UiCodegen dead code** — Or wire up call sites (D8 was OmniView-only) | Compiler Architect (LOW) | S |
| **P3-3** | **Add `TriggerKind::data_attr()` / `trigger_name()` methods** — Eliminate duplicated string mapping (M1 from v2) | Maintainability Expert (MEDIUM) | S |
| **P3-4** | **Derive `Default` for `OmniViewGenerator`** — Eliminate `test_new()` 45-field duplication (N6 from v2) | Maintainability Expert (MEDIUM) | S |
| **P3-5** | **Split `generate_component` into 5-6 smaller methods** — 374-line method, 9 responsibilities | Maintainability Expert (MEDIUM) | M |
| **P3-6** | **Extract motion sub-attribute helpers** — `emit_inview_sub_attrs`, `emit_scroll_sub_attrs`, `emit_drag_sub_attrs` | Maintainability Expert (MEDIUM) | S |
| **P3-7** | **Clean up `formaction` dead code** — Either add to recognized attribute list or remove from `is_url_attr` | Security Specialist (LOW) | XS |
| **P3-8** | **Update stale doc comment in `debug.rs:10`** — References removed `finalize_js` method | Rust Expert (LOW) | XS |
| **P3-9** | **Use `HashMap::get` instead of `contains_key` + index** in `expr.rs:36` | Rust Expert (LOW) | XS |

---

## ADR-026 Fix Verification — Specialist Consensus

| Fix | Compiler | Security | Rust | Maintain. | Pragmatic | Verdict |
|-----|----------|----------|------|-----------|-----------|---------|
| C1: @for save/restore | ✅ Correct (component case) | — | ✅ Sound | ✅ Good pattern | ✅ Justified | **Accepted** |
| C2: mutation inline | ✅ Elegant | — | ✅ Correct | ✅ Working | ✅ Excellent ratio | **Accepted** |
| C3: boolean attr | ✅ Future-proof | — | — | ✅ Correct | ✅ 1-line fix | **Accepted** |
| C4: URL sanitization | ✅ Placed right | ⚠️ Scope gaps | ✅ Sound | — | ✅ Justified | **Accepted, gaps noted** |
| I1: event listener dedup | ✅ Unified | — | ✅ Semantics preserved | ✅ Clean | ✅ Net negative complexity | **Accepted** |
| I2: CSP meta tag | — | ⚠️ Missing directives | — | — | ✅ Justified | **Accepted, gaps noted** |
| I3: CSS sanitization | ✅ Correct layer | ⚠️ Over-broad url() | ✅ Sound | — | ✅ Justified | **Accepted, gaps noted** |
| I5: error severity | ✅ Well-structured | — | ✅ Consistent | — | ⚠️ Borderline YAGNI | **Accepted (keep)** |
| N2: UiCodegen removal | ✅ OmniView done | — | ✅ Gone | ✅ Clean | ✅ -120 lines | **Accepted** |
| D5: phase guards | ✅ Good placement | — | ✅ Idiomatic | ✅ Defensive | ✅ Excellent ratio | **Accepted** |
| D6: source span length | ✅ Propagated | — | ✅ Correct | — | ✅ Justified | **Accepted** |
| N7: design class HashSet | ✅ O(1) correct | — | ✅ No snapshot churn | — | ✅ Justified | **Accepted** |

---

## Risks if Unaddressed

### Immediate Risk

1. **Direct nested @for silent corruption (P1-1)**: Any `.omni` source with two `@for` loops directly nested (no component between them) where dynamic content appears after the inner loop will produce silently non-functional output. The inner `@for` sets `in_for_template = false` unconditionally, causing content after it to generate wrong JS patterns. **No compile error, no runtime error — just non-updating DOM.**

2. **URL sanitization bypass via `data:text/javascript` (P1-2)**: Editor mode (no CSP) + signal-driven `href`/`src` with `data:text/javascript,alert(1)` → arbitrary JS execution. Current sanitizer only blocks `data:text/html`.

3. **`toString()` sanitization bypass (P1-4)**: Computed value returning `{toString: () => "javascript:alert(1)"}` bypasses `typeof !== 'string'` guard. Low likelihood (requires intentional malicious computed) but undermines defense-in-depth.

### Medium-term Risk

4. **CSS `url()` block breaks functionality (P2-1)**: Signal-driven `background-image: url("https://...")` silently fails. Developers may disable or bypass the sanitizer, weakening security posture more than a targeted check would.

5. **CSP gaps (P1-3)**: Missing `base-uri` and `frame-ancestors` directives leave clickjacking and base-tag hijacking vectors open in production deployments.

---

## Success Metrics

| Metric | Pre-ADR-026 (v2) | Post-ADR-026 | Target |
|--------|-------------------|--------------|--------|
| HIGH-severity findings | 6 | 0 | 0 |
| MEDIUM-severity findings | 23 | 9 (all new scope gaps) | → 0 |
| Duplicated event handler emission | 8 points (2 files) | 1 method | ✅ Met |
| Dead code (UiCodegen) | ~120 lines | 0 (OmniView) | ✅ Met |
| Runtime URL sanitization | None | `__omni_sanitizeUrl` (3 schemes) | Expand to all data: |
| Runtime CSS sanitization | None | `__omni_sanitizeCss` (9 patterns) | Relax url() block |
| CSP in standalone output | None | `default-src 'self'` + 4 directives | Add base-uri, frame-ancestors |
| Net code change | — | +227 / −227 (zero net) | ✅ Met |
| Test count (omniview) | ~88 | ~93 (estimated) | Add JS runtime tests |
| Clippy warnings | 0 | 0 | ✅ Maintained |

---

## Follow-up Plan

1. **Immediate** (this session or next): Fix P1-1 through P1-5 — all XS/S effort, addressing the 5 most actionable findings.
2. **This week**: P2-1 through P2-6 — close security scope gaps and remaining MEDIUM items.
3. **Next sprint**: P3-1 through P3-9 — deferred code-quality items, documentation fixes.
4. **ADR creation**: ADR-027 documenting the security sanitization architecture decisions, including:
   - URL scheme allowlist/blocklist rationale
   - CSS `url()` relaxation strategy
   - CSP directive selection criteria
   - `in_for_template` depth-counter pattern (long-term)

---

## Per-Specialist Detailed Reviews

The full individual reviews are preserved below as source data. The aggregation above summarizes and cross-references, but does not replace these verbatim assessments.

### Compiler & Language Architect (Dr. Lin Wei) — IR & Codegen Pipeline

**Perspective**: Evaluates language design soundness, IR representation fidelity, multi-target codegen consistency, and the semantic gap between source language and compilation targets.

#### Key Observations

1. The `@for` save/restore fix (C1) is correct for component-inlining but the direct nested `@for` case remains — `emit_list_state_init` unconditionally sets `in_for_template = false`, leaking state to outer loops
2. The `&mut self` cascade from `format_mutation_stmt_inline` fix is functionally necessary but architecturally notable — `expr_to_js` (called ~400+ times) is now mutable-only
3. ResizeEvent special case remains duplicated between `emit_element_events` and `component_html.rs` event path
4. Runtime sanitization functions have zero standalone JS test coverage — security-critical functions with no behavioral tests
5. UiCodegen dead code persists in React and Vue backends (only OmniView removed)

#### Strengths

- Buffer-capture pattern for `format_mutation_stmt_inline` — textbook use of `std::mem::take`/`replace` to reuse validated implementation without duplication
- Boolean attribute detection via `ExprIR` type matching — future-proof against new expression variants
- Save/restore pattern consistency — new @for state save/restore follows exact same pattern as existing signal_names/name_renames
- Phase guard placement reflects correct understanding of dual-use methods (assert_can_emit_js vs assert_not_css_only split)
- Design class HashSet optimization correct and maintains semantic equivalence (zero snapshot changes)

#### Concerns

**CONCERN 1 (MEDIUM): Direct nested @for `in_for_template` leak**
- **Location**: `list.rs:58-60`
- **Fix**: Replace `self.in_for_template = false` with `self.in_for_template = outer_in_for` (one-line change)

**CONCERN 2 (MEDIUM): Runtime sanitization functions have zero standalone test coverage**
- **Location**: `runtime.rs:133-153`
- **Fix**: Add JS test file exercising `__omni_sanitizeUrl` and `__omni_sanitizeCss` (I10 from v2)

**CONCERN 3 (LOW): ResizeEvent special case remains duplicated**
- **Location**: `events.rs:90-121` vs `component_html.rs:186-198`

**CONCERN 4 (LOW): UiCodegen dead code persists in React/Vue backends**
- **Location**: `react/mod.rs:859-883`, `vue/mod.rs:365-381`

**CONCERN 5 (LOW): `expr_to_js` `&mut self` cascade — design constraint for future refactoring**

#### Recommendations

**Immediate**: Fix nested @for `in_for_template` leak (1 line). Add runtime sanitization tests.
**Short-term**: Move ResizeEvent special case into `emit_event_listener`. Remove React/Vue UiCodegen dead code.
**Long-term**: Consider `in_for_template` as depth counter instead of boolean. Wrap @for binding buffers in scoped `LoopContext` struct.

---

### Security Specialist — Security Analysis

**Perspective**: Evaluates the effectiveness of ADR-026 security mitigations (C4 URL sanitization, I2 CSP, I3 CSS sanitization) and identifies remaining gaps.

#### Key Observations

1. Three distinct trust boundaries: compile-time (omni source → generated code), runtime-signal (dynamic values from signals), editor-mode (postMessage → eval bridge)
2. Sanitization follows layered approach: Rust-side for compile-time, JS runtime for signal-driven values, `escape_html` for text content
3. CSP is bifurcated: production mode includes CSP; debug/editor modes don't (intentional, documented)
4. All signal-to-DOM reactive bindings use safe APIs (`textContent`, `setAttribute`, `el.style.prop =`)

#### Strengths

- Consistent sanitization across all three dynamic style paths ($signal, $concat, $call)
- Reactive text uses `textContent` — immune to HTML injection
- HTML event handler attributes (onclick, onerror) not in recognized attribute list — fall through to `data-onclick`
- `__omni_sanitizeUrl` correctly lowercases and trims — catches mixed-case bypasses
- Runtime and compile-time CSS sanitizers check same patterns — consistent

#### Concerns

**MEDIUM-1**: `__omni_sanitizeUrl` blocks only `data:text/html`, not `data:text/javascript` or other executable `data:` subtypes
- **Location**: `runtime.rs:136`

**MEDIUM-2**: CSP missing `base-uri` directive
- **Location**: `utils.rs:248`

**MEDIUM-3**: `__omni_sanitizeCss` blocks ALL `url()` — overly aggressive, breaks legitimate `background-image: url(...)` via signals
- **Location**: `runtime.rs:146`

**MEDIUM-4**: `__omni_sanitizeUrl` returns non-string values unsanitized — `toString()` bypass
- **Location**: `runtime.rs:134`

**MEDIUM-5**: Static URL attributes not sanitized at compile time — supply-chain gap
- **Location**: `element.rs:175`

**MEDIUM-6**: CSP missing `frame-ancestors` directive — clickjacking vector
- **Location**: `utils.rs:248`

**MEDIUM-7**: `srcset` and `poster` not in URL sanitization
- **Location**: `element.rs:149`

**LOW-8**: `formaction` dead code in `is_url_attr` — not in recognized attribute list
**LOW-9**: Editor mode CSP gap documented but not mitigated

#### Risks if Unaddressed

- `data:text/javascript` execution in editor mode (M1): signal control → JS execution
- Base tag attack surface (M2): compromised component → resource load redirection
- `url()` block breaks functionality (M3): developers bypass sanitizer → weaker security
- No frame protection (M6): clickjacking vector in production

---

### Rust Expert (Katya Volkov) — Rust Safety & Idioms

**Perspective**: Evaluates memory safety, ownership patterns, zero-cost abstractions, and Rust idiom adherence.

#### Key Observations

- Zero `unsafe` blocks in production code; zero production `unwrap()` calls
- `std::mem::take` patterns used correctly throughout (@for save/restore, buffer capture)
- String slicing for buffer capture (`self.js[old_js_len..]`) is sound in current context but fragile
- Design relies heavily on `&mut self` — deliberate choice for incremental code generation

#### Strengths

- `std::mem::take` for @for state save/restore — idiomatic, correct
- `format_mutation_stmt_inline` buffer capture — double-swap pattern is zero-cost and correct
- `emit_event_listener` extraction — semantics preserved across all variants
- Pre-built `design_class_names` HashSet — correct O(1) optimization
- Debug assertion guards with `#[track_caller]` — zero release cost
- Consistent severity prefix handling in `__omni_reportError`

#### Concerns

**MEDIUM: UTF-8 byte boundary slice in `expr_default_display`**
- **Location**: `store.rs:25` — `&js[..39]` not char-boundary-aware; neighboring `expr.rs:382-388` has correct pattern

**MEDIUM: `debug_assert!(false, …)` silent-in-release for `LifecycleKind::Effect`**
- **Location**: `store.rs:333-337` — invariant violation produces no feedback in release builds
- **Fix**: Replace with `unreachable!()` or add production-path diagnostic

**LOW: Guarded HashMap index with fragile pre-check** — `expr.rs:36`
**LOW: Stale doc comment referencing removed `finalize_js`** — `debug.rs:10`
**LOW: String index for quote stripping assumes ASCII** — `store.rs:16`

---

### Maintainability Expert — Code Quality & Technical Debt

**Perspective**: Evaluates file organization, duplication patterns, module coupling, test coverage, and remaining technical debt.

#### Key Observations

- 23 Rust files across 3 directories, max 770 lines (within 800-line guideline)
- Dependency graph clean: `mod.rs` owns struct, all others `impl` methods on it
- No circular module dependencies
- 88 tests all passing; gaps in motion trigger, inline style dynamic binding, editor-mode interplay

#### Strengths

- Clean separation of motion concerns (5 focused files)
- Consistent JS execution order documented with 6-phase rationale
- Defensive programming with `assert_can_emit_js()` / `assert_not_css_only()`
- Per-component state isolation pattern correct
- Good use of `std::mem::take` patterns

#### Concerns

**HIGH 1: CSS-only mode block duplication (deferred N4)**
- **Location**: `element.rs:490-521` (void) vs `element.rs:564-594` (normal) — 31 identical lines

**HIGH 2: Motion trigger factory duplication (deferred N5)**
- **Location**: `motion_triggers.rs:350-600` — 17 match arms across named/inline branches

**MEDIUM 1: TriggerKind string mapping duplicated 2x** — `element.rs:326-339` + `motion_triggers.rs:327-340`

**MEDIUM 2: `test_new()` duplicates 45-field constructor (deferred N6)** — `mod.rs:555-606`

**MEDIUM 3: `generate_component` is 374 lines** — 9 responsibilities in one method

**MEDIUM 4: `emit_element_motion_attrs` handles too many sub-attribute types inline** — 87 lines

**LOW 1-3**: Minor style boilerplate in style.rs, in-view trigger branching, element bindings

---

### Pragmatic Enforcer — YAGNI & Simplicity

**Perspective**: Evaluates whether each ADR-026 fix carried its weight in necessity vs. complexity.

#### Necessity / Complexity / Ratio Analysis

| Fix | Necessity | Complexity | Ratio | Verdict |
|-----|-----------|------------|-------|---------|
| D1: &mut self cascade | Real bug (invalid JS) | 1-line cascade + 4-line buffer swap | Excellent | **Keep** |
| D2: Boolean attr | Real bug (permanent true) | 1 line | Excellent | **Keep** |
| D3: @for save/restore | Real bug (buffer corruption) | 20 lines, proven pattern | Acceptable | **Keep** |
| D4: Runtime sanitizers | Real XSS vector | ~20 lines runtime JS | Excellent | **Keep** |
| D5: Phase guards | Proven bug source | 30 lines, zero release cost | Excellent | **Keep** |
| D6: Error severity | New capability (no consumer) | 30 lines runtime JS | Borderline | **Keep** (cost too low to revert) |
| D7: Event listener dedup | Prevention (drift risk) | Net -100 lines | Excellent | **Keep** |
| D8: UiCodegen removal | Dead code | Net -120 lines | Excellent | **Keep** |
| finalize_js inline | Single-line method | Net -4 lines | Excellent | **Keep** |

#### Key Observations

- **Net code change: zero** (+227 / −227 lines) — perfect complexity balance
- `&mut self` cascade is benign — all 9 methods already wrote to mutable state; `pub(super)` scope limits impact
- Runtime sanitizer layering is correct — compile-time for static, runtime for dynamic
- Deferred items (M2, L1, L2) stable — no deterioration since v2 review
- Error severity is the only borderline YAGNI item — keep given minimal cost

#### Concerns

**MEDIUM: Two bridge methods (editor + debug) could conflict if a third mode activates both** — edge case, no action needed now

**LOW: `emit_element_html_close` has 7 parameters** — function growing, consider splitting if it exceeds 10 params

---

## Review Metadata

- **Total specialists**: 5 (Compiler Architect, Security Specialist, Rust Expert, Maintainability Expert, Pragmatic Enforcer)
- **Files examined**: 21 source files in `omnilang/src/codegen/omniview/`
- **Lines of code**: ~10,753 implementation + ~3,651 test
- **ADR context**: ADR-021 through ADR-026
- **Previous review**: `component-omniview-2026-07-29-v2.md` (11 specialists)
- **Methodology**: 5 parallel specialist subagents, independent deep-dive, cross-cut aggregation
- **Pragmatic mode**: enabled (balanced intensity)
- **Duration**: ~7 minutes wall-clock (parallel dispatch)
- **Key insight**: The codebase has reached diminishing returns on comprehensive review cycles — remaining findings are scope gaps in new features (security layer) and long-deferred code-quality items, not regressions or design flaws
