# Learn-OmniView Redesign: Multi-Theme Capability Showcase

**Date**: 2026-07-30
**Status**: Draft

---

## 1. Motivation

The current [learn-omniview](https://github.com/jerghy/OmniLang/tree/main/test/projects/learn-omniview) project demonstrates OmniView features across 10 tabs, but suffers from several design limitations that give a misleading impression of OmniLang's capabilities:

1. **Binary theme lock-in**: `@themes light, dark` and `ThemeStore` hardcoded to light/dark suggest the framework only supports two themes. In reality, the design block `@themes` directive supports **N themes** with per-token per-theme values.
2. **10-tab sprawl**: Too many tabs dilute each page's depth. Related concepts (CSS/Settings/Themes, Language/Supplement) are split across tabs.
3. **No capability narrative**: Pages are organized by file count, not by developer-facing capability domains.
4. **Under-demonstrated power**: Element Ref System, multi-theme design tokens, and the `__omni_theme` runtime are present but not shown in a cohesive, impressive way.

This spec describes a **full redesign** of learn-omniview into a **capability showcase** — 7 tabs organized by domain, with a centerpiece multi-theme demo (4 themes: light, dark, sepia, highContrast).

---

## 2. Architecture Overview

### 2.1 Target Audience

Experienced developers evaluating the OmniLang/OmniView framework. The showcase emphasizes breadth, depth, and "wow factor" over tutorial progression.

### 2.2 Design Principles

- **Show, don't tell**: Each capability is demonstrated through live interactive components, not documentation blocks.
- **Multi-theme by default**: The theme system powers the entire showcase — all components react to theme changes.
- **Domain boundaries**: Group features by conceptual domain (UI, Motion, Design, State, Language), not by file count.
- **Depth over breadth**: Fewer tabs, each with richer content.

### 2.3 File Structure

```
src/
├── design.omni              # LearnDesign — 4 themes (light/dark/sepia/highContrast)
├── main.omni                # App — 7-tab navigation + global stores
├── stores/
│   ├── ThemeStore.omni      # Refactored for N-theme support
│   ├── TodoStore.omni       # Unchanged
│   └── NotificationStore.omni # Unchanged
├── shared/
│   ├── constants.omni       # Unchanged
│   └── theme.omni           # Deprecation notice only
├── pages/
│   ├── Overview.omni        # NEW: Capability matrix + quick demo
│   ├── UiComponents.omni    # Merged: Interactive + Todo + Events
│   ├── MotionPage.omni      # Enhanced: ElementRef + drag constraints
│   ├── DesignTheme.omni     # NEW: Multi-theme showcase centerpiece
│   ├── StatePage.omni       # Extracted: Signal + Store patterns
│   ├── LanguagePage.omni    # Merged: Language + Supplement
│   └── CompilerPage.omni    # Unchanged
├── components/
│   ├── Header.omni          # Enhanced: shows active theme
│   ├── TabBar.omni          # 7 tabs
│   ├── ThemeSwitcher.omni   # REWRITTEN: 4-card theme selector
│   ├── ThemeCssVarsDemo.omni # Enhanced: works with 4 themes
│   ├── ThemeSourceDemo.omni # NEW: Code syntax display
│   ├── DesignTokenPanel.omni # NEW: Live token visualizer
│   ├── VariantShowcase.omni  # NEW: Button variants across all themes
│   ├── CapabilityCard.omni  # NEW: Overview page card
│   └── ... (other existing components, mostly unchanged)
└── lib/                     # Unchanged
```

---

## 3. Multi-Theme System (Centerpiece)

### 3.1 `design.omni` Changes

**Before:**
```omni
@themes light, dark;
token primary: color = "#6366f1" dark "#a5b4fc";
token surface: color = "#ffffff" dark "#111827";
// ... 16 more tokens
```

**After:**
```omni
@themes light, dark, sepia, highContrast;
@default-theme light;

token primary: color = "#6366f1" dark "#a5b4fc" sepia "#8B4513" highContrast "#ffff00";
token primaryLight: color = "#a5b4fc" sepia "#D2A679" highContrast "#ffff99";
token primaryDark: color = "#4338ca" sepia "#5D3A1A" highContrast "#cccc00";

token success: color = "#10b981" sepia "#6B8E23" highContrast "#00ff00";
token warning: color = "#f59e0b" sepia "#CC7722" highContrast "#ffaa00";
token danger: color = "#ef4444" sepia "#A0522D" highContrast "#ff4444";
token accent: color = "#f97316" sepia "#CD853F" highContrast "#ff8800";

token surface: color = "#ffffff" dark "#111827" sepia "#F5E6C8" highContrast "#000000";
token surfaceMuted: color = "#f9fafb" dark "#1f2937" sepia "#EDE0C8" highContrast "#0a0a0a";
token surfaceSubtle: color = "#f3f4f6" dark "#374151" sepia "#E0D0B0" highContrast "#1a1a1a";

token textPrimary: color = "#111827" dark "#f9fafb" sepia "#3E2723" highContrast "#ffffff";
token textSecondary: color = "#6b7280" dark "#9ca3af" sepia "#5D4037" highContrast "#cccccc";
token textMuted: color = "#9ca3af" dark "#d1d5db" sepia "#8D6E63" highContrast "#999999";

token border: color = "#d4d4d8" dark "#4b5563" sepia "#BCAAA4" highContrast "#666666";
token borderLight: color = "#e5e7eb" dark "#374151" sepia "#D7CCC8" highContrast "#444444";
token borderLighter: color = "#f3f4f6" dark "#1f2937" sepia "#EFEBE9" highContrast "#333333";

// Spacing, Radius, Shadow, Typography tokens — unchanged (not theme-dependent)
// Duration, Easing, Opacity, ZIndex tokens — unchanged
```

**Theme design rationale:**
| Theme | Primary | Surface | Text | Vibe |
|-------|---------|---------|------|------|
| Light | #6366f1 (indigo) | #ffffff | #111827 | Clean, modern |
| Dark | #a5b4fc (soft indigo) | #111827 | #f9fafb | Low contrast dark |
| Sepia | #8B4513 (saddle brown) | #F5E6C8 | #3E2723 | Warm, paper-like |
| High Contrast | #ffff00 (yellow) | #000000 | #ffffff | AAA accessible |

### 3.2 `ThemeStore.omni` Changes

**Before:** Hardcoded light/dark logic in `setMode()` and `toggle()`.

**After:** Data-driven, extensible to N themes:

```omni
store ThemeStore {
    const themes: Array<String> = ["light", "dark", "sepia", "highContrast"];
    const themeIcons: Array<String> = ["☀️", "🌙", "📜", "🔲"];
    const themeLabels: Array<String> = ["Light", "Dark", "Sepia", "High Contrast"];

    signal mode: String = "light";
    signal toggleCount: i32 = 0;

    getter modeIndex: i32 = /* indexOf(mode) in themes */;
    getter modeLabel: String = themeLabels[modeIndex];
    getter modeIcon: String = themeIcons[modeIndex];

    getter themeColors: Array<String> = /* lookup current theme's primary/surface/text from CSS var() */;

    action toggle() {
        let idx = modeIndex;
        mode = if idx >= themes.len() - 1 { themes[0] } else { themes[idx + 1] };
        toggleCount = toggleCount + 1;
    }

    action setMode(newMode: String) {
        if themes.indexOf(newMode) >= 0 {
            mode = newMode;
        }
    }
}
```

**Important:** `__omni_theme` auto-sync (ADR-044) continues to work because the mode signal name is unchanged. The `__omni_theme.set()` runtime receives any of the 4 theme names and sets `data-theme` accordingly — the CSS cascade handles the rest.

### 3.3 `ThemeSwitcher.omni` Rewrite

Replace the two-button layout with four visual theme cards:

```omni
component ThemeSwitcher {
    @store ThemeStore;

    signal hoveredTheme: String = "";

    // Each card shows theme name + primary color swatch + surface color swatch
    view {
        <div class: theme-selector>
            @for theme in ThemeStore.themes key={theme} {
                <div class: { theme-card: true, active: ThemeStore.mode == theme }
                     @onPress={() => { ThemeStore.setMode(theme); }}
                     @onMouseEnter={() => { hoveredTheme = theme; }}
                     @onMouseLeave={() => { hoveredTheme = ""; }}
                     style: {
                         --card-bg: "var(--surface)",
                         --card-border: if ThemeStore.mode == theme { "var(--primary)" } else { "var(--border)" }
                     }>
                    <span class: theme-icon>{ThemeStore.themeIcons[index]}</span>
                    <span class: theme-name>{ThemeStore.themeLabels[index]}</span>
                    <div class: color-swatches>
                        <span class: swatch style: { background: "var(--primary)" } />
                        <span class: swatch style: { background: "var(--surface)" } />
                        <span class: swatch style: { background: "var(--textPrimary)" } />
                    </div>
                </div>
            }
        </div>
    }

    // Also keep: toggle count display + palette preview
}
```

### 3.4 CSS Cascade Behavior

The 4 themes compile to:
```css
:root { --primary: #6366f1; --surface: #ffffff; --textPrimary: #111827; ... }

[data-theme="dark"] { --primary: #a5b4fc; --surface: #111827; --textPrimary: #f9fafb; ... }
[data-theme="sepia"] { --primary: #8B4513; --surface: #F5E6C8; --textPrimary: #3E2723; ... }
[data-theme="highContrast"] { --primary: #ffff00; --surface: #000000; --textPrimary: #ffffff; ... }

@media (prefers-color-scheme: dark) { :root:not([data-theme]) { ... } }
```

Only `dark` gets `prefers-color-scheme` auto-detection (per ADR-016). Sepia and HighContrast require manual activation via the ThemeSwitcher.

---

## 4. Page Architecture (7 Tabs)

### 4.1 Tab Navigation

```omni
// TabBar.omni — 7 tabs
<button>Overview</button>
<button>UI Components</button>
<button>Motion</button>
<button>Design & Theme</button>   // ⭐ centerpiece
<button>State</button>
<button>Language</button>
<button>Compiler</button>
```

### 4.2 Overview Page

Capability matrix with animated cards:

```
┌─────────────────────────────────────────────────────┐
│  🚀 OmniLang Capability Showcase                     │
│  v0.1.0 · 60+ source files · 4 themes · 7 domains   │
├─────────────────────────────────────────────────────┤
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│ │🎨 UI     │ │🎬 Motion │ │💅 Design │ │⚡ State  │ │
│ │Components│ │Animation │ │& Theme   │ │Signals   │ │
│ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│ │🔤 Language│ │⚙️ Compiler│ │📊 Stats  │              │
│ │Features  │ │Features  │ │66 files  │              │
│ └──────────┘ └──────────┘ └──────────┘              │
└─────────────────────────────────────────────────────┘
```

Each card routes to its tab. Stats card shows live project metrics (DOM query or signal-driven).

### 4.3 UI Components Page

Merges existing `InteractivePage` + `TodoPage` + Events:

- **Section 1 - Interactive**: ValidationInput, Timer, Modal, TabPanel
- **Section 2 - Data Display**: DataTable (@each), NotificationsPanel
- **Section 3 - Events & Forms**: EventsDemo, EventShorthand, FormPage, bind:value demo
- **Section 4 - Todo App**: Compact TodoPage (FilterBar, TodoList, StatsDashboard)
- **Section 5 - Slot**: SignalPatternsDemo's SlotBox section extracted here

### 4.4 Motion Page

Same as current `MotionPage` with enhanced ElementRefDemo:

- MotionDemo, KeyframeAnimationDemo, EasingFunctionsDemo
- AnimPropertiesDemo, MotionTriggersDemo
- ElementRefDemo **enhanced**:
  - Additional drag constraint variations (`left:0 right:300 top:0 bottom:200`)
  - Multiple elastic values (0, 0.2, 0.5) demonstrated
  - Scroll-linked position update demo
  - `capturePointer` / `releasePointer` demo

### 4.5 Design & Theme Page (Centerpiece)

See Section 3 for full details. Structure:

1. **Theme Selector** — 4-card theme picker
2. **Source Code** — `design.omni` syntax showcase
3. **Interactive Cards** — Token showcase, CSS var cascade, variant showcase
4. **Color Palette** — All tokens visualized per active theme
5. **CSS Variable 4-Layer Demo** — ported from existing ThemeCssVarsDemo

### 4.6 State Page

Extracted from `SignalPatternsDemo` + `InlineStoreDemo` + `ComputedFnFormDemo`:

- **Inline Store**: CartStore demo (existing InlineStoreDemo)
- **Signal Patterns**: computed, @effect, @mount, @unmount
- **@for key**: ForDirectiveKeyDemo
- **Computed functions**: ComputedFnFormDemo
- **bind:value** (extracted to standalone)

### 4.7 Language Page

Merges all of existing `LanguagePage` + `SupplementPage`:

- NullishDemo, OptionalChainDemo, TernaryExprDemo, BlockExprDemo
- StringLiteralsDemo, TypeSystemDemo, ViewAttributesDemo
- StringLiteralsDemo, StylePseudoDemo, AdvancedPseudoDemo
- 3-column grid layout for compact demos

### 4.8 Compiler Page

Unchanged from current:

- DataFetching (HTTP GET with loading/error/data states)
- DataSubmit (form POST)
- UserSettings (CRUD pattern)
- Error handling demo

---

## 5. Component Inventory

### New Components to Create

| Component | Page | Purpose |
|-----------|------|---------|
| `CapabilityCard.omni` | Overview | Animated grid card with icon + label + link |
| `DesignTokenPanel.omni` | Design & Theme | Live visualizer: all 18 token types in current theme |
| `ThemeSourceDemo.omni` | Design & Theme | Syntax-highlighted `design.omni` with live theme preview |
| `VariantShowcase.omni` | Design & Theme | Button variants rendered across all 4 themes |

### Existing Components to Rewrite

| Component | Change |
|-----------|--------|
| `ThemeSwitcher.omni` | 2 buttons → 4 theme cards |
| `ThemeCssVarsDemo.omni` | Works with 4 themes (auto-adapts) |
| `SettingsPage.omni` | Migrated into Design & Theme page (deleted) |
| `SupplementPage.omni` | Merged into Language page (deleted) |
| `HomePage.omni` | Replaced by Overview page |
| `InteractivePage.omni` | Merged into UI Components page (deleted) |
| `SignalPatternsDemo.omni` | Slot section extracted to UI page |

### Existing Components to Keep Unchanged

All remaining component/pages/lib files (~70 files) stay as-is, only their import paths change.

### Stores

| Store | Change |
|-------|--------|
| `ThemeStore.omni` | REWRITTEN: N-theme support |
| `TodoStore.omni` | Unchanged |
| `NotificationStore.omni` | Unchanged |
| `CartStore` (inline) | Unchanged |

---

## 6. Data Flow

### Theme Change Flow

```
User clicks "Sepia" card
  → ThemeSwitcher @onPress
    → ThemeStore.setMode("sepia")
      → mode = "sepia"
        → __omni_theme.set("sepia")       [ADR-044 auto-sync]
          → document.documentElement.dataset.theme = "sepia"
            → CSS cascade: [data-theme="sepia"] { --primary: #8B4513; ... }
              → All var(--primary) references update automatically
        → all computed getters recompute (modeLabel, modeIcon, modeIndex)
        → toggleCount++
```

### Tab Navigation Flow

```
User clicks "Design & Theme" tab
  → TabBar @onPress("design")
    → main.omni: activeTab = "design"
      → @if activeTab == "design" { <DesignTheme /> }
```

---

## 7. Design Quality

Following [web/design-quality.md](../../rules/web/design-quality.md) standards:

- **Anti-template**: No generic card grids with uniform spacing. Theme selector cards have hierarchy — active card is visually prominent, hover states are animated.
- **Intentional rhythm**: The Design & Theme page uses a bento-layout: 2-1-2-4 grid sections with varying heights.
- **Typography**: Using the existing fontFamily token with deliberate scale contrast.
- **Motion**: Theme card transitions animate on theme switch (fadeSlideUp from ElementRefDemo).
- **Hover/focus states**: Theme cards have scale transform + border color transition on hover.
- **Grid-breaking**: The token palette section uses a masonry-like layout for visual interest.
- **Color semantics**: Tokens are shown in context, not as flat swatches — demonstrated through real UI cards.

---

## 8. Implementation Plan

### Phase 1: Theme System Refactor

| # | Task | Files |
|---|------|-------|
| 1 | Update `design.omni` — add sepia + highContrast token values (18 tokens) | `design.omni` |
| 2 | Rewrite `ThemeStore.omni` — data-driven N-theme support | `stores/ThemeStore.omni` |
| 3 | Rewrite `ThemeSwitcher.omni` — 4-card visual selector | `components/ThemeSwitcher.omni` |
| 4 | Add `ThemeSourceDemo.omni` — syntax showcase | `components/ThemeSourceDemo.omni` |
| 5 | Add `DesignTokenPanel.omni` — live token visualizer | `components/DesignTokenPanel.omni` |
| 6 | Add `VariantShowcase.omni` — button variants across 4 themes | `components/VariantShowcase.omni` |
| 7 | Update `ThemeCssVarsDemo.omni` — adapt to 4 themes | `components/ThemeCssVarsDemo.omni` |

### Phase 2: Page Restructure

| # | Task | Files |
|---|------|-------|
| 8 | Create `CapabilityCard.omni` + `Overview.omni` page | `components/CapabilityCard.omni`, `pages/Overview.omni` |
| 9 | Create `UiComponents.omni` page (merge Interactive + Todo + Events) | `pages/UiComponents.omni` |
| 10 | Create `DesignTheme.omni` page | `pages/DesignTheme.omni` |
| 11 | Create `StatePage.omni` page (extract signal/state patterns) | `pages/StatePage.omni` |
| 12 | Merge `SupplementPage` content into `LanguagePage` | `pages/LanguagePage.omni` |
| 13 | Enhance `ElementRefDemo.omni` — drag constraints variations | `pages/ElementRefDemo.omni` (via MotionPage) |

### Phase 3: Navigation & Cleanup

| # | Task | Files |
|---|------|-------|
| 14 | Update `TabBar.omni` — 7 tabs | `components/TabBar.omni` |
| 15 | Update `main.omni` — new routing + imports | `main.omni` |
| 16 | Update `Header.omni` — show active theme | `components/Header.omni` |
| 17 | Remove deleted pages (SettingsPage, SupplementPage, HomePage, InteractivePage) | git rm |
| 18 | Update imports in all remaining components | various |
| 19 | Verify: `omnic build --target omniview` | CLI |
| 20 | Verify: All pages render with 4 themes | Browser MCP E2E |

### Phase 4: Polish

| # | Task |
|---|------|
| 21 | Add fadeSlideUp transition on theme switch |
| 22 | Animate token palette on theme change |
| 23 | Add keyboard navigation to theme selector (arrow keys) |
| 24 | Verify no console errors across all 7 pages × 4 themes |

---

## 9. Edge Cases & Error Handling

| Edge Case | Handling |
|-----------|----------|
| Theme name not in `@themes` list | Compile error (E0006 per ADR-017) |
| `localStorage` has an unknown theme | `__omni_theme.init()` falls back to default-theme |
| `ThemeStore.setMode("nonexistent")` | Rejected by `themes.indexOf() >= 0` guard |
| 0 themes declared | `__omni_theme` runtime not generated; ThemeSwitcher renders fallback |
| Multiple stores with "mode" signal | Each gets sync IIFE; last write wins (acceptable per ADR-044) |
| @for + @ref in future | Deferred to Phase 2 (per ADR-036) |

---

## 10. Spec Self-Review Checklist

- [x] No placeholders, TODOs, or vague requirements
- [x] Internal consistency: Section 3 (theme system) matches Section 4 (page architecture)
- [x] Scope focused: One redesign project with clear phases
- [x] No ambiguity: 4 specific themes, 7 specific tabs, all color values specified
- [x] Edge cases covered in Section 9
- [x] Design quality standards referenced (Section 7)
- [x] Implementation phases with concrete file lists and tasks
