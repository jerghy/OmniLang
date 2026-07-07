# Poiesis 样式面板重新设计

> 日期: 2026-07-07
> 状态: 设计完成（待进入实现计划）
> 取代: `2026-07-07-style-system-redesign.md`（该设计存在问题，仅部分借鉴）

## 1. 核心理念

**Poiesis 是 OmniView 代码的可视化编辑器。** 所有可视化抽象最终必须产出合法的 `.omni` 代码。样式编辑的 UX 对标 Figma/Webflow 设计工具，而不是 Chrome DevTools 代码编辑器。

三层约束：
- **输出是 .omni 代码** — 合法语法，可被 omnic 编译
- **omni 语法原生承载设计概念** — Token/Class/Variant 由 omnic 编译时验证
- **可视化优先** — 用户用颜色选择器、滑块、拖拽操作，不手写代码

## 2. Omni 语法设计

### 2.1 `design` 块 — 设计系统一等公民

新增顶层关键字 `design`，替代当前未被 codegen 使用的 `theme`。独立文件（`design.omni`），组件通过 import 引用。

```omni
// design.omni —— 设计系统定义文件
import omniview;

design AppDesign {
    // ── Token：原子设计变量，带类型标注 ──
    token primary: color = "#6366f1";
    token primaryDark: color = "#4338ca";
    token surface: color = "#ffffff";
    token text: color = "#111827";
    token border: color = "#d4d4d8";
    token danger: color = "#ef4444";
    token success: color = "#10b981";

    token spacingXs: size = "4px";
    token spacingSm: size = "8px";
    token spacingMd: size = "16px";
    token spacingLg: size = "24px";

    token radiusSm: size = "4px";
    token radiusMd: size = "8px";
    token radiusLg: size = "12px";

    token fontHeading: font = "bold 24px/1.3 system-ui";
    token fontBody: font = "14px/1.5 system-ui";

    // ── Class：可复用样式块 ──
    class Card {
        background: token(surface);
        border: 1px solid token(border);
        border-radius: token(radiusMd);
        padding: token(spacingLg) token(spacingMd);

        &:hover {
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
        }
    }

    // ── Variant：多维组合变体 ──
    variant Button {
        @dim color: primary | danger | ghost;
        @dim size: sm | md | lg;

        @default { color: primary; size: md; }

        base {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            border-radius: token(radiusMd);
            cursor: pointer;
            font-weight: 600;
            border: none;
        }
        color.primary { background: token(primary); color: white; }
        color.danger  { background: token(danger); color: white; }
        color.ghost   { background: transparent; color: token(text); border: 1px solid token(border); }
        size.sm { padding: token(spacingXs) token(spacingSm); font-size: 12px; }
        size.md { padding: token(spacingSm) token(spacingMd); font-size: 14px; }
        size.lg { padding: token(spacingMd) token(spacingLg); font-size: 16px; }
    }
}
```

### 2.2 组件中使用

```omni
// components/FeatureCard.omni
import omniview;
import { AppDesign } from "../design";

component FeatureCard {
    view {
        <div class: AppDesign.Card>
            <h3>FeatureCard Demo</h3>
            <button variant: AppDesign.Button(color=primary, size=md)
                    @onPress={handleClick}>
                Click Me
            </button>
        </div>
    }
}
```

### 2.3 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| Token 类型标注 | `token name: color\|size\|font = value` | Poiesis 据此选 picker 控件，omnic 编译时验证类型 |
| Class 引用 token | `token(name)` 函数式引用 | 编译时解析，区别于 CSS 运行时 `var()` |
| Variant 维度 | `@dim name: a \| b \| c` | 显式声明合法值，omnic 验证 |
| 元素绑定 | `variant: Design.Button(color=primary)` | 语义独立于普通 class |
| Class 绑定 | `class: AppDesign.Card` | 复用现有 `class:` 语法 + 命名空间 |
| 伪类 | `&:hover` | 对标 SCSS，简洁直观 |

### 2.4 v1 范围

| 能力 | v1 | 说明 |
|------|-----|------|
| Design Token | ✓ | 颜色、间距、字体、圆角 |
| Style Class | ✓ | 可复用命名样式，含伪类 |
| Variant | ✓ | 多维组合变体 |
| 主题/模式切换 | v2 | light/dark mode |
| 响应式断点 | v2 | 按屏幕尺寸切换 |
| 样式继承/组合 | v2 | class extend 另一个 class |

## 3. Poiesis 编辑器 UI 设计

### 3.1 整体布局

三栏布局不变。新增 TabType `"design"`——打开 `design.omni` 时中心区域变为可视化设计系统编辑器，不再是 Canvas。

| Tab 类型 | 中心区域 | 左侧面板 | 右侧面板 |
|----------|---------|---------|---------|
| `component` | DesignCanvas（现有） | Palette + Tree | ContextInspector（升级） |
| `design` | Token/Class/Variant 可视化编辑器 | FileTree | 隐藏（或辅助属性） |
| `store` / `code` | CodeFlowCanvas（现有） | 隐藏 | 隐藏 |

### 3.2 核心交互原则

来自 Figma/Webflow/Framer/Tokens Studio 的最佳实践：

| 原则 | 含义 |
|------|------|
| **HUD > 弹窗** | 颜色选择器定位在点击处，不遮挡画布，无 OK 按钮 |
| **拖拽拖动数字** | 每个数值输入支持 click+drag 左右调值，Shift=10x |
| **即时画布预览** | 改 token/class 值，所有引用立即刷新 |
| **不显示代码** | 用户看到色板 chip 和"圆角"，不是 `var(--...)` 和 `border-radius` |
| **继承可视化** | Webflow 风格颜色编码：蓝=已设置，灰=继承自 base |
| **撤销一切** | Cmd+Z 撤销颜色修改、class 创建、variant 变更 |
| **自动保存** | 失焦即提交，无 Apply/Save 按钮 |

### 3.3 Token 编辑器（design.omni 中心区域）

```
┌─ design.omni ───────────────────────────────────────────────────┐
│ [Tokens]  [Classes]  [Variants]                       [Code ▸] │
├─────────────────────────────────────────────────────────────────┤
│ 🔍 搜索 token...                 [+ 新建 token]  [☐ 调色板视图] │
│                                                                 │
│ ┌─ Colors (12) ─────────────────────────────────────────────┐  │
│ │ ● primary    [#6366f1]  ←──[████████░░░░]──→    4 uses    │  │
│ │ ● surface    [#ffffff]  ←──[████████░░░░]──→    8 uses    │  │
│ │ ● text       [#111827]  ←──[████████░░░░]──→   12 uses    │  │
│ │ ● border     [#d4d4d8]  ←──[████████░░░░]──→    3 uses    │  │
│ │ [+ 添加颜色]                                               │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ ┌─ Spacing (5) ─────────────────────────────────────────────┐  │
│ │ xs  [ 4][px ▼]  ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░    1 use   │  │
│ │ sm  [ 8][px ▼]  ████░░░░░░░░░░░░░░░░░░░░░░░░░░    5 uses  │  │
│ │ md  [16][px ▼]  ████████░░░░░░░░░░░░░░░░░░░░░░   12 uses  │  │
│ │ lg  [24][px ▼]  ████████████░░░░░░░░░░░░░░░░░░    7 uses  │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ ┌─ Typography (3) ──────────────────────────────────────────┐  │
│ │ heading  [bold 24px/1.3 system-ui  ]  ✎     3 uses        │  │
│ │ body     [14px/1.5 system-ui       ]  ✎     8 uses        │  │
│ └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**关键交互：**
- **颜色 token**：点 `●` 色块 → HUD 取色器（Saturation/Value 平面 + Hue 条 + Alpha 条 + 最近使用色 + 吸管）。定位在色块旁，不挡住画布，点外部关闭。
- **间距 token**：数字 drag-scrub + 右侧可视化比例尺。点比例尺直接设值（吸附到最近整数）。单位下拉只显示合理选项（px, rem, em）。
- **字体 token**：内联编辑，font-family 下拉搜索系统字体栈。
- **右键 token**：复制值 / 复制引用 / 查找使用 / 重命名 / 生成色板 / 删除。
- **拖拽重排** token 顺序。
- **调色板视图**：所有颜色以 grid 平铺，hover 显示使用次数 tooltip。
- **`[Code ▸]`**：切换到生成的 .omni 源码只读预览。
- **删除 token 保护**：显示"N 个元素使用此 token"，需确认后才能删除。

### 3.4 Class 编辑器

```
┌─ Classes ───────────────────────────────────────────────────────┐
│ [Card] [BtnPrimary] [BtnDanger] [ShadowSm]  [+ 新建]           │
│                                                                 │
│ ┌─ 编辑: .Card ────────────────────────────────────────────┐   │
│ │  [Base]  [:hover]  [:focus]  [+ 伪类]      3 elements    │   │
│ │                                                           │   │
│ │  ┌─ 布局 ────────────────────────────────────────────┐   │
│ │  │  Display:  [flex          ▼]                      │   │
│ │  │  Direction:[column        ▼]                      │   │
│ │  │  Padding:  ┌─ 16 ─┐ ┌─ 16 ─┐   四边联动/独立切换  │   │
│ │  │            └─ 16 ─┘ └─ 16 ─┘                     │   │
│ │  │  Gap:      [8      ▤]                            │   │
│ │  └────────────────────────────────────────────────────┘   │
│ │                                                           │
│ │  ┌─ 视觉 ────────────────────────────────────────────┐   │
│ │  │  Background: [● var(--surface) 🔗]  [× 解除]      │   │
│ │  │  Border:     [1px solid var(--border)]            │   │
│ │  │  Radius:     [12     ▤]                           │   │
│ │  └────────────────────────────────────────────────────┘   │
│ │                                                           │
│ │  ┌─ 文字 ────────────────────────────────────────────┐   │
│ │  │  Font:  [system-ui     ▼]                         │   │
│ │  │  Size:  [14     ▤]                                │   │
│ │  │  Color: [● var(--text) 🔗]                        │   │
│ │  └────────────────────────────────────────────────────┘   │
│ │                                                           │
│ │  [+ 添加属性]                                              │
│ └───────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

**关键交互：**
- **属性分组**：Layout / 视觉 / 文字 三个折叠区域，和 Figma 一致
- **伪类 tabs**：切到 `:hover` 时**只显示与 Base 不同的属性**，蓝色圆点标记。点圆点 = 重置为 Base 值
- **属性值 `🔗`**：亮蓝 = 已绑 token，灰 = 未绑。点 `🔗` → Token Picker（搜索式下拉 + 色板预览）。也可从左侧 Token 列表拖到属性值上
- **值来源颜色编码**：当前 class 直接设置 → 正常色；从 variant base 继承 → 灰色 + tooltip
- **四边联动**：padding/margin 默认四边相同（单值），点链接图标展开四个独立字段
- **[+ 添加属性]**：可搜索下拉框
- **属性标签用设计师术语**："圆角" 而非 `border-radius`，CSS 属性名作为次级提示

### 3.5 Variant 编辑器

三个模式：

**模式 1: 维度树（创作/编辑）**

```
┌─ Variants ──────────────────────────────────────────────────────┐
│ [Button ▼]  [+ 新建 variant]                                    │
│                                                                 │
│ Dimensions:                                                     │
│ ┌ color ──── [primary] [danger] [ghost]           [+ 值] ───┐ │
│ │ size  ──── [sm] [md] [lg]                       [+ 值] ───┘ │
│ └──────────────────────────────────────────────────────────┘  │
│ [+ 添加维度]                                                    │
│                                                                 │
│ 默认组合: color=primary, size=md                                │
│                                                                 │
│ ┌─ 编辑: color=primary ───────────────────────────────────┐    │
│ │  Background: [● var(--primary) 🔗]                       │    │
│ │  Color:      [white        ]                             │    │
│ │  (仅显示与 base 不同的属性 · 灰色=继承自 base)             │    │
│ └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

**模式 2: 矩阵预览（概览）**

- 行列 pivot 选择器选择两个维度
- 每个 cell 实时渲染该 variant 组合的 mini 预览
- 有独立覆盖的 cell 蓝色角标
- 点 cell 编辑该特定组合，多选批量编辑

**模式 3: 列表审计**

全部组合平铺 + 大预览，用于导出截图或全局检查。

**Variant 继承语义：**
- `base` 定义所有 variant 共享的默认样式
- `color.primary` 只在 color=primary 时叠加到 base
- `color=primary size=lg` 同时叠加 color.primary + size.lg（按定义顺序）
- 特定组合可独立覆盖（如 `color=primary size=lg` 有额外 padding-top）
- 继承属性显示为灰色，override 显示为正常色

### 3.6 元素选中时的右侧面板（ContextInspector）

在组件标签页中选中 Canvas 元素时：

```
┌─ ContextInspector ────────────────┐
│  div (选中元素)                     │
├────────────────────────────────────┤
│                                    │
│  ┌─ Classes ──────────────────┐   │
│  │  [AppDesign.Card ×]         │   │
│  │  [🔍 搜索或创建 class...]    │   │
│  │  ▸ AppDesign.ShadowSm  (3)  │   │
│  │  ▸ AppDesign.Container (5)  │   │
│  └─────────────────────────────┘   │
│                                    │
│  ┌─ Variant ──────────────────┐   │
│  │  Button                     │   │
│  │  color: [primary ▾]         │   │
│  │  size:  [md ▾]              │   │
│  │  [解除 variant]              │   │
│  └─────────────────────────────┘   │
│                                    │
│  ┌─ 内联样式 ─────────────────┐   │
│  │  margin-top: [8 ▤] [🔗][×] │   │
│  │  opacity:    [0.9 ]    [×] │   │
│  │  [+ 添加]                   │   │
│  └─────────────────────────────┘   │
│                                    │
└────────────────────────────────────┘
```

- **Class 搜索**：输入即时过滤 design.omni 全部 class，显示名称 + 使用次数
- **Class pill**：`[AppDesign.Card ×]` 点 × 解绑，不删除 class 定义
- **Variant 选择器**：下拉显示维度合法值
- **内联样式 🔗**：和 class 编辑器一致的 token 绑定方式
- **Power user 路径**：从左侧 Classes 面板**拖拽 class 到 Canvas 元素**上 → 高亮 + 释放即绑定

### 3.7 "设计工具感" 微交互

| 交互 | 描述 |
|------|------|
| 数字 drag-scrub | 所有数值输入：click+drag 左右调值，Shift=10x，Alt=精度 |
| 即时预览 | 改 token/class 值 → 画布所有引用元素瞬间刷新 |
| 视觉反馈 | 添加 class → 元素蓝色脉冲；删除 → 红色闪烁 |
| 智能默认 | 新建 `box-shadow` 预填 `0 2px 8px rgba(0,0,0,0.1)` |
| 右键菜单 | 任意对象上右键：复制 / 查找使用 / 重命名 / 生成色板 / 删除 |
| 快捷键 | Cmd+D 复制 token/class，Delete 删除，Cmd+/ 搜索 |
| 自动保存 | 失焦提交，无 Apply 按钮 |

### 3.8 明确避免的反模式

| ❌ | ✅ |
|----|----|
| Modal 弹窗编辑颜色 | HUD 拾色器，定位在点击处 |
| 裸 number input | 所有数字支持 drag-scrub |
| 显示 `var(--color-primary)` 裸文本 | 显示色板 chip + token 名 |
| 伪类重复列所有属性 | 只显示与 Base 不同的属性 |
| Apply / Save 按钮 | 失焦即提交 |
| 平坦属性列表 | 分组折叠：Layout / 视觉 / 文字 |
| CSS 属性名（border-radius） | 设计师术语（"圆角"） |
| 单独删除 token 无警告 | 显示"N 个元素使用"，需确认 |
| 裸 JSON/segments 编辑器 | 隐藏内部实现 |

## 4. 数据模型

### 4.1 TypeScript 类型（扩展现有）

在 `poiesis/src/core/types/style.ts` 基础上扩展：

```typescript
// ── Design Token ──
export type TokenType = "color" | "size" | "font" | "shadow";

export interface DesignToken {
  id: string;
  name: string;                       // "primary", "spacingMd"
  category: string;                   // "colors", "spacing", "typography"
  tokenType: TokenType;
  value: string;                      // "#6366f1", "16px"
}

// ── Design (完整的 design.omni 内容) ──
export interface DesignDefinition {
  name: string;                       // "AppDesign"
  tokens: DesignToken[];
  classes: StyleClass[];             // 复用现有类型
  variants: VariantDefinition[];
}

export interface VariantDefinition {
  id: string;
  name: string;                       // "Button"
  dimensions: VariantDimension[];
  defaults: Record<string, string>;   // { color: "primary", size: "md" }
  baseProperties: StyleProperty[];    // 共享的 base 样式
  branches: VariantBranch[];          // color.primary, size.sm 等
}

export interface VariantDimension {
  name: string;                       // "color", "size"
  values: string[];                   // ["primary", "danger", "ghost"]
}

export interface VariantBranch {
  id: string;
  dimension: string;                  // "color"
  value: string;                      // "primary"
  properties: StyleProperty[];        // 该分支的样式（仅与 base 不同的部分）
}
```

### 4.2 Store 扩展

在 `PoiesisState` 中新增：

```typescript
interface PoiesisState {
  // ...现有字段...

  // ── Design system（跨 Tab 共享）──
  activeDesign: DesignDefinition | null;

  // ── Design 相关 actions ──
  loadDesign: (filePath: string) => void;
  addToken: (token: DesignToken) => void;
  updateToken: (id: string, updates: Partial<DesignToken>) => void;
  removeToken: (id: string) => void;
  addClass: (cls: StyleClass) => void;
  updateClass: (id: string, updates: Partial<StyleClass>) => void;
  removeClass: (id: string) => void;
  addVariant: (variant: VariantDefinition) => void;
  updateVariant: (id: string, updates: Partial<VariantDefinition>) => void;
  removeVariant: (id: string) => void;

  // ── 元素绑定 actions ──
  bindClassToElement: (elementId: string, className: string) => void;
  unbindClassFromElement: (elementId: string, className: string) => void;
  bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) => void;
  unbindVariantFromElement: (elementId: string) => void;
}
```

### 4.3 TabType 扩展

```typescript
// design-page.ts
export type TabType = "component" | "store" | "code" | "design";
```

## 5. 代码生成

### 5.1 Poiesis → .omni

Poiesis 在保存 `design.omni` 时，从 `DesignDefinition` 生成合法的 `design AppDesign { }` 块。

Poiesis 在保存组件 `.omni` 时：
- `class:` 属性写入元素 → `class: AppDesign.Card`
- `variant:` 属性写入元素 → `variant: AppDesign.Button(color=primary, size=md)`
- 内联样式不变 → `style: { ... }`

### 5.2 omnic 编译器改动

omnic（Rust 编译器）需要新增：

| 模块 | 改动 |
|------|------|
| Parser | 新增 `design` 块解析：`token`/`class`/`variant`/`@dim`/`@default`/`base` |
| AST | 新增 `DesignDef`、`TokenDef`、`VariantDef` 等 AST 节点 |
| IR | 新增 `DesignIR`，包含 tokens/HashMap、classes/Vec、variants |
| Codegen | `token()` 编译时解析为 CSS 值；variant 展开为 CSS class 组合；class 引用生成 scoped CSS |
| 验证 | token 类型不匹配 → 编译错误；variant 值不在 `@dim` 合法值中 → 编译错误 |

## 6. 实现优先级

| 优先级 | 内容 | 涉及 |
|--------|------|------|
| P0 | omnic parser/IR：`design` 块语法解析 | Rust: parser, AST, IR |
| P0 | Poiesis `design.omni` 标签页框架 + Token 编辑器 | `DesignPage.tsx`, `TokenEditor.tsx`, store |
| P0 | Poiesis Class 编辑器 | `ClassEditor.tsx`, store |
| P1 | omnic codegen：`token()` 和 variant 展开 | Rust: codegen |
| P1 | Poiesis Variant 编辑器（三种视图） | `VariantEditor.tsx`, store |
| P1 | ContextInspector 升级：class 搜索 + variant 选择 + 内联 🔗 | `ContextInspector.tsx`, `StylesSection.tsx` |
| P2 | 左侧面板 class 拖拽到 Canvas | `ComponentPalette.tsx` |
| P2 | HUD 取色器组件 | `ColorPicker.tsx` |
| P2 | 调色板视图 + 矩阵预览 | `TokenEditor.tsx`, `VariantEditor.tsx` |
| P3 | 类名冲突检测 + 依赖追踪 + "查找使用" | store, utilities |
| P3 | 右键菜单 + 快捷键 | 各编辑器组件 |

## 7. 已知风险

| 风险 | 缓解 |
|------|------|
| omnic 编译器改动大 | P0 只做 parser/IR（能解析不报错），P1 做 codegen |
| Variant 展开代码膨胀 | v1 限制 dimension ≤ 3 × value ≤ 5，组合 ≤ 125 |
| 手动编辑 .omni 后 Poiesis 加载不同步 | design.omni parse 时做 recompile-and-compare 验证 |
| Token/Class 重命名影响全局 | 依赖追踪 + "查找使用" + 批量重命名 |
| 学习曲线（设计师 vs 开发者） | 术语用设计师语言，value 用 chip 显示，不暴露代码 |
