# ADR-045: Learn-OmniView Multi-Theme Capability Showcase Redesign

## Status

Accepted

## Context

The learn-omniview project (`test/projects/learn-omniview/`) serves as the primary demonstration for OmniLang/OmniView capabilities. By mid-2026, it had grown to 10 tabs and ~90 files without a coherent narrative, and several design limitations gave a misleading impression of OmniLang's capabilities:

1. **Binary theme lock-in**: `@themes light, dark` and a hardcoded ThemeStore suggested the framework only supports two themes. In reality, the design block `@themes` directive supports **N themes** with per-token per-theme values — but no demo showed this.

2. **10-tab sprawl**: Pages were organized by incremental file additions rather than by developer-facing capability domains. Related concepts (CSS/Settings/Themes, Language/Supplement) were split across tabs, diluting each page's depth.

3. **Under-demonstrated power**: Element Ref System (ADR-036), multi-theme design tokens, and the `__omni_theme` runtime were present but not shown in a cohesive, impressive way. The ElementRefDemo had no drag constraint variations.

4. **Parser edge cases**: During the redesign, several OmniLang parser limitations were discovered and worked around:
   - `@` prefix in text content (e.g., `@drag-constraints` in a `<p>` element) was parsed as an attribute
   - `@drag` motion trigger did not exist — only `@dragHorizontal` and `@dragVertical` were valid
   - Self-closing XHTML-style tags (`<input />`) caused parser failures in certain contexts

## Decision Drivers

- **Correct misimpression**: The demo should show that OmniLang supports N themes, not just light/dark
- **Capability narrative**: Organize by domain (UI, Motion, Design, State, Language, Compiler) rather than file count
- **Depth over breadth**: Fewer tabs with richer, more focused content per page
- **Showcase, not tutorial**: Target experienced developers evaluating the framework
- **Parser compatibility**: All `.omni` source must compile successfully with the existing parser — work around known limitations rather than fixing the parser
- **Zero regression**: All existing 530+ tests must pass unchanged
- **Preserve ADR-044 contract**: The `mode` signal name must remain unchanged for `__omni_theme` auto-sync

## Decision

**1. Expand the design block to 4 themes.** `@themes light, dark, sepia, highContrast` with per-token values for all 18 color tokens. Color values chosen for distinct visual character:

| Theme | Primary | Surface | Text | Intent |
|-------|---------|---------|------|--------|
| Light | #6366f1 (indigo) | #ffffff | #111827 | Clean, modern |
| Dark | #a5b4fc (soft indigo) | #111827 | #f9fafb | Low-contrast dark |
| Sepia | #8B4513 (saddle brown) | #F5E6C8 | #3E2723 | Warm, paper-like |
| High Contrast | #ffff00 (yellow) | #000000 | #ffffff | AAA accessible |

**2. Refactor ThemeStore for N-theme support.** Remove binary light/dark logic from `toggle()` and `setMode()`. Replace with data-driven cycling (`light → dark → sepia → highContrast → light`). Remove the `nextLabel` getter (it was a binary concept). Keep the `mode` signal name unchanged for ADR-044 sync.

**3. Restructure from 10 tabs to 7 capability domains.**
- Overview (new capability matrix) → UI Components (merged) → Motion (enhanced) → Design & Theme (new centerpiece) → State (extracted) → Language (merged) → Compiler (unchanged)

**4. Create 4 new demo components:**
- `ThemeSourceDemo.omni` — `@themes` syntax showcase with generated CSS output
- `DesignTokenPanel.omni` — Live visualizer of all 18 token types via `var(--name)`
- `VariantShowcase.omni` — Button variants across all 4 themes
- `CapabilityCard.omni` — Reusable grid card for the Overview page

**5. Work around parser limitations:**
- Remove `@` prefix from text content in hint paragraphs
- Use only `@dragHorizontal`/`@dragVertical`, never `@drag`
- Use `<input></input>` instead of `<input />`
- Remove HTML comments (`<!-- -->`) in new components

**Architectural Components Affected:**
- `test/projects/learn-omniview/src/design.omni` — theme expansion
- `test/projects/learn-omniview/src/stores/ThemeStore.omni` — N-theme refactoring
- `test/projects/learn-omniview/src/components/ThemeSwitcher.omni` — 4-card selector
- 7 new files, 8 deleted files, 5 modified files
- `omnilang/tests/cross_target.rs` — updated file references
- `tests/fixtures/golden/MANIFEST.md` — updated manifest

**Interface Changes:**
- `ThemeStore.nextLabel` getter removed
- `ThemeStore.toggle()` now cycles 4 themes instead of binary
- `ThemeStore.setMode()` validates against 4 theme names
- 7 deleted page exports: `HomePage`, `SettingsPage`, `SupplementPage`, `InteractivePage`, `CssPage`, `EventsPage`, `TodoPage`
- 4 new page exports: `Overview`, `UiComponents`, `DesignTheme`, `StatePage`
- `main.omni`: `activeTab` default changed from `"home"` to `"overview"`

## Consequences

### Positive

- **Multi-theme capability visually demonstrated**: 4 themes with distinct personalities (clean/dark/warm/high-contrast) show N-theme support is real
- **7-tab narrative is cleaner**: Each domain is a focused page, not a file listing
- **Design & Theme page as centerpiece**: 5 demo sections (Selector, Source, Tokens, Variants, CSS Vars) create a compelling feature showcase
- **Parser limitations documented**: Working around `@` in text, unsupported `@drag`, and self-closing tags informs future compiler development
- **ElementRefDemo enhanced**: Drag constraint variations (elastic 0.5, elastic 0, vertical) demonstrate the API's flexibility
- **Zero test regression**: All 530+ tests pass with updated file references
- **Golden manifest expanded**: 53 files (up from 51) with new design system coverage

### Negative

- **ThemeSwitcher has class naming inconsistency**: CSS class `themeCard` (camelCase) is used in view object bindings while all other classes use kebab-case. This is a consequence of the parser treating hyphens in object literal keys as subtraction operators.
- **Dead code remains**: `Dashboard.omni` page is orphaned (previously imported by deleted `HomePage.omni`). Left in place as it may be reused in future page expansions.
- **ThemeSourceDemo has cosmetic deviations**: CSS layer comments and some formatting details from the spec were dropped due to parser limitations with `<code>` nesting in `<pre>` blocks.

### Neutral

- **Theme color values are opinionated**: Sepia and High Contrast colors were chosen for visual distinctiveness rather than WCAG compliance. The `dark` theme is the only one with `prefers-color-scheme` auto-detection (per ADR-016).
- **4 themes as default** could be seen as over-engineering — the design block supports arbitrarily many themes, and 4 is enough to make the point without bloat.
- **Component duplication**: `InlineStoreDemo` and `ComputedFnFormDemo` appear on both the Language page and the State page. This is intentional — they demonstrate different concepts in each context (language syntax vs. state management pattern).

## Implementation Strategy

### Blast Radius

**Impact Scope**: Medium — confined to the `test/projects/learn-omniview/` project tree and two test reference files (`cross_target.rs`, `MANIFEST.md`). No changes to the compiler, runtime, or any production code.

**Affected Components**:
- `test/projects/learn-omniview/` — 15 modified files, 7 new files, 8 deleted files
- `omnilang/tests/cross_target.rs` — updated file references (2 lines)
- `tests/fixtures/golden/MANIFEST.md` — 5 line updates, 3 new entries
- `build/debug/` — regenerated build output

**User Impact**: Zero for production users. Demo developers see revised page structure and 4 themes.

**Risk Mitigation**:
- Full test suite pass (530 tests) before merge
- Build verification: `omnic build --target omniview` compiles cleanly
- The `mode` signal name is preserved — no breakage of ADR-044 sync

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**:
- All changes are to non-production demo files and test references
- Reverting the 14 commits restores the original 10-tab layout
- Deleted page files remain in git history

### Sequencing & Timing

**Prerequisites**:
- [x] ADR-016 design theme system (CSS cascade approach)
- [x] ADR-044 ThemeStore ↔ `__omni_theme` auto-sync

**Readiness Assessment**: Implemented and verified. All tasks complete.

### Social Cost

**Learning Curve**: Low
- Developers familiar with the old 10-tab layout only need to learn 7 tab names
- The new organization follows logical capability domains

**Documentation Needs**:
- [x] Design spec: `docs/superpowers/specs/2026-07-30-learn-omniview-redesign-design.md`
- [x] Implementation plan: `docs/superpowers/plans/2026-07-30-learn-omniview-redesign.md`
- [ ] Future: Update `docs/omniview/` reference documentation to reference new demo pages

### Confidence Assessment

**Model Correctness Confidence**: High
- The multi-theme approach reuses the existing `@themes` design block syntax — no new language features
- The page restructure is purely compositional (reorganizing imports)
- All test references updated and verified

**Assumptions**:
1. 4 themes (light/dark/sepia/highContrast) are sufficient to demonstrate N-theme capability — **Validation**: Confirmed by distinct visual appearance of each theme
2. The parser limitations (`@` in text, `@drag`, self-closing tags) are edge cases not affecting normal usage — **Validation**: All 530+ tests pass, and the limitations were discovered only through this demo expansion

**Edge Cases**:
- `@` character in view DSL text content is parsed as attribute start — **Workaround**: Avoid `@` in text, or escape it
- Self-closing tags (`<input />`) fail in some parser contexts — **Workaround**: Use full closing tag `<input></input>`

## Implementation

**Phase 1: Theme System Refactor** (7 tasks, commits `71a48094..03bf8bfb`)
* Expand `design.omni`: 4 themes with 18 per-token color values
* Rewrite `ThemeStore.omni`: N-theme cycling toggle/setMode
* Rewrite `ThemeSwitcher.omni`: 4 visual theme cards with color swatches
* Create `ThemeSourceDemo.omni`, `DesignTokenPanel.omni`, `VariantShowcase.omni`
* Update `ThemeCssVarsDemo.omni`: remove `nextLabel` dependency

**Phase 2: Page Restructure** (6 tasks, commits `acb70e3f..cd5723f6`)
* Create `CapabilityCard.omni` + `Overview.omni`
* Create `UiComponents.omni`, `DesignTheme.omni`, `StatePage.omni`
* Merge `SupplementPage` into `LanguagePage.omni`
* Enhance `ElementRefDemo.omni` with drag constraint variations

**Phase 3: Navigation & Cleanup** (4 tasks, commits `708fe56a..c97af3ca`)
* Update `TabBar.omni`: 10 tabs → 7
* Update `main.omni`: new routing with 7 pages
* Update `Header.omni`: show active theme via `ThemeStore.modeLabel`
* Delete 7 old pages, 1 orphaned page

**Phase 4: Test & Polish** (commits `4efd615d..9ca5def0`)
* Fix `cross_target.rs`: update file references
* Fix `golden_parse.rs` / `MANIFEST.md`: update golden file manifest
* Remove accidentally committed snapshot files

## Alternatives Considered

### Alternative 1: Keep 10 tabs, incrementally add multi-theme demo

Keep the existing 10-tab structure and only modify the Settings page to add 4-theme support.

**Pros:**
* Minimal diff — only change ThemeStore and ThemeSwitcher
* No page reorganization risk

**Cons:**
* Does not fix the core problem (10-tab sprawl, weak narrative)
* Multi-theme demo would be buried in Settings, not featured
* Missed opportunity to use the redesign as a showcase of OmniLang capabilities
* The "supplement" and "language" split would remain confusing

**Rejected in favor of**: A full redesign that reorganizes around capability domains and features multi-theme as the centerpiece.

### Alternative 2: Partial restructure (merge only, no new pages)

Merge the 10 tabs into 7 by consolidating imports, but create no new pages or components.

**Pros:**
* Less code to write and maintain
* Faster implementation

**Cons:**
* Leaves the Design & Theme page without a proper centerpiece
* No ThemeSourceDemo, DesignTokenPanel, or VariantShowcase
* Overview page would lack the capability matrix
* Misses the opportunity to deliver a "wow" experience for evaluators

**Rejected in favor of**: 4 new components that substantially improve the showcase quality.

### Alternative 3: Fix the parser instead of working around limitations

Fix the OmniLang parser to properly handle `@` in text content, `@drag` attribute, and self-closing tags.

**Pros:**
* Cleaner source code without workarounds
* Better developer experience for all .omni authors

**Cons:**
* Parser changes have broader blast radius (affects all .omni files)
* Requires deep understanding of the parser internals
* Dramatically increases scope of this change
* These are edge cases — the parser handles 530+ tests correctly

**Rejected in favor of**: Working around the limitations and documenting them. Parser fixes can be addressed in a focused follow-up.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: Medium complexity — the decision touches 30+ files and 14 commits, but each change is mechanical (editing design tokens, rewriting imports, updating test references). The complexity is in the scope, not the depth.

**Decision Challenge**: "Expand learn-omniview from 2-theme/10-tab layout to 4-theme/7-tab capability showcase"

**Necessity Assessment**: 7/10
- **Current need**: HIGH — the existing demo gives a misleading impression of OmniLang's theme capabilities (binary only). Evaluators would reasonably assume N-theme is not supported.
- **Future need**: HIGH — as OmniLang adoption grows, a compelling demo is the primary marketing asset.
- **Cost of waiting**: Medium — every evaluator who sees only 2 themes reinforces the wrong impression.
- **Evidence of need**: The user explicitly identified this gap after E2E testing the existing demo.

**Complexity Assessment**: 5/10
- **Added complexity**: Moderate — 7 new files, 8 deleted files, 14 commits. But no new compiler code, no new runtime code, no new abstractions.
- **Maintenance burden**: Low — the new structure is simpler (7 tabs vs 10).
- **Learning curve**: Near zero — same project structure, same component patterns.
- **Dependencies introduced**: Zero — all new components use existing APIs.

**Alternative Analysis**: 
- Alternative 1 (keep 10 tabs, incremental theme change) is simpler but fails to create a compelling showcase.
- Alternative 3 (fix the parser) is higher complexity with broader risk.
- The chosen approach is the middle ground: scope is contained to the demo project, but the result is substantially improved.

**Simpler Alternative Proposal**: None — the chosen approach IS the simplest viable improvement to achieve the stated goals. A pure "change 2 themes to 4" without the page restructure would leave the 10-tab sprawl problem unsolved.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 7/10
- **Complexity**: 5/10
- **Ratio**: 0.71 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: Appropriate engineering. The decision improves the demo's accuracy and showcase quality with zero impact on production code. The scope is well-bounded to the demo project, test references, and documentation.

## Validation

**Acceptance Criteria:**
- [x] `omnic build --target omniview` compiles cleanly
- [x] All 7 tabs render with content
- [x] All 4 themes (light/dark/sepia/highContrast) switch correctly via ThemeSwitcher
- [x] `data-theme` attribute updates correctly on theme switch (ADR-044 sync preserved)
- [x] All 530+ tests pass (cargo test)
- [x] `__omni_theme` auto-sync works — `mode` signal name unchanged
- [x] No compiler warnings or errors
- [x] Golden parse test covers all 53 manifest files

**Testing Approach:**
* Standard `cargo test` (530+ tests across all crates)
* `omnic build --target omniview` from project directory
* Manual verification of all 7 pages × 4 theme combinations

## References

* [ADR-016: Design Theme System — CSS Cascade Over JS Signal Cascade](./ADR-016-design-theme-system.md)
* [ADR-044: ThemeStore Mode ↔ __omni_theme Auto-Sync](./ADR-044-theme-store-mode-omni-theme-auto-sync.md)
* [OmniLang CSS Variables Memory](../../../../C%3A/Users/XHT/.claude/projects/D--space-OmniLang/memory/omnilang-css-variables.md)
* [Learn-OmniView Refactoring Memory](../../../../C%3A/Users/XHT/.claude/projects/D--space-OmniLang/memory/learn-omniview-refactoring.md)
* [Design Spec](../../docs/superpowers/specs/2026-07-30-learn-omniview-redesign-design.md)
* [Implementation Plan](../../docs/superpowers/plans/2026-07-30-learn-omniview-redesign.md)
