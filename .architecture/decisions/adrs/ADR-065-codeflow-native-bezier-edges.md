# ADR-065: CodeFlowCanvas 连线采用 ReactFlow 原生 bezier 拓扑曲线

## Status

Implemented

## Context

CodeFlowCanvas 连线渲染经历了三轮反复，2026-08-05 用户实测最终拍板：

1. **ADR-063 时代**：自定义 FlowEdge（DOM 测量版正交走廊路由）——wire 精确贴 handle、不穿节点，但实现复杂（每 edge rAF 重试 + ResizeObserver + MutationObserver），且依赖 v12 不传 sourceHandle 给自定义 edge 的规避技巧
2. **工作区实验（未提交）**：删除 FlowEdge，edges 不指定 type——注释声称"内置 smoothstep"但未配置 `defaultEdgeOptions` → 实际渲染 **bezier 弧线**且无人知晓（注释与实现矛盾的隐形回归）
3. **ADR-064 初版**：显式 `type: "smoothstep"`（正交折线）——解决注释矛盾，但用户实测后**主动要求换回原生拓扑曲线**：「让连线有交点，也换成这个样子」——用户偏好曲线自然交叉的拓扑图观感（多分支扇出交叉），而非平行正交走廊

约束：零新依赖；`@pid` round-trip 契约零改动；布局脊柱修复（ADR-064：主干左缘对齐 x=0 + 分支源右缘对齐列右缘 + flow-in/out handle 钉左缘）**全部保留**。

## Decision Drivers

* **用户视觉拍板优先**：连线风格是纯视觉偏好，用户明确要原生曲线交叉观感——smoothstep 正交折线被实测否决
* **原生渲染与跟随**：默认 edge 原生支持拖拽/缩放跟随、选中态，零维护成本（FlowEdge 自定义渲染的 DOM 测量开销与 rAF 重试逻辑全部消除）
* **显式声明防回归**：edge type 必须显式写出（教训：2026-08-05 曾"注释声称 smoothstep 但未配置 → 实际 bezier 无人知晓"）
* **主干脊柱不退化**：flow-in/out handle 钉左缘 + 主干左缘对齐 → 主干 wire 两端同 x → bezier 曲线退化近似垂直直线——曲线风格不牺牲"读图=读代码结构"

## Decision

连线采用 **ReactFlow 原生默认 edge（`type: "default"`，bezier 拓扑曲线）**，显式声明于 FlowCanvas.tsx edges 构造中。接受曲线在列间空隙自然交叉（多分支扇出），不追求正交避让。

**Architectural Components Affected:**
* `poiesis/src/components/canvas/code/FlowCanvas.tsx` — edges 构造 `type: "default"`
* 布局体系（flow-layout.ts / node-base.css）— **零改动**，脊柱/对齐/handle 钉位保留

**Interface Changes:**
* 无——edges 是 ReactFlow 内部渲染属性，数据层（elements/wires/store）零改动

## Consequences

### Positive

* 原生渲染：曲线跟随拖拽/缩放自动重算，选中态/交互原生支持，零自定义代码
* FlowEdge（DOM 测量 + rAF 重试 + ResizeObserver/MutationObserver）维护负担彻底消除
* 拓扑曲线观感：多分支 S 形扇出自然交叉，符合用户"有交点"的视觉偏好
* 主干 wire 两端同 x（handle 钉左缘 + 左缘对齐）→ bezier 退化为近似垂直直线，脊柱可读性保持

### Negative

* 曲线可能扫过目标列上方节点区域（ADR-063 曾实测 bezier 穿节点）——用户明确接受此视觉特征
* 分支曲线不再走固定走廊，交叉点位置不可精确预测（拓扑图特征，非缺陷）

### Neutral

* wire 端点仍精确贴 handle（端点 = handle 中心，与曲线类型无关）
* 布局算法与测试全部复用（507/507 不变）

## Implementation Strategy

> 本决策已实施完毕（2026-08-05），以下为实施记录与复盘。

### Blast Radius

**Impact Scope**: 仅影响 CodeFlowCanvas 连线渲染层；数据层、布局算法、保存链路零接触。

**Affected Components**:
- `FlowCanvas.tsx` - edges `type` 字段 + 注释（唯一改动）
- 视觉验证工具链 - 几何断言脚本（bezier 路径解析）

**User Impact**: 连线外观从正交折线变为拓扑曲线；交互（拖拽/缩放）行为不变。

**Risk Mitigation**:
- 显式 `type: "default"` 而非省略——防"注释与实现矛盾"回归（本次教训）
- 保留 handle 钉左缘——主干 wire 曲线退化直线，脊柱不退化

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**:
- 单行改动（`type: "default"` ↔ `type: "smoothstep"`），git 恢复即时
- 无数据迁移、无接口变更

**Migration Paths**:
- **Forward Migration**: 已完成（本 ADR）
- **Rollback Migration**: 改回 `type: "smoothstep"` 即可，布局/handle 体系无需任何配套改动
- **Evolution Path**: 如需调整曲线弧度/交叉密度，走 `defaultEdgeOptions` 或 edge `style`，不引入自定义 edge

### Sequencing & Timing

**Prerequisites**: 全部完成（ADR-064 布局脊柱修复已落地）

**Readiness Assessment**: Ready to implement（已实施）

### Social Cost

**Learning Curve**: Low — ReactFlow 默认行为，无新概念

**Documentation Needs**:
- [x] ADR 留痕（本文件 + ADR-064 决策 2 已注明反转）

### Confidence Assessment

**Model Correctness Confidence**: High — 实测验证：edge class 全部 `default`、路径含 C 曲线命令、主干退化直线、端点贴 handle

**Assumptions**:
1. 用户偏好曲线交叉观感持续成立 - **Validation**: 用户 2026-08-05 明示拍板（「让连线有交点」）

**Edge Cases**:
- 密集多分支图曲线交叉密集——用户接受的拓扑图特征

## Implementation

**已实施（2026-08-05）**：
1. FlowCanvas.tsx edges 构造：`type: "smoothstep"` → `type: "default"`，注释更新为原生 bezier 拓扑曲线说明 + 教训留痕
2. 实测验证：edge class 全 `default`；主干 wire（n1→n2/n2→n3）x 恒定（1.3~2）退化垂直直线；分支 wire（n4→n5）C 曲线 S 形扇出；节点位置零变化；507/507 vitest + tsc 零错误

## Alternatives Considered

### 1. 自定义 FlowEdge（ADR-063 已实施，被弃）

DOM 测量版正交走廊路由：分支 wire 走固定走廊、精确贴 handle、不穿节点。

**Pros:**
* 正交折线可预测、不穿节点
* 精确贴 handle（DOM 测量）

**Cons:**
* 实现复杂：每 edge rAF 重试 + ResizeObserver + MutationObserver
* 依赖 v12 不传 sourceHandle 的规避技巧（edges data 注入 fromPort）
* 用户视觉拍板否决正交风格——正交本身不再是目标

### 2. 内置 smoothstep（ADR-064 初版，被用户否决）

ReactFlow 内置正交折线，零自定义。

**Pros:**
* 原生、零维护、正交不穿节点

**Cons:**
* 用户实测后明确要求换回曲线——「让连线有交点」；正交走廊观感非用户偏好

### 3. 内置 step（直角折线）

smoothstep 的锐角变体。

**Pros:**
* 原生、零维护

**Cons:**
* 同 smoothstep——正交观感被用户否决；锐角视觉更生硬

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer（AI Software Architect 框架）
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 本决策是**简化**而非增复杂度：从自定义 FlowEdge（数百行 DOM 测量/观察器）+ smoothstep 方案收敛到 ReactFlow 一行原生配置。符合"用现有工具解决"的务实原则。

**Decision Challenge**:

**Proposed Decision**: "连线采用 ReactFlow 原生默认 bezier 拓扑曲线，接受曲线自然交叉"

**Necessity Assessment**: 10/10
- **Current need**: 用户 2026-08-05 实测后明确拍板（「换回原来的 ReactFlow 的原生的拓扑曲线连线，让连线有交点」）——直接的用户需求，非推测
- **Future need**: 无未来性需求，纯当前视觉偏好
- **Cost of waiting**: 延迟 = 用户每次打开画布都看到不想要的连线风格
- **Evidence of need**: 用户原话 + 两轮方案（FlowEdge/smoothstep）均被实测否定的历史

**Complexity Assessment**: 0/10
- **Added complexity**: 零新增——删除自定义 edge 代码（-108 行 FlowEdge + 测试），edges 一行配置
- **Maintenance burden**: 降低——原生 edge 零维护
- **Learning curve**: 无——ReactFlow 默认行为
- **Dependencies introduced**: 无新依赖

**Alternative Analysis**: 三个备选（FlowEdge 自定义 / smoothstep / step）全部经用户实测或架构评审否决，且均为"更复杂或同复杂度"方案——不存在被忽略的更简单选项。"do nothing"（保留 smoothstep）已被用户明示否决。

**Simpler Alternative Proposal**: 不适用——当前方案已是最简（一行原生配置），无更简可行路径。

**Recommendation**: ✅ Approve decision

**Justification**: 用户直接需求（Necessity 10/10）驱动；实现为单行配置 + 删除 108 行自定义代码（Complexity 0/10）；是三轮方案收敛后的最简终点。

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 0/10
- **Ratio**: 0.0 *(Target: <1.5)*

**Overall Assessment**: 恰当工程——用户需求驱动的简化决策，无过度设计痕迹。

## Validation

**Acceptance Criteria:**
- [x] edges 显式 `type: "default"`（原生 bezier），非省略（防注释矛盾回归）
- [x] 主干 wire 退化近似垂直直线（两端同 x：path x=1.3~2 恒定）——脊柱不退化
- [x] 分支 wire C 曲线 S 形扇出，多分支自然交叉
- [x] 端点与 handle 中心对齐（path 起点 vs DOM handle relX 一致）
- [x] 布局位置零变化（store 坐标断言：col0 x=0 脊柱、分支源右缘=列右缘）
- [x] 507/507 vitest + tsc 零错误

**Testing Approach:**
* 单元测试：布局算法测试零改动（连线类型不在纯函数域）
* 实测（victauri DOM 测量）：edge class 断言 + 路径曲线解析 + 节点坐标断言

## References

* [ADR-063 CodeFlowCanvas 布局质量与视觉体系优化](./ADR-063-codeflow-layout-visual-polish.md) — FlowEdge 走廊路由决策（本 ADR 终结其维护）
* [ADR-064 CodeFlowCanvas 布局脊柱修正](./ADR-064-codeflow-layout-spine-realignment.md) — 布局脊柱/对齐/handle 钉位（保留）；其决策 2（smoothstep）被本 ADR 取代
