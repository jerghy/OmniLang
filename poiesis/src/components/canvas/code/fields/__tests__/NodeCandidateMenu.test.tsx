import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeCandidateMenu } from "../NodeCandidateMenu";

// 项目未安装 @testing-library/jest-dom；此处按 vitest 官方 Custom Matchers 方式本地注册
// 等价的 toHaveAttribute（语义与 jest-dom 一致：缺省 value 时仅断言属性存在），
// 使 brief 提供的用例可原样运行；类型声明同步补齐，避免 tsc 报错。
declare module "vitest" {
  interface Assertion<T = any> {
    toHaveAttribute(attr: string, value?: string): void;
  }
  interface AsymmetricMatchersContaining {
    toHaveAttribute(attr: string, value?: string): void;
  }
}

expect.extend({
  toHaveAttribute(received: unknown, attr: string, value?: string) {
    const el = received as Element;
    const actual = el.getAttribute(attr);
    const pass = value === undefined ? actual !== null : actual === value;
    return {
      pass,
      message: () =>
        `expected element to ${pass ? "not " : ""}have attribute "${attr}"${value === undefined ? "" : `="${value}"`} (actual: ${actual === null ? "null" : `"${actual}"`})`,
    };
  },
});

const groups = [
  { label: "变量", items: [{ value: "count", label: "count" }, { value: "total", label: "total" }] },
  { label: "信号", items: [{ value: "ThemeStore.mode", label: "ThemeStore.mode" }] },
];

describe("NodeCandidateMenu", () => {
  it("渲染分组与选项（role=listbox/option + aria-selected）", () => {
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByRole("listbox")).toBeTruthy();
    const opts = screen.getAllByRole("option");
    expect(opts).toHaveLength(3);
    expect(opts[0]).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("变量")).toBeTruthy();
    expect(screen.getByText("信号")).toBeTruthy();
  });
  it("activeIdx 高亮对应项", () => {
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={2} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getAllByRole("option")[2]).toHaveAttribute("aria-selected", "true");
  });
  it("点击选项 → onSelect(item)", () => {
    const onSelect = vi.fn();
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={onSelect} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    fireEvent.click(screen.getByText("total"));
    expect(onSelect).toHaveBeenCalledWith({ value: "total", label: "total" });
  });
  it("鼠标悬停 → onMouseEnter(idx)", () => {
    const onMouseEnter = vi.fn();
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={vi.fn()} onMouseEnter={onMouseEnter} onClose={vi.fn()} />);
    fireEvent.mouseEnter(screen.getByText("ThemeStore.mode"));
    expect(onMouseEnter).toHaveBeenCalledWith(2);
  });
  it("无候选 → 显示无匹配项（不渲染 option）", () => {
    render(<NodeCandidateMenu x={0} y={0} groups={[]} activeIdx={0} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByText("无匹配项")).toBeTruthy();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
  });
  it("点击菜单外 → onClose", () => {
    const onClose = vi.fn();
    render(<NodeCandidateMenu x={0} y={0} groups={groups} activeIdx={0} onSelect={vi.fn()} onMouseEnter={vi.fn()} onClose={onClose} />);
    fireEvent.mouseDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
