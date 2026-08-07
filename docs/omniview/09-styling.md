# 09 — 样式系统

OmniView 组件使用 `style { ... }` 块定义**组件作用域**的 CSS 样式——编译时自动添加 `[data-component="Name"]` 前缀，组件间样式完全隔离。

## 基本语法

```omni
component Card {
    view {
        <div class: container>
            <h2 class: title>Title</h2>
        </div>
    }
    style {
        .container {
            max-width: 800px;
            padding: 16px;
            background: #fff;
        }
        .title {
            font-size: 24px;
            color: #1f2937;
        }
    }
}
```

编译为：
```css
[data-component="Card"] .container { max-width: 800px; ... }
[data-component="Card"] .title { font-size: 24px; ... }
```

## 选择器

### 类选择器

```omni
.button { ... }
.card { ... }
.app-shell { ... }   /* 连字号 */
```

### 标签选择器

```omni
button { cursor: pointer; }
input { border: 1px solid #ddd; }
h1, h2, h3 { font-weight: 600; }
```

### 伪类

```omni
.button:hover { background: #4f46e5; }
.button:focus { outline: 2px solid #6366f1; }
.button:active { transform: scale(0.97); }
```

### 复合选择器

```omni
/* 两个类同时存在 */
.button.primary { font-weight: 700; }
.card.highlighted { border-color: #6366f1; }

/* 复合 + 伪类 */
.var-card.alt:hover { border-color: var(--primary-alt); }

/* 组件变体通过复合选择器实现 */
.ghost.primary { background: transparent; }
```

## 属性值

### 单位

支持 30+ 标准 CSS 单位。数字后缀自动处理：无引号的数字默认加 `px`（如 `16` → `16px`），但部分属性如 `fontWeight` 需用引号字符串 `"700"`。

```omni
width: 200px;
height: 50vh;
font-size: 1.5rem;
transition: 300ms;
opacity: 0.5;
font-weight: "600";
```

**支持的 CSS 单位**：

| 类别 | 单位 |
|------|------|
| 固定 | `px`, `pt`, `cm`, `mm`, `in`, `pc`, `Q` |
| 相对（字体） | `em`, `rem`, `ex`, `ch`, `ic` |
| 相对（视口） | `vh`, `vw`, `vmin`, `vmax`, `svh`, `svw`, `lvh`, `lvw`, `dvh`, `dvw` |
| 相对（容器查询） | `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, `cqmax` |
| 百分比 | `%`, `fr` |
| 时间 | `ms`, `s` |
| 角度 | `deg`, `rad`, `grad`, `turn` |
| 其他 | `lh`, `rlh` |

### 颜色

```omni
color: #1f2937;
background: "#6366f1";           /* 引号 hex */
background: "rgba(0,0,0,0.05)";  /* rgba */
border: "1px solid #d4d4d8";     /* 属性值拼接 */
```

### 多值属性

```omni
margin: "16px 0";
padding: "8px 16px 8px 16px";
border: "1px solid #d4d4d8";
box-shadow: "0 4px 6px rgba(0,0,0,0.1)";
transition: "all 150ms ease";
```

长格式（单独属性）：
```omni
padding-top: 24px;
padding-bottom: 24px;
padding-left: 16px;
padding-right: 16px;
```

## CSS 变量

OmniView 支持 CSS 自定义属性（`--*`）的四种用法：

### 1. Style 块中声明 `--*` 属性

```omni
style {
    .wrapper {
        --card-radius: 12px;
        --accent: #6366f1;
    }
}
```

### 2. Style 块中使用 `var()` 引用

```omni
style {
    .card {
        border-radius: "var(--card-radius)";
        background: "var(--accent)";
    }
}
```

### 3. 内联样式静态 CSS 变量

```omni
view {
    <div style: {
        --box-bg: "#e0e7ff",
        --box-text: "#3730a3",
        --box-radius: "8px"
    }>
        <div class: content>Content</div>
    </div>
}
```

编译器对 `--*` 属性使用 `setProperty()` 输出，而非普通的 `el.style.prop = value`。

### 4. 内联样式动态 CSS 变量（信号 → setProperty）

```omni
component ThemeDemo {
    signal accentColor: String = "#6366f1";
    signal accentHue: i32 = 221;

    view {
        <div style: {
            --accent: accentColor,           // 信号 → 响应式 setProperty
            --accent-h: accentHue            // 数值信号自动 px 后缀
        }>
            <div class: themed-box>
                <p>Current accent: {accentColor}</p>
            </div>
        </div>
    }

    style {
        .themed-box {
            background: "var(--accent)";
            border: "1px solid var(--accent-h)";
        }
    }
}
```

编译器为 `style: { --var: signal }` 生成响应式 `setProperty()` 调用，signal 变化时所有 CSS 变量自动级联。

## 设计系统主题变量

OmniView 的设计系统 `@themes` 声明将令牌编译为 3 层 CSS：

1. **`:root { --name: value }`** — 默认值
2. **`[data-theme="X"] { --name: value }`** — 每主题覆盖（支持 N 个主题）
3. **`@media (prefers-color-scheme: dark)`** — 系统偏好自动检测（仅 `dark` 主题）

组件直接使用 `var(--name)` 引用，无需手动管理主题切换：

```omni
component ThemedPage {
    view {
        <div style: {
            background: "var(--surface)",
            color: "var(--textPrimary)",
            border: "1px solid var(--border)"
        }>
            Theme-aware content
        </div>
    }
}
```

主题切换通过 `data-theme` attribute 控制——设置 `document.documentElement.dataset.theme = "sepia"` 即可切换到 sepia 主题，所有 `var(--name)` 引用自动更新。组件**不需要任何主题感知代码**。

支持 N 个主题（不仅限 light/dark）。详见 [10-设计系统](10-design-system.md)。

## 设计系统引用

优先使用设计系统类，组件 style 块只保留特有样式：

```omni
import { LearnDesign } from "../design";

component DemoCard {
    view {
        <div class: LearnDesign.Card>
            <h3 class: LearnDesign.CardTitle>Title</h3>
            <p class: LearnDesign.Hint>Description</p>
            <button variant: LearnDesign.Button(color=primary, size=md)>
                Action
            </button>
        </div>
    }

    style {
        /* 只写此组件特有的样式 */
    }
}
```

详见 [10-设计系统](10-design-system.md)。

## 内联样式

在元素上使用 `style: { ... }`：

```omni
<div style: {
    display: "flex",
    gap: "12px",
    background: isError ? "#fee2e2" : "#f0fdf4"
}>
```

支持静态值、动态表达式和 CSS 变量。

> **直写通道（ADR-070/073，Phase 1/3）**：`@ref` 元素上的**动态 style 条目**经各目标优化通道发射，不经过框架 diff/reconciliation——React：`useRef` + 直接 DOM 写（`el.style.prop = ...`，零重渲染）；Flutter：`ValueNotifier` + `ListenableBuilder`（零 setState、局部重建）；OmniView：`el.style.setProperty` 直接 DOM 写。激活条件：元素带 `@ref` + 条目为动态表达式 + 不在 `@for` 内（详见 `docs/design/ref-high-frequency-direct-channel.md`）。
> **Flutter 约束**：`width`/`height` 等 numeric widget 参数不接受 String px 值——String→double 类型缺口是**编译期显式诊断**，动态数值请用数字信号（px 显示串由 computed 派生）。

## 样式最佳实践

1. **优先设计系统**：通用颜色、间距、排版定义在 `design` 块中
2. **组件 style 保持聚焦**：只写组件特有样式
3. **语义化类名**：`.card`、`.header`、`.actions`
4. **利用伪类**：`:hover`、`:focus` 提供交互反馈
5. **CSS 属性自动 kebab-case**：`borderRadius` → `border-radius`
6. **使用 CSS 变量做主题**：通过 `--var: signal` 响应式注入，优于每个元素单独绑定
