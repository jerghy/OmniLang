# Poiesis 阶段 v1 M1（打通地基）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把设计画布与 CodeFlow 逻辑画布从"互斥两块"变为一块画布两种呼吸——睡/醒状态机 + 呼吸点 + 逻辑层叠加（视口同步）+ 元素端口视觉层 + 浮动语境面板（替代右栏）。

**Architecture:** 设计层 DOM 渲染（WASM 元素树）保持不动；醒态在其上叠加 ReactFlow 逻辑层（视口单一真相 = useCanvasStore 驱动 ReactFlow viewport）；删掉 codeFlowVisible/TopLayer/DesignPreviewNode 旧互斥机制；选中元素 → 浮动语境面板（主面板 [行为][内容][结构]，副面板仅 [内容]，最多 2 个）；Shift 多选 = 醒区。所有新状态为瞬态视图状态（不入 undo、不持久化）。

**Tech Stack:** Tauri v2 + React 19 + Zustand 5 + ReactFlow（@xyflow/react ^12.11）+ Vitest 4 + @testing-library/react 16。

**契约来源:** `.scratch/poiesis-stage-v1/spec.md` M1 段；设计稿 §17（拍板 #47-49）；CONTEXT.md 词条（呼吸三态/呼吸点/醒区/语境面板/来源印记）。

**验收口径:** 每个 Task 独立可提交、测试全绿；Task 8 为实机验证剧本（Victauri），M1 里程碑验收以 Task 8 清单为准。

**关键状态规则（本计划锁定的实现决策，与 spec 状态机一致并细化）：**
```
awake: boolean                      // 整页醒（呼吸点 / [内容] 自动醒）
mainPanelTab: "structure"|"content"|"behavior"
醒区 awakeArea = selectedElementIds（Shift 多选建立，取消选中自动清空）
logicLayerVisible = awake || selectedElementIds.length >= 2
   // 整页醒 → fn 节点/源节点(M2)/全部元素端口
   // Shift 多选（≥2）→ 醒区：fn 节点/源节点也浮现（spec：逻辑层随醒处）
元素端口可见（逐元素成员语义，§15.4"线随醒处"）= awake || selectedIds.includes(elId)
   // 整页醒 → 全部元素长端口；多选（醒区）→ 仅选中元素长端口；睡态单选 → 无端口（干净画布）
面板 = selectedIds[0]（主，全标签）+ selectedIds[1]（副，仅 [内容]）；最多 2 个
[内容] 标签 → setMainPanelTab("content") → awake = true（意图驱动醒；切回其他标签不睡）
取消选中（clearSelection）→ 元素睡回（线/端口收起；来源印记 M3）
```

---

## Task 1: 视口同步工具（纯函数 + 单测）

**Files:**
- Create: `poiesis/src/components/canvas/viewport-sync.ts`
- Test: `poiesis/src/components/canvas/__tests__/viewport-sync.test.ts`

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/components/canvas/__tests__/viewport-sync.test.ts
import { describe, it, expect } from "vitest";
import { domToFlowViewport, flowToDomViewport } from "../viewport-sync";

describe("viewport-sync", () => {
  it("DOM 视口 → ReactFlow 视口：语义一致直接映射（屏幕 = 画布坐标 × zoom + pan）", () => {
    expect(domToFlowViewport({ zoom: 1.5, panX: 120, panY: -40 })).toEqual({ x: 120, y: -40, zoom: 1.5 });
  });

  it("ReactFlow 视口 → DOM 视口：逆映射", () => {
    expect(flowToDomViewport({ x: 120, y: -40, zoom: 1.5 })).toEqual({ panX: 120, panY: -40, zoom: 1.5 });
  });

  it("往返一致", () => {
    const dom = { zoom: 0.8, panX: 33, panY: 77 };
    expect(flowToDomViewport(domToFlowViewport(dom))).toEqual(dom);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/viewport-sync.test.ts`
Expected: FAIL with "Failed to resolve import ../viewport-sync"

- [ ] **Step 3: 实现工具**

```ts
// poiesis/src/components/canvas/viewport-sync.ts
// DOM 画布（CSS transform）↔ ReactFlow viewport 的坐标映射。
// 两引擎语义一致：屏幕坐标 = 画布坐标 × zoom + pan（pan 为屏幕像素）。
// DOM 侧 transform 为 `scale(zoom) translate(panX/zoom, panY/zoom)`（pan 已被 /zoom 补偿，
// 组合效果 = 先平移再缩放），ReactFlow 侧 x/y/zoom 同义——故直接映射，无换算。
// 若实机出现偏移，修这里两侧的换算（单测锁定契约），不要改消费方。
import type { Viewport } from "@xyflow/react";

export interface DomViewport {
  zoom: number;
  panX: number;
  panY: number;
}

export function domToFlowViewport(v: DomViewport): Viewport {
  return { x: v.panX, y: v.panY, zoom: v.zoom };
}

export function flowToDomViewport(v: Viewport): DomViewport {
  return { panX: v.x, panY: v.y, zoom: v.zoom };
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/viewport-sync.test.ts`
Expected: PASS (3 tests)

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/viewport-sync.ts poiesis/src/components/canvas/__tests__/viewport-sync.test.ts
git commit -m "feat(poiesis-stage-v1): 视口同步工具——DOM 画布 ↔ ReactFlow viewport 直接映射（单测锁定契约）"
```

---

## Task 2: store 睡/醒状态（awake + mainPanelTab + 自动醒）

**Files:**
- Modify: `poiesis/src/core/types/design-page.ts`（DesignPageState）
- Modify: `poiesis/src/store/types.ts`（PoiesisState 动作签名）
- Modify: `poiesis/src/store/operations/tab-lifecycle.ts`（初始状态 + 动作）
- Test: `poiesis/src/store/__tests__/sleep-awake.test.ts`

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/store/__tests__/sleep-awake.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";

// 睡/醒状态机（spec §17.3 状态规则）——瞬态视图状态，不入 undo、不持久化。
describe("睡/醒状态机", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false, mainPanelTab: "structure" },
      selectedElementIds: [],
    });
  });

  it("呼吸点：setAwake 翻转整页醒", () => {
    usePoiesisStore.getState().setAwake(true);
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
    usePoiesisStore.getState().setAwake(false);
    expect(usePoiesisStore.getState().designPage.awake).toBe(false);
  });

  it("[内容] 标签自动醒：setMainPanelTab('content') → awake = true", () => {
    usePoiesisStore.getState().setMainPanelTab("content");
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
    expect(usePoiesisStore.getState().designPage.mainPanelTab).toBe("content");
  });

  it("切回 [结构]/[行为] 不睡：睡/醒正交于标签切换（Q12-A）", () => {
    usePoiesisStore.getState().setMainPanelTab("content"); // 醒
    usePoiesisStore.getState().setMainPanelTab("structure");
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
  });

  it("选中集合 = 醒区：取消选中睡回", () => {
    usePoiesisStore.setState({ selectedElementIds: ["a", "b"] });
    expect(usePoiesisStore.getState().selectedElementIds).toHaveLength(2);
    usePoiesisStore.getState().clearSelection();
    expect(usePoiesisStore.getState().selectedElementIds).toHaveLength(0);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/store/__tests__/sleep-awake.test.ts`
Expected: FAIL（setAwake/setMainPanelTab 不存在，类型错误或 undefined 调用）

- [ ] **Step 3: 加类型与初始状态**

`poiesis/src/core/types/design-page.ts` —— 文件顶部加：

```ts
/** 语境面板主面板标签（[样子] 后置）。[内容] = 选中元素的接口（睡看印记、醒接/断，M3 起可接）。 */
export type PanelTab = "behavior" | "content" | "structure";
```

`DesignPageState` 接口中（`previewOutput` 行后）加两个字段：

```ts
  /** 整页醒（呼吸点 / [内容] 自动醒）。瞬态视图状态，不持久化。 */
  awake: boolean;
  /** 主面板活动标签。 */
  mainPanelTab: PanelTab;
```

`poiesis/src/store/types.ts` —— `PoiesisState` 接口（`toggleCodeFlow` 签名附近）加：

```ts
  setAwake: (awake: boolean) => void;
  setMainPanelTab: (tab: import("./core/types/design-page").PanelTab) => void;
```

`poiesis/src/store/operations/tab-lifecycle.ts` —— `createInitialDesignPageState`（文件头部，`codeFlowVisible: false` 附近）加：

```ts
    awake: false,
    mainPanelTab: "structure",
```

- [ ] **Step 4: 加动作**

`poiesis/src/store/operations/tab-lifecycle.ts` —— `createTabSlice` 返回对象中、`toggleCodeFlow` 定义旁加：

```ts
    setAwake: (awake: boolean) => set((s) => ({
      designPage: { ...s.designPage, awake },
    })),

    setMainPanelTab: (tab: import("../../core/types/design-page").PanelTab) => set((s) => ({
      designPage: {
        ...s.designPage,
        mainPanelTab: tab,
        // [内容] 标签 = 意图驱动醒（Q12-A）；切回其他标签不睡
        awake: tab === "content" ? true : s.designPage.awake,
      },
    })),
```

- [ ] **Step 5: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/store/__tests__/sleep-awake.test.ts`
Expected: PASS（4 tests）

- [ ] **Step 6: 提交**

```bash
git add poiesis/src/core/types/design-page.ts poiesis/src/store/types.ts poiesis/src/store/operations/tab-lifecycle.ts poiesis/src/store/__tests__/sleep-awake.test.ts
git commit -m "feat(poiesis-stage-v1): 睡/醒状态机——awake + mainPanelTab + [内容] 自动醒（store 单测 4 例）"
```

---

## Task 3: 呼吸点组件

**Files:**
- Create: `poiesis/src/components/canvas/BreathingPoint.tsx`
- Create: `poiesis/src/components/canvas/BreathingPoint.module.css`
- Test: `poiesis/src/components/canvas/__tests__/BreathingPoint.test.tsx`

- [ ] **Step 1: 写失败测试**

```tsx
// poiesis/src/components/canvas/__tests__/BreathingPoint.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { BreathingPoint } from "../BreathingPoint";
import { usePoiesisStore } from "../../store";

describe("BreathingPoint", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false },
    });
  });

  it("点击切换整页醒/睡", () => {
    render(<BreathingPoint />);
    fireEvent.click(screen.getByTestId("breathing-point"));
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
    fireEvent.click(screen.getByTestId("breathing-point"));
    expect(usePoiesisStore.getState().designPage.awake).toBe(false);
  });

  it("醒态带唤醒样式", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true } });
    render(<BreathingPoint />);
    expect(screen.getByTestId("breathing-point").className).toContain("awake");
  });

  it("title 随状态变化", () => {
    render(<BreathingPoint />);
    expect(screen.getByTestId("breathing-point").getAttribute("title")).toContain("醒来");
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/BreathingPoint.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现组件与样式**

```tsx
// poiesis/src/components/canvas/BreathingPoint.tsx
// 呼吸点：设计窗口角落的小脉搏——点 = 整页醒（逻辑层浮现），再点 = 睡（干净画布）。
import { usePoiesisStore } from "../../store";
import styles from "./BreathingPoint.module.css";

export function BreathingPoint() {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const setAwake = usePoiesisStore((s) => s.setAwake);

  return (
    <button
      type="button"
      data-testid="breathing-point"
      className={`${styles.pulse} ${awake ? styles.awake : ""}`}
      title={awake ? "睡回（隐藏逻辑层）" : "醒来（显示逻辑层）"}
      onClick={() => setAwake(!awake)}
    >
      <span className={styles.dot} />
    </button>
  );
}
```

```css
/* poiesis/src/components/canvas/BreathingPoint.module.css */
.pulse {
  position: absolute;
  bottom: 16px;
  left: 16px;
  z-index: 20;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1px solid var(--ol-border);
  background: var(--ol-surface);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
  transition: background 0.15s, border-color 0.15s;
}
.pulse:hover { border-color: var(--ol-color-primary); }
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--ol-text-muted);
  transition: background 0.15s, transform 0.15s;
}
.awake { border-color: var(--ol-color-primary); background: var(--ol-color-primary-subtle); }
.awake .dot {
  background: var(--ol-color-primary);
  transform: scale(1.25);
  animation: breathe 2s ease-in-out infinite;
}
@keyframes breathe {
  0%, 100% { box-shadow: 0 0 0 0 rgba(64, 120, 255, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(64, 120, 255, 0); }
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/BreathingPoint.test.tsx`
Expected: PASS（3 tests）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/BreathingPoint.tsx poiesis/src/components/canvas/BreathingPoint.module.css poiesis/src/components/canvas/__tests__/BreathingPoint.test.tsx
git commit -m "feat(poiesis-stage-v1): 呼吸点——角落小脉搏整页睡/醒（呼吸动画 + 组件测试）"
```

---

## Task 3.5: 设计画布 Shift 多选（醒区输入前置）

> 审查发现（HIGH）：设计画布所有 `selectElement` 调用均为单参（OmniViewRenderer.tsx:55-61 handleSelectNid），Shift 多选只存在于将被删除的 codeFlow 画布（FlowCanvas.tsx:857-868）——"Shift 多选 = 醒区 + 副面板"（T6 双面板、T8 验收的核心路径）当前代码不可能达成。本任务补上画布多选输入。

**Files:**
- Modify: `poiesis/src/components/design/OmniViewElement.tsx`（onSelect 传 Shift 修饰）
- Modify: `poiesis/src/components/design/OmniViewElementTree.tsx`（onSelectNid 签名透传）
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx`（handleSelectNid 传 multi）
- Test: `poiesis/src/components/design/__tests__/OmniViewElementSelect.test.tsx`

- [ ] **Step 1: 写失败测试**

```tsx
// poiesis/src/components/design/__tests__/OmniViewElementSelect.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { OmniViewElement } from "../OmniViewElement";

const node = { nid: "n1", tag: "button", text: "按钮", children: [] } as any;

describe("OmniViewElement 选择（醒区输入）", () => {
  it("Shift+点击 → onSelect 带 multi=true", () => {
    const onSelect = vi.fn();
    render(<OmniViewElement node={node} tag="button" isSelected={false} onSelect={onSelect} onDoubleClick={() => {}} />);
    fireEvent.click(screen.getByText("按钮"), { shiftKey: true });
    expect(onSelect).toHaveBeenCalledWith("n1", true);
  });

  it("普通点击 → multi=false（单选替换醒区）", () => {
    const onSelect = vi.fn();
    render(<OmniViewElement node={node} tag="button" isSelected={false} onSelect={onSelect} onDoubleClick={() => {}} />);
    fireEvent.click(screen.getByText("按钮"));
    expect(onSelect).toHaveBeenCalledWith("n1", false);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/OmniViewElementSelect.test.tsx`
Expected: FAIL（onSelect 仅收到一个参数，断言 `(n1, true)` 失败）

- [ ] **Step 3: 实现**

`OmniViewElement.tsx`：
- `OmniViewElementProps.onSelect` 签名改 `(nid: string, multi: boolean) => void`
- `clickHandlerProps` 的 onClick 改为：`onSelect(effectiveNid, e.shiftKey || e.ctrlKey)`

`OmniViewElementTree.tsx`：
- `OmniViewElementTreeProps.onSelectNid` 签名改 `(nid: string, multi: boolean) => void`，透传给 OmniViewElement 的 onSelect

`OmniViewRenderer.tsx`：
- `handleSelectNid` 改：

```tsx
const handleSelectNid = useCallback(
  (nid: string, multi: boolean) => {
    const elementId = nidToElementId.get(nid) ?? nid;
    selectElement(elementId, multi);
  },
  [selectElement, nidToElementId],
);
```

（`selectElement(id, multi)` 已在 element-crud.ts:124-140 实现——multi 追加/移除、非 multi 替换，复用现有逻辑；画布外其他调用点不受影响。）

- [ ] **Step 4: 跑测试确认通过 + 类型检查**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/OmniViewElementSelect.test.tsx src/components/design/__tests__`
Expected: PASS
Run: `cd D:/space/OmniLang/poiesis && npx tsc --noEmit`
Expected: 0 errors

- [ ] **Step 5: 提交**

```bash
git add -A poiesis/src
git commit -m "feat(poiesis-stage-v1): 设计画布 Shift 多选——onSelect 带 multi 修饰（醒区输入前置，双面板路径依赖）"
```

---

## Task 4: 画布叠加改造（codeFlowVisible → awake 全量替换 + 旧机制删除）

> 本 Task 是"打通"的主体手术：旧的"双击进出 + TopLayer 缩放隐藏 + DesignPreviewNode 快照"互斥机制整体删除，换成"设计层常驻 + 逻辑层叠加（logicLayerVisible 控制浮现）+ 视口同步"。

**Files:**
- Create: `poiesis/src/components/canvas/LogicLayer.tsx` + `.module.css`
- Test: `poiesis/src/components/canvas/__tests__/LogicLayer.test.tsx`
- Modify: `poiesis/src/core/types/design-page.ts`（Breadcrumb 去 code-layer、DesignPageState 删 codeFlow 三字段）
- Modify: `poiesis/src/store/operations/tab-lifecycle.ts`（删 toggleCodeFlow + codeFlow 初始/保存/恢复）
- Modify: `poiesis/src/store/tab-helpers.ts`（快照删 codeFlow 字段）
- Modify: `poiesis/src/pages/DesignPage.tsx`（叠加结构 + logicLayerVisible + 物化触发 + 删 TopLayer/双击/ComponentMembersPanel 用法）
- Modify: `poiesis/src/components/design/useWasmCompile.ts`（删 codeFlowVisible 跳过）
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（删 DesignPreviewNode/previewOutput/codeFlowElementId/退出双击 + 视口同步）
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx`（删双击进逻辑画布）
- Modify: `poiesis/src/components/design/OmniViewElementTree.tsx` + `OmniViewElement.tsx`（onDoubleClickNid 移除）
- Modify: `poiesis/src/components/design/BreadcrumbBar.tsx`（删 code-layer 分支）
- Delete: `poiesis/src/components/canvas/code/nodes/DesignPreviewNode.tsx`（含 css；如有同名测试一并删）
- Modify: `poiesis/src/store/__tests__/editor-helpers.test.ts`、`poiesis/src/components/design/__tests__/useWasmCompile-debug.test.ts`（引用替换）

- [ ] **Step 1: 摸底——列全部受影响引用**

Run: `cd D:/space/OmniLang/poiesis && grep -rn "codeFlowVisible\|toggleCodeFlow\|codeFlowElementId\|codeFlowFocusPoint\|DesignPreviewNode\|design-preview\|previewOutput\|setPreviewOutput\|previewPos\|context-inspector" src --include="*.ts" --include="*.tsx" | grep -v ".module.css"`
Expected: 列出所有消费点（DesignPage、FlowCanvas、OmniViewRenderer、useWasmCompile、tab-lifecycle、tab-helpers、types 等）——**本次 grep 不再排除 __tests__**（原稿排除导致 6 个测试文件的破坏未被摸底发现，见 Step 8）。`previewPos`/`context-inspector` 两个符号是审查补入的盲区项。逐项在后续步骤清掉；测试文件在第 8 步处理。

- [ ] **Step 2: 写 LogicLayer 失败测试**

```tsx
// poiesis/src/components/canvas/__tests__/LogicLayer.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { LogicLayer } from "../LogicLayer";

describe("LogicLayer", () => {
  it("可见时透出内容且可交互", () => {
    render(<LogicLayer visible><div data-testid="inner">fn</div></LogicLayer>);
    const layer = screen.getByTestId("logic-layer");
    expect(layer.style.opacity).toBe("1");
    expect(layer.style.pointerEvents).toBe("auto");
    expect(screen.getByTestId("inner")).toBeTruthy();
  });

  it("隐藏时不可交互且不可见（睡态干净画布）", () => {
    render(<LogicLayer visible={false}><div data-testid="inner">fn</div></LogicLayer>);
    const layer = screen.getByTestId("logic-layer");
    expect(layer.style.visibility).toBe("hidden");
    expect(layer.style.pointerEvents).toBe("none");
  });
});
```

- [ ] **Step 3: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/LogicLayer.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 4: 实现 LogicLayer**

```tsx
// poiesis/src/components/canvas/LogicLayer.tsx
// 逻辑层叠加：醒态（或醒区非空）时浮现的 ReactFlow 逻辑层。
// 睡态隐藏且不拦截事件——睡 = 干净画布；醒着也能改样式（叠加层事件穿透，见 CSS）。
import type { ReactNode } from "react";
import styles from "./LogicLayer.module.css";

export function LogicLayer({ visible, children }: { visible: boolean; children: ReactNode }) {
  return (
    <div
      data-testid="logic-layer"
      className={styles.layer}
      style={{
        opacity: visible ? 1 : 0,
        pointerEvents: visible ? "auto" : "none",
        visibility: visible ? "visible" : "hidden",
      }}
    >
      {children}
    </div>
  );
}
```

```css
/* poiesis/src/components/canvas/LogicLayer.module.css */
.layer {
  position: absolute;
  inset: 0;
  z-index: 1;
  transition: opacity 0.15s ease;
}
/* 叠加层只允许节点/边/控件接收事件——pane 区域事件穿透到设计层（醒着也能改样式） */
.layer :global(.react-flow__pane) {
  pointer-events: none;
}
```

- [ ] **Step 5: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/LogicLayer.test.tsx`
Expected: PASS（2 tests）

- [ ] **Step 6: store 层删除 codeFlow 机制**

`poiesis/src/core/types/design-page.ts`：
- `Breadcrumb` 的 type 联合去掉 `"code-layer"`（留 `"architecture" | "component"`）
- `DesignPageState` 删除 `codeFlowVisible`、`codeFlowElementId`、`codeFlowFocusPoint` 三个字段

`poiesis/src/store/operations/tab-lifecycle.ts`：
- `createInitialDesignPageState` 删 `codeFlowVisible/codeFlowElementId/codeFlowFocusPoint` 三行（16-18）
- `openTab` 新开 tab 的状态重置（119-123）：删 `codeFlowVisible/codeFlowFocusPoint` 两行；**保留 122（previewOutput:null）与 123（breadcrumbs 重置）**——它们是正常重置，不属 codeFlow 机制（原稿行号误标为"关 tab 恢复"；真实关 tab 保存在下一项）
- **`closeTab` 关 tab 的保存入口（150-159）**：删 codeFlow 三字段的保存；`switchTab` 恢复（229-234）同步删
- 整个 `toggleCodeFlow` 动作删除（254-268），同时删其 breadcrumbs `"code-layer"` 写入
- **孤儿 API 一并删除（不留静默通道）**：`setPreviewOutput`（274-276）——previewOutput 读者全删后成只写死状态；`setBreadcrumbs`（270-272）——已零调用者（toggleCodeFlow 内联构造 breadcrumbs），现成死 action；两者连同类型签名删除

`poiesis/src/store/types.ts`：
- `PoiesisState` 删 `toggleCodeFlow`/`setPreviewOutput`/`setBreadcrumbs` 签名
- **注意 `types.ts:64-73` 有第二份内联 tabData 类型定义**，同样携带 `codeFlowVisible?`/`codeFlowElementId?` 字段——与 tab-helpers.ts 同步删（原稿漏了这份重复定义，不删则 tsc 报错或残留不一致）

`poiesis/src/store/tab-helpers.ts`：**4 处硬编码**（类型真名 `TabDataEntry`，非原稿的"TabDataSnapshot"）——`TabDataEntry` 可选字段（16-17）、`TabSaveSource.designPage`（32）、`buildTabSaveEntry`（51-52）、`tabRestoreDesignPagePartial` 返回类型（87-95），全部删 `codeFlowVisible/codeFlowElementId`

`poiesis/src/store/slices/component-lifecycle.ts`：第三个 tab-helpers 消费者（52/65/73 行，展开式 `designPage: s.designPage` + `...codeFlowPartial`）——helper 签名变更后**无需改代码**（语义自动跟随），但改动后必须跑 tsc 确认无类型漂移

- [ ] **Step 7: 组件层替换（逐文件）**

**`poiesis/src/components/design/useWasmCompile.ts`**：
- 删 `codeFlowVisible` 订阅（44 行）与"跳过重编译"块（64-66 行：`// Skip WASM recompilation when CodeFlow is active` + `if (codeFlowVisible) return;`）
- effect 依赖数组删 `codeFlowVisible`

**`poiesis/src/components/canvas/code/FlowCanvas.tsx`**：按 **11 项残留清单**全删（只删机制，不留孤儿——审查确认的完整清单）：
1. `DesignPreviewNode` import 与 `nodeTypes` 的 `"design-preview"` entry（69-87）
2. `DESIGN_PREVIEW_ID` 常量（47）
3. `codeFlowElementId` 订阅（107-109）
4. `previewOutput` 订阅（112）
5. design-preview 节点创建 memo（749-760）与 `allNodes` 合并（762-765）
6. **`previewPos` state（263）与 `setPreviewPos`（handleAutoLayout 739）**——唯一读者是 previewNode memo，删除后只写不读 → tsconfig `noUnusedLocals` 报 TS6133 炸 tsc（原稿漏列，且任何 grep 模式都匹配不到 `previewPos`）
7. 空态块对 `codeFlowElementId` 的引用（838-840）
8. **`ReactFlow key={"flow-" + codeFlowElementId}`（846）**——key 语义决策：新模型下逻辑层同时显示全部 fn 图，无"逐 fn 切换重挂载"场景，key 直接删除（React 按 node id 调和）；若实机发现切 tab 后残留旧画布状态，再改用 fn 图内容指纹作 key
9. `onNodeClick` 对 preview 的 guard 与 Shift 多选对 preview 的 guard（857-868 内相关分支）
10. design-preview 双击退出分支（DesignPreviewNode.tsx:83 同款，若 FlowCanvas 内有）
11. 连带：`previewOutput` 写入链（`OmniViewRenderer.tsx:30-33` 的 setPreviewOutput effect——原稿修改范围只提了删双击，这个写者 effect 必须一并删）
- 顶部加视口同步（`useReactFlow` 已存在）：

```tsx
// ── 视口同步：单一真相 = useCanvasStore（DOM 画布），ReactFlow 跟随；onMove 回写 ──
const { zoom: canvasZoom, panX, panY } = useCanvasStore();
const { setViewport } = useReactFlow();
useEffect(() => {
  setViewport({ x: panX, y: panY, zoom: canvasZoom });
}, [canvasZoom, panX, panY, setViewport]);
```

- `ReactFlow` 组件上加 `onMove`（在现有 props 上追加）——**必须带 Object.is 守卫**：审查确认 `setViewport`（d3 编程式变换，无 internal 标志）**必然触发 onMove**，无守卫时浮点漂移会 2-store ping-pong：

```tsx
onMove={(_e, vp) => {
  const cur = useCanvasStore.getState();
  // setViewport 编程式同步也会触发 onMove——Object.is 守卫防 ping-pong
  // 注意：setZoom 钳制 0.1-5.0 与 ReactFlow zoom 范围须一致，否则钳制后值不等 → 每帧回写
  if (vp.zoom !== cur.zoom || vp.x !== cur.panX || vp.y !== cur.panY) {
    cur.setZoom(vp.zoom);
    cur.setPan(vp.x, vp.y);
  }
}}
```

（DOM canvas 是视口唯一真相：叠加层 `.react-flow__pane` 已 pointer-events none，ReactFlow 收不到用户视口手势——此回写只是防御性兜底。）

- import 加 `useCanvasStore`（`../../canvas-store` 或相对现有 import 风格）

**`poiesis/src/pages/DesignPage.tsx`**：
- 删 `codeFlowVisible`/`codeFlowFocusPoint`/`toggleCodeFlow` 订阅（155-156 行）——**注意：157 行 `storePanelVisible` 订阅必须保留**（它是 StoreDrawer 的唯一开关；原稿"删 155-158 行"会误杀它导致抽屉功能死亡）；`showCodeFlowPanel` 删除，新增：

```tsx
const logicLayerVisible = usePoiesisStore(
  (s) => s.designPage.awake || s.selectedElementIds.length >= 2,
);
```

- 物化触发（172-177 行）改为：

```tsx
useEffect(() => {
  if (logicLayerVisible && activeTabType === "component" && activeTabComponentName) {
    materializeFnGraphs(activeTabComponentName);
  }
  // eslint-disable-next-line react-hooks/exhaustive-deps -- 物化幂等，依赖仅触发时机
}, [logicLayerVisible, activeTabType, activeTabComponentName]);
```

- 中心区（338-372 行）Z 轴双层改为叠加：

```tsx
<div ref={zAxisRef} style={{ flex: 1, position: "relative", overflow: "hidden" }}>
  <InfiniteCanvas>
    <OmniViewRenderer />
  </InfiniteCanvas>
  <LogicLayer visible={logicLayerVisible}>
    <CodeFlowCanvas />
  </LogicLayer>
</div>
```

- 删 `TopLayer` 组件定义（507-547 行）与 import（如单独文件）
- 删 FlowCanvas 空白双击退出块（353-371 行旧结构已随上替换消失）
- 左栏：删 `showCodeFlowPanel` 分支与 `ComponentMembersPanel` import/渲染（296-308 行）；左栏恒为 ComponentPalette + ComponentTree（`showDesignPanels` 改为 `activeTabType === "component" && !storePanelVisible`）
- grid 列（237-243 行）改为：`showDesignPanels ? "280px 1fr 300px" : activeTabType === "code" ? "280px 1fr" : "1fr"`（右栏列 T7 再删）
- 中心 gridColumn（329 行）改：`showDesignPanels ? 2 : activeTabType === "code" ? 2 : "1 / -1"`——**保留 code tab 分支**（code tab 左栏 LibMembersPanel 在列 1，若改 "1 / -1" 中心区会横跨盖住它，原稿丢失该分支）；StoreDrawer gridColumn（400 行）同步同式
- 删 `DesignPage.module.css` 的 `.topLayer` 样式块（TopLayer 删除后的样式死代码）

**`poiesis/src/components/design/OmniViewRenderer.tsx`**：
- 删 `handleDoubleClickNid` 回调（97-103 行）与 `toggleCodeFlow` 订阅/import；`OmniViewElementTree` 不再传 `onDoubleClickNid`

**`poiesis/src/components/design/OmniViewElementTree.tsx` + `OmniViewElement.tsx`**：
- 删 `onDoubleClickNid` prop 及传递链（tree props、element props、`onDoubleClick` handler 84-89 行、`clickHandlerProps` 里的 onDoubleClick）

**`poiesis/src/components/design/BreadcrumbBar.tsx`**：
- 删 `"code-layer"` 类型分支处理（如有渲染返回逻辑）

- [ ] **Step 8: 删 DesignPreviewNode 与旧测试引用**

Run: `rm poiesis/src/components/canvas/code/nodes/DesignPreviewNode.tsx poiesis/src/components/canvas/code/nodes/DesignPreviewNode.module.css`（如存在同名测试一并删）

**6 个测试文件**（原稿只列 2 个——摸底 grep 曾排除 `__tests__` 造成盲区，审查后全量列出）：
1. `src/store/__tests__/editor-helpers.test.ts` — 8 处断言（17、28-29、51、60-61、81-82、124-125、130-131、137-138）读 `entry.codeFlowVisible`/`entry.codeFlowElementId` → 改读 `awake` 或删除
2. `src/components/design/__tests__/useWasmCompile-debug.test.ts:61` — baseline fixture 含 `codeFlowVisible: false`（字段删除后多余属性 → tsc 报错）→ 删字段；其"跳过编译"语义断言一并删除（useWasmCompile 已无跳过逻辑）
3. `src/components/canvas/code/__tests__/FlowCanvasDrag.test.tsx:56-57` — fixture 字段删除；**89-121 design-preview 伪节点行为测试整块删除**（被测概念 DESIGN_PREVIEW_ID 已删）
4. `src/components/canvas/code/__tests__/FlowCanvasGroupSelect.test.tsx:76-90` — DESIGN_PREVIEW_ID 守卫行为测试整块删除
5. `src/components/canvas/code/__tests__/FlowCanvasLayout.test.tsx:33` — fixture 字段删除
6. `src/components/canvas/code/__tests__/FlowCanvasLetVars.test.tsx:38` — fixture 字段删除
**另外：6 个 FlowCanvas 测试（Drag/DragMeasured/Layout/GroupCollapse/GroupSelect/LetVars）的 `useReactFlow` mock 必须补 `setViewport: vi.fn()`**——新增的视口同步 effect 挂载即调用 setViewport，mock 缺它 → TypeError 整文件崩溃（审查确认 mock 行：DragMeasured 31 / Layout 12 / GroupCollapse 26 / GroupSelect 20 / LetVars 15）

- [ ] **Step 9: 全量测试 + 类型检查**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run`
Expected: 全绿。若有失败，多为 Step 1 摸底清单遗漏的引用，逐个清。

Run: `cd D:/space/OmniLang/poiesis && npx tsc --noEmit`
Expected: 0 errors（注意：Step 7 是机械替换，tsc 会立刻暴露遗漏）

- [ ] **Step 10: 提交**

```bash
git add -A poiesis/src
git commit -m "feat(poiesis-stage-v1): 画布叠加——codeFlowVisible/TopLayer/DesignPreviewNode 全删，LogicLayer 叠加 + 视口同步（醒态 fn 图浮在元素之上）"
```

---

## Task 5: 元素端口层（视觉层）

**Files:**
- Modify: `poiesis/src/components/design/OmniViewElement.tsx`（data-omni-el-id）
- Modify: `poiesis/src/components/design/OmniViewElementTree.tsx`（elId 传参）
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx`（挂 ElementPortLayer）
- Create: `poiesis/src/components/canvas/ElementPortLayer.tsx` + `.module.css`
- Test: `poiesis/src/components/canvas/__tests__/ElementPortLayer.test.tsx`

- [ ] **Step 1: 写失败测试（端口位收集工具）**

```tsx
// poiesis/src/components/canvas/__tests__/ElementPortLayer.test.tsx
import { describe, it, expect, vi, afterEach } from "vitest";
import { collectPortPositions } from "../ElementPortLayer";

describe("collectPortPositions", () => {
  afterEach(() => { vi.restoreAllMocks(); });

  it("按 data-omni-el-id 收集元素右缘中点（容器相对坐标 + 滚动补偿）", () => {
    const container = document.createElement("div");
    const el = document.createElement("div");
    el.dataset.omniElId = "el-1";
    container.appendChild(el);

    vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
      left: 10, right: 110, top: 20, bottom: 60,
      width: 100, height: 40, x: 10, y: 20,
      toJSON: () => ({}),
    } as DOMRect);
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
      left: 0, right: 800, top: 0, bottom: 600,
      width: 800, height: 600, x: 0, y: 0,
      toJSON: () => ({}),
    } as DOMRect);

    expect(collectPortPositions(container)).toEqual([{ elId: "el-1", x: 110, y: 40 }]);
  });

  it("滚动 + padding 补偿：scrollLeft≠0 且容器有 padding（审查补入的测试矩阵项）", () => {
    const container = document.createElement("div");
    const el = document.createElement("div");
    el.dataset.omniElId = "el-2";
    container.appendChild(el);
    vi.spyOn(container, "scrollLeft", "get").mockReturnValue(30);
    vi.spyOn(container, "scrollTop", "get").mockReturnValue(12);
    vi.spyOn(window, "getComputedStyle").mockReturnValue({ paddingLeft: "8px", paddingTop: "8px" } as CSSStyleDeclaration);
    vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
      left: 48, right: 148, top: 40, bottom: 80,
      width: 100, height: 40, x: 48, y: 40, toJSON: () => ({}),
    } as DOMRect);
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
      left: 0, right: 800, top: 0, bottom: 600,
      width: 800, height: 600, x: 0, y: 0, toJSON: () => ({}),
    } as DOMRect);
    // 元素内容坐标 = rect - (cRect + padding) + scroll：x = 148 - 0 - 8 + 30 = 170；y = 60 - 0 - 8 + 12 = 64
    expect(collectPortPositions(container)).toEqual([{ elId: "el-2", x: 170, y: 64 }]);
  });

  it("无 data-omni-el-id 的元素被忽略", () => {
    const container = document.createElement("div");
    const plain = document.createElement("div");
    container.appendChild(plain);
    vi.spyOn(plain, "getBoundingClientRect").mockReturnValue({
      left: 0, right: 50, top: 0, bottom: 20, width: 50, height: 20, x: 0, y: 0, toJSON: () => ({}),
    } as DOMRect);
    expect(collectPortPositions(container)).toEqual([]);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/ElementPortLayer.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现端口层**

```tsx
// poiesis/src/components/canvas/ElementPortLayer.tsx
// 元素端口层（视觉）：醒态在元素右缘长出接口圆点（出口位）。
// 本阶段纯视觉、不可交互——接线交互（拖线/点名）M3 启用，形状（圆点/双竖线/菱形）后置。
// 端口随画布变换（本层渲染在设计层变换上下文内）；点位 = 容器相对坐标 + 滚动补偿。
import { useEffect, useLayoutEffect, useState } from "react";
import { usePoiesisStore } from "../../store";
import styles from "./ElementPortLayer.module.css";

export interface PortDot {
  elId: string;
  x: number;
  y: number;
}

const RENDERER_SELECTOR = "[data-renderer='omniview']";

/** 在容器内查找带 data-omni-el-id 的元素，输出右缘中点端口位（容器相对 + 滚动补偿 + padding 补偿）。
 *  挂载契约（钉死，审查修正）：ElementPortLayer 必须是 OmniViewRenderer 外层滚动容器
 *  （overflow:auto + padding:8 + position:relative）的 absolute 子节点——与元素同处内容坐标系，
 *  缩放由外层 transform 统一承担（无需 ÷zoom）；层随容器滚动（-scrollLeft/Top），公式加回
 *  scroll 补偿；元素坐标相对 padding-box 原点，公式减去 paddingLeft/Top。三项缺一即错位。 */
export function collectPortPositions(container: HTMLElement): PortDot[] {
  const dots: PortDot[] = [];
  const cRect = container.getBoundingClientRect();
  const padLeft = parseFloat(getComputedStyle(container).paddingLeft) || 0;
  const padTop = parseFloat(getComputedStyle(container).paddingTop) || 0;
  for (const el of container.querySelectorAll<HTMLElement>("[data-omni-el-id]")) {
    const elId = el.dataset.omniElId;
    if (!elId) continue;
    const rect = el.getBoundingClientRect();
    dots.push({
      elId,
      x: rect.right - cRect.left - padLeft + container.scrollLeft,
      y: rect.top + rect.height / 2 - cRect.top - padTop + container.scrollTop,
    });
  }
  return dots;
}

export function ElementPortLayer({ visible }: { visible: boolean }) {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const [dots, setDots] = useState<PortDot[]>([]);

  // 醒区成员语义（§15.4"线随醒处"）：整页醒 → 全部元素；多选 → 仅选中元素长端口，未选中的睡态元素不长
  const selectedSet = new Set(selectedIds);
  const shown = visible && (awake || selectedSet.size >= 2);

  useLayoutEffect(() => {
    if (!shown) return;
    const measure = () => {
      const container = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
      if (!container) return;
      const next = collectPortPositions(container);
      // 仅集合变化时更新，避免每帧 setState 重渲染
      setDots((prev) =>
        prev.length === next.length && prev.every((d, i) => d.elId === next[i].elId && d.x === next[i].x && d.y === next[i].y)
          ? prev
          : next,
      );
    };
    measure();
    const raf = requestAnimationFrame(measure); // 等 WASM 渲染布局稳定
    const root = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
    const observer = root ? new ResizeObserver(measure) : null;
    observer?.observe(root ?? document.body);
    window.addEventListener("resize", measure);
    return () => {
      cancelAnimationFrame(raf);
      observer?.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [shown, awake, selectedIds]);

  if (!shown) return null;

  return (
    <div className={styles.layer} aria-hidden data-testid="element-ports">
      {dots
        .filter((d) => awake || selectedSet.has(d.elId)) // 成员过滤：多选时只有醒区元素长端口
        .map((d) => (
          <span key={d.elId} className={styles.port} style={{ left: d.x, top: d.y }} title={d.elId} />
        ))}
    </div>
  );
}
```

```css
/* poiesis/src/components/canvas/ElementPortLayer.module.css */
.layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 5;
}
.port {
  position: absolute;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  border: 2px solid var(--ol-surface);
  background: var(--ol-color-primary);
  transform: translate(-50%, -50%);
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.15);
}
```

- [ ] **Step 4: 给渲染元素加 data-omni-el-id**

`poiesis/src/components/design/OmniViewElement.tsx`：
- props 加 `elId?: string | null`
- `React.createElement` 的 props 对象（`"data-nid": node.nid,` 行）加 `"data-omni-el-id": elId ?? undefined`
- 子元素递归调用传 `elId`（子节点从 tree 侧解析，见下）

`poiesis/src/components/design/OmniViewElementTree.tsx`：
- 加 elId 解析（nid_map 可能是 Map）：

```tsx
const elIdMap = useMemo(() =>
  output.nid_map instanceof Map ? output.nid_map : new Map(Object.entries(output.nid_map)),
  [output.nid_map],
);
```

- 每个 `OmniViewElement` 传 `elId={elIdMap.get(node.nid) ?? null}`；子节点递归由 OmniViewElement 内部完成——**注意：OmniViewElement 内部的 children 递归渲染（`node.children.map`）也要能拿到 elIdMap**。改法：把 elIdMap 作为 prop 传给 OmniViewElement（`elIdMap`），子元素用 `elIdMap.get(child.nid)`。具体：tree 传 `elIdMap={elIdMap}`；element 内部 children map 加 `elId={elIdMap.get(child.nid) ?? null}` 与 `elIdMap={elIdMap}`。

- [ ] **Step 5: 挂进渲染器**

`poiesis/src/components/design/OmniViewRenderer.tsx`：在返回 JSX 中（`OmniViewElementTree` 之后、错误徽章之前）加：

```tsx
<ElementPortLayer visible={!!output && errors.length === 0} />
```

import `ElementPortLayer` 自 `../canvas/ElementPortLayer`。

- [ ] **Step 6: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/canvas/__tests__/ElementPortLayer.test.tsx src/components/design/__tests__`
Expected: PASS

Run: `cd D:/space/OmniLang/poiesis && npx tsc --noEmit`
Expected: 0 errors

- [ ] **Step 7: 提交**

```bash
git add -A poiesis/src
git commit -m "feat(poiesis-stage-v1): 元素端口层——醒态元素右缘长出接口圆点（data-omni-el-id + 测量工具单测）"
```

---

## Task 6: 语境面板（浮动主/副面板 + 浮标 + 标签页 + 自动醒）

**Files:**
- Create: `poiesis/src/components/design/panel-position.ts`（纯函数）
- Test: `poiesis/src/components/design/__tests__/panel-position.test.ts`
- Create: `poiesis/src/components/design/ContextPanel.tsx` + `ContextPanel.module.css`
- Create: `poiesis/src/components/design/StructureTab.tsx`
- Create: `poiesis/src/components/design/ContentTab.tsx`
- Create: `poiesis/src/components/design/BehaviorTab.tsx`
- Create: `poiesis/src/components/panels/inspector/shared.ts`（BASIC_TAGS / useSignalItems / useHandlerItems 迁移地）
- Modify: `poiesis/src/components/panels/ContextInspector.tsx`（改用 shared 导入；文件本体 T7 删）
- Modify: `poiesis/src/pages/DesignPage.tsx`（挂 ContextPanel）
- Test: `poiesis/src/components/design/__tests__/ContextPanel.test.tsx`

- [ ] **Step 1: 写失败测试（面板落位工具）**

```ts
// poiesis/src/components/design/__tests__/panel-position.test.ts
import { describe, it, expect } from "vitest";
import { computePanelPlacement } from "../panel-position";

const viewport = { width: 1280, height: 800 };

describe("computePanelPlacement", () => {
  it("右侧放得下 → 右落位，y 贴合元素顶", () => {
    const p = computePanelPlacement({ x: 100, y: 80, width: 200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.side).toBe("right");
    expect(p.x).toBe(100 + 200 + 12);
    expect(p.y).toBe(80);
  });

  it("右侧放不下 → 左落位", () => {
    const p = computePanelPlacement({ x: 1100, y: 80, width: 200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.side).toBe("left");
    expect(p.x).toBe(1100 - 280 - 12);
  });

  it("两侧都放不下 → 元素下方贴合左缘", () => {
    const p = computePanelPlacement({ x: 600, y: 600, width: 1200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.side).toBe("bottom");
    expect(p.y).toBe(600 + 40 + 12);
  });

  it("y 溢出视口 → 钳制", () => {
    const p = computePanelPlacement({ x: 100, y: 700, width: 200, height: 40 }, { width: 280, height: 360 }, viewport);
    expect(p.y).toBe(800 - 360);
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/panel-position.test.ts`
Expected: FAIL（模块不存在）

- [ ] **Step 3: 实现落位工具**

```ts
// poiesis/src/components/design/panel-position.ts
// 语境面板落位：优先元素右侧，放不下换左侧，两侧都不行落元素下方。
export interface Rect { x: number; y: number; width: number; height: number; }
export interface Size { width: number; height: number; }

export interface PanelPlacement {
  x: number;
  y: number;
  side: "right" | "left" | "bottom";
}

const GAP = 12;

function clampY(top: number, panel: Size, viewport: Size): number {
  return Math.max(0, Math.min(top, viewport.height - panel.height));
}

export function computePanelPlacement(
  elementRect: Rect,
  panelSize: Size,
  viewportSize: Size,
): PanelPlacement {
  const right = elementRect.x + elementRect.width + GAP;
  const left = elementRect.x - panelSize.width - GAP;
  if (right + panelSize.width <= viewportSize.width) {
    return { x: right, y: clampY(elementRect.y, panelSize, viewportSize), side: "right" };
  }
  if (left >= 0) {
    return { x: left, y: clampY(elementRect.y, panelSize, viewportSize), side: "left" };
  }
  const bottom = elementRect.y + elementRect.height + GAP;
  return {
    x: Math.max(0, Math.min(elementRect.x, viewportSize.width - panelSize.width)),
    y: Math.min(bottom, viewportSize.height - panelSize.height),
    side: "bottom",
  };
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/panel-position.test.ts`
Expected: PASS（4 tests）

- [ ] **Step 5: 迁移共享块到 inspector/shared.ts**

`poiesis/src/components/panels/inspector/shared.ts` —— 从 ContextInspector.tsx **原样搬移**（不改逻辑）：
- `BASIC_TAGS` 集合（22-45 行）
- `useSignalItems()`（47-147 行）——含 import 调整（SignalPickerItem 类型、store hooks）
- `useHandlerItems` 从 `./inspector/useHandlerItems` re-export（ContextInspector 顶部 `import { useHandlerItems } from "./inspector/useHandlerItems"` → shared.ts 改为 `export { useHandlerItems } from "./useHandlerItems"`）

`ContextInspector.tsx` 顶部改为从 shared 导入这些符号（删除本地定义）；**本文件其余内容不动，T7 整文件删除**。

- [ ] **Step 6: 写失败测试（ContextPanel 骨架）**

```tsx
// poiesis/src/components/design/__tests__/ContextPanel.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { ContextPanel } from "../ContextPanel";
import { usePoiesisStore } from "../../store";

// 面板渲染需要元素存在；用最小元素构造
const el = {
  id: "el-1", nodeType: "view", tag: "button",
  props: { label: "按钮" }, children: [], pid: "p1",
} as any;

describe("ContextPanel", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false, mainPanelTab: "structure" },
      selectedElementIds: [],
      elements: [el],
    });
  });
  afterEach(() => { vi.restoreAllMocks(); });

  it("无选中 → 浮标（不渲染面板）", () => {
    render(<ContextPanel />);
    expect(screen.queryByTestId("context-panel")).toBeNull();
    expect(screen.getByTestId("context-panel-stub")).toBeTruthy();
  });

  it("选中一个 → 主面板（三标签）", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1"] });
    render(<ContextPanel />);
    expect(screen.getByTestId("context-panel")).toBeTruthy();
    expect(screen.getByRole("tab", { name: "结构" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "内容" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "行为" })).toBeTruthy();
  });

  it("点 [内容] → setMainPanelTab('content') → 自动醒", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1"] });
    render(<ContextPanel />);
    fireEvent.click(screen.getByRole("tab", { name: "内容" }));
    expect(usePoiesisStore.getState().designPage.mainPanelTab).toBe("content");
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
  });

  it("选中两个 → 副面板出现且仅 [内容] 标签", () => {
    usePoiesisStore.setState({ selectedElementIds: ["el-1", "el-1"] }); // 复用同一元素，面板只关心 id 集合
    render(<ContextPanel />);
    expect(screen.getAllByTestId("context-panel")).toHaveLength(1); // 两个相同 id 去重？——见实现：面板数 = min(选中数, 2)，按 id 去重
    // 若实现按去重后集合渲染，此用例改为两个不同元素（见 Step 8 注释）
  });
});
```

> 注：最后一条用例依赖"面板数 = 选中 id 去重后 ≤ 2"。真实场景选中集合来自画布（不同元素），测试可用两个不同 id 的元素构造（`selectedElementIds: ["el-1", "el-2"]`，elements 里加 el-2）。实现与测试同步落。

- [ ] **Step 7: 先补 ResizeObserver mock，再跑测试确认失败**

`poiesis/src/test-setup.ts` 末尾追加（jsdom 29 无 ResizeObserver，inspector 组件若内部使用则测试崩——统一空实现，T5/T6 共享）：

```ts
// jsdom 无 ResizeObserver——统一空实现（测量相关测试共享）
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver = ResizeObserverMock;
```

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__/ContextPanel.test.tsx`
Expected: FAIL（ContextPanel 模块不存在）

- [ ] **Step 8: 实现 ContextPanel 与三个标签页**

```tsx
// poiesis/src/components/design/ContextPanel.tsx
// 语境面板（§15.5）：选中元素 → 浮动在主元素旁（跟随/换侧/可拖走）；空选收成小浮标。
// 主面板 = [行为][内容][结构]；副面板 = 仅 [内容]；最多 2 个面板。
// [内容] 标签 = 意图驱动醒（自动醒，store 内实现）。
import { useEffect, useMemo, useRef, useState } from "react";
import { usePoiesisStore } from "../../store";
import { useElements } from "../../store/hooks";
import { computePanelPlacement, type PanelPlacement } from "./panel-position";
import { StructureTab } from "./StructureTab";
import { ContentTab } from "./ContentTab";
import { BehaviorTab } from "./BehaviorTab";
import styles from "./ContextPanel.module.css";

const RENDERER_SELECTOR = "[data-renderer='omniview']";
const PANEL_WIDTH = 280;

interface DraggedPanel { dx: number; dy: number; }

export function ContextPanel() {
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const elements = useElements();
  const mainPanelTab = usePoiesisStore((s) => s.designPage.mainPanelTab);
  const setMainPanelTab = usePoiesisStore((s) => s.setMainPanelTab);
  const [stubOpen, setStubOpen] = useState(false);
  const [placement, setPlacement] = useState<PanelPlacement | null>(null);
  const [secondaryPlacement, setSecondaryPlacement] = useState<PanelPlacement | null>(null);
  const [drag, setDrag] = useState<DraggedPanel | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // 面板集合：主 = 选中[0]，副 = 选中[1]（≤2，按 id 去重）
  const panelIds = useMemo(() => {
    const seen = new Set<string>();
    const out: string[] = [];
    for (const id of selectedIds) {
      if (seen.has(id)) continue;
      seen.add(id);
      out.push(id);
      if (out.length === 2) break;
    }
    return out;
  }, [selectedIds]);

  // 落位跟随：rAF 循环读元素 DOM 矩形（跟随元素移动/布局变化）；拖拽时叠加偏移
  // 审查修正：主面板锚定选中[0]元素、副面板锚定选中[1]元素——各自浮在自己元素旁边（§15.5）
  useEffect(() => {
    if (panelIds.length === 0) { setPlacement(null); setSecondaryPlacement(null); return; }
    let raf = 0;
    const tick = () => {
      const container = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
      // 元素 id 为 uid 生成（字母数字），可安全用于属性选择器
      const measureFor = (elId: string): PanelPlacement | null => {
        const target = container?.querySelector<HTMLElement>(`[data-omni-el-id="${elId}"]`);
        if (!target) return null;
        const rect = target.getBoundingClientRect();
        return computePanelPlacement(
          { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
          { width: PANEL_WIDTH, height: Math.min(rect.bottom - rect.top + 120, 420) },
          { width: window.innerWidth, height: window.innerHeight },
        );
      };
      setPlacement((prev) => measureFor(panelIds[0]) ?? prev);
      if (panelIds[1]) setSecondaryPlacement((prev) => measureFor(panelIds[1]) ?? prev);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [panelIds, selectedIds]);

  // 元素不可见（未渲染/在折叠区）时的兜底：视口右上
  const resolved: PanelPlacement | null = placement ?? (panelIds.length > 0
    ? { x: window.innerWidth - PANEL_WIDTH - 16, y: 64, side: "right" }
    : null);

  // ── 拖拽（头部按住拖动）──
  const onHeaderPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    const startX = e.clientX;
    const startY = e.clientY;
    const onMove = (ev: PointerEvent) => {
      setDrag({ dx: ev.clientX - startX, dy: ev.clientY - startY });
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  if (panelIds.length === 0) {
    // 空选 → 小浮标（应用的面板入口；本阶段点击展开空态面板）
    return (
      <button
        type="button"
        data-testid="context-panel-stub"
        className={styles.stub}
        onClick={() => setStubOpen((v) => !v)}
        title="应用面板"
      >
        面板
        {stubOpen && (
          <span className={styles.stubEmpty} onClick={(e) => e.stopPropagation()}>
            选择元素以编辑属性
          </span>
        )}
      </button>
    );
  }

  const mainId = panelIds[0];
  const secondaryId = panelIds[1];

  return (
    <>
      <div
        ref={panelRef}
        data-testid="context-panel"
        data-panel-for={mainId}
        className={styles.panel}
        style={{
          left: resolved.x + (drag?.dx ?? 0),
          top: resolved.y + (drag?.dy ?? 0),
          zIndex: 30,
        }}
      >
        <div className={styles.header} onPointerDown={onHeaderPointerDown}>
          <span className={styles.title}>元素</span>
          <div className={styles.tabs} role="tablist">
            <button type="button" role="tab" aria-selected={mainPanelTab === "structure"}
              className={mainPanelTab === "structure" ? styles.tabActive : styles.tab}
              onClick={() => setMainPanelTab("structure")}>结构</button>
            <button type="button" role="tab" aria-selected={mainPanelTab === "content"}
              className={mainPanelTab === "content" ? styles.tabActive : styles.tab}
              onClick={() => setMainPanelTab("content")}>内容</button>
            <button type="button" role="tab" aria-selected={mainPanelTab === "behavior"}
              className={mainPanelTab === "behavior" ? styles.tabActive : styles.tab}
              onClick={() => setMainPanelTab("behavior")}>行为</button>
          </div>
        </div>
        <div className={styles.body}>
          {mainPanelTab === "structure" && <StructureTab elementId={mainId} />}
          {mainPanelTab === "content" && <ContentTab elementId={mainId} />}
          {mainPanelTab === "behavior" && <BehaviorTab elementId={mainId} />}
        </div>
      </div>

      {secondaryId && secondaryPlacement && (
        <div
          data-testid="context-panel"
          data-panel-for={secondaryId}
          className={styles.panel}
          style={{
            left: secondaryPlacement.x + (drag?.dx ?? 0),
            top: secondaryPlacement.y + (drag?.dy ?? 0),
            zIndex: 29,
          }}
        >
          <div className={styles.header}>
            <span className={styles.title}>元素（内容）</span>
            <span className={styles.subNote}>仅 [内容]</span>
          </div>
          <div className={styles.body}>
            <ContentTab elementId={secondaryId} />
          </div>
        </div>
      )}
    </>
  );
}
```

```css
/* poiesis/src/components/design/ContextPanel.module.css */
.panel {
  position: fixed;
  width: 280px;
  max-height: 420px;
  display: flex;
  flex-direction: column;
  background: var(--ol-surface);
  border: 1px solid var(--ol-border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.16);
  overflow: hidden;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid var(--ol-border);
  cursor: grab;
  user-select: none;
  background: var(--ol-gray-50);
}
.title { font-size: 12px; font-weight: 600; color: var(--ol-text-secondary); }
.subNote { font-size: 11px; color: var(--ol-text-muted); }
.tabs { display: flex; gap: 2px; }
.tab {
  font-size: 12px;
  padding: 2px 8px;
  border: none;
  background: transparent;
  color: var(--ol-text-secondary);
  border-radius: 4px;
  cursor: pointer;
}
.tab:hover { background: var(--ol-gray-100); }
.tabActive { background: var(--ol-color-primary-subtle); color: var(--ol-color-primary); font-weight: 600; }
.body { overflow: auto; flex: 1; min-height: 0; }
.stub {
  position: fixed;
  right: 16px;
  top: 64px;
  z-index: 30;
  padding: 4px 12px;
  font-size: 12px;
  border: 1px solid var(--ol-border);
  border-radius: 999px;
  background: var(--ol-surface);
  color: var(--ol-text-secondary);
  cursor: pointer;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.1);
}
.stubEmpty {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  width: 180px;
  padding: 12px;
  font-size: 12px;
  color: var(--ol-text-muted);
  background: var(--ol-surface);
  border: 1px solid var(--ol-border);
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
}
```

**`StructureTab.tsx`**（[结构] 标签 = 原 ContextInspector 属性编辑体迁移，删标题栏/Store 按钮）：

```tsx
// poiesis/src/components/design/StructureTab.tsx
// [结构] 标签：原 ContextInspector 的属性编辑体（迁移，不含面板外壳）。
// 事件（[行为]）/绑定（[内容]）分家——EventsSection/BindingsSection 分别在 BehaviorTab/ContentTab。
import { useMemo } from "react";
import { useElements } from "../../store/hooks";
import { BASIC_TAGS, useSignalItems } from "../panels/inspector/shared";
import { PropsSection } from "../panels/inspector/PropsSection";
import { ClassesSection } from "../panels/inspector/ClassesSection";
import { VariantBindingSection } from "../panels/inspector/VariantBindingSection";
import { InlineStylePanel } from "../panels/inspector/InlineStylePanel";
import { ConditionalBlockEditor } from "../panels/inspector/ConditionalBlockEditor";
import { ForBlockEditor } from "../panels/inspector/ForBlockEditor";
import { BranchConditionEditor } from "../panels/inspector/BranchConditionEditor";

export function StructureTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const signalItems = useSignalItems();
  const el = elements.find((e) => e.id === elementId);

  // 分支合成 id：comp.@if.0.br1
  const branchMatch = elementId.match(/^(.+)\.br(\d+)$/);
  const condBlock = branchMatch
    ? elements.find((e) => e.id === branchMatch[1] && e.nodeType === "conditional")
    : null;

  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以编辑属性</div>;

  if (el.nodeType === "for") return <ForBlockEditor blockId={el.id} />;
  if (el.nodeType === "conditional") {
    return <div style={{ padding: 12, overflow: "auto" }}><ConditionalBlockEditor blockId={el.id} /></div>;
  }
  if (branchMatch && condBlock) {
    return <div style={{ padding: 12, overflow: "auto" }}><BranchConditionEditor blockId={branchMatch[1]} branchIndex={parseInt(branchMatch[2], 10)} /></div>;
  }

  const isBasic = "tag" in el && BASIC_TAGS.has(el.tag);
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <h3 style={{ fontSize: 13, fontWeight: 600, margin: "0 0 8px", color: "var(--ol-gray-700)" }}>
        {"tag" in el ? el.tag : "unknown"}
        {"componentRef" in el && el.componentRef ? ` (${el.componentRef})` : ""}
      </h3>
      <PropsSection element={el} signalItems={signalItems} />
      {isBasic && <ClassesSection element={el} />}
      {isBasic && <VariantBindingSection element={el} />}
      {isBasic && <InlineStylePanel element={el} />}
    </div>
  );
}
```

**`ContentTab.tsx`**（[内容] 标签：槽位清单 + 已绑印记 + 现有 BindingsSection 保功能）：

```tsx
// poiesis/src/components/design/ContentTab.tsx
// [内容] 标签 = 选中元素的接口（§15.5）：槽位清单（只读）+ 已绑来源印记；BindingsSection 保现有绑定编辑（M3 换点名绑定）。
import { useElements } from "../../store/hooks";
import { useSignalItems } from "../panels/inspector/shared";
import { BindingsSection } from "../panels/inspector/BindingsSection";

// 按元素类型计算接口槽位（M1 只读展示；形状/接线交互 M3）
function slotsFor(el: { tag?: string; nodeType?: string }): Array<{ key: string; label: string; kind: "文本" | "输入" | "列表" | "条件" | "属性" }> {
  if (el.nodeType === "for") return [{ key: "list", label: "列表内容", kind: "列表" }];
  if (el.nodeType === "conditional") return [{ key: "if", label: "显示条件", kind: "条件" }];
  const tag = el.tag ?? "";
  if (["input", "textarea", "select"].includes(tag)) return [{ key: "value", label: "值", kind: "输入" }];
  if (tag === "img") return [{ key: "src", label: "图片地址", kind: "属性" }];
  if (tag === "a") return [{ key: "href", label: "跳转地址", kind: "属性" }];
  return [{ key: "text", label: "文本内容", kind: "文本" }];
}

export function ContentTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const signalItems = useSignalItems();
  const el = elements.find((e) => e.id === elementId);
  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以查看接口</div>;

  const slots = slotsFor(el);
  // 来源印记展示（spec M1 明列，审查修正）：已绑槽位显示"已绑定"徽章——来源名徽章
  // （"来自 X"）与线显示在 M3 同做，M1 以存在性标记 + BindingsSection 实际绑定内容兑现。
  // 绑定字段形状以 ViewElementData.bindings 实际定义为准（实现时按真实字段读取）。
  const bindings = ("bindings" in el && el.bindings) as Record<string, unknown> | undefined;
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <h3 style={{ fontSize: 13, fontWeight: 600, margin: "0 0 8px", color: "var(--ol-gray-700)" }}>接口</h3>
      {slots.map((s) => {
        const bound = bindings != null && Object.keys(bindings).some((k) => k === s.key || k.includes(s.key));
        return (
          <div key={s.key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "6px 8px", borderBottom: "1px solid var(--ol-border)" }}>
            <span style={{ fontSize: 12 }}>{s.label}</span>
            <span style={{ fontSize: 11, display: "flex", gap: 6, alignItems: "center" }}>
              {bound && (
                <em data-testid={`slot-bound-${s.key}`} style={{ fontStyle: "normal", color: "var(--ol-color-primary)", background: "var(--ol-color-primary-subtle)", borderRadius: 999, padding: "1px 8px" }}>
                  已绑定
                </em>
              )}
              {s.kind}
            </span>
          </div>
        );
      })}
      <div style={{ marginTop: 12, fontSize: 11, color: "var(--ol-text-muted)" }}>
        绑定编辑（M3 换点名绑定 UI；现有编辑暂沿用）
      </div>
      {"tag" in el && <BindingsSection element={el} signalItems={signalItems} />}
    </div>
  );
}
```

**`BehaviorTab.tsx`**（[行为] 标签：现有 EventsSection 保功能，M3 换行为卡）：

```tsx
// poiesis/src/components/design/BehaviorTab.tsx
// [行为] 标签：现有事件编辑（EventsSection）保功能；M3 换行为卡（当[点击]→[把][计数器][加1]）。
import { useElements } from "../../store/hooks";
import { useHandlerItems } from "../panels/inspector/shared";
import { EventsSection } from "../panels/inspector/EventsSection";

export function BehaviorTab({ elementId }: { elementId: string }) {
  const elements = useElements();
  const handlerItems = useHandlerItems();
  const el = elements.find((e) => e.id === elementId);
  if (!el) return <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13 }}>选择元素以编辑行为</div>;
  if (!("tag" in el)) return null;
  return (
    <div style={{ padding: 12, overflow: "auto", boxSizing: "border-box" }}>
      <EventsSection element={el} handlerItems={handlerItems} />
    </div>
  );
}
```

- [ ] **Step 9: 挂进 DesignPage + 补测试用例**

`poiesis/src/pages/DesignPage.tsx`：中心区（`</LogicLayer>` 之后、`</div>` 之前）加 `<ContextPanel />`；import。

Step 6 测试最后一条用例补全（两个不同元素）：

```tsx
it("选中两个 → 主面板 + 副面板（副仅 [内容]）", () => {
  usePoiesisStore.setState({
    selectedElementIds: ["el-1", "el-2"],
    elements: [el, { ...el, id: "el-2" }],
  });
  render(<ContextPanel />);
  const panels = screen.getAllByTestId("context-panel");
  expect(panels).toHaveLength(2);
  expect(screen.getAllByRole("tab")).toHaveLength(3); // 主面板三标签；副面板无标签
});
```

- [ ] **Step 10: 跑测试确认通过**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run src/components/design/__tests__ src/components/panels/__tests__`
Expected: PASS

Run: `cd D:/space/OmniLang/poiesis && npx tsc --noEmit`
Expected: 0 errors

- [ ] **Step 11: 提交**

```bash
git add -A poiesis/src
git commit -m "feat(poiesis-stage-v1): 语境面板——浮动主/副面板（≤2）+ 浮标 + [内容] 自动醒 + 标签分家（结构/内容/行为）"
```

---

## Task 7: 删右栏 ContextInspector + 网格调整 + Store 按钮搬家 + 全量回归

**Files:**
- Modify: `poiesis/src/pages/DesignPage.tsx`（grid 三列→两列；删 ContextInspector 渲染；Store 按钮进工具栏）
- Delete: `poiesis/src/components/panels/ContextInspector.tsx` + `ContextInspector.module.css`
- Modify: `poiesis/src/components/design/DesignToolbar.tsx`（如有 children 插槽直接放按钮；无则加）

- [ ] **Step 1: 删右栏渲染与网格列**

`DesignPage.tsx`：
- 删 `ContextInspector` import 与渲染块（381-394 行）
- grid（237-243 行）改：`showDesignPanels ? "280px 1fr" : activeTabType === "code" ? "280px 1fr" : "1fr"`
- 中心 gridColumn（329 行）与 StoreDrawer gridColumn（400 行）改：`showDesignPanels ? 2 : "1 / -1"`

- [ ] **Step 2: Store 按钮搬家（原在 ContextInspector 标题栏）**

`DesignPage.tsx` 的 `DesignToolbar` children（266 行 `<DebugWindowButton />` 旁）加：

```tsx
<StorePanelToggleButton />
```

新建 `poiesis/src/components/design/StorePanelToggleButton.tsx`：

```tsx
// poiesis/src/components/design/StorePanelToggleButton.tsx
// 📦 Store 抽屉开关（原 ContextInspector 标题栏按钮，右栏删除后搬入工具栏）。
import { usePoiesisStore } from "../../store";

export function StorePanelToggleButton() {
  const visible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const toggle = usePoiesisStore((s) => s.setStorePanelVisible);
  return (
    <button
      type="button"
      data-testid="store-toggle"
      title={visible ? "关闭 Store 面板" : "打开 Store 面板"}
      onClick={() => toggle(!visible)}
      style={{
        border: visible ? "1px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
        background: visible ? "var(--ol-color-primary-subtle)" : "transparent",
        color: visible ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
        borderRadius: 4,
        padding: "2px 8px",
        fontSize: 12,
        cursor: "pointer",
      }}
    >
      📦 Store
    </button>
  );
}
```

- [ ] **Step 3: 删 ContextInspector 与 ComponentMembersPanel 文件**

Run: `rm poiesis/src/components/panels/ContextInspector.tsx poiesis/src/components/panels/ContextInspector.module.css poiesis/src/components/design/ComponentMembersPanel.tsx poiesis/src/components/design/ComponentMembersPanel.css`

先确认无残留引用：`cd D:/space/OmniLang/poiesis && grep -rn "ContextInspector\|ComponentMembersPanel" src | grep -v __tests__`（应为空；ContextInspector 共享块已在 T6 迁走、用法已在 T4 移除，ComponentMembersPanel 用法已在 T4 移除——两者成死文件，审查要求删文件不留残）

- [ ] **Step 4: 全量回归**

Run: `cd D:/space/OmniLang/poiesis && npx vitest run`
Expected: 全绿

Run: `cd D:/space/OmniLang/poiesis && npx tsc --noEmit`
Expected: 0 errors

Run: `cd D:/space/OmniLang/poiesis && npx vitest run --coverage 2>/dev/null || echo "coverage 跳过（无配置则手工核对新增文件覆盖率）"`
（如仓库未配 coverage，手工确认：新增文件均有对应测试：viewport-sync ✅ / sleep-awake ✅ / BreathingPoint ✅ / LogicLayer ✅ / ElementPortLayer(collectPortPositions) ✅ / panel-position ✅ / ContextPanel ✅ / StructureTab/ContentTab/BehaviorTab 经 ContextPanel 用例覆盖）

- [ ] **Step 5: 提交**

```bash
git add -A poiesis/src
git commit -m "feat(poiesis-stage-v1): 删右栏 ContextInspector——网格两列 + Store 按钮搬工具栏 + 全量回归绿"
```

---

## Task 8: 实机验证剧本（Victauri，M1 里程碑验收）

> 前置：`npm run tauri dev`（或项目既有启动方式）跑起 app，Victauri MCP 已连接（`get_plugin_info` 确认 app.identifier 正确）。
> 本 Task 不进 CI，验收以清单勾选为准；发现问题回到对应 Task 修。

- [ ] **Step 1: 启动与基线**

Run: `get_plugin_info` → 确认连接的是 Poiesis
Run: `dom_snapshot` → 确认进入组件 tab，画布渲染（[data-renderer="omniview"] 存在）

- [ ] **Step 2: 呼吸点睡/醒**

- 定位呼吸点：`find_elements` css `[data-testid="breathing-point"]` → 存在且睡态样式
- `interact click` → `eval_js` `document.querySelector("[data-testid='logic-layer']").style.opacity` → "1"（醒）
- 再点 → opacity "0"、visibility hidden（睡）
- 断言 store 态：`eval_js` 不可直接读 zustand → 用 DOM 断言（样式/类名）即可

- [ ] **Step 3: 醒态 fn 节点浮现 + 视口同步**

- 有 fn 的项目：醒后 `dom_snapshot` → `.react-flow__node` 可见（fn 节点卡片）
- 缩放/平移设计层（Ctrl+wheel / 背景滚轮）→ fn 节点随动不脱节（截图对比 `screenshot`）
- 反向：拖 ReactFlow 空白（pane 已 pointer-events none——应穿透到设计层，用设计层滚轮验证同步）

- [ ] **Step 4: 元素端口**

- 醒后 `find_elements` css `[data-testid="element-ports"] span` → 数量 = 可见元素数；点位贴元素右缘（`get_bounding_boxes` 抽查 1-2 个与元素矩形右缘中点吻合，容差 ±4px）

- [ ] **Step 5: 语境面板**

- 睡态选中元素 → `find_elements` `[data-testid="context-panel"]` 出现，三标签（结构/内容/行为）
- 点 [内容] → 面板 tab 激活 + `eval_js` logic-layer opacity "1"（自动醒 ✅）
- 空选（点画布空白）→ 面板消失、`[data-testid="context-panel-stub"]` 出现
- Shift 多选两个元素 → 两个面板（主全标签 + 副仅 [内容]）；logic-layer opacity "1"（醒区）
- 面板跟随：拖动元素（或改布局）→ 面板位置跟随（截图对比前后）
- 右栏（原 300px 列）已消失：dom_snapshot 确认 grid 两列结构（**注意**：组件内部类名 `context-inspector` 仍会出现在 ForBlockEditor 等内部组件上——断言"右栏区域不存在"（grid 结构），不看类名，否则假失败）

- [ ] **Step 6: 不倒退检查**

- 睡态：直接操纵（选中/拖动/双击改字/属性编辑）与改造前一致
- 醒态：选中元素 → [结构] 标签改属性 → 画布当场变（醒着也能改样式 ✅）
- Store 抽屉：工具栏 📦 按钮 → 抽屉照常开合
- 保存 → 重开 → 元素/样式原样（往返闭环无回归）

- [ ] **Step 7: 验收结论**

在 `.scratch/poiesis-stage-v1/` 追加验收记录（`Status:` 行更新为 `ready-for-human` 或按 M2 计划推进）；发现缺陷 → 记 issue（`.scratch/poiesis-stage-v1/issues/NN-*.md`）或回修对应 Task。

---

## Self-Review 对照

**Spec 覆盖（M1 段）**：睡/醒状态机 → T2；呼吸点 → T3；**设计画布 Shift 多选（醒区输入前置）→ T3.5**；画布叠加 + 视口同步 + 删旧机制 → T4；语境面板（主/副/浮标/标签/[内容] 自动醒/≤2）→ T6；删右栏 → T7；元素端口视觉层 → T5；undo 兼容（新状态不入 undo、绑定 M3 才入）→ T2 决策注明；保存往返不变 → T4 不动 emitter/parser + T8 Step 6 验证。**来源印记分两半**：既有绑定的印记展示 = M1（T6 ContentTab"已绑定"徽章，spec M1 明列）；新绑定完成后的徽章/亮线收尾 = M3（spec §17.7）——原稿误引"spec §17.10 遗留"（§17.10 待定 #16 无印记项），已改正。§17.8 第五删（双击 CodeCardEditor）经审查确认代码库现状已达成（FlowCanvas 546-547 行 fn/while/for-in 节点 onDoubleClick 已 undefined，画布内联编辑），无动作项。

**占位符扫描**：无 TBD/TODO；所有代码步骤含完整实现；迁移块（shared.ts）以"原样搬移"精确指令给出。

**类型一致性**：`PanelTab`（T2 定义）在 T6 三处使用一致；`logicLayerVisible`（T4 定义）T4/T5 使用一致；`computePanelPlacement` 签名（T6 Step 3）与测试/ContextPanel 调用一致；`collectPortPositions` 返回 `PortDot[]` 与测试一致。

**已知边界（如实声明，含审查修正）**：
- 视口映射假定两引擎语义一致（单测锁定）；若实机偏移，修 `viewport-sync.ts` 换算（文件头注释已声明），不动消费方
- 元素 id 进 CSS 属性选择器（uid 字母数字，安全）；若未来 id 格式变化需 `CSS.escape`
- ContextPanel 副面板锚定自身元素（T6 已修）；**面板间连线（Q15）归 M3 拖线交互规格，落位跟随归 M1**——两者不再混谈
- 端口层测量用 rAF + ResizeObserver，极端频繁布局下可能丢帧——M1 可接受，实机验证关注
- **awake 为全局会话状态**：code/store/design tab 分支不受 LogicLayer 影响（LogicLayer 只渲染在 component tab 分支内）；跨 tab 切换 awake 保留（瞬态不持久化）
- **T4→T7 中间提交态**：showDesignPanels 公式变更后、T7 删右栏前，睡态右栏 ContextInspector 短暂保留（三列 grid）——过渡行为，T7 收口
- **T5 端口层挂载契约（钉死）**：ElementPortLayer = OmniViewRenderer 外层滚动容器的 absolute 子节点，公式含 padding 与滚动补偿，测试矩阵覆盖 scrollLeft≠0 / padding 偏移（zoom 由外层 transform 承担，公式不含 zoom）
