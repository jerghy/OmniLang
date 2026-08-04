import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConditionNode } from "../ConditionNode";
import { ReturnNode } from "../ReturnNode";

describe("ConditionNode 内联编辑", () => {
  it("点击 if 表达式进入编辑，blur 提交 onEditCondition", () => {
    const onEditCondition = vi.fn();
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "" }], onEditCondition }} />);
    fireEvent.click(screen.getByText("..."));
    const input = screen.getByLabelText("编辑条件表达式");
    fireEvent.change(input, { target: { value: "count > 0" } });
    fireEvent.blur(input);
    expect(onEditCondition).toHaveBeenCalledWith("br0", "count > 0");
  });

  it("已有条件直接显示，点击进入编辑", () => {
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br0", type: "if", condition: "x == 1" }], onEditCondition: vi.fn() }} />);
    expect(screen.getByText("x == 1")).toBeTruthy();
  });

  it("else 分支不可编辑", () => {
    render(<ConditionNode data={{ label: "判断", branches: [{ id: "br1", type: "else", condition: "" }], onEditCondition: vi.fn() }} />);
    expect(screen.queryByLabelText("编辑条件表达式")).toBeFalsy();
  });
});

describe("ReturnNode 内联编辑", () => {
  it("显示 return 表达式", () => {
    render(<ReturnNode data={{ label: "返回", returnExpr: "items.len()" }} />);
    expect(screen.getByText(/items\.len\(\)/)).toBeTruthy();
  });

  it("点击表达式区进入编辑，Enter 提交", () => {
    const onCommit = vi.fn();
    render(<ReturnNode data={{ label: "返回", returnExpr: "x", onCommitReturn: onCommit }} />);
    fireEvent.click(screen.getByText("return x"));
    const input = screen.getByLabelText("return 表达式");
    fireEvent.change(input, { target: { value: "y" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("y");
  });
});
