// poiesis/src/components/canvas/code/nodes/__tests__/LoopNode.test.tsx
// LoopNode（while / for-in）形态渲染 + 三端口 handle 断言。
// Handle 依赖 ReactFlowProvider（error#001 抛错）；测试经 mock 渲染为普通 div，
// data-handle-id 透传可断言（生产路径 HandlesBoundary 兜底，与 ConditionNode 同构）。
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { LoopNode } from "../LoopNode";

vi.mock("@xyflow/react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@xyflow/react")>();
  return {
    ...actual,
    Handle: (props: Record<string, unknown>) => <div {...props} />,
  };
});

const baseData = {
  label: "循环",
  tag: "while",
  props: { condition: "n < 10" },
  source: "while n < 10 {",
  onEditProp: vi.fn(),
  scope: undefined,
  letVars: [],
  onDoubleClick: undefined,
};

describe("LoopNode", () => {
  it("while 形态渲染条件文本与三端口 handle", () => {
    render(<LoopNode id="w-1" data={baseData as never} />);
    expect(screen.getByText("n < 10")).toBeTruthy();
    // 端口 handle（xyflow 测试环境经 HandlesBoundary 返回 inert 节点）
    expect(document.querySelector('[data-handle-id="flow-in"]')).toBeTruthy();
    expect(document.querySelector('[data-handle-id="br0"]')).toBeTruthy();
    expect(document.querySelector('[data-handle-id="flow-out"]')).toBeTruthy();
  });
  it("for-in 形态渲染 varName/iterable", () => {
    render(<LoopNode id="fi-1" data={{ ...baseData, tag: "for-in", props: { varName: "item", iterable: "items" } } as never} />);
    expect(screen.getByText("item")).toBeTruthy();
    expect(screen.getByText("items")).toBeTruthy();
  });
});
