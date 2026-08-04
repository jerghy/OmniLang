# ADR-060: CodeFlowCanvas 遗留清理（门禁信封适配 / 分支真实 id / 位置持久化独立字段 / 公式框扩展 / redo / fn 双击）

## Status

Implemented

## Context

ADR-059（公式框）交付后遗留 7 项技术债，2026-08-04 用户拍板清理（计划 `docs/superpowers/plans/2026-08-04-codeflow-leftovers.md`，7 任务 + 收尾）：

1. **保存门禁静默失效（HIGH）**：`compile_multi_to_omniview` 自 ebd9cacd 起返回 `MultiCompileEnvelope { result, cache_state }` 信封（错误在 `result.result.errors`），`validateSources` 旧实现只查顶层 `errors` → 恒返回 `[]` → design/store/component 三处保存门禁全部静默放行非法源（`else` 在 `elif` 前、`let 1x = 0` 等均可保存）。
2. **condition 分支 id/索引漂移**：graph-compiler 用数组索引合成端口 key（`br${i}`），连线 handle 用分支真实 id——历史源/手写编辑使 branches 数组乱序（如 `[br0, br2, br1]`）时查错分支或静默丢分支体。
3. **节点位置不持久化**：物化元素无 position（解析侧不生成），reload 后全部堆叠/回行布局，拖拽布局保存即丢。
4. **公式框未覆盖 condition/return/args**：assign/let 已有公式框（ADR-059），condition 条件、return 表达式、store-call/call 参数行仍是纯文本。
5. **redo 缺失**：undo 有按钮，redo 无入口。
6. **fn 双击打开空白 Monaco**：fn 无 source（CodeBlockEditor 打开即空白）。
7. **延期 minor**：表达式触发显式化、运算符串多字符回扫、转义引号跳过、fn 拓扑 BFS 跨 fn 泄漏、`@pid` 注释约定、物化空合并短路、`createCleanState` 未清 persistedPositions 等。

约束：`@pid` round-trip 契约零改动；graph-parser 的分支 id 扫描序生成不改（round-trip 既有设计）；return 表达式仍存 `el.source`；中文注释；无新依赖。

## Decision Drivers

* **门禁必须真实**：WASM 分析是保存最终裁决，信封适配是门禁恢复的唯一缺口；适配后行为变化（非法源被拦）是**预期修复**，非回归
* **编译器消费侧修复而非解析侧**：分支 id 的失配根源在发射侧合成 key，解析侧 id 生成是 round-trip 稳定性既有设计（`br${branchIndex}` 扫描序）——修消费侧 + UI 追加防碰撞（max+1），两处小改消除漂移
* **持久化独立字段而非改图形状**：v3 `codeFlowGraphs` 是解析侧快照（保存不回流），改图形状带 position 纯属浪费（加载侧置 `codeFlowGraphs={}`，图内 position 到不了物化）；独立 `persistedPositions`（per-ID Record）merge-friendly、幂等（物化跳过已存在元素）、链上各处零改动
* **共享单一来源**：公式框候选源 buildFormulaSource 归服务层（flow-scope.ts 纯函数），condition/return/SyntaxNode 三处共用——消除第三处复制
* **最小交互改点**：redo 只加按钮 + store 既有 `redo()`；fn 双击只改 data 构造一处（`onDoubleClick: undefined`），节点组件零改动

## Decision

### 1. 保存门禁信封适配（Task 1）

`validateSources`（diagnostics.ts）在顶层 `errors` 检查之前插入信封分支：`"result" in result` → 取 `result.result.errors ?? []`。顺序关键：envelope 无顶层 errors、旧形状无 `result` 键，两分支互斥。保留旧形状分支（防御性）。调用补第 4 参 `cache_state`（wasm 绑定 4 参必填，同 rustCompiler.ts:301 既有模式）。**一处修复三处门禁**（design/store/component 保存路径共用 validateSources）。

### 2. condition 分支端口用真实分支 id（Task 2）

- graph-compiler `emitNode` condition 分支：`brPortId = br.id`（原 `br${i}` 索引）——连线 handle 即分支真实 id，解析/UI 追加均产生唯一 id
- ConditionNode `handleAddBranch`：新分支 id 取现有 id 的 `max+1`（非 `branches.length`——乱序数组时 length 序 id 可能重复 → 连线 handle 冲突）
- **不改 graph-parser 的 id 生成**（扫描序 `br${branchIndex}` 是 round-trip 稳定性既有设计；compiler 现按真实 id 查端口，与 parser 生成天然一致）

### 3. 位置持久化独立 `persistedPositions` 字段（Task 3）

- ProjectJson 顶层新增 `persistedPositions: Record<string, {x,y}>`（merge-friendly；migration 旧文件回退 `{}`）
- **保存侧**：toProjectJson 收集——扫 `store.elements` + 各 tab `tabData` 槽（跨 tab 不丢；活动 tab 最后覆盖，最新编辑优先）；不依赖 `codeFlowGraphs` 形状
- **加载侧**：loadOmniFiles 顶部 `await api.annotation.loadProject()`（在 .omni 解析**之前**——解析合并图会覆盖 codeFlowGraphs 字段，顺序颠倒 F1 物化失效）；修复 load-manager `newProject()` 清 projectDir 的既有缺陷（改 `resetEditorState()` 保留 dir/name）
- **物化消费**：materializeFnGraphs 缺 position 的新元素先查 `persistedPositions[e.id]`（命中不占行布局序号），未命中回退确定性行布局（y 递增 60，防全叠原点）；已存在元素不覆盖（画布编辑优先）
- `createCleanState` 补清 `persistedPositions`（跨项目 @pid 相同会套旧位置）

### 4. 公式框扩展 condition/return/args（Task 4/5）

- ConditionNode 条件字段（if/elif 行）→ NodeFormulaField（candidates=formulaSource，提交走 onEditCondition；else 行保持不可编辑）
- ReturnNode 表达式 → NodeFormulaField（提交走 onCommitReturn → setElementSource，`el.source` 语义不变）
- NodeArgsRows 可选 `candidates` prop：有 candidates 时每行（含 "+ 添加参数" 本地 adding 行）渲染 NodeFormulaField，无则退化 NodeTextField（既有 12 用例零改动）
- 无 scope 时 buildFormulaSource 返回 undefined → 退化纯文本（与 SyntaxNode 既有行为逐字一致）

### 5. buildFormulaSource 服务层共用（Task 4 归并）

`flow-scope.ts` 新增导出 `FormulaCandidateSource` 接口 + `buildFormulaSource(scope, letVars)`（纯函数层定义自己的接口，结构同 NodeFormulaField 版本）；SyntaxNode 原内联组装（69-82 行）整体搬移为调用共享函数，DRY 第三处复制。

### 6. redo 按钮（Task 6）

DesignToolbar 加 redo 按钮（undo 旁，↪），props `onRedo`；undo/redo 均补 aria-label。消费 store 既有 `redo()`（undo/redo 栈为既有机制，按钮只是入口补齐）。

### 7. fn 双击不再打开空白 Monaco（Task 6）

FlowCanvas data 构造：`onDoubleClick: el.tag === "fn" ? undefined : () => setEditingNodeId(el.id)`——fn 双击回落 onNodeClick 选中（FlowInspector 参数表单）；code-block 等双击开 Monaco 不受影响。fn 无 source（CodeBlockEditor 打开即空白）故不进入编辑。

### 8. minor 清理（Task 7）

表达式触发显式化（`!` 断言 → `?? ""`）、运算符串多字符回扫 + 光标在串中间前扫补齐、转义引号跳过、openMenu 死参数移除、fn-topology BFS 边界截断（跨 fn 不收集）+ id 索引 Map、`@pid` 须紧邻 fn/store 声明注释、物化空合并短路（引用保持）、view {expr} 状态机专测、createCleanState 补清。

**Architectural Components Affected:**
* `poiesis/src/services/monaco/diagnostics.ts`——validateSources 信封适配
* `poiesis/src/services/codegen/graph-compiler.ts`——分支端口真实 id
* `poiesis/src/components/canvas/code/nodes/ConditionNode.tsx`——分支 id 防碰撞 + 条件公式框
* `poiesis/src/core/types/project-json.ts`、`poiesis/src/services/persistence/project-json.ts`、`migration.ts`——persistedPositions 字段
* `poiesis/src/store/slices/serialization.ts`——保存收集 + 加载恢复
* `poiesis/src/store/slices/element-crud.ts`——物化消费 persistedPositions + 空合并短路
* `poiesis/src/services/persistence/load-manager.ts`——newProject → resetEditorState
* `poiesis/src/App.tsx`——loadOmniFiles 顶部 loadProject 接线
* `poiesis/src/services/codegen/flow-scope.ts`——buildFormulaSource 共享
* `poiesis/src/components/canvas/code/fields/NodeField.tsx`——NodeArgsRows candidates
* `poiesis/src/components/canvas/code/nodes/ReturnNode.tsx` / `SyntaxNode.tsx`——公式框接线
* `poiesis/src/components/design/DesignToolbar.tsx`——redo 按钮
* `poiesis/src/components/canvas/code/FlowCanvas.tsx`——fn 双击
* `poiesis/src/services/codegen/expression-trigger.ts`、`fn-topology.ts`、`parser.ts`（注释）——minor

**Interface Changes:**
* `validateSources` 行为变化：非法源保存现被拦截（预期修复）
* ProjectJson 新增顶层 `persistedPositions`
* `NodeArgsRows` Props 新增可选 `candidates`
* `NodeFormulaField` `openMenu` 去两参（内部）
* DesignToolbar Props 新增 `onRedo`

## Consequences

### Positive

* **门禁恢复真实裁决**：非法源（非法分支序、非法标识符、未声明引用）保存即被拦，错误明细直达工具栏（E2E 实测 6 错误清单）；此前静默写入磁盘的坏源不再产生
* **分支体不再静默丢失**：乱序 branches 数组 round-trip 完整（`[br0, br2, br1]` 顺序断言测试）
* **reload 布局保持**：拖拽 → 保存 → 真实进程重启 → 进入 codeflow 布局逐点恢复（E2E 实测 5 节点坐标逐字吻合）
* **公式框全节点覆盖**：condition 条件/return 表达式/args 参数行均触发式补全，候选数据同一来源
* **redo 入口补齐**：undo/redo 对称（redo 机制缺陷见 Negative）
* **fn 双击不再空白**：回落选中 + FlowInspector 表单
* **全量 429/429 + tsc 零错误**（58 文件；含本系列净增 32 测试）

### Negative

* **redo 机制结构性缺陷（E2E 发现，未修）**：`computeInverse`（undo-apply.ts）仅 `reorderElement`/`setElementsAndWires` 区分 undo/redo 方向；addElement/removeElement、addWire/removeWire、setProp、moveElement 等方向性动作的 redo 应用与 undo 相同逆操作（redo 删除 = 再删、redo 添加 = 再删）——redo 按钮（Task 6 新增）暴露了既有机制缺陷。修复轮由计划作者裁决（undo 方向实测正常）
* **门禁行为变化影响既有 demo**：demo2 的 TodoStore/count 未声明引用此前可静默保存，现被拦（预期）；演示项目需先合法化
* **物化链共享节点仅发射一次**：同一节点被两条分支可达时（如 call→return 边 + br1→return 边），后到的分支体为空——链式发射 visited 语义（既有设计，非本系列引入）

### Neutral

* persistedPositions 与 codeFlowGraphs 双轨并存（解析侧快照 vs 画布侧实时），各自 merge-friendly；保存侧不依赖 codeFlowGraphs 形状
* 门禁收集源集合的盲区：新建文件（如磁盘手写 TodoStore.omni）需先经 app 打开为 tab 才进已知路径集合（collectProjectSources 不列目录）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中等偏小——全在 poiesis 前端（诊断层 1 函数 + 画布组件 + store 持久化链 + 控件扩展），编译器（omnilang crates）零改动；`@pid` 持久化格式零改动

**Affected Components**: 见上表（~18 文件 + 5 新建测试文件）

**User Impact**: 非法源保存被拦；分支/布局/redo/公式框/双击五处体验补齐

**Risk Mitigation**: 逐任务 TDD + 独立审查 + 全量 429 测试 + tsc 零基线 + Victauri 真机 E2E（门禁/分支/位置/公式框/redo/双击/回归）

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: 代码级回退；`persistedPositions` 字段对旧版本无害（未知字段忽略）；门禁修复回退即恢复静默放行

**Migration Paths**:
- **Forward Migration**: 无迁移（新增字段 migration 回退 `{}`）
- **Rollback Migration**: `git revert` 合并 commit
- **Evolution Path**: redo 方向性修复（undo-apply.ts 补 isRedo 分支）；门禁收集源改目录扫描；循环节点（for/while）

**Commitments Made**: `@pid` round-trip 契约持续为稳定性基础；graph-parser 分支 id 扫描序生成不改；return 表达式 `el.source` 契约不变；门禁信封分支与旧形状分支互斥保留

### Sequencing & Timing

**Prerequisites**:
- [x] Task 1-7 全部完成（10 commits：fc04cddf → 0540c672，逐任务审查 + 评审 clean）
- [x] 429/429 tests 全绿 + tsc 零错误
- [x] Victauri 真机 E2E（Task 8 收尾：门禁/分支漂移/位置持久化真实进程重启/公式框/redo/fn 双击/回归）

**Readiness Assessment**: Ready to implement —— 已实现并合并 main（本 ADR 提交）

### Social Cost

**Learning Curve**: Low——交互与既有公式框/拖拽一致；persistedPositions 概念与 codeFlowGraphs 同构

**Cognitive Load**: 低——每项决策单点改动，无新抽象

**Documentation Needs**:
- [x] 实现计划 `docs/superpowers/plans/2026-08-04-codeflow-leftovers.md`
- [x] 任务台账 `.superpowers/sdd/2026-08-04-codeflow-leftovers/progress.md`（L-Task 1-7 全记录）

## Alternatives Considered

### 门禁：修 wasm 返回形状（拒绝）

让 wasm 恢复旧返回形状。
**拒绝原因**：信封是 cache_state 特性（增量缓存）的正确演化，wasm 侧为权威形状；前端适配是正确方向（且仅 1 处函数）。

### 分支 id：改 graph-parser 生成顺序（拒绝）

解析侧按 id 排序输出 branches。
**拒绝原因**：`br${branchIndex}` 扫描序是 round-trip 稳定性既有设计（文件序即解析序）；改解析侧破坏既有 id 语义，修消费侧（按真实 id 查端口）即可。

### 位置持久化：codeFlowGraphs 加 position / 改 v3 形状（拒绝，设计 B 采纳）

**拒绝原因**：codeFlowGraphs 是解析侧快照，保存不回流；加载侧置 `codeFlowGraphs={}`，图内 position 到不了物化——改图形状纯属浪费。独立字段零改动链上各处，幂等天然成立。

### 公式框：SyntaxNode 候选组装复制到三处（拒绝）

**拒绝原因**：第三处复制（SyntaxNode 内联组装已是第二处）——提升到 flow-scope.ts 共享，行为逐字一致（原代码整体搬移）。

### 位置持久化加载：直接接 loadProject 链（探索后修正）

`loadProject` 链无既有调用方，直接接线后 `newProject()` 会清 projectDir 打回 Welcome 页——先修 load-manager 再接线；且必须在 .omni 解析**之前**执行（解析合并图覆盖 codeFlowGraphs）。

## Validation

- 429/429 vitest 全绿（58 文件；397 基线 + 32 净增）+ `tsc --noEmit` 零错误（Task 1 消除 diagnostics.ts:151 基线错误）
- Victauri 真机 E2E（Task 8 报告）：
  - **门禁**：`let 1x = 0`（非法标识符）→ 保存 → 拦截（⚠ 6 错误清单，含 expected variable name）；修复后保存通过（磁盘核验无写入）
  - **分支漂移**：画布 +elif/+else 三分支连线 → 保存 → 文件三分支体完整（if/elif/else 各带 body）→ 重启后保持
  - **位置持久化**：拖拽 5 节点到特征布局 → 保存 → **真实进程重启（kill + relaunch）** → 进入 codeflow → persistedPositions 坐标逐字恢复（E2E 缺陷警告：勿用 location.reload——Vite HMR 残留 loadedRef 阻止 mount effect）
  - **公式框扩展**：condition 条件/return 表达式/args 参数行/value 字段输入字母弹候选（"t"→[total]）
  - **redo**：undo 正常（添加节点撤销消失）；**redo 不恢复——真实缺陷（computeInverse 方向性缺失），已记录待裁决**
  - **fn 双击**：dblclick 不再打开 Monaco，回落选中
  - **回归**：F1 重启后物化恢复（画布全节点在）；round-trip（@pid id 与文件逐字吻合）；门禁同时拦下 E0010（void fn 返回值）
- 磁盘产物核验：demo2 `TestCodeFlow.omni` 三分支 + `@pid` id 与画布元素逐字吻合

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 7 项遗留均单点、低耦合：门禁 1 函数适配；分支 2 行消费侧 + 1 处防碰撞；持久化独立字段（不动既有形状）；公式框复用既有组件；redo/fn 双击各 1 行。无新架构、无新依赖。

**Proposed Decision**: "门禁信封适配 + 分支真实 id + persistedPositions 独立字段 + 公式框复用扩展 + redo/fn 双击最小改点"

**Necessity Assessment**: 8/10
- **Current need**: 门禁静默放行（HIGH 数据质量）、分支体静默丢失、布局保存即丢、redo 无入口均为真实缺口
- **Future need**: 位置持久化是画布编辑的基础设施；公式框覆盖是 ADR-059 的自然补齐
- **Cost of waiting**: 非法源持续写入磁盘、拖拽布局反复丢失
- **Evidence of need**: 用户拍板清理 + E2E 全程复现

**Complexity Assessment**: 2.5/10
- **Added complexity**: 每项单文件单函数改动；持久化字段零形状变更
- **Maintenance burden**: 低——双轨字段（codeFlowGraphs/persistedPositions）需注释说明分工
- **Learning curve**: 与既有模式同构
- **Dependencies introduced**: 零

**Alternative Analysis**: 修 wasm 形状（拒绝）、改 parser 生成（拒绝）、改 codeFlowGraphs 形状（拒绝）均为"改权威侧换表面一致"，所选方案全部落在消费侧/独立字段，爆破面最小。

**Simpler Alternative Proposal**: 位置持久化可只存 codeFlowGraphs 内联 position——无法满足"解析侧快照不回流 + 加载侧置空"的既有设计，不成立。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 2.5/10
- **Ratio**: 0.313 *(Target: <1.5)*

**Overall Assessment**: 适当的工程——7 项决策全部最小改点、消费侧修复、独立字段零形状变更；E2E 发现的 redo 方向性缺陷为既有机制问题（按钮暴露），属修复轮范畴。

## References

- 实现计划：`docs/superpowers/plans/2026-08-04-codeflow-leftovers.md`
- 任务台账：`.superpowers/sdd/2026-08-04-codeflow-leftovers/progress.md`
- Task 1-7 报告：`.superpowers/sdd/2026-08-04-codeflow-leftovers/task-{1..7}-report.md`
- Task 8 报告（E2E 全记录 + redo 缺陷根因链）：`.superpowers/sdd/2026-08-04-codeflow-leftovers/task-8-report.md`
- 上游决策：ADR-057（语法节点 + `@pid` round-trip）、ADR-058（受控 ReactFlow 拖拽回写）、ADR-059（公式框）
- 权威来源：omnilang-wasm `MultiCompileEnvelope`（omnilang-wasm/src/lib.rs:81-85）
- 遗留项（修复轮）：redo `computeInverse` 方向性缺陷（undo-apply.ts 仅两动作区分方向）；门禁收集源目录扫描；物化链共享节点发射语义
