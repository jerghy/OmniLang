# Design System 机制详解

> Token → Class → Variant 三级设计系统。定义在 `design.omni`，由组件文件 import 引用。

## 一、.omni 语法

```omni
import omniview;

design AppDesign {
    // ── Token: 原子设计变量，带类型标注 ──
    token primary: color = "#6366f1";
    token spaceMd: size = "16px";
    token radiusMd: size = "8px";
    token fontSizeMd: size = "14px";
    token fontWeightSemibold: fontWeight = "600";
    token durationFast: duration = "0.15s";

    // ── Class: 可复用样式块（含伪类） ──
    class Card {
        background: "#ffffff";
        border-radius: token(radiusMd);
        padding: token(spaceMd);
        &:hover {
            box-shadow: "0 4px 12px rgba(0,0,0,0.08)";
        }
    }

    // ── Variant: 多维组合变体 ──
    variant Button {
        @dim color: primary | danger | ghost;
        @dim size: sm | md | lg;
        @default { color: primary; size: sm; }
        base {
            display: "inline-flex";
            align-items: "center";
            border-radius: token(radiusMd);
            font-weight: token(fontWeightSemibold);
        }
        color.primary { background: token(primary); color: "white"; }
        color.danger  { background: token(danger); color: "white"; }
        color.ghost   { background: "transparent"; color: token(textPrimary); }
        size.sm {
            padding-top: token(spaceXs);
            padding-bottom: token(spaceXs);
            padding-left: token(spaceSm);
            padding-right: token(spaceSm);
            font-size: token(fontSizeSm);
        }
        size.md { /* padding + font-size */ }
        size.lg { /* padding + font-size */ }
    }
}
```

## 二、TypeScript 类型定义

来自 `poiesis/src/core/types/design.ts`：

```typescript
// ── Token ──
type TokenType = "color" | "size" | "radius" | "fontFamily" | "fontSize"
  | "fontWeight" | "lineHeight" | "letterSpacing" | "shadow"
  | "duration" | "easing" | "opacity" | "zIndex";

interface DesignToken {
  id: string;          // "tk-mrdlgski-ytdnwl-g"
  name: string;        // "primary"
  category: string;    // "colors" | "spacing" | "typography" | ...
  tokenType: TokenType;
  value: string;       // "#6366f1" | "16px"
}

// ── Design Class ──
interface DesignClass {
  id: string;
  name: string;               // "Card"
  properties: StyleProperty[];
  pseudoStates: PseudoState[]; // :hover, :focus, :active
}

// ── Variant ──
interface VariantDefinition {
  id: string;
  name: string;               // "Button"
  dimensions: VariantDimension[];
  defaults: Record<string, string>;  // { color: "primary", size: "sm" }
  baseProperties: StyleProperty[];   // 共享的 base 样式
  branches: VariantBranch[];         // color.primary, size.sm 等
  inheritedProps?: string[];         // 锁定在 base 中的属性名列表
}

interface VariantDimension {
  name: string;     // "color", "size"
  values: string[]; // ["primary", "danger", "ghost"]
}

interface VariantBranch {
  id: string;
  dimension: string;          // "color" | "size"
  value: string;              // "primary" | "sm"
  properties: StyleProperty[]; // ★ 仅该分支与 base 不同的属性
}

// ── 元素绑定 ──
interface ElementVariantBinding {
  variantName: string;                      // "Button"
  dimensionValues: Record<string, string>;  // { color: "danger", size: "lg" }
}
```

## 三、Variant 渲染机制

### 3.1 属性合并逻辑

```
元素 variant: AppDesign.Button(color=danger, size=lg)
  → 合并: base properties
         + color.danger branch properties  (覆盖 base 同名属性)
         + size.lg branch properties       (覆盖前面同名属性)
  → 最终输出 CSS
```

### 3.2 VariantEditor 中的分支继承显示

| 选中分支 | 显示效果 |
|----------|---------|
| `base` | 所有属性正常亮色可编辑 |
| `color.primary` | base 已有属性灰色显示"继承自 base"；branch 覆盖属性正常亮色 |
| `size.lg` | 同上，"已封锁（分支可覆盖）— 点击解锁" 锁定 base 属性到 base |

**蓝色圆点标记**：branch 中覆盖了 base 的属性，左侧显示蓝色圆点。

### 3.3 常见陷阱

> **漏定义分支属性 → 掉到浏览器默认值**

```omni
// ❌ size.lg 只定义了 font-size，没定义 padding
size.lg {
    font-size: token(fontSizeLg);
}
// 渲染 Button(size=lg) → padding 从哪来？base 没有 → 浏览器默认 1px 6px

// ✅ 正确
size.lg {
    padding-top: token(spaceMd);
    padding-bottom: token(spaceMd);
    padding-left: token(spaceLg);
    padding-right: token(spaceLg);
    font-size: token(fontSizeLg);
}
```

### 3.4 @inherit 机制

`@inherit` 将某些属性锁定在 base 中，所有分支只读继承，不可覆盖：

```omni
variant Button {
    @inherit border-radius, cursor;
    base {
        border-radius: token(radiusMd);
        cursor: "pointer";
    }
    // 任何分支都不能覆盖 border-radius 和 cursor
}
```

对应代码：`VariantDefinition.inheritedProps: string[]`

## 四、StyleProperty 与 StyleValueSegment

```typescript
// 一个 CSS 属性
interface StyleProperty {
  name: string;              // 属性名 (camelCase): "paddingTop", "fontSize"
  segments: StyleValueSegment[];  // 值（可能多段）
  id: string;
}

// 值的每一段
type StyleValueSegment =
  | { type: "text"; value: string }       // 纯文本: "16px"
  | { type: "var-ref"; name: string };    // Token 引用: token(spacingMd)
```

### 多段拼接示例

`.omni` 中：
```omni
transition: ""all " " + token(durationFast);
```

解析为：
```typescript
segments: [
  { type: "text", value: '"all "' },     // 注意：含 .omni 引号
  { type: "text", value: '" "' },
  { type: "var-ref", name: "durationFast" }
]
```

`design-emitter.ts` `compileDesignSegments()` 负责将 segments 重新拼回 .omni 表达式。

## 五、Token 系统

### 5.1 TokenType → 编辑器控件

| TokenType | 编辑器控件 |
|-----------|-----------|
| `color` | HUD 取色器（Hue/Saturation/Value + Alpha） |
| `size` | ScrubInput（拖拽调值，Shift=10x）+ 单位下拉（px/rem/em） |
| `fontSize` | ScrubInput + 单位下拉 |
| `fontWeight` | 下拉（100-900，步进 100）|
| `fontFamily` | 搜索式下拉（系统字体栈） |
| `radius` | ScrubInput + 单位下拉 |
| `shadow` | 多值输入（x y blur spread color） |
| `duration` | ScrubInput + 单位下拉（s/ms） |
| `easing` | 下拉（ease/in-out/cubic-bezier 预设） |
| `opacity` | ScrubInput（0-1，步进 0.01） |
| `zIndex` | 整数 ScrubInput |
| `lineHeight` | ScrubInput |
| `letterSpacing` | ScrubInput |

### 5.2 Token Category 分组

```
tokenTypeToCategory():
  color/fontFamily/fontSize/fontWeight/lineHeight/letterSpacing → "typography" (同组)
  size → "spacing"
  radius → "radii"
  shadow → "shadows"
  duration/easing → "animation"
  opacity → "effects"
  zIndex → "layout"
```

### 5.3 Token 引用语法

```omni
// 编译时解析（omnic 编译器处理）
border-radius: token(radiusMd);
padding: token(spaceSm) token(spaceMd);

// 混合：文本 + token
transition: "all " + token(durationFast);
border: "1px solid " + token(border);
```

> `token()` 是**编译时解析**，不是 CSS `var()` 运行时解析。

## 六、Token 引用交互

### 6.1 Slash 触发 Token 面板

在属性输入框中按 `/` 弹出 `TokenSlashPicker`：

- **`TokenizedInput`**（Layout/Typography 属性行）：检测 `/` 结尾 → 调用 `onSlash(inputEl)` → `handleSlashTrigger` → TokenSlashPicker 定位在输入框下方
- **`ScrubInput`**（卡片中）：编辑模式按 `/` → 取消编辑 → 触发 `onSlash` → 卡片通过 `onPropSlash(propName, el)` 传递给 VariantEditor → TokenSlashPicker
- **卡片接口**：`onPropSlash?: (propName: string, el: HTMLElement) => void` — 所有卡片（SizeCard/RadiusCard/SpacingCard/BorderCard/BackgroundCard）均支持

代码路径：`VariantEditor.tsx` `TokenizedInput` → `handleSlashTrigger` (line ~707) → `TokenSlashPicker` (line ~1529)

### 6.2 点击 Token 标签更换

选中 `token(xxx)` 后显示为蓝色等宽文本。点击 token 文本重新弹出 TokenSlashPicker 更换。

- `handleTokenSelect` 三种模式：
  - 从 `/` 触发 → 替换末尾 `/` 为 `token(name)`
  - 点击已有 token → 完整替换为新的 `token(name)`
  - 从 ScrubInput 触发 → 直接设置为 `token(name)`

### 6.3 × 删除按钮

每个 `token(xxx)` 标签旁有 × 按钮。点击后移除该属性（filter props），恢复为普通输入。

实现：卡片中 token 标签 → `<button onClick={e.stopPropagation(); onChange(props.filter(p => p.name !== propName))}>×</button>`

### 6.4 LockIcon 组件

`cards/LockIcon.tsx` — 共享的锁定/解锁切换图标：

```typescript
// 导出
export const LockIcon: React.memo(function LockIcon({...}) // 🔒/🔓 按钮
export function isPropLocked(name, inheritedProps): boolean  // 检测锁定状态
export const LOCKED_INPUT_STYLE: React.CSSProperties        // opacity: 0.45; pointerEvents: none
```

- 仅在 base 模式（`!readOnly && onToggleInherited`）显示
- 🔒 锁定 = 灰色 + 禁止交互（`pointerEvents: none`），分支可覆盖
- 🔓 解锁 = 蓝色高亮，属性继承到所有分支（分支只读）

## 七、组件文件中如何使用 Design System

### 7.1 Class 绑定

```omni
import { AppDesign } from "./design";

component MyPage {
    view {
        <div class: AppDesign.Card>    <!-- ★ class: 前缀 + 命名空间 -->
            Content
        </div>
    }
}
```

元素数据中存储为：
```typescript
element.classRefs: string[]  // ["AppDesign.Card"]
```

### 7.2 Variant 绑定

```omni
<button variant: AppDesign.Button(color=danger, size=lg)>
    Delete
</button>
```

元素数据中存储为：
```typescript
elementVariants[elementId] = {
  variantName: "Button",
  dimensionValues: { color: "danger", size: "lg" }
}
```

### 7.3 样式解析输出

编译器解析 variant 后生成的 CSS class 名称格式：
```
Button--color-danger--size-lg
```

## 八、Store Actions（design-system.ts）

| Action | 功能 |
|--------|------|
| `loadDesign(design)` | 加载 DesignDefinition |
| `addToken(name, category, tokenType, value)` | 新建 token |
| `updateToken(id, updates)` | 更新 token |
| `removeToken(id)` | 删除 token |
| `addDesignClass(name)` | 新建 class |
| `updateDesignClass(id, updates)` | 更新 class（含 properties/pseudoStates） |
| `removeDesignClass(id)` | 删除 class |
| `addVariant(name)` | 新建 variant（默认空 dimensions/base/branches） |
| `updateVariant(id, updates)` | 更新 variant（含 dimensions/branches/baseProperties） |
| `removeVariant(id)` | 删除 variant |
| `toggleInheritedProp(variantId, propName)` | 切换属性的 @inherit 状态 |
| `bindVariantToElement(elementId, variantName, dimensionValues)` | 绑定 variant 到元素 |
| `unbindVariantFromElement(elementId)` | 解绑 |

## 九、往返闭环：解析 → 生成

### 解析（parser.ts: design.omni → DesignDefinition）

1. Tokenizer 将 .omni 文本切分为 token 流
2. Parser 识别 `design` 块 → `token`/`class`/`variant` 子块
3. `@token id=xxx` 注释 → 提取为 `DesignToken.id`
4. 属性值 → 解析为 `StyleValueSegment[]`（识别 `token()` 引用）
5. Variant `@dim` → `VariantDimension`；`base { }` → `baseProperties`；`color.primary { }` → `VariantBranch`

### 生成（design-emitter.ts: DesignDefinition → design.omni）

1. 每个 `DesignToken` → `// @token id=xxx` + `token name: type = "value";`
2. 每个 `DesignClass` → `// @class id=xxx` + `class Name { props; &:pseudo { } }`
3. 每个 `VariantDefinition` → `// @variant id=xxx` + `variant Name { @dim; @default; base; branches; }`
4. `compileDesignSegments()` 将 `StyleValueSegment[]` → .omni 表达式

### PID 稳定性保证

- 解析时从 `// @token id=xxx` 恢复 ID
- 生成时从 `DesignToken.id` 写出 `// @token id=xxx`
- 新增的 token/class/variant 自动分配新 ID
- 删除的 token/class/variant 对应的 ID 行在下次生成时消失
