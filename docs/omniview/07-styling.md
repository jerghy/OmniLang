# 07 — 样式系统

OmniView 组件使用 `style { ... }` 块定义**组件作用域**（scoped）的 CSS 样式。每个组件的样式自动隔离，不会影响其他组件。

## 基本语法

```omni
component MyComponent {
    view {
        <div class: container>
            <h1 class: title>Hello</h1>
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
[data-component="MyComponent"] .container { max-width: 800px; padding: 16px; background: #fff; }
[data-component="MyComponent"] .title { font-size: 24px; color: #1f2937; }
```

## 选择器

### 类选择器

```omni
style {
    .button { ... }
    .card { ... }
    .app-shell { ... }   /* 连字符名称 */
}
```

### 标签选择器

```omni
style {
    button { cursor: pointer; }
    input { border: 1px solid #ddd; }
    p { line-height: 1.5; }
    h1, h2, h3 { font-weight: 600; }
}
```

### 伪类

```omni
style {
    .button:hover {
        background: #4f46e5;
    }
    .button:focus {
        outline: 2px solid #6366f1;
    }
    .button:active {
        transform: scale(0.97);
    }
}
```

### 复合选择器

```omni
style {
    .button.primary { font-weight: 700; }
    .button.large.dark { padding: 16px 32px; }
}
```

## 样式属性值

### 尺寸与单位

支持所有标准 CSS 单位：

```omni
style {
    .box {
        width: 200px;
        height: 50vh;
        padding: 1.5rem;
        margin: 2em;
        border-radius: 8px;
        font-size: 14px;
        max-width: 80vw;

        /* 更多单位 */
        gap: 5%;            /* 百分比 */
        line-height: 1.5;   /* 无单位 */
        transition: 300ms;  /* 毫秒 */
        animation: 2s;      /* 秒 */
        rotation: 15deg;    /* 角度 */
    }
}
```

支持的单位列表：
`px`, `rem`, `em`, `vh`, `vw`, `%`, `pt`, `cm`, `mm`, `fr`, `ms`, `s`, `deg`, `rad`, `grad`, `turn`, `vmin`, `vmax`, `svh`, `svw`, `lvh`, `lvw`, `dvh`, `dvw`, `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, `cqmax`, `ch`, `ex`, `ic`, `lh`, `rlh`

### 颜色值

```omni
style {
    .text {
        color: #1f2937;          /* hex */
    }
    .bg {
        background: "#6366f1";  /* 引号 hex */
    }
    .overlay {
        background: "rgba(0,0,0,0.05)";  /* rgba */
    }
}
```

### CSS 函数

```omni
style {
    .transform {
        transform: "rotate(15deg)";
        transform: "translateY(-2px)";
        transform: "scale(1.05)";
    }
    .shadow {
        box-shadow: "0 4px 6px rgba(0,0,0,0.1)";
    }
    .calc {
        width: "calc(100% - 32px)";
    }
}
```

### 多值属性

```omni
style {
    .margin { margin: "16px 0"; }
    .padding { padding: "8px 16px 8px 16px"; }
    .border { border: "1px solid #d4d4d8"; }
    .transition { transition: "all 150ms ease"; }

    /* 长格式（单独属性） */
    .box {
        padding-top: 24px;
        padding-bottom: 24px;
        padding-left: 16px;
        padding-right: 16px;
    }
}
```

## CSS 变量（Custom Properties）

在 `style { }` 块中可以直接定义 `--*` CSS 自定义属性，编译器原生支持：

```omni
style {
    .wrapper {
        --card-radius: 12px;
        --action-h: 48px;
    }

    .themed-card {
        background: "var(--card-bg)";
        border-radius: "var(--card-radius)";
        font-family: "var(--font)";
    }
}
```

**关键机制**：编译器 parser 专门识别 `--` 前缀：
- 解析时：`parse_css_property_name()` 检测 `--` 前缀并按 Token 逐段拼接属性名
- 代码生成时：`emit_style_assignment()` 对 `--*` 属性使用 `el.style.setProperty("--name", value)`，而对普通属性使用 `el.style.prop = value`

### CSS 变量 + 信号联动（主题化实战）

将 store 信号注入为 CSS 变量，实现声明式主题切换：

```omni
component ThemeCssVarsDemo {
    @store ThemeStore;

    view {
        <!-- 在根元素注入所有主题变量 -->
        <div class: theme-root
            style: {
                --theme-bg: ThemeStore.currentBg,
                --theme-text: ThemeStore.currentText,
                --theme-primary: ThemeStore.currentPrimary,
                --theme-border: ThemeStore.currentBorder
            }
        >
            <!-- 子元素只需用 var() 引用 -->
            <div class: themed-card>
                <h4>标题</h4>
                <p>此卡片颜色完全由 CSS 变量驱动</p>
            </div>
        </div>
    }

    style {
        .themed-card {
            background: "var(--theme-bg)";
            color: "var(--theme-text)";
            border: "1px solid var(--theme-border)";
        }
        .themed-card h4 {
            color: "var(--theme-primary)";
        }
    }
}
```

编译器为每个 `style: { --var: signal }` 生成：
```js
__omni_effect(() => {
  const el = __omni_el("nid");
  if (el) el.style.setProperty("--theme-bg", ThemeStore_currentBg.value);
});
```

这样主题切换只需更新 store，所有 CSS 变量自动级联响应。

### calc() 与 CSS 变量组合

```omni
style {
    .card {
        border: "calc(var(--border-w, 2) * 1px) solid var(--theme-border)";
        transform: "scale(var(--theme-scale, 1))";
    }
    .card:hover {
        filter: "brightness(1.1)";
    }
}
```

## 完整样式示例

```omni
component Card {
    signal isHighlighted: bool = false;

    view {
        <div class: card class: { highlighted: isHighlighted }>
            <h2 class: title>Card Title</h2>
            <p class: body>Card content goes here</p>
        </div>
    }

    style {
        .card {
            background: white;
            border: "1px solid #e5e7eb";
            border-radius: 12px;
            padding: 20px;
            margin-bottom: 16px;
            box-shadow: "0 1px 3px rgba(0,0,0,0.05)";
            transition: "all 200ms ease";
        }
        .card:hover {
            box-shadow: "0 4px 12px rgba(0,0,0,0.1)";
            transform: "translateY(-2px)";
        }
        .card.highlighted {
            border-color: "#6366f1";
            box-shadow: "0 0 0 3px rgba(99,102,241,0.2)";
        }
        .title {
            font-size: 18px;
            font-weight: 600;
            color: #1f2937;
            margin: 0 0 8px 0;
        }
        .body {
            font-size: 14px;
            color: #6b7280;
            line-height: 1.6;
            margin: 0;
        }
    }
}
```

## 内联样式

使用 `style: { ... }` 在元素上设置内联样式：

```omni
<div style: {
    display: "flex",
    alignItems: "center",
    gap: "12px",
    padding: "16px",
    background: "#f9fafb"
}>
    Inline styled content
</div>
```

内联样式支持动态值（signal 驱动的样式）：

```omni
<div style: {
    opacity: isLoading ? "0.5" : "1",
    backgroundColor: isError ? "#fee2e2" : "#f0fdf4"
}>
```

## 设计系统样式引用

引用设计系统的类名和变体（详见 [08-设计系统](08-design-system.md)）：

```omni
view {
    <!-- 引用设计系统类 -->
    <div class: LearnDesign.Card>
        <h3 class: LearnDesign.CardTitle>Title</h3>
        <p class: LearnDesign.Hint>Hint text</p>
    </div>

    <!-- 引用设计系统变体 -->
    <button variant: LearnDesign.Button(color=primary, size=md)>
        Click
    </button>
}
```

## 作用域规则

- 每个组件的 `style { ... }` 编译时自动添加 `[data-component="ComponentName"]` 前缀
- 组件之间样式完全隔离——同名类不会冲突
- 设计系统的类使用 `[data-design~="DesignName"]` 作用域

## 样式最佳实践

1. **优先使用设计系统**：通用的颜色、间距、排版应该定义在 `design` 块中
2. **组件样式保持聚焦**：style 块中只写该组件特有的样式
3. **使用语义化类名**：`.card`、`.button`、`.header` 等自解释的名称
4. **利用伪类**：`:hover`、`:focus` 提供交互反馈
5. **注意 CSS 属性自动 kebab-case**：`borderRadius` → `border-radius`
