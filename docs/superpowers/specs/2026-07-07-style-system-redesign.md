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

### 3.2 Layer 2: Style Recipes（Poiesis 内部概念）

Recipe 是 Poiesis 编辑器内部设计工具，存储在 Poiesis 元数据中。**不生成 .omni 文件，不进入 omnic 编译。**

**Poiesis 编辑视图（用户设计时看到和操作的）：**

```
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

在组件编辑器中，用户给元素绑定 recipe：`recipe: {Button color="primary" size="sm"}`

**保存时展开为 .omni：**

```
Poiesis 内部                         生成的 .omni（合法现有语法）
─────────────────────────────────    ──────────────────────────────
recipe: {Button color="primary"}  →  class: btn-base btn-color-primary btn-size-md
                                       ↑ 多类名，omnic 照常处理

Recipe 的 base + variant 样式       →  style {
                                         .btn-base { display: inline-flex; ... }
                                         .btn-color-primary { background: var(--color-primary); ... }
                                         .btn-size-sm { padding: 4px 8px; ... }
                                       }
                                       ↑ 展开进组件 style {} 块

多个组件使用同一 recipe              → 每个组件保存时都展开一份
                                       omnic 的 [data-component] 作用域自动隔离
```

**命名约定：** Recipe 名称 → CSS 类名前缀：内置映射表 `{ "Button": "btn", "Badge": "badge", ... }` + kebab-case fallback。Poiesis 保存时检测类名冲突。

**v1 限制：** Recipe 绑定只支持静态 variant 值。动态值（`variant={signal}`）用 `@if` 在每个分支内绑不同的静态 recipe。

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

**CSS 级联规则：** 所有样式都在组件的 scoped style 块内，共享 `[data-component="X"]` 作用域。recipe 展开的类和组件自有类在同一作用域内，CSS 源代码顺序决定优先级（后出现的覆盖前面的）。内联 `style:` 始终最高特异性。

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

### 4.4 不需要注释的数据（从 Omni 语法直接解析）

| 数据 | 解析来源 |
|------|---------|
| Token 定义 | `theme AppTheme { }` 语法 |
| 元素→类绑定 | `class: className` 属性 |
| Token 引用 | `var(--token-name)` 字符串 |
| 伪类 | `.card:hover { }` 在 style 块内 |
| 内联样式 | `style: { key: value }` 元素属性 |

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

## 6. Poiesis → .omni 代码生成

omnic 编译器**零改动**。Poiesis 负责在保存时把所有内部设计概念展开为合法的 .omni 语法。

### 6.1 生成映射表

| Poiesis 内部概念 | 生成的 .omni 代码 | omnic 如何处理 |
|-----------------|------------------|---------------|
| Token 值（如 `primary="#6366f1"`） | `var(--color-primary)` | 原样透传到 CSS |
| Recipe 调用 `{Button color="primary" size="sm"}` | `class: btn-base btn-color-primary btn-size-sm` | 多类名，照常处理 |
| Recipe 样式（base + variant） | `style { .btn-base { ... } .btn-color-primary { ... } }` | `[data-component]` 作用域 |
| Theme `@mode dark` | ThemeStore `getter` + `style: { --var: signal }` + `setProperty()` | 已有运行时模式 |
| 组件 class（如 `.card`） | `class: card` + `style { .card { ... } }` | 现有语法，不变 |
| 伪类（如 `.card:hover`） | `style { .card:hover { ... } }` | 现有语法，不变 |

### 6.2 关键约束

- **Recipe 是 Poiesis 内部概念**——不出现在生成的 .omni 中
- **`recipes/Button.omni` 文件**——Poiesis 设计时使用，单独保存为 Poiesis 元数据（project.json），不作为 .omni 源文件被 omnic 编译
- **`theme.omni`**——Poiesis 数据文件，`theme AppTheme { }` 块可写可不写（omnic 即使看到也会忽略），token 值靠 Poiesis 写入组件 `.omni` 的 style 块
- **每个组件 .omni 独立可编译**——所有依赖（token 值、recipe 样式）都在保存时展开进组件自己的 style 块，omnic 按现有流程编译

### 6.3 Poiesis 实现优先级

| 优先级 | 内容 | 涉及 |
|--------|------|------|
| P0 | StyleRegistry Store：class/variable CRUD | `store/index.ts`, `core/types/style.ts` |
| P0 | 代码生成：recipe 展开 + token 展开 → 写入 .omni | `source-emitter.ts`, `collect-styles.ts` |
| P1 | `@style-prop` 注释 round-trip（segment 结构保存） | `source-emitter.ts`, `parser.ts`, `style-parser.ts` |
| P1 | ContextInspector 升级：Class 选择器 + Token Picker | `ContextInspector.tsx`, `StylesSection.tsx`, `TokenPicker.tsx` |
| P2 | Preview 修复：注入 style 标签 + classRefs + dark mode | `OmniViewRenderer.tsx` |
| P2 | TabType 扩展 + Token 编辑器 Tab | `design-page.ts`, `DesignPage.tsx`, `TokenEditor.tsx` |
| P3 | Recipe 编辑器 Tab + Variant 矩阵 | `RecipeEditor.tsx`, `VariantMatrix.tsx` |
| P4 | Dynamic recipe variant (`variant={signal}` → `@if` 展开) | `source-emitter.ts`, `collect-styles.ts` |

## 7. 已知风险

| 风险 | 缓解 |
|------|------|
| Recipe 展开后类名与组件自有类名冲突 | Poiesis 保存时检测类名冲突，提示用户 |
| 手动编辑 .omni 导致 `@style-prop` 不同步 | load-time recompile-and-compare 验证 |
| `compileSegments()` 单段引号 bug | 用 `@style-prop` 绕过；单独修复 |
| Recipe 修改后需重新保存所有引用组件 | Poiesis 追踪 recipe→component 依赖，提示批量重新保存 |
| 多组件使用同一 recipe，展开后代码量膨胀 | 单个组件 style 块通常 <100 行，可接受 |
