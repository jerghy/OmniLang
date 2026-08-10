// 折叠态：成员节点从 nodes 消失、edges 重路由到组节点（mock @xyflow/react 捕获 props）
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor, act } from "@testing-library/react";
import type { ReactNode } from "react";

const captured = {
  nodes: [] as unknown[],
  edges: [] as unknown[],
  onNodesChange: null as null | ((changes: unknown[]) => void),
};
vi.mock("@xyflow/react", () => ({
  ReactFlow: (props: Record<string, unknown>) => {
    captured.nodes = props.nodes as unknown[];
    captured.edges = props.edges as unknown[];
    captured.onNodesChange = props.onNodesChange as never;
    return <div data-testid="rf-mock" />;
  },
  ReactFlowProvider: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  // 真实 xyflow Handle 渲染 data-handleid={id}；mock 转发 props 以支持 data-handleid 断言
  Handle: ({ id, ...rest }: { id?: string } & Record<string, unknown>) => <div data-handleid={id} {...rest} />,
  Controls: () => <div />,
  Background: () => <div />,
  BackgroundVariant: { Dots: "dots" },
  // GroupBox 折叠卡 Handle 用到 Position（Left/Right 钉缘）；mock 缺失该导出会在渲染期抛错
  Position: { Left: "left", Right: "right" },
  useReactFlow: () => ({ screenToFlowPosition: (p: { x: number; y: number }) => p }),
}));

import { FlowCanvas } from "../FlowCanvas";
import { GroupBox } from "../nodes/GroupBox";
import { usePoiesisStore } from "../../../../store";

const codeEl = (id: string, tag: string, name?: string) =>
  ({ id, nodeType: "codeblock" as const, tag, parentId: null, source: "", props: name ? { name } : {}, position: { x: 0, y: 0 } });

const reset = () => usePoiesisStore.setState({
  elements: [
    codeEl("fn1", "fn", "increment"),
    codeEl("cb1", "let", "count"),
    codeEl("cb2", "assign"),
    codeEl("cb3", "call", "close"),
  ],
  wires: [
    { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cb1", toPort: "flow-in" },
    { id: "w2", fromId: "cb1", fromPort: "flow-out", toId: "cb2", toPort: "flow-in" },
    { id: "w3", fromId: "cb2", fromPort: "flow-out", toId: "cb3", toPort: "flow-in" },
  ],
  codeFlowGraphs: {}, componentDefs: {}, persistedPositions: {}, tabData: {},
  codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true }] },
});

describe("FlowCanvas 折叠组", () => {
  beforeEach(() => { reset(); vi.clearAllMocks(); });

  it("折叠组：成员节点 cb1/cb2 不在渲染 nodes，组节点 g1 存在", () => {
    render(<FlowCanvas />);
    const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(ids).toContain("g1");
    expect(ids).not.toContain("cb1");
    expect(ids).not.toContain("cb2");
    expect(ids).toContain("cb3"); // 组外节点保留
  });

  it("折叠组：edges 重路由（fn1→g1、g1→cb3），w2 组内连线隐藏", () => {
    render(<FlowCanvas />);
    const edges = captured.edges as Array<{ id: string; source: string; target: string }>;
    expect(edges.find((e) => e.id === "w1")).toMatchObject({ source: "fn1", target: "g1" });
    expect(edges.find((e) => e.id === "w3")).toMatchObject({ source: "g1", target: "cb3" });
    expect(edges.find((e) => e.id === "w2")).toBeUndefined();
  });

  it("展开（collapsed:false）→ 成员与原始 edges 恢复", () => {
    usePoiesisStore.setState({
      codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }] },
    });
    render(<FlowCanvas />);
    const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
    expect(ids).toContain("cb1");
    const edges = captured.edges as Array<{ id: string }>;
    expect(edges.map((e) => e.id).sort()).toEqual(["w1", "w2", "w3"]);
  });

  it("折叠组卡 draggable=true（spec §4 组卡可整体拖动）", () => {
    render(<FlowCanvas />);
    const g = (captured.nodes as Array<{ id: string; draggable?: boolean }>).find((n) => n.id === "g1");
    expect(g?.draggable).toBe(true);
  });

  it("展开组 draggable=false（框跟随成员，不整体拖动）", async () => {
    // 展开态组节点渲染依赖 DOM 测量（bbox）；mock 测量返回成员尺寸
    const origQS = document.querySelector.bind(document);
    const memberMock = (w: number, h: number) => ({ getBoundingClientRect: () => ({ width: w, height: h }) }) as unknown as Element;
    vi.spyOn(document, "querySelector").mockImplementation((sel: string) => {
      if (sel === ".react-flow__viewport") return memberMock(0, 0);
      if (sel === '[data-id="cb1"]' || sel === '[data-id="cb2"]') return memberMock(200, 100);
      return origQS.call(document, sel);
    });
    vi.spyOn(window, "getComputedStyle").mockImplementation(() => ({ transform: "matrix(1, 0, 0, 1, 0, 0)" }) as CSSStyleDeclaration);
    const fallbackRaf: typeof requestAnimationFrame = (cb) => setTimeout(() => cb(performance.now()), 16) as unknown as number;
    const raf = typeof requestAnimationFrame === "function" ? requestAnimationFrame : fallbackRaf;
    const fallbackCaf: typeof cancelAnimationFrame = (id) => clearTimeout(id as unknown as ReturnType<typeof setTimeout>);
    const caf = typeof cancelAnimationFrame === "function" ? cancelAnimationFrame : fallbackCaf;
    vi.stubGlobal("requestAnimationFrame", raf);
    vi.stubGlobal("cancelAnimationFrame", caf);
    try {
      usePoiesisStore.setState({
        codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }] },
      });
      render(<FlowCanvas />);
      await waitFor(() => {
        const g = (captured.nodes as Array<{ id: string; draggable?: boolean }>).find((n) => n.id === "g1");
        expect(g?.draggable).toBe(false);
      });
    } finally {
      vi.restoreAllMocks();
      vi.unstubAllGlobals();
    }
  });

  it("折叠组卡拖动：position change 写回组节点（不触碰 elements）", () => {
    const spy = vi.spyOn(usePoiesisStore.getState(), "setElementPosition");
    try {
      render(<FlowCanvas />);
      act(() => {
        captured.onNodesChange!([{ type: "position", id: "g1", position: { x: 123, y: 456 } }]);
      });
      const after = (captured.nodes as Array<{ id: string; position: { x: number; y: number } }>).find((n) => n.id === "g1");
      expect(spy).not.toHaveBeenCalledWith("g1", expect.anything());
      expect(after?.position).toEqual({ x: 123, y: 456 });
    } finally {
      spy.mockRestore();
    }
  });


  it("折叠组卡渲染 flow-in/flow-out Handle（error008：重路由边不被 ReactFlow 丢弃）", () => {
    // 折叠态连线重路由为 source/target=g1 + flow-in/flow-out handle；
    // ReactFlow 要求组节点渲染对应 id 的 Handle 才保留边（缺失 → error008 静默丢弃）。
    const { container } = render(
      <div className="codeflow-group">
        <GroupBox data={{
          group: { id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: true },
          editing: false, onRename: () => {}, onColorChange: () => {}, onToggleCollapsed: () => {}, onDelete: () => {},
        }} />
      </div>,
    );
    expect(container.querySelector('.codeflow-group [data-handleid="flow-in"]')).toBeTruthy();
    expect(container.querySelector('.codeflow-group [data-handleid="flow-out"]')).toBeTruthy();
  });

  it("展开态组框不渲染 Handle（保持 Task 4 现状）", () => {
    const { container } = render(
      <GroupBox data={{
        group: { id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false },
        editing: false, onRename: () => {}, onColorChange: () => {}, onToggleCollapsed: () => {}, onDelete: () => {},
      }} />,
    );
    expect(container.querySelector("[data-handleid]")).toBeNull();
  });

  it("展开态：bbox 测量等待 ReactFlow 渲染（rAF 后成员 DOM 就绪）→ 组节点 g1 渲染", async () => {
    // 回归测试：展开后 group 节点必须出现。根因（真机 error009）——
    // FlowCanvas 的测量 useEffect 在 store 更新后立即查询成员 DOM，但 ReactFlow 内部
    // 节点渲染是异步的（zustand store 同步 + 渲染在 commit 后），useEffect 执行瞬间
    // 成员节点尚未渲染到 DOM → sizes 空 → bbox 永不写入 → 展开态组框缺 bbox 被跳过。
    // 修复：测量推迟到 requestAnimationFrame（ReactFlow 渲染完成后）。
    // 本测试用 memberRendered 标志模拟 ReactFlow 的异步渲染：同步查询返回 null，
    // rAF 回调后返回 mock 尺寸——修复前（同步测量）组节点缺失，修复后（rAF 测量）出现。
    usePoiesisStore.setState({
      codeFlowGroups: { increment: [{ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false }] },
    });
    let memberRendered = false;
    const origQS = document.querySelector.bind(document);
    const memberMock = (w: number, h: number) => ({ getBoundingClientRect: () => ({ width: w, height: h }) }) as unknown as Element;
    vi.spyOn(document, "querySelector").mockImplementation((sel: string) => {
      if (sel === ".react-flow__viewport") return memberMock(0, 0);
      if (sel === '[data-id="cb1"]' || sel === '[data-id="cb2"]') return memberRendered ? memberMock(200, 100) : null;
      return origQS.call(document, sel);
    });
    vi.spyOn(window, "getComputedStyle").mockImplementation(() => ({ transform: "matrix(1, 0, 0, 1, 0, 0)" }) as CSSStyleDeclaration);
    // 用真实 rAF 模拟异步渲染完成时刻；若环境无 rAF 则以 setTimeout 兜底
    const fallbackRaf: typeof requestAnimationFrame = (cb) => setTimeout(() => cb(performance.now()), 16) as unknown as number;
    const raf = typeof requestAnimationFrame === "function" ? requestAnimationFrame : fallbackRaf;
    const fallbackCaf: typeof cancelAnimationFrame = (id) => clearTimeout(id as unknown as ReturnType<typeof setTimeout>);
    const caf = typeof cancelAnimationFrame === "function" ? cancelAnimationFrame : fallbackCaf;
    vi.stubGlobal("requestAnimationFrame", raf);
    vi.stubGlobal("cancelAnimationFrame", caf);
    raf(() => { memberRendered = true; });
    try {
      render(<FlowCanvas />);
      await waitFor(() => {
        const ids = (captured.nodes as Array<{ id: string }>).map((n) => n.id);
        expect(ids).toContain("g1");
      });
    } finally {
      vi.restoreAllMocks();
      vi.unstubAllGlobals();
    }
  });
});
