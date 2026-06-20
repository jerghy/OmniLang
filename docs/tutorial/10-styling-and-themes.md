# 10. 样式与主题

> **目标**：用作用域 CSS、设计令牌和内联样式美化 OmniView 组件。

OmniView 为每个组件提供 CSS-in-JS 样式，样式作用域限定在组件内。一个组件定义的样式不会泄露到其他组件。

---

## 作用域 CSS：`style` 块

在组件中用 `style { ... }` 块定义样式：

```omni
component Card {
    signal title: String = "你好";

    view {
        <div class: card>
            <h2 class: title>{title}</h2>
            <p class: body>卡片内容在这里</p>
        </div>
    }

    style {
        .card {
            padding: 16px;
            border-radius: 8px;
            background-color: #ffffff;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
        }

        .title {
            font-size: 20px;
            font-weight: bold;
            margin-bottom: 8px;
        }

        .body {
            font-size: 14px;
            color: #666666;
        }
    }
}
```

### 作用域原理

每个 CSS class 自动限定在组件作用域内。不同组件可以安全地使用同名 class 而不会冲突：

```omni
component ComponentA {
    view { <div class: box>A</div> }
    style {
        .box { background-color: red; }    // 只影响 ComponentA
    }
}

component ComponentB {
    view { <div class: box>B</div> }
    style {
        .box { background-color: blue; }   // 只影响 ComponentB
    }
}
```

---

## 应用样式类

使用 `class:` 属性（不是 `class=`）来应用作用域样式：

```omni
<div class: container>    <!-- 应用作用域 .container 样式 -->
    <h1 class: title>你好</h1>
</div>
```

> **注意**：`class:`（带冒号）应用组件作用域样式。全局 class 和内联样式见下文。

---

## 内联样式

对于动态或一次性样式，使用 `style:` 属性配合映射：

```omni
component DynamicBanner {
    signal bgColor: String = "#3498db";
    signal fontSize: i32 = 24;

    view {
        <div style: {
            background-color: bgColor,
            font-size: fontSize + "px",
            padding: "16px",
            color: "white",
        }>
            动态横幅
        </div>
    }
}
```

> CSS 属性名使用连字符形式：`background-color`、`font-size`、`margin-left`。

---

## 支持的 CSS 属性

OmniView 支持标准 CSS 属性。以下是常用属性：

### 布局
- `display`（`block`, `flex`, `grid`, `none`）
- `flex-direction`、`justify-content`、`align-items`
- `gap`、`grid-template-columns`

### 间距
- `margin`、`margin-top`、`margin-right`、`margin-bottom`、`margin-left`
- `padding`、`padding-top`、`padding-right`、`padding-bottom`、`padding-left`

### 尺寸
- `width`、`height`、`min-width`、`max-width`、`min-height`、`max-height`

### 排版
- `font-size`、`font-weight`、`font-family`
- `line-height`、`text-align`、`text-decoration`
- `color`

### 视觉效果
- `background-color`、`background-image`
- `border`、`border-radius`
- `box-shadow`、`opacity`
- `cursor`

### 定位
- `position`、`top`、`right`、`bottom`、`left`
- `z-index`

---

## 主题：设计令牌

**主题（Theme）** 定义可复用的设计令牌——颜色、间距、排版——组件可以引用它们：

```omni
theme AppTheme {
    colors {
        primary: "#3498db",
        secondary: "#2ecc71",
        danger: "#e74c3c",
        text: "#2c3e50",
        background: "#f8f9fa",
    }

    spacing {
        small: "8px",
        medium: "16px",
        large: "24px",
        xlarge: "48px",
    }

    typography {
        heading: "24px",
        body: "16px",
        caption: "12px",
    }
}
```

### 在样式中使用主题令牌

在组件样式中引用主题令牌：

```omni
component ThemedCard {
    view {
        <div class: card>
            <h2 class: heading">主题卡片</h2>
            <p class: text">使用设计令牌保持一致的样式。</p>
        </div>
    }

    style {
        .card {
            padding: var(--spacing-medium);
            background-color: var(--colors-background);
            border-radius: 8px;
        }

        .heading {
            font-size: var(--typography-heading);
            color: var(--colors-primary);
        }

        .text {
            font-size: var(--typography-body);
            color: var(--colors-text);
        }
    }
}
```

---

## 条件样式

结合 class 和内联样式与信号实现动态外观：

```omni
component StatusIndicator {
    signal isActive: bool = true;
    signal severity: String = "info";    // "info" | "warning" | "error"

    view {
        <div
            class: if isActive { "active" } else { "inactive" }
            style: {
                background-color: if severity == "error" {
                    "#e74c3c"
                } else if severity == "warning" {
                    "#f39c12"
                } else {
                    "#3498db"
                },
                padding: "8px 16px",
                color: "white",
                border-radius: "4px",
            }
        >
            {if isActive { "活跃" } else { "离线" }}
        </div>
    }
}
```

---

## 响应式布局

```omni
component ResponsiveGrid {
    signal columns: i32 = 3;

    view {
        <div class: grid>
            <div class: item>项目 1</div>
            <div class: item>项目 2</div>
            <div class: item>项目 3</div>
            <div class: item>项目 4</div>
            <div class: item>项目 5</div>
            <div class: item>项目 6</div>
        </div>
    }

    style {
        .grid {
            display: grid;
            grid-template-columns: "repeat(" + columns.to_string() + ", 1fr)";
            gap: 16px;
        }

        .item {
            padding: 24px;
            background-color: #f0f0f0;
            border-radius: 8px;
            text-align: center;
        }
    }
}
```

---

## 综合示例：带样式的仪表板

```omni
import omniview;

theme DashboardTheme {
    colors {
        primary: "#6366f1",
        success: "#22c55e",
        warning: "#f59e0b",
        bg: "#f8fafc",
        surface: "#ffffff",
        text: "#1e293b",
        muted: "#94a3b8",
    }
    spacing {
        sm: "8px",
        md: "16px",
        lg: "24px",
    }
}

component KpiCard {
    prop label: String;
    prop value: String;
    signal trend: String = "up";

    view {
        <div class: card>
            <span class: label>{label}</span>
            <span class: value>{value}</span>
            <span class: trend>{if trend == "up" { "↑" } else { "↓" }}</span>
        </div>
    }

    style {
        .card {
            background-color: var(--colors-surface);
            padding: var(--spacing-md);
            border-radius: 12px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
            display: flex;
            flex-direction: column;
            gap: 4px;
        }
        .label {
            font-size: 14px;
            color: var(--colors-muted);
        }
        .value {
            font-size: 32px;
            font-weight: bold;
            color: var(--colors-text);
        }
        .trend {
            font-size: 20px;
            color: var(--colors-success);
        }
    }
}

component Dashboard {
    view {
        <div class: container>
            <h1 class: heading>仪表板</h1>
            <div class: grid>
                <KpiCard label="收入" value="¥12,345" />
                <KpiCard label="用户" value="1,234" />
                <KpiCard label="转化率" value="3.2%" />
                <KpiCard label="流失率" value="1.1%" />
            </div>
        </div>
    }

    style {
        .container {
            padding: var(--spacing-lg);
            background-color: var(--colors-bg);
            min-height: 100vh;
        }
        .heading {
            font-size: 28px;
            color: var(--colors-text);
            margin-bottom: var(--spacing-md);
        }
        .grid {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: var(--spacing-md);
        }
    }
}
```

---

## 样式最佳实践

1. **用主题管理共享值**——颜色、间距、排版放在主题中
2. **样式作用域限定在组件内**——避免全局样式泄露
3. **动态值用内联样式**——当值依赖信号时使用 `style:`
4. **Class 名语义化**——`.card`、`.heading`、`.sidebar`，而不是 `.red-box`
5. **样式靠近模板**——`style` 块紧跟在 `view` 后面

---

## 下一步

- **[7. OmniView 组件](07-omniview-components.md)** — 组件基础
- **[8. 响应式与状态](08-reactivity-and-state.md)** — 用信号实现动态样式
- **[9. Store 与 Action](09-stores-and-actions.md)** — 跨组件的主题
