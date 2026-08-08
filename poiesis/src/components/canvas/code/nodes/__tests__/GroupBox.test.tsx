// GroupBox 渲染与回调测试（jsdom，无 ReactFlow 依赖——组件内不渲染 Handle）
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { GroupBox } from "../GroupBox";
import type { CodeFlowGroup } from "../../../../../core/types/project-json";

const group = (over: Partial<CodeFlowGroup> = {}): CodeFlowGroup =>
  ({ id: "g1", name: "初始化", memberIds: ["cb1", "cb2"], color: "#6366f1", collapsed: false, ...over });

// nodeTypes 契约要求组件接收 data prop（与 FnNode/LoopNode 同款轻量 Props 惯例）
const renderBox = (over: Partial<CodeFlowGroup> = {}) => ({
  group: group(over),
  editing: false,
  onRename: () => {},
  onColorChange: () => {},
  onToggleCollapsed: () => {},
  onDelete: () => {},
  ...over,
});

describe("GroupBox", () => {
  it("展开态渲染标题栏（组名 + 折叠按钮 + 删除按钮）", () => {
    render(<GroupBox data={renderBox()} />);
    expect(screen.getByText("初始化")).toBeTruthy();
    expect(screen.getByRole("button", { name: /折叠|收起/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /删除分组/ })).toBeTruthy();
  });

  it("折叠态渲染组卡（组名 + 成员数 + 展开按钮）", () => {
    render(<GroupBox data={renderBox({ collapsed: true })} />);
    expect(screen.getByText(/2 条语句/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /展开/ })).toBeTruthy();
  });

  it("editing=true 显示行内命名输入框，Enter 提交 onRename", () => {
    const onRename = vi.fn();
    render(<GroupBox data={{ ...renderBox(), editing: true, onRename }} />);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "校验逻辑" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onRename).toHaveBeenCalledWith("校验逻辑");
  });

  it("色板 8 色点击触发 onColorChange", () => {
    const onColorChange = vi.fn();
    render(<GroupBox data={{ ...renderBox(), onColorChange }} />);
    // 先打开色板（默认收起；计划原文直接点 [3] 会因色板未开而取到 undefined——计划缺陷已修正）
    fireEvent.click(screen.getByRole("button", { name: "分组颜色" }));
    fireEvent.click(screen.getAllByRole("button", { name: /颜色 #/ })[3]);
    expect(onColorChange).toHaveBeenCalled();
  });
});
