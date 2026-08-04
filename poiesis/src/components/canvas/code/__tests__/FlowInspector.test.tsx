import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { FlowInspector } from "../FlowInspector";

const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        {
          id: "l1", nodeType: "codeblock", tag: "let", parentId: null,
          props: { name: "x", mutable: true, value: "0" },
          source: "let mut x = 0;",
        },
        {
          id: "f1", nodeType: "codeblock", tag: "fn", parentId: null,
          props: { name: "add", params: "text: String" },
          source: "",
        },
      ],
      setProp: (id: string, prop: string, value: unknown) => { mockSetProp(id, prop, value); },
      setElementSource: () => {},
      storeReferences: {},
      componentDefs: {},
      storeDefs: {},
      inlineStoreDefs: {},
      designPage: { activeTabId: "t1", openTabs: [] },
    }),
}));

beforeEach(() => { mockSetProp.mockClear(); });

describe("FlowInspector 降级为预览", () => {
  it("语法节点显示编辑提示与 formatSyntaxNode 实时预览", () => {
    render(<FlowInspector selectedNodeIds={["l1"]} />);
    expect(screen.getByText("⚡ 在节点上直接编辑")).toBeTruthy();
    expect(screen.getByText("let mut x = 0;")).toBeTruthy();
  });

  it("fn 节点仍渲染参数编辑表单", () => {
    render(<FlowInspector selectedNodeIds={["f1"]} />);
    expect(screen.getByLabelText("Function name")).toBeTruthy();
    expect(screen.getByRole("button", { name: "+ Add" })).toBeTruthy();
  });

  it("未选择时显示占位提示", () => {
    render(<FlowInspector selectedNodeIds={[]} />);
    expect(screen.getByText("Select a node to edit")).toBeTruthy();
  });
});
