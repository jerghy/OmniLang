import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { NodeArgsRows } from "../NodeField";

function inputs() {
  return screen.getAllByRole("textbox") as HTMLInputElement[];
}

describe("NodeArgsRows", () => {
  it("每参一行输入框", () => {
    render(<NodeArgsRows values={["a", "b"]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(2);
    expect(inputs()[0].value).toBe("a");
  });

  it("编辑行内容提交 onChange", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.change(inputs()[0], { target: { value: "text" } });
    fireEvent.blur(inputs()[0]);
    expect(onChange).toHaveBeenCalledWith(["text"]);
  });

  it("点击 × 删除行", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a", "b"]} onChange={onChange} />);
    fireEvent.click(screen.getAllByRole("button", { name: "×" })[0]);
    expect(onChange).toHaveBeenCalledWith(["b"]);
  });

  it("点击 + 添加参数渲染本地可编辑空行（不立即 onChange）", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    expect(screen.getByLabelText("新参数")).toBeTruthy();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("新参数行输入非空提交 onChange", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    const input = screen.getByLabelText("新参数");
    fireEvent.change(input, { target: { value: "x" } });
    fireEvent.blur(input);
    expect(onChange).toHaveBeenCalledWith(["a", "x"]);
  });

  it("新参数行空值 blur 不提交（行消失）", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    fireEvent.blur(screen.getByLabelText("新参数"));
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("新参数")).toBeFalsy();
  });

  it("新参数行 × 取消", () => {
    const onChange = vi.fn();
    const { container } = render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    const addingRow = container.querySelector('[data-field-kind="args-adding"]')!;
    fireEvent.click(within(addingRow as HTMLElement).getByRole("button", { name: "×" }));
    expect(screen.queryByLabelText("新参数")).toBeFalsy();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("尾随空行渲染为可编辑输入行，+ 按钮隐藏", () => {
    render(<NodeArgsRows values={["a", ""]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(2);
    expect(inputs()[1].value).toBe("");
    expect(screen.queryByRole("button", { name: "+ 添加参数" })).toBeFalsy();
  });

  it("空 args（[\"\"]）渲染一个空输入行，可直接输入首参", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={[""]} onChange={onChange} />);
    expect(inputs()).toHaveLength(1);
    expect(screen.queryByRole("button", { name: "+ 添加参数" })).toBeFalsy();
    fireEvent.change(inputs()[0], { target: { value: "text" } });
    fireEvent.blur(inputs()[0]);
    expect(onChange).toHaveBeenCalledWith(["text"]);
  });

  it("多参数连续添加：+ → 输入提交 → + 恢复 → 再加", () => {
    const onChange = vi.fn();
    const { rerender } = render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    expect(screen.getByLabelText("新参数")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("新参数"), { target: { value: "b" } });
    fireEvent.blur(screen.getByLabelText("新参数"));
    expect(onChange).toHaveBeenCalledWith(["a", "b"]);
    rerender(<NodeArgsRows values={["a", "b"]} onChange={onChange} />);
    expect(screen.getByRole("button", { name: "+ 添加参数" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    expect(screen.getByLabelText("新参数")).toBeTruthy();
  });

  it("中间空行保留为输入行", () => {
    render(<NodeArgsRows values={["a", "", "b"]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(3);
  });

  it("尾随多个空行折叠为一个输入行", () => {
    render(<NodeArgsRows values={["a", "", ""]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(2);
  });
});
