import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { ValueEditor } from "../ValueEditor";
import { usePoiesisStore } from "../../../store";
import type { SourceNode } from "../source-nodes";

const node: SourceNode = {
  id: "sig:abc", kind: "signal", name: "count", type: "i32", value: 0, ownerKey: "Card",
};

describe("ValueEditor", () => {
  beforeEach(() => {
    usePoiesisStore.setState({ debugValues: {} });
  });

  it("渲染节点名 + 当前值到输入框", () => {
    render(<ValueEditor node={node} onClose={() => {}} />);
    expect(screen.getByText("count")).toBeTruthy();
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    expect(input.value).toBe("0");
  });

  it("保存 → setDebugValue(ownerKey, name, 解析后字面量, valid)；不 dirty", () => {
    render(<ValueEditor node={node} onClose={() => {}} />);
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "42" } });
    fireEvent.click(screen.getByTestId("value-editor-save"));
    const dv = usePoiesisStore.getState().debugValues;
    expect(dv["Card"]?.["count"]).toEqual({ text: "42", valid: true });
    expect(usePoiesisStore.getState().isDirty).toBe(false);
  });

  it("非法输入（i32 无法解析）→ valid:false 存原文", () => {
    render(<ValueEditor node={node} onClose={() => {}} />);
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "abc" } });
    fireEvent.click(screen.getByTestId("value-editor-save"));
    const dv = usePoiesisStore.getState().debugValues;
    expect(dv["Card"]?.["count"]).toEqual({ text: "abc", valid: false });
  });

  it("关闭 → onClose 回调", () => {
    const onClose = vi.fn();
    render(<ValueEditor node={node} onClose={onClose} />);
    fireEvent.click(screen.getByTestId("value-editor-close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("IME 组合期 Enter 不保存（isComposing 守卫，拼音选字回车）", () => {
    const onClose = vi.fn();
    render(<ValueEditor node={node} onClose={onClose} />);
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "42" } });
    // jsdom 的 KeyboardEvent 构造器不实现 isComposing（RTL 的 nativeEvent init 键会被丢弃）→ 手工定义到原生事件
    const evt = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
    Object.defineProperty(evt, "isComposing", { value: true });
    fireEvent(input, evt);
    expect(usePoiesisStore.getState().debugValues).toEqual({});
    expect(onClose).not.toHaveBeenCalled();
    expect(input.value).toBe("42"); // 编辑态保留，未保存未关闭
  });

  it("回显 valid 条目剥引号（'\"hi\"' → hi）", () => {
    usePoiesisStore.setState({ debugValues: { Card: { count: { text: '"hi"', valid: true } } } });
    render(<ValueEditor node={node} onClose={() => {}} />);
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    expect(input.value).toBe("hi");
  });

  it("回显 invalid 条目不剥引号（存的是原文）", () => {
    usePoiesisStore.setState({ debugValues: { Card: { count: { text: "abc", valid: false } } } });
    render(<ValueEditor node={node} onClose={() => {}} />);
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    expect(input.value).toBe("abc");
  });

  it("defaultValue undefined 时回显 '—' 占位（不显示 undefined）", () => {
    render(<ValueEditor node={{ ...node, value: undefined }} onClose={() => {}} />);
    const input = screen.getByTestId("value-editor-input") as HTMLInputElement;
    expect(input.value).toBe("—");
  });
});
