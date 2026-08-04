import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeTextField, NodeSelect, NodeChipToggle } from "../NodeField";

describe("NodeTextField", () => {
  it("blur 提交 onCommit", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    const input = screen.getByLabelText("变量名");
    fireEvent.change(input, { target: { value: "count" } });
    expect(onCommit).not.toHaveBeenCalled(); // 草稿阶段不提交
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledWith("count");
  });

  it("Enter 提交", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    fireEvent.change(screen.getByLabelText("变量名"), { target: { value: "y" } });
    fireEvent.keyDown(screen.getByLabelText("变量名"), { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("y");
  });

  it("Esc 取消恢复原值", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    fireEvent.change(screen.getByLabelText("变量名"), { target: { value: "z" } });
    fireEvent.keyDown(screen.getByLabelText("变量名"), { key: "Escape" });
    expect(onCommit).not.toHaveBeenCalled();
    expect((screen.getByLabelText("变量名") as HTMLInputElement).value).toBe("x");
  });

  it("外部 value 变化时同步草稿", () => {
    const { rerender } = render(<NodeTextField value="x" onCommit={vi.fn()} ariaLabel="变量名" />);
    rerender(<NodeTextField value="changed" onCommit={vi.fn()} ariaLabel="变量名" />);
    expect((screen.getByLabelText("变量名") as HTMLInputElement).value).toBe("changed");
  });

  it("invalid 时带警示样式类", () => {
    render(<NodeTextField value="x" onCommit={vi.fn()} ariaLabel="变量名" invalid />);
    expect(screen.getByLabelText("变量名").className).toContain("invalid");
  });
});

describe("NodeSelect", () => {
  const options = [
    { value: "=", label: "=" },
    { value: "+=", label: "+=" },
    { value: "custom", label: "自定义…" },
  ];
  it("变更即提交 onCommit", () => {
    const onCommit = vi.fn();
    render(<NodeSelect value="=" options={options} onCommit={onCommit} ariaLabel="运算符" />);
    fireEvent.change(screen.getByLabelText("运算符"), { target: { value: "+=" } });
    expect(onCommit).toHaveBeenCalledWith("+=");
  });
});

describe("NodeChipToggle", () => {
  it("点击切换", () => {
    const onChange = vi.fn();
    render(<NodeChipToggle checked={false} onChange={onChange} label="mut" />);
    fireEvent.click(screen.getByRole("button", { name: "mut" }));
    expect(onChange).toHaveBeenCalledWith(true);
  });
});
