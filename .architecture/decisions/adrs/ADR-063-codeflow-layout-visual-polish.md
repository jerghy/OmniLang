# ADR-063: CodeFlowCanvas 布局质量与视觉体系优化（正交 wire 路由 / node-base 统一视觉 / 数据丢失防线）

## Status

Implemented

## Context

CodeFlowCanvas 成熟化（ADR-057~061 时代，502 tests）交付后，用户真机实测反馈三类问题，经多轮修复（含 agent-vision 视觉评审辅助）：

1. **布局排布质量**：① 预览组件（design-preview）位置固定/可拖，布局后盖住左上角节点；② 兄弟分支 wire 交叉（分支 lane 按 wire 数组顺序分配，乱序时 br1 目标占左列）；③ wire 起点被同列下方更宽节点覆盖（分支源左对齐 + 估算尺寸严重偏大——旧 codecondition 估 200 宽 vs 真实 117，右对齐按估算算时"估算即列内最宽"→ 无需右移，真实渲染时右缘仍被覆盖）
2. **连线视觉问题**：① wire 端点与 handle 中心存在可见间隙（硬编码中心常数 4px 按 8px handle 算，实际 codeflow-handle 10px+2px 边框=12px）；② 正交路由实施过程中发现 ReactFlow handleBounds 缓存失真
3. **节点美观**：生硬矩形、无统一圆角/阴影/选中态、handle 不清晰、`flow-node-selected` 类无任何 CSS（选中无视觉反馈）
4. **数据丢失事件**：用户实测画布预览空——store 的 design 元素丢失（project.json nodes 空、保存覆盖 .omni view 块——重启用 .omni 解析恢复）

约束：零新依赖；`@pid` round-trip 契约零改动；布局语义（主干垂直/分支右展/子图堆叠）保持。

## Decision Drivers

* **布局必须可读**：一键整理的目标是"读图=读代码结构"——wire 交叉、节点挡线、预览遮挡直接破坏可读性
* **wire 端点必须精确贴 handle**：视觉上"线没连到节点"即失败——端点与 handle 中心的任何可见间隙都是缺陷（用户实测）
* **绕开 ReactFlow handleBounds 失真**：handleBounds 在 handle mount 时测量一次，CSS 变化（position:relative 归位）不触发重测——实测偏移 187.67 vs DOM 实际 100；且 v12 EdgeProps 不传 sourceHandle 给自定义 edge
* **数据安全优先**：setElementsAndWires 全量替换在 store 异常状态（design 缺失）会覆盖 view 树——宁可不动不可清空
* **视觉统一**：节点体系对标 ComfyUI/Notion——统一圆角/阴影/选中态/handle，tag 色系保留

## Decision

### 1. 布局算法质量修复（effce8fb）

- **preview 归位**：`previewPos` 改为 state，布局按钮把 design-preview 节点归位到布局包围盒右下角外（maxX/maxY + 40）
- **分支 lane 按分支 id 排序**（`branchPortOrder`：br0 < br1 < br2 数字后缀序）——lane 分配与 wire 数组序解耦，消除兄弟分支交叉
- **分支源右对齐到列右缘**（`hasBranchOut` 判定 + `x = 列左缘 + 列宽 - H_GAP - 自身宽`）——wire 起点（节点右侧 handle）不被同列下方更宽节点覆盖
- **BASE_SIZES 按实测校准**（fn 150/let 200/condition 140/while 130/for-in 140 等，旧全 200 宽严重偏大）——估算贴近真实，右对齐按估算有效（误差 ≤6px）
- **同桶行高预留**（`maxBucketPerDepth` + `rowHeight = maxH + (n-1)×(maxH+INLINE_GAP) + V_GAP`）+ **compHeight 最深行含预留**——无重叠不变式严格成立（同桶 idx≥1 节点底不穿入下一行/下一子图）

### 2. 正交 wire 路由（FlowEdge，6fc5ed61 → 301f7fbe）

- **FlowEdge 自定义 edge**（edgeTypes `{ flow }` + edges `type: "flow"`）：分支 wire（右侧→顶部）走走廊——source 右侧 CORRIDOR(24px) 垂直段（列间空隙内）→ 目标行水平进；flow-out→flow-in 同列保持直线；`computeFlowEdgePath` 纯函数可单测
- **位置来源：具体 handle 元素 DOM 测量**（绕开 handleBounds 失真）——`[data-nodeid][data-handleid]` 查询（fromPort 经 edges `data` 注入——v12 不传 sourceHandle 给自定义 edge）；`useStore transform` 换算画布坐标；handle 未挂载 rAF 重试；ResizeObserver + MutationObserver 跟随节点变化
- **handle 中心动态计算**：`rect 宽高 ÷ 2 ÷ zoom`——不硬编码半径（10px+2px 边框=12px，旧常数 4px 致端点偏 2px 画布，0.573 缩放下视觉可见）

### 3. node-base 统一视觉体系（node-base.css，6fc5ed61）

- 共享类 `.codeflow-node`（position:relative 锚点 + 圆角 10px + 双层柔和阴影 + 悬停加深 + 过渡）/ `.codeflow-handle`（10px 圆点 + 白边 + hover 放大 1.5× 变主色）/ `.codeflow-header`
- `.react-flow__node` 前缀提特异性 (0,2,0)——防各节点 module.css 同特异性覆盖（import 顺序不保证）
- 选中态：`.flow-node-selected`（此前无任何 CSS）→ 主色 outline 高亮
- 6 节点组件接线（Syntax/Loop/Condition/Return/Fn/CodeBlock），tag 色系保留

### 4. 数据丢失防线（2d14f9ff）

布局按钮 `handleAutoLayout` 早退条件追加：`allElements` 无非 codeblock 元素（design 缺失）时直接返回——setElementsAndWires 全量替换在异常状态会覆盖 view 树（Task 12 CRITICAL 的变体，用户实测画布空）

**Architectural Components Affected:**
* `poiesis/src/services/codegen/flow-layout.ts`——lane 排序/右对齐/行高预留/compHeight/BASE_SIZES
* `poiesis/src/components/canvas/code/FlowEdge.tsx`（新建）——正交路由 + handle DOM 测量
* `poiesis/src/components/canvas/code/FlowCanvas.tsx`——previewPos/布局防线/edge data 注入/ReactFlow key
* `poiesis/src/components/canvas/code/nodes/node-base.css`（新建）+ 6 节点组件——视觉体系
* `poiesis/src/components/canvas/code/__tests__/FlowEdge.test.ts`（新建）——纯函数 4 单测

**Interface Changes:**
* `computeLayeredLayout` 行为变化：分支源右对齐（x > 0）、lane 按 id 序（既有断言更新 2 处）
* edges 数据新增 `data: { fromPort, toPort }`（FlowEdge 消费）
* `handleAutoLayout` 新增 design 守卫（异常状态静默不动作）

## Consequences

### Positive

* **布局可读**：preview 零遮挡（实测归位包围盒外）、兄弟 wire 零交叉（lane id 序）、wire 起点零覆盖（分支源右缘 ≥ 列内最大右缘）、无重叠不变式严格成立（双 fix 轮：同桶行高 + compHeight）
* **wire 精确贴合**：端点与 handle 中心距离 ≤1.5px（像素级实测 10 wire）；视觉模型（窗口精确截图）确认"所有连线精确贴住端口圆点"
* **穿节点根除**：正交走廊垂直段在列间空隙（画布坐标精确采样 0 命中——此前 bezier 圆弧 6 点穿过）
* **视觉统一**：圆角/阴影/选中态/handle 体系化，`flow-node-selected` 有视觉反馈
* **数据防线**：异常状态布局按钮静默不动作，view 树不被覆盖

### Negative

* **正交路径牺牲 bezier 圆滑**：走廊直角在视觉上比曲线"硬"（功能优先的取舍）
* **FlowEdge 依赖 DOM 测量**：handle 未挂载时 rAF 重试（短暂闪烁窗口）；ResizeObserver/MutationObserver 有微小开销（每 edge 2 观察器）
* **ReactFlow key 重挂载**：进入 codeflow 时 ReactFlow 全量重挂载（fitView 重放）——handleBounds 缓存问题的替代方案
* **数据丢失事件根因未完全定位**：design 元素丢失的确切触发路径（疑似布局/保存链在 store 异常时序）未复现——防线是"宁可不动"而非根治

### Neutral

* BASE_SIZES 估算与真实尺寸仍有 ≤6px 误差（measured 就绪的二次布局精确）
* 分支源右对齐使主干列内分支源右移（视觉上 condition 比 let 偏右）——右缘对齐的取舍

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——poiesis 前端（flow-layout 纯函数 + FlowEdge 组件 + 节点 CSS + FlowCanvas 接线），编译器零改动；`@pid` 契约零改动

**Affected Components**: 见上表（~12 文件）

**User Impact**: 布局一键整理后结构清晰（无交叉/无遮挡/无穿节点）；连线精确贴端口；节点视觉专业

**Risk Mitigation**: 每修复轮 TDD + 像素级几何验证（wire 端点 vs handle 中心距离）+ 窗口精确截图视觉验证（agent-vision）+ 508/508 全量

### Reversibility

**Reversibility Level**: High——代码级回退；FlowEdge 可回退 xyflow 默认 bezier；node-base.css 移除即还原

**Migration Paths**: 无迁移（纯前端增强）；`@pid` 持久化格式零改动

### Sequencing & Timing

**Prerequisites**: 成熟化主线（T1-12）交付后用户反馈驱动

**Readiness Assessment**: Ready——已实现并合并 main

### Social Cost

**Learning Curve**: Low——交互不变（布局按钮/拖拽/连线）；视觉体系是既有 tag 色系的统一

**Documentation Needs**: 本 ADR + FlowEdge 注释（handleBounds 失真根因链）+ node-base.css 注释（特异性/锚点理由）

## Alternatives Considered

### handleBounds 缓存修复（拒绝）

让 ReactFlow 的 handleBounds 正确（重挂载/强制重测）。
**拒绝原因**：handleBounds 在 mount 时测量、CSS 时序下失真（187.67 vs DOM 100），且实测 ReactFlow key 重挂载后仍失真——机制不可控；DOM 测量绕开是确定性方案。

### 纯节点 rect 推断 handle 位置（尝试后否决）

FlowEdge 用节点 rect + fromPort 推断 handle（分支=右缘中部）——condition 多分支 handle 分散在各分支行（dy 29/43/57/71），wire 起点全堆中部 → 视觉悬空断线（用户实测）。
**终选**：查询具体 handle 元素（`[data-nodeid][data-handleid]`）测量精确位置。

### DOM 测量版先失败于 sourceHandle 缺失（教训）

首版 DOM 测量用 props.sourceHandle 查询——v12 EdgeProps 不传 sourceHandle 给自定义 edge → 查询恒失败 → edge 全空 + rAF 无限重试。
**修正**：edges data 注入 fromPort。

### 批量注入 className 脚本的子串陷阱（教训）

`'<Handle' in line` 误匹配 `<HandlesBoundary`（子串）——className 注入成 HandlesBoundary 文本子节点渲染进节点（视觉模型实测：condition 分支行显示 `className="codeflow-handle"`）。
**修正**：精确标签匹配（`prev == "<Handle"`）清除误注入。

## Validation

- 508/508 vitest（+4 FlowEdge 单测）；tsc 零错误
- 像素级几何验证：10 wire 端点与 handle 中心视口距离全部 ≤1.5px（修复前 2-4px）
- 画布坐标精确采样：wire 穿节点 0 命中（修复前 while→body 6 点穿过 call）
- 窗口精确截图视觉验证（agent-vision）："所有连线精确贴住端口圆点，无可见间隙"、"判断节点各分支线准确连接对应分支行端口"
- 数据恢复验证：重启后设计画布 5 元素恢复、codeflow 13 节点、保存后 .omni view 保留、persistedPositions 13 条重建
- 布局防线验证：布局后仍 13 节点 + preview 在

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 四项决策均针对用户实测缺陷（布局可读性/wire 贴合/视觉统一/数据安全），修复沿既有模式（纯函数/组件/CSS）展开；FlowEdge 的 DOM 测量是绕开 ReactFlow 机制失真的必要路径（三次迭代收敛），非过度设计。

**Proposed Decision**: "布局算法质量修复 + 正交 wire 路由（handle DOM 测量）+ node-base 视觉体系 + 布局数据防线"

**Necessity Assessment**: 10/10
- **Current need**: 用户逐项实测反馈（交叉/遮挡/悬空/画布空）；数据丢失为实际事故
- **Future need**: 布局是画布编辑的基础体验；wire 贴合是节点编辑器基本契约
- **Cost of waiting**: 每次使用都看到问题；数据丢失会再发生
- **Evidence of need**: 用户原话 + 像素级实测 + 视觉模型双重确认

**Complexity Assessment**: 4/10
- **Added complexity**: FlowEdge DOM 测量（rAF 重试 + 双观察器）是最大新增；其余为既有模式延伸
- **Maintenance burden**: 中——FlowEdge 的测量逻辑需注释维护（handleBounds 根因链已记录）；node-base 特异性规则一句话可守
- **Learning curve**: 低
- **Dependencies introduced**: 零新依赖

**Alternative Analysis**: handleBounds 缓存修复（不可控）被拒；节点 rect 推断（不精确）被否决；DOM 测量是三者中唯一确定性方案。

**Simpler Alternative Proposal**: 正交路由可只做走廊段（不做 handle 测量，接受 2-4px 间隙）——但用户实测"线没连到节点"即失败，间隙不可接受。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 4/10
- **Ratio**: 0.4 *(Target: <1.5)*

**Overall Assessment**: 适当的工程——四项决策全部用户缺陷驱动，修复路径沿既有模式，FlowEdge 的 DOM 测量是绕开机制失真的最小确定方案；数据防线是低成本高价值的安全网。

## References

- 上游决策：ADR-057（语法节点）、ADR-060（布局持久化 persistedPositions）、ADR-061（中链 condition/redo）
- Commits：effce8fb（布局质量）→ 6fc5ed61（美观+路由）→ de183e65（relative 锚点）→ 301f7fbe（handle 中心动态）→ 2d14f9ff（数据防线）
- 任务台账：`.superpowers/sdd/2026-08-05-codeflow-maturity/progress.md`（美观优化轮 + 连线修复轮 + 数据丢失事件记录）
- 遗留项：数据丢失事件确切触发路径未复现（防线为"宁可不动"）；knownStores 仅 openTabs 推导（既有）
