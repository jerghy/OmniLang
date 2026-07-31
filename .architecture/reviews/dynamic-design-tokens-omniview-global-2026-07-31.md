# Architectural Review: Dynamic Design Tokens — Global OmniView Integration

## Overview

- **Review Target**: Dynamic Design Tokens (Phase 1 compile-time const refs + Phase 2 runtime signal refs) — **global OmniView framework integration perspective**
- **Review Date**: 2026-07-31
- **Previous Review**: 2026-07-31 (feature-level, 11 specialists, 3 HIGH + 9 MEDIUM → 13 commits fixed)
- **This Review**: 12 specialists (Systems Architect, Compiler & Language Architect, Security Specialist, Performance Specialist, Rust Expert, TypeScript & React Specialist, Maintainability Expert, Domain Expert, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Documentation Expert)
- **Methodology**: Multi-perspective parallel review with cross-cut analysis
- **State at Review**: 552+ tests passing, ADR-050 validation gaps closed, 5 deferred items (M1, P3-1 through P3-4)
- **Previous Review**: `.architecture/reviews/dynamic-design-tokens-2026-07-31.md` — focused on feature-level correctness; this review extends to framework-level integration

---

## Executive Summary

The Dynamic Design Tokens feature has a **clean core architecture** — the `TokenValue` (AST: `Literal|Variable|Expression`) → `TokenValueIR` (IR: `Static|Dynamic`) mapping is an elegant separation of user-intent from execution-strategy. The Phase 1/2 split followed correct YAGNI discipline. The ADR-050 validation gap closure resolved the three most critical silent-degradation paths.

However, this global review reveals **3 CRITICAL framework-level issues** that the previous feature-level review did not surface:

1. **Dynamic token effects accumulate across editor hot-reload cycles** — no dispose/cleanup path, linear memory leak (6 specialists converged)
2. **Design blocks produce zero output on all 9 non-OmniView backends** — silent data loss with no diagnostic (5 specialists)
3. **Feature is completely invisible in user-facing documentation** — grammar BNF wrong, no tutorial, no migration guide (3 specialists)

Additionally, the **Pragmatic Enforcer** found that Phase 2 has **zero non-test consumers** in the reference learn-omniview project — `style: { --var: signal }` already achieves the same result without the 5-layer pipeline overhead.

**Overall Architecture Health**: **Adequate** — core design strong, validation gaps closed, but significant framework-level integration gaps remain. The feature is functionally correct for the OmniView target, but the cross-backend gap, hot-reload lifecycle, and documentation invisibility must be addressed before shipping as production-ready.

### Key Strengths
- Clean static/dynamic IR enum split makes codegen incorrectness structurally impossible
- Additive, reversible architecture — both phases add enum variants without restructuring
- ADR-050 validation gap closure was thorough and correctly prioritized
- Comprehensive test coverage (552+ tests across all pipeline layers)
- Excellent error messages (E0001 with syntax fix suggestions) — AI-self-correction ready
- Phase 1/2 split discipline — Phase 1 first, Phase 2 deferred until need materialized

### Critical Concerns
- **C1**: Dynamic token `__omni_effect` closures accumulate across hot-reload cycles with no dispose path — linear memory leak in editor mode
- **C2**: Design blocks produce zero CSS output on React/Flutter/Vue/Dioxus/Dart/Python/JS/Rust backends — silent data loss with zero diagnostics
- **C3**: User-facing documentation (grammar BNF, design-system guide) has zero mention of dynamic tokens — feature is undiscoverable

---

## Cross-Cutting Themes

### Theme 1: Hot-Reload Lifecycle Gap (6 specialists)

**Flagged by**: Systems Architect (CRITICAL), TypeScript & React Specialist (3 HIGH), Performance Specialist (MEDIUM), Implementation Strategist, AI Engineer, Pragmatic Enforcer

The `emit_dynamic_token_effects()` function generates `__omni_effect(() => { ... })` calls whose return value (cleanup function) is discarded. In editor mode, each hot-reload creates new effect closures while old ones remain subscribed to signals. Over repeated edits, this produces:
- Accumulating signal subscriptions (N × reloads effects)
- Redundant `setProperty` calls on every signal change
- No mechanism to dispose old effects (unlike component effects which are cleaned up per-nid)

The TypeScript/React specialist identified the concrete HMR mechanism: token effects are emitted to `gen.js` (top-level IIFE), not `component_js`. The HMR diff in `useDebugCompile.ts` only checks per-component hashes, so token expression changes never trigger hot-replace — the stale IIFE persists indefinitely.

### Theme 2: Cross-Backend Design Token Gap (5 specialists)

**Flagged by**: Systems Architect (HIGH), Compiler & Language Architect (MEDIUM), Domain Expert (MEDIUM), Implementation Strategist (CRITICAL), Documentation Expert

`ModuleIR.designs: Vec<DesignIR>` is populated during IR lowering and passed to every codegen backend via the `Codegen` trait. But only `codegen/omniview/` reads this field. All other backends (React, Flutter, Vue, Dioxus, Dart, JavaScript, Python, Rust) silently ignore design blocks. There is no compile-time diagnostic. A user writing `token primary: color = "#6366f1";` and compiling to React gets zero CSS output — no `:root` declarations, no `[data-theme]` selectors, no `__omni_effect` bindings.

The React backend has partial awareness (it knows about `_design_variant` props) but no corresponding design token codegen implementation.

### Theme 3: Parser Expression Boundary Scanner Fragility (4 specialists)

**Flagged by**: Compiler & Language Architect (HIGH), Domain Expert (HIGH), Systems Architect (MEDIUM), AI Engineer (LOW)

The dual-mode parser bridge (`parse_design_value_expression()` in `parser/design.rs:298-371`) scans token streams to find expression boundaries by tracking paren/bracket depth and breaking at bare identifiers (depth==0, not preceded by dot). Two specific truncation cases were identified:

- **Binary operators**: `store.getValue() + someVar` — scanner breaks at `someVar` (bare identifier, prev is `Plus` not `Dot`), extracting only `[Ident("store"), Dot, Ident("getValue"), LParen, RParen, Plus]`. The sub-parser gets partial input and falls back to `TokenValue::Variable("store")` — silent degradation.
- **FontWeight type gap**: `TokenType::FontWeight` is missing from the numeric-accepting types list in `check_token_type_compatibility()`, despite CSS font-weight accepting numeric values 100-900.

### Theme 4: Documentation Discoverability Gap (3 specialists)

**Flagged by**: Documentation Expert (CRITICAL + 2 HIGH), AI Engineer (MEDIUM), Maintainability Expert (MEDIUM)

The feature has thorough internal documentation (3 ADRs, design doc, code comments) but is completely invisible to end users. The grammar BNF in `15-grammar-view.md` shows outdated token syntax (no type annotation, no semicolon). The `const` description in `14-grammar-lang.md` says "parsed but discarded" — obsolete since Phase 1. The user-facing design system guide (`10-design-system.md`) has zero mention of dynamic tokens, variable references, or the naming convention.

### Theme 5: Pipeline Validation Coverage Gaps (4 specialists)

**Flagged by**: Domain Expert (MEDIUM), Implementation Strategist (MEDIUM), Maintainability Expert (MEDIUM), AI Engineer (MEDIUM)

Several validation gaps remain in the semantic analysis layer:
- **Theme value const-kind asymmetry**: Theme values validate symbol existence but not const-kind, unlike primary values — same class of silent degradation as the original H3 gap
- **`var(--name)` fallback ambiguity**: When a const value is unresolved, the IR emits `var(--name)` — conflating "intentional CSS variable reference" with "resolution failure"
- **`infer_token_expr_type()` depth limit**: Only handles one level of `FieldAccess`; chained access (`Store.nested.field`) falls through to `infer_expr_type()` which returns `Any`, bypassing type checking
- **Expression parse failure → Variable fallback**: When `parse_design_value_expression()` fails, the parser silently falls back to `TokenValue::Variable(ident)` with no diagnostic

---

## Trade-offs and Disagreements

### T1: Single `__omni_effect` consolidation vs per-token granularity

**ADR-050 D5** consolidated N individual `__omni_effect` blocks into 1. The Performance Specialist notes this causes unnecessary `setProperty` calls for unchanged tokens (same-value `setProperty` is a browser no-op, but still triggers style invalidation path). The Systems Architect confirms the trade-off is correct at current scale (1-5 tokens). **Resolution**: Accept the consolidation; revisit if N grows past ~10.

### T2: `is_css_value()` heuristic — keep vs replace with quote-all-literals

**Pragmatic Enforcer** argues the heuristic (100 lines) is unnecessary YAGNI complexity — requiring quotes for all CSS values (`token easing: easing = "ease-in-out";`) would be simpler and deterministic. **Maintainability Expert** and **Domain Expert** note the heuristic is fragile but documented. **Resolution**: Keep for now (backward compatibility), but add a deprecation path to quote-all-literals in a future language version.

### T3: Phase 2 necessity — zero non-test consumers

**Pragmatic Enforcer** rates Phase 2 Necessity at 2/10 with Complexity at 6/10 (ratio 3.0, exceeding the 1.5 threshold). The `style: { --var: signal }` pattern already achieves the same result. **Counterpoint from Domain Expert**: the design token syntax is semantically different — it expresses "this is a design system concern" vs "this is a component-level style override." **Resolution**: Phase 2 is implemented and functional; the question is whether to invest further in it (Flutter support, SSR, animation) before production adoption materializes. Defer Phase 3 until real usage data exists.

### T4: Security sanitization in theme `[data-theme]` blocks

**Security Specialist** flagged that `sanitize_css_value()` is applied to `:root` values but missing from `[data-theme]` and `@media` blocks. **Counterpoint**: Theme values come from developer source code, not user input — the `:root` fix was also defense-in-depth. **Resolution**: Apply sanitization for consistency; it's a 2-line change with zero performance impact.

---

## Intentional Design Decisions (Aligned with ADRs)

The following findings from the global review align with explicit design decisions documented in past ADRs. These are NOT bugs — they represent intentional trade-offs that should be preserved:

| Finding | ADR | Design Intent |
|---------|-----|---------------|
| Theme sync is unidirectional | [ADR-044](.architecture/decisions/adrs/ADR-044-theme-store-mode-omni-theme-auto-sync.md) L83 | "The coupling is unidirectional (mode → \_\_omni\_theme)" — the store is the single source of truth. `__omni_theme.toggle()` is designed to be called FROM store actions, not independently |
| Binary operators not in expression scope | [ADR-047](.architecture/decisions/adrs/ADR-047-dynamic-variable-design-tokens.md) L137 | "Not a full expression system… not expressions like `BRAND + "80"` or function calls. This is intentional" |
| `var(--name)` fallback is graceful degradation | [ADR-047](.architecture/decisions/adrs/ADR-047-dynamic-variable-design-tokens.md) L128 | "Graceful degradation: Unresolved variables emit `var(--name)` CSS fallback — no compile error" (listed as Positive consequence) |
| `SymbolKind::Unknown` exemption | [ADR-050](.architecture/decisions/adrs/ADR-050-dynamic-token-validation-gap-closure.md) L159-162 | Rejecting Unknown would "break every cross-file const import." The exemption is the pragmatic choice until cross-file kind propagation is implemented |
| Effect consolidation trade-off | [ADR-050](.architecture/decisions/adrs/ADR-050-dynamic-token-validation-gap-closure.md) L109 | "Per-token dependency granularity is lost… Accepted because `setProperty` with the same value is a browser no-op" |

---

## Prioritized Recommendations

### Critical (0-2 weeks)

| # | Action | Effort | Addresses | Specialists |
|---|--------|--------|-----------|-------------|
| **R1** | Add effect dispose/cleanup for hot-reload: capture `__omni_effect` return value under `window.__omni_tokenEffectCleanup`, call before re-registering | S | C1 (hot-reload leak) | Systems, TS/React, Performance |
| **R2** | Add compile-time WARNING when design blocks are compiled to non-OmniView targets | XS | C2 (cross-backend gap) | Systems, Implementation, Domain |
| **R3** | Update `docs/omniview/10-design-system.md` with dynamic token usage, naming convention, and theme restriction | S | C3 (documentation invisibility) | Documentation, AI Engineer |
| **R4** | Fix grammar BNF in `15-grammar-view.md` (token_def syntax) and obsolete const description in `14-grammar-lang.md` | XS | C3 | Documentation |
| **R5** | Fix broken ADR cross-references in ADR-050 (wrong filenames) and review report (ADR-048 → ADR-050) | XS | Documentation quality | Documentation |

### Important (2-8 weeks)

| # | Action | Effort | Addresses |
|---|--------|--------|-----------|
| **R6** | Extend expression boundary scanner to include binary/unary operators — or restructure to try full expression parse first, fall back to Variable/Literal | M | Parser truncation (C1 in Compiler review) |
| **R7** | Add `TokenType::FontWeight` to numeric-accepting types in `check_token_type_compatibility()` (1-line fix) | XS | FontWeight type gap |
| **R8** | Apply `sanitize_css_value()` to theme `[data-theme]` blocks and pseudo-state properties (3-line fix) | XS | Security consistency |
| **R9** | Add const-kind validation to theme value Variable references (mirror primary value check) | S | Theme value validation asymmetry |
| **R10** | Upgrade `infer_token_expr_type()` to handle chained FieldAccess (recursive resolution to root VarRef) | S | P3-3 type inference gap |
| **R11** | Add WARNING diagnostic for `SymbolKind::Unknown` in TokenValue::Variable references | XS | Cross-file silent degradation |
| **R12** | Emit dynamic token effects into `component_js` (or a replaceable slot) instead of `gen.js` for HMR compatibility | M | HMR hot-replace |
| **R13** | Fix `error_offset` duplicate assignment in `parse_design_value_expression()` (shadowed dead code) | XS | Code quality |
| **R14** | Add `#[non_exhaustive]` to `TokenValueIR` enum (crate boundary defense) | XS | Forward compatibility |
| **R15** | Implement static `:root` CSS emission for React backend (portable, no runtime needed) | M | Cross-backend gap (partial) |

### Nice-to-Have (2-6 months)

| # | Action | Effort | Addresses |
|---|--------|--------|-----------|
| **R16** | Thread source spans through `TokenValue` variants for accurate design token diagnostics (M1) | M | Zero-span diagnostics |
| **R17** | Add `cargo feature = "dynamic-tokens"` to omnilang-core for size-constrained builds | S | Feature gating |
| **R18** | Extract class/variant parsing from `parser/design.rs` (1176 → ~700 lines) | M | File size guideline |
| **R19** | Add `Deserialize` to `Expr` to enable AST-level JSON round-trip for design blocks with expressions | L | Editor round-trip |
| **R20** | Add `suggest_similar_name()` to token variable undefined diagnostic (3-line fix) | XS | AI self-correction |
| **R21** | Consider deprecating `is_css_value()` heuristic in favor of quote-all-literals rule | M | Heuristic fragility |
| **R22** | Add per-element effect cleanup in `__omni_cleanupElement` for `@for` re-render lifecycle | M | Effect leak in dynamic lists |

---

## Risks if Unaddressed

| Risk | Source | Impact | Timeframe |
|------|--------|--------|-----------|
| Editor hot-reload memory exhaustion with dynamic design tokens | C1 | 50+ hot-reloads with 5 tokens = 250+ stale effect closures, browser tab slowdown | Immediate (editor usage) |
| User compiles design blocks to React/Flutter, gets no CSS | C2 | Hours of debugging, eroded trust in multi-target compiler | Immediate (any non-web user) |
| New user cannot discover dynamic token syntax | C3 | Feature may as well not exist; continued hex code duplication | Immediate (all new users) |
| Binary operator expressions silently degrade to `var(--name)` | Parser scanner | Same class of silent degradation as original H1 gap | When expressions grow complex |
| Stale dynamic token effects produce CSS `"undefined"` after signal rename | HMR lifecycle | Visual corruption during development, only full page reload recovers | Development workflow |
| Theme value non-const symbols pass validation then produce dangling CSS | Validation asymmetry | Same silent degradation as original H3 gap | When user adds theme to non-const token |
| Chained store access bypasses type checking | Type inference depth | Wrong CSS values at runtime with no diagnostic | When nested store structures used |

---

## Deferred Items (Explicitly Deferred)

| # | Item | Effort | Rationale |
|---|------|--------|-----------|
| E1 | `infer_token_expr_type()` chained FieldAccess resolution | S | Upgrade P3-3 LOW→MEDIUM. Recursive FieldAccess needed for `Store.nested.field`. S-effort fix when demand materializes |
| E2 | Per-element effect cleanup in `@for` re-renders | M | Requires `__omni_cleanupElement` extension + effect registry per-nid. Performance impact only at scale (50+ dynamic elements in @for loops) |
| E3 | Poiesis `DesignToken` type dynamic expression support | M | Requires discriminated union on DesignToken.value + emitter changes. Editor integration is not on critical path for compiler correctness |
| E4 | Feature flag `dynamic-tokens` for omnilang-core | S | Adds build-time gate for size-constrained builds. Deferred until cross-backend design support exists |

---

## Individual Member Reviews

The following sections preserve each specialist's verbatim review as source data. The aggregation above summarizes but does not replace these per-member analyses.

### Systems Architect Review

**Perspective**: Global OmniView framework integration — how dynamic tokens interact with multiple codegen backends, CSS cascade priority, signal reactivity lifecycle, and the architectural boundary between OmniView-specific and framework-agnostic features.

**Key Findings**:
- CRITICAL: Dynamic token `__omni_effect` closures accumulate across editor hot-reload cycles with no dispose path — memory leak grows linearly with hot-reload count
- HIGH: Theme mode sync is unidirectional (`store.mode → __omni_theme` only); `__omni_theme.toggle()` permanently desyncs store from CSS
- HIGH: Design blocks produce zero CSS output on all 9 non-OmniView backends with no diagnostic
- MEDIUM: Per-token `__omni_effect` creates O(N) subscriptions (mitigated by ADR-050 consolidation)
- MEDIUM: Expression boundary scanner doesn't track `{...}` depth for object literal arguments
- MEDIUM: Dynamic tokens don't participate in `@media (prefers-color-scheme)` auto-detection
- LOW: `__omni_theme.init()` runs before store signals are defined, creating transient race window
- LOW: Rust `sanitize_css_value()` returns un-normalized CSS hex escapes

**Key Strengths**:
- Clean IR-level static/dynamic split enables correct-by-construction codegen
- Pre-collection pattern correctly manages Rust borrow checker boundaries
- Consistent sanitization across compile-time and runtime CSS paths
- Theme coercion works correctly for common store naming conventions
- Well-documented design decisions with ADR trail

### Compiler & Language Architect Review

**Perspective**: Compiler pipeline correctness and language design soundness — parser architecture, AST/IR type design, semantic validation coverage, and multi-target codegen strategy.

**Key Findings**:
- HIGH: Expression boundary scanner truncates expressions with binary operators (`store.getValue() + someVar` → partial parse → silent Variable fallback)
- MEDIUM: All design token semantic diagnostics use zero spans (line 1, col 1 regardless of actual error location)
- MEDIUM: Type mismatch diagnostics are fire-and-forget (no compile-time rejection — the IR still carries the expression)
- MEDIUM: React/Flutter/Dioxus codegen targets have zero design token support
- LOW: TokenType information is not propagated into `TokenValueIR::Dynamic`

**Key Strengths**:
- Clean IR separation of static vs dynamic
- Pragmatic validation gap closure (ADR-050)
- Theme restriction uniformity (symmetric main value + theme_values restriction)
- Store field access type resolution in `infer_token_expr_type`
- Consolidated `__omni_effect` generation

### Security Specialist Review

**Perspective**: Security implications of dynamic CSS custom property injection — CSS injection vectors, sanitization coverage, expression evaluation boundaries, and signal lifecycle safety.

**Key Findings**:
- MEDIUM: `sanitize_css_value()` missing from theme `[data-theme]` blocks (design.rs:94-95) and `@media` blocks (design.rs:109) — only `:root` path was fixed in ADR-050
- MEDIUM: `sanitize_css_value()` missing from pseudo-state property values (design.rs:152) — base class properties are sanitized but pseudo-states are not
- LOW: Effect cleanup return values never captured — resource leak for element-level effects
- LOW: CSS `url()` inside `data:` URI scheme not blocked (both Rust and JS sanitizers)

**Key Strengths**:
- Defense-in-depth architecture (3 layers: compile-time, runtime, CSP)
- Near-perfect CSS hex escape normalization parity between Rust and JS sanitizers
- CR/LF/TAB stripping in both sanitizers
- `String()` coercion prevents `toString()` bypass
- `document.documentElement` null guard for SSR/Worker safety
- Consistent handling across classes, variant bases, and variant branches

**Assessment**: Zero CRITICAL or HIGH security vulnerabilities. The MEDIUM findings are defense-in-depth inconsistencies — static `:root` values are sanitized but theme and pseudo-state paths are not. All values originate from developer source code, not user input. Fixes are single-line additions.

### Performance Specialist Review

**Perspective**: Hot-path costs, signal subscription granularity, browser CSS custom property invalidation scope, and runtime allocation patterns.

**Key Findings**:
- MEDIUM: `__omni_sanitizeCss` runtime overhead is redundant for internal store signals (full sanitization pipeline runs on every signal change even for compile-time-known-safe values)
- MEDIUM: Per-element inline style effects don't clean up on `@for` re-render — disconnected element effects persist and re-run on every signal change
- LOW-MEDIUM: Consolidated `__omni_effect` causes unnecessary `setProperty` calls for unchanged tokens (browser no-op but triggers style invalidation path)
- LOW: Signal setter traversal of all subscriptions on every change
- LOW: `sanitizeCss` string allocations (acceptable at current scale)
- INFO: CSS custom property invalidation on `<html>` triggers main-thread style recalc for all `var()` consumers

**Key Strengths**:
- Static path is genuinely zero-cost (Phase 1 resolves at compile time, pure CSS output)
- Single effect consolidation reduces subscriptions from O(N) to O(1)
- `document.documentElement` cached per-effect (not re-queried in hot path)
- Null guard prevents errors in SSR/Worker contexts at zero cost
- Pre-collection is one-time per module (not in hot path)

### Rust Expert Review

**Perspective**: Rust code quality, safety, ownership patterns, error handling, and idiomatic conventions across the 5 Rust modules implementing dynamic tokens.

**Key Findings**:
- MEDIUM: Duplicated dead-code assignment of `error_offset` in `parse_design_value_expression()` (lines 356-360 — two identical assignments)
- MEDIUM: Verbose `with_capacity_and_hasher(_, Default::default())` in IR lowering
- LOW: `.iter().cloned().collect()` should be `.to_vec()` (clippy)
- LOW: `.get(key).is_none()` should be `!contains_key(key)` (clippy)
- LOW: Missing `#[non_exhaustive]` on `TokenValueIR` (crate boundary defense)
- LOW: `err()` helper always reports `offset: 0`, degrading diagnostic precision
- LOW: `name: String::new()` in returned `DesignDef` is always overwritten (wasted allocation)

**Key Strengths**:
- No `unsafe` code anywhere in the feature
- No production `unwrap()` or `expect()` — all fallible operations use `?` or explicit `Result`
- Manual `Deserialize` for `TokenValue` is well-motivated and correctly implemented
- Manual `PartialEq` is correctly scoped with documented rationale
- Error handling in dual-mode parser is comprehensive (`DesignParseError` with offset)
- `emit_dynamic_token_effects` is focused, efficient, and well-structured
- `FxHashMap` usage is appropriate for compiler-internal structures
- `#[serde(default)]` on optional fields ensures backward-compatible deserialization

**Assessment**: No CRITICAL or HIGH issues. The code is well-structured, uses safe Rust exclusively, has proper error handling, and follows idiomatic patterns.

### TypeScript & React Specialist Review

**Perspective**: Frontend runtime behavior and integration — generated JS quality, React/SSR compatibility, signal reactivity lifecycle, HMR/debug window, and Poiesis editor integration.

**Key Findings**:
- HIGH: Dynamic token effects lost during HMR component hot-replace — effects are in `gen.js` (not `component_js`), and HMR diff only checks component hashes
- HIGH: Stale dynamic token effects produce visible CSS corruption after signal rename/removal — `String(undefined)` passes sanitization, produces literal `"undefined"` as CSS value
- HIGH: Orphaned subscriptions on repeated re-registration (same root cause as above)
- MEDIUM: Inconsistent try/catch in per-element style effects vs attribute effects (style effects lack specific error context)
- MEDIUM: Poiesis `DesignToken` type has no dynamic expression representation — editor is static-only
- MEDIUM: Per-element inline style creates N separate `__omni_effect` subscriptions per shared signal
- MEDIUM: No `requestAnimationFrame` coordination for `setProperty` calls (acknowledged in ADR-037)

**Key Strengths**:
- Clean, debuggable generated JS with proper indentation and null guard
- Consistent sanitization through all code paths
- Theme vs. dynamic token separation is clean (CSS cascade resolves predictably)
- Good test coverage for dynamic token codegen
- Error resilience — `__omni_effect` wraps callbacks in try/catch

### Maintainability Expert Review

**Perspective**: Long-term maintainability, technical debt, code organization, documentation quality, and module structure.

**Key Findings**:
- MEDIUM: `is_css_value()` heuristic is fragile with no structural mitigation — lowercase const names silently treated as CSS literals
- MEDIUM: Manual `PartialEq` and `Deserialize` impls for `TokenValue::Expression` are maintenance traps — if `Expr` gains `Deserialize`, manual impls become silently unnecessary
- MEDIUM: `SymbolKind::Unknown` exemption creates a blind spot in const-kind validation
- MEDIUM: Zero source spans on all design token diagnostics (5 diagnostic sites)
- MEDIUM: `parser/design.rs` is 1176 lines and growing (exceeds 800-line guideline)
- MEDIUM: Design doc in Chinese is drifting from implementation (stale function names)
- MEDIUM: `TokenValue` `PartialEq` silently treats Expression as always-unequal
- MEDIUM: Pre-collection borrow checker workaround is fragile if generator structure changes
- LOW: CSS keyword list is unsorted (80 entries, no alphabetical ordering)
- LOW: `TokenType` and `TokenTypeIR` enums duplicate 13 variants without shared definition

**Key Strengths**:
- Clear Phase 1/2 separation in the type system
- Pre-collection pattern in codegen is well-commented
- ADR cross-referencing is strong
- Dead code was actually cleaned up (resolve_token_value, lower_design)
- Exhaustive match in lowering provides compile-time drift protection between TokenType and TokenTypeIR

### Domain Expert Review

**Perspective**: Domain model correctness and semantic integrity — type system coverage, expression classification, theme value semantics, and cross-backend domain boundaries.

**Key Findings**:
- HIGH: `TokenType::FontWeight` missing from numeric-accepting types in `check_token_type_compatibility()` — CSS font-weight accepts numeric 100-900
- HIGH: Binary operators not recognized as expressions in token values — `BASE + "px"` parsed as Variable("BASE"), remainder causes parse error
- MEDIUM: Theme value Variable references only check symbol existence, not const-kind — asymmetry with primary value validation
- MEDIUM: `var(--name)` fallback silently produces dangling CSS references — conflates "intentional CSS var ref" with "resolution failure"
- MEDIUM: Dynamic token codegen is OmniView-only — IR implies cross-backend but implementation doesn't deliver
- LOW: Type mismatch diagnostic always says "expects a string value" even for numeric-accepting types
- LOW: `is_css_value()` heuristic conflates case convention with declaration scope

**Key Strengths**:
- Clean pipeline separation (AST → IR → Codegen) with clear domain boundaries
- Ubiquitous language consistency — TokenType mirrors TokenTypeIR perfectly
- Thoughtful theme system integration (3 cascade layers + media query auto-detection)
- Cross-file constants thoroughly tested (`cross_file_const_test.rs`)
- Expression-vs-variable boundary carefully ordered (expression check before CSS-value heuristic)

### Implementation Strategist Review

**Perspective**: Implementation sequencing, change risk, reversibility, deferred item risk assessment, and cross-backend readiness.

**Key Findings**:
- CRITICAL: Design token codegen is web-only — other backends silently produce zero design CSS
- HIGH: `#[serde(skip)]` on `TokenValue::Expression` breaks AST-level JSON round-trip for design blocks with dynamic tokens (IR-level round-trip works)
- MEDIUM: No feature flag gating — dynamic token support is always compiled (~1270 lines)
- MEDIUM: `SymbolKind::Unknown` exemption leaves a cross-file const-kind validation gap
- MEDIUM: `infer_token_expr_type()` depth limit bypasses type checking for chained store access
- MEDIUM: No feature flags for dynamic token effects — SSR risk (documented design choice)

**Deferred Items Re-Assessment**:
- P3-3 (`infer_token_expr_type` one-level depth): **Upgraded LOW → MEDIUM** — active validation gap
- New: Cross-backend design codegen gap: **CRITICAL** — silent zero-output on 9 targets
- New: No feature flag for dynamic tokens: **MEDIUM** — always-compiled, no build-time gate
- New: `#[serde(skip)]` on Expression variant: **Upgraded LOW → MEDIUM** — editor impact uncertain but real
- New: Unknown exemption cross-file silent degradation: **MEDIUM** — same class as original H3 gap

**Key Strengths**:
- Phase 1→Phase 2 sequencing was correct (additive from the start)
- Phase 1 constraints did NOT negatively influence Phase 2 design
- 13-commit fix sequence was correctly ordered by risk priority
- High reversibility for both phases (collapse enums back to String)
- Review fix discipline — all 12 fixes were additive diagnostics or dead code removal

### AI Engineer Review

**Perspective**: AI-assisted development and developer experience — AI model code generation accuracy, error message actionability, test quality as few-shot examples, and documentation discoverability for AI tools.

**Key Findings**:
- MEDIUM: `is_css_value()` heuristic creates a naming trap for AI-generated code — AI naturally generates lowercase const names which silently become CSS literals
- MEDIUM: `SymbolKind::Unknown` exemption creates silent degradation path for cross-file AI-generated imports
- MEDIUM: Zero source spans harm AI self-correction — no line/column info in diagnostics
- LOW: Expression parse failure silently falls back to Variable — masks AI errors with no feedback
- LOW: Missing `suggest_similar_name()` in token E0001 — unlike regular VarRef resolution
- LOW: Documentation fragmentation increases AI context acquisition cost (5 documents needed)
- LOW: `currentColor` CSS_KEYWORDS membership — only mixed-case CSS keyword, can't be used as const name
- LOW: Non-string const Phase 1 limitation undocumented — AI may generate `const RADIUS = 8`

**Key Strengths**:
- AI-ready error messages — E0001 const-kind diagnostic provides exact fix syntax with template placeholders
- Theme+dynamic restriction diagnostic equally actionable (explains why + suggests workaround)
- Pattern consistency reduces AI learning cost (import syntax, store.field access, __omni_effect all follow established patterns)
- Tests as AI few-shot examples are excellent — descriptive names, positive and negative cases, clear patterns

### Pragmatic Enforcer Review

**Perspective**: YAGNI analysis — necessity vs complexity assessment for both phases, deferred features, and forward-looking complexity.

**Scoring**:

| Feature | Necessity | Complexity | Ratio | Verdict |
|---------|-----------|------------|-------|---------|
| Phase 1 (compile-time const refs) | 4/10 | 4/10 | 1.0 | Marginally justified; simpler alternative exists |
| Phase 2 (runtime signal refs) | 2/10 | 6/10 | **3.0** | Premature — zero non-test consumers |
| ADR-050 validation fixes | 8/10 | 3/10 | 0.38 | Necessary and well-executed |
| **Overall** | 5/10 | 6/10 | 1.2 | Both phases implemented; Phase 2 should have been deferred |

**Key Findings**:
- Phase 2 has **zero non-test consumers** in learn-omniview — `style: { --var: signal }` already achieves the same result
- `is_css_value()` heuristic is the biggest YAGNI violation — 100 lines avoidable by requiring quoted CSS values
- `check_token_type_compatibility()` is borderline — 65 lines catching a theorized error class (string signal in size token)
- ADR-050 effect consolidation was speculative optimization at N=0-5 scale
- `sanitize_css_value()` for static `:root` values is defense-in-depth for developer-authored code (not user input)
- Phase 3 (Flutter dynamic tokens, animation, SSR) should NOT be built until Phase 2 shows meaningful production usage

**Recommendations**:
- Keep Phase 1 — functionally sound, achieves DRY with zero runtime overhead
- Document Phase 2 zero consumers clearly — guides future prioritization
- Don't build Phase 3 without production usage data
- Consider deprecating `is_css_value()` heuristic in favor of quote-all-literals in a future language version
- Don't add more micro-optimizations without profiling evidence

### Documentation Expert Review

**Perspective**: Documentation quality and developer onboarding — user-facing docs, ADR quality, code comments, grammar accuracy, tutorial completeness, and cross-reference discoverability.

**Key Findings**:
- CRITICAL: User-facing documentation has zero mention of dynamic tokens — feature is invisible to end users
- HIGH: Grammar BNF in `15-grammar-view.md` is outdated and inaccurate for token definitions (missing type annotation, semicolon, uses old syntax)
- HIGH: Grammar doc's `const` description is obsolete ("parsed but discarded" — no longer true since Phase 1)
- MEDIUM: `is_css_value()` heuristic undocumented in user-facing docs — most important user-facing behavior is invisible
- MEDIUM: ADR cross-references broken in ADR-050 (wrong filenames for ADR-047 and ADR-049)
- MEDIUM: Review report references wrong ADR (ADR-048 instead of ADR-050)
- MEDIUM: No cross-references from tests or user-facing docs to design docs
- MEDIUM: All design token diagnostics use zero source spans
- MEDIUM: No migration guide or tutorial for dynamic tokens
- LOW: Design doc has minor inaccuracies (stale function names, internal inconsistencies)
- LOW: Only one code comment links back to ADRs (parser/design.rs:383 references ADR-047)

**Key Strengths**:
- ADR series quality — coherent three-part narrative with consistent Pragmatic Enforcer scoring
- Code comments on complex logic are excellent (`is_css_value()` documentation at parser/design.rs:374-385)
- Public type doc comments are clear with concrete code examples
- Error messages are actionable with fix suggestions
- Review report structure is well-organized with cross-cutting themes

---

## Files of Interest

| File | Role | Lines |
|------|------|-------|
| `crates/omnilang-core/src/parser/design.rs` | Dual-mode parser: string body parser + token-stream expression bridge | 1176 |
| `crates/omnilang-core/src/parser/ast/design.rs` | AST types: TokenValue, TokenDef, TokenType, manual trait impls | 209 |
| `crates/omnilang-core/src/semantic/resolve.rs` | Semantic validation: const-kind checks, type compatibility, theme restrictions | ~400 (token section) |
| `crates/omnilang-core/src/ir/design.rs` | IR types: TokenValueIR, TokenIR, TokenTypeIR | 129 |
| `crates/omnilang-core/src/ir/lower/design.rs` | AST→IR lowering: const resolution, expression lowering | ~200 |
| `omnilang/src/codegen/omniview/design.rs` | CSS codegen: :root emission, theme blocks, dynamic effect JS | 306 |
| `omnilang/src/codegen/omniview/mod.rs` | Codegen orchestration: effect collection, theme runtime generation | ~750 |
| `omnilang/src/codegen/mod.rs` | Backend dispatch — design blocks silently ignored by 9/10 backends | ~120 |
| `docs/designs/dynamic-token-variables.md` | Original design document (Chinese, partially stale) | 289 |
| `.architecture/decisions/adrs/ADR-047*.md` | Phase 1 decisions | ~400 |
| `.architecture/decisions/adrs/ADR-049*.md` | Phase 2 decisions | ~200 |
| `.architecture/decisions/adrs/ADR-050*.md` | Validation gap closure | ~250 |
| `docs/omniview/10-design-system.md` | User-facing design system docs (no dynamic token content) | ~400 |
| `docs/omniview/14-grammar-lang.md` | Grammar reference (obsolete const description) | ~800 |
| `docs/omniview/15-grammar-view.md` | View grammar BNF (wrong token_def) | ~700 |

---

## Success Metrics

- [ ] Zero hot-reload effect accumulation after 50+ consecutive edits
- [ ] Compile-time WARNING emitted when design blocks compiled to non-OmniView targets
- [ ] User-facing docs show complete dynamic token syntax with naming convention
- [ ] Grammar BNF matches actual parser behavior for token definitions
- [ ] All 3 CRITICAL findings resolved
- [ ] All 6 HIGH findings resolved or explicitly deferred with tracking issue
- [ ] Test count maintained or increased (552+ tests)
- [ ] No regression in learn-omniview project

## Follow-up Plan

1. **This session**: Create ADR-051 documenting the global review findings and prioritization decisions
2. **This sprint**: Implement CRITICAL fixes (R1-R5) — estimated 2-4 hours total
3. **Next sprint**: Implement HIGH/important fixes (R6-R15) — estimated 1-2 days
4. **Defer**: Nice-to-have items (R16-R22) — schedule based on user demand and team capacity
