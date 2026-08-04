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
    const del = screen.getAllByRole("button", { name: "×" });
    fireEvent.click(del[0]);
    expect(onChange).toHaveBeenCalledWith(["b"]);
  });

  it("点击 + 添加参数追加空行", () => {
    const onChange = vi.fn();
    render(<NodeArgsRows values={["a"]} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: "+ 添加参数" }));
    expect(onChange).toHaveBeenCalledWith(["a", ""]);
  });

  it("尾随空行折叠为 + 按钮（不渲染输入框）", () => {
    render(<NodeArgsRows values={["a", ""]} onChange={vi.fn()} />);
    expect(inputs()).toHaveLength(1);
    expect(screen.getByRole("button", { name: "+ 添加参数" })).toBeTruthy();
  });
});
