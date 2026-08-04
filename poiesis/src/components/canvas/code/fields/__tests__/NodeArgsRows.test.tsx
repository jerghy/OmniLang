import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
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

  it("点击 + 添加参数追加空行（渲染为可编辑输入行）", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    expect(onChange).toHaveBeenCalledWith(["a", ""]);
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

  it("多参数连续添加：+ → 输入 → + 恢复可用", () => {
    const onChange = vi.fn();
    const { rerender } = render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    const next1 = ["a", ""];
    rerender(<NodeArgsRows values={next1} onChange={onChange} />);
    expect(inputs()).toHaveLength(2);
    fireEvent.change(inputs()[1], { target: { value: "b" } });
    fireEvent.blur(inputs()[1]);
    expect(onChange).toHaveBeenCalledWith(["a", "b"]);
    rerender(<NodeArgsRows values={["a", "b"]} onChange={onChange} />);
    expect(screen.getByRole("button", { name: "+ 添加参数" })).toBeTruthy();
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
