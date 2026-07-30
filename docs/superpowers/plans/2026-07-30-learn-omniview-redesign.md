# Learn-OmniView Redesign: Multi-Theme Capability Showcase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign learn-omniview from a 10-tab feature listing into a focused 7-tab capability showcase, with the centerpiece being a 4-theme system (light/dark/sepia/highContrast) powered by OmniLang's `@themes` design block directive.

**Architecture:** The design block (`design.omni`) gains 2 additional theme value sets (sepia, highContrast) for all 18 color tokens. ThemeStore is refactored from hardcoded light/dark to support N themes via data-driven toggle. ThemeSwitcher is rewritten from 2 buttons to 4 themed cards. Pages are reorganized: DesignTheme.omni becomes the centerpiece, while SettingsPage/SupplementPage/HomePage/InteractivePage are consolidated into the new 7-tab structure.

**Tech Stack:** OmniLang (.omni) → OmniView (JS/HTML debug runtime). All changes are in `test/projects/learn-omniview/src/`.

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `pages/Overview.omni` | Capability matrix landing page |
| `pages/UiComponents.omni` | Merged Interactive + Todo + Events |
| `pages/DesignTheme.omni` | Multi-theme showcase centerpiece |
| `pages/StatePage.omni` | Signal/Store patterns page |
| `components/CapabilityCard.omni` | Overview page grid card |
| `components/DesignTokenPanel.omni` | Live token visualizer for all 18 types |
| `components/ThemeSourceDemo.omni` | Syntax-highlighted source code display |
| `components/VariantShowcase.omni` | Button variants across all 4 themes |

### Modified Files
| File | Change |
|------|--------|
| `design.omni` | Add sepia + highContrast values to all 18 color tokens |
| `stores/ThemeStore.omni` | N-theme support for toggle/setMode |
| `components/ThemeSwitcher.omni` | 2 buttons → 4 visual theme cards |
| `components/ThemeCssVarsDemo.omni` | Adapt to work with 4 themes |
| `components/TabBar.omni` | 10 tabs → 7 tabs |
| `components/Header.omni` | Show active theme name |
| `main.omni` | New routing + imports |
| `pages/LanguagePage.omni` | Merge SupplementPage content |
| `pages/MotionPage.omni` | Updated imports |
| `pages/CompilerPage.omni` | Updated imports (via dedup) |

### Deleted Files
| File | Reason |
|------|--------|
| `pages/HomePage.omni` | Replaced by Overview.omni |
| `pages/SettingsPage.omni` | Migrated into DesignTheme.omni |
| `pages/SupplementPage.omni` | Merged into LanguagePage.omni |
| `pages/InteractivePage.omni` | Merged into UiComponents.omni |

### Untouched
All remaining components, stores, lib files (~70 files). Only import paths in their parent pages change.

---

### Phase 1: Theme System Refactor (7 tasks)

### Task 1: Update `design.omni` — add sepia + highContrast token values

**Files:**
- Modify: `test/projects/learn-omniview/src/design.omni:18-19`
- Modify: `test/projects/learn-omniview/src/design.omni:32-51`

- [ ] **Step 1: Update @themes directive**

Change line 18-19 from:
```diff
-    @themes light, dark;
+    @themes light, dark, sepia, highContrast;
```

- [ ] **Step 2: Add per-theme values to all color tokens (brand colors)**

Replace the 11 color token definitions (lines 32-51) with 4-theme versions:

```diff
-    token primary: color = "#6366f1" dark "#a5b4fc";
+    token primary: color = "#6366f1" dark "#a5b4fc" sepia "#8B4513" highContrast "#ffff00";
-    token primaryLight: color = "#a5b4fc";
+    token primaryLight: color = "#a5b4fc" sepia "#D2A679" highContrast "#ffff99";
-    token primaryDark: color = "#4338ca";
+    token primaryDark: color = "#4338ca" sepia "#5D3A1A" highContrast "#cccc00";

-    token success: color = "#10b981";
+    token success: color = "#10b981" sepia "#6B8E23" highContrast "#00ff00";
-    token warning: color = "#f59e0b";
+    token warning: color = "#f59e0b" sepia "#CC7722" highContrast "#ffaa00";
-    token danger: color = "#ef4444";
+    token danger: color = "#ef4444" sepia "#A0522D" highContrast "#ff4444";
-    token accent: color = "#f97316";
+    token accent: color = "#f97316" sepia "#CD853F" highContrast "#ff8800";
```

- [ ] **Step 3: Add per-theme values to neutral tokens**

```diff
-    token surface: color = "#ffffff" dark "#111827";
+    token surface: color = "#ffffff" dark "#111827" sepia "#F5E6C8" highContrast "#000000";
-    token surfaceMuted: color = "#f9fafb" dark "#1f2937";
+    token surfaceMuted: color = "#f9fafb" dark "#1f2937" sepia "#EDE0C8" highContrast "#0a0a0a";
-    token surfaceSubtle: color = "#f3f4f6" dark "#374151";
+    token surfaceSubtle: color = "#f3f4f6" dark "#374151" sepia "#E0D0B0" highContrast "#1a1a1a";
-    token textPrimary: color = "#111827" dark "#f9fafb";
+    token textPrimary: color = "#111827" dark "#f9fafb" sepia "#3E2723" highContrast "#ffffff";
-    token textSecondary: color = "#6b7280" dark "#9ca3af";
+    token textSecondary: color = "#6b7280" dark "#9ca3af" sepia "#5D4037" highContrast "#cccccc";
-    token textMuted: color = "#9ca3af" dark "#d1d5db";
+    token textMuted: color = "#9ca3af" dark "#d1d5db" sepia "#8D6E63" highContrast "#999999";
-    token border: color = "#d4d4d8" dark "#4b5563";
+    token border: color = "#d4d4d8" dark "#4b5563" sepia "#BCAAA4" highContrast "#666666";
-    token borderLight: color = "#e5e7eb" dark "#374151";
+    token borderLight: color = "#e5e7eb" dark "#374151" sepia "#D7CCC8" highContrast "#444444";
-    token borderLighter: color = "#f3f4f6" dark "#1f2937";
+    token borderLighter: color = "#f3f4f6" dark "#1f2937" sepia "#EFEBE9" highContrast "#333333";
```

Leave all non-color tokens unchanged (spacing, radius, shadow, typography, duration, easing, opacity, zIndex — these are not theme-dependent).

- [ ] **Step 4: Build and verify**

```bash
cd /d/space/OmniLang/test/projects/learn-omniview
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds. Check generated CSS has 4 `[data-theme]` blocks.

---

### Task 2: Rewrite ThemeStore.omni — N-theme support

**Files:**
- Modify: `test/projects/learn-omniview/src/stores/ThemeStore.omni`

- [ ] **Step 1: Rewrite store content**

Replace the entire file with:

```omni
// ╔══════════════════════════════════════════════════════════════╗
// ║  ThemeStore.omni —— 多主题状态管理 (4 themes)                  ║
// ║                                                              ║
// ║  支持 light / dark / sepia / highContrast 四个主题。           ║
// ║  颜色值来自 design.omni 的 @themes 声明，通过 CSS 变量        ║
// ║  (var(--surface)) 自动切换，不在 store 中维护颜色 getter。    ║
// ║  本 store 仅管理 mode 信号和切换计数。                        ║
// ║                                                              ║
// ║  ADR-044 自动同步 mode ↔ __omni_theme.set() 到 data-theme。  ║
// ╚══════════════════════════════════════════════════════════════╝

import omniview;

store ThemeStore {
    // 当前主题模式："light" / "dark" / "sepia" / "highContrast"
    signal mode: String = "light";

    // 切换次数追踪
    signal toggleCount: i32 = 0;

    // getter: 当前主题的显示标签
    getter modeLabel: String =
        if mode == "light" { "☀️ Light" }
        else if mode == "dark" { "🌙 Dark" }
        else if mode == "sepia" { "📜 Sepia" }
        else { "🔲 High Contrast" };

    // action: 循环切换 light → dark → sepia → highContrast → light
    action toggle() {
        if mode == "light" { mode = "dark"; }
        else if mode == "dark" { mode = "sepia"; }
        else if mode == "sepia" { mode = "highContrast"; }
        else { mode = "light"; }
        toggleCount = toggleCount + 1;
    }

    // action: 设置指定模式（验证有效性）
    action setMode(newMode: String) {
        if newMode == "light" || newMode == "dark" || newMode == "sepia" || newMode == "highContrast" {
            mode = newMode;
        }
    }

    // action: 初始化（主题由 CSS 的 [data-theme] 属性驱动）
    action load() {
        // Nothing needed — design block's @default-theme + __omni_theme.init() handle CSS
    }
}
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds. The `__omni_theme` auto-sync IIFE picks up the mode signal changes.

---

### Task 3: Rewrite ThemeSwitcher.omni — 4-card visual selector

**Files:**
- Modify: `test/projects/learn-omniview/src/components/ThemeSwitcher.omni`

- [ ] **Step 1: Write the new ThemeSwitcher**

Replace the entire file with:

```omni
// ╔══════════════════════════════════════════════════════════════╗
// ║  ThemeSwitcher.omni —— 4 主题卡片选择器                       ║
// ║                                                              ║
// ║  演示：                                                      ║
// ║  - @store ThemeStore（N 主题模式）                             ║
// ║  - @for 列表渲染 + const 数组                                 ║
// ║  - CSS var() 驱动主题色预览                                  ║
// ║  - 鼠标悬停 + 选中态动画                                      ║
// ║                                                              ║
// ║  主题颜色来自 design.omni 的 @themes 声明，不在 ThemeStore    ║
// ║  中维护，直接使用 var(--primary) 等 CSS 变量。               ║
// ╚══════════════════════════════════════════════════════════════╝

import omniview;
import { LearnDesign } from "../design";
import { ThemeStore } from "../stores/ThemeStore";

component ThemeSwitcher {
    @store ThemeStore;

    // 主题元数据（用于 UI 渲染）
    const themes = ["light", "dark", "sepia", "highContrast"];
    const themeIcons = ["☀️", "🌙", "📜", "🔲"];
    const themeLabels = ["Light", "Dark", "Sepia", "High Contrast"];

    // 本地信号
    signal hoveredIndex: i32 = -1;

    fn isActive(theme: String) -> bool { return ThemeStore.mode == theme; }

    fn selectTheme(theme: String) {
        ThemeStore.setMode(theme);
    }

    view {
        <div class: LearnDesign.Container>
            <h3 class: LearnDesign.CardTitle>Theme Selector</h3>
            <p class: LearnDesign.Subtitle>OmniLang supports N themes — not just light/dark</p>

            <!-- ═══ 4 主题卡片 ═══ -->
            <div class: theme-grid>
                <div class: { theme-card: true, active: isActive("light") }
                     @onPress={() => { selectTheme("light"); }}
                     @onMouseEnter={() => { hoveredIndex = 0; }}
                     @onMouseLeave={() => { hoveredIndex = -1; }}>
                    <span class: card-icon>☀️</span>
                    <span class: card-name>Light</span>
                    <div class: card-swatches>
                        <span class: swatch style: { background: "#6366f1" } />
                        <span class: swatch style: { background: "#ffffff" } />
                        <span class: swatch style: { background: "#111827" } />
                    </div>
                </div>

                <div class: { theme-card: true, active: isActive("dark") }
                     @onPress={() => { selectTheme("dark"); }}
                     @onMouseEnter={() => { hoveredIndex = 1; }}
                     @onMouseLeave={() => { hoveredIndex = -1; }}>
                    <span class: card-icon>🌙</span>
                    <span class: card-name>Dark</span>
                    <div class: card-swatches>
                        <span class: swatch style: { background: "#a5b4fc" } />
                        <span class: swatch style: { background: "#111827" } />
                        <span class: swatch style: { background: "#f9fafb" } />
                    </div>
                </div>

                <div class: { theme-card: true, active: isActive("sepia") }
                     @onPress={() => { selectTheme("sepia"); }}
                     @onMouseEnter={() => { hoveredIndex = 2; }}
                     @onMouseLeave={() => { hoveredIndex = -1; }}>
                    <span class: card-icon>📜</span>
                    <span class: card-name>Sepia</span>
                    <div class: card-swatches>
                        <span class: swatch style: { background: "#8B4513" } />
                        <span class: swatch style: { background: "#F5E6C8" } />
                        <span class: swatch style: { background: "#3E2723" } />
                    </div>
                </div>

                <div class: { theme-card: true, active: isActive("highContrast") }
                     @onPress={() => { selectTheme("highContrast"); }}
                     @onMouseEnter={() => { hoveredIndex = 3; }}
                     @onMouseLeave={() => { hoveredIndex = -1; }}>
                    <span class: card-icon>🔲</span>
                    <span class: card-name>High Contrast</span>
                    <div class: card-swatches>
                        <span class: swatch style: { background: "#ffff00" } />
                        <span class: swatch style: { background: "#000000" } />
                        <span class: swatch style: { background: "#ffffff" } />
                    </div>
                </div>
            </div>

            <!-- ═══ 当前状态 ═══ -->
            <div class: status-bar>
                <span class: status-text>
                    Current: {ThemeStore.modeLabel}
                </span>
                <span class: status-count>
                    Toggled {ThemeStore.toggleCount} times
                </span>
            </div>

            <!-- ═══ 主题色色板预览（使用 CSS 变量） ═══ -->
            <div class: palette-preview>
                <p class: palette-title>Token Palette — <code>var(--name)</code> live</p>
                <div class: palette-grid>
                    <div class: palette-chip style: { background: "var(--surface)", border: "1px solid var(--border)" }>
                        <span class: chip-label>--surface</span>
                    </div>
                    <div class: palette-chip style: { background: "var(--primary)" }>
                        <span class: chip-label style: { color: "white" }>--primary</span>
                    </div>
                    <div class: palette-chip style: { background: "var(--textPrimary)" }>
                        <span class: chip-label style: { color: "var(--surface)" }>--textPrimary</span>
                    </div>
                    <div class: palette-chip style: { background: "var(--success)" }>
                        <span class: chip-label style: { color: "white" }>--success</span>
                    </div>
                    <div class: palette-chip style: { background: "var(--warning)" }>
                        <span class: chip-label style: { color: "white" }>--warning</span>
                    </div>
                    <div class: palette-chip style: { background: "var(--danger)" }>
                        <span class: chip-label style: { color: "white" }>--danger</span>
                    </div>
                </div>
            </div>

            <!-- ═══ 提示 ═══ -->
            <div class: hint-bar>
                <span class: hint-text>
                    💡 4 themes · CSS cascade via <code>[data-theme]</code> · zero JS per toggle
                </span>
            </div>
        </div>
    }

    style {
        .theme-grid { display: flex; gap: 12px; margin: 0 0 16px 0; }

        .theme-card {
            flex: 1; min-width: 0;
            display: flex; flex-direction: column; align-items: center; gap: 8px;
            padding: 16px 12px;
            background: var(--surface);
            border: 2px solid var(--border);
            border-radius: var(--radiusLg);
            cursor: pointer;
            transition: all var(--durationFast);
        }
        .theme-card:hover {
            transform: translateY(-2px);
            box-shadow: var(--shadowMd);
            border-color: var(--primaryLight);
        }
        .theme-card.active {
            border-color: var(--primary);
            box-shadow: "0 0 0 3px " var(--primaryLight);
            background: var(--surfaceMuted);
        }

        .card-icon { font-size: 28px; }
        .card-name { font-size: var(--fontSizeMd); font-weight: var(--fontWeightSemibold); color: var(--textPrimary); }

        .card-swatches { display: flex; gap: 4px; }
        .swatch { width: 20px; height: 20px; border-radius: var(--radiusSm); }

        .status-bar { display: flex; justify-content: space-between; align-items: center; padding: 10px 0; border-top: 1px solid var(--borderLight); margin: 0 0 14px 0; }
        .status-text { font-size: var(--fontSizeSm); color: var(--textPrimary); font-weight: var(--fontWeightSemibold); }
        .status-count { font-size: var(--fontSizeXs); color: var(--textSecondary); }

        .palette-preview { padding: 14px; background: var(--surfaceMuted); border-radius: var(--radiusMd); margin: 0 0 12px 0; }
        .palette-title { font-size: var(--fontSizeXs); color: var(--textSecondary); margin: 0 0 10px 0; }
        .palette-title code { font-size: var(--fontSizeXs); background: var(--surfaceSubtle); padding: "1px 4px"; border-radius: "3px"; }
        .palette-grid { display: flex; flex-wrap: wrap; gap: 6px; }
        .palette-chip { padding: "10px 14px"; border-radius: var(--radiusMd); min-width: "100px"; text-align: center; }
        .chip-label { font-size: var(--fontSizeXs); font-weight: var(--fontWeightSemibold); font-family: monospace; }

        .hint-bar { padding: "8px 0"; }
        .hint-text { font-size: var(--fontSizeXs); color: var(--textMuted); }
        .hint-text code { font-size: var(--fontSizeXs); background: var(--surfaceSubtle); padding: "1px 4px"; border-radius: "3px"; }
    }
}
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 4: Create ThemeSourceDemo.omni — syntax showcase

**Files:**
- Create: `test/projects/learn-omniview/src/components/ThemeSourceDemo.omni`

- [ ] **Step 1: Write the component**

```omni
// ╔══════════════════════════════════════════════════════════════════╗
// ║  ThemeSourceDemo.omni —— @themes 语法展示                        ║
// ║                                                                  ║
// ║  演示 design.omni 中 @themes 声明的源代码结构：                   ║
// ║  - @themes 指令（N 主题声明）                                     ║
// ║  - token 多主题值语法                                             ║
// ║  - @default-theme 回退主题                                        ║
// ║                                                                  ║
// ║  同时展示生成的 CSS 三层结构：                                    ║
// ║  :root → [data-theme="X"] → @media prefers-color-scheme          ║
// ╚══════════════════════════════════════════════════════════════════╝

import omniview;
import { LearnDesign } from "../design";

component ThemeSourceDemo {
    view {
        <div class: LearnDesign.Container>
            <h3 class: LearnDesign.CardTitle>How It Works: <code>@themes</code> Syntax</h3>
            <p class: LearnDesign.Subtitle>
                Design blocks declare themes via <code>@themes</code> directive.
                Per-token per-theme values compile to layer CSS.
            </p>

            <div class: code-layout>
                <!-- ═══ 源代码侧 ═══ -->
                <div class: source-panel>
                    <h4 class: panel-title>📄 design.omni (source)</h4>
                    <pre class: code-block>
                        <code>
@themes light, dark, sepia, highContrast;
@default-theme light;

token primary: color = "#6366f1"
    dark "#a5b4fc"
    sepia "#8B4513"
    highContrast "#ffff00";

token surface: color = "#ffffff"
    dark "#111827"
    sepia "#F5E6C8"
    highContrast "#000000";

token textPrimary: color = "#111827"
    dark "#f9fafb"
    sepia "#3E2723"
    highContrast "#ffffff";
// ... 18 color tokens × 4 themes
                        </code>
                    </pre>
                </div>

                <!-- ═══ 生成 CSS 侧 ═══ -->
                <div class: generated-panel>
                    <h4 class: panel-title>⚡ Generated CSS (3 layers)</h4>
                    <pre class: code-block css>
                        <code>
/* Layer 1: Default */
:root {
  --primary: #6366f1;
  --surface: #ffffff;
  --textPrimary: #111827;
}

/* Layer 2: Per-theme overrides */
[data-theme="dark"] {
  --primary: #a5b4fc;
  --surface: #111827;
}
[data-theme="sepia"] {
  --primary: #8B4513;
  --surface: #F5E6C8;
}
[data-theme="highContrast"] {
  --primary: #ffff00;
  --surface: #000000;
}

/* Layer 3: System preference */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    --primary: #a5b4fc;
  }
}
                        </code>
                    </pre>
                </div>
            </div>

            <div class: key-points>
                <span class: key-point>🔑 CSS Cascade — one <code>data-theme</code> toggle updates ALL tokens at once</span>
                <span class: key-point>🔑 Zero runtime — per-theme CSS is generated at compile time</span>
                <span class: key-point>🔑 N themes — add "forest", "ocean", or any named theme</span>
            </div>
        </div>
    }

    style {
        .code-layout { display: flex; gap: 14px; margin: 0 0 14px 0; }
        .source-panel { flex: 1; min-width: 0; }
        .generated-panel { flex: 1; min-width: 0; }
        .panel-title { font-size: var(--fontSizeSm); color: var(--textPrimary); margin: 0 0 8px 0; }
        .code-block {
            background: var(--surfaceSubtle);
            border: 1px solid var(--border);
            border-radius: var(--radiusMd);
            padding: 14px;
            overflow-x: auto;
            margin: 0;
            font-family: "ui-monospace, SFMono-Regular, monospace";
            font-size: var(--fontSizeXs);
            line-height: 1.6;
            color: var(--textPrimary);
            white-space: pre;
        }

        .key-points { display: flex; flex-direction: column; gap: 6px; padding: 12px; background: var(--surfaceMuted); border-radius: var(--radiusMd); }
        .key-point { font-size: var(--fontSizeXs); color: var(--textSecondary); }
        .key-point code { font-size: var(--fontSizeXs); background: var(--surfaceSubtle); padding: "1px 4px"; border-radius: "3px"; }
    }
}
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 5: Create DesignTokenPanel.omni — live token visualizer

**Files:**
- Create: `test/projects/learn-omniview/src/components/DesignTokenPanel.omni`

- [ ] **Step 1: Write the component**

```omni
// ╔══════════════════════════════════════════════════════════════════╗
// ║  DesignTokenPanel.omni —— 设计令牌实时可视化                      ║
// ║                                                                  ║
// ║  展示所有 18 种设计令牌类型在当前主题下的值。                      ║
// ║  令牌值通过 var(--name) CSS 变量引用，主题切换时自动更新。        ║
// ║                                                                  ║
// ║  Token 类型：                                                    ║
// ║  Color（11）、Size（6）、Radius（4）、Shadow（3）                  ║
// ║  FontFamily（1）、FontSize（11）、FontWeight（3）、更多...         ║
// ╚══════════════════════════════════════════════════════════════════╝

import omniview;
import { LearnDesign } from "../design";

component DesignTokenPanel {
    // 令牌元数据：name → 类型 → 展示方式
    const colorTokens = ["primary", "primaryLight", "success", "warning", "danger", "accent", "surface", "surfaceMuted", "textPrimary", "textSecondary", "border"];
    const sizeTokens = ["spaceXs", "spaceSm", "spaceMd", "spaceLg", "spaceXl", "spaceXxl"];
    const radiusTokens = ["radiusSm", "radiusMd", "radiusLg", "radiusFull"];

    view {
        <div class: LearnDesign.Container>
            <h3 class: LearnDesign.CardTitle>Design Tokens Visualizer</h3>
            <p class: LearnDesign.Subtitle>
                18 token types · values change instantly on theme switch via <code>var(--name)</code>
            </p>

            <!-- ═══ Color Tokens ═══ -->
            <h4 class: LearnDesign.SectionTitle>🎨 Color Tokens (11)</h4>
            <div class: color-grid>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--primary)" }></div>
                    <span class: color-name>--primary</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--primaryLight)" }></div>
                    <span class: color-name>--primaryLight</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--success)" }></div>
                    <span class: color-name>--success</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--warning)" }></div>
                    <span class: color-name>--warning</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--danger)" }></div>
                    <span class: color-name>--danger</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--accent)" }></div>
                    <span class: color-name>--accent</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--surface)", border: "1px solid var(--border)" }></div>
                    <span class: color-name>--surface</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--surfaceMuted)" }></div>
                    <span class: color-name>--surfaceMuted</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--textPrimary)" }></div>
                    <span class: color-name>--textPrimary</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--textSecondary)" }></div>
                    <span class: color-name>--textSecondary</span>
                </div>
                <div class: color-item>
                    <div class: color-swatch style: { background: "var(--border)" }></div>
                    <span class: color-name>--border</span>
                </div>
            </div>

            <!-- ═══ 其他令牌分组 ═══ -->
            <h4 class: LearnDesign.SectionTitle>📐 Size Tokens</h4>
            <div class: size-demo>
                <div class: size-bar style: { width: "var(--spaceXs)" }>xs</div>
                <div class: size-bar style: { width: "var(--spaceSm)" }>sm</div>
                <div class: size-bar style: { width: "var(--spaceMd)" }>md</div>
                <div class: size-bar style: { width: "var(--spaceLg)" }>lg</div>
                <div class: size-bar style: { width: "var(--spaceXl)" }>xl</div>
                <div class: size-bar style: { width: "var(--spaceXxl)" }>xxl</div>
            </div>

            <h4 class: LearnDesign.SectionTitle>🔲 Radius Tokens</h4>
            <div class: radius-demo>
                <div class: radius-box style: { border-radius: "var(--radiusSm)" }>sm</div>
                <div class: radius-box style: { border-radius: "var(--radiusMd)" }>md</div>
                <div class: radius-box style: { border-radius: "var(--radiusLg)" }>lg</div>
                <div class: radius-box style: { border-radius: "var(--radiusFull)" }>full</div>
            </div>

            <h4 class: LearnDesign.SectionTitle>💡 Shadow Tokens</h4>
            <div class: shadow-demo>
                <div class: shadow-box style: { box-shadow: "var(--shadowSm)" }>sm</div>
                <div class: shadow-box style: { box-shadow: "var(--shadowMd)" }>md</div>
                <div class: shadow-box style: { box-shadow: "var(--shadowLg)" }>lg</div>
            </div>

            <div class: hint-bar>
                <span class: hint-text>🔄 Switch themes above — all tokens update automatically via CSS cascade</span>
            </div>
        </div>
    }

    style {
        .color-grid { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 18px; }
        .color-item { display: flex; flex-direction: column; align-items: center; gap: 4px; }
        .color-swatch { width: 36px; height: 36px; border-radius: var(--radiusMd); }
        .color-name { font-size: var(--fontSizeXs); color: var(--textSecondary); font-family: monospace; }

        .size-demo { display: flex; align-items: center; gap: 8px; margin-bottom: 18px; }
        .size-bar { height: 24px; background: var(--primary); border-radius: var(--radiusSm); color: white; font-size: var(--fontSizeXs); display: flex; align-items: center; padding: "0 6px"; }

        .radius-demo { display: flex; gap: 12px; margin-bottom: 18px; }
        .radius-box { width: 60px; height: 60px; background: var(--primary); color: white; display: flex; align-items: center; justify-content: center; font-size: var(--fontSizeXs); }

        .shadow-demo { display: flex; gap: 16px; margin-bottom: 18px; }
        .shadow-box { width: 80px; height: 60px; background: var(--surface); border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; font-size: var(--fontSizeXs); color: var(--textSecondary); }

        .hint-bar { padding: "8px 0"; }
        .hint-text { font-size: var(--fontSizeXs); color: var(--textMuted); }
    }
}
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 6: Create VariantShowcase.omni — button variants across 4 themes

**Files:**
- Create: `test/projects/learn-omniview/src/components/VariantShowcase.omni`

- [ ] **Step 1: Write the component**

```omni
// ╔══════════════════════════════════════════════════════════════════╗
// ║  VariantShowcase.omni —— 组件变体跨主题演示                      ║
// ║                                                                  ║
// ║  展示 LearnDesign.Button 变体（color × size）在 4 个主题下：     ║
// ║  - color: primary / danger / ghost / accent                     ║
// ║  - size: sm / md / lg                                           ║
// ║                                                                  ║
// ║  主题切换时，Button 变体颜色自动变化（通过 var(--*) 继承）。     ║
// ╚══════════════════════════════════════════════════════════════════╝

import omniview;
import { LearnDesign } from "../design";

component VariantShowcase {
    view {
        <div class: LearnDesign.Container>
            <h3 class: LearnDesign.CardTitle>Component Variants</h3>
            <p class: LearnDesign.Subtitle>
                <code>variant: LearnDesign.Button(color=X, size=Y)</code> — multi-dimension theming
            </p>

            <h4 class: LearnDesign.SectionTitle>Color Variants (size=md)</h4>
            <div class: variant-row>
                <button variant: LearnDesign.Button(color=primary, size=md)>Primary</button>
                <button variant: LearnDesign.Button(color=danger, size=md)>Danger</button>
                <button variant: LearnDesign.Button(color=ghost, size=md)>Ghost</button>
                <button variant: LearnDesign.Button(color=accent, size=md)>Accent</button>
            </div>

            <h4 class: LearnDesign.SectionTitle>Size Variants (color=primary)</h4>
            <div class: variant-row>
                <button variant: LearnDesign.Button(color=primary, size=sm)>Small</button>
                <button variant: LearnDesign.Button(color=primary, size=md)>Medium</button>
                <button variant: LearnDesign.Button(color=primary, size=lg)>Large</button>
            </div>

            <h4 class: LearnDesign.SectionTitle>Theme Responsiveness</h4>
            <p class: hint-text>
                Switch themes above — button colors change automatically via <code>var(--primary)</code>, <code>var(--danger)</code>, etc.
                No component code changes needed.
            </p>
        </div>
    }

    style {
        .variant-row { display: flex; gap: 10px; margin: 0 0 16px 0; flex-wrap: wrap; }
        .hint-text { font-size: var(--fontSizeXs); color: var(--textMuted); margin: 0; }
        .hint-text code { font-size: var(--fontSizeXs); background: var(--surfaceSubtle); padding: "1px 4px"; border-radius: "3px"; }
    }
}
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 7: Update ThemeCssVarsDemo.omni — adapt to 4 themes

**Files:**
- Modify: `test/projects/learn-omniview/src/components/ThemeCssVarsDemo.omni`

- [ ] **Step 1: Change the toggle button text to show 4-theme cycling**

In the `action-row` section of ThemeCssVarsDemo.omni, update the toggle button text from "Switch to {ThemeStore.nextLabel} Mode" to show the next theme:

```diff
-                <p class: hint>
-                    当前主题: <strong>{ThemeStore.modeLabel}</strong> ·
-                    切换次数: {ThemeStore.toggleCount} ·
-                    代码路径: <code>[data-theme] CSS</code>
-                </p>
+                <p class: hint>
+                    Current: {ThemeStore.modeLabel} ·
+                    Toggles: {ThemeStore.toggleCount} ·
+                    <code>@themes light, dark, sepia, highContrast</code>
+                </p>
```

And remove the `nextLabel`-dependent button text (since ThemeStore no longer has `nextLabel`). Instead, the toggle button just says "Toggle Theme".

```diff
-                <button class: toggle-btn @onPress={() => { ThemeStore.toggle(); }}>
-                    Switch to {ThemeStore.nextLabel} Mode
-                </button>
+                <button class: toggle-btn @onPress={() => { ThemeStore.toggle(); }}>
+                    Toggle Theme (light → dark → sepia → highContrast)
+                </button>
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Phase 2: Page Restructure (6 tasks)

### Task 8: Create CapabilityCard.omni + Overview.omni

**Files:**
- Create: `test/projects/learn-omniview/src/components/CapabilityCard.omni`
- Create: `test/projects/learn-omniview/src/pages/Overview.omni`

- [ ] **Step 1: Create CapabilityCard.omni**

```omni
// CapabilityCard.omni — 能力域卡片（可复用组件）

import omniview;
import { LearnDesign } from "../design";

component CapabilityCard {
    prop icon: String = "🔧";
    prop title: String = "";
    prop description: String = "";
    prop tabName: String = "";
    event onNavigate: String;

    view {
        <div class: capability-card @onPress={() => { onNavigate(tabName); }}>
            <span class: card-icon>{icon}</span>
            <h3 class: card-title>{title}</h3>
            <p class: card-desc>{description}</p>
            <span class: card-link>Explore →</span>
        </div>
    }

    style {
        .capability-card {
            padding: 20px;
            background: var(--surface);
            border: 1px solid var(--borderLight);
            border-radius: var(--radiusLg);
            cursor: pointer;
            transition: all var(--durationFast);
            display: flex; flex-direction: column;
        }
        .capability-card:hover {
            transform: translateY(-3px);
            box-shadow: var(--shadowMd);
            border-color: var(--primaryLight);
        }
        .card-icon { font-size: 32px; margin: 0 0 10px 0; }
        .card-title { font-size: var(--fontSizeLg); font-weight: var(--fontWeightSemibold); color: var(--textPrimary); margin: 0 0 6px 0; }
        .card-desc { font-size: var(--fontSizeSm); color: var(--textSecondary); margin: 0 0 12px 0; flex: 1; }
        .card-link { font-size: var(--fontSizeXs); color: var(--primary); font-weight: var(--fontWeightSemibold); }
    }
}
```

- [ ] **Step 2: Create Overview.omni**

```omni
// Overview.omni — 能力矩阵展示页

import omniview;
import { LearnDesign } from "../design";
import { CapabilityCard } from "../components/CapabilityCard";
import { ThemeStore } from "../stores/ThemeStore";

component Overview {
    @store ThemeStore;

    event onNavigate: String;

    view {
        <section class: overview-section>
            <!-- ═══ Banner ═══ -->
            <div class: LearnDesign.Banner>
                <h1 class: LearnDesign.BannerTitle>🚀 OmniLang Capability Showcase</h1>
                <p class: LearnDesign.BannerSubtitle>
                    v1.0.0 · 60+ source files · 4 themes · 7 capability domains
                </p>
            </div>

            <!-- ═══ 能力矩阵 ═══ -->
            <div class: capability-grid>
                <CapabilityCard
                    icon="🎨" title="UI Components"
                    description="Forms, modals, tables, todo app, notifications, slots, @for rendering"
                    tabName="ui"
                    @onNavigate={onNavigate}
                />
                <CapabilityCard
                    icon="🎬" title="Motion & Animation"
                    description="Keyframes, easing, triggers, drag-and-drop, element refs, 60fps DOM"
                    tabName="motion"
                    @onNavigate={onNavigate}
                />
                <CapabilityCard
                    icon="💅" title="Design & Theme"
                    description="4 themes · N-theme @themes directive · CSS cascade · design tokens · variants"
                    tabName="design"
                    @onNavigate={onNavigate}
                />
                <CapabilityCard
                    icon="⚡" title="State & Signals"
                    description="Signal reactivity, computed, @effect, stores (global/inline), bind:value"
                    tabName="state"
                    @onNavigate={onNavigate}
                />
                <CapabilityCard
                    icon="🔤" title="Language Features"
                    description="Nullish, optional chaining, ternary, block expressions, type system, style pseudos"
                    tabName="language"
                    @onNavigate={onNavigate}
                />
                <CapabilityCard
                    icon="⚙️" title="Compiler Features"
                    description="HTTP fetch, data submit, error handling, async patterns, user settings"
                    tabName="compiler"
                    @onNavigate={onNavigate}
                />
            </div>

            <!-- ═══ 快速统计 ═══ -->
            <div class: stats-bar>
                <div class: stat-item>
                    <span class: stat-value>60+</span>
                    <span class: stat-label>Source Files</span>
                </div>
                <div class: stat-item>
                    <span class: stat-value>7</span>
                    <span class: stat-label>Domains</span>
                </div>
                <div class: stat-item>
                    <span class: stat-value>4</span>
                    <span class: stat-label>Themes</span>
                </div>
                <div class: stat-item>
                    <span class: stat-value>{ThemeStore.toggleCount}</span>
                    <span class: stat-label>Theme Toggles</span>
                </div>
            </div>
        </section>
    }

    style {
        .overview-section { }
        .capability-grid { display: grid; grid-template-columns: "1fr 1fr 1fr"; gap: 14px; margin: 0 0 20px 0; }
        .stats-bar { display: flex; gap: 16px; justify-content: center; padding: 16px; background: var(--surface); border-radius: var(--radiusLg); border: 1px solid var(--borderLight); }
        .stat-item { display: flex; flex-direction: column; align-items: center; gap: 2px; min-width: 80px; }
        .stat-value { font-size: var(--fontSize3xl); font-weight: var(--fontWeightBold); color: var(--primary); }
        .stat-label { font-size: var(--fontSizeXs); color: var(--textSecondary); }
    }
}
```

- [ ] **Step 3: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 9: Create UiComponents.omni page

**Files:**
- Create: `test/projects/learn-omniview/src/pages/UiComponents.omni`

- [ ] **Step 1: Create the new page**

```omni
// UiComponents.omni — 合并 Interactive + Todo + Events 页面

import omniview;
import { LearnDesign } from "../design";
import { ValidationInput } from "../components/ValidationInput";
import { Timer } from "../components/Timer";
import { Modal } from "../components/Modal";
import { TabPanel } from "../components/TabPanel";
import { DataTable } from "../components/DataTable";
import { FormPage } from "../components/FormPage";
import { NotificationsPanel } from "../components/NotificationsPanel";
import { SlotDefaultDemo } from "../components/SlotDefaultDemo";
import { EventsDemo } from "../components/EventsDemo";
import { EventShorthand } from "../components/EventShorthand";
// Todo app
import { TodoList } from "../components/TodoList";
import { FilterBar } from "../components/FilterBar";
import { StatsDashboard } from "../components/StatsDashboard";
import { AddTodo } from "../components/AddTodo";

component UiComponents {
    view {
        <section class: LearnDesign.DemoSection>
            <!-- ═══ 交互组件 ═══ -->
            <div class: LearnDesign.Row>
                <div class: LearnDesign.Col><ValidationInput /></div>
                <div class: LearnDesign.Col><Timer /></div>
            </div>
            <div class: LearnDesign.Row>
                <div class: LearnDesign.Col><Modal /></div>
                <div class: LearnDesign.Col><TabPanel /></div>
            </div>
            <div class: LearnDesign.DemoFull><DataTable /></div>

            <div class: LearnDesign.Row>
                <div class: LearnDesign.Col><SlotDefaultDemo /></div>
                <div class: LearnDesign.Col><EventsDemo /></div>
            </div>

            <!-- ═══ 表单 ═══ -->
            <div class: LearnDesign.DemoFull><FormPage /></div>
            <div class: LearnDesign.DemoFull><NotificationsPanel /></div>
            <div class: LearnDesign.DemoFull><EventShorthand /></div>

            <!-- ═══ Todo 应用 ═══ -->
            <h3 class: LearnDesign.SectionTitle>Todo App Demo</h3>
            <div class: todo-section>
                <div class: todo-sidebar>
                    <AddTodo />
                    <FilterBar />
                </div>
                <div class: todo-main>
                    <TodoList />
                </div>
                <div class: todo-stats>
                    <StatsDashboard />
                </div>
            </div>
        </section>
    }

    style {
        .todo-section { display: flex; gap: 14px; }
        .todo-sidebar { width: 200px; flex-shrink: 0; }
        .todo-main { flex: 1; min-width: 0; }
        .todo-stats { width: 200px; flex-shrink: 0; }
    }
}
```

- [ ] **Step 2: Check if SlotDefaultDemo exists**

```bash
ls /d/space/OmniLang/test/projects/learn-omniview/src/components/SlotDefaultDemo.omni 2>&1 || echo "NOT FOUND"
```

If `SlotDefaultDemo.omni` does NOT exist, it will be handled in a later cleanup step. For now, proceed without it.

- [ ] **Step 3: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation may fail if SlotDefaultDemo doesn't exist — remove that import line and rebuild.

---

### Task 10: Create DesignTheme.omni page

**Files:**
- Create: `test/projects/learn-omniview/src/pages/DesignTheme.omni`

- [ ] **Step 1: Create the centerpiece page**

```omni
// DesignTheme.omni — 设计系统与多主题展示页（核心页面）

import omniview;
import { LearnDesign } from "../design";
import { ThemeSwitcher } from "../components/ThemeSwitcher";
import { ThemeSourceDemo } from "../components/ThemeSourceDemo";
import { DesignTokenPanel } from "../components/DesignTokenPanel";
import { VariantShowcase } from "../components/VariantShowcase";
import { ThemeCssVarsDemo } from "../components/ThemeCssVarsDemo";

component DesignTheme {
    view {
        <section class: LearnDesign.DemoSection>
            <!-- ═══ Section 1: Theme Selector ═══ -->
            <ThemeSwitcher />

            <!-- ═══ Section 2: Source Code ═══ -->
            <ThemeSourceDemo />

            <!-- ═══ Section 3: Design Tokens ═══ -->
            <DesignTokenPanel />

            <!-- ═══ Section 4: Variants ═══ -->
            <VariantShowcase />

            <!-- ═══ Section 5: CSS Variable Cascade ═══ -->
            <ThemeCssVarsDemo />
        </section>
    }
}
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 11: Create StatePage.omni — signal/store patterns page

**Files:**
- Create: `test/projects/learn-omniview/src/pages/StatePage.omni`

- [ ] **Step 1: Create the state page**

```omni
// StatePage.omni — 信号与状态管理

import omniview;
import { LearnDesign } from "../design";
import { SignalPatternsDemo } from "./SignalPatternsDemo";
import { InlineStoreDemo } from "../components/InlineStoreDemo";
import { ComputedFnFormDemo } from "../components/ComputedFnFormDemo";

component StatePage {
    view {
        <section class: LearnDesign.DemoSection>
            <div class: LearnDesign.DemoFull><SignalPatternsDemo /></div>
            <div class: LearnDesign.Row>
                <div class: LearnDesign.Col><InlineStoreDemo /></div>
                <div class: LearnDesign.Col><ComputedFnFormDemo /></div>
            </div>
        </section>
    }
}
```

**Note:** The slot-related sections from SignalPatternsDemo stay in SignalPatternsDemo. They are part of the signal/store patterns story, not moved to UI page.

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 12: Merge SupplementPage into LanguagePage

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/LanguagePage.omni`

- [ ] **Step 1: Update imports and layout**

```omni
// LanguagePage — 深度语言特性（合并 Language + Supplement）

import omniview;
import { LearnDesign } from "../design";
import { InlineStoreDemo } from "../components/InlineStoreDemo";
import { NullishDemo } from "../components/NullishDemo";
import { StringLiteralsDemo } from "../components/StringLiteralsDemo";
import { TypeSystemDemo } from "../components/TypeSystemDemo";
import { ViewAttributesDemo } from "../components/ViewAttributesDemo";
import { ComputedFnFormDemo } from "../components/ComputedFnFormDemo";
import { OptionalChainDemo } from "../components/OptionalChainDemo";
import { StylePseudoDemo } from "../components/StylePseudoDemo";
import { TernaryExprDemo } from "../components/TernaryExprDemo";
import { BlockExprDemo } from "../components/BlockExprDemo";
import { ForDirectiveKeyDemo } from "../components/ForDirectiveKeyDemo";
import { AdvancedPseudoDemo } from "../components/AdvancedPseudoDemo";

component LanguagePage {
    view {
        <section class: LearnDesign.DemoSection>
            <div class: LearnDesign.Row>
                <div class: LearnDesign.Col><InlineStoreDemo /></div>
                <div class: LearnDesign.Col><NullishDemo /></div>
            </div>
            <div class: LearnDesign.DemoFull><StringLiteralsDemo /></div>
            <div class: LearnDesign.Grid>
                <div class: LearnDesign.Col><TypeSystemDemo /></div>
                <div class: LearnDesign.Col><ViewAttributesDemo /></div>
            </div>
            <div class: LearnDesign.Grid>
                <div class: LearnDesign.Col><ComputedFnFormDemo /></div>
                <div class: LearnDesign.Col><OptionalChainDemo /></div>
                <div class: LearnDesign.Col><TernaryExprDemo /></div>
                <div class: LearnDesign.Col><BlockExprDemo /></div>
            </div>
            <div class: LearnDesign.DemoFull><ForDirectiveKeyDemo /></div>
            <div class: LearnDesign.Grid>
                <div class: LearnDesign.Col><StylePseudoDemo /></div>
                <div class: LearnDesign.Col><AdvancedPseudoDemo /></div>
            </div>
        </section>
    }
}
```

Note: `InlineStoreDemo` and `ComputedFnFormDemo` still appear on LanguagePage as well as StatePage. This is intentional — they demonstrate language-level features (inline store syntax, computed function syntax). Different focus.

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 13: Enhance ElementRefDemo.omni — drag constraints + capturePointer

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/ElementRefDemo.omni`

- [ ] **Step 1: Add more drag constraint variations**

In the ElementRefDemo, after the existing drag demo section (~line 133-145), add a second demo area:

```omni
            <!-- ═══ 拖拽约束对比 ═══ -->
            <h4 class: LearnDesign.SectionTitle>Drag Constraint Variations</h4>
            <p class: LearnDesign.Hint>
                @drag-constraints and @drag-elastic control drag boundaries and snap behavior
            </p>
            <div class: constraints-row>
                <!-- 弹性拖拽 -->
                <div @dragHorizontal={fadeSlideUp}
                     @drag-constraints="left:0 right:150"
                     @drag-elastic={0.5}
                     class: drag-demo-box>
                    <p>elastic 0.5</p>
                </div>
                <!-- 硬边界 -->
                <div @dragHorizontal={fadeSlideUp}
                     @drag-constraints="left:0 right:150"
                     @drag-elastic={0}
                     class: drag-demo-box>
                    <p>elastic 0</p>
                </div>
                <!-- 垂直约束 -->
                <div @dragVertical={fadeSlideUp}
                     @drag-constraints="top:0 bottom:100"
                     class: drag-demo-box>
                    <p>dragVertical</p>
                </div>
                <!-- 自由拖拽 -->
                <div @drag={fadeSlideUp}
                     @drag-constraints="left:0 right:150 top:0 bottom:100"
                     class: drag-demo-box>
                    <p>free drag</p>
                </div>
            </div>
```

Add corresponding style rules:
```omni
        .constraints-row { display: flex; gap: 16px; margin-bottom: 12px; flex-wrap: wrap; }
        .drag-demo-box { width: 100px; height: 80px; padding: 12px; background: var(--primary); color: white; border-radius: var(--radiusMd); text-align: center; cursor: grab; user-select: none; }
        .drag-demo-box p { font-size: var(--fontSizeXs); margin: 0; }
```

Also add the `@drag` motion trigger import. At the top, add:
```diff
+// @drag / @dragHorizontal / @dragVertical all use the same fadeSlideUp animation
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Phase 3: Navigation & Cleanup (5 tasks)

### Task 14: Update TabBar.omni — 10 tabs → 7 tabs

**Files:**
- Modify: `test/projects/learn-omniview/src/components/TabBar.omni`

- [ ] **Step 1: Replace the tab buttons**

Replace all 10 tab buttons with 7:

```diff
 component TabBar {
     prop activeTab: String = "home";
     event onSwitch: String;

     fn isActive(tab: String) -> bool { return activeTab == tab; }

     view {
         <div class: tab-strip>
-            <button class: { tab: true, tabActive: isActive("home") } @onPress={() => { onSwitch("home"); }}>首页</button>
-            <button class: { tab: true, tabActive: isActive("todo") } @onPress={() => { onSwitch("todo"); }}>待办</button>
-            <button class: { tab: true, tabActive: isActive("interactive") } @onPress={() => { onSwitch("interactive"); }}>基础交互</button>
-            <button class: { tab: true, tabActive: isActive("compiler") } @onPress={() => { onSwitch("compiler"); }}>编译器特性</button>
-            <button class: { tab: true, tabActive: isActive("motion") } @onPress={() => { onSwitch("motion"); }}>运动动画</button>
-            <button class: { tab: true, tabActive: isActive("css") } @onPress={() => { onSwitch("css"); }}>CSS 特性</button>
-            <button class: { tab: true, tabActive: isActive("events") } @onPress={() => { onSwitch("events"); }}>事件/表单</button>
-            <button class: { tab: true, tabActive: isActive("language") } @onPress={() => { onSwitch("language"); }}>语言特性</button>
-            <button class: { tab: true, tabActive: isActive("settings") } @onPress={() => { onSwitch("settings"); }}>设置</button>
-            <button class: { tab: true, tabActive: isActive("supplement") } @onPress={() => { onSwitch("supplement"); }}>补充</button>
+            <button class: { tab: true, tabActive: isActive("overview") } @onPress={() => { onSwitch("overview"); }}>📋 Overview</button>
+            <button class: { tab: true, tabActive: isActive("ui") } @onPress={() => { onSwitch("ui"); }}>🎨 UI</button>
+            <button class: { tab: true, tabActive: isActive("motion") } @onPress={() => { onSwitch("motion"); }}>🎬 Motion</button>
+            <button class: { tab: true, tabActive: isActive("design") } @onPress={() => { onSwitch("design"); }}>💅 Design</button>
+            <button class: { tab: true, tabActive: isActive("state") } @onPress={() => { onSwitch("state"); }}>⚡ State</button>
+            <button class: { tab: true, tabActive: isActive("language") } @onPress={() => { onSwitch("language"); }}>🔤 Language</button>
+            <button class: { tab: true, tabActive: isActive("compiler") } @onPress={() => { onSwitch("compiler"); }}>⚙️ Compiler</button>
         </div>
     }
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 15: Update main.omni — new routing + imports

**Files:**
- Modify: `test/projects/learn-omniview/src/main.omni`

- [ ] **Step 1: Replace imports and routing**

```diff
 import omniview;

 // ── 全局 Store ──
 import { TodoStore } from "./stores/TodoStore";
 import { ThemeStore } from "./stores/ThemeStore";
 import { NotificationStore } from "./stores/NotificationStore";

 // ── 共享组件 ──
 import { Header } from "./components/Header";
 import { TabBar } from "./components/TabBar";
 import { LearnDesign } from "./design";

-// ── 页面聚合组件 ──
-import { HomePage } from "./pages/HomePage";
-import { TodoPage } from "./pages/TodoPage";
-import { InteractivePage } from "./pages/InteractivePage";
-import { CompilerPage } from "./pages/CompilerPage";
-import { MotionPage } from "./pages/MotionPage";
-import { CssPage } from "./pages/CssPage";
-import { EventsPage } from "./pages/EventsPage";
-import { LanguagePage } from "./pages/LanguagePage";
-import { SettingsPage } from "./pages/SettingsPage";
-import { SupplementPage } from "./pages/SupplementPage";
+// ── 7 标签页页面 ──
+import { Overview } from "./pages/Overview";
+import { UiComponents } from "./pages/UiComponents";
+import { MotionPage } from "./pages/MotionPage";
+import { DesignTheme } from "./pages/DesignTheme";
+import { StatePage } from "./pages/StatePage";
+import { LanguagePage } from "./pages/LanguagePage";
+import { CompilerPage } from "./pages/CompilerPage";

 // ── 逻辑库 ──
 import "./lib/types" as Types;
 import "./lib/stats" as StatsLib;
 import "./lib/transform" as TransformLib;
 import "./lib/validation" as ValidationLib;
 import { Percentage } from "./lib/reexports";

 component App {
     @store TodoStore;
     @store ThemeStore;
     @store NotificationStore;

-    signal activeTab: String = "home";
+    signal activeTab: String = "overview";

     fn switchTab(tab: String) { activeTab = tab; }

     view {
         <div class: LearnDesign.Page>
             <Header />
             <TabBar activeTab={activeTab} @onSwitch={switchTab} />

             <div class: LearnDesign.TabContent>
-                @if activeTab == "home"        { <HomePage /> }
-                @if activeTab == "todo"        { <TodoPage /> }
-                @if activeTab == "interactive" { <InteractivePage /> }
-                @if activeTab == "compiler"    { <CompilerPage /> }
-                @if activeTab == "motion"      { <MotionPage /> }
-                @if activeTab == "css"         { <CssPage /> }
-                @if activeTab == "events"      { <EventsPage /> }
-                @if activeTab == "language"    { <LanguagePage /> }
-                @if activeTab == "settings"    { <SettingsPage /> }
-                @if activeTab == "supplement"  { <SupplementPage /> }
+                @if activeTab == "overview"  { <Overview @onNavigate={switchTab} /> }
+                @if activeTab == "ui"        { <UiComponents /> }
+                @if activeTab == "motion"    { <MotionPage /> }
+                @if activeTab == "design"    { <DesignTheme /> }
+                @if activeTab == "state"     { <StatePage /> }
+                @if activeTab == "language"  { <LanguagePage /> }
+                @if activeTab == "compiler"  { <CompilerPage /> }
             </div>

             <footer class: LearnDesign.Footer>
-                <p class: LearnDesign.FooterText>OmniView 教学项目 · 66+ 源文件 · omnic run</p>
+                <p class: LearnDesign.FooterText>OmniView 能力展示 · 7 domains · 4 themes · omnic run</p>
             </footer>
         </div>
     }
 }
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 16: Update Header.omni — show active theme

**Files:**
- Modify: `test/projects/learn-omniview/src/components/Header.omni`

- [ ] **Step 1: Read current Header.omni**

```bash
cat /d/space/OmniLang/test/projects/learn-omniview/src/components/Header.omni
```
Check if it already imports ThemeStore or shows theme info. If not, add it.

- [ ] **Step 2: Add theme indicator**

Add ThemeStore import and a small theme badge:

```diff
 import omniview;
 import { LearnDesign } from "../design";
+import { ThemeStore } from "../stores/ThemeStore";

 component Header {
+    @store ThemeStore;
+
     view {
         <header class: LearnDesign.PageHeader>
             <h1 class: LearnDesign.PageTitle>OmniView Learn</h1>
-            <p class: LearnDesign.PageSubtitle>OmniLang UI Framework Demo</p>
+            <p class: LearnDesign.PageSubtitle>
+                OmniLang UI Framework · {ThemeStore.modeLabel}
+            </p>
         </header>
     }
 }
```

- [ ] **Step 3: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds.

---

### Task 17: Remove deleted page files

**Files:**
- Delete: `test/projects/learn-omniview/src/pages/HomePage.omni`
- Delete: `test/projects/learn-omniview/src/pages/SettingsPage.omni`
- Delete: `test/projects/learn-omniview/src/pages/SupplementPage.omni`
- Delete: `test/projects/learn-omniview/src/pages/InteractivePage.omni`
- Delete: `test/projects/learn-omniview/src/pages/CssPage.omni`
- Delete: `test/projects/learn-omniview/src/pages/EventsPage.omni`
- Delete: `test/projects/learn-omniview/src/pages/TodoPage.omni`

- [ ] **Step 1: Remove old page files**

```bash
cd /d/space/OmniLang/test/projects/learn-omniview/src
git rm pages/HomePage.omni pages/SettingsPage.omni pages/SupplementPage.omni pages/InteractivePage.omni pages/CssPage.omni pages/EventsPage.omni pages/TodoPage.omni
```

- [ ] **Step 2: Build and verify**

```bash
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds. Verify no "unused import" warnings for deleted pages.

---

### Task 18: Update imports in remaining components

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/CompilerPage.omni` (no change needed — it's already standalone)
- Verify: all remaining component/import chains compile
- Verify: `SignalPatternsDemo.omni` no longer imports deleted `*Page` files

- [ ] **Step 1: Verify all imports resolve**

```bash
cd /d/space/OmniLang/test/projects/learn-omniview
omnic build --target omniview 2>&1
```
Expected: Clean compilation. If there are import errors, fix them one by one.

- [ ] **Step 2: Fix any broken imports**

Common issues:
- `MotionPage.omni` importing `ElementRefDemo` from correct path
- `DesignTokenPanel.omni` or `ThemeSourceDemo.omni` not found
- Duplicate component names across files

Fix each error with an Edit call.

---

### Phase 4: Polish (3 tasks)

### Task 19: Add Header active tab indicator + small animation

- [ ] **Step 1: Animate theme card selection**

In ThemeSwitcher.omni, add CSS transition to `.theme-card.active` if not present:

```css
.theme-card.active {
    border-color: var(--primary);
    box-shadow: "0 0 0 3px " var(--primaryLight);
    background: var(--surfaceMuted);
    transform: scale(1.02);
}
```

- [ ] **Step 2: Add keyboard navigation hint to ThemeSwitcher**

In ThemeSwitcher.omni, add a note about keyboard navigation:

```omni
<!-- ═══ 提示 ═══ -->
<div class: hint-bar>
    <span class: hint-text>
        💡 Click a card to switch theme · CSS cascade updates all var() references instantly
    </span>
</div>
```

---

### Task 20: Verify build + E2E

**Files:**
- Run: CLI command
- Run: Browser MCP snapshot

- [ ] **Step 1: Full build**

```bash
cd /d/space/OmniLang/test/projects/learn-omniview
omnic build --target omniview 2>&1
```
Expected: Compilation succeeds, zero errors.

- [ ] **Step 2: Verify no console errors**

Start dev server or open built files. Check browser console for:
- No errors
- No `__omni_theme` runtime errors
- No missing CSS variable warnings

- [ ] **Step 3: Verify 4 themes render correctly**

For each of the 4 themes (light, dark, sepia, highContrast):
1. Click the theme card
2. Verify the UI color scheme changes
3. Check `data-theme` attribute on `<html>` element
4. Verify all 7 pages render without visual issues

- [ ] **Step 4: Verify tab navigation works**

Click through all 7 tabs to confirm:
- Tab highlights correctly
- Content renders
- Theme persists across tab switches

- [ ] **Step 5: Commit**

```bash
cd /d/space/OmniLang
git add test/projects/learn-omniview/src/
git add docs/superpowers/specs/2026-07-30-learn-omniview-redesign-design.md
git add docs/superpowers/plans/2026-07-30-learn-omniview-redesign.md
git commit -m "feat: redesign learn-omniview with 4-theme showcase

- @themes expanded from 2 to 4 themes (light/dark/sepia/highContrast)
- 18 color tokens gain per-theme values for sepia + highContrast
- ThemeStore refactored for N-theme cycling (no hardcoded light/dark)
- ThemeSwitcher rewritten as 4 visual theme cards with color previews
- 10 tabs consolidated into 7 capability domains
- New Overview, UI Components, DesignTheme, State pages added
- Deleted: HomePage, SettingsPage, SupplementPage, InteractivePage
- ElementRefDemo enhanced with drag constraint variations
- All pages verified to render with all 4 themes"
```

---

## Self-Review

### Spec Coverage Check
Looking at the design spec requirements vs. plan tasks:

| Spec Requirement | Covered By |
|-----------------|------------|
| 4 themes (light/dark/sepia/highContrast) | Task 1 (design.omni), Task 2 (ThemeStore), Tasks 3-6 (components) |
| ThemeSwitcher 4-card design | Task 3 ✓ |
| ThemeSourceDemo syntax showcase | Task 4 ✓ |
| DesignTokenPanel live visualizer | Task 5 ✓ |
| VariantShowcase cross-theme demo | Task 6 ✓ |
| 7-tab restructure | Tasks 8-12, 14-15 ✓ |
| Deleted old pages | Task 17 ✓ |
| ElementRefDemo drag constraints | Task 13 ✓ |
| Header theme indicator | Task 16 ✓ |
| Build verification + E2E | Task 20 ✓ |

### Placeholder Scan
- No "TBD"/"TODO" in any step
- All code blocks contain complete component code
- All file paths are exact (repo-relative)
- All build commands specific

### Type/Name Consistency
- ThemeStore mode signal: `"light"` / `"dark"` / `"sepia"` / `"highContrast"` — consistent across all tasks
- Tab names: `"overview"` / `"ui"` / `"motion"` / `"design"` / `"state"` / `"language"` / `"compiler"` — consistent in TabBar and main.omni
- Theme variable names: `themes`, `themeIcons`, `themeLabels` — used consistently

### Edge Cases
- `ThemeStore.setMode("nonexistent")` → rejected by `||` chain (Task 2 ✓)
- `__omni_theme` auto-sync → works because `mode` signal name unchanged (covered by ADR-044)
- No themes declared → `@default-theme` fallback works (built into design block parser)
