# CodeFlowCanvas 成熟化设计（自动整理布局 / 循环节点 / Notion 式公式框）

> 日期：2026-08-05
> 状态：待实现
> 范围：poiesis 前端（poiesis/src/services/codegen + poiesis/src/components/canvas/code），编译器（omnilang crates）零改动

## 一、背景与目标

CodeFlowCanvas（ADR-057~061 时代）已具备 8 种节点、内联编辑、公式框补全、保存门禁、位置持久化、redo 等能力（440/440 tests）。本次「成熟化」三个方向（用户 brainstorming 拍板）：

1. **自动整理布局（本次重点）**——节点位置一键整理，自上而下分层，让布局"更清晰、有逻辑架构感"
2. **循环节点 for-in / while**——补全语句级语法节点体系（语言层 parser/IR/codegen 已支持，画布层是唯一缺口）
3. **Notion 式公式框**——表达式输入从"自由文本 + 触发补全"改为"输入即分词 + 候选菜单 + token 块显示"（对标 Notion 公式框），全部公式框统一替换

三者配合实现「可视化代码逻辑编程」闭环：**布局让结构可读 → 循环补全语句体系 → 公式框让表达式结构化**。

## 二、用户定性边界（约束，全程适用）

- **代码的节点编辑器**：节点=语句、连线=执行序。**不做数据端口/值连线**（语句是文本表达式，无法数据流化）
- **不做通用编辑器交互打磨**：多选/框选、复制粘贴、节点折叠等一概不做
- **只做**：代码编辑体验与功能完整性。每项功能先问"这改善逻辑代码的编辑/表达/保存体验吗？"
- **零新依赖**：不引入 dagre/elkjs/contenteditable 库等任何新 npm 依赖（手写实现）
- **`@pid` round-trip 契约零改动**：`@pid` 注释格式、graph-parser 分支 id 扫描序生成、return 表达式 `el.source` 契约全部不变
- **语言合法性优先**：生成源码必须通过 OmniLang WASM 语义验证（保存门禁最终裁决）

## 三、方向一：自动整理布局（一键按钮）

### 3.1 触发与写入

- **触发**：DesignToolbar 新增「自动整理布局」按钮（redo 旁，aria-label；画布无元素时禁用）
- **覆盖语义**：全部重排（含手动拖过的节点）；一次 undo 条目可撤销
- **写入通道**：纯函数算出新位置表 → **`setElementsAndWires` 批量替换**（undo-slice 已有 prev/new 双方向，一次条目）——不逐节点 moveElement（污染撤销栈）；`setElementPosition`（无 undo）不用于布局写入
- **持久化**：保存侧 persistedPositions 收集 store 元素当前位置，布局结果自然落盘；undo 恢复后保存存回旧位置（自洽）

### 3.2 computeLayeredLayout 纯函数

新文件 `poiesis/src/services/codegen/flow-layout.ts`，签名 `computeLayeredLayout(elements, wires, nodeSizes) → Record<id, {x, y}>`：

1. **子图划分**：wires 无向连通分量；无 wire 元素为孤立组（多 fn 画布每个 fn 子图独立）
2. **每子图分层**：
   - 根 = 子图内 fn 节点（无入 wire 优先 fn；无 fn 取无入 wire 节点）
   - **最长路径分层**：`depth(node) = 到根的最长路径长度`（前驱恒在 `depth-1` 或更上，保证前驱在上方）
   - **主干/分支区分**：condition/loop 的 flow-out 续接链为主干延续（branchColumn 继承）；从分支 handle（br0/br1/…/body）进入的子树为分支，branchColumn = 进入时跨越的分支边累计数
   - 隐式回边设计（见 4.1）保证图无环 → 纯 DAG 分层，无环处理
3. **坐标化**：
   - `y = depth × 行高(depth)`；行高 = 该 depth 所有节点最大估算高 + V_GAP
   - `x = branchColumn × 列宽(column)`；列宽 = 该列最大节点估算宽 + H_GAP
   - 同 (depth, branchColumn) 多节点纵向均布（间距 ≥ V_GAP）
4. **节点尺寸**：优先读 xyflow `node.measured` 缓存；无则按 tag 估算函数（`estimateNodeSize(tag, props)`：condition/loop 按分支/字段行数，fn 按参数数，基础常量 per tag）
5. **子图堆叠**：各子图独立布局后垂直堆叠（间距 SUBGRAPH_GAP=80）；孤立节点底部横排（x 均匀）
6. **布局参数**：命名常量 `LAYOUT_CONSTS`（V_GAP/H_GAP/SUBGRAPH_GAP/估宽估高基准），不可配置化（YAGNI）

### 3.3 测试

- flow-layout 纯函数单测：父 y < 子 y、同层同 y、分支 x > 主干 x、无重叠、多 fn 子图堆叠、孤立节点、循环节点 body 右展
- UI 测试：按钮 → setElementsAndWires 调用（一次 undo 条目）
- E2E：真机点按钮重排 → 保存 → 真实进程重启 → 布局保持

## 四、方向二：循环节点 for-in / while

### 4.1 节点形态（与 condition 同构）

| tag | 节点 | 生成代码 | props | 端口 |
|-----|------|---------|-------|------|
| `while` | 循环判断 | `while <condition> { <body 链> } <flow-out 续接>` | `{ condition }` | flow-in + body（端口 id `br0`）+ 底部 flow-out |
| `for-in` | 迭代循环 | `for <varName> in <iterable> { <body 链> } <续接>` | `{ varName, iterable }` | flow-in + body（`br0`）+ 底部 flow-out |

- **端口 id 复用 `br0`**：loop 是单分支 condition 变体，graph-parser 分支 id 扫描序（`br${branchIndex}`）天然兼容，`@pid` round-trip 零改动；UI handle 显示标签「循环体」
- **隐式回边**：body 链末句自动回流循环头（语言语义），画布无回边 wire → 图保持 DAG（布局引擎无需环处理）
- 语言层已全通（`crates/omnilang-core/parser/stmt.rs` while/for-in 解析、WhileStmtIR/ForInStmtIR、React codegen 发射），画布层是唯一缺口

### 4.2 管线扩展

- `syntax-nodes.ts`：`formatSyntaxNode` + 2 case（单一来源），`defaultSyntaxProps`；`CODE_TAGS`/`TAG_CN`/`MENU_SECTIONS`（控制流分类）同步
- `graph-compiler.ts` emitNode：while/for-in case——条件行（`@pid` 注释行内）+ body 块（body 链发射，块内缩进）+ flow-out 续接（块后对齐）；镜像 condition 的块嵌套实现（ADR-061 对称模式）
- `graph-parser.ts`：
  - `detectSegmentType` 识别 `while ... {` / `for <ident> in ... {` 段
  - 段内语句分类（body 链重建）；**body 首句 wire 到 loop 节点 body 端口（br0）+ 段后语句续接 wire 到 flow-out**——镜像 ADR-061 中链 condition 修复模式（线形重建 + 过渡检测）
  - `parseSyntaxProps` 提取 condition / varName / iterable（多行段 props 解析截断到首个结构行——ADR-061 既有规则）
- 语言合法性：body 内嵌套 fn 声明非法（E0001，ADR-057 同规则）；保存门禁兜底

### 4.3 UI

- LoopNode 新组件（`nodes/LoopNode.tsx`）：条件/iterable 字段接 NotionFormulaField（方向三统一替换），body 端口标签「循环体」，颜色体系随控制流分类
- FlowInspector 表单：while（condition）/ for-in（varName + iterable）
- 拖出创建：body handle 拖出自动连线走既有机制（onConnectStart 已记录 handleId）

### 4.4 break / continue

语言层已支持（React codegen 发射 `break;`/`continue;`）。**不做专用语句节点**（YAGNI）——body 内以 code-block 手写。body 内的 break/continue 解析归 code-block 自由文本（既有分类），不进语法节点范畴。

### 4.5 测试

- formatSyntaxNode 单测（while/for-in 互逆）
- round-trip 幂等：compile→parse→compile（out2 === out1）
- golden 语料 `CodeFlowLoop.omni`：中链 loop（前语句 → loop → 后语句）、loop-in-condition 嵌套、for-in + while 混合
- UI 组件测试；E2E：添加 loop → 连线 body/续接 → 保存 → 真实进程重启形态保持 → `.omni` 产物过 WASM 验证

## 五、方向三：Notion 式公式框（NotionFormulaField）

### 5.1 关键决策：props 契约不变，内部实现替换

`NodeFormulaField` 的 props 契约（`value` / `candidates` / `onCommit` / `ariaLabel` / `placeholder` / `className`）**保持不变**，内部交互实现替换为 Notion 式——**所有使用点零改动统一生效**：

- SyntaxNode：assign/let 的 value
- ConditionNode：if/elif 条件行（else 行保持不可编辑）
- ReturnNode：返回表达式
- NodeArgsRows：args 参数行（含本地 adding 行）
- store-call 的 value / LoopNode 的 condition/iterable（本 spec 新增）

提交语义不变：Enter/blur 提交草稿、Esc 关闭弹层不取消、dirty 基线不编辑不提交。formatSyntaxNode / graph-compiler / graph-parser / 持久化零改动（存的是字符串）。

### 5.2 Tokenizer（纯函数，扩展 expression-trigger）

`expression-trigger.ts` 扩展为完整分词：表达式字符串 → token 序列：

| token | 识别 | 显示 |
|-------|------|------|
| `identifier` | 标识符（变量/信号/函数名） | 紫色胶囊（命中函数候选 → 蓝色胶囊） |
| `operator` | `+ - * / % == != < > <= >= && \|\| ?? is ?. !`（以 infix.rs 为权威，复用 OPERATOR_PREFIX_CANDIDATES） | 符号块 |
| `number` | 数字字面量 | 普通文本块 |
| `string` | 引号包裹整段（可含空格/中文） | 普通文本块 |
| `paren` | `( )` | 符号块（配对高亮） |
| `member` | `Store.signal`（store 名 + `.` + 信号） | 紫色胶囊（同变量） |
| `unknown` | 无法识别字符 | 原文保留 + 亮红提示 |

保留既有 `detectContext` 的上下文语义（operand/member/operator/none）供候选菜单定位：光标所在 token 类型决定候选上下文。

### 5.3 编辑态渲染与交互

- **容器**：contenteditable 单行 + React 状态驱动 re-tokenize（输入 → 字符串 → tokenizer → keyed token span 序列）
- **光标**：浏览器原生 Selection API 管理（Range 保存/恢复），**不造零宽锚点/合成光标**（CodeCard 积木光标 4 次失败教训）
- **IME 守卫**：compositionstart → 暂停分词（原样显示组合输入）；compositionend → re-tokenize（中文输入法组合期不跳动）
- **候选菜单**：键入字母 → 按光标上下文弹候选（数据层分组：变量/信号/store 信号/函数，复用 buildFormulaSource 零新数据源；UI 渲染扁平候选列表——ADR-059 既有契约；复用 NodeCandidateMenu 键盘交互 ↑↓/Enter/Esc）；空候选显示「无匹配候选」灰字（Notion 式菜单空态）
- **运算符**：直接敲字符成符号 token + **同时弹下拉**：多字符变体候选（`=` → `==`、`<` → `<`/`<=`、`&` → `&&` 等，替换 token 不重复插入）；运算符落位后光标处于操作数期待位 → 自动弹变量候选
- **结构化编辑**：Backspace 在 token 边界删整块；←→ 跨 token 导航（token 内部字符级）；括号配对高亮
- **输入法/粘贴**：粘贴 → 整串 re-tokenize（`unknown` 亮红提示，WASM 门禁兜底）

### 5.4 预览 token 化（整体搭配亮点）

新只读组件 `TokenPreview`（`fields/TokenPreview.tsx`）：节点**非编辑态**的表达式显示（formatSyntaxNode 预览的 value/condition/iterable/args 部分）用同一 tokenizer 渲染只读胶囊——**节点显示 = 编辑态同款结构化样式**，表达式结构一眼可见。无 candidates 上下文时纯文本 fallback。

使用点：SyntaxNode value 预览、ConditionNode 条件行、ReturnNode 表达式、LoopNode 条件/iterable、NodeArgsRows 行。

### 5.5 测试

- tokenizer 单测：全部 token 类型、运算符最长匹配（`<=` 优先 `<`）、字符串含空格/中文/运算符字符、member、unknown、嵌套括号
- 组件测试：输入分词渲染、候选选择插入、运算符弹下拉与变体替换、Backspace 整块删除、IME 守卫（composition 事件模拟）、提交语义（Enter/blur/dirty）
- TokenPreview 测试；使用点回归（既有 NodeFormulaField 相关测试更新）
- E2E：真机中文输入法组合输入、运算符弹框、候选选择、保存重开

## 六、整体搭配：可视化代码逻辑编程

三方向合一的体验闭环：

1. **布局**：一键整理让执行结构（主干/分支/嵌套）一眼可读——"读图 = 读代码结构"
2. **循环节点**：循环（界面开发最高频控制流之一）成为结构化节点，与 let/assign/condition 同级，语句体系完备
3. **公式框**：表达式以 token 块呈现（编辑态与节点预览一致），变量/函数/运算符视觉可分——"读表达式 = 读 token 结构"

三者共同点：**把文本代码的隐性结构显性化为画布/胶囊的可视结构**，符合「代码的节点编辑器」定性，不越界到交互打磨。

## 七、测试策略

- 基线 440/440 vitest + tsc 零错误
- 新增测试预估：flow-layout ~10、loop round-trip ~10、tokenizer ~12、NotionFormulaField ~8、TokenPreview ~3、使用点回归更新
- Victauri 真机 E2E：布局按钮重排 + 重启保持；loop 添加/连线/保存/重启/编译产物 WASM 验证；公式框中文 IME/运算符弹框/候选选择/round-trip
- 磁盘产物核验：`.omni` 的 `@pid` id 与画布元素逐字吻合

## 八、风险与缓解

| 风险 | 缓解 |
|------|------|
| contenteditable 光标/IME 问题（中文环境高频） | Selection API 原生管理 + composition 守卫；CodeCard 零宽锚点教训前置参考 |
| 布局在复杂图（深嵌套 condition+loop）下重叠 | 行高/列宽按节点估算尺寸动态计算；单测断言无重叠 |
| loop round-trip 边界（中链 loop、嵌套） | 镜像 ADR-061 已验证的中链 condition 模式；golden 语料 + 幂等测试先行（RED→GREEN） |
| 运算符候选与 infix.rs 漂移 | 候选表继续以 infix.rs 为权威（单一来源原则延续） |
| 既有公式框行为回归（草稿提交/候选菜单） | props 契约不变 + 使用点回归测试 + 组件测试更新 |

## 九、范围外（YAGNI / 定性明确不做）

- 多选/框选、复制粘贴、节点折叠等通用编辑器交互（用户定性）
- 数据端口/值连线（用户定性：语句是文本表达式）
- break/continue 专用语句节点（code-block 手写）
- 布局参数配置 UI（命名常量）
- @mount/@effect 生命周期、@for/@empty 视图模板可视化、跨组件事件中继（Poiesis Features 其它缺口，后续独立轮次）
- 候选分组标签显示（数据层分组，UI 菜单渲染扁平列表——ADR-059 既有契约）

## 十、架构影响文件清单

**新增**：
- `poiesis/src/services/codegen/flow-layout.ts` — 布局纯函数
- `poiesis/src/components/canvas/code/nodes/LoopNode.tsx` — while/for-in 节点
- `poiesis/src/components/canvas/code/fields/TokenPreview.tsx` — 只读 token 预览
- golden 语料 `CodeFlowLoop.omni`

**修改**：
- `poiesis/src/components/canvas/code/fields/expression-trigger.ts` — tokenizer 扩展
- `poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx` — Notion 式内部实现替换
- `poiesis/src/services/codegen/syntax-nodes.ts` — formatSyntaxNode + 2 case
- `poiesis/src/services/codegen/graph-compiler.ts` — emitNode loop case
- `poiesis/src/services/codegen/graph-parser.ts` — detectSegmentType/parseSyntaxProps loop 段
- `poiesis/src/components/canvas/code/flowConstants.ts` — MENU_SECTIONS/TAG_CN/CODE_TAGS
- `poiesis/src/components/canvas/code/FlowInspector.tsx` — loop 表单
- `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx` / `ConditionNode.tsx` / `ReturnNode.tsx` / `fields/NodeField.tsx` — TokenPreview 接线
- `poiesis/src/components/design/DesignToolbar.tsx` — 布局按钮
- `poiesis/src/components/canvas/code/FlowCanvas.tsx` — 布局按钮接线 + nodeSizes 来源
- 测试文件（flow-layout/tokenizer/loop round-trip/组件）对应更新

**编译器（omnilang crates）零改动**；`@pid` 持久化格式零改动。
