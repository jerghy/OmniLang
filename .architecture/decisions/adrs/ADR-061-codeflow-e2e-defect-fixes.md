# ADR-061: CodeFlowCanvas E2E 缺陷修复（中链 condition round-trip / redo 方向性）

## Status

Implemented

## Context

ADR-060（遗留清理）交付后，Victauri 真机 E2E（L-Task 8）发现 2 个真实缺陷，均经独立修复轮（2 commits）修复。本 ADR 记录这两个决策——它们改动了 graph-parser 的重建语义与 undo 机制，此前仅在 commit 中无正式记录：

1. **中链 condition round-trip 数据丢失（HIGH）**：`graph-parser.ts` 线形重建 `else if (seg.type !== "condition")` 跳过**中链** condition 段（condition 位于线性链中间、非首段时）——重载后 `assign→condition` 入链 wire 与末分支 wire 丢失，condition 成孤岛，**下次保存条件结构整体丢失**（graph-compiler 从画布元素编译时找不到完整图）。golden 语料只覆盖 condition 为首段，此形状无测试暴露。附带两个次根因：段内 `} else`/`} else if` 过渡被短路分支漏检（多分支 wire 错接）；语句 props 正则被多行段尾行污染（`fn()` 数据损坏）。UI 侧另一隐患：ConditionNode 无底部 flow-out handle——条件块后置同深度代码经 flow-out 续接（graph-parser 既有重建），但 UI 无法渲染该 handle → 不可见边（xyflow error008）。
2. **redo 方向性缺陷（Important）**：Task 6 的 redo 按钮暴露既有缺陷——`undo-apply.ts` 的 `computeInverse` 仅 `reorderElement`/`setElementsAndWires` 区分 undo/redo 方向（docstring 自证 "Only matters for actions where the undo and redo directions differ"），**添加/删除/连线/setProp 的 redo 不生效**（undo 正常）。另 `setElementBinding` 无 computeInverse case → 该动作 undo/redo 均静默无操作。

约束延续：`@pid` round-trip 契约零改动（parser 分支 id 扫描序生成保持）；与编译器 emit 语义对称；undo 既有行为兼容（旧逆动作语义等价）。

## Decision Drivers

* **数据丢失必须根治**：中链 condition 是常见图形（前置语句 → 判断 → 后置语句），reload 即丢 wire、保存即丢结构——修复必须让解析侧重建与编译器发射**对称**（compiler 语义：condition 分支经分支端口发射 + 条件块后置段经 flow-out 续接）
* **修复消费侧而非生成侧**：parser 分支 id 扫描序是 round-trip 稳定性既有设计（ADR-059/060 重申），只改重建逻辑与 UI 端口，不动 id 生成
* **undo 机制单向数据→双向**：既有"仅 2 动作区分方向"的设计是残缺的（其余动作的 redo 路径错乱），统一为正向动作 + 按方向双向应用，与既有 2 动作模式对齐而非另起炉灶

## Decision

### 决策 1：中链 condition round-trip 修复（ff289059）

**Architectural Components Affected:**
* `graph-parser.ts` 线形重建（wire 重建循环）——condition 段进入线形通道：前驱 `flow-out → condition flow-in` 入链 wire；条件块后置段经 flow-out 续接（与编译器 emit 对称）
* `graph-parser.ts` 分支过渡检测——`} else {`/`} else if` 从 else-if 链脱离为**独立检测**（`!isElifElseCode` 仅排除段首过渡防重复推进；段内残余过渡无条件续设 `pendingTransition`，2c 消费后不再短路漏检）
* `graph-parser.ts` `parseSyntaxProps`——多行段 props 解析**截断到首个结构行**（trim 后 `}` / `^}\s*else` / `^else\s*(if\b|\{)`），杜绝尾行污染（`fn()` 数据损坏）
* `ConditionNode.tsx`——补底部 **flow-out source handle**（Position.Bottom，id "flow-out"），使后置续接 wire 可见可编辑（消除不可见边 error008；与既有 flow-in/分支 handle 无 id 冲突）

**Interface Changes:**
* `parseCodeFlowGraph`/`compileCodeFlowGraph` 签名不变；condition 元素形状不变；**行为变化**：中链 condition 现在重建入链 wire 与后置 flow-out 续接（此前丢失）
* `ConditionNode` 端口集合 +1（flow-out）——拖出创建/连线语义与其它节点一致（onConnectStart 已记录 flow-out 拖出）

### 决策 2：redo 方向性重构（0841e4ac）

**Architectural Components Affected:**
* `undo-slice.ts`——undo 栈改存**正向动作**（undo 弹栈应用 inverse、action 压 redo 栈；redo 反向），幂等守卫 `if (!inverse) return;` 防栈损坏
* `undo-apply.ts` `computeInverse`——按方向**双向应用**：10 种 action 全部补齐 isRedo 分支（addElement/removeElement/setProp/addWire/removeWire/moveElement/reorderElement/setElementsAndWires/setElementBinding/restoreConditionalBlock），每个方向性 case 带幂等守卫（addWire-redu 判存在、removeWire-undo 判缺失等）
* `element-crud.ts`——`setElementBinding` 补齐缺失 case（此前 default → null 静默无操作；现 undo→oldBinding、redo→newBinding，null 语义逐字镜像 slice 原实现）；restoreConditionalBlock 5 处调用点在入栈前先计算 new 值（顺序正确）

**Interface Changes:**
* `UndoAction` 联合类型 10 成员与 computeInverse 10 case 一一对应；undo/redo 对外行为不变（undo 语义与旧实现等价——addElement-undo 移除+后代+相连 wire、removeElement-undo 恢复元素+连带 wire 按 id 去重）
* 无 store API 变化；100 条上限与 per-tab 键控保留

## Consequences

### Positive

* **中链 condition 数据丢失根治**：compile→parse→compile 幂等（新 golden 语料 CodeFlowMidChain），真机重启后 6 wire 完整（assign→condition/br2→return2 恢复）
* **UI 完整性**：ConditionNode flow-out handle 使"条件块后置续接"连线可见可编辑，消除不可见边 error008 隐患
* **redo 全动作生效**：add/remove/wire/setProp/move/binding/conditional 全部 undo→redo 对称（7 用例 + 链式收敛测试 + 真机复验）
* **setElementBinding 修复附带**：此前 undo/redo 静默无操作的动作现在正确恢复

### Negative

* **undo 栈语义变更**：栈存正向动作（此前存逆动作）——依赖栈内容的外部逻辑需复核（全仓核验无消费方）
* **removeElement-undo 暴露面拓宽**：undo 恢复顶层元素 + 全部连带 wire，但**不恢复被删子树的后代元素**（既有局限）——被删元素有子代且子代带 wire 时，undo 后 wire 指向不存在元素（xyflow 不可见边）；codeflow 元素全扁平（parentId null）实际不可达，view 树 conditional 场景重载后解析重建自愈，不落盘
* **redo 栈卡滞边界**：computeInverse 返回 null 时 early return 不弹栈——该动作滞留 redo 栈顶阻塞其后动作（仅状态失同步时可触发，新动作清栈后自愈）

### Neutral

* 嵌套 condition 为外层分支末句且后接外层 elif 时，续接循环可能产生 `内层cond(flow-out)→elif首句` 与 `外层(brN)→elif首句` 双 wire（既有行为，wire() 按 from:port→to:port 去重已挡同一对）
* ADR-060 记录测试数 429 为提交时快照，修复轮后实际 440（文档漂移，无碍）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——限定 poiesis 前端（graph-parser 重建 + ConditionNode 端口 + undo 机制），编译器零改动

**Affected Components**:
- `graph-parser.ts` — 线形重建/过渡检测/props 截断（round-trip 正确性核心，golden 语料与既有测试全覆盖）
- `ConditionNode.tsx` — 端口集合（画布连线/拖出创建）
- `undo-slice.ts`/`undo-apply.ts`/`element-crud.ts` — undo 机制（10 种 action 双向）

**User Impact**: 中链 condition 保存重开结构保持（此前静默丢）；redo 按钮全动作可用；condition 后置续接连线可见

**Risk Mitigation**: RED→GREEN（中链测试修复前 3/4 失败、redo 测试 2/7 失败）+ 真机复验（重启后 wire 完整、add→undo→redo 恢复）+ 全量 440/440

### Reversibility

**Reversibility Level**: High——代码级回退（回退合并 commit 即可）；`@pid` 注释格式未变，旧代码仍可解析

**Commitments Made**: 解析重建与编译器 emit 对称是 condition round-trip 的稳定性基础；undo 双向应用是 redo 正确性的契约

### Sequencing & Timing

**Prerequisites**: 缺陷 2（数据丢失）优先于缺陷 1（功能残缺）——同一修复轮，2 commits

**Readiness Assessment**: Ready——已实现并合并 main

### Social Cost

**Learning Curve**: 低——修复沿既有模式（线形通道/过渡检测/端口集合）

**Documentation Needs**: 本 ADR + golden 语料 CodeFlowMidChain.omni 作为中链形状的规范样例

## Alternatives Considered

### 方案 A：仅修解析侧（拒绝）

只改线形重建保留 condition wire，UI 不加 flow-out handle——后置续接 wire 仍不可见（不可见边 error008 残留），且用户无法在画布上编辑该连线。**拒绝**：修复不完整，隐患转移而非消除。

### 方案 B：改动 parser 分支 id 生成（拒绝）

改为按注释/元素 id 生成分支 id 以对齐 UI——违反 ADR-059/060 的 `@pid` 契约零改动约束（扫描序是 round-trip 稳定性既有设计），且编译器/解析器/UI 三处 id 语义大改。**拒绝**：修消费侧（compiler 用 br.id + UI max+1）已闭环。

### 方案 C：redo 按旧机制逐动作补 isRedo 分支（拒绝）

在"逆动作 + 仅 2 动作区分方向"的残缺模型上继续打补丁——方向语义混乱的根源不除，未来新动作仍易漏。**采纳**：统一为正向动作 + 双向应用，与既有 2 动作模式对齐。

## Validation

- 440/440 vitest（60 文件，429 基线 + 11 新增）；tsc 零错误
- 中链测试：修复前 3/4 失败（入链 wire 缺失/多分支错接/多行 props 损坏）→ 修复后全绿；compile→parse→compile 幂等（out2 === out1）
- redo 测试：修复前 2/7 失败（add/remove wire 循环）→ 修复后全绿（7 用例：add/remove/wire/setProp 循环 + 链式收敛 + conditional 双向）
- Victauri 真机：中链 condition 画布 → 保存 → **真实进程重启** → 6 wire 完整 + flow-out handle 渲染 + 保存文件三分支体无丢失；add→undo→redo 恢复生效
- 终审（fable）：Approve，无阻塞；5 Minor 全裁决可延期

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 两个缺陷均为真实用户可见问题（数据丢失/功能残缺），修复沿既有模式（对称重建/正向动作）展开，无新抽象、无新依赖。

**Proposed Decision**: "中链 condition 重建与编译器对称 + ConditionNode flow-out handle；undo 栈改正向动作 + computeInverse 双向应用"

**Necessity Assessment**: 10/10
- **Current need**: E2E 实证数据丢失（中链 condition 保存即毁）与 redo 全动作失效（按钮已交付）
- **Future need**: condition 是画布最高频节点之一，中链形态常见；undo/redo 是编辑器基础交互
- **Cost of waiting**: 缺陷 2 每次"中链 condition + 保存重开"即丢结构；缺陷 1 每按钮点击即暴露
- **Evidence of need**: E2E 复现 + vitest 复现（RED 3/4、2/7）

**Complexity Assessment**: 4/10
- **Added complexity**: 重建逻辑 3 处小改 + 1 个 UI handle；undo 机制从"2 动作方向"统一为"全动作双向"（模式对齐非新抽象）
- **Maintenance burden**: 对称性契约（重建=发射）一句话可守；10 case 双向与既有 2 case 同构
- **Learning curve**: 低
- **Dependencies introduced**: 零

**Alternative Analysis**: 方案 A/B（不完整修复/违约大改）被拒；方案 C（打补丁）被统一模型替代——所选方案是三者中最小完整实现。

**Simpler Alternative Proposal**: 仅修缺陷 2 的解析侧（放弃 UI handle）可省 1 处改动——但不可见边 error008 与"后置续接不可编辑"使修复不完整，成本与风险不对称。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 4/10
- **Ratio**: 0.4 *(Target: <1.5)*

**Overall Assessment**: 适当的工程——数据丢失缺陷以最小对称修复根治（解析重建=编译器发射），UI handle 补齐使修复完整；undo 统一模型消除残缺方向语义；无过度设计。

## References

- 修复 commits：`ff289059`（中链 condition）、`0841e4ac`（redo 方向性）
- 设计文档：`docs/superpowers/plans/2026-08-04-codeflow-leftovers.md`（L-Task 8 + 修复轮）
- 上游：ADR-060（遗留清理 7 项）、ADR-059（公式框）
- 遗留登记：`.architecture/deferrals.md` #77（parked 观察项）
