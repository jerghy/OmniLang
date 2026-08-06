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
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ComponentDefPanel } from "../ComponentDefPanel";

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

// mock store：最小形态
const { mockState } = vi.hoisted(() => ({
  mockState: {
    designPage: {
      activeTabId: "t1",
      openTabs: [{ id: "t1", componentName: "Card", filePath: "Card.omni", tabType: "component" }],
    },
    componentDefs: { Card: { signals: [{ pid: "s1", name: "count", type: "i32", defaultValue: "0" }], props: [], computeds: [] } },
    debugValues: {},
    debugOverridesEnabled: true,
    clearDebugValues: vi.fn(),
    setDebugOverridesEnabled: vi.fn(),
  },
}));

vi.mock("../../../store", () => ({
  usePoiesisStore: (sel: (s: unknown) => unknown) => sel(mockState),
}));

vi.mock("../../canvas/canvas-store", () => ({
  useCanvasStore: (sel: (s: unknown) => unknown) =>
    sel({ panelPosition: { x: 400, y: 20 }, isPanelCollapsed: false, toggleCollapsed: vi.fn(), setPanelPosition: vi.fn() }),
}));

beforeEach(() => {
  vi.clearAllMocks();
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

  it("Header 显示组件名与用途徽章", () => {
    render(<ComponentDefPanel />);
    expect(screen.getByText("Card")).toBeTruthy();
  });

  it("总开关 🔧 点击切换 setDebugOverridesEnabled", () => {
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByTitle(/调试/));
    expect(mockState.setDebugOverridesEnabled).toHaveBeenCalledWith(false);
  });
});
