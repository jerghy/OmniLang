# Poiesis 阶段 v2（收口 + 源语义清理 + 源节点形象化）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** v2 四件套——行为卡保存往返实机验证（A1）、拖拽交互人工实测（A2）、源语义清理（B1：显示元素只作目标，源 = 值型控件/源节点/fn 出口）、源节点五类形象化（B2：小灯/账本/算盘）。语法零变更。

**Architecture:** A1/A2 是验证收口（Victauri 实机剧本 + 人工实测），暴露缺陷按 bug 修；B1 是语义修正——点名清单"画布元素"分组收窄为值型控件、`connectElementLink` 源端校验拒绝显示元素、点名选值型控件与拖线走同一条 auto-link 落盘路径；B2 是 SourceNodeCard 视觉层替换（emoji → SVG 形象体系 + 类型配色）。

**Tech Stack:** 同 v1（Tauri v2 + React 19 + Zustand 5 + ReactFlow @xyflow/react ^12 + Vitest 4）。

**契约来源:** `docs/superpowers/specs/2026-08-12-poiesis-stage-v2-design.md`；设计稿 §15.2（形象）/ §15.3（端口语义：输入框值出口、标题文本入口）/ §17.7（线源限三类）。

**验收口径:** 每 Task 独立提交、vitest 全绿 + tsc 0 错误；Task 1/2 为实机/人工验证（Victauri + 用户操作），Task 3-5 为代码（TDD）。

**提交纪律（v1 教训）:** 并行会话活跃（batch-4 等）——**只精确 add 本计划文件**（`git add <file>` 后 `git commit -- <file>` 或先 `git reset` 检查暂存区），提交前查 `git status --porcelain` 确认未携带他人暂存内容；react-doctor hook 本机不可用，`--no-verify` 是既定惯例。

**已知边界（本计划锁定）:**
```
行为卡 = 事件绑定填写入口；重开后的管理走 EventsSection（不做口语回显清单）
显示元素当源 = 语义边界（明确拒绝，非后置待定）
变量（便签）/ 数据页（工作台）形象 = 无语言载体，不涉及
值型控件已绑 bind:value 时再当源 → 现有语义覆盖原绑定（M3 既有行为，v2 不处理）
```

---

## Task 0: v1 工件收口提交

**Files:**
- Add: `.scratch/poiesis-stage-v1/spec.md`、`acceptance-m1.md`、`acceptance-m2.md`、`acceptance-m3.md`、`issues/01-storeReferences-direction.md`
- Add: `docs/superpowers/plans/2026-08-11-poiesis-stage-v1-m1.md`、`2026-08-12-poiesis-stage-v1-m2.md`、`2026-08-12-poiesis-stage-v1-m3.md`

- [ ] **Step 1: 精确暂存并提交（勿 `git add -A`）**

```bash
git add .scratch/poiesis-stage-v1/spec.md .scratch/poiesis-stage-v1/acceptance-m1.md .scratch/poiesis-stage-v1/acceptance-m2.md .scratch/poiesis-stage-v1/acceptance-m3.md .scratch/poiesis-stage-v1/issues/01-storeReferences-direction.md docs/superpowers/plans/2026-08-11-poiesis-stage-v1-m1.md docs/superpowers/plans/2026-08-12-poiesis-stage-v1-m2.md docs/superpowers/plans/2026-08-12-poiesis-stage-v1-m3.md
git commit -m "chore(poiesis-stage-v1): v1 收官工件落库——spec/三里程碑验收/遗留 issue/实施计划"
```

- [ ] **Step 2: 验证未携带并行会话文件**

```bash
git status --porcelain   # 应只剩并行会话的 M/A 文件（crates/omnilang-core、omnilang/src/codegen/react 等）
git show --stat HEAD | tail -12   # 应只含本次 8 个文件
```

---

## Task 1: A1 行为卡保存往返实机验证（Victauri 剧本 + 缺陷修复）

**Files:**
- 无源码修改预期；缺陷暴露时按 bug 修（涉及 `poiesis/src/store/slices/behavior.ts`、`poiesis/src/services/codegen/` parser/emitter、`flow-scope` 等）
- 验收记录: `.scratch/poiesis-stage-v2/acceptance-a1.md`（新建）

**Interfaces:**
- Consumes: v1 M3 交付（applyBehaviorCard 三件套 + EventsSection + auto fn 命名 `on<Event>Auto`）
- Produces: 往返闭环确认 or 缺陷修复提交；验收记录供 v2 收口

- [ ] **Step 1: 启动 app 并连接 Victauri**

```bash
bash /d/space/OmniLang/.claude/skills/start-poiesis/start-poiesis.sh
# 用户重连 Victauri MCP；先 get_plugin_info 确认 app.identifier
```

- [ ] **Step 2: 打开 demo2 项目（D:\space\omni\demo2）**

打开组件 tab（如含按钮的组件），`dom_snapshot` 确认画布渲染、行为卡表单可见（`[data-testid="behavior-card"]`）。

- [ ] **Step 3: 行为卡提交**

`find_elements` 定位行为卡表单 → 选事件"点击"（`[data-testid="behavior-event"]`）→ 动作"加"（`[data-testid="behavior-action"]`）→ 目标信号（`[data-testid="behavior-target"]`）→ 提交（`[data-testid="behavior-commit"]`）。

验证（`eval_js` 读 store）：
```js
// 自动 fn 生成 + 事件指向
const s = usePoiesisStore.getState();
JSON.stringify(s.elements.filter(e => e.nodeType === "codeblock" && e.tag === "fn").map(e => e.props.name))  // 含 onPressAuto
// 按钮事件指向
s.elements.find(e => e.tag === "button")?.props.onPress   // "onPressAuto"
// wire：fn → code-block
JSON.stringify(s.wires)
```

- [ ] **Step 4: 保存**

编辑器保存（顶部保存按钮或 Ctrl+S）。`eval_js` 或磁盘检查：
```bash
grep -n "onPressAuto\|@onPress" /d/space/omni/demo2/src/**/*.omni   # fn onPressAuto + @onPress={onPressAuto}
```

- [ ] **Step 5: 重开项目**

关闭 tab → 重开（或重启 app 重新打开 demo2）。等待 WASM 编译完成（`wait_for` 画布元素出现）。

- [ ] **Step 6: 验证全链往返**

```js
const s = usePoiesisStore.getState();
JSON.stringify(s.elements.filter(e => e.nodeType === "codeblock" && e.tag === "fn").map(e => ({name: e.props.name, src: e.source})))  // fn + body source 原文回来
JSON.stringify(s.wires)   // flow-out → flow-in 线回来
s.elements.find(e => e.tag === "button")?.props.onPress   // "onPressAuto"
```
另验证：画布无编译错误（`logs console` 无 WASM 错误）；EventsSection 显示 `onPress → onPressAuto`（dom_snapshot）；实机点击按钮 → 值变化（`eval_js` 触发点击或人工点击后读源节点卡片值）。

- [ ] **Step 7: 缺陷修复（如暴露）**

往返缺陷（如 fn body source 丢失、codeFlowGraphs 未恢复、事件指向丢失）按 bug 修复 + 补测试，独立提交：`fix(poiesis-stage-v2): <缺陷描述>（A1 实机发现）`。

- [ ] **Step 8: 写验收记录并提交**

`.scratch/poiesis-stage-v2/acceptance-a1.md`：剧本步骤/证据/缺陷（如有）。提交：`docs(poiesis-stage-v2): A1 行为卡往返实机验收记录`。

---

## Task 2: A2 拖拽交互人工实测（用户真实指针操作）

**Files:**
- 无源码修改预期；手感缺陷按 bug 修
- 验收记录: `.scratch/poiesis-stage-v2/acceptance-a2.md`（新建）

**Interfaces:**
- Consumes: v1 M3 交付（PortHandleLayer 手柄拖线 / 底栏组件库 / SourceNodeLayer 拖动改值）
- Produces: 真实指针交互确认 or 缺陷修复；边界记录

- [ ] **Step 1: 用户实机操作（真实指针）**

启动 app（同 Task 1），用户按剧本操作：
1. 醒态手柄拖线：从输入框"值"手柄拖到标题"文本"手柄 → 绑定成功（auto signal 生成）
2. 底栏 [组件库] 拖元素入画布
3. 源节点卡片拖动排布 / 点击改值浮层

- [ ] **Step 2: 验证落盘形状**

每次交互后 `eval_js` 读 store 验证（拖线 → wires + componentDefs.signals 含 `<源>_value`；组件库拖入 → elements 增加；改值 → debugValues 更新）：
```js
const s = usePoiesisStore.getState();
JSON.stringify(s.wires)
JSON.stringify((s.componentDefs["<组件名>"]?.signals ?? []).map(x => x.name))
```

- [ ] **Step 3: 缺陷修复（如暴露）**

真实指针下发现的问题（手柄命中/拖拽激活/落点解析）按 bug 修 + 补测试，独立提交。

- [ ] **Step 4: 写验收记录并提交**

`.scratch/poiesis-stage-v2/acceptance-a2.md`。提交：`docs(poiesis-stage-v2): A2 拖拽人工实测验收记录`。

---

## Task 3: B1a 源端语义——connectElementLink 拒绝非值型源 + 点名清单收窄

**Files:**
- Modify: `poiesis/src/store/slices/auto-link.ts`（connectElementLink 源端校验）
- Modify: `poiesis/src/components/design/binding-sources.ts`（画布元素分组收窄为值型控件）
- Test: `poiesis/src/store/__tests__/auto-link.test.ts`（新用例）
- Test: `poiesis/src/components/design/__tests__/binding-sources.test.ts`（改 fixture + 断言）

**Interfaces:**
- Consumes: 现有 `connectElementLink(sourceId, sourceSlot, targetId, targetSlot)`；`buildBindingSources(state)` → `BindingSource[]`
- Produces:
  - `connectElementLink` 对非值型源拒绝（console.warn + 不落盘）
  - `buildBindingSources` 的 element 分组只含 tag ∈ `["input","textarea","select","slider"]` 的 view 元素

- [ ] **Step 1: 写失败测试（auto-link 源校验）**

`poiesis/src/store/__tests__/auto-link.test.ts` 追加 describe：
```ts
describe("auto-link: 源端校验（B1 显示元素只作目标）", () => {
  beforeEach(setupStore);

  it("显示元素（h1）当源 → 拒绝：不建 signal、不落 wire", () => {
    const store = usePoiesisStore.getState();
    store.connectElementLink("title_1", "label", "input_1", "value");
    expect(getAppSignals()).toHaveLength(0);
    expect(usePoiesisStore.getState().wires).toHaveLength(0);
  });

  it("值型控件（input）当源 → 正常创建（既有语义不回归）", () => {
    const store = usePoiesisStore.getState();
    store.connectElementLink("input_1", "value", "title_1", "label");
    expect(getAppSignals()).toHaveLength(1);
    expect(getAppSignals()[0].name).toBe("input_1_value");
  });
});
```

- [ ] **Step 2: 运行确认失败**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/store/__tests__/auto-link.test.ts
```
Expected: 新用例失败（显示元素当源当前被接受）

- [ ] **Step 3: 实现源端校验**

`poiesis/src/store/slices/auto-link.ts`：模块级常量 + connectElementLink 内 source 查找后校验：
```ts
/** 值型控件标签集合（B1：源 = 值型控件/源节点/fn 出口三类；显示元素只作目标） */
const SOURCE_VALUE_TAGS = new Set(["input", "textarea", "select", "slider"]);
```
在 `const source = s.elements.find(...)` 后插入：
```ts
// B1 源端校验：显示元素无"值出口"语义（设计稿 §15.3 输入框值出口/标题文本入口），拒绝当源
if (!source || !("tag" in source) || !SOURCE_VALUE_TAGS.has((source as { tag?: string }).tag ?? "")) {
  console.warn(`[auto-link] 源元素 ${sourceId} 非值型控件（显示元素只作目标），拒绝连线`);
  return;
}
```

- [ ] **Step 4: 运行确认通过**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/store/__tests__/auto-link.test.ts
```
Expected: 全部通过（含既有用例——input 源不受影响）

- [ ] **Step 5: 改 binding-sources 收窄 + 测试**

`binding-sources.ts` 第 3 段（画布元素）过滤条件改为值型控件：
```ts
// 3) 画布元素（值型控件，有"值出口"语义；显示元素只作目标——B1 收窄）
const VALUE_TAGS = new Set(["input", "textarea", "select", "slider"]);
for (const e of elements) {
  if (e.nodeType !== "view") continue;
  if (!e.tag || !VALUE_TAGS.has(e.tag)) continue;
  const label = typeof e.props?.label === "string" ? e.props.label : "";
  const name = label.trim().split("\n")[0] || e.id;
  out.push({ id: `el:${e.id}`, kind: "element", name, type: e.tag ?? "元素" });
}
```

`binding-sources.test.ts`：fixture 中 `el5`（button）改为期望排除；断言更新：
```ts
test("画布元素分组只列值型控件（input/textarea/select/slider）；button 等显示元素排除", () => {
  const els = buildBindingSources(state).filter((s) => s.kind === "element");
  expect(els.map((s) => s.name)).toEqual(["搜索框"]);
  expect(els.map((s) => s.id)).toEqual(["el:el4"]);
});
```
（保留原"非 view 跳过"行为：el6 conditional 已排除）

- [ ] **Step 6: 运行 + tsc + 提交**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/store/__tests__/auto-link.test.ts src/components/design/__tests__/binding-sources.test.ts
npx tsc --noEmit   # 0 错误
```
提交（精确 add，仅本 Task 文件）：
```bash
git add poiesis/src/store/slices/auto-link.ts poiesis/src/store/__tests__/auto-link.test.ts poiesis/src/components/design/binding-sources.ts poiesis/src/components/design/__tests__/binding-sources.test.ts
git commit -m "fix(poiesis-stage-v2): 源语义——connectElementLink 拒绝非值型源 + 点名清单收窄为值型控件（B1a）"
```

---

## Task 4: B1b 点名统一路径——选值型控件 → connectElementLink

**Files:**
- Modify: `poiesis/src/components/design/BindingPicker.tsx`（BindingSource 加 elementId 字段）
- Modify: `poiesis/src/components/design/ContentTab.tsx`（pick 分支：element 来源 → connectElementLink）
- Test: `poiesis/src/components/design/__tests__/ContentTab.test.tsx`（新用例）

**Interfaces:**
- Consumes: Task 3 的 `connectElementLink`（含源校验）；`buildBindingSources` 收窄后的 element 分组
- Produces:
  - `BindingSource` 增加 `elementId?: string`（kind === "element" 时携带源元素 id）
  - ContentTab pick：`source.kind === "element"` → `connectElementLink(source.elementId!, "value", el.id, slot.key)`

- [ ] **Step 1: 写失败测试**

`ContentTab.test.tsx` 追加（先看现有测试的渲染装配方式，沿用其 store 注入/挂载模式）：
```ts
it("点名选值型控件 → connectElementLink 统一路径（建 auto signal + 目标绑定 + wire）", () => {
  // 装配：App 组件含 input_1（值型控件）+ 当前选中文本元素 title_1
  // 打开 [内容] 标签 → 点"文本内容"槽的绑定按钮 → 点画布元素分组里的 input 来源
  // 断言：componentDefs.signals 含 input_1_value；title_1.props.label === "{input_1_value}"；wires 长度 1
});
```

- [ ] **Step 2: 运行确认失败**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/ContentTab.test.tsx
```
Expected: 新用例失败（当前 pick 把 label 文本当表达式落盘）

- [ ] **Step 3: BindingSource 加 elementId**

`BindingPicker.tsx`：
```ts
export interface BindingSource {
  id: string;
  kind: "signal" | "fn" | "element";
  name: string;
  type: string;
  /** kind === "element" 时携带源元素 id（B1：点名统一路径 → connectElementLink） */
  elementId?: string;
}
```

`binding-sources.ts` 第 3 段 push 时补 `elementId: e.id`。

- [ ] **Step 4: ContentTab pick 分支**

`ContentTab.tsx` pick 函数开头插入：
```ts
const pick = (slot: SlotDef, source: BindingSource) => {
  // B1 点名统一路径：值型控件当源 = 与拖线同一条落盘路径（auto signal + 目标绑定 + wire）
  if (source.kind === "element" && source.elementId) {
    usePoiesisStore.getState().connectElementLink(source.elementId, "value", el.id, slot.key);
    setPickingFor(null);
    return;
  }
  switch (slot.kind) { ...原有分支... }
};
```

- [ ] **Step 5: 运行 + tsc + 提交**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/ContentTab.test.tsx src/components/design/__tests__/binding-sources.test.ts
npx tsc --noEmit
```
提交：
```bash
git add poiesis/src/components/design/BindingPicker.tsx poiesis/src/components/design/ContentTab.tsx poiesis/src/components/design/binding-sources.ts poiesis/src/components/design/__tests__/ContentTab.test.tsx
git commit -m "feat(poiesis-stage-v2): 点名选值型控件 = 与拖线统一落盘路径（connectElementLink）（B1b）"
```

---

## Task 5: B2 源节点五类形象化

**Files:**
- Modify: `poiesis/src/components/canvas/SourceNodeCard.tsx`（KIND_ICONS → SVG 形象体系 + 类型配色类）
- Modify: `poiesis/src/components/canvas/SourceNodeCard.module.css`（图标/色条/角标样式）
- Modify: `poiesis/src/components/canvas/SourceNodeLayer.tsx`（自动连线分组分隔线微调，可选）
- Test: `poiesis/src/components/canvas/__tests__/SourceNodeCard.test.tsx`（形象断言）

**Interfaces:**
- Consumes: `SourceNode.kind`（"signal" | "computed" | "store-signal" | "store-getter" | "inline-signal"）
- Produces: 五类形象图标 + 类型配色 + getter 页签 + inline 本地角标；交互（拖拽/改值）不回归

**形象映射（spec 已拍板）:** signal→小灯、computed→算盘、store-signal→账本、store-getter→账本（"算"页签）、inline-signal→小灯（"本地"角标）

- [ ] **Step 1: 写失败测试**

`SourceNodeCard.test.tsx` 追加（沿用现有渲染模式）：
```ts
it("五类形象：signal 小灯 / computed 算盘 / store-signal 账本 / store-getter 账本+算页签 / inline 小灯+本地角标", () => {
  const kinds: SourceNode["kind"][] = ["signal", "computed", "store-signal", "store-getter", "inline-signal"];
  // 逐个渲染 SourceNodeCard（node.kind = kind），断言：
  //   - 图标容器 data-kind 属性 = kind
  //   - store-getter 渲染"算"页签；inline-signal 渲染"本地"角标
});
```
（图标断言用 data 属性而非 SVG 细节——视觉断言不可脆）

- [ ] **Step 2: 运行确认失败**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/SourceNodeCard.test.tsx
```

- [ ] **Step 3: 实现形象体系**

`SourceNodeCard.tsx`：
```tsx
// B2 形象体系（设计稿 §15.2 拍板映射）：signal 小灯 / computed 算盘 / store 账本 /
// store-getter 账本+算页签 / inline-signal 小灯+本地角标。data-kind 供测试与主题挂钩。
const KIND_ICON: Record<SourceNode["kind"], JSX.Element> = {
  signal: <svg data-kind="signal" viewBox="0 0 16 16" width="14" height="14" aria-hidden><path d="M8 2v1M8 6v1M8 10v1M8 13v1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" fill="none"/><circle cx="8" cy="4" r="1.6" fill="currentColor"/><circle cx="8" cy="8" r="1.6" fill="currentColor"/><circle cx="8" cy="12" r="1.6" fill="currentColor"/></svg>, // 小灯（串灯）
  computed: <svg data-kind="computed" viewBox="0 0 16 16" width="14" height="14" aria-hidden><rect x="3" y="2" width="10" height="12" rx="1" fill="none" stroke="currentColor"/><path d="M5 5h6M5 8h6M5 11h4" stroke="currentColor"/></svg>, // 算盘（框+算珠）
  "store-signal": <svg data-kind="store-signal" viewBox="0 0 16 16" width="14" height="14" aria-hidden><path d="M3 4h10v9H3z" fill="none" stroke="currentColor"/><path d="M3 4l2-2h6l2 2" fill="none" stroke="currentColor"/></svg>, // 账本（开本）
  "store-getter": <svg data-kind="store-getter" viewBox="0 0 16 16" width="14" height="14" aria-hidden><path d="M3 4h10v9H3z" fill="none" stroke="currentColor"/><path d="M3 4l2-2h6l2 2" fill="none" stroke="currentColor"/><path d="M8 7v4M6 9h4" stroke="currentColor"/></svg>, // 账本+算（+号）
  "inline-signal": <svg data-kind="inline-signal" viewBox="0 0 16 16" width="14" height="14" aria-hidden><path d="M8 2v1M8 6v1M8 10v1M8 13v1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" fill="none"/><circle cx="8" cy="4" r="1.6" fill="currentColor"/><circle cx="8" cy="8" r="1.6" fill="currentColor"/><circle cx="8" cy="12" r="1.6" fill="currentColor"/></svg>, // 小灯同形（本地角标区分）
};
```
卡片渲染：图标外层包 `<span className={styles.icon} data-kind={node.kind}>`；store-getter 追加 `<span className={styles.badge} data-testid="kind-badge">算</span>`；inline-signal 追加 `<span className={styles.badge}>本地</span>`。配色类按 kind 挂 `<span className={\`${styles.iconWrap} ${styles["kind-" + node.kind]}\`}>`。

`SourceNodeCard.module.css` 追加：
```css
.iconWrap { display: inline-flex; align-items: center; flex: 0 0 auto; }
.kind-signal { color: #f59e0b; }            /* 小灯 · 琥珀 */
.kind-computed { color: #8b5cf6; }          /* 算盘 · 紫 */
.kind-store-signal { color: #10b981; }      /* 账本 · 绿 */
.kind-store-getter { color: #10b981; }      /* 账本 · 绿 */
.kind-inline-signal { color: #f59e0b; }     /* 小灯 · 琥珀 */
.badge {
  flex: 0 0 auto; font-size: 9px; line-height: 1;
  padding: 1px 4px; border-radius: 999px;
  background: var(--ol-color-primary-subtle); color: var(--ol-color-primary);
}
```
（旧 `.icon` emoji 样式保留给兼容或删除——选择删除并同步测试）

- [ ] **Step 4: 运行 + 全量回归 + tsc + 提交**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/SourceNodeCard.test.tsx src/components/canvas/__tests__/SourceNodeLayer.test.tsx
npx vitest run   # 全量（~910 tests）全绿
npx tsc --noEmit
```
提交：
```bash
git add poiesis/src/components/canvas/SourceNodeCard.tsx poiesis/src/components/canvas/SourceNodeCard.module.css poiesis/src/components/canvas/__tests__/SourceNodeCard.test.tsx
git commit -m "feat(poiesis-stage-v2): 源节点五类形象化——小灯/算盘/账本/算页签/本地角标（B2）"
```

---

## Task 6: v2 收口（可选——A1/A2 缺陷修复后的全量回归）

- [ ] **Step 1: 全量回归**

```bash
cd /d/space/OmniLang/poiesis && npx vitest run && npx tsc --noEmit
```
- [ ] **Step 2: 验收总结**

`.scratch/poiesis-stage-v2/acceptance-v2.md`：四 Task 验收汇总 + 边界记录。提交：`docs(poiesis-stage-v2): v2 里程碑验收汇总`。
