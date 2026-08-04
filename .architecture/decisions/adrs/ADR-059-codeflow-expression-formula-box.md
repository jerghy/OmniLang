# ADR-059: CodeFlowCanvas 表达式公式框（触发式自动补全 + 运算符权威约束）

## Status

Implemented

## Context

CodeFlowCanvas（节点编辑器，ADR-057 引入 let/assign/store-call/call 语法节点）中，assign/let 节点的 value 字段此前是普通文本输入，用户需手写完整表达式（变量名、store 信号、运算符全靠记忆），无候选提示、无运算符引导，错误率高。2026-08-04 用户需求：节点编辑器应像 IDE 一样提供表达式辅助（"节点编辑器加上常用的界面开发时会用的代码……"延伸出的表达式编辑体验）。

同时暴露两个历史问题（F1/F2）：
- **F1**：进入组件 codeflow 时画布为空——fn 图只存在于解析侧（`.omni` 的 `@pid` 注释），从未物化进画布，组件 fn 的节点图重进即丢。
- **F2**：call 节点的 fnName 无候选——component fn 元数据（fnRegistry）未被登记/消费，只能手输函数名。

约束（用户确认）：
- 候选数据零新数据源：全部派生自 store 现有数据（componentDefs/storeDefs/inlineStoreDefs/elements）
- 不打断既有 `@pid` round-trip 管线（ADR-057 契约零改动）
- 运算符集合以编译器为权威，UI 不自行定义

## Decision Drivers

* **表达力 vs 简洁**：候选插入保持"草稿"语义（Enter/blur 才提交，props 仍是唯一事实来源），既有 NodeTextField 提交模式复用，不引入受控表单体系
* **触发式而非常驻**：只在字符输入产生候选上下文时弹层（operand/member/operator 三上下文），避免常驻下拉的噪音与遮挡
* **编译器权威**：运算符候选表（`OPERATOR_PREFIX_CANDIDATES`）只含 infix.rs 真实支持的运算符；`=` 仅提供 `==`（OmniLang 无赋值表达式，`=` 是赋值语句符号，不应出现在表达式候选里）
* **零新数据源**：候选 = buildExpressionCandidates(scope, letVars) 纯函数，scope 复用既有 flow-scope 数据层
* **round-trip 稳定**：F1/F2 均走解析侧合成 + 物化，`@pid` 注释格式不变

## Decision

### 1. 文本输入 + 触发式自动补全（NodeFormulaField）

assign/let 的 value 字段保留为文本输入（`role=combobox`，`data-field-kind=formula`），叠加字符触发补全：

- **字符触发规则**（`expression-trigger.ts` 纯函数，表驱动）：
  - `operand` 上下文（标识符 token）：前缀过滤候选（空 token = 表达式起点/运算符后，弹全量前 8）
  - `member` 上下文（`Store.` 点后）：store 信号候选（`storeName 信号` 分组）
  - `operator` 上下文（运算符串）：前缀运算符候选（`<` → `<`、`<=`）
- **单字符直插**：`+ - * / %` 五个单字符运算符由用户直接键入，不弹层不重复插入
- **多字符前缀弹框**：`< = ! & | ?` 等多字符运算符前缀弹选择框，选中经 `replaceToken` 替换（不重复已键入前缀）
- **`=` 只提供 `==`**：运算符候选表以 `infix.rs` 为权威（`+ - * / % == != < > <= >= && || ?? is ?.` 与一元 `!`），`=` 不在表达式运算符之列；候选 `=` 前缀仅映射 `==`
- **候选插入仍是草稿**：选中候选只更新草稿与光标，Enter/blur 才 `onCommit`；Esc 只关弹层不取消草稿
- 无 `candidates` 源时退化为纯文本输入（无弹层）

### 2. 候选数据组装（buildExpressionCandidates / buildTargetOptions）

纯函数分组：变量（letVars + fnParams）/ 信号 / 属性 / 计算属性 / store 信号（`Store.signal`）/ 函数（component fn + store action）。`buildTargetOptions` 输出 assign target 下拉候选（变量类 + `Store.信号`）。

### 3. letVars 按 fn 拓扑归属

`collectLetVarsPerFn`（BFS wire 拓扑）把每个 let 变量归属到所属 fn；SyntaxNode 经 `findOwnerFnId` 取当前节点所属 fn 的 letVars 注入公式框——多 fn 画布互不泄漏（Task 4 决策，ADR-059 记录归并）。

### 4. assign target 下拉（NodeSelect）

assign target 字段由自由文本改为下拉（allowCustom + `invalid` 高亮保留 TARGET 校验），候选 = `buildTargetOptions(scope, letVars)`。

### 5. 括号配对轻校验

assign/let value 非空时 `countUnbalancedParens` 正负分支提示（`N 个未闭合的括号` / `N 个多余的右括号`），警告条只提示不阻塞（WASM 保存门禁最终裁决）。

### 6. F1：fn 图物化（解析侧合成 + 画布合并）

- **解析侧**（`parser.ts` `parseFn` / `synthesizeFnEntry`）：组件 fn 与 store action **共用 synthesizeFnEntry helper**（Task 6 裁定，替代计划中「镜像复制」）——从签名合成 fn 入口元素（`@pid` 稳定 id + 首语句 wire），`parsedFnBodies` 收集为 `codeFlowGraphs`
- **画布侧**（`materializeFnGraphs` + DesignPage 触发 effect）：组件 tab 进入 codeflow 时把 `codeFlowGraphs` 中该组件的 fn 图合并进画布（pid/wire id 去重、已存在不覆盖——保留画布编辑；幂等；无 undo 条目）

### 7. F2：fnRegistry 登记与消费

`parseFn` 把 fn 元数据（pid/name/params/returnType）按组件登记进 `fnRegistry` → 物化为 `componentDefs.fns`；call 节点 fnName 下拉候选 = componentDefs.fns 的 fn 名（SyntaxNode `fnCandidates`）。

**Architectural Components Affected:**
* `poiesis/src/components/canvas/code/fields/`（新增）——`expression-trigger.ts` / `NodeFormulaField` / `NodeCandidateMenu`
* `poiesis/src/services/codegen/flow-scope.ts`——`buildExpressionCandidates` / `buildTargetOptions` / `validateSyntaxProps`
* `poiesis/src/services/codegen/fn-topology.ts`——`collectLetVarsPerFn` / `findOwnerFnId`
* `poiesis/src/services/codegen/parser.ts`——`parseFn` fnRegistry 登记 + `synthesizeFnEntry` 共用
* `poiesis/src/services/codegen/graph-parser.ts`——fn 段 props 解析（name/params/returnType）
* `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx`——value 公式框 + target 下拉 + 括号提示
* `poiesis/src/components/canvas/code/FlowCanvas.tsx`——letVarsByFn 注入
* `poiesis/src/store/slices/element-crud.ts`——`materializeFnGraphs`
* `poiesis/src/pages/DesignPage.tsx`——进入 codeflow 触发物化

**Interface Changes:**
* 新增 `FormulaCandidateSource`（`filter` / `storeSignals`）与 `NodeFormulaField` props
* `parseCodeFlowGraph` / `compileCodeFlowGraph` 签名不变（`@pid` 契约零改动）
* store 新增 `materializeFnGraphs(componentName)` action

## Consequences

### Positive

* **表达式输入效率与正确率**：变量/store 信号/运算符候选直达，运算符表以 infix.rs 为权威，杜绝手输错运算符
* **round-trip 双向稳定保持**：公式框草稿提交语义 + `@pid` 注释管线零改动，保存重开节点形态、props、连线完整（390 测试 + 真机 E2E 验证）
* **F1 画布不再为空**：进入 codeflow 即恢复 fn 图（`@pid` 稳定 id），重复进入幂等
* **F2 fnName 有候选**：component fn 名直达下拉，消除手输错名
* **store action 与组件 fn 共用合成**：一条合成路径两处生效，无双实现漂移

### Negative

* **候选为空时无反馈**：operand 前缀无匹配时弹层关闭且无"无结果"提示（用户需自行判断——设计选择，避免噪音）
* **member 选中前缀重复缺陷**（E2E 发现，未修）：`Store.` + 选中 `Store.signal` → `Store.Store.signal`（member tokenStart 语义缺陷，见报告缺陷 1；修复轮由计划作者裁决）
* **物化元素无 position 堆叠 (0,0)**（E2E 确认，未修）：parseCodeFlowGraph 合成元素不生成 position，进入 codeflow 时节点全叠原点需手动拖开（修复轮由计划作者裁决）

### Neutral

* 候选菜单分组标签（变量/信号/store 信号/函数）存在于数据层；UI 菜单按 Task 3 契约渲染为扁平「候选」列表（filter 接口返回扁平数组）
* fn 节点 params 显示含解析侧空格伪影（`fn handleAdd( )`），及 **Task 5 引入的空参 fn 幻影参数保存污染**（`fn handleAdd(: String)`，见报告缺陷 3，HIGH，待修复轮）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中等——限定 poiesis 前端（codegen 服务 + 画布组件 + store 物化），编译器（omnilang crates）零改动；`@pid` 持久化格式零改动

**Affected Components**: 见上表（fields 3 新文件 + 8 个既有文件扩展）

**User Impact**: assign/let value 获得补全；assign target 变下拉；进入组件 codeflow 画布自动恢复 fn 图；call fnName 有候选

**Risk Mitigation**: 逐任务 TDD（Task 1-9 独立审查 + 2 轮返工）+ 全量 390 测试 + tsc 基线 + Victauri 真机 E2E 全链路（补全/运算符/单字符/成员/store/下拉/F1/F2/round-trip/回归抽查）

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: 代码级回退即可；已保存项目的 `@pid` 注释与 `.omni` 文本格式未变（除缺陷 3 的空参 fn 签名污染——回退到 Task 4 前的 commit 即消除复发）

**Migration Paths**:
- **Forward Migration**: 无迁移（纯 UI 增强 + 物化，语义不变）
- **Rollback Migration**: `git revert` 合并 commit
- **Evolution Path**: 循环节点（for/while，2026-07-10 spec 排队）；公式框类型感知候选（消费 flow-scope 的类型信息）；候选"无结果"提示

**Commitments Made**: 运算符候选以 infix.rs 为权威（`OPERATOR_PREFIX_CANDIDATES` 单表）；`@pid` round-trip 契约继续作为稳定性基础；`synthesizeFnEntry` 为 fn 入口合成唯一来源

### Sequencing & Timing

**Prerequisites**:
- [x] Task 1-9 全部完成（11 commits，逐任务审查 + 3 轮返工 + 最终修复波）
- [x] 390/390 tests 全绿 + tsc 仅允许基线
- [x] Victauri 真机 E2E（补全/运算符/单字符直插/成员/store 候选/target 下拉/F1/F2/round-trip/回归抽查）

**Readiness Assessment**: Ready to implement —— 已实现并合并 main（be91010d 及前序 10 commits）

### Social Cost

**Learning Curve**: Low——公式框交互与 IDE 补全一致；候选分组与画布已有 scope 概念同构

**Cognitive Load**: 低——触发规则 3 上下文表驱动，无新抽象

**Documentation Needs**:
- [x] 实现计划 `docs/superpowers/plans/2026-08-04-codeflow-expression-formula.md`
- [x] 任务台账 `.superpowers/sdd/2026-08-04-codeflow-expression-formula/progress.md`（Task 1-10 全记录）

## Alternatives Considered

### 方案 B：常驻候选下拉（拒绝）

value 字段改为 combobox 常驻候选 + 允许自由输入。
**拒绝原因**：表达式内容动态（自由文本 + 补全），常驻下拉遮挡画布、选择噪音大；触发式（只在该弹时弹）交互更符合节点编辑器密集空间。

### 方案 C：候选弹层复用 NodeSearchMenu（拒绝）

直接复用画布搜索菜单组件做候选。
**拒绝原因**：NodeSearchMenu 是节点分类菜单（键盘/焦点语义不同），候选菜单需要 listbox 受控交互（↑↓/Tab/Enter 选中、mousedown 防夺焦）；新建 NodeCandidateMenu（Task 2）独立实现，二者互不耦合。

### 方案 D：表达式校验内联强阻断（拒绝）

括号不配对/运算符非法时阻塞提交。
**拒绝原因**：与既有"WASM 保存门禁最终裁决"哲学冲突（轻校验只提示不阻塞）；阻塞会打断草稿流。

### 实现期替代：F1 画布侧合成 fn 入口（尝试后否决）

先在 materializeFnGraphs 侧凭空合成 fn 节点。
**否决原因**：画布侧无签名信息（name/params），且每次进入物化都要判断"缺 fn 则合成"；改为解析侧合成（parseFn/store action 共用 synthesizeFnEntry，`@pid` 稳定 id），画布侧只管合并去重——单一职责更清晰。

## Validation

- 390/390 vitest 全绿（Task 1-9 新增约 71 测试 + 既有 319 基线）
- tsc --noEmit 仅允许基线 diagnostics.ts:151（Task 8 测试文件 TS2339 已修）
- Victauri 真机 E2E（Task 10）：公式框候选（t→total）、运算符 `count <`→Tab→`count <` 无前缀重复、单字符 `+` 直插、`demostore.`→成员信号候选、target 下拉、F1 保存→reload→重进画布恢复（两次）、F2 fnName 候选 [handleAdd, onSelect]、round-trip（`.omni` 含 `count = count + 1;` 与 `@pid:assign-...` 且 reload 保持）、空格搜索菜单、ADR-058 回写链真机验证
- E2E 发现 3 缺陷（成员前缀重复 / 物化堆叠 (0,0) / 空参 fn 签名污染）如实记录于 Task 10 报告，修复轮由计划作者裁决
- 磁盘产物核验：demo2 `TestCodeFlow.omni` 的 `@pid` id 与验证报告逐字吻合

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 用户需求明确（表达式编辑辅助），候选数据零新数据源（纯函数派生），F1/F2 修复复用既有 `@pid`/解析管线；复杂度集中在触发规则与既有 round-trip 的融合，而非抽象膨胀。

**Proposed Decision**: "文本输入 + 触发式自动补全（3 上下文规则），运算符以 infix.rs 为权威，F1/F2 走解析侧合成 + 画布物化"

**Necessity Assessment**: 9/10
- **Current need**: 用户明确提出；表达式手输无辅助、fn 图重进即空、fnName 无候选均为真实可用性缺口
- **Future need**: 表达式辅助是节点编辑器的自然演进（类型感知候选已在 Evolution Path）
- **Cost of waiting**: 维持现状则表达式错误率高、画布空态困惑持续
- **Evidence of need**: 用户需求确认 + demo 项目真实使用（handleAdd fn 图、store 信号引用）

**Complexity Assessment**: 4/10
- **Added complexity**: 触发规则纯函数表驱动（expression-trigger.ts 一处）；候选组装纯函数（flow-scope.ts 扩展）；F1/F2 复用解析管线（零新架构）
- **Maintenance burden**: 中——核心规则集中在两个纯函数模块；缺陷 1/3（member tokenStart、空参 fn 幻影参数）暴露的契约测试缺口待修复轮补齐
- **Learning curve**: 公式框交互与 IDE 一致
- **Dependencies introduced**: 零新依赖

**Alternative Analysis**: 常驻下拉（方案 B）交互噪音大被拒；复用 NodeSearchMenu（方案 C）语义耦合被拒；内联强阻断（方案 D）与既有校验哲学冲突被拒。所选方案是四者中最小完整实现。

**Simpler Alternative Proposal**: 只做 datalist 静态补全可省弹层组件——但无法实现运算符替换、成员上下文、键盘导航等核心交互，不满足诉求。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 4/10
- **Ratio**: 0.444 *(Target: <1.5)*

**Overall Assessment**: 适当的工程——需求明确、数据零新源、复用既有 round-trip 与解析管线，触发规则单一来源；E2E 暴露的 3 个缺陷为契约测试缺口（member 选中语义、空参 fn、position 合成），属修复轮范畴而非设计否决项。

## References

- 实现计划：`docs/superpowers/plans/2026-08-04-codeflow-expression-formula.md`
- 任务台账：`.superpowers/sdd/2026-08-04-codeflow-expression-formula/progress.md`
- Task 10 报告（E2E 全记录 + 3 缺陷根因链）：`.superpowers/sdd/2026-08-04-codeflow-expression-formula/task-10-report.md`
- 上游决策：ADR-057（语法节点 + `@pid` round-trip）、ADR-058（受控 ReactFlow 拖拽回写）
- 权威来源：编译器 `infix.rs` 运算符表（候选表唯一依据）
- 遗留项（独立修复轮）：成员上下文选中前缀重复；物化元素 position 堆叠 (0,0)；空参 fn 幻影参数保存污染（HIGH）；`validateSources` 未适配 ebd9cacd 信封（ADR-057 遗留）
