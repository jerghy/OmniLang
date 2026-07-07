# OmniView 样式系统重构设计

> 日期: 2026-07-07
> 状态: 设计阶段
> 三轮 subagent 验证完成

## 1. 目标

重新设计 Poiesis 生成应用样式的架构。核心原则：

- **Poiesis 是可视化编辑器** — 可以任意复杂来降低设计成本
- **输出是纯净的 OmniView 代码** — 复用现有语法和编译管道，结构清晰可编译
- **注释标记位置，Omni 语法承载数据** — 只有在无法从语法解析时才使用注释指令或 JSON

## 2. 当前架构分析

### 2.1 现有管线

```
Parser (view/style.rs) → AST (StyleRule/StyleValue) → IR Lowering (StyleRuleIR) → Codegen (per target)
```

### 2.2 核心问题

| 问题 | 详情 |
|------|------|
| 没有设计系统 | `theme AppTheme { }` 存在但 codegen 完全不用，IR lowering 丢弃主题数据 |
| 没有样式复用 | 每个组件独立写 style block，无全局样式/继承/共享 |
| StyleRuleIR 扁平 | 只有 `class_name + HashMap<String, ExprIR>`，选择器是字符串拼接 |
| 伪类是字符串 | `.btn:hover` 的 class_name 就是 `"btn:hover"`，无结构化表示 |
| 主题靠手动 | ThemeStore + CSS var() + setProperty() 全部手写 |
| 无 segment 往返 | `compileSegments()` 把 signal/var-ref 打平成纯文本，加载后类型丢失 |
| `compileSegments()` 有引号 bug | 单文本段输出 `"value"`，每次保存叠加一层引号 |

## 3. 四层样式架构

### 3.1 Layer 1: Design Tokens (`theme.omni`)

**语法：**

```omni
theme AppTheme {
    colors {
        primary: "#6366f1",
        surface: "#ffffff",
        text: "#111827",
        border: "#d4d4d8",
        gray: {
            50: "#f9fafb",
            100: "#f3f4f6",
        }
    }
    spacing {
        xs: "4px", sm: "8px", md: "16px", lg: "24px",
    }
    typography {
        heading: { size: "24px", weight: 700, lineHeight: 1.3 },
        body: { size: "14px", weight: 400, lineHeight: 1.5 },
    }
    radii { sm: "4px", md: "8px", lg: "12px", }

    @mode dark {
        colors {
            primary: "#a5b4fc",
            surface: "#111827",
            text: "#f9fafb",
            border: "#4b5563",
        }
    }
}
```

**Poiesis 内部行为：** 用户通过 Poiesis Token 编辑器管理 token 值，保存时：

- 组件 `.omni` 中 token 引用写成 `var(--color-primary)`——omnic 原样透传
- `theme.omni` 中写 `theme AppTheme { }` 块——omnic 能解析不报错
- CSS 变量解析靠运行时——沿用 learn-omniview 已有模式（ThemeStore + `style: { --var: signal }` → `setProperty()`），无需 omnic 改动

**命名约定：** `{category}-{key}` → `--color-primary`, `--spacing-md`, `--radius-lg`
嵌套 token (`gray.50`) → `--color-gray-50`

### 3.2 Layer 2: Style Recipes (`recipes/*.omni`)

**语法：**

```omni
// recipes/Button.omni
import omniview;

recipe Button {
    base {
        display: inline-flex;
        border-radius: var(--radius-md);
        cursor: pointer;
        font-weight: 600;
    }
    @variant color {
        primary:  { background: var(--color-primary); color: white; }
        danger:   { background: var(--color-danger); color: white; }
        ghost:    { background: transparent; color: var(--color-text); border: 1px solid var(--color-border); }
    }
    @variant size {
        sm: { padding: 4px 8px; font-size: 12px; }
        md: { padding: 8px 16px; font-size: 14px; }
        lg: { padding: 12px 24px; font-size: 16px; }
    }
    @default { color: primary; size: md; }
}
```

**编译器输出（全局 atomic CSS，去重一次生成）：**

```css
.btn-base { display: inline-flex; border-radius: var(--radius-md); cursor: pointer; font-weight: 600; }
.btn-color-primary { background: var(--color-primary); color: white; }
.btn-color-danger { background: var(--color-danger); color: white; }
.btn-color-ghost { background: transparent; color: var(--color-text); border: 1px solid var(--color-border); }
.btn-size-sm { padding: 4px 8px; font-size: 12px; }
.btn-size-md { padding: 8px 16px; font-size: 14px; }
.btn-size-lg { padding: 12px 24px; font-size: 16px; }
```

**命名约定：** 内置常量映射 `{ "Button": "btn", "Badge": "badge", "Link": "lnk", ... }`，其余用 `camel_to_kebab`。检测类名冲突，生成编译错误。

**组件中使用（用 `recipe:` 属性消除歧义）：**

```omni
import { Button } from "../recipes/Button";

component FeatureCard {
    view {
        <button recipe: {Button color="primary" size="sm"}>Click</button>
        <!-- 不传 size → @default 填充 → md -->
        <button recipe: {Button color="danger"}>Delete</button>
    }
}
```

**v1 限制：** `recipe:` 的 variant 值只支持静态字符串。动态值（`variant={signal}`）用 `@if`：

```omni
@if status == "focused" {
    <span recipe: {Badge variant="focused"}>Focused</span>
}
@else {
    <span recipe: {Badge variant="muted"}>Muted</span>
}
```

### 3.3 Layer 3: Style Utilities（可选）

从 Token 自动生成 atomic utility classes，Poiesis 中开关控制。无需新语法。

### 3.4 Layer 4: Component Styles（`components/*.omni`）

保留现有 `style {}` 语法，值升级为 token 引用：

```omni
component FeatureCard {
    view {
        <div class: card>
            <h3 class: title>FeatureCard Demo</h3>
            <button recipe: {Button color="primary" size="sm"} @onPress={reset}>Reset</button>
        </div>
    }
    style {
        .card {
            background: var(--color-surface);
            border: 1px solid var(--color-border);
            border-radius: var(--radius-lg);
            padding: var(--spacing-lg) var(--spacing-md);
        }
        .title { color: var(--color-text); font-size: var(--typography-subheading-size); }
    }
}
```

**CSS 级联规则：**

| 来源 | 选择器 | 特异性 |
|------|--------|--------|
| Theme 变量 | `:root { --color-primary }` | — (继承) |
| Recipe 类 | `.btn-base` | (0,1,0) |
| 组件 style | `[data-component="X"] .card` | (0,2,0) |
| 内联 style | `style="padding: 20px"` | (1,0,0,0) |

组件 scoped style 覆盖 recipe 类（有意为之）。内联覆盖所有（标准 CSS）。

## 4. 注释指令设计

### 4.1 原则

- 注释 = 位置标记（`@pid`），值从 Omni 语法解析
- 只有 `StyleValueSegment` 结构无法从编译器输出恢复时，才用 JSON 注释
- 纯文本属性无需注释

### 4.2 指令表

| 指令 | 用途 | 示例 |
|------|------|------|
| `// @pid:v1_v1` | 元素位置（已有） | 编辑器 PID |
| `// @component id=... category=...` | 组件元数据（已有） | 编辑器分类 |
| `// @prop name=... editor=...` | 属性控件类型（已有） | text/select/color |
| `// @style-prop class=... name=... segments=[...]` | **新增**：段结构 JSON | 仅在含 signal/var-ref/混合段时发出 |

### 4.3 `@style-prop` 格式

```
// @style-prop class=card name=background segments=[{"type":"var-ref","name":"--color-surface"}]
// @style-prop class=card name=border segments=[{"type":"text","value":"1px solid "},{"type":"var-ref","name":"--color-border"}]
// @style-prop class=card name=color segments=[{"type":"signal","name":"isActive","label":"Is Active"}]
```

跳过纯文本属性（如 `cursor: pointer`、`display: flex`）。

**往返验证：** 加载时 `recompile-and-compare`——若 `compileSegments(annotation.segments) !== actualValue`，注释被判定为过时，回退到 text-only。

### 4.4 不需要注释的数据

| 数据 | 解析来源 |
|------|---------|
| Token 定义 | `theme AppTheme { }` 语法 |
| Recipe 定义 | `recipe Button { }` 语法 |
| Recipe 调用 | `recipe: {Button ...}` 属性 |
| 元素→类绑定 | `class: className` 属性 |
| Token 引用 | `var(--token-name)` 字符串 |

## 5. Poiesis 编辑器设计

### 5.1 布局

保持现有三栏布局（组件面板 | Canvas | Inspector），新增 TabType：

```typescript
export type TabType = "component" | "store" | "code" | "theme" | "recipe";
```

| Tab 类型 | 中心区域 | 左侧面板 | 右侧面板 |
|----------|---------|---------|---------|
| `component` | DesignCanvas（现有） | Palette + Tree | ContextInspector（升级） |
| `theme` | Token 编辑器 | 隐藏 | Token 属性面板 |
| `recipe` | Recipe 编辑器 | 隐藏 | Variant 属性面板 |
| `store` / `code` | CodeFlowCanvas（现有） | 隐藏 | 隐藏 |

### 5.2 升级后的 ContextInspector

选中元素时：

```
┌─ ContextInspector ──────────────────────┐
│  div.card                                │
├──────────────────────────────────────────┤
│  [Props]  [Styles]  [Events]  [Recipe]  │
│                                          │
│  ┌─ Class 选择 ──────────────────────┐  │
│  │ 当前: [card ▼]         [重命名]    │  │
│  │ 引用: 3 个元素                     │  │
│  │ [+ 新建 class...]                  │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ┌─ 属性 ────────────────────────────┐  │
│  │ background [🔗 var(--color-...) ▼]│  │
│  │ padding    [🔗 var(--spacing-lg)] │  │
│  │ [+ 添加]                           │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ┌─ 伪类 ────────────────────────────┐  │
│  │ :hover → filter: brightness(1.1)  │  │
│  │ [+ :hover :focus :active ...]     │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ┌─ Recipe ──────────────────────────┐  │
│  │ Recipe: [Button ▼]                  │  │
│  │   color: [primary ▼]               │  │
│  │   size:  [sm ▼]                    │  │
│  │ [解除绑定]                           │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
```

### 5.3 Token Picker

在属性值编辑器旁边显示 `🔗` 按钮，点击弹出：

```
┌─ Token Picker ────────────────────────────┐
│ 🔍 [搜索 token...]                        │
│                                            │
│ ┌ Colors ─────────────────────────────┐   │
│ │ ● primary    #6366f1 → dark: #a5b4fc│   │
│ │ ○ surface    #ffffff → dark: #111827│   │
│ └──────────────────────────────────────┘   │
│ ┌ Spacing ────────────────────────────┐   │
│ │ sm 8px    md 16px    lg 24px        │   │
│ └──────────────────────────────────────┘   │
│                                            │
│ [创建新 token...]                          │
└────────────────────────────────────────────┘
```

### 5.4 预览修复

**`OmniViewRenderer` 需要三处修改：**

1. 注入 `<style>` 标签包含当前组件的 style block CSS + 全局 theme 变量
2. 读取 `classRefs` 并将它们作为 HTML class 应用
3. 添加 dark mode toggle → `data-theme` attribute

### 5.5 Store 改造

```typescript
// 新增状态
styleRegistry: StyleRegistry;           // 每个 Tab 的 class + variable 定义
globalTheme: CssVariable[];              // 全局 theme token（跨 Tab 共享）
elementRecipes: Record<string, RecipeBinding>;  // 元素→recipe 绑定
previewTheme: "light" | "dark";

// 新增 actions
addStyleClass(name, properties)  / removeStyleClass(name)
addClassProperty(className, prop)
setElementRecipe(elementId, recipeName, variants)
setPreviewTheme(mode)
```

## 6. 编译器改造

### 6.1 优先级

| 优先级 | 内容 | 文件数 | 复杂度 |
|--------|------|--------|--------|
| P0 | Lexer: `Recipe`, `AtVariant`, `AtDefault`, `AtMode` tokens | 3 Rust | 低 |
| P0 | AST: `RecipeDef`, `RecipeCall`, `ThemeMode`, 修复 `ThemeDef` | 3 Rust | 中 |
| P0 | Parser: `parse_recipe_def()`, `recipe:` 属性解析, `@mode` | 4 Rust | 中 |
| P1 | IR: `RecipeIR`, `ThemeIR`, `RecipeCallIR`, 修复 lowering | 3 Rust | 中 |
| P1 | Codegen: `emit_theme_css()`, `emit_recipe_css()`, `expand_recipe_call()` | 2 Rust | 中 |
| P1 | Poiesis: `recipe:` 解析/发射 round-trip | 5 TS | 中 |
| P1 | Poiesis: `@style-prop` 注释 round-trip | 9 TS | 中 |
| P2 | Poiesis: StyleRegistry Store + ContextInspector 升级 | 8 TS | 高 |
| P3 | Poiesis: Preview 修复 + dark mode toggle | 3 TS | 中 |
| P4 | Poiesis: TabType 扩展 + Token/Recipe 编辑器 | 6 TS | 高 |
| P5 | Dynamic recipe variant (`variant={signal}`) | 5 Rust + 3 TS | 高 |

### 6.2 CSS 输出顺序

```
1. :root { --color-* }                ← theme 变量
2. [data-theme="dark"] { --color-* }  ← mode 覆盖
3. .btn-base, .btn-color-*, ...       ← recipe 类（全局，去重）
4. [data-component="X"] .card { }    ← 组件 scoped 样式（现有）
```

必须保证此顺序（使用 `IndexMap` 替代 `HashMap`）。

### 6.3 关键设计决策

- **Recipe 类名前缀：** 内置常量映射 + kebab-case fallback。冲突检测 → 编译错误
- **`@default` 缺失：** 编译错误——提示"请指定 variant 或添加 @default"
- **`var()` 验证：** 编译时收集所有 token 名，检查 `var()` 引用是否存在 → warning
- **`@mode` 覆盖不完整：** Warning——"token X 缺少 dark mode 覆盖"
- **ThemeStore 演化：** 保留为 mode toggle（`signal mode + action toggle + data-theme`），删除颜色 getter
- **动态 variant：** v1 不支持，用 `@if` + 静态 recipe 替代

## 7. 已知风险

| 风险 | 缓解 |
|------|------|
| CSS 特异性陷阱：组件 scoped 覆盖 recipe 类 | 文档说明；未来考虑 `@layer` |
| `@media(prefers-color-scheme)` 与 `data-theme` 冲突 | 用 `:not([data-theme])` 守卫 |
| 手动编辑 .omni 导致 `@style-prop` 不同步 | load-time recompile-and-compare 验证 |
| `compileSegments()` 单段引号 bug | 用 `@style-prop` 绕过；单独修复 |
| Recipe 类名冲突（两 recipe → 同前缀） | 内置映射表 + 检测 + 编译错误 |
