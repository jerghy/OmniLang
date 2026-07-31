# ADR-051: Dynamic Design Tokens Global Review — Framework Integration Gap Closure

## Status

Implemented (2026-07-31)

## Context

Following the feature-level architecture review (2026-07-31, 11 specialists, 3 HIGH + 9 MEDIUM → ADR-050) and the Phase 1+2 implementation (ADR-047, ADR-049), a second **global OmniView framework integration review** was conducted by 12 specialists. This review examined how dynamic design tokens interact with the broader OmniView framework — hot-reload, cross-backend codegen, signal lifecycle, documentation, and editor integration.

The review found **22 findings**: 3 CRITICAL, 9 HIGH, and 10 MEDIUM. Cross-referencing against past ADRs (ADR-044, ADR-047, ADR-049, ADR-050) revealed that **5 findings align with intentional design decisions** — they should be preserved and documented, not "fixed." The remaining **17 findings** fall into genuine gaps (not addressed by any ADR) or incomplete applications of prior ADR fixes.

The 12-specialist review report is at `.architecture/reviews/dynamic-design-tokens-omniview-global-2026-07-31.md`.

## Decision Drivers

- **Correctness**: CRITICAL findings represent silent failure paths (hot-reload memory leak, cross-backend zero-output, documentation invisibility)
- **ADR consistency**: 5 findings align with explicit ADR decisions — must be preserved and documented, not overridden
- **Pragmatic scope**: All fixes are additive (new diagnostics, single-line guards, documentation) or subtractive (dead code removal). Zero pipeline restructuring
- **YAGNI**: Deferred items (FieldAccess recursion, @for effect cleanup, Poiesis editor integration, feature flags) are explicitly deferred with justification
- **Symmetric validation**: ADR-050 fixes must be consistently applied across primary value and theme_value paths

## Decision

**Implement 14 targeted fixes across 5 groups, document 5 intentional ADR-aligned design decisions, and defer 4 items with explicit justification.**

### Group A: Critical Lifecycle & Cross-Backend Fixes (2 tasks)

**A1 — Hot-reload effect cleanup**: Capture `__omni_effect` return value under `window.__omni_tokenEffectCleanup`, calling the old dispose function before registering new effects. Prevents linear memory leak in editor mode where each hot-reload creates new effect closures while old ones remain subscribed.

**A2 — Cross-backend WARNING**: Add `has_design_blocks()` check in `compile()` dispatch. When design blocks exist in IR but the target is non-OmniView (React, Flutter, Vue, Dioxus, Python, JavaScript, TypeScript, Rust, Dart), return early with a `CompileOutput` containing a warning. Previously these backends silently produced zero design CSS output.

### Group B: Documentation Fixes (4 tasks)

**B1 — Design system guide**: Add "Dynamic Token Values" subsection to `docs/omniview/10-design-system.md` documenting const references, signal expressions, naming convention, and restrictions.

**B2 — Grammar BNF**: Fix `token_def` production in `docs/omniview/15-grammar-view.md` to include type annotation, semicolon, and correct `token_type` enum. Replace outdated color-* naming convention example.

**B3 — Const description**: Update `docs/omniview/14-grammar-lang.md` from "parsed but discarded at the AST level" to reflect Phase 1 behavior (stored in `Module.consts`).

**B4 — ADR cross-references**: Fix wrong filenames in ADR-050 References section and wrong ADR number in the previous review report.

### Group C: Validation & Sanitization Symmetry (4 tasks)

**C1 — FontWeight numeric types**: Add `TokenType::FontWeight` to the `numeric_ok` match in `check_token_type_compatibility()`. CSS font-weight accepts numeric values 100-900 but was missing from the numeric-accepting list.

**C2 — Sanitization consistency**: Apply `sanitize_css_value()` to three CSS emission paths missed by ADR-050: `[data-theme]` block values, `@media (prefers-color-scheme: dark)` block values, and pseudo-state property values.

**C3 — Theme value const-kind validation**: Add `SymbolKind::Const` validation to `TokenValue::Variable` references in the `theme_values` loop. Previously only symbol existence was checked — non-const symbols passed and silently degraded to `var(--name)`.

**C4 — Unknown-kind warning**: Emit a `Diagnostic::warning` when a cross-file imported symbol with `SymbolKind::Unknown` is used as a `TokenValue::Variable`. Alerts users without breaking cross-file const imports (the `Unknown` exemption from ADR-050 D3 is preserved).

### Group D: Code Quality Improvements (3 tasks)

**D1 — Dead code removal**: Remove duplicate `error_offset` assignment and replace `.iter().cloned().collect()` with `.to_vec()`.

**D2 — `#[non_exhaustive]` on `TokenValueIR`**: Add crate-boundary defense attribute. All existing match sites already had wildcard arms — no downstream changes needed.

**D3 — Did-you-mean suggestions**: Add `suggest_similar_name()` to the E0001 undefined-variable diagnostic for design token values, plus naming convention guidance.

### Group E: Deferred Items Documentation (1 task)

**E1 — Document intentional design decisions and deferred items** in the global review report.

### Intentional Design Decisions (Preserved, NOT Modified)

These 5 findings from the review align with explicit ADR decisions and were documented rather than modified:

| Finding | ADR Reference | Design Intent |
|---------|--------------|---------------|
| Theme sync unidirectional | ADR-044 L83 | Store is single source of truth; `__omni_theme.toggle()` designed for store-action use |
| Binary operators out of scope | ADR-047 L137 | "Not a full expression system… This is intentional" |
| `var(--name)` graceful degradation | ADR-047 L128 | Listed as Positive consequence — lenient over crash |
| `SymbolKind::Unknown` exemption | ADR-050 L159-162 | Rejecting Unknown would break every cross-file const import |
| Effect consolidation trade-off | ADR-050 L109 | "Per-token dependency granularity lost… Accepted because setProperty no-op" |

### Deferred Items (Explicitly Deferred)

| # | Item | Effort | Rationale |
|---|------|--------|-----------|
| E1 | `infer_token_expr_type()` chained FieldAccess resolution | S | Upgrade P3-3 LOW→MEDIUM. Needed for `Store.nested.field` patterns |
| E2 | Per-element effect cleanup in `@for` re-renders | M | Requires `__omni_cleanupElement` extension + effect registry |
| E3 | Poiesis `DesignToken` type dynamic expression support | M | Requires discriminated union + emitter changes. Editor not on critical path |
| E4 | Feature flag `dynamic-tokens` for omnilang-core | S | Build-time gate; deferred until cross-backend design support exists |

## Consequences

### Positive

- **CRITICAL C1 (hot-reload leak) closed**: Effect cleanup guard prevents linear memory leak in editor workflow
- **CRITICAL C2 (cross-backend silence) closed**: Non-OmniView targets now emit a clear warning when design blocks exist
- **CRITICAL C3 (documentation invisibility) closed**: User-facing docs now document Phase 1 and Phase 2 syntax with naming convention
- **Validation symmetry achieved**: Theme value const-kind check mirrors primary value check — same class of silent degradation as original H3 gap is now closed at both sites
- **Sanitization consistency**: All 5 static CSS emission paths (`:root`, `[data-theme]`, `@media`, classes, pseudo-states) now uniformly apply `sanitize_css_value()`
- **Better diagnostics**: Did-you-mean suggestions for undefined variables, Unknown-kind warnings for cross-file imports, FontWeight type acceptance
- **Forward compatibility**: `#[non_exhaustive]` on `TokenValueIR` forces compile errors on new variants at the crate boundary
- **ADR alignment preserved**: 5 intentional design decisions documented rather than overridden — future reviewers can distinguish bugs from trade-offs

### Negative

- **Cross-backend warning is a blunt instrument**: For targets like React (which has partial design awareness via `_design_variant` props), the warning says "design blocks not supported" rather than "partial support." This is intentionally conservative — over-promising would be worse
- **ADR-047/049 documentation gaps remain**: The unchecked documentation checkboxes in ADR-047 and ADR-049 were addressed by Group B, but the ADRs themselves should be updated to reflect completion
- **Effect consolidation still has per-token `setProperty` calls**: Consolidating from N effects to 1 effect was done in ADR-050; the per-token `setProperty` calls within the single effect remain — accepted trade-off per ADR-050 D5

### Neutral

- The 5 intentional design decisions are now searchable in the review document with exact ADR line references
- 4 deferred items replace the previous P3-1 through P3-4 list with clearer effort estimates and rationales

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low. All changes are additive (new diagnostics, single-line guards, documentation) or subtractive (dead code removal). Zero pipeline restructuring.

**Affected Components**:
- `omnilang/src/codegen/omniview/design.rs` — effect cleanup + theme sanitization (3 sites)
- `omnilang/src/codegen/mod.rs` — cross-backend design block check
- `crates/omnilang-core/src/semantic/resolve.rs` — FontWeight type, theme const-kind, Unknown warning, did-you-mean
- `crates/omnilang-core/src/parser/design.rs` — dead code removal
- `crates/omnilang-core/src/ir/design.rs` — `#[non_exhaustive]`
- `docs/omniview/10-design-system.md`, `14-grammar-lang.md`, `15-grammar-view.md` — doc updates
- `.architecture/decisions/adrs/ADR-050-*.md` — cross-reference fix
- `.architecture/reviews/` — review report updates

**Risk Mitigation**:
- Full test suite (554 omnilang-core + 161 omnilang) verified zero regressions
- Each fix is an independent commit — revertible in isolation
- Documentation-only changes have zero code impact

### Reversibility

**Reversibility Level**: High. Each fix is a single commit that can be reverted independently. No data migration, no schema change, no API contract change.

### Sequencing & Timing

**Prerequisites**: ADR-047 (Phase 1), ADR-049 (Phase 2), ADR-050 (validation gap closure).

**System Readiness**: All changes gated behind existing test infrastructure. No new dependencies.

### Social Cost

**Learning Curve**: Low. The documentation changes make the feature discoverable; the diagnostics make errors actionable; the cleanup guard is transparent to users.

**Cognitive Load**: The naming convention (PascalCase/UPPER_SNAKE_CASE for variables, lowercase for CSS values) is now documented in the user-facing guide. Users no longer need to discover it through trial and error.

## Alternatives Considered

### Alternative A: Implement bidirectional theme sync

Make `__omni_theme.set()` push back to store signals, enabling bidirectional `store.mode ↔ __omni_theme`. Flagged as HIGH by Systems Architect.

**Rejected**: ADR-044 explicitly states "The coupling is unidirectional (mode → `__omni_theme`)" — the store is the single source of truth. Adding bidirectional sync would introduce feedback loops and violate the architecture's explicit design intent.

### Alternative B: Parse binary expressions in design token values

Extend the expression boundary scanner to handle `+`, `-`, `*`, `/`, and other binary operators.

**Rejected**: ADR-047 L137 explicitly states "Not a full expression system… not expressions like `BRAND + "80"` or function calls. This is intentional." The silent fallback to Variable is the concern — addressed by the Unknown-kind warning (C4).

### Alternative C: Reject all SymbolKind::Unknown in TokenValue::Variable

Strict enforcement — no exemption for cross-file imports.

**Rejected**: ADR-050 L159-162 documents that this would "break every cross-file const import." The C4 WARNING diagnostic provides user visibility without the breaking change.

### Alternative D: Skip sanitization consistency fix

Leave the three unsanitized CSS paths as-is, since values come from developer source code.

**Rejected**: ADR-050 established the pattern of defense-in-depth sanitization for static CSS values. Consistency reduces the maintenance burden of tracking which paths are sanitized and which are not.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This decision adds zero new abstractions, traits, or IR passes. All 14 fixes are additive diagnostics, single-line guards, documentation additions, or dead code removal. The complexity profile matches ADR-050's excellent ratio — targeted, minimal, independently verifiable.

**Decision Challenge**:

**Proposed Decision**: "Implement 14 targeted fixes for the 3 CRITICAL + 6 IMPORTANT + 4 MEDIUM gaps found in the global OmniView architecture review, document 5 intentional ADR-aligned design decisions, and defer 4 items."

**Necessity Assessment**: 9/10
- **Current need**: The 12-specialist review identified concrete silent-failure paths (hot-reload leak, cross-backend zero-output, documentation invisibility). These are not speculative.
- **Future need**: As OmniView adoption grows and more users hit these gaps, the cost per encounter increases.
- **Cost of waiting**: Each CRITICAL finding wastes developer time: hot-reload leaks degrade editor performance, cross-backend silence requires manual debugging, missing docs prevent feature discovery.
- **Evidence of need**: 12-specialist architecture review with cross-cut analysis, ADR cross-referencing, and independent verification.

**Complexity Assessment**: 3/10
- **Added complexity**: 9 files changed, +1179/-366 lines. All additive or subtractive — zero restructuring.
- **Maintenance burden**: Low — match arms follow existing patterns. No new traits or abstractions.
- **Learning curve**: None — developers familiar with the diagnostic pattern and codegen conventions will recognize these as minor extensions.
- **Dependencies introduced**: Zero.

**Alternative Analysis**: Alternatives A-D were considered and rejected on pragmatic or ADR-alignment grounds. The chosen approach is the minimal set of changes that close the identified gaps without restructuring existing code and without violating intentional ADR decisions.

**Simpler Alternative Proposal**: The implementation itself is the simplest viable alternative. Further simplification would mean accepting the gaps — which contradicts the 12-specialist review's consensus.

**Recommendation**: ✅ Approve decision (implemented)

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 3/10
- **Ratio**: 0.33 (well below 1.5 target)

**Overall Assessment**: This decision represents appropriate engineering for current needs. The 14 fixes are collectively the minimal coherent response to a systematic global architecture review. The intentional design decisions are preserved and documented rather than overridden. The deferred items have explicit effort estimates and trigger conditions.

## Validation

**Acceptance Criteria**:
- [x] All 554 omnilang-core lib tests pass
- [x] All 161 omnilang lib tests pass
- [x] All integration tests pass
- [x] Zero new clippy warnings
- [x] Hot-reload effect cleanup guard emitted in generated JS
- [x] Cross-backend WARNING emitted for non-OmniView targets with design blocks
- [x] User-facing docs show dynamic token syntax with naming convention
- [x] Grammar BNF matches actual parser behavior
- [x] FontWeight accepted as numeric token type
- [x] `sanitize_css_value()` applied to all 5 static CSS emission paths
- [x] Theme value Variable references validated for const-kind
- [x] Unknown-kind warning emitted for cross-file imports in design token values
- [x] Did-you-mean suggestions in undefined variable diagnostic
- [x] `#[non_exhaustive]` on `TokenValueIR` with zero downstream breakage
- [x] 5 intentional design decisions documented with ADR line references
- [x] 4 deferred items documented with effort estimates

**Testing Approach**:
- Unit: `cargo test -p omnilang-core` (554 tests, 0 failures)
- Integration: `cargo test -p omnilang` (161 tests, 0 failures)
- Documentation: manual review of rendered markdown

## References

- [ADR-047: Dynamic Variable References in Design Tokens — Phase 1](ADR-047-dynamic-variable-design-tokens.md)
- [ADR-049: Dynamic Token Variables — Phase 2 Runtime Signal References](ADR-049-dynamic-token-variables-phase2-runtime-signals.md)
- [ADR-050: Dynamic Design Token Validation Gap Closure](ADR-050-dynamic-token-validation-gap-closure.md)
- [ADR-044: Theme Store Mode ↔ __omni_theme Auto-Sync](ADR-044-theme-store-mode-omni-theme-auto-sync.md)
- [Architecture Review: Dynamic Design Tokens — Global OmniView Integration](../../.architecture/reviews/dynamic-design-tokens-omniview-global-2026-07-31.md)
- [Architecture Review: Dynamic Design Tokens 2026-07-31 (feature-level)](../../.architecture/reviews/dynamic-design-tokens-2026-07-31.md)
- [Implementation Plan: Global Review Fixes](../../../docs/superpowers/plans/2026-07-31-dynamic-token-global-review-fixes.md)
