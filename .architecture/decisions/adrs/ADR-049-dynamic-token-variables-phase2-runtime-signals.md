# ADR-049: Dynamic Token Variables — Phase 2 Runtime Signal References

## Status

Implemented (2026-07-30)

## Context

ADR-047 defined a two-phase approach for dynamic variable references in design tokens. Phase 1 (also implemented 2026-07-30) enabled compile-time constant references: `token primary: color = BRAND;` resolves at compile time to the value of `const BRAND = "#6366f1"`.

Phase 2 addresses the deferred requirement: enabling design token values to reference runtime signals and store fields, generating `__omni_effect` blocks that dynamically update CSS custom properties via `document.documentElement.style.setProperty()`.

Prior to Phase 2, the design token pipeline was completely isolated from the expression system at every layer:

| Layer | Expression | Design Token (Phase 1) | Gap |
|-------|-----------|----------------------|-----|
| Parser | Pratt (24 Expr variants) | String parser + `TokenValue::Literal\|Variable` | No expression parsing |
| AST | `Expr` enum | `TokenValue` enum | Two variants only |
| Semantic | Full resolve + type check | Variable lookup only | No expr resolution |
| IR | `ExprIR` nodes | `TokenIR { value: String }` | Static strings only |
| Codegen | JS expression emission | Static `:root { --x: val; }` CSS | Compile-time only |

## Decision Drivers

* **User demand for reactive design tokens**: Users need token values that respond to store signals (e.g., theme color changes from a signal rather than CSS `data-theme` attribute switching)
* **ADR-016 consistency**: The CSS cascade layer is correct for static theme switching; dynamic tokens need the JS signal bridge for reactive values
* **ADR-047 phasing**: Phase 2 was explicitly deferred pending ≥2 user requests or internal need — both conditions met
* **Reuse existing infrastructure**: `emit_style_assignment()` bridge, `__omni_effect` signal system, and Pratt expression parser all exist — Phase 2 wires them together
* **Backward compatibility**: All Phase 1 behavior (compile-time consts, static literals, CSS `var()` fallback) must remain unchanged

## Decision

**Extend the design token pipeline to support runtime expression values, bridging signals to CSS custom properties via `__omni_effect`.**

### Pipeline Changes

1. **AST**: `TokenValue` gains `Expression(Expr)` variant alongside `Literal(String)` and `Variable(String)`. Manual `PartialEq` and `Deserialize` impls added to avoid cascading trait requirements through the 27-variant `Expr` type.

2. **Parser**: Design body inner text is tokenized so the Pratt expression parser can be invoked for token values. When an identifier is followed by `.`, `(`, or `[`, the parser delegates to `parse_expr()`, creating a sub-`Parser` from the relevant token slice. Paren/bracket depth tracking prevents early termination at identifiers inside function arguments or array indices.

3. **Semantic**: `TokenValue::Expression(expr)` values are resolved via the existing `resolve_expr_refs()` pass. Store names are declared as module-level `SymbolKind::Store` so expressions like `ThemeStore.primaryColor` resolve correctly.

4. **IR**: New `TokenValueIR` enum replaces `TokenIR.value: String`:
   ```rust
   pub enum TokenValueIR {
       Static(String),     // compile-time resolved
       Dynamic(ExprIR),    // runtime expression
   }
   ```

5. **IR Lowering**: `lower_design_with_consts()` accepts `variant_map` and `semantics` to call `lower_expr()` for `TokenValue::Expression` variants, producing `TokenValueIR::Dynamic(ExprIR)`. `TokenValue::Variable` retains Phase 1 behavior (compile-time lookup → `var(--name)` fallback).

6. **CSS Codegen**: `emit_design_css_impl()` filters tokens by `TokenValueIR::Static` — dynamic tokens are excluded from `:root`, `[data-theme]`, and `@media` CSS blocks.

7. **JS Codegen**: New `emit_dynamic_token_effects()` iterates `TokenValueIR::Dynamic` tokens and generates:
   ```javascript
   __omni_effect(() => {
     const root = document.documentElement;
     root.style.setProperty("--tokenName", __omni_sanitizeCss(expr));
   });
   ```
   The `document.documentElement.style.setProperty()` path gives dynamic values the highest CSS specificity (inline style on `<html>`), ensuring they take precedence over CSS `[data-theme]` selectors and `:root` declarations.

8. **Type Compatibility Check**: `check_token_type_compatibility()` verifies the inferred expression type matches the token's expected type:
   - `String` / `Any` → compatible with all token types
   - `Int` / `Float` → compatible with Duration, Opacity, ZIndex, Size, LineHeight
   - Other types → E0010 type mismatch diagnostic
   - `infer_token_expr_type()` extends generic inference to resolve `FieldAccess` expressions (e.g. `ThemeStore.flag`) by looking up store signal types — avoiding the `Any` fallback that generic `infer_expr_type()` would produce for field accesses.

### Theme Interaction Restriction

Dynamic tokens cannot have theme overrides. A mixed declaration like `token primary: color = ThemeStore.primaryColor dark "#fff"` produces a compile-time diagnostic (E0001). The rationale:
- JS `style.setProperty()` sets inline style on `<html>`, which overrides CSS `[data-theme]` selectors
- The interaction between theme-aware CSS and dynamic JS assignment is semantically unclear
- Users who need theme-aware dynamic tokens should handle theme switching in the store signal itself

**Architectural Components Affected:**
* `crates/omnilang-core/src/parser/ast/design.rs` — `TokenValue::Expression(Expr)`
* `crates/omnilang-core/src/parser/design.rs` — expression parsing + depth tracking
* `crates/omnilang-core/src/parser/top_level/design.rs` — tokenize design body
* `crates/omnilang-core/src/semantic/collect.rs` — store name symbol collection
* `crates/omnilang-core/src/semantic/resolve.rs` — expression ref resolution + theme restriction + type compatibility check
* `crates/omnilang-core/src/ir/design.rs` — `TokenValueIR` enum
* `crates/omnilang-core/src/ir/lower/design.rs` — `lower_expr()` for token values
* `crates/omnilang-core/src/ir/lower/mod.rs` — pass lowering context to design lowering
* `omnilang/src/codegen/omniview/design.rs` — CSS filtering + `emit_dynamic_token_effects()`
* `omnilang/src/codegen/omniview/mod.rs` — integration in `generate()`

**Interface Changes:**
* `TokenIR.value`: `String` → `TokenValueIR`
* `TokenIR.theme_values`: `FxHashMap<String, String>` → `FxHashMap<String, TokenValueIR>`
* `lower_design_with_consts()`: adds `variant_map` and `semantics` parameters
* `parse_design_body()`: adds `design_tokens: &[Token]` parameter

## Consequences

### Positive

* **Reactive CSS custom properties**: Store signal changes automatically propagate to CSS via `setProperty()`, enabling genuinely dynamic design tokens
* **Reuses proven infrastructure**: `lower_expr()`, `expr_to_js()`, `__omni_effect`, and `__omni_sanitizeCss` are all existing — Phase 2 adds only wiring
* **Phase 1 fully preserved**: Static tokens and compile-time const references are unchanged; `TokenValue::Variable` retains `var(--name)` fallback
* **Clear static/dynamic separation**: `TokenValueIR::Static` vs `Dynamic` makes it obvious at codegen time which tokens need runtime JS
* **Graceful degradation**: Parse failures in expression detection fall back to `TokenValue::Variable`, preserving Phase 1 behavior
* **Defensive against mixed semantics**: Theme+dynamic combination rejected at compile time, preventing confusing runtime behavior
* **Zero overhead for static tokens**: The `__omni_effect` blocks are generated only for tokens with `TokenValueIR::Dynamic` values

### Negative

* **O(n) `setProperty` calls per dynamic token**: Each dynamic token generates one `__omni_effect` block. With typical use (1-5 dynamic tokens), this is negligible. If a design system had 50+ dynamic tokens, it may warrant batching — but no such use case exists yet
* **No static HTML fallback**: Dynamic tokens have no CSS value in `:root`, so SSR/static HTML renders without those custom properties. JS execution is required to set them
* **Heuristic ambiguity persists**: The `is_css_value()` vs expression detection heuristic from Phase 1 still applies — lowercase identifiers like `store` could be miscategorized if not followed by `.`/`(`/`[`
* **TokenValue IR enum lacks PartialEq**: `ExprIR` is `#[non_exhaustive]` and doesn't implement `PartialEq`, so `TokenValueIR` can't derive it. Tests use `matches!()` instead of `assert_eq!()`

### Neutral

* Theme+dynamic restriction means users who want theme-aware dynamic tokens must implement theme switching in their store rather than using `@themes` declarations alongside dynamic values
* The `design_tokens: &[Token]` parameter is threaded through the parser call chain — callers that don't need expression parsing pass `&[]`

## Implementation Strategy

### Blast Radius

**Impact Scope**: Cross-cutting — touches parser, semantic analyzer, IR types, IR lowering, CSS codegen, and JS codegen.

**Affected Components**:
- Parser AST (`ast/design.rs`) — new enum variant
- Design body parser (`parser/design.rs`) — expression parsing + depth tracking (~260 lines added)
- Top-level parser (`top_level/design.rs`) — tokenization of design body
- Semantic analyzer (`semantic/collect.rs`, `resolve.rs`) — symbol collection + expression resolution
- IR types (`ir/design.rs`) — `TokenValueIR` enum
- IR lowering (`ir/lower/design.rs`, `lower/mod.rs`) — expression lowering context
- CSS codegen (`omnilang/src/codegen/omniview/design.rs`) — dynamic token filtering
- JS codegen (`omnilang/src/codegen/omniview/design.rs`, `mod.rs`) — `__omni_effect` generation

**User Impact**: Existing `.omni` files with design tokens continue to work unchanged. New syntax `token name: type = store.field;` available for runtime dynamic values. Theme+dynamic combination rejected with clear error.

**Risk Mitigation**:
- Extensive test coverage at each pipeline layer (parser, lowering, semantic, CSS codegen, JS codegen)
- Expression parse failures fall back to Phase 1 `Variable` behavior
- Theme+dynamic combination produces compile-time error rather than silent misbehavior
- Pre-collection pattern in JS codegen avoids borrow-checker conflicts

### Reversibility

**Reversibility Level**: Medium

**Rollback Feasibility**:
- `TokenValue::Expression` variant can be removed; match arms revert to two-variant code
- `TokenValueIR` can be collapsed back to `String` by mapping all `Static` variants
- `emit_dynamic_token_effects()` call in `generate()` is a single line removal
- Complexity is additive (new types, new functions) rather than restructuring existing logic

**Options Preserved**:
- Future explicit `@dynamic` syntax if heuristic proves insufficient
- Token-level batching if profiling shows setProperty overhead
- Type-level CSS custom property validation (Color vs Size vs Duration)

**Commitments Made**:
- `TokenValueIR` enum design locks in static/dynamic distinction at IR level
- `document.documentElement.style.setProperty()` is the mechanism for dynamic CSS bridging
- Theme+dynamic restriction is a compile-time gate — cannot be relaxed without removing the diagnostic

### Sequencing & Timing

**Prerequisites**:
- [x] ADR-047 Phase 1 (compile-time const references)
- [x] Full expression parser (Pratt, 24 Expr variants)
- [x] `__omni_effect` signal system
- [x] `emit_style_assignment()` CSS bridging pattern
- [x] Store signal system

**Readiness Assessment**: Ready to implement (completed 2026-07-30)

### Social Cost

**Learning Curve**: Low — `token primary: color = ThemeStore.primaryColor;` is natural syntax for anyone familiar with dot-notation field access.

**Cognitive Load**:
- Users must understand: dynamic tokens use `document.documentElement.style.setProperty()` (inline style priority)
- Theme+dynamic is rejected at compile time with clear diagnostic
- Static tokens remain CSS-only; dynamic tokens add one `__omni_effect` per token

**Documentation Needs**:
- [ ] Update design token documentation with dynamic token examples
- [ ] Document theme+dynamic restriction and workaround (handle theme in store)
- [ ] Update OmniView language reference

## Implementation

**Phase 2 implementation** (10 tasks, 11 commits + 1 follow-up, 2026-07-30):

1. AST: `TokenValue::Expression(Expr)` variant with manual `PartialEq`/`Deserialize`
2. Parser: Tokenize design body, thread `design_tokens` through parser chain
3. Parser: Expression detection + depth tracking + Pratt delegation in `parse_token_value()`
4. Semantic: Store name collection + `resolve_expr_refs()` for token expressions
5. IR: `TokenValueIR::Static | Dynamic` enum, `TokenIR` field type changes
6. IR Lowering: `lower_expr()` integration via `variant_map` + `semantics` parameters
7. CSS Codegen: Filter `TokenValueIR::Dynamic` from `:root`/`[data-theme]`/`@media` blocks
8. JS Codegen: `emit_dynamic_token_effects()` generating `__omni_effect` + `setProperty()`
9. E2E test: Full source-to-IR pipeline verification
10. Validation: Dynamic+theme combination rejected at compile time
11. Type check: Expression-to-TokenType compatibility with FieldAccess resolution (`b5909b3b`)

**Metrics**: 23 files changed, +1270/-180 lines. 547 core tests + 11 design_theme tests + 5 design codegen tests pass.

## Alternatives Considered

### Alternative A: Full Runtime Approach (All Tokens Dynamic)

Make ALL token values support arbitrary expressions, generating `__omni_effect` for all tokens.

**Pros:**
* Maximum expressiveness — any expression anywhere
* Cleaner architecture — tokens are just another expression context

**Cons:**
* Violates ADR-016 (CSS cascade is correct for static values)
* O(n) `setProperty()` for every token — significant overhead for static tokens (99% of use cases)
* Loses browser-native `var()` cascade behavior and DevTools debugging
* **Rejected**: Over-engineering for speculative needs

### Alternative B: Explicit `@dynamic` Marker

New syntax: `@dynamic token primary: color = ThemeStore.primaryColor;`

**Pros:**
* Explicit — no heuristic ambiguity
* Clear separation of static vs dynamic tokens

**Cons:**
* New syntax to learn and maintain
* Redundant — the dotted-path syntax (`ThemeStore.field`) already unambiguously identifies runtime references
* Adds visual noise
* **Rejected**: Adding syntax complexity for a distinction the parser can already determine

### Alternative C: CSS `var()` Chaining

`token primary: color = var(--otherToken);` — CSS-level token references.

**Pros:**
* Zero implementation cost — CSS already supports `var()` references
* No parser changes needed

**Cons:**
* Doesn't solve the actual problem — users want OmniLang variable references, not CSS variable references
* No compile-time validation
* Circular: tokens already emit as CSS custom properties
* **Rejected**: Doesn't address the stated requirement

## Pragmatic Enforcer Analysis

**Reviewer**: AI Software Architect — Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**:
Phase 2 extends an existing pipeline with new data variants rather than introducing new architectural patterns. The AST, IR, and codegen changes follow established patterns (enum variants, lowering context threading, pattern-matching codegen). The parser change is the most complex piece (depth tracking + sub-Parser creation) but is contained within one function. No new crates, traits, or abstraction layers.

**Decision Challenge**:

**Proposed Decision**: "Extend design token pipeline from `String`-only values to `Literal | Variable | Expression` at AST and `Static | Dynamic` at IR, generate `__omni_effect` blocks for dynamic tokens."

**Necessity Assessment**: 8/10
- **Current need**: Demonstrated — users need tokens to reference store signals for reactive design values. The Phase 1 workaround (duplicating values in both tokens and store getters) is a real friction point observed in learn-omniview.
- **Future need**: Next logical step after Phase 1. The `TokenValue::Expression` variant was explicitly reserved in the Phase 1 `TokenValue` enum design.
- **Cost of waiting**: Low — Phase 1 compile-time references cover the 80% use case. Users who need runtime dynamic values can use the existing inline `style: { --token: signal.value }` pattern. But the gap between Phase 1 capability and user expectation creates confusion.
- **Evidence of need**: Multiple design system users have requested signal-driven token values. The learn-omniview project demonstrates the pattern of wanting store-driven theme values.

**Complexity Assessment**: 6/10
- **Added complexity**: `TokenValue::Expression` variant, `TokenValueIR` enum (2 variants), `emit_dynamic_token_effects()` function (~15 lines), parser depth tracking (~20 lines). Core addition is modest — most complexity is in threading context through existing APIs.
- **Maintenance burden**: Low — enums are the simplest Rust abstraction. New match arms are straightforward. The parser heuristic may need occasional CSS keyword additions (same as Phase 1).
- **Learning curve**: Low — the `Expr` → `ExprIR` → JS pipeline already exists; Phase 2 just routes design token values through it.
- **Dependencies introduced**: None. All infrastructure reused.

**Alternative Analysis**:
- Alternative B (`@dynamic` marker) is simpler at the parser level but adds user-facing syntax complexity — net complexity increase
- Alternative C (CSS `var()`) is simpler to implement but doesn't solve the problem — false simplicity
- Alternative A (full runtime) is dramatically more complex — correctly rejected
- The current approach is the minimal change that solves the demonstrated need while preserving Phase 1 behavior

**Simpler Alternative Proposal**:
The current implementation IS the simpler alternative. Phase 2 was designed as the minimal extension of Phase 1:
- One new enum variant at each pipeline layer (AST, IR)
- One new codegen function
- Parser integration reuses the existing Pratt parser via sub-Parser creation
- No new traits, abstraction layers, or architectural patterns

A truly simpler "do nothing" approach would tell users to continue using inline `style: { --token: signal }` — but this duplicates the token value across every binding site, defeating the purpose of centralized design tokens.

**Recommendation**: ✅ Approve decision

**Justification**:
Phase 2 completes the two-phase architecture outlined in ADR-047. It delivers the most requested post-Phase-1 feature (store signal references in token values) with well-understood, additive changes to the existing pipeline. The implementation reuses proven infrastructure (`lower_expr`, `expr_to_js`, `__omni_effect`) rather than building new machinery. The theme+dynamic restriction is a pragmatic guardrail that prevents confusing semantics without over-engineering a solution for a use case that doesn't exist.

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 6/10
- **Ratio**: 0.75 *(Target: <1.5 for balanced mode)*

**Overall Assessment**:
This decision represents appropriate engineering for current needs. It resists the temptation to make all tokens dynamic (which would violate ADR-016's CSS-cascade preference) and instead adds targeted dynamism for the specific case where it's needed. The static/dynamic split is explicit at the IR level, ensuring the zero-overhead CSS path is preserved for the vast majority of tokens.

## Validation

**Acceptance Criteria:**
- [x] `token primary: color = ThemeStore.primaryColor;` parses as `TokenValue::Expression`
- [x] `token primary: color = BRAND;` still parses as `TokenValue::Variable` (Phase 1)
- [x] `token easing: easing = ease-in-out;` still parses as `TokenValue::Literal` (CSS keyword)
- [x] Semantic analyzer resolves references within `TokenValue::Expression`
- [x] `TokenValueIR::Dynamic(ExprIR)` produced in IR for expression tokens
- [x] `TokenValueIR::Static(String)` produced for literal and resolved-const tokens
- [x] `:root` CSS excludes dynamic tokens
- [x] JS output contains `__omni_effect(() => { ... style.setProperty("--name", ...) })`
- [x] Dynamic + theme override combination produces compile-time E0001 diagnostic
- [x] All 547 core tests + 5 design codegen tests pass
- [x] Phase 1 compile-time const reference behavior unchanged

**Testing Approach:**
* Parser unit tests: expression vs variable vs literal detection (4 new tests)
* IR lowering tests: Static vs Dynamic variant production (2 new tests)
* Semantic integration tests: store field access, undefined reference, theme+dynamic rejection (3 new tests)
* CSS codegen test: dynamic tokens excluded from `:root` (1 new test)
* JS codegen test: `__omni_effect` + `setProperty` + `__omni_sanitizeCss` verification (1 new test)
* E2E pipeline test: full source-to-IR for dynamic tokens (1 new test)
* Regression: full 547-test suite passes

## References

* [ADR-047: Dynamic Variable References in Design Tokens — Phase 1](ADR-047-dynamic-variable-design-tokens.md)
* [ADR-016: Design Theme System — CSS Cascade Over JS Signal Cascade](ADR-016-design-theme-system.md)
* [ADR-044: Theme Store Mode ↔ __omni_theme Auto-Sync](ADR-044-theme-store-mode-omni-theme-auto-sync.md)
* [Design: Dynamic Token Variables](../../../docs/designs/dynamic-token-variables.md)
* [Phase 2 Implementation Plan](../../../docs/superpowers/plans/2026-07-30-dynamic-token-variables-phase2.md)
