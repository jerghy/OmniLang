# Editor 样式面板重设计规格

> 状态：草稿 | 日期：2026-06-24

## 概述

将 Editor 的样式编辑从「12 个扁平 CSS 属性的纯文本输入」升级为「class-based 样式系统 + CSS 变量 + inline 覆盖」三层体系，对齐 OmniLang 的 `style { }` 语法设计。样式数据完全由 `.omni` 源码承载（不依赖编辑器注释），与 [[2026-06-21-editor-omni-format-redesign]] 的保存/解析体系一致。

---

## 一、设计目标

1. 样式脱离 `props` 平面混装，独立为 `StyleEntry[]` 列表
2. 三层体系：CSS 变量 → Class 样式 → Inline 覆盖（后覆盖前，遵循 CSS 声明顺序，不涵盖特异性规则）
3. 属性编辑器按 CSS 属性类型智能匹配控件和单位
4. 面板内联展开（非弹窗），实时预览
5. 全链路支持 CSS 自定义属性（`--primary-color`），包括 parser / codegen / WASM
6. 旧数据向后兼容：无 `style { }` 块的旧 `.omni` 文件可正常解析（props 中样式 key → inline StyleEntry）

---

## 二、数据模型

### 2.1 新增类型

```typescript
interface StyleEntry {
  id: string;                    // 唯一标识，如 "s1", "s2"
  type: "class" | "variable" | "inline";
  name: string;                  // class 名 | 变量名(--xxx) | "" (inline)
  properties: StyleProperty[];   // variable 类型时固定为 1 条
}

interface StyleProperty {
  name: string;                  // CSS 属性名，如 "padding", "font-size", "--primary-color"
  value: string;                 // CSS 值，如 "16px", "#333", "var(--primary-color)"
}
```

### 2.2 ElementData 变更

```typescript
interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";
  tag: string;
  props: Record<string, unknown>;   // 保留：功能属性 + 事件绑定
  styles: StyleEntry[];             // 新增：样式列表，有序，后覆盖前
  children: string[];
  parentId: string | null;
  componentRef?: string;
  // ...
}
```

**迁移规则：** 旧数据 `props` 中包含的样式 key → 迁移到元素的 `type: "inline"` StyleEntry。

样式 key 的识别标准：key 命中 `STYLE_PROP_CSS_MAP`（`style-resolver.ts:23-29`）或者 key 在下列**全局样式属性名集合**中：

```typescript
const STYLE_PROPERTY_NAMES = new Set([
  // 映射表中已有的
  "background_color", "border_radius", "font_size", "font_weight",
  "text_align", "shadow", "padding", "margin", "gap",
  "width", "height", "color",
  // 额外的 CSS 属性（连字符式 + 驼峰式都支持）
  "display", "position", "opacity", "overflow", "z_index", "zIndex",
  "box_shadow", "boxShadow", "border", "border_color", "borderColor",
  "font_family", "fontFamily", "line_height", "lineHeight",
  "letter_spacing", "letterSpacing", "text_decoration", "textDecoration",
  "text_transform", "textTransform", "white_space", "whiteSpace",
  "word_break", "wordBreak", "min_width", "minWidth",
  "max_width", "maxWidth", "min_height", "minHeight",
  "max_height", "maxHeight", "transform", "transition",
  "animation", "cursor", "box_sizing", "boxSizing",
  "flex_direction", "flexDirection", "flex_wrap", "flexWrap",
  "justify_content", "justifyContent", "align_items", "alignItems",
  "align_self", "alignSelf", "grid_template_columns", "gridTemplateColumns",
  "background_image", "backgroundImage",
]);
```

不在上述集合中的 key → 保留在 `props` 中（功能属性）。

### 2.3 覆盖优先级

```
StyleEntry 列表从上到下 → 后覆盖前（CSS 声明顺序行为，不涵盖特异性规则）

  styles[0]: variable(--primary-color)    ← 底层 token
  styles[1]: class(.card-style)           ← 复用样式
  styles[2]: inline                       ← 元素级覆盖（最高优先级）
```

编译产出时，`class:` 属性按 styles 列表顺序空格拼接：

```
styles: [s1(variable), s2(class), s3(inline)]
  → class: _v-primary card-style              ← variable 生成 _v- 前缀类名
  + style: { padding: "8px" }                 ← inline 的覆盖属性
```

### 2.4 变量 → 类名映射

变量名 `--xxx` 生成类名 `_v-xxx`（`_v-` 前缀避免与用户 class 名冲突）。

```
--primary-color  →  _v-primary-color
--spacing-md     →  _v-spacing-md

编译产出：
  style { ._v-primary-color { --primary-color: #4F46E5; } }
  元素: class: _v-primary-color
```

解析回读：类名以 `_v-` 开头 → 提取 `--` + 剩余部分 → 还原为 variable 类型 StyleEntry。若不想用自动映射，用户可在编辑器中自定义类名。

---

## 三、PropertyPanel 右侧面板

### 3.1 面板结构

```
┌─────────────────────────────────────┐
│ 选择: Button / counterBtn           │
├─────────────────────────────────────┤
│ 属性                                │
│   label       [submit_______]       │ ← 功能 props（不变）
│   variant     [primary      ▼]     │
├─────────────────────────────────────┤
│ 事件                        [+添加] │ ← 现有事件列表
│   onPress → handleSubmit       [×] │
├─────────────────────────────────────┤
│ 样式                        [+添加] │ ← 新增，逻辑同事件列表
│ ▶ .card-style                       │   折叠：只显示类名/变量名
│ ▼ .card-style                       │   展开一个（手风琴）
│   ┌─────────────────────────────┐   │
│   │ ○class ○inline ○variable        │   │  类型切换（UI 文本按 locale 本地化）
│   │ 类名: [card-style______]    │   │  class/变量时有此字段
│   │                             │   │
│   │ [padding  ▼] [16] [px ▼][×]│   │  属性编辑器行
│   │ [color   ▼] [■] [#333  ][×]│   │
│   │ [+ 添加属性]                │   │
│   └─────────────────────────────┘   │
│ ▶ --primary-color                   │  变量条目
│ ▶ inline                            │  inline 条目
└─────────────────────────────────────┘
```

### 3.2 交互规则

| # | 规则 |
|---|------|
| U1 | 点击样式条目 → 展开/收起（手风琴，同时只展开一个） |
| U2 | 展开后的编辑直接修改 store → 画布即时刷新（无确定/取消按钮） |
| U3 | 每次改动走现有 Command 体系（undo/redo 复用 `SetProp` 模式） |
| U4 | 点击 `[+添加]` → 新建一个默认 inline 类型的 StyleEntry，自动展开 |
| U5 | 样式条目支持拖拽排序（同事件列表） |
| U6 | 点击 `[×]` → 删除该 StyleEntry |

### 3.3 样式列表 vs 事件列表 对比

| | 事件列表 | 样式列表 |
|---|---|---|
| 数据结构 | `events: EventBinding[]` | `styles: StyleEntry[]` |
| 展开编辑 | 弹窗编辑事件绑定 | 面板内联展开编辑属性 |
| 排序 | 拖拽 | 拖拽 |
| 添加按钮 | `[+添加]` | `[+添加]` |
| 删除 | `[×]` | `[×]` |

### 3.4 Command 体系（undo/redo）

现有 `SetProp` Command 只处理扁平 `(elementId, propKey, value)`，无法支持 `StyleEntry[]` 的嵌套操作。新增专用 Command：

```typescript
// 条目级操作
class AddStyleEntry implements Command {
  // undo: 删除该 entry, redo: 重新添加
  elementId: string; entry: StyleEntry;
}
class RemoveStyleEntry implements Command {
  // undo: 恢复该 entry（含原始索引位置）, redo: 删除
  elementId: string; entry: StyleEntry; index: number;
}
class ReorderStyleEntry implements Command {
  // undo: 恢复旧顺序, redo: 应用新顺序
  elementId: string; oldOrder: string[]; newOrder: string[]; // entry id 列表
}

// 条目内部操作
class SetStyleEntryType implements Command {
  // 切换 class/variable/inline（保留 properties）
  elementId: string; entryId: string; oldType: StyleEntry["type"]; newType: StyleEntry["type"];
}
class UpdateStyleProperty implements Command {
  // 单个属性的增/改/删
  elementId: string; entryId: string;
  propertyIndex: number; // -1 = 新增
  oldProperty: StyleProperty | null;
  newProperty: StyleProperty | null;
}
class RenameStyleEntry implements Command {
  elementId: string; entryId: string; oldName: string; newName: string;
}
```

**实现模式：** 每个 Command 在 execute 时修改 `element.styles` 数组（遵循不可变模式——返回新数组/新对象），undo 时用保存的快照还原。

---

## 四、属性编辑器控件

### 4.1 属性行结构

```
[属性名下拉 ▼]  [值输入区 + 单位下拉 ▼]  [× 删除此行]
```

### 4.2 属性名下拉（分类 + 搜索）

```
┌─────────────────────┐
│ [🔍 搜索属性...     ] │
│─────────────────────│
│ 布局                │
│   width             │
│   height            │
│   min-width         │
│   max-width         │
│   margin            │
│   padding           │
│   display           │
│   gap               │
│─────────────────────│
│ 排版                │
│   font-size         │
│   font-weight       │
│   font-family       │
│   color             │
│   text-align        │
│   line-height       │
│   letter-spacing    │
│─────────────────────│
│ 背景 & 边框          │
│   background-color  │
│   background-image  │
│   border            │
│   border-radius     │
│   border-color      │
│   box-shadow        │
│   opacity           │
│─────────────────────│
│ 定位                │
│   position          │
│   top / right /     │
│     bottom / left   │
│   z-index           │
│   overflow          │
│─────────────────────│
│ 变换 & 过渡          │
│   transform         │
│   transition        │
│   animation         │
│   cursor            │
│─────────────────────│
│ 自定义              │
│   (自由输入)         │  ← 支持 --custom-property
└─────────────────────┘
```

### 4.3 值编辑器 + 智能单位映射

| 属性类型 | 值控件 | 单位选项 |
|---------|--------|---------|
| 颜色 (`color`, `background-color`, `border-color`) | 颜色选择器 + hex 文本 | — |
| 长度 (`width`, `height`, `padding`, `margin`, `gap`, `min-width`, `max-width`) | 数字输入 | **常用**: px, rem, em, %   **高级**: vw, vh, vmin, vmax, ch, auto |
| 字号 (`font-size`) | 数字输入 | px, rem, em, %, pt, ch |
| 行高 (`line-height`) | 数字输入 | **无单位优先**（如 1.5），也支持 px, rem, em, % |
| 透明度 (`opacity`) | 滑块 + 数字输入（0-1，步长 0.01） | — |
| 字重 (`font-weight`) | 下拉选择 + 自由输入 | 100-900, normal, bold, lighter, bolder |
| 文本对齐 (`text-align`) | 下拉选择 | left, center, right, justify, start, end |
| 显示 (`display`) | 下拉选择 | block, flex, grid, inline, inline-block, inline-flex, none |
| 定位 (`position`) | 下拉选择 | static, relative, absolute, fixed, sticky |
| 时间 (`transition-duration`, `animation-duration`) | 数字输入 | ms, s |
| 角度（`transform` 子属性） | 数字输入 | deg, rad, turn, grad |
| 圆角 (`border-radius`) | 数字输入（可选四角独立） | px, rem, em, % |
| 阴影 (`box-shadow`) | **Phase 1: 文本输入（自由）**；Phase 2: 结构化编辑器 | px |
| 字体 (`font-family`) | 文本输入 + 常用下拉 | — |
| 溢出 (`overflow`) | 下拉选择 | visible, hidden, scroll, auto |
| 光标 (`cursor`) | 下拉选择 | pointer, default, text, move, not-allowed, grab... |
| 背景图 (`background-image`) | 文本输入 | url(...) |
| 变换 (`transform`) | **Phase 1: 文本输入（自由）**；Phase 2: 函数选择器 | — |
| **通用文本** | 文本输入（自由） | — |

> **Phase 1/2 说明**：`box-shadow` 和 `transform` 的值解析/序列化复复杂，Phase 1 先提供自由文本输入（值正确透传即可），Phase 2 再做结构化编辑器。`opacity` 和 `line-height` 在 Phase 1 就支持专用控件。

### 4.4 示例行

```
编辑 padding:
  [padding ▼]  [16] [px ▼] [×]

编辑 color:
  [color ▼]  [■] [#333333____] [×]

编辑 font-weight:
  [font-weight ▼]  [bold ▼] [×]

编辑 transform:
  [transform ▼]  [rotate(] [15] [deg ▼] [)] [×]

编辑自定义属性:
  [--primary-color ▼]  [#4F46E5____] [×]
```

---

## 五、编译产出映射

### 5.1 Class 类型

```
StyleEntry:                              .omni 产出:
{                                        component Button {
  id: "s1",                                style {
  type: "class",                             .card-style {
  name: "card-style",                          padding: 16px;
  properties: [                                color: #333;
    { name: "padding", value: "16px" },        font-weight: bold;
    { name: "color", value: "#333" },        }
    { name: "font-weight", value: "bold" },  }
  ]                                          view {
}                                              Button(class: card-style, label: "Submit")
                                            }
                                          }
```

### 5.2 Variable 类型

```
StyleEntry:                              .omni 产出:
{                                        component Button {
  id: "s2",                                style {
  type: "variable",                          ._v-primary-color {
  name: "--primary-color",                     --primary-color: #4F46E5;
  properties: [                              }
    { name: "value", value: "#4F46E5" },    }
  ]                                          view {
}                                              Button(class: _v-primary-color)
                                            }
                                          }
```

> 注：variable 条目内只有一条 property，name 固定为 `"value"`。变量名 `--primary-color` → 类名 `_v-primary-color`（`_v-` 前缀防止与用户 class 名冲突）。

### 5.3 Inline 类型

```
StyleEntry:                              .omni 产出:
{                                        component Button {
  id: "s3",                                view {
  type: "inline",                             Button(
  name: "",                                     class: card-style var-primary,
  properties: [                                  style: { padding: "8px", color: "#333" }
    { name: "padding", value: "8px" },         )
    { name: "color", value: "#333" },        }
  ]                                        }
}
```

### 5.4 元素 class 拼接

```
styles: [s1(class), s2(variable), s3(inline)]
  → class: "card-style _v-primary-color"
  + style: { padding: "8px", color: "#333" }

class 名生成规则：
  - class 类型:  直接使用 name ("card-style")
  - variable 类型: "--primary-color" → "_v-primary-color" (_v- 前缀)
  - inline 类型:  不生成 class 名（属性直接写入 style: { }）
```

---

## 六、保存 & 解析

> 对齐 [[2026-06-21-editor-omni-format-redesign]]，`.omni` 源码 = 唯一真相源。

### 6.1 保存：StyleEntry[] → `.omni` 源码

Style 数据完全是合法 `.omni` 语法，**不需要**编辑器注释（`// @style`）标记。

**保存规则：**

| # | 规则 |
|---|------|
| S1 | class 和 variable 类型 → 写入组件级 `style { }` block |
| S2 | inline 类型 → 写入对应元素的 `style: { }` |
| S3 | 元素引用 class → 通过 `class:` 属性，空格分隔多个类名 |
| S4 | `style { }` block 位置：在 `view { }` 之前 |
| S5 | 样式列表有序，`style { }` 内 class 定义顺序 = 列表中 class/variable entry 的顺序 |

### 6.2 解析：`.omni` 源码 → StyleEntry[]

编辑器加载 `.omni` 时，从 AST 还原 StyleEntry 列表：

```
解析流程:

  parse .omni source → AST
      │
      ├── 遍历 style { .className { properties... } }
      │     ├── 属性名以 -- 开头
      │     │     → StyleEntry { type: "variable", name: "--xxx",
      │     │         properties: [{ name: "value", value: "#xxx" }] }
      │     └── 否则
      │           → StyleEntry { type: "class", name: "className",
      │               properties: [{ name, value }, ...] }
      │
      ├── 遍历 view 元素上的 class: xxx yyy
      │     → 记录元素 → class 关联（PropertyPanel 显示用）
      │
      └── 遍历 view 元素上的 style: { prop: value, ... }
            → StyleEntry { type: "inline", name: "",
                properties: [{ name, value }, ...] }
```

**解析规则：**

| # | 规则 |
|---|------|
| P1 | 解析失败 → 报错，不静默跳过（实时编辑期间：保留最后有效状态，保存时严格执行） |
| P2 | `style { }` 内 class 未被任何元素引用 → 保留（可能被其他组件元素引用） |
| P3 | 元素 `class:` 引用了不存在的 class → 在 PropertyPanel 中显示警告图标 |
| P4 | 同名 class 出现多次 → 合并为一个 entry（后覆盖前） |
| P5 | **向后兼容**：组件定义中无 `style { }` 块 → 从元素 `props` 中提取样式 key（按 2.2 的 `STYLE_PROPERTY_NAMES` 集合识别），为每个元素生成单个 `type: "inline"` StyleEntry |

### 6.3 数据一致性

```
StyleEntry[]  ←── 保存(emit .omni) ──→  .omni 源码
              ──── 解析(AST walk) ──→

修改 style → 更新 AST → emit .omni → 编译 → 画布实时预览
```

---

## 七、Compiler 全链路改动

### 7.1 Lexer/Parser — CSS 自定义属性支持

**文件：** `omni-compiler/src/parser/view.rs`  
**函数：** `parse_style_rules()` (~line 1171)

**问题：** lexer 将 `--primary-color` tokenize 为 `Minus Minus Ident("-") Ident("primary") Minus Ident("color")`。`expect_ident()` 遇到 `Minus` 报错。

**改动：** 属性名解析前检测 `Minus Minus` 前缀，若存在则收集后续 `Ident` 和 `Minus` token，拼接为 `"--primary-color"` 字符串。

**伪代码：**

```rust
// 当前: let mut key = self.expect_ident("expected property name")?;

// 改为:
let mut key = if self.peek_kind() == TokenKind::Minus {
    // 检测 -- 前缀
    let saved = self.save_position();
    self.advance(); // 吃第一个 Minus
    if self.peek_kind() == TokenKind::Minus {
        self.advance(); // 吃第二个 Minus
        // 拼接 "--xxx-xxx" 格式
        let mut name = String::from("--");
        // 收集后续 Ident 和 Minus token
        loop {
            match self.peek_kind() {
                TokenKind::Ident(_) => {
                    name.push_str(&self.current_token_text());
                    self.advance();
                }
                TokenKind::Minus => {
                    name.push('-');
                    self.advance();
                }
                _ => break,
            }
        }
        name
    } else {
        self.restore_position(saved);
        self.expect_ident("expected property name")?
    }
} else {
    self.expect_ident("expected property name")?
};
```

### 7.2 OmniView Codegen — 动态内联样式 setProperty

**文件：** `omni-compiler/src/codegen/omniview.rs`  
**位置：** line ~1902, ~1930, ~1965

**问题：** 三个动态样式代码路径使用 `el.style.{prop} = value`。对于 `--primary-color` 会生成 `el.style.--primary-color = value` → 无效 JS。

**改动：** 检测 `prop` 是否 `--` 开头，若是 → `el.style.setProperty("prop", value)`，否则 → 保持 `el.style.prop = value`。

**伪代码：**

```rust
// 当前: el.style.{js_prop} = {value};

// 改为: 在生成 JS 前判断
fn emit_dynamic_style_property(&mut self, prop: &str, value_expr: &str) {
    if prop.starts_with("--") {
        // el.style.setProperty("--prop", value)
        self.js.push_str(&format!(
            "el.style.setProperty(\"{}\", {});", prop, value_expr
        ));
    } else {
        // el.style.prop = value
        self.js.push_str(&format!(
            "el.style.{} = {};", to_js_prop(prop), value_expr
        ));
    }
}
```

影响三处：
- line 1902: `$signal` 动态样式
- line 1930: `$concat` 动态样式
- line 1965: `$call` 动态样式

### 7.3 Editor Style Resolver

**文件：** `editor/client/src/omni-gen/style-resolver.ts`  
**函数：** `resolveStyle()` (~line 93)

**问题：** CSS 自定义属性不在 `STYLE_PROP_CSS_MAP` 中，被误判为 HTML attribute。

**改动：** 在 fallback 到 `htmlAttrs` 之前，检测 key 是否以 `--` 开头，若是 → 写入 `cssRules`。

```typescript
// 当前: 未匹配到映射表 → htmlAttrs[key] = String(val);

// 改为:
if (key.startsWith("--")) {
  cssRules[key] = String(val);  // CSS 自定义属性
} else {
  htmlAttrs[key] = String(val); // HTML 属性
}
```

### 7.4 WASM 重编译

**文件：** `omni-compiler/` → `editor/public/wasm/`

**命令：**

```bash
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm
```

**产出文件：**
- `omni_compiler.js`（JS glue）
- `omni_compiler_bg.wasm`（编译产物）
- `omni_compiler.d.ts`（类型声明）

**验证步骤（重编译后必须执行）：**

```bash
# 1. 重编译 WASM
cd omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm

# 2. 验证：含 --custom-property 的 .omni 能正常编译
echo 'component Test { style { .foo { --my-color: #333; color: var(--my-color); } } view { <div class: foo/> } }' > /tmp/test_var.omni
cargo run -- compile /tmp/test_var.omni
# 期望：输出含 --my-color: #333 的 CSS，无报错

# 3. 验证：动态 inline style 生成 setProperty
# 检查编译后的 JS 中 -- 开头的属性用 setProperty
```

---

## 八、改动文件总览

| # | 文件 | 改动类型 | 说明 |
|---|------|---------|------|
| 1 | `editor/client/src/store/editor.ts` | 数据模型 | ElementData 加 `styles: StyleEntry[]` |
| 2 | `editor/client/src/panels/PropertyPanel.tsx` | UI 重构 | 样式列表 + 内联展开 |
| 3 | `editor/client/src/panels/StylePropertyEditor.tsx` | **新建** | 属性编辑器行组件（属性下拉 + 值控件 + 单位） |
| 4 | `editor/client/src/panels/StyleEntryEditor.tsx` | **新建** | 单条 StyleEntry 编辑器（三种模式） |
| 5 | `editor/client/src/constants/style-properties.ts` | **新建** | `PROPERTY_UNIT_MAP` + `DROPDOWN_PROPERTIES` + `STYLE_PROPERTY_NAMES`（单一真相源） |
| 6 | `editor/client/src/commands/AddStyleEntry.ts` | **新建** | 添加样式条目 Command |
| 7 | `editor/client/src/commands/RemoveStyleEntry.ts` | **新建** | 删除样式条目 Command |
| 8 | `editor/client/src/commands/ReorderStyleEntry.ts` | **新建** | 排序样式条目 Command |
| 9 | `editor/client/src/commands/UpdateStyleProperty.ts` | **新建** | 更新样式属性 Command |
| 10 | `editor/client/src/omni-gen/source-emitter.ts` | 保存 | style block 生成 |
| 11 | `editor/client/src/omni-gen/pipeline.ts` | 保存/解析 | style entry 收集 & AST 读取 + 向后兼容 |
| 12 | `editor/client/src/omni-gen/style-resolver.ts` | 解析 | CSS 自定义属性透传 (`--` → cssRules) |
| 13 | `omni-compiler/src/parser/view.rs` | Parser | `--` 属性名支持 |
| 14 | `omni-compiler/src/codegen/omniview.rs` | Codegen | `setProperty` for `--` props（3处） |
| 15 | `editor/public/wasm/*` | WASM | 重编译 + 验证测试 |

---

## 九、实现阶段

### Phase 1: Compiler 层（依赖基础）

1. Parser: `--` 自定义属性名支持
2. OmniView codegen: `setProperty` 修复
3. WASM 重编译
4. Editor style-resolver: 透传 `--` 属性
5. 验证：`.omni` 文件含 `--custom-property` 正常编译

### Phase 2: 数据模型 & 保存/解析

1. ElementData 加 `styles: StyleEntry[]`
2. 旧数据迁移（props 中的样式 key → inline StyleEntry）
3. source-emitter 生成 `style { }` block
4. pipeline 从 AST 读取 style 还原 StyleEntry[]
5. 验证：editor 中修改样式 → `.omni` 文件 → 重新加载 → 样式还原

### Phase 3: UI 面板

1. StyleEntryEditor 组件（class/variable/inline 三种模式）
2. StylePropertyEditor 组件（智能控件 + 单位）
3. PropertyPanel 样式列表集成（手风琴展开、拖拽排序）
4. 实时预览验证

---

## 十、属性→单位映射参考表

```typescript
// 键名使用连字符式 CSS 属性名，与 StyleProperty.name 一致
const PROPERTY_UNIT_MAP: Record<string, { default: string; common: string[]; advanced?: string[] }> = {
  // 长度类
  width:         { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "vmin", "vmax", "auto"] },
  height:        { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "vmin", "vmax", "auto"] },
  "min-width":   { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh"] },
  "max-width":   { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "none"] },
  "min-height":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh"] },
  "max-height":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "none"] },
  padding:       { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-top":    { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-right":  { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-bottom": { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-left":   { default: "px", common: ["px", "rem", "em", "%"] },
  margin:        { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-top":    { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-right":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-bottom": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-left":   { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  gap:           { default: "px", common: ["px", "rem", "em", "%"] },

  // 字号
  "font-size":      { default: "px", common: ["px", "rem", "em", "%"], advanced: ["pt", "ch"] },
  "line-height":    { default: "", common: ["", "px", "rem", "em", "%"], advanced: ["ch"] },  // "" = 无单位
  "letter-spacing": { default: "px", common: ["px", "rem", "em"] },

  // 圆角
  "border-radius":  { default: "px", common: ["px", "rem", "em", "%"] },

  // 阴影
  "box-shadow":     { default: "px", common: ["px"] },

  // 时间
  "transition-duration": { default: "ms", common: ["ms", "s"] },
  "animation-duration":  { default: "ms", common: ["ms", "s"] },
};
```

---

## 十一、单元属性集（不在映射表中的通用处理）

```typescript
// 键名使用连字符式 CSS 属性名，与 StyleProperty.name 一致
const DROPDOWN_PROPERTIES: Record<string, string[]> = {
  display:      ["block", "flex", "grid", "inline", "inline-block", "inline-flex", "none"],
  position:     ["static", "relative", "absolute", "fixed", "sticky"],
  "font-weight":   ["100", "200", "300", "400", "500", "600", "700", "800", "900", "normal", "bold", "lighter", "bolder"],
  "text-align":    ["left", "center", "right", "justify", "start", "end"],
  overflow:     ["visible", "hidden", "scroll", "auto"],
  "overflow-x":    ["visible", "hidden", "scroll", "auto"],
  "overflow-y":    ["visible", "hidden", "scroll", "auto"],
  cursor:       ["auto", "default", "pointer", "text", "move", "not-allowed", "grab", "grabbing", "crosshair", "zoom-in", "zoom-out"],
  "box-sizing":    ["content-box", "border-box"],
  "text-transform": ["none", "uppercase", "lowercase", "capitalize"],
  "text-decoration": ["none", "underline", "line-through", "overline"],
  "white-space":   ["normal", "nowrap", "pre", "pre-wrap", "pre-line"],
  "word-break":    ["normal", "break-all", "break-word", "keep-all"],
  "flex-direction": ["row", "row-reverse", "column", "column-reverse"],
  "flex-wrap":     ["nowrap", "wrap", "wrap-reverse"],
  "justify-content": ["flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly"],
  "align-items":   ["stretch", "flex-start", "flex-end", "center", "baseline"],
  "align-self":    ["auto", "stretch", "flex-start", "flex-end", "center", "baseline"],
  // Grid 子属性
  "grid-template-columns": null,  // 自由文本
  "grid-template-rows": null,
  "grid-column": null,
  "grid-row": null,
  // 过渡/动画子属性
  "transition-property": null,
  "transition-timing-function": ["ease", "ease-in", "ease-out", "ease-in-out", "linear", "step-start", "step-end"],
  "animation-name": null,
  "animation-iteration-count": ["infinite", "1", "2", "3"],
  "animation-direction": ["normal", "reverse", "alternate", "alternate-reverse"],
  "animation-fill-mode": ["none", "forwards", "backwards", "both"],
  "animation-timing-function": ["ease", "ease-in", "ease-out", "ease-in-out", "linear"],
  // Flex 子属性
  "flex-grow": null,     // 数字输入，无单位
  "flex-shrink": null,   // 数字输入，无单位
  "flex-basis": null,    // 文本输入
};
```

> `null` 值 = 自由文本输入（不在下拉中显示，但属性名可选）。
