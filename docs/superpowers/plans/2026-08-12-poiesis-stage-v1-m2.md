# Poiesis 阶段 v1 M2（睡态工具）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 底栏两件套（[组件库]/[数据节点]）+ 源节点卡片（store/signal 简化卡片，数据区自动排布可拖动）+ 改值调试（点卡片改值 → 画布当场演）。

**Architecture:** 底栏 = DesignPage grid 第 4 行改造（面包屑左侧保留，右侧两件套按钮）；源节点数据 = 纯函数收集（组件 signals + 引用 store signals + inline store signals + computeds，含当前值）；数据区 = 醒态 + [数据节点] 开关下可见的源节点卡片层（画布一角自动排布、@dnd-kit 可拖动）；改值走现有 debugValues 管线（setDebugValue → useWasmCompile applyDebugOverrides → WASM 重编译，零新机制）。左栏删 ComponentPalette（[组件库] 承载其拖拽能力，左栏只留 ComponentTree）。

**Tech Stack:** 同 M1（Tauri v2 + React 19 + Zustand 5 + Vitest 4 + @dnd-kit 6）。

**契约来源:** `.scratch/poiesis-stage-v1/spec.md` M2 段；设计稿 §15（底栏两件套/源节点/改值调试）。

**验收口径:** 每个 Task 独立可提交、测试全绿；Task 8 为实机验证剧本（Victauri），M2 里程碑验收以 Task 8 清单为准。

**关键状态规则（本计划锁定）:**
```
dataNodesVisible: boolean（[数据节点] 开关，store 瞬态，不入 undo/持久化）
源节点可见 ⇔ awake && dataNodesVisible（spec：源节点睡态不可见；开关只在醒态下有效）
源节点 = 当前组件 tab 的：组件 signals + computeds + 引用的 store signals/getters + inline store signals
改值 = 现有 setDebugValue(ownerKey, name, text, valid) → applyDebugOverrides 管线（不改编译器）
[组件库] 弹出面板 = ComponentPalette 现有内容（拖拽走 @dnd-kit useDraggable，原样复用）
```

---

## Task 1: 底栏两件套（BottomBar 组件 + store 开关）

**Files:**
- Create: `poiesis/src/components/design/BottomBar.tsx` + `BottomBar.module.css`
- Modify: `poiesis/src/pages/DesignPage.tsx`（grid rows + 挂 BottomBar）
- Modify: `poiesis/src/store/operations/tab-lifecycle.ts` + `types.ts` + `core/types/design-page.ts`（dataNodesVisible + setDataNodesVisible）
- Test: `poiesis/src/components/design/__tests__/BottomBar.test.tsx`

- [ ] **Step 1: TDD 失败测试** — BottomBar.test.tsx（@testing-library/react）：
  1. 渲染两个按钮：[组件库]（data-testid="palette-toggle"）+ [数据节点]（data-testid="data-nodes-toggle"）
  2. 点 [数据节点] → store `designPage.dataNodesVisible` 翻转（true → false）
  3. [组件库] 点击 → 触发回调 `onPaletteToggle`（BottomBar 受控，DesignPage 持有弹出面板 state）

- [ ] **Step 2: store 层** — `DesignPageState` 加 `dataNodesVisible: boolean`（初始 false）；`PoiesisState` 加 `setDataNodesVisible: (v: boolean) => void`；tab-lifecycle 初始 `dataNodesVisible: false` + action（同 setStorePanelVisible 模式）。瞬态不入 undo/持久化。

- [ ] **Step 3: BottomBar 组件** — 底栏布局：左 = 面包屑插槽（children 或直接渲染 BreadcrumbBar），右 = 两件套按钮组。
  ```tsx
  // poiesis/src/components/design/BottomBar.tsx
  // 底栏（M2）：左 = 面包屑；右 = [组件库]（弹出面板开关，DesignPage 受控）+ [数据节点]（源节点显隐开关）。
  // [数据节点] 只在组件 tab 显示；开关状态 = store 瞬态（dataNodesVisible）。
  export function BottomBar({ onPaletteToggle }: { onPaletteToggle: () => void }) { ... }
  ```
  - `data-testid="palette-toggle"` / `data-testid="data-nodes-toggle"`，激活态样式（同 StorePanelToggleButton 模式）
  - [数据节点] 激活条件：`designPage.dataNodesVisible && designPage.awake`（醒态下有效——睡态显示但点击仅存开关，源节点层由 awake 门控）

- [ ] **Step 4: DesignPage 接线** — grid rows `"36px 32px 1fr 24px"` → `"36px 32px 1fr 28px"`；第 4 行替换 BreadcrumbBar div 为 `<BottomBar onPaletteToggle={() => setPaletteOpen(v => !v)} />`（内部渲染 BreadcrumbBar 于左侧）。`paletteOpen` 为 DesignPage 本地 state（T6 用）。

- [ ] **Step 5: 验证 + 提交** — `npx vitest run src/components/design/__tests__` 绿；tsc 0 错误。提交：
  `feat(poiesis-stage-v1): 底栏两件套——[组件库]/[数据节点] 按钮 + dataNodesVisible 开关（M2 T1）`

---

## Task 2: 源节点数据收集工具（纯函数 + 单测）

**Files:**
- Create: `poiesis/src/components/canvas/source-nodes.ts`（纯函数）
- Test: `poiesis/src/components/canvas/__tests__/source-nodes.test.ts`

- [ ] **Step 1: TDD 失败测试** — collectSourceNodes(store state 输入) → 源节点列表：
  ```ts
  export interface SourceNode {
    id: string;          // `sig:${pid}` | `computed:${pid}` | `store:${storeName}.${pid}` | `inline:${key}.${pid}`
    kind: "signal" | "computed" | "store-signal" | "store-getter" | "inline-signal";
    name: string;        // 显示名（store 信号带 store 前缀，inline 不带）
    type: string;
    value: unknown;      // 当前值（defaultValue / 调试覆盖值）
    ownerKey: string;    // setDebugValue 的 ownerKey（组件名 | store 名 | "Comp.Store"）
  }
  export function collectSourceNodes(state: {
    componentName: string | null;
    componentDefs: Record<string, ComponentDef>;
    storeDefs: Record<string, StoreDefinition>;
    inlineStoreDefs: Record<string, StoreDefinition>;
    storeReferences: Record<string, string[]>;
    debugValues: Record<string, Record<string, DebugValueEntry>>;
  }): SourceNode[]
  ```
  测试矩阵：组件信号 + computeds 收集；引用 store 信号/getters（带前缀名）；inline store（key 前缀匹配组件名）；debugValues 覆盖值优先于 defaultValue；无组件 tab → 空数组。

- [ ] **Step 2: 实现** — 从 ContextInspector useSignalItems（已迁 shared.ts）的收集逻辑对齐（同源），补 value 解析（defaultValue → debugValues[ownerKey][name] 覆盖）。**注意 debug 覆盖值形状**：DebugValueEntry = { text, valid }——value 用 text（valid 时），defaultValue 兜底。

- [ ] **Step 3: 验证 + 提交** — 单测绿 + tsc 0 错误。提交：
  `feat(poiesis-stage-v1): 源节点收集工具——组件/引用 store/inline 信号 + 调试覆盖值解析（M2 T2）`

---

## Task 3: 源节点卡片组件（SourceNodeCard）

**Files:**
- Create: `poiesis/src/components/canvas/SourceNodeCard.tsx` + `.module.css`
- Test: `poiesis/src/components/canvas/__tests__/SourceNodeCard.test.tsx`

- [ ] **Step 1: TDD 失败测试**：
  1. 渲染图标 + 名 + 当前值（kind 图标映射：signal=⚡/computed=🧮/store=📦/inline=🔌——同设计稿源节点形象基础版）
  2. 点击卡片 → `onEdit` 回调
  3. 卡片可拖（@dnd-kit useDraggable 包裹——draggable id = 源节点 id）

- [ ] **Step 2: 实现** — 简化卡片（图标 + 名 + 当前值截断），useDraggable（@dnd-kit，同 ComponentPalette PaletteDraggableItem 模式 38 行），onEdit 点击回调。CSS：紧凑卡片（~180×32），hover 态。

- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 源节点卡片——图标+名+当前值 + 可拖 + 点击编辑（M2 T3）`

---

## Task 4: 数据区（SourceNodeLayer：醒态 + 开关 + 自动排布 + 可拖动）

**Files:**
- Create: `poiesis/src/components/canvas/SourceNodeLayer.tsx` + `.module.css`
- Test: `poiesis/src/components/canvas/__tests__/SourceNodeLayer.test.tsx`
- Modify: `poiesis/src/pages/DesignPage.tsx`（挂载）

- [ ] **Step 1: TDD 失败测试**：
  1. 不可见条件（睡态 / 开关关）→ 不渲染（测试：awake=false → null；dataNodesVisible=false → null）
  2. 可见时按源节点列表渲染卡片（collectSourceNodes mock 或 store 构造）
  3. 点击卡片 → 打开改值浮层（T5 的 ValueEditor 弹层出现）

- [ ] **Step 2: 实现** — 挂载契约（仿 ElementPortLayer）：DesignPage 中心区（zAxisRef div 内）absolute 子节点，数据区 = **画布一角**（设计稿 §15：数据区画布一角；本计划：视口右上角固定区，宽 ~360 卡片列排，顶部横向滚动或纵向堆叠——实现选：右上角纵向堆叠，卡片自动排布（不重叠），DndContext 拖动自由摆放（拖动位置本地 state，不入 store——瞬态））。可见门控：`awake && dataNodesVisible && activeTabType === "component"`。
  - 数据来源：usePoiesisStore 订阅（components 的 signals/computeds + storeDefs + inlineStoreDefs + storeReferences + debugValues），经 collectSourceNodes 收集
  - DndContext（@dnd-kit）包裹卡片区；拖动位置 = 组件内 useState（初始按序排布）

- [ ] **Step 3: DesignPage 挂载** — zAxisRef div 内、LogicLayer 之后加 `<SourceNodeLayer />`（component tab 分支内）。

- [ ] **Step 4: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 数据区——源节点卡片层（醒态+开关门控，右上角自动排布可拖动）（M2 T4）`

---

## Task 5: 改值调试（ValueEditor 浮层）

**Files:**
- Create: `poiesis/src/components/canvas/ValueEditor.tsx` + `.module.css`
- Modify: `poiesis/src/components/canvas/SourceNodeLayer.tsx`（点卡片 → 浮层）
- Test: `poiesis/src/components/canvas/__tests__/ValueEditor.test.tsx`

- [ ] **Step 1: TDD 失败测试**：
  1. 渲染当前值到输入框；保存按钮提交
  2. 提交 → store.setDebugValue(ownerKey, name, text, valid)（valid：字面量可解析——参考 debug-values.test.ts 现有语义；非法值 valid=false 不参与编译但保留输入）
  3. 关闭按钮 → 浮层消失

- [ ] **Step 2: 实现** — 浮层（position fixed，卡片旁/下方），输入框 + 保存/关闭；提交走 setDebugValue + markDirty？——**不 dirty**（spec：调试专用，不 dirty 项目——debugValues 注释已声明）。值类型提示（type 字段）。

- [ ] **Step 3: 验证 + 提交** — 提交：
  `feat(poiesis-stage-v1): 改值调试——源节点浮层编辑值 → setDebugValue 管线当场演（M2 T5）`

---

## Task 6: [组件库] 弹出面板 + 左栏删 ComponentPalette

**Files:**
- Create: `poiesis/src/components/design/PaletteDrawer.tsx`（弹出面板，承载 ComponentPalette）
- Modify: `poiesis/src/pages/DesignPage.tsx`（paletteOpen state 接线；左栏删 ComponentPalette）
- Delete: `poiesis/src/components/panels/ComponentPalette.tsx` + `.module.css`（能力迁入 PaletteDrawer；如组件树也引用则检查）
- Test: `poiesis/src/components/design/__tests__/PaletteDrawer.test.tsx`

- [ ] **Step 1: 摸底** — grep ComponentPalette 引用（DesignPage 唯一消费方？测试？）
- [ ] **Step 2: TDD** — PaletteDrawer：open 时渲染 ComponentPalette 内容（迁移组件体），close 按钮；拖拽经现有 DndContext（DesignPage 顶层已包裹——PaletteDrawer 在 DndContext 内即可拖）
- [ ] **Step 3: 实现** — ComponentPalette 组件体迁入 PaletteDrawer（原样搬移），DesignPage：左栏删 ComponentPalette 块（grid 第 1 列只剩 ComponentTree），paletteOpen → PaletteDrawer 浮层（fixed 底部上方/左侧）
- [ ] **Step 4: 验证 + 提交** — 全量 vitest 绿 + tsc 0。提交：
  `feat(poiesis-stage-v1): [组件库] 弹出面板 + 左栏只留组件树（M2 T6）`

---

## Task 7: 全量回归

- [ ] **Step 1** — `npx vitest run` 全绿（0 failed）；`npx tsc --noEmit` 0 错误
- [ ] **Step 2** — grep 无 ComponentPalette 残留引用（除 PaletteDrawer 内部）；新增文件均有测试
- [ ] **Step 3** — 提交（如有零散调整）或声明无改动

---

## Task 8: 实机验证剧本（Victauri，M2 里程碑验收）

> 前置：`npm run tauri dev`（后台，勿用管道 head 截断——SIGPIPE 杀进程），Victauri REST fallback（`$TEMP/victauri/<pid>/token`）。脚本参考 M1 验收（`$CLAUDE_JOB_DIR/tmp/vt.sh`）。

- [ ] **Step 1**: get_plugin_info 确认 com.omnilang.poiesis；打开 demo2 项目；进 main 组件 tab
- [ ] **Step 2**: 底栏出现 [组件库][数据节点]；点 [数据节点] → store dataNodesVisible true（eval_js 读 `window.__POIESIS_STORE__.getState().designPage.dataNodesVisible`）
- [ ] **Step 3**: 睡态点 [数据节点] → 源节点不出现（睡态门控 ✓）；点呼吸点醒 → 源节点卡片浮现（数据区右上角，数量 = 组件信号 + store 信号；demo2 main 有 newSignal + TodoStore 引用 → 预期 ≥2 张卡）
- [ ] **Step 4**: 卡片显示名/值正确（newSignal 卡显示当前值 "signalcs/"）；拖动卡片 → 位置变化（get_bounding_boxes 前后对比）
- [ ] **Step 5**: 点卡片 → 改值浮层 → 输入 "T8 Test" → 保存 → 画布 {newSignal} 文本当场变（applyDebugOverrides → WASM 重编译链路 ✓）；重开 app 值还原（调试值不持久化 ✓）
- [ ] **Step 6**: [组件库] → 弹出面板 → 拖 div 到画布 → 元素创建（拖拽链路复用 ✓）；左栏只剩组件树
- [ ] **Step 7**: 不倒退——睡态画布交互/语境面板/呼吸点/Save 往返与 M1 验收一致
- [ ] **Step 8**: 验收记录写入 `.scratch/poiesis-stage-v1/acceptance-m2.md`

---

## Self-Review 对照

**Spec 覆盖（M2 段）**：底栏两件套 → T1+T6；源节点卡片（图标+名+当前值）→ T3；数据区自动排布可拖动 → T4；[数据节点] 开关显隐（醒态下有效）→ T1+T4 门控；改值调试（点卡改值 → 画布当场演）→ T5（走现有 debugValues 管线，零新机制）；左栏只留组件树 → T6。变量/数据页源节点 + 账本/小灯形象 = 后置（spec Out of Scope）。

**已知边界（如实声明）**：
- 源节点睡态不可见（spec 明列）；开关状态睡态可切但无视觉（醒时生效）
- 拖动位置本地 state 不入 store（瞬态；M3 若需持久化再议）
- 调试值 valid 语义沿用现有 debug-values 测试（text/valid 双字段，valid=false 不参与编译）
- 数据区固定右上角（画布一角简化版；"账本/工作台/小灯形象"后置）
- 并行会话活跃（batch-4）：所有提交用精确文件 add，勿 `git add -A`
