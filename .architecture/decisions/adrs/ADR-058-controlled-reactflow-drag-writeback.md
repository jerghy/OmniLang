# ADR-058: 受控 ReactFlow 拖拽位置经 onNodesChange 回写

## Status

Implemented

> commit 9948b6fb，2026-08-04

## Context

CodeFlowCanvas（FlowCanvas.tsx）节点拖拽存在用户可见缺陷：**拖动节点不跟随鼠标，松开后才瞬移到位**。经 systematic-debugging（源码 + 真机双证据）定位根因：

- FlowCanvas 以**受控模式**使用 @xyflow/react v12（传 `nodes` prop，不传 `defaultNodes`）
- xyflow 内部 store 的 `triggerNodeChanges` 对拖拽中的位置更新做 `if (hasDefaultNodes)` 门控（`hasDefaultNodes = defaultNodes !== undefined`，react 包源码 3486-3491 行）——受控模式下内部 `setNodes` 被跳过，变更**只经 `onNodesChange` 回调**暴露给应用
- FlowCanvas 未接 `onNodesChange` → 拖拽中位置更新全部丢弃 → 节点冻结在存储位置；仅 `onNodeDragStop`（松开）提交最终位置 → 瞬移
- 原代码注释"During drag, ReactFlow manages node positions internally (uncontrolled for position)"——该假设在受控模式下不成立

实测证据（Victauri 合成 mousedown/mousemove 序列采样节点 DOM transform）：拖拽中（`dragging` 类存在）transform 恒为存储位置，`mouseup` 后跳变到最终位置；且拖拽期间 FlowCanvasInner **零重渲染**（排除"父重渲染→setNodes 覆盖"假设路径）。

## Decision Drivers

* 修复用户可见的拖拽体验缺陷（冻结 + 瞬移）
* 遵循 xyflow v12 受控模式的规范契约（onNodesChange 是受控流位置变更的唯一通道）
* 不污染 undo 栈（每帧提交必须无 undo 副作用）
* 保持 props 为单一事实来源、不引入 overlay 状态的额外生命周期

## Decision

FlowCanvas 接入 `onNodesChange`，对 `type === "position"` 的变更**实时回写** `setElementPosition(change.id, change.position)`；删除冗余的 `onNodeDragStop` 与错误注释。

```tsx
const handleNodesChange = useCallback(
  (changes: NodeChange[]) => {
    for (const change of changes) {
      if (change.type === "position" && change.position) {
        setElementPosition(change.id, change.position);
      }
    }
  },
  [setElementPosition],
);
```

**关键前提验证**：`setElementPosition`（store/slices/element-crud.ts:167）纯映射 + `isDirty: true`，**无 buildUndoUpdate**——每帧提交不产生 undo 条目。拖拽末帧（`dragging: false`）的 position 变更即最终落点，故 `onNodeDragStop` 冗余。

**Architectural Components Affected:**
* FlowCanvas.tsx（ReactFlow props 接线）

**Interface Changes:**
* 无公开接口变化；ReactFlow 新增 `onNodesChange` 内部 props

## Consequences

### Positive

* 拖拽实时跟随鼠标（与受控流规范一致，xyflow 官方受控示例同款模式）
* 无 undo 污染（setElementPosition 无 undo 条目，已源码验证）
* 代码更简单：移除误导注释 + 冗余 handler，单一回写路径

### Negative

* 拖拽期间每帧触发 store 写入与 FlowCanvasInner 重渲染（受控模式的标准成本；画布节点数小，可接受）
* `isDirty` 每帧置 true（拖拽期间保存指示恒为脏——编辑中本就脏，无实际影响）

### Neutral

* 选择/删除/尺寸等非 position 变更被显式忽略（我们的选中态由自有 selectedNodeIds 管理）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 仅 CodeFlowCanvas 拖拽路径；store 无 schema 变化；round-trip 管道零触碰。

**User Impact**: 拖拽体验修复；无回退风险（原行为即缺陷）。

**Risk Mitigation**:
- 每帧提交经真机验证无抖动/无 snap-back（store 写入位置 = xyflow 刚计算位置，adoptUserNodes 对等引用重建不产生位移）
- undo 栈无污染经源码验证

### Reversibility

**Reversibility Level**: High——移除 onNodesChange 两行即回退（回到缺陷行为，故无回退动机）；12 行接线 + 3 用例测试。

### Sequencing & Timing

**Prerequisites**: 无。

**Readiness Assessment**: Ready to implement（已实现并验证）。

### Social Cost

**Learning Curve**: Low——受控 ReactFlow 必须接 onNodesChange 是 xyflow 文档规范；本 ADR 记录该陷阱防止回归（避免未来"优化"重新移除接线）。

**Documentation Needs**:
- [x] ADR-058 记录根因与契约
- [x] FlowCanvas 注释说明 hasDefaultNodes 门控机制

### Confidence Assessment

**Model Correctness Confidence**: High——根因经库源码（react 包 3486-3491 门控）+ 运行期双证据（拖拽中 transform 冻结 + 父组件零重渲染）；修复后真机逐帧跟随。

**Assumptions**:
1. setElementPosition 保持无 undo 语义——**Validation**: 已源码验证（element-crud.ts:167-170）；若未来加 undo，需在 onNodesChange 中过滤 `dragging: true` 变更
2. 画布节点规模小，每帧重渲染可接受——**Validation**: 真机拖拽无卡顿；若未来大画布需要优化，可升级为拖拽 overlay（见 Alternatives B）

## Implementation

**Phase 1（已完成）**:
* `FlowCanvasDrag.test.tsx`（mock @xyflow/react 捕获 props，3 用例：接线存在 / position 回写 / 非 position 忽略）——修复前 RED
* FlowCanvas 接入 `handleNodesChange`；删除 onNodeDragStop 与错误注释
* 319/319 回归 + 真机 Victauri 拖拽采样验证（拖拽中 transform 逐帧跟随、松开无瞬移）

## Alternatives Considered

### A. 本地拖拽 overlay（组件内 state 覆盖拖拽中位置，drag 结束一次性提交）

**Pros**: 避免每帧 store 写入与整树重渲染（大画布性能优势）。
**Cons**: overlay 生命周期管理（开始/结束/取消清理）、与 store 双源同步、undo 边界复杂——为当前画布规模（数节点）引入不必要的状态机。YAGNI 拒绝。

### B. 改 uncontrolled（defaultNodes + onNodesChange 全权管理）

**Pros**: 与 xyflow 内部路径对齐（hasDefaultNodes=true）。
**Cons**: 全链路改造（选中/删除/增删节点/跨 tab 同步都要重接 onNodesChange 与 store 映射），大爆炸风险，收益为零。拒绝。

### C. 维持原状（仅 drag 结束提交）

**Cons**: 即缺陷本身。拒绝。

## Pragmatic Enforcer Analysis

**Reviewer**: AI Software Architect
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 极简修复——12 行接线 + 1 个测试文件，无新依赖、无新抽象、无状态机。针对当前真实缺陷（用户可见 bug），非投机性设计。

**Decision Challenge**:

**Proposed Decision**: "受控 ReactFlow 拖拽位置经 onNodesChange 实时回写 setElementPosition（每帧提交）"

**Necessity Assessment**: 9/10
- **Current need**: 用户报告的真实缺陷，拖拽是画布核心交互
- **Future need**: xyflow 受控模式契约是持续约束，任何新画布/升级都需此接线
- **Cost of waiting**: 缺陷持续存在；每帧提交模式无后续迁移成本
- **Evidence of need**: 源码门控（hasDefaultNodes）+ 真机冻结/瞬移双证据

**Complexity Assessment**: 1/10
- **Added complexity**: 一个过滤循环 + 一个 prop
- **Maintenance burden**: 近零（onNodesChange 是 xyflow 标准接口）
- **Learning curve**: 低——受控模式规范
- **Dependencies introduced**: 无

**Alternative Analysis**: 三个备选（overlay / uncontrolled / 维持原状）均充分评估；overlay 与 uncontrolled 为当前规模过度设计，维持原状即缺陷。无"do nothing"可行选项。

**Simpler Alternative Proposal**: 无更简方案——接线即最小可行修复。

**Recommendation**: ✅ Approve decision

**Justification**: 当前真实缺陷 + 规范契约 + 最小改动 + 零新依赖；overlay 等复杂方案留待大画布性能需求触发（YAGNI）。

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 1/10
- **Ratio**: 0.11 *(Target: <1.5)*

**Overall Assessment**: 恰当工程——针对用户可见缺陷的最小规范修复，无投机复杂度。

## Validation

**Acceptance Criteria:**
- [x] 拖拽中节点 transform 逐帧跟随鼠标（真机 Victauri 采样验证）
- [x] 松开后位置 = 最后拖拽位置，无瞬移
- [x] undo 栈无位置条目（setElementPosition 无 undo 语义，源码验证）
- [x] 319/319 vitest 回归；tsc 零新增错误

**Testing Approach:**
* FlowCanvasDrag.test.tsx：mock @xyflow/react 捕获 ReactFlow props，直接调用捕获的 onNodesChange 断言 store 回写（position 变更回写 / 非 position 忽略 / 接线存在）
* 真机：合成 mousedown/mousemove/mouseup 序列采样节点 DOM transform

## References

* @xyflow/react 12.11.1 / @xyflow/system 0.0.78 源码：`triggerNodeChanges`（react 包 3486-3491 行 hasDefaultNodes 门控）、`adoptUserNodes`（checkEquality 引用对等）、`XYDrag.updateNodePositions`
* [Session 2026-08-04 CodeFlowCanvas Inline Editing](memory: session-2026-08-04-codeflowcanvas-inline-editing.md)（含拖拽修复根因记录）
* xyflow 官方受控节点示例（onNodesChange + applyNodeChanges 模式）
