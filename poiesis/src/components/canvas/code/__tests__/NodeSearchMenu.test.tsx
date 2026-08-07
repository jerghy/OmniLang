import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeSearchMenu, filterMenuItems } from "../NodeSearchMenu";

describe("filterMenuItems", () => {
  it("空 query 返回全部 3 个分组", () => {
    const r = filterMenuItems("");
    expect(r.map((s) => s.section)).toEqual(["控制流", "变量与赋值", "调用"]);
    expect(r.flatMap((s) => s.items)).toHaveLength(12);
  });
  it("中文子串匹配", () => {
    const r = filterMenuItems("赋值");
    expect(r.flatMap((s) => s.items).map((i) => i.tag)).toEqual(["assign"]);
  });
  it("英文 tag 匹配", () => {
    const r = filterMenuItems("store");
    expect(r.flatMap((s) => s.items).map((i) => i.tag)).toEqual(["store-call"]);
  });
  it("快捷键别名匹配", () => {
    const r = filterMenuItems("ret");
    expect(r.flatMap((s) => s.items).map((i) => i.tag)).toEqual(["return"]);
  });
  it("无匹配返回空", () => {
    expect(filterMenuItems("zzz").flatMap((s) => s.items)).toEqual([]);
  });
});

describe("NodeSearchMenu", () => {
  it("挂载后自动聚焦搜索框", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    expect(document.activeElement).toBe(screen.getByPlaceholderText("搜索节点…"));
  });

  it("输入过滤并高亮命中", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText("搜索节点…"), { target: { value: "let" } });
    expect(screen.getByText((_, node) => node?.textContent === "变量声明 (let)")).toBeTruthy();
    expect(screen.queryByText("函数定义 (fn)")).toBeFalsy();
  });

  it("Enter 创建当前选中项", () => {
    const onSelect = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={onSelect} onClose={vi.fn()} />);
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("fn");
  });

  it("↓ 移动选中后 Enter 创建对应项", () => {
    const onSelect = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={onSelect} onClose={vi.fn()} />);
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("condition");
  });

  it("Esc 关闭", () => {
    const onClose = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={onClose} />);
    fireEvent.keyDown(screen.getByPlaceholderText("搜索节点…"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("无结果显示提示", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    fireEvent.change(screen.getByPlaceholderText("搜索节点…"), { target: { value: "zzz" } });
    expect(screen.getByText("无匹配节点")).toBeTruthy();
  });

  it("combobox aria-controls 指向结果列表 id", () => {
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={vi.fn()} />);
    const input = screen.getByLabelText("搜索节点");
    expect(input.getAttribute("aria-controls")).toBe("node-search-results");
    expect(document.getElementById("node-search-results")).toBeTruthy();
  });

  it("window 级 Escape 关闭（焦点离开输入框后）", () => {
    const onClose = vi.fn();
    render(<NodeSearchMenu x={0} y={0} onSelect={vi.fn()} onClose={onClose} />);
    fireEvent.blur(screen.getByPlaceholderText("搜索节点…"));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });
});
