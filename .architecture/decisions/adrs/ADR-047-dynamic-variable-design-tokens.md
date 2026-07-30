# ADR-047: Dynamic Variable References in Design Tokens — Two-Phase Compile-Time-First Approach

## Status

Proposed

## Context

Design tokens (`design` blocks) are the single source of truth for design values in OmniView. Per ADR-016, theme switching operates at the CSS cascade layer — `@themes` declarations generate compile-time CSS custom property blocks switched via `data-theme` HTML attribute, with zero JS runtime overhead.

However, token **values** can only be static string literals:

```omni
design AppDesign {
    token primary: color = "#6366f1";     // ✅ static literal
    token primary: color = BRAND;         // ❌ parsed as literal "BRAND"
    token primary: color = store.mode;    // ❌ parsed as literal "store.mode"
}
```

The design token pipeline is completely isolated from the expression system:

| Pipeline Stage | Expression System | Design Token System |
|---|---|---|
| Parser | Pratt parser (24 Expr variants) | Standalone string parser (`parse_string_or_ident`) |
| AST | `Expr` enum | `TokenDef { value: String }` |
| Semantic | Symbol collection + reference resolution + type checking | **Completely bypassed** |
| IR | `ExprIR` nodes | `TokenIR { value: String }` |
| Codegen | JS expression emission | Static `:root { --token: value; }` CSS |

Users have requested the ability to:
1. Reference imported constants: `import { BRAND } from "./tokens.omni"` → `token primary: color = BRAND;`
2. Reference store signals: `token primary: color = ThemeStore.primaryColor;`

Additionally, `const` declarations at the module level are **parsed and immediately discarded** — there is no mechanism to define constants for reuse.

## Decision Drivers

* **DRY for color/size values**: Currently the same hex value must be repeated in design tokens and in any store/component logic that needs the value
* **CSS cascade is the correct layer for static values** (ADR-016): Static token values should remain compile-time CSS with zero JS overhead
* **Pragmatic scope**: The most common use case is `const` definitions shared across files — runtime dynamic tokens are a niche need
* **Backward compatibility**: All existing design blocks with static values must continue to work unchanged
* **Heuristic ambiguity**: Bare identifiers like `white`, `ease-in-out`, `16px` are valid CSS values, not variable references — distinguishing CSS values from variable names requires heuristics
* **Minimal pipeline disruption**: The standalone design parser must be minimally modified; the Pratt expression parser should be reused where possible

## Decision

**Two-phase approach: compile-time constants first, runtime signals deferred.**

### Phase 1: Compile-Time Constant References (Proposed)

Enable design tokens to reference `const` declarations at compile time via a heuristic-based `TokenValue` enum:

```omni
// tokens.omni
const BRAND = "#6366f1";
const SPACING = "16px";

// theme.omni
import { BRAND, SPACING } from "./tokens.omni";

design AppDesign {
    token primary: color = BRAND;       // TokenValue::Variable("BRAND") → resolved at compile time
    token spacing: size = SPACING;      // TokenValue::Variable("SPACING") → resolved at compile time
    token white: color = white;         // TokenValue::Literal("white") — heuristic detects CSS keyword
    token radius: size = "8px";         // TokenValue::Literal("8px") — quoted, always literal
}
```

**Pipeline changes per layer:**

1. **AST**: `const` declarations are stored in `Module.consts: Vec<ConstDef>` instead of discarded
2. **AST**: `TokenDef.value: String` → `TokenValue::Literal(String) | Variable(String)`
3. **Parser**: `parse_string_or_ident()` → `parse_token_value()` with heuristic `is_css_value()` to distinguish CSS keywords/values from variable names
4. **Semantic**: Token variable references validated against symbol table; `Const` symbol kind added
5. **IR**: `lower_design_with_consts()` resolves `TokenValue::Variable` against a compile-time constant map built from all modules; unresolved variables fall back to `var(--name)`
6. **Codegen**: No change — resolved values are static strings, same CSS output

**Heuristic for CSS value detection (`is_css_value()`):**
- Starts with digit → CSS value (e.g., `16px`, `1.5`, `0.5s`)
- Starts with `#` → hex color (e.g., `#6366f1`)
- In known CSS keyword set → CSS value (e.g., `white`, `solid`, `ease-in-out`)
- All lowercase with hyphens → CSS value (e.g., `inline-flex`, `border-box`)
- Contains uppercase or camelCase → variable reference (e.g., `BRAND`, `primaryColor`)

### Phase 2: Runtime Signal/Store References (Deferred)

Enable design tokens to reference runtime signals:

```omni
token primary: color = ThemeStore.primaryColor;
// → generates: __omni_effect(() => { document.documentElement.style.setProperty("--primary", ThemeStore_primaryColor.value); });
```

This is **deferred** because:
1. It reintroduces the O(n) `setProperty()` pattern that ADR-016 explicitly removed for themes — for individual dynamic tokens this is justified, but the need must be demonstrated
2. It requires full expression parsing within design token values (major parser change)
3. It requires `TokenIR::value` to store `ExprIR` (major IR change)
4. It adds runtime JS overhead where CSS custom properties already provide browser-native dynamism via `var()`

**Phasing strategy:** Implement Phase 1 first. If user demand for runtime dynamic tokens materializes (demonstrated by real use cases, not speculative needs), Phase 2 can be added as a `TokenValue::Expression(Expr)` variant with the existing `emit_style_assignment()` bridge.

**Architectural Components Affected:**
* `crates/omnilang-core/src/parser/ast/design.rs` — TokenDef, TokenValue
* `crates/omnilang-core/src/parser/ast/mod.rs` — Module (add consts), ConstDef
* `crates/omnilang-core/src/parser/design.rs` — parse_token_value(), is_css_value()
* `crates/omnilang-core/src/parser/top_level/mod.rs` — const parsing in parse_module()
* `crates/omnilang-core/src/semantic/collect.rs` — const symbol collection
* `crates/omnilang-core/src/semantic/resolve.rs` — token reference resolution
* `crates/omnilang-core/src/semantic/symbols.rs` — SymbolKind::Const
* `crates/omnilang-core/src/ir/lower/design.rs` — lower_design_with_consts()
* `crates/omnilang-core/src/ir/lower/mod.rs` — const_values map, cross-file resolution
* `crates/omnilang-core/src/validate.rs` — CrossFileSymbolKind::Const

**Interface Changes:**
- `TokenDef.value` type change: `String` → `TokenValue` (breaking AST change, contained within parser/IR boundary)
- `lower_design()` signature: adds `const_values: &FxHashMap<String, String>` parameter
- `Module` struct: adds `consts: Vec<ConstDef>` field (additive, backward compatible via Default)

## Consequences

### Positive

* **Single source of truth**: Color/size values defined once as `const`, referenced in both design tokens and component logic
* **Zero runtime overhead**: Phase 1 is pure compile-time — same CSS output as static literals
* **Cross-file reuse**: `import { BRAND } from "./tokens.omni"` works with existing import system
* **Backward compatible**: All existing static token values are `TokenValue::Literal` — no behavior change
* **Graceful degradation**: Unresolved variables emit `var(--name)` CSS fallback — no compile error
* **Phased risk**: Phase 1 changes are additive and contained; Phase 2 can be added or abandoned independently

### Negative

* **Heuristic ambiguity risk**: `is_css_value()` may misclassify edge cases (e.g., all-lowercase variable names like `brand` would be treated as CSS keywords)
  * **Mitigation**: Users can quote any value (`"brand"`) to force literal treatment; documentation will recommend PascalCase/camelCase for constant names
* **Const value storage**: Adds `ConstDef` to AST/Module — currently constants are discarded; storing them adds memory for modules with many constants
  * **Mitigation**: `ConstDef` is small (name + Expr + span); negligible impact
* **Not a full expression system**: Phase 1 only supports simple `const` variable references, not expressions like `BRAND + "80"` or function calls
  * **Mitigation**: This is intentional — compile-time constant folding of arbitrary expressions is a separate feature; users who need computation should use a `computed` in a store

### Neutral

* Phase 2 deferral means users who need truly dynamic token values must continue using the existing inline `style: { --token: signal }` pattern
* The heuristic adds a small mental model: "lowercase = CSS value, CamelCase = variable" — this is learnable but non-obvious

## Implementation Strategy

### Blast Radius

**Impact Scope**: Parser → Semantic → IR Lowering pipeline. No codegen changes. No runtime changes.

**Affected Components**:
- Design token parser (`parser/design.rs`) — core change: value type from String to TokenValue
- Semantic analyzer — new const collection + token reference validation
- IR lowerer — new const resolution pass
- Cross-file validation (`validate.rs`) — const symbol registration

**User Impact**: Existing `.omni` files with design tokens continue to work. New syntax available for users who want to reference constants.

**Risk Mitigation**:
- Extensive parser tests for heuristic edge cases
- Semantic validation catches undefined variable references with clear E0001 errors
- Fallback `var(--name)` ensures unresolvable references don't break CSS output

### Reversibility

**Reversibility Level**: Medium

**Rollback Feasibility**:
- `TokenValue` enum can be collapsed back to `String` by mapping all variants to their string representations
- `ConstDef` storage can be removed — was already parsed-and-discarded before
- Complexity is additive (new types) rather than restructuring existing logic

**Migration Paths**:
- **Forward Migration**: Implement Phase 1 tasks sequentially (Task 0 → Task 5)
- **Rollback Migration**: Revert commits in reverse order; all tests pass at each step
- **Evolution Path**: Phase 1 → Phase 2 by adding `TokenValue::Expression(Expr)` variant and `TokenValueIR::Dynamic(ExprIR)`

**Options Preserved**:
- Future explicit syntax (`$BRAND` or `@BRAND`) for variable references if heuristic proves insufficient
- Phase 2 runtime dynamic tokens via existing `emit_style_assignment()` bridge
- Type checking for design token values (Color vs Size vs Duration)

**Commitments Made**:
- `TokenValue` enum design locks in the compile-time/runtime distinction
- Heuristic `is_css_value()` function commits to a specific set of recognized CSS keywords
- `ConstDef` stored in module — future const optimizations must be compatible

### Sequencing & Timing

**Prerequisites**:
- [x] Full compiler pipeline understanding (completed 2026-07-30)
- [x] Design document (`docs/designs/dynamic-token-variables.md`)
- [x] Implementation plan (`docs/superpowers/plans/2026-07-30-dynamic-design-tokens.md`)

**System Readiness**:
- **Observability**: Existing 800+ test suite covers design parsing, lowering, and codegen
- **Dependencies**: No new dependencies
- **Infrastructure**: No infrastructure changes
- **Data Migration**: No data migration needed

**Sequencing Concerns**:
- Task 0 (store consts) MUST precede Tasks 1-5
- Tasks 1-3 are sequential (Parser → Semantic → IR)
- Task 4 (cross-file) depends on Task 3
- Task 5 (E2E CSS) is independent verification

**Readiness Assessment**: Ready to implement

### Social Cost

**Learning Curve**: Low — new syntax is natural: `token primary: color = BRAND;` looks like any other variable reference

**Cognitive Load**:
- Users must understand: lowercase identifiers are CSS values, CamelCase identifiers are variables
- Tooling can help: semantic analyzer provides `did you mean 'BRAND'?` suggestions for unresolved references
- Documentation will clearly explain the heuristic

**Documentation Needs**:
- [ ] Update design token documentation with constant reference examples
- [ ] Document `is_css_value()` heuristic and how to force literal treatment
- [ ] Update OmniView language reference (BNF grammar)

### Confidence Assessment

**Model Correctness Confidence**: High

**Assumptions**:
1. Users primarily want to share color/size constants across files — **Validation**: This is the pattern observed in existing learn-omniview examples where colors are manually duplicated
2. The heuristic adequately distinguishes CSS values from variable names — **Validation**: Tested against all 12 existing CSS keywords in `CSS_KEYWORDS` set + common patterns; edge cases can be quoted
3. Phase 2 is a future need with no current urgency — **Validation**: Zero user requests for runtime-dynamic tokens; all current theme needs are met by `@themes`

**Uncertainty Areas**:
- Whether the heuristic will need refinement for non-English CSS keyword sets
- Whether `const` at module level should be `pub const` to control export visibility (currently all consts are implicitly public)

**Edge Cases**:
- Variable named the same as a CSS keyword (e.g., `const white = "#ffffff"`) — resolved by quoting: `token color: color = "white";` for literal, `token color: color = white;` references the const (Variable takes priority when name matches a symbol)
- Multi-word CSS keywords with spaces — always quoted
- Numeric consts (e.g., `const RADIUS = 8`) — Phase 1 only supports string consts; numeric consts deferred

## Implementation

**Phase 1: Compile-Time Constants** (5 tasks, ~3 hours)

See full implementation plan: `docs/superpowers/plans/2026-07-30-dynamic-design-tokens.md`

1. **Task 0**: Store `const` declarations in AST (`Module.consts: Vec<ConstDef>`)
2. **Task 1**: Add `TokenValue` enum (`Literal | Variable`), update parser heuristic
3. **Task 2**: Semantic analysis — collect const symbols, validate token variable references
4. **Task 3**: IR lowering — compile-time constant resolution via `const_values` map
5. **Task 4**: Cross-file import support (`CrossFileSymbolKind::Const`)
6. **Task 5**: E2E CSS output verification

**Phase 2: Runtime Signal References** (Deferred — outlined in design doc)

Design document: `docs/designs/dynamic-token-variables.md`

## Alternatives Considered

### Alternative A: CSS `var()` Chaining Only

Allow token values to reference other tokens via CSS: `token primary: color = var(--otherToken);`

**Pros:**
* Zero implementation cost — CSS already supports `var()` references
* No parser or semantic changes needed

**Cons:**
* Doesn't solve the actual problem — users want to reference OmniLang variables, not just CSS variables
* No compile-time validation — broken references only visible in browser DevTools
* Circular: tokens already emit as CSS custom properties; referencing them via `var()` would work but is confusing

### Alternative B: Full Runtime Approach (All Tokens Dynamic)

Make ALL token values support arbitrary expressions, generating `__omni_effect` calls for all of them.

**Pros:**
* Maximum expressiveness — any expression anywhere
* Cleaner architecture — tokens are just another expression context

**Cons:**
* Violates ADR-016's principle that CSS cascade is the correct layer for static design values
* O(n) `setProperty()` calls for every token at page load — significant runtime overhead
* Static tokens (99% of use cases) pay the complexity cost
* Loses browser-native `var()` cascade behavior and DevTools debugging

### Alternative C: Explicit `@dynamic` Marker

New syntax to explicitly mark dynamic tokens: `@dynamic token primary: color = ThemeStore.primaryColor;`

**Pros:**
* Explicit — no heuristic ambiguity
* Clear separation of static vs dynamic tokens

**Cons:**
* New syntax to learn and maintain
* Redundant for Phase 1 (compile-time constants don't need `@dynamic`)
* Adds visual noise for what should be a seamless feature

### Alternative D: String Interpolation

`token primary: color = "${BRAND}";` — template string syntax in token values.

**Pros:**
* Unambiguous — explicitly marks variable interpolation
* Familiar syntax (JS template literals)

**Cons:**
* Verbose for simple variable references: `"${BRAND}"` vs `BRAND`
* Requires parser to understand template syntax within design token context
* Doesn't solve the `const` storage prerequisite

## Pragmatic Enforcer Analysis

**Reviewer**: AI Software Architect
**Mode**: Balanced

**Overall Decision Complexity Assessment**:
This decision adds moderate complexity (new enum type, heuristic function, const storage) to enable a concrete, frequently-requested capability. The two-phase approach deliberately defers the complex Phase 2, keeping Phase 1 focused on solving the most common use case (sharing constants across files) with minimal pipeline disruption.

**Decision Challenge**:

**Proposed Decision**: "Extend design token values from `String` to `TokenValue::Literal | Variable` with heuristic CSS-value detection, resolve variable references at compile time against stored `const` declarations, defer runtime signal references to Phase 2."

**Necessity Assessment**: 7/10
- **Current need**: Users currently duplicate color/size values across design tokens, stores, and component logic. This is a real, observed pattern in learn-omniview.
- **Future need**: Runtime-dynamic tokens are speculative — no user has requested them. Deferred to Phase 2.
- **Cost of waiting**: Low — users can continue duplicating values or use existing inline `style: { --token: signal }` pattern
- **Evidence of need**: Existing learn-omniview project duplicates hex colors in design tokens AND in store getters — this would eliminate that duplication

**Complexity Assessment**: 5/10
- **Added complexity**: `TokenValue` enum (2 variants, simple), `ConstDef` struct (3 fields), `is_css_value()` heuristic (~30 CSS keywords, straightforward)
- **Maintenance burden**: Low — `TokenValue` is a simple data enum; the heuristic function may need keyword additions over time
- **Learning curve**: Low — `token primary = BRAND;` is natural syntax for anyone who knows variables
- **Dependencies introduced**: None

**Alternative Analysis**:
- Alternative A (CSS var() chaining) is genuinely simpler but doesn't solve the problem
- Alternative B (full runtime) is dramatically more complex and violates ADR-016
- Alternative D (string interpolation) is a reasonable alternative — more explicit but more verbose

**Simpler Alternative Proposal**:
The current Phase 1 plan IS the simpler alternative — it's the minimal change that solves the demonstrated need. Compared to Alternative B (full runtime), it avoids: expression parser integration, ExprIR storage in design IR, JS runtime effect generation, and O(n) setProperty overhead.

**Recommendation**: ✅ Approve decision (Phase 1 implementation; Phase 2 deferred)

**Justification**:
Phase 1 addresses a concrete, observed need (sharing constant values across files) with well-understood, additive changes to 3 pipeline stages. The heuristic approach has known edge cases with clear mitigations. Phase 2 is correctly deferred — implementing it now would over-engineer for speculative needs. The separation into `TokenValue::Literal | Variable` cleanly reserves space for a future `Expression` variant without forcing it.

**If Deferring or Simplifying**:
- **Trigger conditions for Phase 2**: Demonstrated user need for dynamic token values (real use cases, not hypothetical); at least 2 independent user requests or project-internal needs
- **Minimal viable alternative for dynamic tokens**: Continue using existing inline `style: { --token: signal.value }` pattern — already works today
- **Migration path**: Add `TokenValue::Expression(Expr)` variant + `TokenValueIR::Dynamic(ExprIR)` + `emit_dynamic_token_effects()` in codegen — all additive changes compatible with Phase 1 types

**Pragmatic Score**:
- **Necessity**: 7/10
- **Complexity**: 5/10
- **Ratio**: 0.71 *(Target: <1.5 for balanced mode)*

**Overall Assessment**:
This decision represents appropriate engineering for current needs. It resists the temptation to build a full expression system for design tokens (which would be over-engineering) and instead delivers the 80% use case (shared constants) with 20% of the complexity cost. The two-phase structure ensures Phase 2 is only built if demand materializes.

## Validation

**Acceptance Criteria:**
- [ ] `const BRAND = "#6366f1";` is stored in AST and not discarded
- [ ] `token primary: color = BRAND;` parses as `TokenValue::Variable("BRAND")`
- [ ] `token primary: color = white;` parses as `TokenValue::Literal("white")` (CSS keyword)
- [ ] Semantic analyzer produces E0001 error for undefined variable references in tokens
- [ ] IR lowering resolves `TokenValue::Variable("BRAND")` to `"#6366f1"` when const is defined
- [ ] Unresolved variable references fall back to `var(--name)` in IR
- [ ] Cross-file `import { BRAND } from "./tokens.omni"` followed by `token primary = BRAND` works
- [ ] All existing design tests pass without modification
- [ ] Generated CSS is byte-identical to static literal tokens

**Testing Approach:**
* Unit tests for `is_css_value()` heuristic edge cases
* Parser tests for `TokenValue::Literal` vs `TokenValue::Variable` detection
* Semantic tests for undefined variable error (E0001) and valid resolution
* IR lowering tests for compile-time const resolution and fallback
* Integration test: cross-file const → design token → CSS output
* Regression: full test suite (800+ tests) must pass

## References

* [ADR-016: Design Theme System — CSS Cascade Over JS Signal Cascade](ADR-016-design-theme-system.md)
* [ADR-044: Theme Store Mode ↔ __omni_theme Auto-Sync](ADR-044-theme-store-mode-omni-theme-auto-sync.md)
* [Plan: Dynamic Variables in Design Tokens](../../../docs/superpowers/plans/2026-07-30-dynamic-design-tokens.md)
* [Design: Dynamic Token Variables](../../../docs/designs/dynamic-token-variables.md)
* Full pipeline analysis: 5-agent deep-dive of parser, semantic, IR lowering, codegen, import/runtime (2026-07-30)
