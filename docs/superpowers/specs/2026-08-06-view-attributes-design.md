# View 属性三态保真：class="x" / 裸布尔属性 / prop={expr} 动态属性 — 设计

> **日期**：2026-08-06
> **背景研究**：`docs/2026-08-06-view-syntax-usage-guide.md`（六类语法真实用法详解，第 4/5/6 节）
> **范围**：本次只完成三件事——① `class="x"` 属性形式不再丢失 ② 裸布尔属性（required/disabled 等）可解析可编辑 ③ `prop={expr}` 动态属性往返保真。

## 问题陈述

### 现状缺口（learn-omniview 真实使用量）

| 语法 | 使用量 | 现状 | 后果 |
|------|--------|------|------|
| `class="x"` | simple.omni 1 处 | parser-view STRING 分支对 `attrName === "class"` 静默跳过（parser-view.ts:388-389） | 类名消失 |
| `class: "a b"` 字符串形式 | ViewAttributesDemo 3 处 | parseClassName 只认 IDENTIFIER，STRING token 落 else break（parser-view.ts:595-598） | 工具类名消失 |
| 裸布尔属性 `<input disabled>` | FormPage 等 8 处 | 非 EQUALS/COLON 的 IDENTIFIER 直接 break **整个属性循环**（parser-view.ts:449-451） | 属性丢失且**后续属性全部不解析** |
| `prop={expr}` 动态属性 | 遍布 30+ 文件 | BRACE 分支只吃单 IDENTIFIER 且**丢弃花括号**（parser-view.ts:371-382） | `value={count}` → 保存变 `value="count"`（动态→静态退化）；`value={fn(x)}` 直接解析失败 |

### 根因

`ViewElementData.props: Record<string, unknown>` 的值无法区分「静态文本」与「动态表达式」——解析时花括号**必须**丢掉（否则发射端会把 `{count}` 当字面文本输出）。数据模型类型本身已容纳 string/boolean（`unknown`），缺的是**值形态约定**与**解析/发射两端对约定的识别**。

### 关键代码事实（设计依据）

1. `applyStyles` 是空函数（collect-styles.ts:15）→ 发射端 `style.htmlAttrs` 分支是死代码；**普通元素属性发射的真实路径是 `elNode.props`**（source-emitter.ts:620-631），且该路径已有 `typeof val === "boolean"` → 裸属性输出（625-626）。
2. UI 层已有既定约定：PropsSection 的 label 芯片编辑器把 props 字符串中的 `{expr}` 段渲染为紫色表达式芯片（`parseLabelSegments`/`serializeSegments`，PropsSection.tsx:20-43）；`/` 信号插入已对所有属性输入框生效（PropsSection.tsx:188-194）。**编辑能力已就绪，缺的是解析/发射不认该约定**。
3. `setProp(id, prop, value: unknown)` 已支持任意值类型（element-crud.ts:77-91）。

---

## 设计决策

### D1：props 值三态约定（零 schema 改动）

`props: Record<string, unknown>` 的值按形态解释，与 label 芯片编辑器现有约定统一：

| 值形态 | 含义 | 发射输出 |
|--------|------|---------|
| `"Enter name"`（string，非 `{}` 包裹） | 静态文本 | `placeholder="Enter name"` |
| `"{count}"`（string，`{` 开头 `}` 结尾） | 动态表达式（任意合法表达式） | `value={count}` |
| `true`（boolean） | 裸布尔属性开启 | `disabled` |
| `false`（boolean） | 布尔关闭 | 不发射（语义 = 无属性） |

判定规则：字符串 `trim()` 后匹配 `/^\{(.+)\}$/s` 且 `$1` 非空 → 动态表达式形态。`{expr}` 包裹无转义机制（OmniLang view 中 `{...}` 恒为插值），约定安全。

### D2：BRACE 属性值解析任意表达式（parser-view.ts:371-382 重写）

- 弃用「expect 单 IDENTIFIER」：改为**配平花括号收集原始表达式**（复用事件 handler 的 315-329 模式——depth 计数、EOF 保护），`expr.trim()` 后存 `"{" + expr + "}"`。
- 效果：`value={count}` 存 `"{count}"`；`value={inputText.to_string()}`、`value={if x { "a" } else { "b" }}` 等任意表达式保真。
- `attrName === "id"` 特例：**保持现状**（存裸值，丢花括号）——动态 id 在 learn-omniview 零使用，且 `el.id` 承担编辑器节点身份，重构属于范围外（见 D7）。

### D3：裸布尔属性 → `props[name] = true`（修复 break bug）

- IDENTIFIER 分支中，next 既非 EQUALS 也非 COLON 时：若 `attrName ∈ BOOLEAN_ATTRS` → `attrs[attrName] = true` + **`continue`**；否则维持 `break` 容错（未知裸属性仍是解析错误）。
- 连带修复：`<input disabled placeholder="x">` 中 disabled 不再截断后续属性解析（现 bug：break 终止整个属性循环，后续属性全部丢失）。
- `{expr}` 形式布尔：`checked={agreed}` 走 D2 → 存 `"{agreed}"`（表达式形态，优先级高于裸布尔）。

### D4：BOOLEAN_ATTRS 共享常量

- 现状：source-emitter.ts:476-481 私有静态集合（实际 18 个：disabled/required/readonly/checked/selected/hidden/multiple/autofocus/novalidate/async/defer/ismap/itemscope/open/loop/muted/controls/autoplay——实现期交叉验证修正，见计划 Task 1 report）。
- 动作：提升为共享导出常量（如 `codegen/boolean-attrs.ts` 或从 source-emitter 导出），parser-view 与 source-emitter 共用。
- 按语法文档补全差异：`autocomplete` 等文档 15 个布尔属性中的缺失项（文档 15 ⊂ 集合核对后补齐；以 docs/omniview/06-templates.md「布尔属性」清单为准）→ 终态 **19 项**（18 + autocomplete）。

### D5：class 属性统一走 classRefs 管线

- `class="x"`（STRING 分支）：不再跳过 → 值拆空白 → 与 `class:` 形式同一条 className → classRefs 管线（parser-finalize/转换处现有路径）。
- `class: "a b"` 字符串形式（parseClassName）：补 STRING 分支 → 拆空白进 classes。
- 输出形态：发射端经 classParts 输出 `class: a b`（文档确认 `class="x"` 与 `class: x` 语义等价，统一为 class: 形式是等价改写，非信息损失）。
- UI 侧不变：classRefs 由 ClassesSection 管理，props.class 不进入 PropsSection 面板（现状保持）。

### D6：发射端动态属性输出（source-emitter.ts:620-631 props 路径）

```ts
for (const [key, val] of Object.entries(elNode.props)) {
  if (key === "className" || key === "style" || key === "key" || key === "label") continue;
  if (key.startsWith("on")) continue;
  if (typeof val === "boolean") {
    if (val) parts.push(` ${key}`);                       // 已有逻辑
  } else if (typeof val === "string") {
    const exprMatch = /^\{(.+)\}$/s.exec(val.trim());
    if (exprMatch && exprMatch[1].trim().length > 0) {
      parts.push(` ${key}={${exprMatch[1]}}`);            // 新增：动态表达式
    } else {
      parts.push(` ${key}="${escapeAttr(val)}"`);         // 静态
    }
  } else if (typeof val === "number") {
    parts.push(` ${key}="${val}"`);
  }
}
```

- `style.htmlAttrs` 死分支不改（applyStyles no-op，多根 wrapper 无属性）。
- `id` 属性：elNode.id 路径（576 行）不变。

### D7：边界 —— 不做动态 id

`id={x}` 动态 id 属性：learn-omniview 零使用；`el.id` 是编辑器节点身份（PID 注释锚点），重构语义复杂 → **本次保持现状**（丢花括号存裸值），spec 记录边界，不做。

### D8：UI —— 芯片编辑器推广（label → 全部属性）

- label 专用 LabelEditor 参数化为通用 **ValueChipEditor**：静态文本段 + 紫色 `{expr}` 表达式芯片，点击段/芯片编辑、`/` 信号插入（现有 188-194 通用分支保持）。
- 所有 relevantProps 属性行改用 chip 编辑器（label 行为不变）；**id 行保持普通输入框**（id 是元素标识符，不做芯片分段）。
- 芯片交互与现有 label 芯片一致（文本段内嵌输入、`{expr}` 芯片整体编辑、删除）。

### D9：UI —— 布尔属性行（开关 ↔ 表达式互斥视图）

**前置（本次评审发现的实质缺口）**：`checked`/`disabled`/`required`/`readonly` 等布尔属性**不在 getRelevantProps 白名单**（PropsSection.tsx:573-588——input 只显示 type/placeholder/value/name，button 只显示 label/type/name）→ 解析进 props 后属性面板不可见、不可编辑。必须将 BOOLEAN_ATTRS 相关属性按 tag 补入白名单（input: checked/disabled/required/readonly；button: disabled；a/textarea/select/label: disabled 等），否则 D9 控件不可达。

- 命中 BOOLEAN_ATTRS 的属性显示专用行，**两视图互斥出现**（避免「开关与表达式并存」的语义歧义）：
  - **无表达式**：显示**开关**（勾选 = `true` 存 boolean；`false` 时面板显示关态）
  - **点「表达式」按钮**（文字标签 + tooltip）→ 输入框出现，输 `{expr}`（`/` 插入信号可用）→ 存 `"{expr}"` 字符串
  - **有表达式**：开关隐藏，显示紫色芯片（点击编辑 / × 删除回开关态，与 D8 交互一致）
- 三态互转：开关 ↔ boolean；表达式 ↔ `"{expr}"` 字符串；从表达式清除 → 回开关态。

### D10：范围外（明确不做）

- props 属性白名单扩展（自定义属性键 UI 可达性）——本次只保真已知属性
- 表达式合法性校验——编译器为最终闸门（与调试面板 parseDebugValue 哲学一致）
- `id={x}` 动态 id（D7）
- label 之外属性的 `{expr}` 类型感知（值类型推断）

---

## 文件影响面

| 文件 | 改动 |
|------|------|
| `poiesis/src/services/codegen/parser-view.ts` | D2 BRACE 重写 / D3 裸布尔 / D5 class STRING + parseClassName STRING 分支 |
| `poiesis/src/services/codegen/source-emitter.ts` | D4 BOOLEAN_ATTRS 导出 / D6 动态属性发射 |
| `poiesis/src/services/codegen/boolean-attrs.ts`（新建） | D4 共享常量（或从 source-emitter 导出，二选一，以 parser 无循环依赖为准） |
| `poiesis/src/services/codegen/parser-finalize.ts` 或现有转换处 | D5 class="x" → classRefs 管线确认（className 已有路径） |
| `poiesis/src/components/panels/inspector/PropsSection.tsx` | D8 芯片推广 / D9 布尔行 + 布尔属性进 getRelevantProps 白名单 |
| 测试：`parser-view` 相关 `__tests__`、`source-emitter` 相关 `__tests__`、`PropsSection.test.tsx`（如存在） | 往返矩阵 + UI 用例 |

## 测试策略（测试即契约）

### 解析往返矩阵（parser-view + emitter 联合）

| 输入源码 | 解析中间态 | 再发射 | 断言 |
|---------|-----------|--------|------|
| `<input value={count}>` | `props.value = "{count}"` | `<input value={count}>` | 花括号保真 |
| `<input value={inputText.to_string()}>` | `"{inputText.to_string()}"` | 同左 | 任意表达式可解析（现为解析失败） |
| `<input disabled>` | `props.disabled = true` | `<input disabled>` | 裸布尔往返 |
| `<input disabled placeholder="x">` | 两者都在 | 两者都发射 | break 修复（disabled 不截断） |
| `<input checked={agreed}>` | `props.checked = "{agreed}"` | `<input checked={agreed}>` | 表达式布尔 |
| `<div class="app">` | `classRefs = ["app"]` | `<div class: app>` | class 属性不再丢（等价改写） |
| `<div class: "a b">` | `classRefs = ["a", "b"]` | `<div class: a b>` | 字符串类名往返 |
| `<input type="text">` | `props.type = "text"` | 同左 | 静态属性回归不破 |
| `placeholder="Enter name"`（含空格值） | 静态字符串 | `placeholder="Enter name"` | 静态保真（escapeAttr） |
| `disabled={false}` | `props.disabled = "{false}"` | `disabled={false}` | 常量表达式形态保真 |

### UI 测试（PropsSection）

- 芯片：`{expr}` 段渲染紫色芯片、编辑/删除、`/` 插入写回 `"{expr}"`；输入 `Hello {count}` 提交后自动拆段（自动芯片化的既有机制回归）
- 布尔行：开关 ↔ boolean；表达式切换 ↔ `"{expr}"`；三态互转；**白名单可达性**（`<input checked>` 解析后属性面板显示 checked 行）
- label 行为回归（芯片编辑器推广不破坏 label）

### 真机验证（Victauri + learn-omniview）

1. 打开 `simple.omni`（class="app"）→ 保存 → 源码仍含类名
2. 打开 `FormPage.omni` → 保存 → disabled/readonly 属性保留
3. 打开 `FormControlsDemo.omni`（checked={agreed}）→ 保存 → 动态属性保留
4. 属性面板：value 属性输 `{count}` → 紫色芯片 → 保存后源码为 `value={count}`

## 回归风险

- **现有测试期望更新**：旧 parser-view/emitter 测试中「`value={count}` → `value="count"`」的用例必须改为新语义（花括号保真）——测试即契约，计划中列出具体改动的既有用例。
- **label 芯片编辑器推广**：PropsSection 渲染路径改动面较大，label 行为需回归测试覆盖。
- **class 输出形态改写**：`class="x"` → `class: x` 属等价改写，但保存后 diff 可见（格式重排本就存在——PID 注释体系已接受格式改写）。

## 验收标准

1. learn-omniview 全项目（62 个 view 文件）经画布打开→保存后：无 `value="sig"` 退化、无布尔属性丢失、无 class 属性丢失（diff 只含格式重排与 PID 注释）
2. 解析/发射往返测试全绿（新增矩阵 + 既有回归）
3. 属性面板：任意属性可编辑动态表达式（芯片 + `/` 插入）、布尔属性开关/表达式双形态
