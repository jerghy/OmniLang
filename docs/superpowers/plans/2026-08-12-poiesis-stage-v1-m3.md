# Poiesis 阶段 v1 M3（醒态绑定）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 醒态绑定三件套——行为卡（当[点击]→[把][计数器][加1]，自动生成 fn）、五类槽点名绑定（文本/输入/列表/条件/属性，落盘现有语法）、手柄拖线（元素接口手柄 ReactFlow 拖线即绑定）+ 来源印记（已绑徽章 + 亮线）。语法零变更。

**Architecture:** 绑定全部走现有语法落盘（`@onClick` 语义事件 + 自动生成 fn 元素进 store → 现有 emitter 落盘；`bind:value`/`{expr}`/`@for`/`@if`/属性 绑定走现有 setElementBinding/文本插值/条件引用机制）；元素间连线 = 自动中转 signal（`<源>_<槽位>` 命名、同槽复用、数据区"自动连线"分组、断线回收）；手柄 = 现有元素端口（M1 T5 ElementPortLayer 视觉点）升级为 ReactFlow 可拖手柄（醒态下端口接入 ReactFlow 坐标系）；来源印记 = ContentTab 徽章深化（"来自 X"）+ 悬停/选中亮线。

**Tech Stack:** 同 M1/M2（Tauri v2 + React 19 + Zustand 5 + ReactFlow @xyflow/react ^12 + Vitest 4）。

**契约来源:** `.scratch/poiesis-stage-v1/spec.md` M3 段；设计稿 §17（拍板 #47-53）；语言事实附录（`{ref.value}` 快照非响应式；bind:value 只绑 signal；事件经 e.value 读触发元素值）。

**验收口径:** 每个 Task 独立可提交、测试全绿；Task 8 为实机验证剧本（Victauri），M3 里程碑验收以 Task 8 清单为准（也是阶段 v1 收官）。

**关键状态规则（本计划锁定）:**
```
行为卡 = BehaviorTab 深化：当[事件] → [动作][参数] 表单；动词集最小 ~10（点击/提交/删除/编辑/取消/加/减/跳转/切换/关闭）
行为卡落盘：动作 → 自动生成 fn 元素（store elements 加 fn 节点，名字 `on<Event>Auto` 或语义名）+ 元素 @onClick 指向 fn
   → emitter 编译 fn → .omni 落盘 `fn onXxx() { ... }` + `@onClick={onXxx}`（语法零变更，往返闭环）
五类槽点名：点槽 → 来源清单（源节点 / fn 出口 / 醒着的元素）→ 落盘：
   文本槽 → props.label = {expr}；输入槽 → bind:value={sig}；列表槽 → @for 引用；条件槽 → @if 条件；属性槽 → 属性值表达式
自动中转 signal（元素间连线）：源元素值槽 → 自动 signal（名 `<源元素id>_<槽语义>`，无 @ref 时 `<tag>_<序号>`）
   同槽复用：同一 (源, 槽) 的 N 条连线共用一个 signal；数据区独立分组"自动连线"（与用户 signal 分排）
   回收：断开且无其他绑定引用 → 删除该 signal（store 生命周期）
拖线手柄：醒态 + 元素选中 → 端口变 ReactFlow Handle（源/目标双向），拖线 → 自动 signal + 目标槽绑定
来源印记：ContentTab 已绑槽位显示"来自 X"徽章（绑定名来源）；悬停/选中 → 相关线亮回（M3 末 Task）
```

---

## Task 1: 行为卡 UI（BehaviorTab 深化）

**Files:**
- Modify: `poiesis/src/components/design/BehaviorTab.tsx`（当[事件]→[动作][参数] 表单）
- Create: `poiesis/src/components/design/BehaviorCard.tsx` + `.module.css`（行为卡组件）
- Create: `poiesis/src/components/design/action-verbs.ts`（最小动词集 + 动作→代码生成映射）
- Test: `poiesis/src/components/design/__tests__/BehaviorCard.test.tsx`

- [ ] **Step 1: TDD 失败测试** — BehaviorCard 渲染当[点击]→[把][计数器][加1] 形式（事件下拉/动作下拉/参数槽）；动词集 action-verbs 导出 ACTIONS（最小 ~10：点击/提交/删除/编辑/取消/加/减/跳转/切换/关闭，各含 label/代码模板）
- [ ] **Step 2: 实现** — BehaviorTab 显示元素现有 @onClick 行为（读 element props 事件字段），未绑定时显示行为卡入口；行为卡 = 事件选择（点击/提交等现有语义事件）+ 动作选择 + 参数（动作相关：加/减 → 目标 signal + 步长；跳转 → 页面；切换 → 目标 signal；编辑/删除/取消/提交 → 参数可选）
- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 行为卡 UI——当[事件]→[动作][参数] 表单 + 最小动词集（M3 T1）`

---

## Task 2: 行为卡落盘（自动 fn 生成 + emitter 往返）

**Files:**
- Modify: `poiesis/src/services/codegen/design-emitter.ts` 或 `source-emitter.ts`（自动 fn 落盘支持）
- Modify: `poiesis/src/store/operations/` 或新建 `poiesis/src/store/operations/behavior.ts`（applyBehaviorCard 动作）
- Modify: `poiesis/src/components/design/BehaviorTab.tsx`（提交 → applyBehaviorCard）
- Test: `poiesis/src/store/__tests__/behavior.test.ts`（主缝② codegen 往返）

- [ ] **Step 1: 摸底** — 读 source-emitter.ts fn 编译路径（compileCodeFlowGraph → fn 定义）与 design-emitter 事件落盘（@onClick 写法）；确认自动 fn 的注入点（store elements 加 fn 节点 + 元素 props.onPress 指向）
- [ ] **Step 2: TDD** — applyBehaviorCard(elementId, {event, action, params}) → store：生成 fn 元素（`fn on<Event>Auto() { ... }`，body 由动作代码模板生成）+ 元素事件字段指向 fn 名；往返测试：apply → emitOmniSource → parse → 形状不变（fn 存在 + 事件引用）
- [ ] **Step 3: 实现** — 动作代码模板（action-verbs.ts 扩展：代码生成函数；加/减 → `sig = sig + 1` 类；跳转 → `navigate(...)`；切换 → `sig = !sig`）；fn 生成走现有 CodeblockElementData（tag:"fn"）——复用现有 fn 物化/落盘
- [ ] **Step 4: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 行为卡落盘——自动 fn 生成 + @onClick 指向（语法零变更，往返闭环）（M3 T2）`

---

## Task 3: 五类槽点名绑定（ContentTab 深化）

**Files:**
- Modify: `poiesis/src/components/design/ContentTab.tsx`（点槽 → 来源清单）
- Create: `poiesis/src/components/design/BindingPicker.tsx` + `.module.css`（来源清单浮层：源节点 / fn 出口 / 醒着的元素）
- Modify: `poiesis/src/components/design/ContentTab.tsx`（五类槽落盘：文本/输入/列表/条件/属性）
- Test: `poiesis/src/components/design/__tests__/BindingPicker.test.tsx`

- [ ] **Step 1: TDD** — BindingPicker 渲染来源清单（collectSourceNodes 数据 + fn 出口 + 醒着元素）；点选 → onPick(source) 回调
- [ ] **Step 2: 实现落盘** — 五类槽提交：
  - 文本槽（props.label）：`{expr}` 插值写入（现有文本插值机制——查 setProp 文本插值写法）
  - 输入槽（bind:value）：`setElementBinding(id, "value", sig)`（已有 action ✅）
  - 列表槽（@for）：block.collection 写入（ForBlockEditor 机制）
  - 条件槽（@if）：分支 condition 写入（updateBranchCondition 机制）
  - 属性槽：属性值表达式写入（setProp 表达式形态）
- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 五类槽点名绑定——点槽选来源落盘现有语法（M3 T3）`

---

## Task 4: 自动中转 signal 生命周期（store）

**Files:**
- Create: `poiesis/src/store/operations/auto-signal.ts`（纯逻辑：命名/同槽复用/回收）
- Modify: `poiesis/src/store/types.ts` + `index.ts`（动作接线：connectElementLink / disconnectElementLink）
- Test: `poiesis/src/store/__tests__/auto-signal.test.ts`（主缝① store 切片）

- [ ] **Step 1: TDD** — 测试矩阵：
  1. connectElementLink(源元素, 槽, 目标元素, 目标槽) → 自动 signal 创建（名 `<源id>_<槽>`），源端 bind:value + 目标端绑定
  2. 同槽复用：同一 (源,槽) 二次连接 → 不建新 signal（数量不变，目标端加引用）
  3. 回收：断开且无其他引用 → signal 删除；有引用 → 保留
  4. 自动连线分组：自动 signal 标记 `auto: true`（数据区分组依据——collectSourceNodes/数据区消费）
- [ ] **Step 2: 实现** — 自动 signal = 普通组件 signal（componentDefs 信号？或 store elements signal 节点？——**设计决策：自动 signal 存 store elements 的 signal 元素（nodeType:"signal"？查现有 signal 元素形态）或 componentDefs.signals 追加**——摸底后定，倾向 store elements signal 元素（与 fn 同层，emitter 落盘 `signal <name>: <type> = <default>`））；命名 `<源元素id>_<槽>`（无 @ref 时 `<tag>_<序号>` 由源元素 tag+出现序）；回收扫描引用（目标端 bindings/插值/条件）
- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 自动中转 signal 生命周期——命名/同槽复用/回收/自动连线分组（M3 T4）`

---

## Task 5: 手柄拖线（元素端口 → ReactFlow Handle）

**Files:**
- Modify: `poiesis/src/components/canvas/ElementPortLayer.tsx`（端口 → ReactFlow Handle，醒态 + 选中）
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（handle 集成——或新建独立 ReactFlow 实例？**设计决策：复用 FlowCanvas 的 ReactFlow（醒态逻辑层内）**——端口 handle 需在 ReactFlow 内 → ElementPortLayer 改为在 LogicLayer 内渲染 ReactFlow 节点式手柄层？——摸底 ReactFlow 是否支持 DOM 外 handle。倾向：新建轻量 ReactFlow 层（仅手柄节点，无 fn 图）叠加在端口位置，与 FlowCanvas 并存）
- Test: `poiesis/src/components/canvas/__tests__/PortHandleLayer.test.tsx`

- [ ] **Step 1: 摸底** — ReactFlow Handle 只能在 Node 内（onConnect 需节点上下文）——方案：醒态 + 选中元素的端口位置生成 ReactFlow 节点（不可见 Node + Handle），onConnect → connectElementLink
- [ ] **Step 2: TDD + 实现** — PortHandleLayer：数据 = 选中元素端口（复用 collectPortPositions 定位）；每个端口一个隐藏 Node（position = 端口坐标）+ Handle（source/target 双向）；onConnect → connectElementLink；连线渲染（自动 signal 绑定 → 画线）
- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 手柄拖线——醒态选中元素端口 ReactFlow 可拖 + 拖线即绑定（M3 T5）`

---

## Task 6: 来源印记 + 亮线

**Files:**
- Modify: `poiesis/src/components/design/ContentTab.tsx`（徽章"来自 X"深化——绑定名来源）
- Modify: `poiesis/src/components/canvas/ElementPortLayer.tsx` 或 PortHandleLayer（悬停/选中亮线）
- Test: `poiesis/src/components/design/__tests__/ContentTab.test.tsx`（徽章深化）

- [ ] **Step 1: TDD** — 已绑槽位徽章显示来源名（"来自 newSignal"）；M1 徽章（存在性）升级为名称展示
- [ ] **Step 2: 实现** — 亮线：悬停/选中已绑元素 → 相关自动 signal 连线高亮（PortHandleLayer 线条样式）；睡态收起（印记保留徽章）
- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 来源印记——已绑徽章显示来源名 + 悬停/选中亮线（M3 T6）`

---

## Task 7: 全量回归

- [ ] **Step 1** — `npx vitest run` 全绿（0 failed）；`npx tsc --noEmit` 0 错误
- [ ] **Step 2** — 新增文件均有测试；grep 无死代码残留
- [ ] **Step 3** — 提交（如有零散调整）或声明无改动

---

## Task 8: 实机验证剧本（Victauri，M3 里程碑验收 = 阶段 v1 收官）

> 前置：start-poiesis 技能启动 app + Victauri MCP。剧本参考 M1/M2 验收（`$CLAUDE_JOB_DIR/tmp/vt.sh` 或 MCP 工具）。

- [ ] **Step 1**: get_plugin_info 确认 com.omnilang.poiesis；打开 demo2；进 main 组件 tab；**画布可见性断言（getBoundingClientRect 在视口内——M2 教训）**
- [ ] **Step 2**: 行为卡——选中按钮 → [行为] 标签 → 行为卡表单（当[点击]→[把][计数器][加1]）→ 提交 → 画布元素 @onClick 生效（点击按钮 → 计数器信号变化）；保存 → 重开 → 行为仍在（往返闭环）
- [ ] **Step 3**: 五类槽点名——选中元素 → [内容] 标签 → 点文本槽 → 来源清单（newSignal 等）→ 选中 → 画布文本变；输入槽 bind:value → 打字实时响应
- [ ] **Step 4**: 自动 signal——元素间连线（输入框 → 标题文本）：拖线/点名 → 自动 signal 出现（数据区"自动连线"分组）；同槽复用（同一输入喂两个目标）→ 不重复建；断开 → 无引用时回收
- [ ] **Step 5**: 拖线手柄——醒态选中元素 → 端口可拖（真实指针或代码级验证）；拖线即绑定
- [ ] **Step 6**: 来源印记——已绑槽位显示"来自 X"；悬停/选中 → 线亮回
- [ ] **Step 7**: 不倒退——睡态画布/语境面板/呼吸点/源节点改值/保存往返（M1/M2 验收项）
- [ ] **Step 8**: 验收记录写入 `.scratch/poiesis-stage-v1/acceptance-m3.md` + 阶段 v1 收官总结（spec Status 更新）

---

## Self-Review 对照

**Spec 覆盖（M3 段）**：行为卡（当[事件]→[动作][参数] + 最小动词集）→ T1+T2；五类槽点名绑定 → T3；手柄拖线（双向可连 + 拖线即绑定 + 两路并存）→ T5；自动中转 signal（命名/同槽复用/分组/回收/落盘形状）→ T4；来源印记 + 亮线 → T6；语法零变更（落盘全走现有语法）→ T2-T4 设计锁定；保存往返 → T8 Step 2/7。

**已知边界（如实声明）**：
- 自动 fn 命名与用户 fn 冲突策略（`on<Event>Auto` 前缀防冲突；用户改名后行为卡跟随——面板改 fn 跟着改）
- 手柄方案依赖 ReactFlow 节点式 Handle（Node 内）——PortHandleLayer 用隐藏 Node 承载；若 ReactFlow 不支持（摸底后），降级为"点名绑定"优先 + 拖线后置
- 亮线范围：自动 signal 连线（用户 fn 图连线不动）
- 元素当线源（显示元素 → 其他元素的源）后置（spec Out of Scope）
- 并行会话活跃：所有提交用精确文件 add，勿 `git add -A`；tsc 已 0（并行会话修了预存错误）

**待处理前置**：issue 01（storeReferences 方向）——T3 来源清单含"源节点"依赖 storeReferences 正确方向（TestCodeFlow 场景）；@store 接线未做，demo2 的 TodoStore 引用卡在 storeReferences 空——**T3 前修 issue 01（消费者翻转 4 处 + fixture）**。
