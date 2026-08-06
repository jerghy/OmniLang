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
      componentDefs: { Card: { signals: [{ pid: "s1", name: "count", type: "i32", defaultValue: "0" }], props: [], computeds: [] } },
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

  it("重置全部按钮 → clearDebugValues", () => {
    mockState.debugValues = { Card: { count: { text: "7", valid: true } } };
    render(<ComponentDefPanel />);
    fireEvent.click(screen.getByRole("tab", { name: /^信号调试$/ }));
    fireEvent.click(screen.getByRole("button", { name: /重置全部/ }));
    expect(mockState.clearDebugValues).toHaveBeenCalled();
  });
});
