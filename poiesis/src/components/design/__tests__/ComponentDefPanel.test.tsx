// ComponentDefPanel 三 Tab 骨架：默认界面信号调试 + Tab 切换 + 定义 Tab 内容。
// 相对 brief 的修正（如实验证，详见 task-8-report.md）：
// 1) mock 路径从 __tests__/ 出发解析：store = ../../../store（= src/store），
//    canvas-store = ../../canvas/canvas-store（= src/components/canvas/）——
//    brief 的 ../../../../store 与 ../../../canvas/... 解析到不存在的模块
//    （同 lesson 见 useWasmCompile-debug.test.ts 头部注记）；
// 2) mockState 的 activeTabId/openTabs 按真实 store 形状包进 designPage
//    （组件读 s.designPage.activeTabId，真实 store 亦为 designPage 子对象，
//    store/index.ts designPage: createInitialDesignPageState()）；
// 3) 项目未装 @testing-library/jest-dom，本地注册等价 toHaveAttribute
//    （NodeCandidateMenu.test.tsx 同款先例），使 brief 用例可原样运行。
// 4) (Task 9) 静态 (sel) => sel(mockState) 无法在 change 后重渲染——「非法输入
//    标红」用例需要 store 变更 → 重渲染 → className 更新。mock 升级为
//    useSyncExternalStore 订阅（zustand 自身同款机制；见 useWasmCompile-debug
//    先例），setDebugValue/clearDebugValues 忠实镜像真实 store 行为（回写
//    mockState.debugValues 后 notify，见 store/slices/component-lifecycle.ts）。
//    另：brief 的 tab 正则 /信号调试/ 子串匹配到「界面信号调试」，锚定为
//    /^信号调试$/（getByRole name 正则按子串匹配）。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ComponentDefPanel } from "../ComponentDefPanel";
import type { DebugValueEntry } from "../../../store/types";
import type { DefEntry } from "../../../core/types/project-json";

declare module "vitest" {
  // 泛型默认值必须与 vitest 官方 Assertion<T = any> 声明同形（否则 tsc 报
  // "All declarations must have identical type parameters"）；用 this: T 引用
  // 该泛型避免 no-unused-vars，any 仅为类型参数默认值故局部豁免。
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  interface Assertion<T = any> {
    toHaveAttribute(this: T, attr: string, value?: string): void;
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

// mock store：最小形态 + 可订阅（useSyncExternalStore）。setDebugValue/
// clearDebugValues 忠实镜像真实 store：不可变回写 mockState.debugValues 后
// notify，订阅者重渲染（真实实现见 store/slices/component-lifecycle.ts）。
const { mockState, storeListeners } = vi.hoisted(() => {
  const storeListeners = new Set<() => void>();
  const notifyStore = () => {
    storeListeners.forEach((l) => l());
  };
  return {
    storeListeners,
    mockState: {
      designPage: {
        activeTabId: "t1",
        openTabs: [{ id: "t1", componentName: "Card", filePath: "Card.omni", tabType: "component" }],
      },
      // 类型注解：props/computeds 空数组字面量否则推断为 never[]，后续测试赋值真实对象报 TS2322
      // (M8 修复 2026-08-06) Card 带 elements——generateOmniFragment mock 镜像真实短路
      // （空 elements → ""），Card 用例需非空才能走 fixture 分支
      componentDefs: {
        Card: {
          signals: [{ pid: "s1", name: "count", type: "i32", defaultValue: "0" }],
          props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "Card.@if", parentId: null, props: {}, source: "" }],
        },
      } as Record<string, { signals: DefEntry[]; props: DefEntry[]; computeds: DefEntry[]; elements: unknown[] }>,
      // Task 10：面板读 storeDefs/inlineStoreDefs 收集 store 信号（界面卡 ownerKey 限定）
      storeDefs: {},
      inlineStoreDefs: {},
      // (M8) main 组件的设计树在 store.elements（画布实时状态）
      elements: [] as unknown[],
      wires: [] as unknown[],
      rootOrder: [] as string[],
      debugValues: {} as Record<string, Record<string, DebugValueEntry>>,
      debugOverridesEnabled: true,
      clearDebugValues: vi.fn(() => {
        mockState.debugValues = {};
        notifyStore();
      }),
      setDebugOverridesEnabled: vi.fn(),
      setDebugValue: vi.fn((ownerKey: string, name: string, text: string, valid: boolean) => {
        const ownerValues = { ...(mockState.debugValues[ownerKey] || {}) };
        ownerValues[name] = { text, valid };
        mockState.debugValues = { ...mockState.debugValues, [ownerKey]: ownerValues };
        notifyStore();
      }),
    },
  };
});

vi.mock("../../../store", async () => {
  const { useSyncExternalStore } = await import("react");
  return {
    usePoiesisStore: (sel: (s: unknown) => unknown) =>
      useSyncExternalStore(
        (cb) => {
          storeListeners.add(cb);
          return () => {
            storeListeners.delete(cb);
          };
        },
        () => sel(mockState),
      ),
  };
});

// (Task 10) 组件源码经 generateOmniFragment 面板内惰性生成（ComponentDefPanel
// 从 componentDefs[componentName].elements 生成），测试环境 mock pipeline 返回
// fixture view（默认含 @if status == "loading" / @else / 复杂条件 / @for），
// 使 analyzeSignalUsage 产出 interfaceSignals（brief Step 1 注记的默认路径）。
// (F2/D12) fixture 可变：单分支二态开关用例替换为 `@if badgeVisible` 单一分支
//（组件 useMemo 依赖不含 fixture 文本，mount 前替换即可生效；顶层 beforeEach 恢复默认）。
const { mockViewSource, mockMainSource } = vi.hoisted(() => ({
  mockViewSource: {
    value:
      "view {\n" +
      '  @if status == "loading" { } @else { }\n' +
      "  @if items.len() > 0 { }\n" +
      "  @for item in items { }\n" +
      "}",
  },
  // main 组件的 fixture（M8 实锤修复 2026-08-06）：main 设计树在 store.elements，
  // componentDefs.main.elements 恒空。mock 镜像真实 generateOmniFragment 的空元素
  // 短路（pipeline.ts:441 `if (componentElements.length === 0) return { value: "" }`），
  // 否则修复前 main 分支也会返回 fixture、RED 无效。
  mockMainSource: {
    value:
      "component main {\n" +
      '  signal newSignal: String = "";\n' +
      "  view {\n" +
      '    @if newSignal == "demo" { }\n' +
      "  }\n" +
      "}",
  },
}));

vi.mock("../../../services/codegen/pipeline", () => ({
  generateOmniFragment: vi.fn((opts: { componentName: string; designElements?: unknown[] }) => {
    const els = opts.designElements || [];
    if (els.length === 0) return { value: "", errors: [] };
    return { value: opts.componentName === "main" ? mockMainSource.value : mockViewSource.value, errors: [] };
  }),
}));

vi.mock("../../canvas/canvas-store", () => ({
  useCanvasStore: (sel: (s: unknown) => unknown) =>
    sel({ panelPosition: { x: 400, y: 20 }, isPanelCollapsed: false, togglePanelCollapsed: vi.fn(), setPanelPosition: vi.fn() }),
}));

beforeEach(() => {
  vi.clearAllMocks();
  // 恢复默认 fixture（单分支二态用例会替换）
  mockViewSource.value =
    "view {\n" +
    '  @if status == "loading" { } @else { }\n' +
    "  @if items.len() > 0 { }\n" +
    "  @for item in items { }\n" +
    "}";
});

describe("ComponentDefPanel 三 Tab 骨架", () => {
  it("默认显示界面信号调试 Tab", () => {
    render(<ComponentDefPanel />);
    expect(screen.getByRole("tab", { name: /界面信号调试/ })).toHaveAttribute("aria-selected", "true");
  });

  it("Tab 切换：点「定义」显示 Props/Signals/Computed 段", () => {
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /定义/ }));
    expect(screen.getByRole("tab", { name: /定义/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("count")).toBeTruthy();
  });

  it("Header 显示组件名", () => {
    // 注：旧名「…与用途徽章」未断言徽章（徽章在界面信号卡的信号行内，不在 Header），
    // 改为准确名称。
    render(<ComponentDefPanel />);
    expect(screen.getByText("Card")).toBeTruthy();
  });

  it("总开关 🔧 点击切换 setDebugOverridesEnabled", () => {
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByTitle(/调试/));
    expect(mockState.setDebugOverridesEnabled).toHaveBeenCalledWith(false);
  });
});

describe("信号调试 Tab（数据信号行）", () => {
  it("列出数据信号：名称/类型/输入框；输入 → setDebugValue", () => {
    mockState.debugValues = {};
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    // mock 组件只有 count（i32，纯数据）
    const input = screen.getByLabelText("count 值");
    fireEvent.change(input, { target: { value: "5" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "count", "5", true);
  });

  it("非法输入 → 标红且 valid=false 传给 store", () => {
    mockState.debugValues = {};
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.change(screen.getByLabelText("count 值"), { target: { value: "abc" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "count", "abc", false);
    expect(screen.getByLabelText("count 值").className).toContain("invalid");
  });

  it("回显已有调试值", () => {
    mockState.debugValues = { Card: { count: { text: "7", valid: true } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    expect((screen.getByLabelText("count 值") as HTMLInputElement).value).toBe("7");
  });

  it("步进按钮：空值步进产出 1（合法路径）", () => {
    mockState.debugValues = {};
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.click(screen.getByRole("button", { name: "count 加一" }));
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "count", "1", true);
  });

  it("步进按钮：非法文本不产出 NaN（守卫回退 0）", () => {
    // 旧实现 Number("abc") + 1 = NaN → 产出 "NaN"；守卫回退 "0"
    mockState.debugValues = { Card: { count: { text: "abc", valid: false } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.click(screen.getByRole("button", { name: "count 加一" }));
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "count", "0", true);
  });

  it("重置全部按钮 → clearDebugValues", () => {
    mockState.debugValues = { Card: { count: { text: "7", valid: true } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.click(screen.getByRole("button", { name: /重置全部/ }));
    expect(mockState.clearDebugValues).toHaveBeenCalled();
  });

  // fix1 裁决：SignalDebugRow（Task 9）一并修复——text 语义 = 可注入字面量
  // （parseDebugValue 输出），回显剥 String 引号还原用户文本。
  it("String 信号：输入补引号存字面量 + 回显剥引号（Task 9 fix1 一并修复）", () => {
    // title 不在 view 的 @if/@for 中 → 数据信号，出现在信号调试 Tab
    mockState.componentDefs = {
      Card: {
        signals: [
          { pid: "s1", name: "title", type: "String", defaultValue: '"x"' },
          { pid: "s2", name: "status", type: "String", defaultValue: '"x"' },
        ],
        props: [], computeds: [],
        elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }],
},
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.change(screen.getByLabelText("title 值"), { target: { value: "loading" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "title", '"loading"', true);
    expect((screen.getByLabelText("title 值") as HTMLInputElement).value).toBe("loading");
  });

  // 2026-08-06 真机实测发现：数组输入 a, b → 存字面量 ["a","b"] 但回显也显示
  // 字面量（与 String 剥引号还原不一致）——displayDebugText 补数组还原。
  // 注意信号名用 tags（fixture 的 items 被 @for 引用 → 归界面信号，不在本 Tab）。
  it("数组信号：输入逗号分隔存字面量 + 回显还原用户文本", () => {
    mockState.componentDefs = {
      Card: {
        signals: [{ pid: "s1", name: "tags", type: "Array<String>", defaultValue: "[]" }],
        props: [], computeds: [],
        elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }],
      },
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.change(screen.getByLabelText("tags 值"), { target: { value: "a, b" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "tags", '["a", "b"]', true);
    // 回显还原逗号分隔（用户输入形态，可再编辑）
    expect((screen.getByLabelText("tags 值") as HTMLInputElement).value).toBe("a, b");
  });
});

// ── 界面信号调试 Tab（Task 10：信号卡 + 分支胶囊 + 列表胶囊）──
// 相对 brief 的修正（详见 task-10-report.md）：
// 1) ownerKey 为组件名（Task 9 已确认语义 + applyDebugOverrides 按
//    resolveOwnerFile 消费），brief 用例原写信号名（"status"/"items"）会落
//    入永远不生效的 ownerKey；
// 2) brief 断言 setDebugValue(..., "loading", true) 无引号——测试为契约，
//    实现按原样存合成字面量（String 不补引号，输入框回显原文）；
// 3) brief 用例 3 依赖前例遗留 componentDefs，补显式 mock（状态卫生
//    beforeEach 重置 debugOverridesEnabled/debugValues）；
// 4) brief 的 getByText(/status/) 会命中分支胶囊文本（status == "loading"），
//    改为精确匹配卡片名；
// 5) 组件源码经 generateOmniFragment mock（fixture view，见文件头）喂分析器。
describe("界面信号调试 Tab（信号卡）", () => {
  beforeEach(() => {
    mockState.debugOverridesEnabled = true;
    mockState.debugValues = {};
  });

  it("界面信号卡：@if 分支胶囊 + 条件文本 + 点选驱动合成值", () => {
    // mock 组件 view 有 @if status == "loading" / @else → analyzeSignalUsage 产出分支
    mockState.componentDefs = {
      Card: {
        signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }],
        props: [], computeds: [],
        elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }],
},
    };
    render(<ComponentDefPanel />);
    // 默认即界面信号调试 Tab
    expect(screen.getByTestId("interface-tab")).toBeTruthy();
    expect(screen.getByText("status")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /loading/ }));
    // fix1 裁决：text 语义 = 可注入字面量 → String 合成值补引号入库
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "status", '"loading"', true);
  });

  it("String 输入：补引号存字面量 + 回显剥引号（fix1）", () => {
    mockState.componentDefs = {
      Card: {
        signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }],
        props: [], computeds: [],
        elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }],
},
    };
    render(<ComponentDefPanel />);
    fireEvent.change(screen.getByLabelText("status 值"), { target: { value: "loading" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "status", '"loading"', true);
    // 回显：字面量剥首尾引号 → 用户友好文本
    expect((screen.getByLabelText("status 值") as HTMLInputElement).value).toBe("loading");
  });

  it("复杂条件分支胶囊置灰", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "items", type: "Array<String>", defaultValue: "[]" }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    render(<ComponentDefPanel />);
    const btn = screen.getByRole("button", { name: /len\(\)/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });

  it("总开关 off → 分支胶囊禁用", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    mockState.debugOverridesEnabled = false;
    render(<ComponentDefPanel />);
    expect((screen.getByRole("button", { name: /loading/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("列表卡：@for 信号 + 空数组候选 + 手动输入", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "items", type: "Array<String>", defaultValue: "[]" }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    render(<ComponentDefPanel />);
    expect(screen.getByText(/@for/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /空/ }));
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "items", "[]", true);
  });

  it("else 分支胶囊可点：驱动成假（同链首可合成条件取假值）（F2）", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    render(<ComponentDefPanel />);
    const elseChip = screen.getByRole("button", { name: /^else$/ });
    expect((elseChip as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(elseChip);
    // status == "loading" 成假 → 字符串非空取 "" → String 补引号入库
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "status", '""', true);
  });

  it("胶囊点选后该分支高亮（F5）", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("button", { name: /loading/ }));
    expect((screen.getByRole("button", { name: /loading/ }) as HTMLButtonElement).style.background)
      .toBe("var(--ol-color-primary)");
  });

  it("输入框手改值失配 → 胶囊高亮取消（F5）", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "status", type: "String", defaultValue: '"x"' }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    mockState.debugValues = { Card: { status: { text: '"loading"', valid: true } } };
    render(<ComponentDefPanel />);
    // 初始：调试值 == 分支合成字面量 → 高亮
    expect((screen.getByRole("button", { name: /loading/ }) as HTMLButtonElement).style.background)
      .toBe("var(--ol-color-primary)");
    fireEvent.change(screen.getByLabelText("status 值"), { target: { value: "other" } });
    // 手改失配 → 全部取消
    expect((screen.getByRole("button", { name: /loading/ }) as HTMLButtonElement).style.background)
      .toBe("var(--ol-surface)");
  });

  it("store 信号行：写入 owner=store 名、name=裸名（F3）", () => {
    mockState.storeDefs = {
      NotificationStore: { signals: [{ pid: "s1", name: "unreadCount", type: "i32", defaultValue: "0" }] },
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.change(screen.getByLabelText("NotificationStore.unreadCount 值"), { target: { value: "5" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("NotificationStore", "unreadCount", "5", true);
  });

  it("内联 store 信号行：owner=Comp.Store、name=裸名（F3）", () => {
    mockState.inlineStoreDefs = {
      "Card.CartStore": { signals: [{ pid: "s1", name: "items", type: "i32", defaultValue: "0" }] },
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.change(screen.getByLabelText("Card.CartStore.items 值"), { target: { value: "3" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card.CartStore", "items", "3", true);
  });

  it("prop 参与信号分析：出现在信号调试列表（F4）", () => {
    mockState.componentDefs = {
      Card: {
        signals: [{ pid: "s1", name: "count", type: "i32", defaultValue: "0" }],
        props: [{ pid: "p1", name: "title", type: "String", defaultValue: '"x"' }],
        computeds: [],
        elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }],
},
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    const input = screen.getByLabelText("title 值");
    fireEvent.change(input, { target: { value: "hello" } });
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "title", '"hello"', true);
  });

  it("prop 参与信号分析：界面信号卡同样可点（F4）", () => {
    // fixture view 的 @if status == "loading" 由 prop 驱动 → 界面信号卡
    mockState.componentDefs = {
      Card: {
        signals: [],
        props: [{ pid: "p1", name: "status", type: "String", defaultValue: '"x"' }],
        computeds: [],
        elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }],
},
    };
    render(<ComponentDefPanel />);
    expect(screen.getByText("status")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /loading/ }));
    expect(mockState.setDebugValue).toHaveBeenCalledWith("Card", "status", '"loading"', true);
  });
});

// ── 单分支二态开关（D12 / F2b）──
describe("单分支二态开关（D12）", () => {
  beforeEach(() => {
    mockState.debugOverridesEnabled = true;
    mockState.debugValues = {};
    mockViewSource.value = "view {\n  @if badgeVisible {\n    <span>New</span>\n  }\n}";
  });

  it("点击成真 → 再点击成假（toggle）", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "badgeVisible", type: "bool", defaultValue: "false" }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("button", { name: /badgeVisible/ }));
    expect(mockState.setDebugValue).toHaveBeenLastCalledWith("Card", "badgeVisible", "true", true);
    // 回显 true 匹配成真合成 → 再点击切换成假
    fireEvent.click(screen.getByRole("button", { name: /badgeVisible/ }));
    expect(mockState.setDebugValue).toHaveBeenLastCalledWith("Card", "badgeVisible", "false", true);
  });

  it("已有成假调试值 → 点击切换回成真", () => {
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "badgeVisible", type: "bool", defaultValue: "false" }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    mockState.debugValues = { Card: { badgeVisible: { text: "false", valid: true } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("button", { name: /badgeVisible/ }));
    expect(mockState.setDebugValue).toHaveBeenLastCalledWith("Card", "badgeVisible", "true", true);
  });

  it("!sig 单分支：成真合成 false（F1 端到端）", () => {
    mockViewSource.value = "view {\n  @if !badgeVisible {\n    <span>New</span>\n  }\n}";
    mockState.componentDefs = {
      Card: { signals: [{ pid: "s1", name: "badgeVisible", type: "bool", defaultValue: "false" }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("button", { name: /badgeVisible/ }));
    expect(mockState.setDebugValue).toHaveBeenLastCalledWith("Card", "badgeVisible", "false", true);
  });
});

describe("main 组件界面信号（M8 实锤修复）", () => {
  it("main 从 store.elements 生成源码——conditional 节点驱动信号卡显示", () => {
    // main 的设计树在 store.elements（componentDefs.main.elements 恒空）——
    // 修复前 getComponentSource 走 componentDefs 路径返回空源码 → 无界面信号
    mockState.designPage.activeTabId = "t-main";
    mockState.designPage.openTabs = [{ id: "t-main", componentName: "main", filePath: "src/main.omni", tabType: "component" }];
    mockState.componentDefs = {
      main: { signals: [{ pid: "s1", name: "newSignal", type: "String", defaultValue: '""' }], props: [], computeds: [],
          elements: [{ nodeType: "conditional", tag: "conditional", id: "T.@if", parentId: null, props: {}, source: "" }] },
    };
    mockState.elements = [{ nodeType: "conditional", tag: "conditional", id: "Main.@if", parentId: null, props: {}, source: "" }];
    mockState.wires = [];
    mockState.rootOrder = ["Main.@if"];
    render(<ComponentDefPanel />);
    // mock 对 main 返回含 `@if newSignal == "demo"` 的 fixture → newSignal 卡 + 分支胶囊
    expect(screen.getByRole("button", { name: /newSignal == "demo"/ })).toBeTruthy();
    expect(screen.queryByText("无界面驱动信号（@if/@for 未用信号）")).toBeNull();
  });
});
