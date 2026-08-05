# ADR-064: CodeFlowCanvas 布局脊柱修正（主干左缘对齐 / smoothstep 正交连线 / 守卫移除）

## Status

Implemented

## Context

ADR-063（5daad755）后，工作区出现一组未提交实验改动：删除了 FlowEdge（DOM 测量版正交走廊路由）、edges 不指定 type（注释声称 smoothstep 但未配置 → 实际渲染 **bezier 弧线**）、布局算法改为**列内全部右缘对齐**、flow-in/out handle 钉节点右缘内侧、加 view 设计树守卫。用户实测：「整理布局后感觉更乱」，期望「有代码逻辑、逻辑架构感、节点不遮挡」。

实测复现（victauri DOM 测量）确认三要素：

1. **主干无脊柱**：列内统一右缘对齐 → 主干（col0）节点左缘参差（x=0/107/111/129/158…）——读不出「一条垂直主干 + 分支右展」的代码结构感
2. **连线是 bezier 弧线**：edges 无 type（默认 bezier），弧线扫过节点、互相交叉——注释与实现矛盾（声称 smoothstep 但无 `defaultEdgeOptions`/type）
3. **布局按钮被守卫静默废掉**：`hasDesignElements` 早退（2d14f9ff 数据防线）在「store 无 view 元素」状态触发——而代码流视图（如从架构页进入）无 view 元素是**合法状态** → 按钮点击无任何反应（用户实测）

另外：同桶 inline 堆叠节点（两个子图被跨界 wire 合并成一分量时）主干 wire 沿邻节点左缘 ~1.3px 擦过——亚像素级，属可接受残留。

## Decision Drivers

* **读图 = 读代码结构**：主干必须是一条视觉脊柱（左缘对齐如代码缩进 0 列），分支 lane 右展如缩进层级
* **连线必须正交且不穿节点**：bezier 弧线扫过节点即失败；smoothstep 原生折线 + handle 钉左缘 → 主干 wire 垂直直线
* **数据安全与可用性平衡**：写回 map 基于 allElements 全量（非 codeblock 元素原样保留）本身不丢 view 树——「view 缺失时禁用布局」的守卫是过度防御，牺牲了合法状态下的可用性
* **零新依赖**：用 ReactFlow 内置 smoothstep 替代自定义 FlowEdge（DOM 测量开销大，删除后不再维护）

## Decision

### 1. 主干左缘对齐 + 分支源右缘对齐（flow-layout.ts）

- 列内：**无分支口的节点左缘对齐**（x = 列左缘）——主干/线性链左缘成垂直脊柱
- **有分支口的节点**（condition/loop，br0/br1/body 出口）**右缘对齐到列右缘**（x = 列左缘 + 列内最大宽 − 自身宽）——分支 wire 起点（节点右侧分支 handle）落在列右缘，smoothstep 垂直段在列间空隙，不被同列下方更宽节点覆盖（ADR-063 决策 1 的「分支源右对齐」保留；「列内统一右缘对齐」撤销）
- 主干 wire 垂直改由 **flow-in/out handle 钉节点左缘内侧**（node-base.css `left:0; translate(-50%,-50%)`）保证——handle 列 = 列左缘 + 7.5，与节点宽度无关
- 历史注记：ADR-063 后实验改「统一右缘对齐」修主干斜线，但牺牲脊柱感；本决策以「左缘对齐 + handle 钉左缘」同时达成脊柱感与主干 wire 垂直

### 2. 连线：ReactFlow 原生默认 bezier 拓扑曲线（FlowCanvas.tsx edges `type: "default"`）

- FlowEdge 组件删除（DOM 测量开销大、维护复杂）；edges **显式 `type: "default"`（原生 bezier 拓扑曲线）**——用户 2026-08-05 实测拍板：放弃 ADR-064 初版的 smoothstep 正交折线，换回原生曲线、接受曲线自然交叉（「让连线有交点」）；显式声明杜绝「注释与实现矛盾 → 实际 bezier 却无人知晓」回归（2026-08-05 曾发生）
- 实测：主干 wire 两端同 x（flow-in/out 钉左缘 + 主干左缘对齐）→ bezier 退化近似垂直直线（脊柱保持）；分支 wire（br0/br1 右缘 → 目标左缘）S 形扫过列间空隙，多分支自然交叉；端点与 handle 中心对齐（relX 1.3 vs path x 1.3）；节点间零矩形重叠
- 初版（本 ADR 提交时）曾选 smoothstep 正交折线——被用户实测要求推翻，记录以留痕

### 3. 移除 view 设计树守卫（FlowCanvas.tsx handleAutoLayout）

- 守卫删除；数据安全由 `allElements.map` 全量写回保证（非 codeblock 原样保留——已由 FlowCanvasLayout.test.tsx「布局不丢 view 元素」测试覆盖）
- 保留：空画布早退（codeElements.length === 0）+ 按钮禁用
- 回归测试新增：纯 codeblock 状态布局仍生效

## Consequences

- **正向**：主干脊柱可读（col0 x=0 全部节点左缘成线）；连线正交无弧线扫节点；纯代码状态布局按钮可用；507/507 vitest + tsc 零错误
- **残留**：同桶 inline 堆叠（多子图被跨界 wire 合并为单一连通分量）时，主干 wire 沿邻节点左缘 ~1.3px 擦过——亚像素（0.664 缩放下 <1px 屏幕），任何 x 偏移方案都会让 wire 穿入邻节点体内（更糟），接受并记录
- **撤销项**：ADR-063「列内统一右缘对齐」；FlowEdge 正交走廊路由（DOM 测量版）不再维护
- **实验留痕**：工作区未提交实验（右缘对齐/无 type edge/守卫）被本 ADR 部分采纳（smoothstep、handle 钉边缘）、部分推翻（右缘对齐、守卫）

## Supersedes

- ADR-063 决策 1 的「列内统一右缘对齐」条款
- 工作区未提交实验中的「view 设计树守卫」
- 本 ADR 初版「edges `type: "smoothstep"`」（用户 2026-08-05 拍板换回原生 bezier，见决策 2）

## 验证

- 用户主流程实测：拖乱 n5/n6 → 点「整理布局」→ n5 回 col1 (370.7)、n6 回 col2 (741.3)、兄弟分支同层 y=1166.1；col0 全节点 x=0；分支源右缘 ≈ 列右缘 310（±1 尺寸实测差）
- 几何验证：119 个 wire 线段，零「穿节点体内」命中；42 处命中全部为 inline 邻节点左缘擦碰（亚像素，见残留）
- 507/507 vitest、tsc 零错误
