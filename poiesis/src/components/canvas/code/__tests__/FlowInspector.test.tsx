import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FlowInspector } from "../FlowInspector";

// vi.mock 会被提升到 import 之前执行，mock 中引用的变量必须用 vi.hoisted 创建。
const { mockSetProp } = vi.hoisted(() => ({ mockSetProp: vi.fn() }));

vi.mock("../../../../store", () => ({
  usePoiesisStore: (selector: (s: unknown) => unknown) =>
    selector({
      elements: [
        {
          id: "l1", nodeType: "codeblock", tag: "let", parentId: null,
          props: { name: "x", mutable: false, value: "0" },
          source: "let x = 0;",
        },
        {
          id: "s1", nodeType: "codeblock", tag: "store-call", parentId: null,
          props: { storeName: "TodoStore", action: "add", args: "" },
          source: "TodoStore.add();",
        },
      ],
      setProp: (id: string, prop: string, value: unknown) => {
        mockSetProp(id, prop, value);
      },
      setElementSource: () => {},
      storeReferences: { TodoStore: ["App"], CartStore: ["App"] },
    }),
}));

beforeEach(() => {
  mockSetProp.mockClear();
});

describe("FlowInspector syntax node forms", () => {
  it("renders LetInspector fields for a let node", () => {
    render(<FlowInspector selectedNodeIds={["l1"]} />);
    expect(screen.getByLabelText("变量名")).toBeTruthy();
    expect(screen.getByLabelText("可变 (mut)")).toBeTruthy();
    expect(screen.getByLabelText("初始值")).toBeTruthy();
  });

  it("updates let props through setProp", () => {
    render(<FlowInspector selectedNodeIds={["l1"]} />);
    fireEvent.change(screen.getByLabelText("变量名"), { target: { value: "count" } });
    expect(mockSetProp).toHaveBeenCalledWith("l1", "name", "count");
  });

  it("renders store-call form with store name datalist from storeReferences", () => {
    render(<FlowInspector selectedNodeIds={["s1"]} />);
    expect(screen.getByLabelText("store 名")).toBeTruthy();
    expect(screen.getByLabelText("action")).toBeTruthy();
  });
});
