# ADR-016: Design Theme System — CSS Cascade Over JS Signal Cascade

## Status

Accepted

## Context

The OmniView style system had two parallel constructs for managing styles and themes, both with major architectural problems:

1. **`theme` keyword (dead code):** A `theme` keyword existed in the language (`TokenKind::Theme`) that parsed into `ThemeDef`/`ThemeCategory` AST nodes, but was *never lowered to IR* and *never codegen'd*. It was pure dead code — users would write `theme AppTheme { ... }` expecting theme support, but the compiler silently ignored it. Documentation admitted "代码生成尚未实现" (code generation not yet implemented).

2. **Manual theme switching via store getter → `setProperty()`:** The learn-omniview project demonstrated the "de facto" theme pattern — users duplicated design token values in a `ThemeStore.omni` with getters like:
   ```omni
   getter currentBg: String = if mode == "dark" { "#111827" } else { "#ffffff" };
   getter currentText: String = if mode == "dark" { "#f9fafb" } else { "#111827" };
   ```
   Then manually bound each themed CSS variable in each component:
   ```omni
   style: { --theme-bg: ThemeStore.currentBg, --theme-text: ThemeStore.currentText }
   ```
   The compiler generated one `setProperty()` effect per `--` signal binding — O(n) DOM operations per theme toggle.

3. **The `design` block was the intended source of truth** for design tokens, but had no theme awareness — all tokens were hardcoded to `:root` with no per-theme value support.

The architectural violation was **layering**: theme switching (a CSS presentation concern) was implemented through store signals and JS mutation (an application logic concern). This produced:
- Duplicate color definitions (design tokens + store getters)
- O(n) DOM `setProperty()` calls per toggle
- No `prefers-color-scheme` system preference support
- No per-theme values in the token declaration language

## Decision Drivers

* **Design blocks as single source of truth**: Theme values should be declared alongside design tokens, not duplicated in store getters
* **CSS cascade is the correct layer**: Theme switching is CSS variable overrides — one `data-theme` attribute toggle replaces N `setProperty()` calls
* **Zero runtime overhead**: Per-theme CSS is generated at compile time; no JS computation needed on toggle
* **System preference support**: `prefers-color-scheme` must be natively supported
* **Backward compatibility**: Existing non-themed design blocks must continue to work unchanged
* **Remove dead code**: The `theme` keyword had no runtime effect — remove it entirely
* **Minimize approach surface**: Theme switching should not require Store, signal, getter, or component knowledge

## Decision

**1. Remove the `theme` keyword entirely.** All associated parser code, AST types, and the `TokenKind::Theme` enum variant are deleted. A migration error message guides affected users to the `@themes` syntax inside `design` blocks.

**2. Extend `design` blocks with theme declarations.** The `design` block becomes the single source of truth for both design tokens and their theme variants:

```omni
design AppDesign {
    @themes light, dark;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token surface: color = "#ffffff" dark "#111827";
}
```

**3. Theme values cascade via CSS, not JS.** The compiler generates three CSS blocks:
- `:root { --primary: #6366f1; }` — default fallback
- `[data-theme="dark"] { --primary: #a5b4fc; }` — per-theme overrides
- `@media (prefers-color-scheme: dark) { :root:not([data-theme]) { ... } }` — system preference auto-detect

**4. A lightweight `__omni_theme` JS runtime handles attribute toggling.** Generated at compile time, the runtime provides `set()`, `toggle()`, `init()` with localStorage persistence and system preference detection — no store or signal involvement required.

**Architectural Components Affected:**
* `crates/omnilang-core/src/lexer/` — removed `Theme` token
* `crates/omnilang-core/src/parser/ast/` — removed `ThemeDef`/`ThemeCategory` (motion.rs), added fields to `DesignDef`/`TokenDef` (design.rs)
* `crates/omnilang-core/src/parser/design.rs` — string-based parser extended for `@themes`, `@default-theme`, per-theme token values
* `crates/omnilang-core/src/parser/top_level/store.rs` — removed `parse_theme_def()`
* `crates/omnilang-core/src/ir/design.rs` — `DesignIR`/`TokenIR` extended with theme fields
* `crates/omnilang-core/src/ir/lower/design.rs` — lowering passes theme fields through
* `omnilang/src/codegen/omniview/design.rs` — CSS codegen emits triple-layer theme blocks
* `omnilang/src/codegen/omniview/mod.rs` — JS runtime generation for `__omni_theme`

**Interface Changes:**
* `design { }` blocks gain `@themes`, `@default-theme` directives
* `token` declarations accept optional `<theme> "<value>"` pairs after default value
* `__omni_theme` global object injected into page scope at compile time (not a language symbol)
* `theme { }` top-level blocks are no longer valid syntax (parse error with migration hint)

## Consequences

### Positive

* **Single source of truth**: Design tokens and their theme variants are declared together — no more `ThemeStore` getter duplication
* **O(1) theme toggle**: One `data-theme` attribute change vs N `setProperty()` calls
* **Zero component boilerplate**: Components don't need `style: { --theme-*: getter }` bindings — CSS cascade propagates automatically
* **System preference support**: `@media (prefers-color-scheme: dark)` generated automatically for `"dark"` theme
* **Dead code eliminated**: ~150 lines of never-executed parser code removed
* **Type safety preserved**: Theme values are `FxHashMap<String, String>` in the IR — documented and structured, not ad-hoc

### Negative

* **`__omni_theme` is not a compiler-recognized symbol**: `.omni` source files cannot call `__omni_theme.toggle()` directly — requires hand-written JS bridge or a compiler built-in in a follow-up
* **Theme names are free-form strings**: `@themes` declares a list, but there's no validation that per-theme token values only reference declared themes (accepted as a low-priority gap)
* **Commit history is messy**: Cherry-pick conflicts across worktree branches produced a non-linear commit history (acceptance criteria: zero functional impact)

### Neutral

* **The `"dark"` theme name has special semantics**: Only `dark` gets `prefers-color-scheme` auto-detection — non-standard names like `"highContrast"` require manual activation
* **Design block parser is string-based**: Unlike the main token-based parser, `parse_design_body` works on raw strings — this creates an inconsistency but was already the case before this change

## Implementation Strategy

### Blast Radius

**Impact Scope**: Medium — affects the compiler pipeline from lexer through codegen for the `design` block feature path. No impact on component/view/store features.

**Affected Components**:
- Lexer/Parser/AST — `theme` keyword removal (surgical deletions)
- Parser/design.rs — new syntax additions (additive, no breaking changes to existing design syntax)
- IR/IR lower/codegen — new fields on existing types (backward-compat via `#[serde(default)]`)
- learn-omniview project — migration from manual to design-block theme system

**User Impact**: Zero for existing `.omni` files that don't use the `theme` keyword. Files using `theme` will get a parse error (previously were silently ignored).

**Risk Mitigation**:
- Full `#[serde(default)]` on all new IR/AST fields ensures existing serialized data doesn't break
- All existing tests pass unchanged (519 omnilang-core + 225 omnilang + integration tests)
- learn-omniview migration is a showcase of the new pattern, not a backward-compat requirement

### Reversibility

**Reversibility Level**: Medium

**Rollback Feasibility**:
- Removing the `theme` keyword is irreversible at the language level, but no known production usage exists
- The `@themes` syntax is purely additive — removing it returns to the previous design-only state
- Per-theme token values stored in `TokenIR.theme_values` — removing is safe (empty Vec = no change)

**Migration Paths**:
- **Forward Migration**: Add `@themes light, dark;` + per-theme values to existing design blocks
- **Rollback Migration**: Remove `@themes` declarations and per-theme values; delete `__omni_theme` runtime injection

### Sequencing & Timing

**Prerequisites**:
- [x] Phase 1: Remove `theme` keyword dead code (4 commits, Tasks 1-4)
- [x] Phase 2: Extend AST/parser for `@themes` + per-theme tokens (2 commits, Tasks 5-7)
- [x] Phase 3: Extend IR/lowering/codegen for theme fields (2 commits, Tasks 8-10)
- [x] Phase 4: CSS/JS codegen for theme runtime (2 commits, Tasks 11-12)
- [x] Phase 5: Integration test + docs + migration (4 commits, Tasks 13-15)

### Social Cost

**Learning Curve**: Low
- `@themes` and per-theme token values have straightforward syntax
- The transition from store-getter patterns is mechanical
- Documentation updated in `docs/omniview/10-design-system.md`

**Documentation Needs**:
- [x] `docs/omniview/10-design-system.md` — Section 4 replaced with `@themes` documentation
- [ ] Future: Add migration guide from old ThemeStore pattern to new design-block theme system

### Confidence Assessment

**Model Correctness Confidence**: High
- The CSS cascade model for theming is a well-established web pattern (CSS custom properties + `data-theme` attribute)
- The change removes complexity (no more per-property `setProperty()` effects)
- 800+ tests pass, including 2 new integration tests for the full pipeline

**Assumptions**:
1. No production `.omni` code uses the dead `theme` keyword — validated by grep across test projects
2. The `"dark"` name is the de-facto standard for dark mode — validated by web platform conventions

**Uncertainty Areas**:
- `__omni_theme` runtime is not callable from `.omni` source — needs a follow-up to add as a compiler built-in

## Implementation

The work was completed in 11 commits across 5 phases:

**Phase 1: Remove dead code** (4 commits)
* Remove `"theme"` keyword mapping from lexer
* Remove `ThemeDef`/`ThemeCategory` AST types
* Remove all parser consumer sites (9 files)
* Remove `TokenKind::Theme` enum variant

**Phase 2: Extend AST/parser** (2 commits)
* Add `themes`, `default_theme` fields to `DesignDef`; add `theme_values` to `TokenDef`
* Add `parse_themes_directive()`, `parse_default_theme_directive()`, per-theme value parsing in `parse_token()`
* 4 new parser tests

**Phase 3: Extend IR** (2 commits)
* Add `themes`, `default_theme` to `DesignIR`; add `theme_values: FxHashMap` to `TokenIR`
* Pass new fields through `lower_design()`

**Phase 4: Codegen** (2 commits)
* Triple-layer CSS: `:root` + `[data-theme]` + `@media (prefers-color-scheme)`
* `__omni_theme` JS runtime: `set()`, `toggle()`, `init()` with localStorage + system preference
* 4 new tests (2 CSS + 2 JS runtime)

**Phase 5: Verification** (4 commits)
* Full-pipeline integration tests
* Documentation replacement in `10-design-system.md`
* learn-omniview migration

## Alternatives Considered

### Alternative 1: Store-mediated theme with `__omni_theme` as a compiler built-in

This alternative would make `__omni_theme` a first-class language symbol that `.omni` code could reference directly:

```omni
action toggleTheme() {
    __omni_theme.toggle();
}
```

**Pros:**
* Theme actions can be called directly from `.omni` component code
* No hand-written JS bridge needed
* More ergonomic for application developers

**Cons:**
* Requires adding a compiler-recognized global symbol — more invasive pipeline change
* Blurs the line between language constructs and generated runtime artifacts
* The "bridge" problem goes both ways: any generated runtime helper needs the same treatment
* Rejected for v1 because it would require changes to multiple compiler passes (semantic analysis, type checking, IR lowering) for a single codegen helper

**Rejected in favor of**: The current approach where `__omni_theme` is codegen-only, with the understanding that a follow-up ADR can add it as a built-in if demand justifies it.

### Alternative 2: Keep the `theme` keyword and fix its codegen

**Pros:**
* Backward compatible with any `.omni` files using `theme`

**Cons:**
* No known usage (it was dead code from the start)
* A separate `theme` keyword creates conceptual confusion vs `design @themes`
* Maintaining both a `theme` keyword and `design @themes` would be confusing
* The `theme` keyword was a flat key-value structure with no class/variant support

**Rejected in favor of**: Removing dead code and consolidating all design/theme concepts in `design` blocks.

### Alternative 3: Keep the store getter + `setProperty()` pattern and optimize it

**Pros:**
* No language changes needed
* Existing code continues to work

**Cons:**
* Fundamental O(n) scalability problem — every themed variable needs a separate effect
* No CSS cascade — changes don't propagate to children automatically
* System preference detection must be written manually per application
* Duplicate color definitions are inevitable

**Rejected in favor of**: The CSS cascade approach is architecturally correct — theming is a presentation concern, not an application-logic concern.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer (YAGNI Guardian)
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This decision scores high on necessity (removing dead code that could confuse users) and medium on complexity (the `@themes` syntax is a natural extension of the existing `design` block DSL). The ratio is well within the balanced threshold.

**Decision Challenge**: "Replace the dead `theme` keyword with `@themes` in design blocks, using CSS cascade for theme switching"

**Necessity Assessment**: 9/10
- **Current need**: HIGH — the `theme` keyword was dead code; theme switching existed only as a manual, unscalable pattern
- **Future need**: Certain — every design system eventually needs theme support
- **Cost of waiting**: Increasing — more projects adopting `design` blocks would encounter the gap and build workarounds
- **Evidence of need**: learn-omniview already had 60+ lines of manual ThemeStore color getters

**Complexity Assessment**: 5/10
- **Added complexity**: Low — `@themes` syntax follows existing `@dim` pattern; per-theme values are straightforward key-value pairs
- **Maintenance burden**: Low — new fields on existing types; no new abstractions
- **Learning curve**: Low — CSS cascade theming is a standard web pattern
- **Dependencies introduced**: Zero new crates/libraries

**Alternative Analysis**: 
- The do-nothing option (keep dead code + manual pattern) is not viable — the dead code actively misleads users
- Treating `__omni_theme` as a compiler built-in was deferred as too high complexity for v1
- The chosen approach is the minimal viable change: extend the design block DSL with theme-aware token values

**Simpler Alternative Proposal**: None — the chosen approach is already the simplest viable solution. Removing dead code and adding theme awareness to the existing design block DSL is the minimal change that addresses the real requirements.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 5/10
- **Ratio**: 0.56 (well below the 1.5 threshold) — clear win

**Overall Assessment**: Appropriate engineering for a concrete, current need. The approach correctly delegates theme switching to the CSS cascade (the proper layer for presentation concerns) rather than JS signal-cascade. The deferred work (making `__omni_theme` a compiler built-in) is a responsible deferral — the current approach already delivers the core benefit.

## Validation

**Acceptance Criteria:**
- [x] All `theme` keyword references removed from compiler — verified by grep (zero references)
- [x] `@themes light, dark;` syntax correctly parsed — 4 new parser tests pass
- [x] Per-theme token values survive pipeline (parser → AST → IR → codegen) — 2 integration tests pass
- [x] CSS output includes `:root` + `[data-theme]` + `@media` blocks — 2 CSS codegen tests pass
- [x] `__omni_theme` JS runtime generated when designs declare themes — 2 JS runtime tests pass
- [x] No theme runtime generated when no themes declared — backward compatibility verified
- [x] learn-omniview builds and renders correctly — manual build verification

**Testing Approach:**
* Unit tests: 4 new parser tests (`@themes`, `@default-theme`, per-theme token values, multiple values)
* Integration tests: 2 full-pipeline tests (themed + non-themed)
* Codegen tests: 4 tests (CSS + JS runtime) in omnilang codegen
* Existing test suite: 519 omnilang-core + 225 omnilang = 744+ tests all pass (no regressions)
* Workspace test suite: 800+ tests pass

## References

* [OmniView Style System Architecture Review](../reviews/compiler-architect-omniview-style-system.md) — Section C2 "Themed switching entirely absent" is the root finding
* [Implementation Plan](../../docs/superpowers/plans/2026-07-28-design-theme-system.md) — 15-task implementation plan
* [Speculative framework comparison](https://en.wikipedia.org/wiki/CSS_custom_properties#Theme_switching) — CSS custom property theming is a well-established pattern
