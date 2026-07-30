# 10 — 设计系统（Design System）

`design` 块是 OmniView 的一等公民设计系统，统一管理**设计令牌**、**CSS 类**和**组件变体**。

## 基本结构

```omni
import omniview;

design LearnDesign {
    // 1. 令牌 — 编译为 :root { --name: value } CSS 变量
    //    支持多主题：light 值 dark "dark 值" sepia "sepia值" ...
    token primary: color = "#6366f1" dark "#a5b4fc";

    // 2. 类 — 可复用的 CSS 规则（引用令牌用 token(name)）
    class Card { ... }

    // 3. 变体 — 多维度参数化组件样式
    variant Button { ... }
}
```

其他组件通过 `import { LearnDesign } from "../design"` 引用。

---

## 1. 设计令牌（Tokens）

令牌是命名的设计变量，编译为 CSS 自定义属性声明在 `:root` 上。

### 令牌类型（18 种）

| 类型 | CSS 应用 | 示例 |
|------|---------|------|
| `color` | 颜色属性 | `"#6366f1"` `"rgba(0,0,0,0.5)"` `"white"` |
| `size` | `padding` `margin` `gap` 等 | `"16px"` `"1rem"` |
| `radius` | `border-radius` | `"8px"` `"9999px"` |
| `shadow` | `box-shadow` | `"0 4px 6px rgba(0,0,0,0.1)"` |
| `fontFamily` | `font-family` | `"system-ui, sans-serif"` |
| `fontSize` | `font-size` | `"14px"` |
| `fontWeight` | `font-weight` | `"600"` `"700"` |
| `lineHeight` | `line-height` | `"1.5"` |
| `letterSpacing` | `letter-spacing` | `"0.5px"` |
| `duration` | `transition-duration` | `"150ms"` `"300ms"` |
| `easing` | `transition-timing-function` | `"ease"` |
| `opacity` | `opacity` | `"0.5"` `"1"` |
| `zIndex` | `z-index` | `"10"` `"100"` |

编译后：`token primary: color = "#6366f1"` → `:root { --primary: "#6366f1"; }`

### 多主题令牌

每个令牌可以为多个主题分别指定值：

```omni
@themes light, dark, sepia, highContrast;

token primary: color = "#6366f1" dark "#a5b4fc" sepia "#8B4513" highContrast "#ffff00";
token surface: color = "#ffffff" dark "#111827" sepia "#F5E6C8" highContrast "#000000";
token textPrimary: color = "#111827" dark "#f9fafb" sepia "#3E2723" highContrast "#ffffff";
```

编译为：
```css
:root { --primary: "#6366f1"; --surface: "#ffffff"; --textPrimary: "#111827"; }
[data-theme="dark"] { --primary: "#a5b4fc"; --surface: "#111827"; --textPrimary: "#f9fafb"; }
[data-theme="sepia"] { --primary: "#8B4513"; --surface: "#F5E6C8"; --textPrimary: "#3E2723"; }
[data-theme="highContrast"] { --primary: "#ffff00"; --surface: "#000000"; --textPrimary: "#ffffff"; }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) { --primary: "#a5b4fc"; ... }
}
```

支持**任意数量**的主题——`@themes` 列表中的每个名称都可以出现在每个令牌的值中。未指定主题值的令牌继承默认值（CSS 变量的级联行为）。

### @themes 和 @default-theme 指令

```omni
design LearnDesign {
    @themes light, dark, sepia, highContrast; // 声明 N 个主题
    @default-theme light;                      // 默认主题
    // ...
}
```

- `@themes` 声明该设计系统支持的主题列表（支持 2 个到 N 个）
- `@default-theme` 指定初始主题（无 `data-theme` 属性时使用）
- 主题名与令牌值中的关键字匹配
- 运行时切换只需设置 `document.documentElement.dataset.theme = "themeName"`
- `dark` 主题自动生成 `@media (prefers-color-scheme: dark)` 系统偏好检测

### __omni_theme 运行时

当 design 块声明 `@themes` 时，编译器自动生成 `__omni_theme` JS 运行时：

```js
const __omni_theme = {
  set(name) { /* data-theme + localStorage */ },
  toggle() { /* 在声明主题间循环 */ },
  init()   { /* localStorage → prefers-color-scheme → 默认 */ }
};
__omni_theme.init();
```

组件可以通过信号系统与 `__omni_theme` 集成。`mode` 信号自动同步到 `__omni_theme.set()`（详见 ADR-044）。

### 组织令牌

```omni
design LearnDesign {
    @themes light, dark, sepia, highContrast;
    @default-theme light;

    // 品牌色（4 主题值）
    token primary: color = "#6366f1" dark "#a5b4fc" sepia "#8B4513" highContrast "#ffff00";
    token primaryLight: color = "#a5b4fc" sepia "#D2A679" highContrast "#ffff99";
    token primaryDark: color = "#4338ca" sepia "#5D3A1A" highContrast "#cccc00";

    // 语义色（4 主题值）
    token success: color = "#10b981" sepia "#6B8E23" highContrast "#00ff00";
    token danger: color = "#ef4444" sepia "#A0522D" highContrast "#ff4444";
    token warning: color = "#f59e0b" sepia "#CC7722" highContrast "#ffaa00";

    // 中性色（4 主题值）
    token surface: color = "#ffffff" dark "#111827" sepia "#F5E6C8" highContrast "#000000";
    token surfaceMuted: color = "#f9fafb" dark "#1f2937" sepia "#EDE0C8" highContrast "#0a0a0a";
    token textPrimary: color = "#111827" dark "#f9fafb" sepia "#3E2723" highContrast "#ffffff";
    token textSecondary: color = "#6b7280" dark "#9ca3af" sepia "#5D4037" highContrast "#cccccc";
    token border: color = "#d4d4d8" dark "#4b5563" sepia "#BCAAA4" highContrast "#666666";

    // 间距（无主题变体）
    token spaceSm: size = "4px";
    token spaceMd: size = "16px";
    token spaceLg: size = "24px";

    // 圆角（无主题变体）
    token radiusMd: radius = "8px";
    token radiusLg: radius = "12px";

    // 过渡时长（无主题变体）
    token durationFast: duration = "150ms";
    token durationNormal: duration = "300ms";

    // 层级（无主题变体）
    token zModal: zIndex = "100";
    token zToast: zIndex = "150";
}
```

**注意**：非颜色令牌（spacing、radius、shadow、typography、duration、easing、opacity、zIndex）通常不随主题变化，因此不需要每主题值。CSS 变量的级联机制会自动继承默认值。

---

## 2. CSS 类（Design Classes）

在 design 块中定义可复用的 CSS 类，属性值通过 `token(name)` 引用令牌：

```omni
design LearnDesign {
    class Page {
        max-width: "1200px";
        margin: "0 auto";
        background: token(surfaceMuted);
    }

    class Card {
        background: token(surface);
        border: "1px solid " token(border);
        border-radius: token(radiusLg);
        padding: token(spaceMd);
    }

    class CardTitle {
        font-size: token(fontSizeXl);
        font-weight: token(fontWeightBold);
        color: token(textPrimary);
    }

    class Input {
        padding: "12px";
        border: "2px solid " token(border);
        border-radius: token(radiusMd);
        font-size: token(fontSizeLg);
        outline: none;

        &:focus {
            border-color: token(primary);
            box-shadow: "0 0 0 3px " token(primaryLight);
        }
    }
}
```

属性值支持：
- **字符串**：`"12px"` `"white"`
- **令牌引用**：`token(primary)` → `var(--primary)`
- **拼接**：`"0 4px " token(shadow) " rgba(0,0,0,0.08)"`
- **标识符**（无引号）：`flex` `none` `center`
- **伪类**：`&:hover` `&:focus` `&:active`

### 在组件中使用

```omni
view {
    <div class: LearnDesign.Card>
        <h3 class: LearnDesign.CardTitle>Title</h3>
        <p class: LearnDesign.Hint>Description</p>
    </div>
}
```

---

## 3. 组件变体（Variants）

Variant 提供多维度的参数化组件样式：

```omni
design LearnDesign {
    variant Button {
        @dim color: primary | danger | ghost | accent;
        @dim size: sm | md | lg;

        @default { color: primary; size: md; }

        base {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            border: none;
            border-radius: token(radiusMd);
            cursor: pointer;
            font-weight: token(fontWeightSemibold);
            transition: "all " token(durationFast);
        }

        color.primary { background: token(primary); color: white; }
        color.danger  { background: token(danger); color: white; }
        color.ghost   { background: transparent; color: token(textPrimary); border: "1px solid " token(border); }
        color.accent  { background: token(accent); color: white; }

        size.sm { padding: "4px 8px"; font-size: token(fontSizeSm); }
        size.md { padding: "8px 16px"; font-size: token(fontSizeMd); }
        size.lg { padding: "12px 24px"; font-size: token(fontSizeLg); }
    }
}
```

### Variant 结构

| 部分 | 说明 |
|------|------|
| `@dim name: a \| b \| c` | 变体维度及可选值 |
| `@default { ... }` | 默认值（调用时不指定时使用） |
| `base { ... }` | 所有变体共享的基础样式 |
| `dim.value { ... }` | 特定维度值的覆盖样式 |

### 在组件中使用

```omni
<!-- 使用默认值（color=primary, size=md） -->
<button variant: LearnDesign.Button()>Default</button>

<!-- 显式指定 -->
<button variant: LearnDesign.Button(color=primary, size=md)>Submit</button>
<button variant: LearnDesign.Button(color=danger, size=lg)>Delete</button>
<button variant: LearnDesign.Button(color=ghost, size=sm)>Cancel</button>
```

编译时，variant 展开为对应的 CSS 类名组合。

---

## 完整示例

**design.omni**：
```omni
import omniview;

design LearnDesign {
    @themes light, dark, sepia, highContrast;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc" sepia "#8B4513" highContrast "#ffff00";
    token surface: color = "#ffffff" dark "#111827" sepia "#F5E6C8" highContrast "#000000";
    token textPrimary: color = "#111827" dark "#f9fafb" sepia "#3E2723" highContrast "#ffffff";
    token border: color = "#d4d4d8" dark "#4b5563" sepia "#BCAAA4" highContrast "#666666";

    class Card {
        background: token(surface);
        border: "1px solid " token(border);
        border-radius: "12px";
        padding: "20px";
    }

    variant Button {
        @dim color: primary | danger;
        @dim size: sm | md;
        @default { color: primary; size: md; }

        base { border: none; border-radius: "8px"; cursor: pointer; }
        color.primary { background: token(primary); color: white; }
        color.danger { background: "#ef4444"; color: white; }
        size.sm { padding: "4px 12px"; font-size: "13px"; }
        size.md { padding: "8px 20px"; font-size: "14px"; }
    }
}
```

**组件**：
```omni
import omniview;
import { LearnDesign } from "../design";

component MyPage {
    view {
        <div class: LearnDesign.Card>
            <h2>Welcome</h2>
            <button variant: LearnDesign.Button(color=primary, size=md)>
                Get Started
            </button>
        </div>
    }
}
```

## 多主题实践（learn-omniview 演示）

learn-omniview 项目演示了完整的 4 主题系统。关键集成点：

### 设计块声明

```
@themes light, dark, sepia, highContrast;
@default-theme light;
```

每个颜色令牌提供 4 个主题值。非颜色令牌（间距、圆角等）不随主题变化。

### ThemeStore（信号层）

```omni
store ThemeStore {
    signal mode: String = "light";
    // ADR-044: mode 信号自动同步到 __omni_theme.set()
    action toggle() { /* light → dark → sepia → highContrast → light */ }
    action setMode(newMode: String) { /* 验证后设置 */ }
}
```

### ThemeSwitcher（UI 层）

ThemeSwitcher 展示 4 个可视化主题卡片，每张显示主题名称 + 主色调色板。点击即切换。

### CSS 变量层

所有组件通过 `var(--surface)`, `var(--textPrimary)` 等 CSS 变量引用主题值。主题切换时，`__omni_theme.set()` 更新 `data-theme` 属性，CSS 级联自动更新全部变量。**组件不需要任何主题感知代码。**

## 最佳实践

1. **令牌优先**：先定义颜色/间距/圆角等令牌，类再引用令牌
2. **语义命名**：`primary` `success` `danger` 而非 `blue` `green` `red`
3. **合理粒度**：类代表有意义的 UI 模式，不为单属性创建类
4. **变体 2-3 维度**：避免组合爆炸
5. **一个主设计系统**：通常一个 `design` 块覆盖整个项目
6. **多主题**：`@themes` 支持 N 个主题，为颜色令牌指定各主题值；运行时只需切换 `data-theme` attribute
