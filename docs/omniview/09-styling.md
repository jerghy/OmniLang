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
.app-shell { ... }   /* 连字符 */
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
.button.primary { font-weight: 700; }
.card.highlighted { border-color: #6366f1; }
```

## 属性值

### 单位

支持所有标准 CSS 单位：`px` `rem` `em` `vh` `vw` `%` `ms` `s` `deg` 等。

```omni
width: 200px;
height: 50vh;
font-size: 1.5rem;
transition: 300ms;
```

### 颜色

```omni
color: #1f2937;
background: "#6366f1";           /* 引号 hex */
background: "rgba(0,0,0,0.05)";  /* rgba */
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

在 style 块中定义 `--*` 自定义属性：

```omni
style {
    .wrapper {
        --card-radius: 12px;
        --accent: #6366f1;
    }
    .card {
        border-radius: "var(--card-radius)";
    }
}
```

### CSS 变量 + Signal 联动（主题化）

将 signal 注入为 CSS 变量，实现声明式主题切换：

```omni
component ThemedPage {
    @store ThemeStore;

    view {
        <div style: {
            --bg: ThemeStore.currentBg,
            --text: ThemeStore.currentText,
            --primary: ThemeStore.currentPrimary
        }>
            <div class: card>
                <h2>Themed Content</h2>
            </div>
        </div>
    }

    style {
        .card {
            background: "var(--bg)";
            color: "var(--text)";
            border: "1px solid var(--primary)";
        }
    }
}
```

编译器为 `style: { --var: signal }` 生成响应式 `setProperty()` 调用，store 更新时所有 CSS 变量自动级联。

## 设计系统引用

优先使用设计系统类，组件 style 块只保留特有样式：

```omni
import { LearnDesign } from "../design";

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

支持静态值和动态表达式。

## 样式最佳实践

1. **优先设计系统**：通用颜色、间距、排版定义在 `design` 块中
2. **组件 style 保持聚焦**：只写组件特有样式
3. **语义化类名**：`.card`、`.header`、`.actions`
4. **利用伪类**：`:hover`、`:focus` 提供交互反馈
5. **CSS 属性自动 kebab-case**：`borderRadius` → `border-radius`
6. **使用 CSS 变量做主题**：优于每个元素单独绑定 `style: { color: signal }`
