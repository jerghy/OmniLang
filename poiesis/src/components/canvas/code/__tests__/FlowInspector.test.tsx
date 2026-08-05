import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
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
        {
          id: "w1", nodeType: "codeblock", tag: "while", parentId: null,
          props: { condition: "count < 10" },
          source: "",
        },
        {
          id: "fi1", nodeType: "codeblock", tag: "for-in", parentId: null,
          props: { varName: "item", iterable: "items" },
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

describe("FlowInspector loop 表单", () => {
  it("while 节点渲染条件字段，提交走 setProp(condition)", () => {
    render(<FlowInspector selectedNodeIds={["w1"]} />);
    // 测试 mock 无 scope → 公式框退化纯文本（raw 文本渲染，textContent 即值）
    const input = screen.getByLabelText("循环条件") as HTMLElement;
    expect(input.textContent).toBe("count < 10");
    fireEvent.input(input, { target: { textContent: "count <= 20" } });
    fireEvent.blur(input);
    expect(mockSetProp).toHaveBeenCalledWith("w1", "condition", "count <= 20");
  });

  it("for-in 节点渲染循环变量（文本）与遍历表达式（公式框）", () => {
    render(<FlowInspector selectedNodeIds={["fi1"]} />);
    const varInput = screen.getByLabelText("循环变量") as HTMLInputElement;
    expect(varInput.value).toBe("item");
    const iterInput = screen.getByLabelText("遍历表达式") as HTMLElement;
    expect(iterInput.textContent).toBe("items");
    fireEvent.input(iterInput, { target: { textContent: "todos" } });
    fireEvent.blur(iterInput);
    expect(mockSetProp).toHaveBeenCalledWith("fi1", "iterable", "todos");
  });

  it("loop 表单底部给出 formatSyntaxNode 实时预览", () => {
    render(<FlowInspector selectedNodeIds={["w1"]} />);
    expect(screen.getByText("while count < 10 {")).toBeTruthy();
  });
});
