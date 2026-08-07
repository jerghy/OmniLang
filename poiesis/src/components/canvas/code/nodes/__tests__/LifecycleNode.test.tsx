// poiesis/src/components/canvas/code/nodes/__tests__/LifecycleNode.test.tsx
// LifecycleNode（mount/effect/unmount）形态渲染 + 端口 handle 断言（镜像 LoopNode.test.tsx）。
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { LifecycleNode } from "../LifecycleNode";

vi.mock("@xyflow/react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@xyflow/react")>();
  return {
    ...actual,
    Handle: (props: Record<string, unknown>) => <div {...props} />,
  };
});

const baseData = {
  tag: "lifecycle",
  props: { hookType: "effect", deps: "searchText, setFilter" },
  onEditProp: vi.fn(),
  scope: undefined,
  letVars: [],
  onDoubleClick: undefined,
};

describe("LifecycleNode", () => {
  it("effect 形态渲染摘要（含 deps）与 flow-in/flow-out 端口", () => {
    const { container } = render(<LifecycleNode id="lc-1" data={baseData as never} />);
    expect(container.querySelector('[data-testid="lifecycle-summary"]')?.textContent).toBe("@effect(searchText, setFilter)");
    expect(document.querySelector('[data-handle-id="flow-in"]')).toBeTruthy();
    expect(document.querySelector('[data-handle-id="flow-out"]')).toBeTruthy();
  });
  it("mount 形态渲染摘要（无 deps 括号）", () => {
    render(<LifecycleNode id="lc-2" data={{ ...baseData, props: { hookType: "mount" } } as never} />);
    expect(screen.getByText("@mount")).toBeTruthy();
  });
});
