import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/react";

vi.mock("@xyflow/react", () => ({
  Handle: () => <div data-testid="mock-handle" />,
  Position: { Top: "top", Right: "right", Bottom: "bottom" },
}));

import { TryNode } from "../TryNode";

const baseData = {
  label: "try 块",
  branches: [
    { id: "br0", type: "try" as const },
    { id: "br1", type: "catch" as const, catchVar: "err" },
    { id: "br2", type: "finally" as const },
  ],
};

describe("TryNode", () => {
  it("渲染三分支行 + catchVar 展示", () => {
    const { container } = render(<TryNode data={baseData} />);
    const rows = container.querySelectorAll('[data-testid="try-branch-row"]');
    expect(rows.length).toBe(3);
    expect(container.textContent).toContain("尝试");
    expect(container.textContent).toContain("捕获");
    expect(container.textContent).toContain("收尾");
    expect(container.textContent).toContain("err");
  });

  it("hasCatch/hasFinally 时 + 按钮禁用", () => {
    const { getByTestId } = render(<TryNode data={baseData} />);
    const addCatch = getByTestId("add-catch") as HTMLButtonElement;
    const addFinally = getByTestId("add-finally") as HTMLButtonElement;
    expect(addCatch.disabled).toBe(true);
    expect(addFinally.disabled).toBe(true);
  });

  it("仅 try 分支时按钮可用，点击追加新分支（br${max+1}）", () => {
    const onUpdateBranches = vi.fn();
    const { getByTestId } = render(
      <TryNode data={{ label: "try 块", branches: [{ id: "br0", type: "try" }], onUpdateBranches }} />,
    );
    fireEvent.click(getByTestId("add-catch"));
    expect(onUpdateBranches).toHaveBeenCalledWith([
      { id: "br0", type: "try" },
      { id: "br1", type: "catch" },
    ]);
    fireEvent.click(getByTestId("add-finally"));
    expect(onUpdateBranches).toHaveBeenCalledWith([
      { id: "br0", type: "try" },
      { id: "br1", type: "finally" },
    ]);
  });
});
