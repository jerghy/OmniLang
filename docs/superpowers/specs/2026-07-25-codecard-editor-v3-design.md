# CodeCard Editor v3：积木式平板卡片编辑器

> 状态：设计完成 | 2026-07-25
>
> 替代 [2026-07-23-codecard-editor-v2-design.md](./2026-07-23-codecard-editor-v2-design.md) 和 [2026-07-24-codecard-editor-v2-cursor-fix-design.md](./2026-07-24-codecard-editor-v2-cursor-fix-design.md)。
>
> v3 核心变更：简化光标模型（二态）、输入框双态设计、平板卡片视觉、删除语义从容器级改为卡片级。

## 核心原则

**积木式平板卡片**：每个 token 是一张卡片，像积木一样在富文本编辑器中流式排列。卡片两边都是平的，任意两张卡可以相邻。通过键盘打字插入卡片、`\` 创建输入框、Enter 激活/提交输入框。

---

## 1. 光标二态

只有两种光标状态，← → 键在两者之间切换：

| 光标态 | 位置 | 视觉 | 行为 |
|--------|------|------|------|
| **竖线 `│`** | 两张卡之间 / 行首行尾 / 空行 | 标准闪烁竖线（浏览器原生） | 打字触发补全菜单；`\` 创建输入框 |
| **卡片高亮** | 光标落在一张卡上 | 卡片 outline 高亮（primary color）+ box-shadow | Delete → 删此卡；若为输入框卡 → Enter 激活编辑 |

导航规则：
- 竖线 `│` → ←/→ → 相邻卡片高亮
- 卡片高亮 → ←/→ → 竖线 `│`
- ↑ ↓ 在行间移动
- 鼠标点击卡片 → 该卡片高亮
- 鼠标点击卡间 → 竖线 `│`

**不再有方块 `▌` 光标态**——v2 的容器右边界方块光标被移除。

---

## 2. 输入框双态

输入框不再是始终可编辑的 TextNode，而是双态切换：

### 2.1 显示态（SlotCardNode）

```
┌─────────────┐
│  count      │  浅色/半透明背景，与父容器同色系但更淡
└─────────────┘  像其他卡片一样不可编辑（contenteditable="false"）
                  光标可高亮它
```

### 2.2 编辑态

```
┌─────────────┐
│ count▌      │  白色底 + 细边框 + 文字光标闪烁
└─────────────┘  contenteditable="true"
```

### 2.3 切换流程

```
光标高亮输入框卡
  → Enter → 进入编辑态
    框内文字光标闪烁，可输入字符
    编辑态下：
      ← → 在字符间移动，不能跳出输入框
      ↑ ↓ 不能跳出输入框
      Backspace/Delete 删框内字符
  → Enter → 提交，退出编辑态
    内容变为卡片显示态
    光标回到该输入框卡片的"高亮"态
  → Esc → 取消编辑，恢复原值，退出编辑态
    光标回到该输入框卡片的"高亮"态
```

### 2.4 为什么输入框不再用 TextNode？

v2 的 `InputSlotNode extends TextNode` 始终可编辑，导致：
- 文字光标和卡片光标混在一个平面，方向键导航复杂
- 容器内子元素的光标跳转需要 KeyboardPlugin 额外处理

v3 的双态设计：
- 显示态是普通 `ElementNode`（contenteditable="false"），光标导航一致
- 编辑态是隔离的输入框，方向键不会意外跳出
- 光标管理大幅简化

---

## 3. 删除语义

全部以卡片为粒度，像删文字一样删卡片：

| 光标位置 | Delete/Backspace 行为 |
|----------|----------------------|
| 竖线 `│` 在两张卡之间 | Delete → 删右边卡；Backspace → 删左边卡 |
| 卡片高亮 | 删这张卡 |
| 编辑态输入框内 | 删框内字符（正常文本编辑） |

**不再有"删整容器"**——容器只是一个视觉包裹，光标穿过容器最后一个子卡后自然变成竖线 `│`。容器本身不可被单独选中或删除。

### 3.1 块结构（if/while/for）的删除

块结构的 head、body line 各自是独立卡片/行：
- 光标高亮 `if` 头行卡片 → Delete → 删整个 if 块（含所有分支和 body）
- body line 内的子卡按卡片级删除
- 空 body line → 退格删行

---

## 4. 卡片类型体系

### 4.1 关键字卡（实色背景）

| 卡片 | 颜色 | 说明 |
|------|------|------|
| `let` | 紫色系 `#a78bfa` | 变量声明容器头 |
| `set` | 蓝色系 `#60a5fa` | 赋值容器头 |
| `if` | 黄色系 `#fbbf24` | 条件块头 |
| `while` | 橙色系 `#f97316` | while 循环块头 |
| `for` | 绿色系 `#34d399` | for..in 循环块头 |
| `return` | 红色系 `#f87171` | 返回容器头 |
| `await` | 青色系 `#22d3ee` | 异步等待容器头 |
| `call` | 青色系 `#22d3ee` | 函数调用容器头 |

### 4.2 修饰/控制卡（实色背景）

| 卡片 | 颜色 | 说明 |
|------|------|------|
| `mut` | 橙色 `#fde68a` / `#92400e` | 可变修饰 |
| `break` | 灰色 `#d3d2cc` / `#5e5d59` | 跳出循环 |
| `continue` | 灰色 `#d3d2cc` / `#5e5d59` | 跳过本次 |

### 4.3 运算符/分隔卡（灰色背景）

| 卡片 | 说明 |
|------|------|
| `=` `+=` `-=` `*=` `/=` `%=` | 赋值/复合赋值 |
| `>` `<` `>=` `<=` `==` `!=` | 比较运算符 |
| `+` `-` `*` `/` `%` | 算术运算符 |
| `&&` `||` `!` | 逻辑运算符 |
| `:` | 类型标注分隔 |
| `,` | 参数/元素分隔 |
| `.` | dot 访问 |
| `in` | for 循环分隔 |

### 4.4 值卡（实色背景）

| 卡片 | 颜色 | 说明 |
|------|------|------|
| 类型卡 | 青色 `#cffafe` / `#155e75` | `i32` `i64` `f32` `f64` `String` `bool` |
| 字面量/引用卡 | 绿色 `#d1fae5` / `#065f46` | `true` `false` `null` 变量引用 |

### 4.5 输入框卡（浅色/半透明）

与父容器同色系但更淡（opacity 或 lighter 背景色），视觉上区分于实色卡片：

| 上下文 | 输入框类型 | 示例 |
|--------|-----------|------|
| let/set/for 名字位 | `name` | `count` |
| `:` 后面 | `type_value` | `i32` |
| `=` / 运算符后面 | `expression` | `0` |
| if/elif/while 头行 | `condition` | `x > 0` |
| call 内部 | `fn_name` | `print` |
| return/await | `expression` | `sum` |

---

## 5. 视觉模型

### 5.1 卡片原子

所有卡片统一外形：**平板圆角色块**（无拼图接口），流式行内排列：

```
┌───┐ ┌─────┐ ┌──────────┐ ┌───┐ ┌─────┐ ┌───┐ ┌──────────┐
│let│ │ mut │ │ count    │ │ : │ │ i32 │ │ = │ │ 0        │
└───┘ └─────┘ └──────────┘ └───┘ └─────┘ └───┘ └──────────┘
 紫     橙      浅紫(输入框)  灰     青     灰     浅蓝(输入框)
```

卡片间有小间距（2-4px），光标在此处显示为竖线 `│`。

### 5.2 容器包裹

容器仍是 `<div>` 块级包裹，左侧彩色边框 + 浅背景色区分层级：

```
┃ let ┌───┐ ┌─────┐ ┌──────────┐ ┌───┐ ┌─────┐ ┌───┐ ┌──────────┐
┃     │let│ │ mut │ │ count    │ │ : │ │ i32 │ │ = │ │ 0        │
┃     └───┘ └─────┘ └──────────┘ └───┘ └─────┘ └───┘ └──────────┘
```

### 5.3 块结构

三行封闭色条保持不变：头行 `┏`、代码行 `┃`、收口行 `┗`。

```
┏ if ┌──────────┐                    ← 头行
┃    │ x > 0    │ 浅黄(输入框)
┃    └──────────┘
┃   ┃ return ┌───┐ ┌───┐            ← 代码行
┃   ┃        │ret│ │ 1 │
┃   ┃        └───┘ └───┘
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  ← 收口行
```

---

## 6. 输入框实现方案

### 6.1 新增 SlotCardNode（Lexical ElementNode）

```typescript
class SlotCardNode extends ElementNode {
  __slotKind: SlotKind;
  __value: string;
  __isEditing: boolean;  // 编辑态标记

  isInline(): boolean { return true; }
  canBeEmpty(): boolean { return true; }
  canInsertTextBefore(): boolean { return true; }
  canInsertTextAfter(): boolean { return true; }

  createDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = `slot-card slot-card--${this.__slotKind}`;
    el.setAttribute("contenteditable", "false");
    el.setAttribute("data-slot-kind", this.__slotKind);
    el.textContent = this.__value || this.__placeholder;
    return el;
  }
}
```

### 6.2 Enter 激活 → DOM 切换

当 SlotCardNode 被高亮且用户按 Enter：
1. 获取 SlotCardNode 的 DOM 元素
2. 设置 `contentEditable = "true"`，focus
3. 添加 `.slot-card--editing` class（白底+边框+光标）
4. 注册 keydown handler：
   - Enter → 提交值，恢复 `contentEditable = "false"`
   - Esc → 恢复原值，恢复 `contentEditable = "false"`
   - ← → ↑ ↓ → `preventDefault()`（限制在框内）

### 6.3 KeyboardPlugin 变更

Key Enter 处理：
```
if 光标高亮的卡片是 SlotCardNode:
  if 非编辑态 → 激活编辑态
  else → 提交并退出编辑态
else:
  在 body line 内 → 插入新行
  在竖线态 → 无操作（或插入段落）
```

移除：
- 容器右边界检测 (`detectRightEdge`)
- 方块光标相关 CSS (`.at-right-edge`, `block-blink`)
- 容器级 Delete 逻辑

---

## 7. 与 v2 的关键差异

| 方面 | v2 | v3 |
|------|-----|-----|
| InputSlotNode | extends TextNode，始终可编辑 | SlotCardNode (ElementNode)，双态切换 |
| 光标态 | 三态（高亮/竖线/方块） | 二态（高亮/竖线） |
| 容器右边界 | 方块光标 + Delete 删容器 | 竖线光标 + Delete 删右边的卡 |
| 删除粒度 | 卡片级 + 容器级 | 纯卡片级 |
| 卡片视觉 | 圆角色块 | 平板色块（两边平直） |
| 输入框编辑 | 随时可编辑，方向键自由进出 | 编辑态隔离，方向键不跳出 |
| KeyboardPlugin | 右边界检测 + 双层 Delete | 简化：Enter 激活逻辑 + 卡片级删除 |

---

## 8. 数据模型（不变）

`CodeOp[]` ↔ `EditorNode[]` ↔ Lexical Node Tree 三层转换保持不变。

`EditorNode` 类型中：
- `InputSlot` 接口保留但语义改为"输入框的值载体"
- 新增 `slotValue` 字段用于显示态

```typescript
interface InputSlot {
  kind: "slot";
  slotKind: "name" | "type_value" | "expression" | "condition" | "fn_name";
  value: string;       // 当前值
  displayValue: string; // 显示态文本（可能与 value 相同）
  pid: string;
}
```

---

## 9. 文件变更范围

| 文件 | 操作 | 说明 |
|------|------|------|
| `nodes/InputSlotNode.ts` | **重写 → SlotCardNode.ts** | TextNode → ElementNode，双态 |
| `nodes/CardNode.ts` | **微调** | 视觉改为平板（CSS 变更） |
| `nodes/ContainerNode.ts` | **微调** | 移除右边界视觉 |
| `nodes/BlockNode.ts` | 不变 | — |
| `nodes/BodyLineNode.ts` | 不变 | — |
| `plugins/KeyboardPlugin.ts` | **重写** | 移除方块光标逻辑；新增 Enter 双态切换；简化 Delete |
| `plugins/AutocompletePlugin.tsx` | **微调** | \ 创建 SlotCardNode 而非 InputSlotNode |
| `plugins/CodeOpSyncPlugin.ts` | **微调** | 适配 SlotCardNode 的创建/提取 |
| `transforms/slot-operations.ts` | **更新** | 创建 SlotCardNode |
| `CodeCardEditor.module.css` | **重写** | 平板视觉；输入框双态样式；移除方块光标样式 |
| `autocomplete/autocomplete-rules.ts` | 不变 | — |
| `autocomplete/AutocompleteMenu.tsx` | 不变 | — |
| `transforms/convertToNodes.ts` | **微调** | slot → SlotCardNode |
| `transforms/convertToOps.ts` | **微调** | SlotCardNode → slot |
| `core/types/editor-node.ts` | **微调** | InputSlot 加 displayValue |

---

## 10. 不做的东西

- 拼图接口（jigsaw connectors）— 平板卡片已满足需求
- 容器级删除 — 纯卡片粒度
- 方块 `▌` 光标态 — 简化为二态
- 拖拽移动卡片
- 多光标编辑
- ClipboardPlugin / SyntaxLintPlugin

---

## 11. 验证清单

- [ ] 光标二态：竖线 ↔ 卡片高亮，← → 切换
- [ ] 卡片间竖线处打字 → 补全菜单弹出
- [ ] 卡片高亮 + Delete → 删卡
- [ ] 竖线 + Backspace → 删左边卡
- [ ] 竖线 + Delete → 删右边卡
- [ ] 输入框显示态 → Enter → 编辑态
- [ ] 编辑态下 ← → 不跳出输入框
- [ ] 编辑态下 Enter → 提交，恢复显示态
- [ ] 编辑态下 Esc → 取消，恢复原值
- [ ] Storybook 10 个 story 正常渲染
- [ ] `vitest run` 全部通过
- [ ] `tsc --noEmit` 0 错误
