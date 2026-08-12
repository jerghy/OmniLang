// poiesis/src/components/canvas/__tests__/PortHandleLayer.test.tsx
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, afterEach, vi } from "vitest";
import { PortHandleLayer, resolveSlots, edgeIsActive } from "../PortHandleLayer";
import { usePoiesisStore } from "../../../store";
import type { WireData } from "../../../core/types/wire";

// 端口测量 mock：jsdom 无真实渲染布局，直接喂两端口（el-a/el-b）
vi.mock("../ElementPortLayer", () => ({
  collectPortPositions: vi.fn(() => [
    { elId: "el-a", x: 10, y: 10 },
    { elId: "el-b", x: 200, y: 10 },
  ]),
}));

const setAwake = (awake: boolean) =>
  usePoiesisStore.setState((s) => ({ designPage: { ...s.designPage, awake } }));

const setState = (wires: WireData[], selectedElementIds: string[]) =>
  usePoiesisStore.setState({ wires, selectedElementIds });

// ── ReactFlow v12 jsdom 渲染环境补齐（仅测试侧，非产品代码）──────────────
// 边渲染链：useResizeObserver（useState 初始化器内 new ResizeObserver）→ RO 回调 →
// updateNodeInternals（读 data-id / offsetWidth / DOMMatrixReadOnly.m22）→ handleBounds+measured →
// isNodeInitialized 通过 → EdgeWrapper 渲染 rf__edge-<id>。jsdom 三缺：
//   1. RO 永不开火 → 换「observe 即触发」桩（useNodeObserver 仅 !isInitialized 时 observe，循环安全）；
//   2. offsetWidth/offsetHeight 恒 0 → 桩 100/40（getDimensions：节点/容器/手柄共用）；
//   3. 无 DOMMatrixReadOnly（system 仅解构 m22）→ DummyMatrix。
class FiringResizeObserver {
  private cb: ResizeObserverCallback;
  constructor(cb: ResizeObserverCallback) {
    this.cb = cb;
  }
  observe(target: Element) {
    this.cb([{ target } as ResizeObserverEntry], this as unknown as ResizeObserver);
  }
  unobserve() {}
  disconnect() {}
}

class DummyMatrix {
  m22 = 1;
  constructor(_transform?: string) {}
}

const originalResizeObserver = window.ResizeObserver; // test-setup 的 no-op mock
const originalDOMMatrixReadOnly = window.DOMMatrixReadOnly;
const originalOffsetWidth = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetWidth");
const originalOffsetHeight = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "offsetHeight");

function stubLayoutEnv() {
  window.ResizeObserver = FiringResizeObserver as unknown as typeof ResizeObserver;
  window.DOMMatrixReadOnly = DummyMatrix as unknown as typeof DOMMatrixReadOnly;
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get: () => 100 });
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get: () => 40 });
}

function restoreLayoutEnv() {
  window.ResizeObserver = originalResizeObserver;
  if (originalDOMMatrixReadOnly) {
    window.DOMMatrixReadOnly = originalDOMMatrixReadOnly;
  } else {
    delete (window as unknown as Record<string, unknown>).DOMMatrixReadOnly;
  }
  if (originalOffsetWidth) Object.defineProperty(HTMLElement.prototype, "offsetWidth", originalOffsetWidth);
  else delete (HTMLElement.prototype as unknown as Record<string, unknown>).offsetWidth;
  if (originalOffsetHeight) Object.defineProperty(HTMLElement.prototype, "offsetHeight", originalOffsetHeight);
  else delete (HTMLElement.prototype as unknown as Record<string, unknown>).offsetHeight;
}

// 假渲染器根：PortHandleLayer 测量入口 `[data-renderer='omniview']` + 元素标签取槽语义
function mountRendererRoot() {
  const root = document.createElement("div");
  root.setAttribute("data-renderer", "omniview");
  root.innerHTML = `
    <button data-omni-el-id="el-a">A</button>
    <div data-omni-el-id="el-b">B</div>
  `;
  document.body.appendChild(root);
  return root;
}

const WIRE: WireData = { id: "w1", fromId: "el-a", fromPort: "label", toId: "el-b", toPort: "label" };

describe("PortHandleLayer", () => {
  afterEach(() => {
    setAwake(false);
    setState([], []);
    restoreLayoutEnv();
    document.querySelectorAll("[data-renderer='omniview']").forEach((n) => n.remove());
  });

  it("醒态可见：渲染手柄层（ReactFlow 可拖手柄挂载）", () => {
    setAwake(true);
    render(<PortHandleLayer visible />);
    expect(screen.getByTestId("port-handle-layer")).toBeTruthy();
  });

  it("睡态隐藏：返回 null（睡态干净画布，无手柄）", () => {
    setAwake(false);
    const { container } = render(<PortHandleLayer visible />);
    expect(container.querySelector('[data-testid="port-handle-layer"]')).toBeNull();
  });

  it("visible=false 同样返回 null（编译错误态）", () => {
    setAwake(true);
    const { container } = render(<PortHandleLayer visible={false} />);
    expect(container.querySelector('[data-testid="port-handle-layer"]')).toBeNull();
  });

  it("选中元素端点 → 相关自动连线边亮起（port-edge-active 类 + 加粗描边）", async () => {
    setAwake(true);
    setState([WIRE], ["el-a"]);
    mountRendererRoot();
    stubLayoutEnv(); // RO 开火 + 尺寸桩 + DOMMatrixReadOnly 桩 → 边真渲染
    render(<PortHandleLayer visible />);
    const edge = await screen.findByTestId("rf__edge-w1");
    expect(edge.classList.contains("port-edge-active")).toBe(true);
    const path = edge.querySelector<HTMLElement>(".react-flow__edge-path");
    expect(path?.style.strokeWidth).toBe("2.5");
  });

  it("未选中 → 边保持常规样式（无 port-edge-active 类）", async () => {
    setAwake(true);
    setState([WIRE], []);
    mountRendererRoot();
    stubLayoutEnv();
    render(<PortHandleLayer visible />);
    const edge = await screen.findByTestId("rf__edge-w1");
    expect(edge.classList.contains("port-edge-active")).toBe(false);
    const path = edge.querySelector<HTMLElement>(".react-flow__edge-path");
    expect(path?.style.strokeWidth).toBe("1.5");
  });

  it("悬停元素 → 相关边亮起（元素级事件委托驱动；离开 → 复原）", async () => {
    setAwake(true);
    setState([WIRE], []);
    mountRendererRoot();
    stubLayoutEnv();
    render(<PortHandleLayer visible />);
    const edge = await screen.findByTestId("rf__edge-w1");
    expect(edge.classList.contains("port-edge-active")).toBe(false);
    // 悬停 el-a（渲染元素，非边自身——原生 .react-flow__edge:hover 永不触发）
    fireEvent.mouseOver(document.querySelector<HTMLElement>('[data-omni-el-id="el-a"]')!);
    await waitFor(() => {
      expect(screen.getByTestId("rf__edge-w1").classList.contains("port-edge-active")).toBe(true);
    });
    // 离开（relatedTarget 落空）→ 复原常规样式
    fireEvent.mouseOut(document.querySelector<HTMLElement>('[data-omni-el-id="el-a"]')!);
    await waitFor(() => {
      expect(screen.getByTestId("rf__edge-w1").classList.contains("port-edge-active")).toBe(false);
    });
  });
});

describe("edgeIsActive（亮线判定纯函数：选中 ∪ 悬停）", () => {
  it("任一端被选中 → true（源端 / 目标端 / 双端）", () => {
    expect(edgeIsActive(WIRE, ["el-a"], null)).toBe(true);
    expect(edgeIsActive(WIRE, ["el-b"], null)).toBe(true);
    expect(edgeIsActive(WIRE, ["el-a", "el-b"], null)).toBe(true);
  });

  it("悬停端命中 → true；悬停无关元素或未悬停 → false", () => {
    expect(edgeIsActive(WIRE, [], "el-a")).toBe(true);
    expect(edgeIsActive(WIRE, [], "el-b")).toBe(true);
    expect(edgeIsActive(WIRE, [], "el-other")).toBe(false);
    expect(edgeIsActive(WIRE, [], null)).toBe(false);
    expect(edgeIsActive(WIRE, [], "")).toBe(false);
  });

  it("选中与悬停取并集：悬停无关元素但选中任一端 → 仍 true", () => {
    expect(edgeIsActive(WIRE, ["el-a"], "el-other")).toBe(true);
  });
});

describe("resolveSlots（C1 回归：槽解析以测量节点为准）", () => {
  it("节点命中优先：targetHandle 携带对侧（-src）后缀也返回节点槽（不剥成垃圾槽）", () => {
    const nodes = [
      { id: "A", data: { slotKey: "label" } },
      { id: "B", data: { slotKey: "href" } },
    ];
    // C1 场景：端口中心 elementFromPoint 恒命中 source handle → targetHandle 错为 "label-src"
    expect(resolveSlots({ source: "A", target: "B", sourceHandle: "label-src", targetHandle: "label-src" }, nodes)).toEqual({
      sourceSlot: "label",
      targetSlot: "href",
    });
  });

  it("节点未命中：handle-id 后缀剥除兜底（-src/-tgt 双后缀均剥）", () => {
    expect(resolveSlots({ source: "A", target: "B", sourceHandle: "value-src", targetHandle: "img-tgt" }, [])).toEqual({
      sourceSlot: "value",
      targetSlot: "img",
    });
  });

  it("无 handle 且节点未命中 → null（调用方早退，不落垃圾槽）", () => {
    expect(resolveSlots({ source: "A", target: "B", sourceHandle: null, targetHandle: null }, [])).toEqual({
      sourceSlot: null,
      targetSlot: null,
    });
  });
});
