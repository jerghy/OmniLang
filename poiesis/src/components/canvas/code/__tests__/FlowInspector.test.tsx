import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FlowInspector } from "../FlowInspector";
import { usePoiesisStore } from "../../../../store";
import { createInitialDesignPageState } from "../../../../store/operations/tab-lifecycle";
import type { CodeblockElementData } from "../../../../core/types/element";

// 可订阅 mock store（useSyncExternalStore 同款机制，先例见
// ComponentDefPanel.test.tsx / useWasmCompile-debug.test.ts）：静态
// (sel) => sel(mockState) 无法在 setProp 后触发重渲染。setProp 忠实镜像真实
// store 行为（不可变回写 elements 后 notify）；setState/getState 供用例按
// zustand API 设置初始状态与断言。
const { mockSetProp, mockState, storeListeners, resetMockState } = vi.hoisted(() => {
  const mockSetProp = vi.fn();
  const storeListeners = new Set<() => void>();
  const defaultElements = () => [
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
    {
      id: "lc1", nodeType: "codeblock", tag: "lifecycle", parentId: null,
      props: { hookType: "mount" },
      source: "",
    },
    {
      id: "lc2", nodeType: "codeblock", tag: "lifecycle", parentId: null,
      props: { hookType: "effect", deps: "searchText" },
      source: "",
    },
  ];
  const defaultGroups = () => ({
    increment: [{ id: "g1", name: "初始化", memberIds: ["l1", "w1"], color: "#6366f1", collapsed: false }],
  });
  const mockState = {
    elements: [] as unknown[],
    codeFlowGroups: {} as Record<string, unknown[]>,
    setProp: (id: string, prop: string, value: unknown) => {
      mockSetProp(id, prop, value);
      mockState.elements = (
        mockState.elements as Array<{ id: string; props?: Record<string, unknown> }>
      ).map((e) => (e.id === id ? { ...e, props: { ...(e.props ?? {}), [prop]: value } } : e));
      storeListeners.forEach((l) => l());
    },
    setCodeFlowGroups: (fnName: string, groups: unknown[]) => {
      mockState.codeFlowGroups = { ...mockState.codeFlowGroups, [fnName]: groups };
      storeListeners.forEach((l) => l());
    },
    setElementSource: () => {},
    storeReferences: {},
    componentDefs: {},
    storeDefs: {},
    inlineStoreDefs: {},
    designPage: { activeTabId: "t1", openTabs: [] },
  };
  const resetMockState = () => {
    mockState.elements = defaultElements();
    mockState.codeFlowGroups = defaultGroups();
    mockState.designPage = { activeTabId: "t1", openTabs: [] };
  };
  return { mockSetProp, mockState, storeListeners, resetMockState };
});

vi.mock("../../../../store", async () => {
  const { useSyncExternalStore } = await import("react");
  const hook = (sel: (s: unknown) => unknown) =>
    useSyncExternalStore(
      (cb) => {
        storeListeners.add(cb);
        return () => {
          storeListeners.delete(cb);
        };
      },
      () => sel(mockState),
    );
  return {
    usePoiesisStore: Object.assign(hook, {
      setState: (patch: Record<string, unknown>) => {
        Object.assign(mockState, patch);
        storeListeners.forEach((l) => l());
      },
      getState: () => mockState,
    }),
  };
});

beforeEach(() => {
  mockSetProp.mockClear();
  resetMockState();
});

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
    // 测试 mock 无 scope → 无候选菜单（退化）；表达式仍以胶囊标记段渲染（textarea 值=markup）
    const input = screen.getByLabelText("循环条件") as HTMLElement;
    const ta = input.querySelector("textarea") as HTMLTextAreaElement;
    expect(input.querySelector('[data-seg-kind="capsule"]')?.textContent).toBe("count");
    expect(ta.value).toBe("\uE000count\uE001 \uE000<\uE001 10");
    ta.value = "\uE000count\uE001 \uE000<=\uE001 20";
    ta.setSelectionRange(15, 15); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(ta, { target: { value: "\uE000count\uE001 \uE000<=\uE001 20" } });
    fireEvent.blur(ta);
    expect(mockSetProp).toHaveBeenCalledWith("w1", "condition", "count <= 20");
  });

  it("for-in 节点渲染循环变量（文本）与遍历表达式（公式框）", () => {
    render(<FlowInspector selectedNodeIds={["fi1"]} />);
    const varInput = screen.getByLabelText("循环变量") as HTMLInputElement;
    expect(varInput.value).toBe("item");
    const iterInput = screen.getByLabelText("遍历表达式") as HTMLElement;
    const iterTa = iterInput.querySelector("textarea") as HTMLTextAreaElement;
    expect(iterInput.querySelector('[data-seg-kind="capsule"]')?.textContent).toBe("items");
    expect(iterTa.value).toBe("\uE000items\uE001");
    iterTa.value = "\uE000todos\uE001";
    iterTa.setSelectionRange(7, 7); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(iterTa, { target: { value: "\uE000todos\uE001" } });
    fireEvent.blur(iterTa);
    expect(mockSetProp).toHaveBeenCalledWith("fi1", "iterable", "todos");
  });

  it("loop 表单底部给出 formatSyntaxNode 实时预览", () => {
    render(<FlowInspector selectedNodeIds={["w1"]} />);
    expect(screen.getByText("while count < 10 {")).toBeTruthy();
  });
});

describe("FlowInspector LifecycleForm", () => {
  it("mount 节点渲染 Hook type 选择（无 deps 字段）", () => {
    render(<FlowInspector selectedNodeIds={["lc1"]} />);
    expect(screen.getByText("⚡ Lifecycle")).toBeTruthy();
    expect(screen.getByDisplayValue("mount")).toBeTruthy();
    expect(screen.queryByLabelText("Deps")).toBeFalsy();
  });

  it("effect 节点渲染 deps 输入框（受控值），提交走 setProp(deps)", () => {
    render(<FlowInspector selectedNodeIds={["lc2"]} />);
    const input = screen.getByLabelText("Deps") as HTMLInputElement;
    expect(input.value).toBe("searchText");
    fireEvent.change(input, { target: { value: "a, b" } });
    expect(mockSetProp).toHaveBeenCalledWith("lc2", "deps", "a, b");
  });

  it("切换 hookType 走 setProp(hookType)", () => {
    render(<FlowInspector selectedNodeIds={["lc2"]} />);
    fireEvent.change(screen.getByDisplayValue("effect"), { target: { value: "unmount" } });
    expect(mockSetProp).toHaveBeenCalledWith("lc2", "hookType", "unmount");
  });
});

describe("FlowInspector FnInspector pub/declKind/async 表单", () => {
  it("FnInspector 显示 declKind/pub/async 并回写 props", () => {
    const el = {
      id: "fn_x",
      nodeType: "codeblock" as const,
      tag: "fn",
      parentId: null,
      props: { name: "load", params: "", returnType: "" },
      source: "",
    };
    // openTabs 含 code tab（tabType: "code"）→ pub 开关可见
    usePoiesisStore.setState({
      elements: [el],
      designPage: {
        ...createInitialDesignPageState(),
        activeTabId: "t1",
        openTabs: [
          { id: "t1", filePath: "src/lib/x.omni", componentName: "x", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} }, tabType: "code" },
        ],
      },
    });
    render(<FlowInspector selectedNodeIds={["fn_x"]} />);
    expect(screen.getByLabelText("Function name")).toBeTruthy();
    expect(screen.getByText("Declaration kind")).toBeTruthy();
    // code tab → pub 可见
    expect(screen.getByText("pub")).toBeTruthy();
    fireEvent.click(screen.getByText("pub").closest("label")!.querySelector("input")!);
    expect(usePoiesisStore.getState().elements.find(
      (e): e is CodeblockElementData => e.id === "fn_x" && e.nodeType === "codeblock",
    )!.props?.pub).toBe(true);
  });
});

describe("FlowInspector 分组面板（点击分组框后显示组信息并可编辑）", () => {
  it("选中组 id 时渲染组名、成员列表、折叠与删除操作", () => {
    render(<FlowInspector selectedNodeIds={["g1"]} />);
    expect((screen.getByLabelText("分组名") as HTMLInputElement).value).toBe("初始化");
    expect(screen.getByText(/变量声明 · x/)).toBeTruthy(); // 成员：tag 中文 + 变量名
    expect(screen.getByText(/循环/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /折叠|展开/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /删除分组/ })).toBeTruthy();
  });

  it("改名：NodeTextField 提交走 setCodeFlowGroups（组名=功能描写注释）", () => {
    render(<FlowInspector selectedNodeIds={["g1"]} />);
    const input = screen.getByLabelText("分组名") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "校验逻辑" } });
    fireEvent.keyDown(input, { key: "Enter" });
    const groups = usePoiesisStore.getState().codeFlowGroups as Record<string, Array<{ id: string; name: string }>>;
    expect(groups.increment[0].name).toBe("校验逻辑");
  });

  it("折叠/展开切换走 setCodeFlowGroups（collapsed 翻转）", () => {
    render(<FlowInspector selectedNodeIds={["g1"]} />);
    fireEvent.click(screen.getByRole("button", { name: /折叠|展开/ }));
    const groups = usePoiesisStore.getState().codeFlowGroups as Record<string, Array<{ id: string; collapsed: boolean }>>;
    expect(groups.increment[0].collapsed).toBe(true);
  });

  it("删除分组：移除该组且保留节点", () => {
    render(<FlowInspector selectedNodeIds={["g1"]} />);
    fireEvent.click(screen.getByRole("button", { name: /删除分组/ }));
    const groups = usePoiesisStore.getState().codeFlowGroups as Record<string, unknown[]>;
    expect(Object.values(groups).flat()).toHaveLength(0);
  });

  it("组 id 不存在时显示 Node not found（不崩溃）", () => {
    render(<FlowInspector selectedNodeIds={["ghost"]} />);
    expect(screen.getByText("Node not found")).toBeTruthy();
  });
});
