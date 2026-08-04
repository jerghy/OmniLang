# CodeFlowCanvas 表达式公式框设计

> 日期：2026-08-04
> 状态：待实现
> 范围：CodeFlowCanvas 表达式字段的编辑体验升级——assign value / let value 改为「文本 + 触发式自动补全」公式框，assign target 改为下拉；附带修复 F1（组件 fn 画布 reload 物化）与 F2（fn 候选恒空）

## 一、背景与目标

### 1.1 背景

CodeFlowCanvas 是**代码的节点编辑器**（节点=语句，连线=执行顺序，不做数据端口/值连线——用户定性）。当前 assign/let 节点的 value 字段是纯文本输入框，只能输入固定值，无法以结构化方式表达「变量 + 值」或其它运算；assign target 也是文本输入。

用户需求（2026-08-04）：表达式字段升级为**公式框**——设计规则：**纯变量字段 → 下拉选择框；变量+文本/数字混合字段 → 公式框**。

### 1.2 目标

1. assign value / let value 改为公式框：自由文本输入 + 上下文敏感的候选自动补全（变量/函数）+ 运算符字符触发插入
2. assign target 改为下拉（变量选择）
3. 附带修复 F1（组件 fn 代码流 reload 后画布为空）与 F2（call 节点 fn 候选恒空）

### 1.3 非目标（明确不做）

- ❌ condition / return / call / store-call args 字段的公式框（本次范围外，控件复用后可后续扩展）
- ❌ 节点位置持久化、保存 WASM 门禁静默失效、redo UI、多组件画布过滤（既有遗留项，记录不修）
- ❌ chip 结构化公式框（n8n 数据药丸形态——复杂度高一个量级，与「语句是文本表达式」哲学有张力）
- ❌ 表达式完整解析 / 语法高亮 / Monaco 表达式编辑器
- ❌ 数据端口 / 值连线（既有定性，延续）

## 二、开源研究结论（复刻对象）

| 模式 | 来源 | 复刻要点 |
|------|------|---------|
| trigger 上下文自动补全 | [OverType autocomplete 模式](https://raw.gitcode.com/gh_mirrors/ov/overtype/raw/main/docs/AUTOCOMPLETE.md)、[表达式构建器专利 US9348807](https://patentimages.storage.googleapis.com/47/55/0a/28d5c9cf86fda6/US9348807.pdf) | 输入事件读光标前 token → 按上下文枚举候选（变量/函数/运算符）→ 光标处弹 listbox → ↑↓/Enter/Tab 插入 → Esc 关闭 |
| n8n 表达式编辑器 | [n8n PR #4631](https://github.com/n8n-io/n8n/pull/4631)、[PR #8869](https://github.com/n8n-io/n8n/pull/8869) | 变量插入 + 自动补全 + 实时校验反馈的完整编辑体验（本设计取其自动补全与插入交互，不做数据药丸） |
| ManageIQ chip 表达式构建器 | [Knaur UX 论文](https://is.muni.cz/th/d3yyl/DPRobinKnaurDigital_Archive.pdf) | 无效输入要**解释原因**而非只禁按钮；键盘导航 ←→/Enter/↑↓ 纪律 |
| 本项目既有模式 | NodeSearchMenu（ComfyUI 复刻） | listbox/option aria 语义、↑↓/Enter/Tab/Esc、命中高亮——候选弹层直接复用此交互模式 |

**本项目的差异化约束**：表达式始终以**纯文本**存 `props.value`（草稿提交语义、`formatSyntaxNode` 单一来源、`@pid` round-trip 全部零改动）——公式框只是编辑体验层，不改变数据模型。

## 三、整体架构

```
src/components/canvas/code/
├── fields/
│   ├── NodeField.tsx            → 不动（NodeTextField 草稿提交语义复用）
│   ├── NodeFormulaField.tsx     → 新增：表达式公式框（NodeTextField 升级 + 候选弹层）
│   ├── NodeCandidateMenu.tsx    → 新增：候选弹层（复用 NodeSearchMenu 交互模式，轻量化无搜索框）
│   └── expression-trigger.ts    → 新增：纯函数触发规则（输入+光标 → 候选类型/列表），表驱动可测
├── nodes/SyntaxNode.tsx         → assign value / let value 换 NodeFormulaField；assign target 换 NodeSelect
└── FlowCanvas.tsx               → 节点 data 注入当前 fn 的 letVars（wire 拓扑推断）
```

**服务层**：

```
src/services/codegen/
├── flow-scope.ts                → 候选数据源（signals/props/computeds/fnParams/store signals/actions 已有）
│                                   fns 来源修复（F2：componentDef.fns 不再恒空）
├── fn-topology.ts（新）         → wire 拓扑纯函数：给定 elements+wires，推断每个 fn 的 letVars 归属
└── parser.ts                    → fnRegistry 登记（F2：component → fn 元数据，Additive 字段）
```

**入口层**：

```
src/App.tsx                      → 加载时：codeFlowGraphs → store.codeFlowGraphs（F1）；fnRegistry → componentDefs.fns（F2）
src/pages/DesignPage.tsx         → 组件 tab 首次进入 codeflow：物化该组件 fn 图合并进 store.elements（F1）
```

## 四、NodeFormulaField 交互设计

### 4.1 基础

- 复用 NodeTextField 全部草稿提交语义：Enter/blur 提交、Esc 取消（含弹层关闭不提交）、dirty 基线、聚焦守卫
- 底层 `<input type="text">`，`class="nodrag"`；候选弹层绝对定位输入框下方
- 中文输入法（IME）期间不触发候选（`isComposing` 守卫）；输入法确认后才检测

### 4.2 触发规则（纯函数，表驱动）

输入事件后读 `value + selectionStart`，提取光标前 token 判定：

| # | 光标前 token | 上下文位置 | 行为 |
|---|-------------|-----------|------|
| 1 | 单字符运算符 `+ - * / %` | 操作数后 | **直接插入**，不弹框（单字符无歧义） |
| 2 | 多字符前缀 `< = ! & \| ?` | 操作数后 | 弹**运算符选择框**，前缀过滤：`<`→`[<, <=]`、`=`→`[==]`、`!`→`[!=, !]`、`&`→`[&&]`、`\|`→`[\|\|]`、`?`→`[??, ?.]` |
| 3 | 字母/下划线开头标识符串 | 操作数位置 | 弹**变量/函数候选**（前缀匹配，分类分组） |
| 4 | `StoreName.`（成员点后） | store 名后 | 弹**store 信号候选**（`ThemeStore.` → mode/items…） |
| 5 | 其他 | — | 关闭弹层 |

运算符全集以编译器权威为准（`crates/omnilang-core/src/parser/expr/infix.rs`）：
`+ - * / %`（算术）、`== != < > <= >=`（比较）、`&& ||`（逻辑）、`??`（null 合并）、`is`（类型判断）、`?.`（可选链）、一元 `!`。
注意：`=` 单等号在表达式中不合法（`parse_assign` 是语句级）——`=` 触发时只提供 `==`，**不**提供 `=`。

### 4.3 候选内容与分组

变量/函数候选（操作数位置，完整候选集）：

| 分组 | 来源 | 数据现状 |
|------|------|---------|
| 变量 | `scope.letVars`（当前 fn，wire 拓扑推断，见 §6.2） | 需修复归属 |
| 函数参数 | `scope.fnParams` | ✅ 已有 |
| 信号 | `scope.signals` | ✅ 已有 |
| 属性 | `scope.props` | ✅ 已有 |
| 计算属性 | `scope.computeds` | ✅ 已有 |
| store 信号 | `scope.stores[].signals`（`Store.signal` 格式） | ✅ 已有 |
| 函数 | `scope.fns`（component fn） | 🔧 F2 修复后可用 |
| store action | `scope.stores[].actions`（带签名 `add(text: String, count: i32)`） | ✅ 已有 |

候选条目显示 `name` + 类型/签名；分组标题（变量/信号/store/函数）；前缀匹配沿用 `filterCandidates`（限 8 项）。

### 4.4 插入语义

- **变量候选**：替换当前 token（从 token 起始到光标）
- **运算符**：插入光标处
- 插入后光标置于插入内容之后，继续文本编辑
- 候选插入是草稿的一部分：**不立即提交**，Enter/blur 才 `setProp`（props 仍是唯一事实来源，undo 栈干净）
- 弹层键盘：↑↓ 导航（自动滚动可见）、Enter/Tab 插入、Esc 关闭（草稿保留）、输入不匹配字符即关闭
- 无匹配时显示「无匹配项」提示行（不阻塞输入）

### 4.5 校验

延续「只提示不阻塞」哲学：
- 沿用 `validateSyntaxProps` 警告条（let 变量名/assign target 校验已有）
- 新增轻量**括号配对**检查（`(`/`)` 计数，仅提示，不阻塞）——保存时 WASM 门禁仍是最终裁决

## 五、assign target 改下拉

- 用 NodeSelect（已有控件，`allowCustom` 兜底手输）替换 NodeTextField
- 候选 = signals + props + computeds + fn 参数 + let 变量（当前 fn）+ store 信号（`Store.信号` 拼接）
- `TARGET` 校验保留（标识符或 `Store.信号`，flow-scope.ts:78）

## 六、F1/F2 修复

### 6.1 F2：fn 元数据登记与物化

**根因**：`componentDef.fns` 恒空（App.tsx 初始化为 `[]`），`ParseResult` 无 fns 字段，parser 的 `parseFn` 不登记 fn 元数据。

**修复**：
1. `parser.ts`：`parseFn` 调用处（组件上下文已知）登记 `ParseResult.fnRegistry: Record<componentName, Array<{ name, params, fnPid }>>`——**Additive 新字段，零行为变化**
2. `App.tsx`：加载时物化 `componentDefs[comp].fns = fnRegistry[comp]`

**受益面**：call 节点 fn 候选（SyntaxNode 已从 `scope.fns` 读）+ 公式框函数候选 + FlowInspector 同时修复。

### 6.2 F1：组件 fn 画布 reload 物化

**根因**：`App.tsx` `loadOmniFiles` 只物化 view 元素（designElements/designWires）进 store；`parseResult.codeFlowGraphs`（parser.ts:1484 产物）从未物化。组件 tab 首次打开时 `tabData[tabId]` 无数据 → `elements=[]` → 画布空。唯一既有物化路径是 StorePage 选 action（StorePage.tsx:148，替换语义）。

**修复**：
1. `App.tsx` 加载末尾：`parseResult.codeFlowGraphs` → `store.codeFlowGraphs`（状态字段 index.ts:52 已存在，从未被写入）
2. **组件 tab 首次进入 codeflow**（DesignPage 双击进入 / `toggleCodeFlow`）时：物化当前组件 fn 图**合并**进 `store.elements`
   - 合并语义：保留 view 元素（nodeType ≠ codeblock），codeblock 元素按 pid（id）去重合并，**已存在的 pid 不覆盖**（保留用户在画布上的编辑），**不用** StorePage 的 setElementsAndWires 替换语义
   - 物化时机选「进入 codeflow 时」而非「切 tab 时」：避开 switchTab 从 tabData 恢复 elements 的时序竞争
3. 保存链路零变化（pipeline.ts 仍遍历 `tag==="fn"` 元素编译，`@pid` round-trip 契约不变）

**行为变化**：组件 fn 画布 reload 后恢复图形（不再为空）；store tab 行为不变（action 选择仍替换物化）。

### 6.3 letVars 按 fn 拓扑归属

**现状**：`flow-scope.ts:63-66` 的 `letVars` 是全画布所有 let 节点（多 fn 时串名）。

**修复**：新增纯函数 `fn-topology.ts`——给定 elements+wires，从每个 fn 节点沿 wire 反向/正向遍历收集其 let 节点（**wire 拓扑推断**，用户确认）：
- FlowCanvas 构建节点 data 时，对每个语法节点沿 flow-in wires 反推到所属 fn，注入该 fn 的 let 变量列表
- 推断失败（孤岛节点无 fn 可达）时降级：空列表（不污染其它 fn 候选）

## 七、错误处理

| 场景 | 处理 |
|------|------|
| 候选无匹配 | 弹层显示「无匹配项」，不阻塞输入 |
| IME 组合输入 | `isComposing` 期间不触发候选、不弹层 |
| 弹层定位（text input 无 caretRect） | 镜像 div 测宽或固定输入框下方定位（研究结论 mirror-div 方案） |
| 括号不配对 | 轻量提示（警告条），不阻塞 |
| F1 物化失败 | store.codeFlowGraphs 为空时静默跳过（画布回退为空状态提示） |
| 运算符触发与画布快捷键冲突 | 输入焦点守卫已有（空格唤出搜索菜单不抢 input 焦点）；公式框内空格合法输入 |

## 八、测试策略

| 层 | 内容 |
|----|------|
| 单元（纯函数） | `expression-trigger` 表驱动：5 种触发规则逐一断言（含边界：token 起始、`.` 后、`(` 后、IME 守卫）；运算符前缀过滤（`<`→[<,<=]、`=`→[==] 不提供 `=`）；插入语义（替换 token/光标位置）；`fn-topology` 拓扑遍历（多 fn 隔离、孤岛降级）；F2 `fnRegistry` 登记 |
| 单元（管线） | App 加载物化合并（pid 去重、view 元素保留）；componentDefs.fns 物化 |
| 组件 | NodeFormulaField：聚焦→输入→弹层开合→Tab 插入→Enter 提交→Esc 取消→失焦关闭；SyntaxNode 集成（let value 公式框、assign value 公式框、assign target 下拉）；NodeCandidateMenu 键盘/aria |
| 回归 | 319 vitest 基线全绿（NodeField/NodeSearchMenu/SyntaxNode 既有断言不动）；tsc 零新增错误 |
| E2E（Victauri） | 真机：输入字母弹变量候选→Tab 插入；输入 `<` 弹运算符选择框→选中插入；assign target 下拉选择；保存→reload→画布恢复（F1）；保存重开表达式文本保持（round-trip） |

## 九、风险清单

1. **F1 物化时序**：与 switchTab/tabData 恢复的竞争 → 物化固定在「进入 codeflow」时机，规避恢复窗口
2. **F2 parser 改动**：Additive 字段，只加不删 → 319 基线防回归
3. **运算符与表达式合法性的关系**：弹框只提供合法表达式运算符（无 `=`、无 `+=`）→ 前缀过滤表以 infix.rs 为准
4. **弹层定位精度**：text input 无 caretRect → 镜像 div 测宽；定位不准时降级固定位置
5. **letVars 拓扑推断**：wire 不完整（未连线节点）时降级空列表，不误报
6. **候选弹层与节点拖拽冲突**：所有控件 `nodrag`（NodeField 既有模式延续）

## 十、关键文件

| 动作 | 文件 |
|------|------|
| 新增 | `fields/NodeFormulaField.tsx` + css、`fields/NodeCandidateMenu.tsx` + css、`fields/expression-trigger.ts`、`services/codegen/fn-topology.ts` |
| 修改 | `nodes/SyntaxNode.tsx`（value 换公式框、target 换下拉）、`FlowCanvas.tsx`（letVars 注入）、`flow-scope.ts`（fns 依赖修复后可用）、`parser.ts`（fnRegistry）、`types.ts`（ParseResult 字段）、`App.tsx`（两处物化）、`DesignPage.tsx`（进入 codeflow 物化） |
| 不动 | `fields/NodeField.tsx`、`formatSyntaxNode`、`graph-compiler.ts`、`graph-parser.ts`、`@pid` round-trip 契约 |
