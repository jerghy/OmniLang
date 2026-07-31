# ADR-050: Close Dynamic Design Token Validation Gaps — Architecture Review Fixes

## Status

Implemented

## Context

The 2026-07-31 multi-perspective architecture review of dynamic design tokens (Phases 1+2) identified 12 findings: 3 HIGH-severity validation gaps where the compiler produced wrong or dropped output with zero diagnostic feedback, plus 9 MEDIUM-severity hardening issues. All three HIGH findings shared a common root cause: the boundary between semantic analysis and IR lowering lacked compile-time checks for edge cases introduced by the `TokenValue::Variable` and `TokenValue::Expression` enum variants.

The three HIGH gaps were:

1. **H1 — Bare-ident signal refs silently degraded**: `App_dynamicAccent` (documented as a Phase 2 use case) was parsed as `TokenValue::Variable`, passed semantic validation (symbol exists), but produced `var(--App_dynamicAccent)` CSS fallback instead of `__omni_effect` — with zero diagnostics.
2. **H2 — Dynamic theme values silently dropped**: `token primary: color = "#fff" dark ThemeStore.darkColor` passed every pipeline stage, but both CSS codegen (`[data-theme]` skips Dynamic variants) and JS codegen (only collects from `token.value`) dropped the theme override — with zero diagnostics.
3. **H3 — Non-const symbols passed validation**: `TokenValue::Variable` only checked `self.symbols.contains(name)` — stores, signals, and components all passed, then silently degraded to `var(--name)` CSS fallback at IR lowering.

Nine MEDIUM findings covered parser hardening (brace depth tracking, error offset), codegen improvements (effect consolidation, null guard, sanitization), dead code removal, and documentation.

See `.architecture/reviews/dynamic-design-tokens-2026-07-31.md` for the full review.

## Decision Drivers

- **Correctness**: Silently producing wrong or dropped output erodes user trust in the compiler. Every edge case must produce either correct output or a clear diagnostic.
- **Least surprise**: Users expect `const` references to resolve; they do not expect `var(--App_dynamicAccent)` in CSS from a documented syntax pattern.
- **Symmetric rules**: Theme value validation should mirror main-value validation — if dynamic expressions are restricted in one position, they should be restricted in both.
- **Pragmatic scope**: Fixes must be additive (add diagnostics or remove dead code), never restructure working paths. Zero risk to existing static-token users.
- **YAGNI**: No new IR passes, traits, or abstractions — only targeted match-arm additions and function removals.

## Decision

Add compile-time validation to catch three categories of silent degradation in design token values, plus hardening improvements across parser, codegen, and dead-code cleanup.

### 1. Semantic Validation (HIGH fixes)

**Reject `TokenValue::Expression` in theme values** (`crates/omnilang-core/src/semantic/resolve.rs`):
- Add E0001 diagnostic in the `theme_values` loop's `Expression` arm, symmetric to the existing dynamic-main-value restriction (line 314).
- Diagnostic: "Theme value '{theme}' for token '{name}' cannot be a dynamic expression. Theme overrides must use static values."

**Add const-kind validation to `TokenValue::Variable` references**:
- After verifying symbol existence, resolve the symbol and check `sym.kind == SymbolKind::Const`.
- `SymbolKind::Unknown` is exempted — cross-file imports are tagged Unknown during collection, and rejecting them would break all imported const references.
- Diagnostic for non-Const symbols: "'{name}' is a {kind} in design token '{token}', not a compile-time constant. Only const declarations can be used as direct variable references."

**Regression test for bare signal names (H1 closure)**:
- Test that `App_dynamicAccent` (`SymbolKind::Signal`) triggers the const-kind diagnostic.

### 2. Parser Hardening (MEDIUM fixes)

**Add `{...}` depth tracking to expression boundary scanner** (`crates/omnilang-core/src/parser/design.rs`):
- `LBrace` increments depth, `RBrace` at depth>0 decrements, `RBrace` at depth==0 still terminates.
- Enables expressions with object literal arguments: `store.getConfig({theme: "dark"})`.

**Fix error offset in sub-parser failures**:
- Replace hardcoded `offset: 0` with `expr_tokens.first().map(|t| t.span.start).unwrap_or(offset)`.
- Error now points to the actual expression location, not the design block's `{`.

### 3. Codegen Hardening (MEDIUM fixes)

**Consolidate dynamic token `__omni_effect` blocks** (`omnilang/src/codegen/omniview/design.rs`):
- Replace N separate `__omni_effect` calls with a single block containing all `setProperty` calls.
- Reduces effect subscriptions from O(N) to O(1) per design block.

**Add `document.documentElement` null guard**:
- `if (!root) return;` after `const root = document.documentElement;`.
- Consistent with existing guard in inline style effects (`style.rs`).

**Apply `sanitize_css_value()` to static `:root` token values**:
- Aligns static token CSS path with class/variant property emission.

### 4. Dead Code Removal (MEDIUM fixes)

**Remove `resolve_token_value()`** (`crates/omnilang-core/src/ir/lower/design.rs`):
- `#[allow(dead_code)]` function with zero callers.

**Remove `lower_design()` wrapper**:
- Silently discarded `SemanticResult` diagnostics. All callers updated to use `lower_design_with_consts()` directly.

### 5. Documentation (MEDIUM fix)

**Document `is_css_value()` naming convention**:
- PascalCase/UPPER_SNAKE_CASE identifiers = variable references; all-lowercase = CSS literals.
- Reference ADR-047 design rationale.

**Architectural Components Affected:**
- Semantic analyzer (`semantic/resolve.rs`)
- Design body parser (`parser/design.rs`)
- CSS/JS codegen (`codegen/omniview/design.rs`)
- IR lowering (`ir/lower/design.rs`, `ir/lower/mod.rs`, `ir/mod.rs`)

**Interface Changes:**
- `lower_design()` removed — callers must use `lower_design_with_consts()`.
- `resolve_token_value()` removed — no public interface change (already dead).

## Consequences

### Positive

- Three silent degradation paths now produce clear E0001 compile-time diagnostics.
- `SymbolKind::Unknown` exemption preserves cross-file const imports — existing projects unaffected.
- Effect consolidation reduces JS subscription overhead from O(N) to O(1).
- Null guard prevents TypeError in SSR/Worker contexts.
- Two dead functions removed, reducing maintenance surface.
- All existing 552+ tests pass; 3 new tests added for the validation gaps.

### Negative

- `SymbolKind::Unknown` exemption means cross-file imported const-kind validation is weaker than local const validation — a non-const imported symbol tagged Unknown passes where a local non-const would be rejected. This is a pre-existing limitation of the collect pass (`collect.rs:212`).
- Zero source spans remain on design token diagnostics (deferred — requires threading `Span` through `TokenValue` variants, estimated M effort).
- Per-token dependency granularity is lost after effect consolidation — changing signal A re-sets tokens that depend on signal B. Accepted because `setProperty` with the same value is a browser no-op.

### Neutral

- Diagnostic format for const-kind violations uses zero spans (pre-existing pattern, not made worse).
- `is_css_value()` heuristic footgun remains (lowercase const names silently become CSS literals) — documented with naming convention but not fixed structurally.

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low. All changes are additive diagnostics or targeted replacements within existing match arms. No structural changes to the compiler pipeline.

**Affected Components**:
- `semantic/resolve.rs` — 3 diagnostic additions (E0001), 1 new else-if branch
- `parser/design.rs` — 2 match arms added, 1 line changed, doc comment
- `codegen/omniview/design.rs` — function body rewritten, 1 guard line added, 1 sanitize wrapper added
- `ir/lower/design.rs` — 2 functions removed, callers updated

**Risk Mitigation**:
- All changes are additive (new diagnostics) or subtractive (dead code removal). Nothing that could break existing valid inputs was changed.
- Full test suite (552+ unit + integration) verified zero regressions.

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: Each fix is a single commit that can be reverted independently. No data migration, no schema change, no API contract change (except `lower_design` removal which had zero production callers).

### Sequencing & Timing

**Prerequisites**: None — all fixes are self-contained.

**System Readiness**: All changes gated behind existing test infrastructure. No new dependencies.

## Alternatives Considered

### Alternative A: Auto-convert bare signal names to Expression

Instead of emitting a diagnostic for `token accent: color = App_dynamicAccent;`, auto-promote `TokenValue::Variable` to `TokenValue::Expression(Expr::VarRef(name))` when the symbol is a signal.

**Pros**: "Just works" for the user — no diagnostic to fix.
**Cons**: Surprising behavior if the user actually intended a const reference. Violates explicit-over-implicit principle. Requires parser/semantic cross-talk.

**Rejected**: Explicit diagnostic is clearer and preserves user intent.

### Alternative B: Reject all non-Const symbols including Unknown

Strict enforcement: only `SymbolKind::Const` passes, everything else (including `Unknown`) is rejected.

**Pros**: 100% enforcement at the semantic boundary.
**Cons**: Breaks every cross-file const import. The collect pass tags imported symbols as Unknown, and the `validate_cross_file` step does not propagate the actual kind. Fixing this would require significant refactoring of the import resolution pipeline.

**Rejected**: Unknown exemption is the pragmatic choice until cross-file kind propagation is implemented.

### Alternative C: Skip effect consolidation (keep per-token effects)

**Pros**: Preserves per-token dependency granularity.
**Cons**: O(N) subscriptions scale with dynamic token count.

**Rejected**: At realistic N (0-5), the difference is unmeasurable — but consolidating now prevents structural O(N) scaling if adoption grows.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This decision adds zero new abstractions, traits, or IR passes. Every fix is either an additional match arm emitting a diagnostic, a single-line guard, or a function removal. The complexity profile is excellent — all 12 changes are in the lowest-complexity tier for a compiler refactor.

**Decision Challenge**:

**Proposed Decision**: "Add 12 targeted fixes (3 HIGH validation gaps + 9 MEDIUM hardening) to the dynamic design token compiler pipeline."

**Necessity Assessment**: 8/10
- **Current need**: The 11-specialist architecture review identified concrete silent-failure paths. These are not speculative — they produce wrong output for documented syntax patterns.
- **Future need**: As dynamic token adoption grows, the silent degradation paths would be encountered more frequently.
- **Cost of waiting**: Each encounter wastes developer time debugging CSS `var()` fallbacks with zero compiler feedback. The cost per encounter is high (confusing behavior, eroded trust).
- **Evidence of need**: Architecture review with 11 independent specialists, cross-cut analysis, ADR cross-referencing.

**Complexity Assessment**: 3/10
- **Added complexity**: 3 new diagnostic branches, 2 parser match arms, 1 function rewrite (consolidation), 1 guard line, 1 sanitize wrapper, 2 function removals, 1 doc comment. All are single-site changes.
- **Maintenance burden**: Low — match arms follow existing patterns. No new traits or abstractions to maintain.
- **Learning curve**: None — developers already familiar with the diagnostic pattern and match-arm style will recognize these as minor extensions.
- **Dependencies introduced**: Zero.

**Alternative Analysis**: Alternatives A-C were considered and rejected on pragmatic grounds. The chosen approach is the minimal set of changes that close the identified gaps without restructuring existing code.

**Simpler Alternative Proposal**: The implementation itself is the simplest viable alternative. Further simplification would mean accepting the gaps — which contradicts the architecture review's consensus that these are genuine correctness concerns.

**Recommendation**: ✅ Approve decision (implemented)

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 3/10
- **Ratio**: 0.38 (well below 1.5 target)

**Overall Assessment**: This decision represents appropriate engineering for current needs. The 12 fixes are collectively the minimal coherent response to a systematic architecture review — each fix targets a specific, independently verified gap, and none introduces speculative generality.

## Validation

**Acceptance Criteria:**
- [x] All 552+ existing tests pass
- [x] 3 new semantic validation tests pass (expression-in-theme-value, non-const-reference, bare-signal-name)
- [x] 1 new parser test passes (brace-object-argument expression)
- [x] Zero new clippy warnings
- [x] Dead code removal verified by grep (zero callers for removed functions)
- [x] `cargo check --workspace` clean
- [x] All architecture review H1-H3 gaps demonstrably closed

**Testing Approach**:
- Semantic tests: `cargo test -p omnilang-core` (3 new, 548+ existing)
- Parser tests: `cargo test -p omnilang-core parse_` (1 new, 22 existing)
- Codegen tests: `cargo test -p omnilang design` (17 tests)
- Integration: `cargo test --workspace` (full suite)

## References

- [Architecture Review: Dynamic Design Tokens 2026-07-31](../../.architecture/reviews/dynamic-design-tokens-2026-07-31.md)
- [ADR-047: Dynamic Variable References in Design Tokens Phase 1](ADR-047-dynamic-variable-design-tokens.md)
- [ADR-049: Dynamic Token Variables Phase 2 Runtime Signal References](ADR-049-dynamic-token-variables-phase2-runtime-signals.md)
- [Implementation Plan: Dynamic Token Validation Fixes](../../../docs/superpowers/plans/2026-07-31-dynamic-token-validation-fixes.md)
