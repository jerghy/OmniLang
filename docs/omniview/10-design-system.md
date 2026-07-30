# 10 — 设计系统（Design System）

`design` 块是 OmniView 的一等公民设计系统，统一管理**设计令牌**、**CSS 类**和**组件变体**。

## 基本结构

```omni
import omniview;

design LearnDesign {
    // 1. 令牌 — 编译为 :root { --name: value } CSS 变量
    //    支持双主题：light 值 dark "dark 值"
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

### 双主题令牌

每个令牌可以为浅色和深色主题分别指定值：

```omni
token surface: color = "#ffffff" dark "#111827";
token textPrimary: color = "#111827" dark "#f9fafb";
token primary: color = "#6366f1" dark "#a5b4fc";
token border: color = "#d4d4d8" dark "#4b5563";
```

编译为：
```css
:root { --surface: "#ffffff"; --textPrimary: "#111827"; --primary: "#6366f1"; }
[data-theme="dark"] { --surface: "#111827"; --textPrimary: "#f9fafb"; --primary: "#a5b4fc"; }
```

### @themes 和 @default-theme 指令

```omni
design LearnDesign {
    @themes light, dark;          // 声明支持的主题名
    @default-theme light;         // 默认主题
    // ...
}
```

- `@themes` 声明该设计系统支持的主题列表
- `@default-theme` 指定初始主题（无 `data-theme` 属性时使用）
- 主题名与令牌值中 `dark` 等关键字匹配

运行时切换只需设置 `document.documentElement.dataset.theme = "dark"`。

### 组织令牌

```omni
design LearnDesign {
    // 品牌色
    token primary: color = "#6366f1" dark "#a5b4fc";
    token primaryDark: color = "#4338ca";

    // 语义色
    token success: color = "#10b981";
    token danger: color = "#ef4444";

    // 中性色（双主题）
    token surface: color = "#ffffff" dark "#111827";
    token textPrimary: color = "#111827" dark "#f9fafb";
    token border: color = "#d4d4d8" dark "#4b5563";

    // 间距
    token spaceSm: size = "4px";
    token spaceMd: size = "16px";
    token spaceLg: size = "24px";

    // 圆角
    token radiusMd: radius = "8px";
    token radiusLg: radius = "12px";

    // 过渡时长
    token durationFast: duration = "150ms";
    token durationNormal: duration = "300ms";

    // 层级
    token zModal: zIndex = "100";
    token zToast: zIndex = "150";
}
```

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
    @themes light, dark;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token surface: color = "#ffffff" dark "#111827";
    token border: color = "#d4d4d8" dark "#4b5563";

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

## 最佳实践

1. **令牌优先**：先定义颜色/间距/圆角等令牌，类再引用令牌
2. **语义命名**：`primary` `success` `danger` 而非 `blue` `green` `red`
3. **合理粒度**：类代表有意义的 UI 模式，不为单属性创建类
4. **变体 2-3 维度**：避免组合爆炸
5. **一个主设计系统**：通常一个 `design` 块覆盖整个项目
6. **双主题**：为常用令牌指定 `dark` 值，运行时只需切换 `data-theme` attribute
