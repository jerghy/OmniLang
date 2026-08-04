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

  it("Enter 提交后 blur 不重复提交（dirty 已消费）", () => {
    const onCommit = vi.fn();
    render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    const input = screen.getByLabelText("变量名");
    fireEvent.change(input, { target: { value: "y" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledTimes(1);
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledTimes(1);
  });

  it("聚焦未编辑时外部变化不被 blur 旧值回滚", () => {
    const onCommit = vi.fn();
    const { rerender } = render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    fireEvent.focus(screen.getByLabelText("变量名"));
    rerender(<NodeTextField value="external" onCommit={onCommit} ariaLabel="变量名" />);
    fireEvent.blur(screen.getByLabelText("变量名"));
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("聚焦编辑中外部变化不覆盖草稿，blur 提交用户编辑", () => {
    const onCommit = vi.fn();
    const { rerender } = render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    const input = screen.getByLabelText("变量名");
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "user" } });
    rerender(<NodeTextField value="external" onCommit={onCommit} ariaLabel="变量名" />);
    expect((input as HTMLInputElement).value).toBe("user");
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenCalledWith("user");
  });

  it("invalid 时设置 aria-invalid", () => {
    render(<NodeTextField value="x" onCommit={vi.fn()} ariaLabel="变量名" invalid />);
    expect(screen.getByLabelText("变量名").getAttribute("aria-invalid")).toBe("true");
  });

  it("Enter 提交后继续输入不被外部同步清空", () => {
    const onCommit = vi.fn();
    const { rerender } = render(<NodeTextField value="x" onCommit={onCommit} ariaLabel="变量名" />);
    const input = screen.getByLabelText("变量名");
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "ab" } });
    fireEvent.keyDown(input, { key: "Enter" });        // 提交 ab
    fireEvent.change(input, { target: { value: "abc" } }); // 提交后继续输入
    rerender(<NodeTextField value="ab" onCommit={onCommit} ariaLabel="变量名" />); // 外部同步
    expect((input as HTMLInputElement).value).toBe("abc"); // 草稿保留
    fireEvent.blur(input);
    expect(onCommit).toHaveBeenLastCalledWith("abc");
  });

  it("NodeSelect allowCustom 展示当前自由文本值", () => {
    const options = [{ value: "=", label: "=" }];
    render(<NodeSelect value="自定义值" options={options} onCommit={vi.fn()} ariaLabel="运算符" allowCustom />);
    const select = screen.getByLabelText("运算符") as HTMLSelectElement;
    expect([...select.options].map((o) => o.value)).toContain("自定义值");
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
