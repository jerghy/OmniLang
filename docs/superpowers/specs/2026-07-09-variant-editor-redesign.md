# Variant 编辑器重构：卡片化属性 + Token 绑定 + 实时预览

> 日期: 2026-07-09
> 状态: 设计完成
> 关联: `2026-07-08-poiesis-class-editor-visual-redesign.md`（复用其卡片系统和预览组件）

## 1. 问题与目标

当前 VariantEditor 的属性编辑是纯文本 key→value 输入，无预览、无 token 绑定、无卡片化控件。设计师无法直观看到编辑效果。

**目标**：保留现有的维度管理系统（已验证好用），升级属性编辑区和新增预览面板。

## 2. 整体布局

```
┌──────┬──────────────────────────────────────┬──────────────┐
│Variant│ Variant: Button                      │ Preview      │
│ 列表  │ 默认组合: color=primary, size=md     │              │
│(不变) │                                      │ color: [pri▾]│
│       │ ── Dimensions ─────────────────────  │ size:  [md▾] │
│       │ color: [primary] danger ghost accent │              │
│       │ size:  sm [md] lg                    │ ┌──────────┐ │
│       │                                      │ │  Button  │ │
│       │ ── 分支标签 ────────────────────────  │ │ preview  │ │
│       │ [base] [color.primary] [color.danger]│ └──────────┘ │
│       │ [color.ghost] [size.sm] [size.md]... │              │
│       │                                      │ [<div>  ▾]  │
│       │ ── 属性卡片（新）──────────────────  │              │
│       │ ┌ Size ────────────────────────────┐ │              │
│       │ │ Width [120px]  Height [40px]     │ │              │
│       │ └──────────────────────────────────┘ │              │
│       │ ┌ Background ──────────────────────┐ │              │
│       │ │ Color [■ #6366f1]  Opacity [1]   │ │              │
│       │ └──────────────────────────────────┘ │              │
│       │ ┌ Border ──────────────────────────┐ │              │
│       │ ┌ Radius ──────────────────────────┐ │              │
│       │ ┌ Spacing ─────────────────────────┐ │              │
│       │ + Add property                       │              │
└──────┴──────────────────────────────────────┴──────────────┘
```

- 左侧：180px，Variant 列表（不变）
- 中间：flex:1，维度管理 + 分支标签 + 属性卡片
- 右侧：260px，实时预览

## 3. 保持不变的部件

### 3.1 左侧 Variant 列表
- 选中/添加/删除 Variant 的逻辑不变
- 样式与 ClassEditor 侧栏一致

### 3.2 维度管理
- `@dim` 维度的定义、添加值、删除值、设默认值——全部保持
- Value pills 的交互不变（点击设默认，× 删除）

### 3.3 分支标签页
- base + dimension.value 标签的生成和切换逻辑不变
- 选中 base → 编辑共享属性；选中分支 → 编辑该分支覆盖

### 3.4 默认组合显示
- 显示 `color=primary, size=md` 形式，不变

## 4. 属性卡片升级

### 4.1 复用 ClassEditor 卡片

将当前纯文本属性行替换为以下卡片（从 ClassEditor 直接复用）：

| 卡片 | 来源 | 包含属性 |
|------|------|----------|
| `SizeCard` | 已有 | width, height |
| `BackgroundCard` | 已有 | background（颜色选择器）, opacity |
| `BorderCard` | 已有 | border（宽度/样式/颜色） |
| `RadiusCard` | 已有 | borderRadius（四角独立或统一） |
| `SpacingCard` | 已有 | padding, margin（BoxModelPreview 可视化） |

每个卡片保持 ClassEditor 中的交互：ScrubInput 拖拽调节数值、IconButtonGroup 枚举选择、颜色选择器等。

### 4.2 可折叠分组

不在卡片中的属性（布局 Layout、文字 Typography）使用 `CollapsibleGroup` 折叠，和 ClassEditor 一致。

### 4.3 "+ Add property" 

保留搜索式添加属性的交互（和 ClassEditor 一致）。

### 4.4 分支编辑的继承显示

和 ClassEditor 的 pseudo 继承逻辑一致：
- 选中分支（如 `color.primary`）时，base 已有的属性灰色显示为"继承自 base"
- 分支覆盖/新增的属性正常亮色可编辑
- 被覆盖的属性行左侧显示蓝色圆点标记

## 5. Token 绑定：`/` 快捷选择

### 5.1 交互流程

1. 用户在任意属性值输入框中输入 `/`
2. 输入框下方弹出 token 选择面板
3. 面板按类别分组：colors / spacing / typography / radii / shadows / durations
4. 每个 token 显示名称 + 预览值（颜色显示色块，间距显示数值等）
5. 点击某个 token → 将其插入为 `token(name)` 引用
6. 用户可继续输入文本，形成混合值如 `"0 4px " token(shadowMd) " rgba(0,0,0,0.08)"`

### 5.2 显示状态

- 未绑定 token：普通文本样式
- 已绑定 token：属性值中 token 引用部分显示为蓝色标签样式（类似 chip），可点击 × 移除
- 属性值输入框支持 token 引用 + 普通文本混合

### 5.3 实现方式

将 `/` token 选择逻辑封装为一个可复用的 hook 或组件，供卡片内部的值输入框使用。Card 组件暴露 `onTokenRequest` 回调，由上层 Editor 统一管理 token 面板的显示。

## 6. 实时预览

### 6.1 组件复用

直接复用 `ClassPreview`，它已经支持：
- 多元素类型切换（div, button, input, h1-h3, p, span, a 等）
- 基于 properties + pseudoStates 渲染内联样式
- hover/focus/active 交互模拟
- token 值解析（`var(--xxx)` 格式）
- 可选 interactive 模式（拖拽 resize）

### 6.2 维度组合选择器

在预览面板顶部增加两个下拉框（根据 variant 的维度动态生成）：

```
┌─ Preview ──────────────────┐
│ 组合: [color: primary ▾]   │
│       [size: md ▾]         │
│                            │
│ ┌──────────────────────┐   │
│ │     Button 预览       │   │
│ └──────────────────────┘   │
│                            │
│ 元素: [<button> ▾]         │
└────────────────────────────┘
```

- 下拉选项从当前 variant 的 dimensions 动态生成
- 默认选择 `@default` 设定的值
- 切换组合时，预览合并 base + 选中分支的属性，实时更新

### 6.3 样式合并逻辑

预览渲染的属性 = base 属性 + 所有选中分支的覆盖属性（后者覆盖前者）。

```typescript
function getCombinedStyle(
  variant: VariantDefinition,
  selections: Record<string, string>
): StyleProperty[] {
  const base = variant.baseProperties;
  const overrides: StyleProperty[] = [];
  for (const [dim, val] of Object.entries(selections)) {
    const branch = variant.branches.find(
      (b) => b.dimension === dim && b.value === val
    );
    if (branch) overrides.push(...branch.properties);
  }
  // Merge: override properties take precedence over base
  const merged = new Map<string, StyleProperty>();
  for (const p of base) merged.set(p.name, p);
  for (const p of overrides) merged.set(p.name, p);
  return Array.from(merged.values());
}
```

## 7. 不做什么

- 不改变 VariantDefinition 数据结构
- 不修改维度管理逻辑
- 不引入 focus mode（ClassEditor 的聚焦编辑模式暂不移植）
- 不在 Variants 中引入 pseudo states（那是 Class 的领域）
- 不改变组合矩阵（方案 A 中的网格预览暂不引入，保持简单）

## 8. 涉及文件

| 文件 | 改动 |
|------|------|
| `VariantEditor.tsx` | 主要重构：替换属性编辑区为卡片系统，新增预览面板 |
| `ClassPreview.tsx` | 可能无需改动（直接复用），或微调以支持外部传入维度选择器 |
| `PropertyRow.tsx` | 不加改动（已在 ClassEditor 中使用） |
| 卡片组件 (`cards/*.tsx`) | 不加改动（直接复用） |
| `CollapsibleGroup.tsx` | 不加改动 |
| `ScrubInput.tsx` | 不加改动 |
| 新增：`TokenSlashPicker.tsx` | 可复用的 `/` token 选择面板组件 |
| 新增：`TokenizedInput.tsx` | 支持 token 引用 + 普通文本混合的输入框组件 |
