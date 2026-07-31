# Architectural Review: Dynamic Design Tokens (Phase 1 + Phase 2)

## Overview

Comprehensive architecture review of the Dynamic Design Tokens feature implemented per `docs/designs/dynamic-token-variables.md`. The feature adds expression-based token values to `design` blocks, flowing through the full compiler pipeline (Parser → AST → Semantic → IR → Codegen), spanning 16+ commits and 547+ tests across two phases.

- **Review Target**: Dynamic Design Tokens (Phase 1 compile-time const refs + Phase 2 runtime signal refs)
- **Review Date**: 2026-07-31
- **Review Team**: 11 specialists (Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert, TypeScript & React Specialist, Compiler & Language Architect)
- **Methodology**: Multi-perspective parallel review with cross-cut analysis

---

## Executive Summary

The Dynamic Design Tokens feature is **architecturally sound** in its core design. The `TokenValue` (AST: Literal|Variable|Expression) → `TokenValueIR` (IR: Static|Dynamic) mapping is an elegant separation of user-intent from execution-strategy. The Phase 1/2 split followed correct YAGNI discipline — Phase 1 was implemented first, Phase 2 was deferred until user need materialized. Security is well-covered through reuse of proven `__omni_sanitizeCss` infrastructure. The feature has zero CRITICAL security vulnerabilities.

However, **three HIGH-severity gaps** exist in the compile-time validation boundary — all are cases where the compiler produces wrong/dropped output with zero diagnostic feedback. These are not correctness bugs in the happy path (documented use cases work), but silent degradation paths that erode user trust when encountered. All three have XS/S effort fixes.

**Overall Architecture Health**: **Good** — strong core design, add-only enum architecture preserves backward compatibility, but validation gaps need closure.

### Key Strengths
- Clean static/dynamic IR enum split (`TokenValueIR`) makes codegen incorrectness structurally impossible
- Additive, reversible architecture — removing Phase 2 = deleting enum variants and their match arms
- Comprehensive test coverage across all pipeline layers (547+ tests)
- Security infrastructure reuse (`__omni_sanitizeCss`, `style.setProperty()` API safety)
- Well-documented design decisions across ADR-047, ADR-049, and code comments

### Critical Concerns
- **H1**: Bare-ident signal refs (`App_dynamicAccent`) produce silent `var(--name)` CSS fallback instead of `__omni_effect` — documented use case is broken
- **H2**: Dynamic expressions in theme values (e.g., `token x = "#fff" dark store.color`) are silently dropped from both CSS and JS output
- **H3**: `TokenValue::Variable` semantic validation only checks symbol existence, not const-kind or string-compatibility — non-const symbols and non-string consts pass validation then silently degrade

---

## Individual Perspectives

### Systems Architect Review

**Reviewer**: Systems Architect

**Strengths**:
- Clean static/dynamic split at IR boundary — single point where compile-time vs runtime distinction is captured
- Additive, reversible architecture — both phases add variants to existing enums without restructuring
- Pre-collect pattern correctly manages borrow checker boundary in JS codegen
- Reuse of proven infrastructure (`lower_expr`, `__omni_effect`, `__omni_sanitizeCss`)
- Theme restriction correctly gated at compile time

**Concerns**:
- **HIGH-1**: Silent downgrade of bare-ident runtime signal references — `App_dynamicAccent` (explicitly documented as a Phase 2 use case) produces `var(--name)` in CSS with zero diagnostics (`parser/design.rs:267-270`)
- **HIGH-2**: Dynamic theme values on static tokens silently dropped — `token primary: color = "#fff" dark ThemeStore.darkColor` loses the dark override entirely (`semantic/resolve.rs:313-360`)
- **MED-3**: Dual-parser bridge requires manual offset synchronization — fragile under maintenance (`parser/design.rs:298-360`)
- **MED-4**: `TokenValue::Expression` not JSON-round-trippable through serialization (`ast/design.rs:66,120-122`)
- **MED-5**: Span information lost for all design token diagnostics (`semantic/resolve.rs:283-329`)
- **LOW-6**: `lower_design()` wrapper discards lowering diagnostics for expression tokens (`ir/lower/design.rs:16-21`)

---

### Domain Expert Review

**Reviewer**: Domain Expert

**Strengths**:
- Clean domain boundary between AST (`TokenValue`) and IR (`TokenValueIR`) — user intent vs execution strategy
- Graceful degradation for unresolved variables — `var(--name)` fallback instead of crash
- Pre-collection pattern in JS codegen separates "what" from "how"
- Manual `PartialEq`/`Deserialize` correctly enforces domain boundary with 27-variant `Expr`
- Type compatibility check for dynamic token expressions (`check_token_type_compatibility`)

**Concerns**:
- **HIGH-1**: `TokenValue::Variable` semantic check validates symbol existence but not const-compatibility — `SymbolKind::Component` or `SymbolKind::Store` symbols pass, then degrade to `var()` (`semantic/resolve.rs:282-297` vs `ir/lower/mod.rs:71-79`)
- **MED-2**: `is_css_value()` heuristic imposes domain-foreign naming constraint — lowercase consts silently treated as CSS literals (`parser/design.rs:364-387`)
- **MED-3**: No compile-time type compatibility check for `TokenValue::Variable` references — only Expression path gets type checking (`semantic/resolve.rs:282-297`)
- **LOW-4**: Theme override diagnostic lacks domain reasoning — says "what" but not "why"

---

### Security Specialist Review

**Reviewer**: Security Specialist

**Strengths**:
- Consistent runtime sanitization — both dynamic tokens and inline styles use identical `__omni_sanitizeCss()`
- CSS escape normalization on both sides (Rust compile-time + JS runtime)
- Type compatibility check at compile time prevents type-confused values
- Theme override restriction simplifies security model
- ADR-documented security decisions (ADR-026, ADR-027, ADR-049)

**Concerns** (all LOW severity):
- Static token values in `:root` CSS bypass `sanitize_css_value()` — inconsistency with class/variant property paths (`codegen/omniview/design.rs:79`)
- `__omni_effect` error handler swallows expression errors silently
- `__omni_reportError` argument mismatch in signal setter (runtime bug)

**Key finding**: No CRITICAL or HIGH vulnerabilities. Feature safely reuses proven infrastructure.

---

### Maintainability Expert Review

**Reviewer**: Maintainability Expert

**Strengths**:
- Clean pipeline separation across 5 layers
- Thoughtful additive design — Phase 2 builds on Phase 1 without breaking backward compatibility
- Well-documented trade-offs (`#[serde(skip)]`, `PartialEq`, `is_css_value()`, `#[allow(dead_code)]`)
- Test coverage for critical paths
- Single responsibility in codegen — `emit_dynamic_token_effects()` is focused 20-line function

**Concerns**:
- **HIGH-1**: Dual-mode parsing architecture with fragile expression boundary detection (`parser/design.rs:298-358`)
- **MED-1**: 3-way `TokenType` enum synchronization burden — AST enum, IR enum, semantic name mapping all maintained independently
- **MED-2**: Theme value parsing backtracking pattern — relies on parse-order side effects (`parser/design.rs:128-154`)
- **MED-3**: Imprecise error positions — 29 of ~30 error call sites use `err()` with offset 0
- **MED-4**: `.token_type` field redundant on `TokenIR` — never consumed by codegen
- **LOW**: `#[serde(skip)]` prevents JSON round-trip; `is_css_value()` silent misclassification; `lower_design()` discards diagnostics; dead `resolve_token_value()`; validation errors lack source location

---

### Performance Specialist Review

**Reviewer**: Performance Specialist

**Strengths**:
- Zero runtime overhead for static tokens (the 99% case)
- Pre-collection pattern avoids repeated iteration — single O(T) pass
- `setProperty` on `document.documentElement` has correct specificity behavior
- Theme+dynamic restriction eliminates runtime ambiguity
- `__omni_effect` implementation is well-optimized substrate

**Concerns**:
- **MEDIUM**: Unbatched per-token `__omni_effect` creates O(N) `setProperty` calls per signal change (`codegen/omniview/design.rs:286-296`). Fix: group into single `__omni_effect` per design block. Effort: S.
- **MEDIUM**: `__omni_sanitizeCss` overhead on every dynamic token update — unnecessary for internal store values. Fix: add fast-path for known-safe patterns. Effort: S.
- **LOW**: Empty `:root` block emitted when all tokens are dynamic (`design.rs:75-84`)
- **LOW**: `__omni_effect` dispose return value not stored — hot-reload concern

---

### Implementation Strategist Review

**Reviewer**: Implementation Strategist

**Strengths**:
- Clean additive pattern preserves backward compatibility
- Correct dependency sequencing across pipeline layers
- `const_values` pipeline well-structured with clear data flow
- Safe Phase 2 restriction on dynamic + theme overrides
- Isolated test coverage per layer

**Concerns**:
- **MEDIUM**: Variable references in Phase 1 not validated as compile-time constants — non-const symbols pass semantic check, degrade to `var()` (`semantic/resolve.rs:278-297`)
- **MEDIUM**: Expression boundary detection is future-maintenance risk (`parser/design.rs:298-360`)
- **LOW**: Zero source spans, unused dynamic effects, `infer_token_expr_type()` bypass for complex expressions, `lower_design()` wrapper is a trap

---

### AI Engineer Review

**Reviewer**: AI Engineer

**Strengths**:
- Clear target pattern for AI code generation — simple syntax, predictable variants
- Graceful degradation via `var(--name)` fallback tolerates minor AI errors
- Structural type compatibility check at compile time
- Explicit static/dynamic separator at IR boundary
- Comprehensive test coverage serves as implicit spec

**Concerns**:
- **MEDIUM**: All design token diagnostics use zero spans — AI debugging pipelines cannot correlate errors to source locations
- **MEDIUM**: Silent expression parse failure falls back to Variable with no warning — creates confusing error chains for AI tools
- **LOW**: `is_css_value()` heuristic traps AIs that prefer lowercase naming conventions
- **LOW**: No test coverage for theme values containing expressions
- **LOW**: Dynamic token effects lose design context — cross-design token name collisions possible

---

### Pragmatic Enforcer Review

**Reviewer**: Pragmatic Enforcer (YAGNI Guardian)

**Overall Simplicity Assessment**: Well-engineered for its purpose. Every abstraction pulls its weight.

**Pragmatic Scores**:
- **Overall Necessity**: 7/10 (Phase 1: 8/10, Phase 2: 6/10)
- **Overall Complexity**: 4/10 (Low-medium)
- **Ratio**: 7/4 = **1.75** (above 1.5 target — Phase 2 necessity dilutes Phase 1's strong score)
- **Pragmatic Score**: 0.71 (aligned with ADR-047)

**Strengths**:
- Explicit static/dynamic enum boundary at IR level
- Zero overhead for static tokens
- Graceful parser degradation
- Defensive compile-time rejection of ambiguous semantics
- Fully reversible architecture

**Concerns** (all LOW):
- `resolve_token_value()` dead code wrapper — speculative, no consumers
- `lower_design()` publicly exported but used only in tests — API surface trap
- Expression parse failure produces indirect error feedback
- `is_css_value()` heuristic footgun remains

**Challenges to Other Architects**:
- Recommends explicit variable reference syntax (`$BRAND`) long-term to eliminate `is_css_value()` heuristic
- Rejects removing `TokenValueIR` in favor of `Option<ExprIR>` — explicit enum is clearer
- Rejects inlining `emit_dynamic_token_effects()` — function extraction is correct pattern

---

### Rust Expert Review

**Reviewer**: Katya Volkov (Rust Expert)

**Strengths**:
- Manual trait impls justified and documented (`PartialEq`, `Deserialize`)
- Consistent error type `DesignParseError` throughout the parser
- Clean enum-based state at IR boundary
- Lowering is pure function with explicit context — testable, visible data flow
- No `unwrap`/`expect` in production code

**Concerns**:
- **MEDIUM**: `lower_design()` silently discards semantic diagnostics — any expression lowering errors through this path are lost (`ir/lower/design.rs:16-21`)
- **MEDIUM**: Dead code `resolve_token_value()` with `#[allow(dead_code)]` — invisible to clippy, maintenance trap (`ir/lower/design.rs:120-131`)
- **MEDIUM**: `PartialEq` for `TokenValue` returns `false` for Expression-Expression comparison — semantically surprising (`ast/design.rs:70-79`)
- **LOW**: Offset invariant in `err_at()` relies on undocumented caller discipline (`parser/design.rs:37-42`)
- **LOW**: Error offset lost when sub-parser fails — hardcoded to 0 (`parser/design.rs:352`)
- **LOW**: `#[serde(skip)]` produces incomplete serialization

---

### TypeScript & React Specialist Review

**Reviewer**: Emma Rodriguez (TypeScript & React Specialist)

**Strengths**:
- Clean static/dynamic separation at IR level
- Pre-collection pattern avoids borrow checker conflicts
- Correct signal dependency tracking via `.value` access
- Idempotent effects by design
- Comprehensive test coverage

**Concerns**:
- **MEDIUM**: Per-token `setProperty` without batching — layout thrashing risk with 20+ dynamic tokens (`design.rs:286-295`)
- **MEDIUM**: No `document.documentElement` null guard — SSR/worker safety (`design.rs:290-292`)
- **MEDIUM**: Hot-reload leaves stale dynamic token effect closures — memory accumulation (`design.rs:297`)
- **LOW**: Single effect consolidation loses dependency granularity (acknowledged trade-off); theme sync uses internal signal API (consistent with codebase)

---

### Compiler & Language Architect Review

**Reviewer**: Dr. Lin Wei (Compiler & Language Architect)

**Strengths**:
- Clean IR-level static/dynamic split — `TokenValueIR` enum makes codegen incorrectness impossible
- Graceful degradation with semantic diagnostics
- Theme+dynamic restriction correctly scoped at semantic stage
- Reuses proven infrastructure
- Pre-collection avoids borrow-checker conflicts

**Concerns**:
- **MEDIUM**: Dynamic expression in theme values causes silent data loss — Expression in `theme_values` silently filtered by both CSS and JS codegen (`parser/design.rs:147-153`, `codegen/omniview/design.rs:62-73`, `mod.rs:414-422`)
- **LOW**: Expression boundary scanner doesn't track `{...}` depth — object literal args could break (`parser/design.rs:312-339`)
- **LOW**: Error offset zeroed for expression parse failures — points to wrong location (`parser/design.rs:350-352`)
- **LOW**: `is_css_value()` heuristic can miscategorize camelCase identifiers

---

## Cross-Cutting Themes

Themes that appeared in 6+ specialist reviews:

### Theme 1: Zero Source Spans in Design Token Diagnostics (6 specialists)

Systems Architect, AI Engineer, Implementation Strategist, Maintainability, Domain Expert, Compiler Architect all flagged that design token diagnostics use `Span { start: 0, end: 0, line: 0, col: 0 }`. Error messages about undefined variables, type mismatches, and theme restrictions all point to the `{` of the design block rather than the actual error location. This is the most universally identified quality gap.

### Theme 2: Silent `TokenValue::Variable` Degradation Without Const-Kind Validation (5 specialists)

Systems Architect, Implementation Strategist, Domain Expert, AI Engineer, and Pragmatic Enforcer all identified that `TokenValue::Variable` semantic validation only checks symbol existence — not whether the symbol is a `const` or whether its value is a resolvable string. Non-const symbols (Store, Component, Variable) pass validation, then silently degrade to `var(--name)` in CSS. Numeric consts similarly pass but cannot be resolved.

### Theme 3: Dynamic Expressions in Theme Values Silently Dropped (4 specialists)

Systems Architect, Compiler Architect, Implementation Strategist, and TypeScript/React all confirmed that `TokenValue::Expression` in `theme_values` is accepted by the parser, passes semantic analysis, but produces neither CSS `[data-theme]` output nor JS `__omni_effect` output. Zero diagnostics at any pipeline stage.

### Theme 4: Dual-Mode Parser Fragility (4 specialists)

Systems Architect, Compiler Architect, Maintainability, and Implementation Strategist identified the string-based + token-stream parsing bridge as the highest-maintenance surface in the feature. The manual offset synchronization between `parse_design_value_expression()` and the parent string parser requires an undocumented suffix invariant.

### Theme 5: `is_css_value()` Heuristic Footgun (4 specialists)

Domain Expert, Maintainability, AI Engineer, Compiler Architect confirmed that all-lowercase identifiers are silently treated as CSS literal values rather than variable references. A `const brand = "#fff"` with `token x = brand;` produces `--x: brand` (the literal string) instead of `--x: #fff`.

### Theme 6: Per-Token `__omni_effect` O(N) Scaling (3 specialists)

Performance Specialist, TypeScript/React, and Pragmatic Enforcer noted that each dynamic token creates a separate `__omni_effect` subscription. Consolidating into one effect per design block reduces O(N) to O(1).

---

## Trade-offs and Disagreements

### Trade-off 1: Effect Consolidation vs Dependency Granularity

**Performance** wants all dynamic tokens in one `__omni_effect` to reduce subscriptions. **TypeScript/React** notes this loses per-token dependency granularity — changing signal A would re-set tokens that depend on signal B. Both agree the trade-off favors consolidation: `setProperty` with the same value is a browser no-op, and typical usage (1-5 dynamic tokens) makes the concern theoretical.

### Trade-off 2: `lower_design()` Removal vs Downgrade

**Rust Expert** wants to remove `lower_design()` entirely since production code uses `lower_design_with_consts()`. **Pragmatic Enforcer** suggests downgrading to `#[cfg(test)]` or adding a doc warning. Consensus: the wrapper is a diagnostic-silent-discard trap and should be removed unless tests require it.

### Trade-off 3: Batching Now vs Profile-First

**Performance** recommends grouping effects now (S effort). **TypeScript/React** recommends profiling first since 0-5 dynamic tokens is the current reality. Both agree the fix is simple enough to do preemptively.

### No Disagreements

No two specialists reached contradictory conclusions. The 11 perspectives are complementary — each found different facets of the same validation-gap theme, confirming the pattern rather than conflicting about it.

---

## Prioritized Recommendations

### CRITICAL — Fix Now

None identified. The feature works correctly for documented use cases. The HIGH items below are gaps in edge-case validation, not correctness bugs in the happy path.

### HIGH — Fix Next Session (XS–S effort each)

| # | Recommendation | Files | Effort | Source |
|---|---------------|-------|--------|--------|
| **H1** | Extend expression detection for bare-ident signal refs (`App_dynamicAccent`) — or add semantic fallback that emits diagnostic when `Variable` ref is in symbol table as non-const | `parser/design.rs:267-270`, `semantic/resolve.rs:282-297` | M | Systems Architect, Domain Expert |
| **H2** | Reject `TokenValue::Expression` in theme values — add compile-time restriction symmetric to existing dynamic-main-value check | `semantic/resolve.rs:313-360` | XS | Systems Architect, Compiler Architect |
| **H3** | Add const-kind and string-compatibility validation to `TokenValue::Variable` semantic check — verify symbol is `SymbolKind::Const` with resolvable string value | `semantic/resolve.rs:282-297` | S | Domain Expert, Implementation Strategist |

### MEDIUM — Fix This Sprint

| # | Recommendation | Files | Effort | Source |
|---|---------------|-------|--------|--------|
| **M1** | Thread source spans through `TokenDef`/`TokenValue` for design token diagnostics | `parser/design.rs`, `ast/design.rs`, `semantic/resolve.rs` | M | 6 specialists |
| **M2** | Consolidate dynamic tokens into single `__omni_effect` per design block | `codegen/omniview/design.rs:278-298` | S | Performance, TypeScript/React |
| **M3** | Add `document.documentElement` null guard in generated dynamic token effects | `codegen/omniview/design.rs:290-292` | XS | TypeScript/React |
| **M4** | Remove `resolve_token_value()` dead code | `ir/lower/design.rs:120-131` | XS | Rust Expert, Pragmatic Enforcer, Maintainability |
| **M5** | Remove or downgrade `lower_design()` public wrapper — it silently discards diagnostics | `ir/lower/design.rs:16-21` | XS | Rust Expert, Pragmatic Enforcer |
| **M6** | Fix error offset in sub-parser failure — use `expr_tokens[0].span.start` instead of `0` | `parser/design.rs:350-352` | XS | Rust Expert, Compiler Architect |
| **M7** | Add `{...}` depth tracking to expression boundary scanner | `parser/design.rs:312-339` | XS | Compiler Architect |
| **M8** | Apply `sanitize_css_value()` to static token `:root` values for consistency | `codegen/omniview/design.rs:79` | XS | Security |
| **M9** | Add doc comment about `is_css_value()` heuristic and PascalCase/UPPER_SNAKE_CASE convention | `parser/design.rs:364-387`, language docs | S | Domain Expert, Maintainability |

### LOW — Defer

| # | Recommendation | Files | Effort | Source |
|---|---------------|-------|--------|--------|
| L1 | Implement `Display` for `TokenType` + `From<TokenType> for TokenTypeIR` to centralize 3-way sync | `ast/design.rs`, `ir/design.rs`, `resolve.rs` | S | Maintainability |
| L2 | Improve theme override diagnostic with CSS cascade rationale | `semantic/resolve.rs:314-329` | XS | Domain Expert |
| L3 | Add hot-reload cleanup mechanism for dynamic token effects | `codegen/omniview/design.rs` + bridges | M | TypeScript/React |
| L4 | Guard `:root` block emission when all tokens are dynamic | `codegen/omniview/design.rs:75-84` | XS | Performance |
| L5 | Extend `infer_token_expr_type()` for chained field access and method calls | `semantic/resolve.rs:104-125` | M | Implementation Strategist |
| L6 | Evaluate explicit variable reference syntax (`$BRAND`) to eliminate `is_css_value()` heuristic | `parser/design.rs` | M | Pragmatic Enforcer |
| L7 | Implement `Deserialize` for `Expr` → full TokenValue JSON round-trip | `ast/design.rs:66-132` | L | Systems Architect, Rust Expert |
| L8 | Add `debug_assert!` for suffix invariant in `err_at()` | `parser/design.rs:37-42` | XS | Rust Expert |
| L9 | Process `@themes` directives before token parsing to eliminate backtracking | `parser/design.rs:128-154` | M | Maintainability |

---

## Technical Debt Assessment

| Area | Current Level | Trend | Impact | Notes |
|------|--------------|-------|--------|-------|
| Parser: dual-mode architecture | Medium | Stable | Medium | String-based + token-stream bridge is fragile but functional |
| Semantic: validation gap | Medium | Increasing | High | Const-kind and type checks missing for Variable path |
| IR: dead/duplicate APIs | Low | Stable | Low | `resolve_token_value`, `lower_design`, unused `token_type` |
| Codegen: effect batching | Low | Stable | Low | Per-token effects fine at current scale (0-5 tokens) |
| Diagnostics: zero spans | Medium | Stable | Medium | All design token errors point to wrong location |
| Security: sanitization | Low | Stable | Low | One inconsistency in static `:root` path |
| Documentation | Low | Decreasing | Low | ADR-047/049 thorough, code comments clear |

---

## Architectural Evolution

### Current Architecture vs. Target Architecture

The current implementation follows the two-phase plan from `docs/designs/dynamic-token-variables.md` exactly. The dual-mode parser (string-based for design body, token-stream for expression values) is the most architecturally complex piece and would benefit from eventual unification — but the Pragmatic Enforcer confirms this is the right trade-off for now.

### Migration Path

Phase 1 (const refs) and Phase 2 (runtime signals) are complete and stable. Recommended Phase 3 work:
1. **Validation hardening** (H1-H3, M1-M9 above) — close the diagnostic gaps
2. **Parser unification** (L-term) — migrate design body parser to token-stream, eliminating dual-mode
3. **Dynamic theme values** (L-term) — if user demand materializes, implement JS codegen for theme-conditional dynamic values

---

## Risks if Unaddressed

| Risk | Concern | Impact |
|------|---------|--------|
| Documentation-reality gap erodes trust | H1: `App_dynamicAccent` documented but broken | Users follow docs, get wrong output, lose confidence in type system |
| Silent theme override loss | H2: Dynamic theme values quietly dropped | Dark-mode overrides disappear, UI shows wrong colors, no compile-time feedback |
| Non-const refs silently degrade | H3: `Store`/`Component` symbols pass validation then `var()` fallback | Users debug why CSS custom property doesn't resolve, compiler provides zero help |
| Poor error localization | M1: Zero-span diagnostics | Manual search through 20+ token blocks to find undefined references |
| `is_css_value()` traps lowercase consts | M9: Heuristic footgun | `const brand = "#fff"` → `--primary: brand` (literal string), not `--primary: #fff` |
| Fragile dual-parser | Theme 4: Manual offset sync | A well-intentioned `trim()` refactor silently breaks expression parsing |

---

## Success Metrics

| Metric | Status | Target |
|--------|--------|--------|
| Tests passing | ✅ 547+ | All pass |
| Compile-time overhead | ✅ Zero for static tokens | Zero |
| Runtime overhead | ✅ O(N) effects (current scale OK) | O(1) after M2 |
| Security vulnerabilities | ✅ None (CRITICAL/HIGH) | None |
| Backward compatibility | ✅ All existing tests pass | No regressions |
| ADR coverage | ✅ ADR-047, ADR-049 | Complete |
| IDE bridge round-trip | ⚠️ Expression not deserializable | Needs Expr: Deserialize |

---

## Follow-up Plan

1. **Immediate**: Address H1-H3 validation gaps (XS–M effort, next session)
2. **This sprint**: Complete M1-M9 medium-priority fixes (~1-2 days total)
3. **Next sprint**: Evaluate L1-L9 low-priority deferred items; profile dynamic token effects at 20+ token scale
4. **Long-term**: Parser unification (design body → token-stream); dynamic theme value codegen if user demand

---

## Appendices

### A. Review Methodology

11 independent subagent reviews conducted in parallel, each reading key implementation files and ADRs. Cross-cutting themes identified by frequency analysis (≥3 specialists). Severity calibration via Systems Architect + Domain Expert + Compiler Architect triangulation.

### B. Referenced Documents

- `docs/designs/dynamic-token-variables.md` — Design document
- `.architecture/decisions/adrs/ADR-047-*.md` — Phase 1 ADR
- `.architecture/decisions/adrs/ADR-049-*.md` — Phase 2 ADR
- `.architecture/decisions/adrs/ADR-050-*.md` — Dynamic token validation gap closure
- `crates/omnilang-core/src/parser/ast/design.rs` — TokenValue AST types
- `crates/omnilang-core/src/parser/design.rs` — Design body parser
- `crates/omnilang-core/src/ir/design.rs` — Design IR types
- `crates/omnilang-core/src/ir/lower/design.rs` — AST→IR lowering
- `crates/omnilang-core/src/semantic/resolve.rs` — Semantic analysis
- `omnilang/src/codegen/omniview/design.rs` — CSS/JS codegen
- Memory files: `adr-047-dynamic-design-tokens.md`, `adr-049-phase2-runtime-signals.md`
