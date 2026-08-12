# Spec：Poiesis 阶段 v2 — 收口与源语义清理 + 源节点形象化

Status: ready-for-agent
Type: spec
Source: 阶段 v1 交接文档（2026-08-12 handoff）+ 设计稿 docs/design/poiesis-visual-app-editor-design.md（§15.2 源节点形象 / §15.3 端口语义 / §17.6 形象后置）+ 2026-08-12 brainstorming 会话（五轮拍板）

---

## Problem Statement

阶段 v1（M1 睡/醒地基 + M2 睡态工具 + M3 醒态绑定）已收官，但留有收口缺口与两处半成品：

1. **行为卡保存往返未做实机**（handoff 焦点 #2）：行为卡提交在 store 层已验证（自动 fn 三件套 + emitter 结构），但"保存 → 重开 → 行为仍在"完整往返未验证——fn 元素 + code-block 体 + flow 线 + 事件指向全链在磁盘往返中是否无损未知
2. **拖拽交互未人工实测**（handoff 焦点 #3）：手柄拖线 / 组件库拖拽的真实指针交互（dnd-kit/ReactFlow 激活）依赖真实指针捕获，合成事件不可靠——v1 已记录为人工验证项
3. **源语义半成品**（M3 引入，超出设计稿语义）：点名路径的"画布元素"来源列出所有 view 元素，pick 时把元素 label 文本当表达式落盘（`{报名}` 这类非法表达式）；拖线路径显示元素当源会写 `bindings.label`（div 无 bind:label 语法，未定义行为）。设计稿 §15.3 端口语义 = 输入框有"值"出口（源）、标题有"文本"入口（目标）——**显示元素只作目标**
4. **源节点形象未落地**（设计稿 §15.2 后置）：v1 用 emoji 简化卡片，账本/小灯/算盘形象待做

## Solution

**一个 v2 里程碑、四个独立 Task**（每 Task 独立提交、独立验收），语法零变更（全部落盘现有 .omni 语法）：

### Task A1：行为卡保存往返实机验证（收口，无 UI 新增）

行为卡本质 = 元素事件绑定的口语填写入口（当[点击]→[加]→[计数器] → 自动生成 fn + 事件指向）；重开后的管理走现有事件列表（EventsSection），**不做口语回显清单**（已拍板）。

实机剧本（start-poiesis + Victauri 驱动 demo2）：
1. 行为卡提交：当[点击]→[加]→[计数器] → 自动 fn `onPressAuto` + code-block 体 + flow 线 + `@onPress={onPressAuto}` 落盘
2. 保存 → 重开项目
3. 验证全链往返：fn 元素 / code-block 体（source 原文）/ flow 线 / 事件指向全部回来；WASM 编译通过（无语法错误）；EventsSection 显示 `onPress → onPressAuto`；实机点击按钮行为生效
4. 暴露的往返缺陷（parser/emitter/codeFlowGraphs 侧）按 bug 修复并提交

验收口径：剧本全过 + 缺陷修复提交；无缺陷则记录"往返闭环确认"。

### Task A2：拖拽交互人工实测（收口）

真实指针交互收口（v1 合成事件局限的记录项）：
- 手柄拖线：源（值型控件/源节点/fn 出口）→ 目标（元素端口）真实指针拖拽绑定
- 组件库（底栏 [组件库]）拖元素入画布
- 源节点卡片拖动排布 / 点击改值浮层

验收：真实指针交互确认可用；缺陷修复或如实记入边界。

### Task B1：源语义清理（核心语义修正）

设计稿边界（§15.3 / §17.7）：**源 = 值型控件（值出口）/ 源节点 / fn 出口；显示元素 = 目标（文本/属性入口）**。显示元素当源明确不做（非后置待定，是语义边界）。

1. **点名清单收窄**（`poiesis/src/components/design/binding-sources.ts`）："画布元素"分组只列**值型控件**（input/textarea/select/slider，有值出口者）；显示元素不再出现为来源
2. **拖线源端校验**（`poiesis/src/store/slices/auto-link.ts` connectElementLink）：源元素必须为值型控件，否则拒绝（console.warn + 不落盘）——消除"显示元素当源写 bindings.label"未定义行为
3. **点名选值型控件 = 同一条落盘路径**：点名 pick 值型控件 → 复用 connectElementLink（建 auto signal + 源端 bind:value + 目标端绑定 + wire）——点名与拖线语义统一（均走 auto-link 现有机制）
4. 显示元素 label 手柄保留为**目标**（接受拖入，现状不变）

测试：binding-sources 收窄测试；connectElementLink 拒绝非值型源；点名→连线落盘形状（auto signal 命名/同槽复用/回收语义不变）。

### Task B2：源节点五类形象化

设计稿 §15.2 形象映射（已拍板，含补定）：**signal→小灯、computed→算盘、store-signal/store-getter→账本（getter 带"算"页签）、inline-signal→小灯+本地角标**。变量/数据页形象（便签/工作台）因语言无载体（无 var 实体、无数据页概念）明确不涉及。

实现：`poiesis/src/components/canvas/SourceNodeCard.tsx` KIND_ICONS 替换为 SVG 图标体系 + 类型配色 + 卡片微深化（类型色条/分组分隔）；模块 CSS 调整。

交互不回归：拖拽、改值浮层、自动连线分组、数据节点开关。

测试：卡片渲染（图标/配色/值）+ 交互回归。

## Implementation Decisions

- **语法零变更**：A1/B1 全部落盘现有 .omni 语法（fn/事件指向/auto signal）；不发明语言概念
- **行为卡不做口语回显**：管理走 EventsSection（用户拍板——行为卡 = 事件绑定填写入口）
- **显示元素只作目标**：B1 按设计稿 §15.3 端口语义校准（源三类不变），"转发"发明不引入（已绑 {sig} 的显示元素喂给别处 = 直接点名选该信号）
- **A1/A2 是验证收口**：无新功能，暴露缺陷按 bug 修；A2 人工项不进 CI
- **提交纪律**：并行会话活跃（batch-4 等）——精确文件 add，提交前查 `git status --porcelain`；v1 工件（spec/acceptance/plans，git 未跟踪）随 v2 开工收口提交一次

## Testing Decisions

- **主缝 ① store 切片**（B1 语义核心）：connectElementLink 拒绝非值型源；点名→连线统一路径落盘形状。先例：`src/store/__tests__/auto-link.test.ts`、`behavior.test.ts`
- **主缝 ② 来源组装**（B1 收窄）：buildBindingSources 画布元素分组只含值型控件。先例：`src/components/design/__tests__/`
- **缝 ③ 组件测试**（B2）：SourceNodeCard 图标/配色/值渲染 + 交互回归。先例：`src/components/canvas/__tests__/SourceNodeCard.test.tsx`
- **缝 ④ 实机验证**（A1/A2，不进 CI）：Victauri MCP 驱动剧本（行为卡往返全链 / 真实指针拖拽）

## Out of Scope

- 行为卡口语回显清单、动词词库扩充（G3）
- 显示元素当源（设计稿语义边界，非后置）
- 变量（便签）/ 数据页（工作台）源节点形象（无语言载体）
- 六接口形状可视化、拖线交互规格、[样子] 衣柜、局部唤醒细化（设计稿 §17.10 其余待定 #16 项）
- 骨架视图、果园（设计稿待深化项，另行）

## Further Notes

- 权威契约：设计稿 §15.2（形象）/ §15.3（端口语义：输入框值出口、标题文本入口）/ §17.6（形象后置）/ §17.7（线源限三类）
- 语言事实（实证，勿重调研）：`{ref.value}` 快照非响应式；bind:value 只绑 signal（validate_component.rs E0005）；@for 迭代变量非 signal（item 字段仅表达式可读）；@ref 写侧直写（ADR-070）
- 阶段 v1 工件索引：spec `.scratch/poiesis-stage-v1/spec.md`、验收 `acceptance-m1/m2/m3.md`、遗留 issue `issues/01-storeReferences-direction.md`（已修 66fe11b7）
