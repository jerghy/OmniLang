# 10 — 设计系统（Design System）

`design` 块是 OmniView 的一等公民设计系统，统一管理**设计令牌**、**CSS 类**和**组件变体**。

## 基本结构

```omni
import omniview;

design LearnDesign {
    // 1. 令牌 — 编译为 :root { --name: value }
    token primary: color = "#6366f1";

    // 2. 类 — 可复用的 CSS 规则
    class Card { ... }

    // 3. 变体 — 多维度参数化组件样式
    variant Button { ... }
}
```

其他组件通过 `import { LearnDesign } from "../design"` 引用。

---

## 1. 设计令牌（Tokens）

令牌是命名的设计变量，编译为 CSS 自定义属性声明在 `:root` 上。

### 令牌类型（13 种）

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

### 组织令牌

```omni
design LearnDesign {
    // 品牌色
    token primary: color = "#6366f1";
    token primaryDark: color = "#4338ca";

    // 语义色
    token success: color = "#10b981";
    token danger: color = "#ef4444";

    // 中性色
    token surface: color = "#ffffff";
    token textPrimary: color = "#111827";
    token border: color = "#d4d4d8";

    // 间距
    token spaceSm: size = "8px";
    token spaceMd: size = "16px";
    token spaceLg: size = "24px";

    // 圆角
    token radiusMd: radius = "8px";
    token radiusLg: radius = "12px";

    // 过渡
    token durationFast: duration = "150ms";
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

## 4. Theme 关键字（Theme Keyword）

`theme` 是 OmniView 的一等公民关键字（`TokenKind::Theme`），用于对设计令牌进行**元级别分组**。

### 语法

```omni
theme AppTheme {
    colors {
        primary: "#6366f1",
        surface: "#ffffff",
        text: "#111827",
    }
    spacing {
        sm: "8px",
        md: "16px",
        lg: "24px",
    }
    typography {
        fontFamily: "system-ui, sans-serif",
        headingSize: "24px",
        bodySize: "14px",
    }
    radii {
        sm: "4px",
        md: "8px",
        lg: "12px",
    }
}
```

### 与 `design` 关键字的区别

| 维度 | `theme` | `design` |
|------|---------|----------|
| 用途 | 元级别组织：将设计令牌按类别分组 | 定义具体令牌值、CSS 类、组件变体 |
| 分组方式 | 命名类别（colors、spacing、typography 等） | token 声明、class 定义、variant 定义 |
| 输出 | 结构化的令牌分组（未来用于 CSS 自定义属性） | CSS 自定义属性声明、CSS 规则 |
| 层级 | 更高抽象层级 | 实现层级 |

简而言之：`theme` 定义"有哪些类别的令牌"，`design` 定义"每个令牌的具体值是什么以及如何使用"。

### 当前状态

- **解析已完成**：`theme` 块被编译器正确解析为 `ThemeDef` AST 节点
- **代码生成尚未实现**：目前 `theme` 块仅被解析，编译器不会为其生成任何 CSS 输出
- **未来计划**：`theme` 块的类别结构将用于自动生成 CSS 自定义属性分组，例如 `--colors-primary`、`--spacing-md` 等

> **注意**：在代码生成就绪之前，`theme` 块主要用作设计令牌的文档化组织结构。实际样式定义仍需依赖 `design` 块。

---

## 完整示例

**design.omni**：
```omni
import omniview;

design LearnDesign {
    token primary: color = "#6366f1";
    token surface: color = "#ffffff";
    token border: color = "#d4d4d8";

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
