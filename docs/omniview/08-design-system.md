# 08 — 设计系统（Design System）

`design` 块是 OmniView 的一等公民设计系统。它将**设计令牌（Tokens）**、**CSS 类（Classes）**和**组件变体（Variants）**统一在一个声明式定义中。

## 基本结构

```omni
import omniview;

design LearnDesign {
    // 1. 设计令牌
    token primary: color = "#6366f1";

    // 2. CSS 类
    class Card { ... }

    // 3. 组件变体
    variant Button { ... }
}
```

其他组件通过 `import { LearnDesign }` 引用。

## 1. 设计令牌（Design Tokens）

设计令牌是命名的设计变量，编译为 CSS 自定义属性（`--tokenName: value`）：

### 令牌类型

OmniView 支持 **13 种**令牌类型：

| 类型 | 用途 | 示例 |
|------|------|------|
| `color` | 颜色值 | `"#6366f1"` |
| `size` | 尺寸值 | `"16px"` |
| `radius` | 圆角值 | `"8px"` |
| `shadow` | 阴影值 | `"0 4px 6px rgba(0,0,0,0.1)"` |
| `fontFamily` | 字体族 | `"system-ui, sans-serif"` |
| `fontSize` | 字号 | `"14px"` |
| `fontWeight` | 字重 | `"600"` |
| `lineHeight` | 行高 | `"1.5"` |
| `letterSpacing` | 字间距 | `"0.5px"` |
| `duration` | 过渡时长 | `"150ms"` |
| `easing` | 缓动函数 | `"ease"` |
| `opacity` | 透明度 | `"0.5"` |
| `zIndex` | 层级 | `"100"` |

### 定义令牌

```omni
design LearnDesign {
    // ── 品牌色 ──
    token primary: color = "#6366f1";
    token primaryLight: color = "#a5b4fc";
    token primaryDark: color = "#4338ca";

    // ── 语义色 ──
    token success: color = "#10b981";
    token warning: color = "#f59e0b";
    token danger: color = "#ef4444";

    // ── 中性色 ──
    token surface: color = "#ffffff";
    token surfaceMuted: color = "#f9fafb";
    token textPrimary: color = "#111827";
    token textSecondary: color = "#6b7280";
    token border: color = "#d4d4d8";

    // ── 间距 ──
    token spaceXs: size = "4px";
    token spaceSm: size = "8px";
    token spaceMd: size = "16px";
    token spaceLg: size = "24px";
    token spaceXl: size = "32px";

    // ── 圆角 ──
    token radiusSm: radius = "4px";
    token radiusMd: radius = "8px";
    token radiusLg: radius = "12px";
    token radiusFull: radius = "9999px";

    // ── 阴影 ──
    token shadowSm: shadow = "0 1px 2px rgba(0,0,0,0.05)";
    token shadowMd: shadow = "0 4px 6px rgba(0,0,0,0.07)";
    token shadowLg: shadow = "0 10px 25px rgba(0,0,0,0.1)";

    // ── 排版 ──
    token fontFamily: fontFamily = "system-ui, sans-serif";
    token fontSizeSm: fontSize = "13px";
    token fontSizeMd: fontSize = "14px";
    token fontSizeLg: fontSize = "16px";
    token fontSizeXl: fontSize = "18px";
    token fontWeightNormal: fontWeight = "500";
    token fontWeightBold: fontWeight = "700";

    // ── 过渡 ──
    token durationFast: duration = "150ms";
    token durationNormal: duration = "300ms";

    // ── 层级 ──
    token zModal: zIndex = "100";
    token zToast: zIndex = "150";
}
```

### 引用令牌

在 CSS 类中使用 `token(name)` 语法引用令牌：

```omni
class Button {
    background: token(primary);
    padding-top: token(spaceSm);
    padding-bottom: token(spaceSm);
    border-radius: token(radiusMd);
    font-family: token(fontFamily);
    transition: "all " token(durationFast);
}
```

编译后，`token(primary)` → `var(--primary)`。

## 2. CSS 类（Design Classes）

在 design 块中定义可复用的 CSS 类：

```omni
design LearnDesign {
    // ── 页面布局 ──
    class Page {
        max-width: "1200px";
        width: "92vw";
        margin: "0 auto";
        background: token(surfaceMuted);
        min-height: "100vh";
    }

    class Section {
        padding: "0 " token(spaceMd) " " token(spaceMd) " " token(spaceMd);
    }

    class Row {
        display: flex;
        gap: token(spaceSm);
        margin-bottom: token(spaceSm);
    }

    class Col {
        flex: 1;
        min-width: 0;
    }

    // ── 卡片 ──
    class Card {
        background: token(surface);
        border: "1px solid " token(border);
        border-radius: token(radiusLg);
        padding: token(spaceMd);
        margin: "16px 0";
    }

    // ── 输入框 ──
    class Input {
        padding: "12px";
        border: "2px solid " token(border);
        border-radius: token(radiusMd);
        font-size: token(fontSizeLg);
        outline: none;
        width: 100%;
        box-sizing: border-box;

        &:focus {
            border-color: token(primary);
            box-shadow: "0 0 0 3px " token(primaryLight);
        }
    }

    // ── 文本层级 ──
    class CardTitle {
        font-size: token(fontSizeXl);
        font-weight: token(fontWeightBold);
        color: token(textPrimary);
    }

    class Hint {
        font-size: token(fontSizeSm);
        color: token(textSecondary);
    }

    class Small {
        font-size: "11px";
        color: token(textSecondary);
    }
}
```

### CSS 属性值支持

属性值可以是：
- **字符串**：`"14px"`、`"bold"`、`"#ffffff"`
- **令牌引用**：`token(primary)` → 编译为 `var(--primary)`
- **混合拼接**：`"0 4px " token(shadow) " rgba(0,0,0,0.08)"`
- **标识符**（无引号）：`flex`、`none`、`center`、`0`
- **伪类**：`&:hover { ... }`、`&:focus { ... }`、`&:active { ... }`

### 在组件中引用设计类

```omni
view {
    <div class: LearnDesign.Card>
        <h3 class: LearnDesign.CardTitle>标题</h3>
        <p class: LearnDesign.Hint>说明文字</p>
    </div>
}
```

设计类使用 `DesignName.ClassName` 格式引用。

## 3. 组件变体（Variants）

Variant 定义了组件的**多维度参数化**样式变体。

### 定义 Variant

```omni
design LearnDesign {
    variant Button {
        @dim color: primary | danger | ghost | accent;
        @dim size: sm | md | lg;

        @default { color: primary; size: md; }

        // 基础样式（所有变体共享）
        base {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            border: none;
            border-radius: token(radiusMd);
            font-family: token(fontFamily);
            cursor: pointer;
            font-weight: token(fontWeightSemibold);
            transition: "all " token(durationFast);
        }

        // 颜色分支
        color.primary { background: token(primary); color: white; }
        color.danger  { background: token(danger); color: white; }
        color.ghost   { background: transparent; color: token(textPrimary); border: "1px solid " token(border); }
        color.accent  { background: token(accent); color: white; }

        // 尺寸分支
        size.sm {
            padding: "4px 8px";
            font-size: token(fontSizeSm);
        }
        size.md {
            padding: "8px 16px";
            font-size: token(fontSizeMd);
        }
        size.lg {
            padding: "12px 24px";
            font-size: token(fontSizeLg);
        }
    }
}
```

### Variant 的结构

| 部分 | 说明 |
|------|------|
| `@dim name: a \| b \| c` | 定义变体维度及其可选值 |
| `@default { ... }` | 默认维度值（不指定时使用） |
| `base { ... }` | 所有变体共享的基础样式 |
| `dim.value { ... }` | 特定维度值的覆盖样式 |

### 使用 Variant

在组件中使用 `variant:` 属性引用变体：

```omni
<!-- 使用默认（color=primary, size=md） -->
<button variant: LearnDesign.Button()>Default</button>

<!-- 指定所有维度 -->
<button variant: LearnDesign.Button(color=primary, size=md)>Submit</button>

<!-- Danger 大按钮 -->
<button variant: LearnDesign.Button(color=danger, size=lg)>Delete</button>

<!-- Ghost 小按钮 -->
<button variant: LearnDesign.Button(color=ghost, size=sm)>Cancel</button>
```

### Variant 展开

编译时，`variant: LearnDesign.Button(color=primary, size=md)` 展开为对应的 CSS 类名。最终每个 variant 组合会生成独立的 CSS 类。

## 设计系统的导入和使用

### 定义设计系统（design.omni）

```omni
import omniview;

design LearnDesign {
    token primary: color = "#6366f1";
    // ...
    class Card { ... }
    variant Button { ... }
}
```

### 在组件中使用

```omni
import omniview;
import { LearnDesign } from "../design";

component MyPage {
    view {
        <div class: LearnDesign.Card>
            <h3 class: LearnDesign.CardTitle>Welcome</h3>
            <p class: LearnDesign.Hint>Some description</p>
            <button variant: LearnDesign.Button(color=primary, size=md)>
                Get Started
            </button>
        </div>
    }
}
```

## 设计系统最佳实践

1. **令牌优先**：先定义所有颜色、间距、圆角等令牌，类再引用令牌
2. **语义命名**：使用 `primary`、`success`、`danger` 而非 `blue`、`green`、`red`
3. **合理的粒度**：类代表有意义的 UI 模式（Card、Input），不要为单个属性创建类
4. **变体维度 2-3 个**：每个 variant 保持 2-3 个 `@dim`，避免组合爆炸
5. **一个项目一个主设计系统**：通常一个 `design` 块即可覆盖整个项目
