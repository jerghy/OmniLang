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
});
