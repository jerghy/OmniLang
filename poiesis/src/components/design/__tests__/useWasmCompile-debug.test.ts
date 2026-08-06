// useWasmCompile 调试覆盖接线：debugValues → 文件级覆盖 → WASM 编译输入。
// 实现注记（Task 7 brief Step 1 注）：
// 不 mock ../../store 模块——usePoiesisStore 是 zustand hook 调用形态，
// mock 为普通函数后既无 .setState 也无法用 mockImplementationOnce 切总开关
// （once 只拦每次渲染的首次选择器调用，即 useElements 内部那次）。
// 改用真实 store + usePoiesisStore.setState 切总开关，hook 自身订阅自动
// 触发重渲染（useSyncExternalStore），无需 wrapper。契约不变：总开关 off
// 时覆盖不生效。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";
import { useWasmCompile } from "../useWasmCompile";

const { mockResolve } = vi.hoisted(() => ({ mockResolve: vi.fn() }));

// brief 中的 mock 路径为 ../../ 系，从 __tests__/ 出发实际解析到不存在的
// src/components/{commands,services}/...（mock 静默不生效）；useWasmCompile 的
// import 是 ../../（= src/），故此处用 ../../../ 对齐真实模块路径。
vi.mock("../../../commands/rustCompiler", () => ({
  rustCompiler: {
    isReady: true,
    resolveStylesMultiCached: (sources: Record<string, string>) => mockResolve(sources),
  },
}));

vi.mock("../../../services/codegen/pipeline", () => ({
  generateOmniFiles: () => ({
    value: {
      "src/main.omni": 'signal status: String = "loading";\ncomponent main {\n  signal status: String = "loading";\n  view { <p>{status}</p> }\n}',
    },
    errors: [],
  }),
  generateOmniFragment: () => ({ value: null, errors: [] }),
}));

vi.mock("../../../services/codegen/design-emitter", () => ({ emitDesignSource: () => null }));

const ELEMENT: ElementData = {
  id: "e1",
  nodeType: "view",
  tag: "p",
  parentId: null,
  props: {},
  children: [],
  position: { x: 0, y: 0 },
};

/** 基线：单元素画布 + debugValues(main.status = "error") + 总开关 ON */
function baseline() {
  usePoiesisStore.setState({
    elements: [ELEMENT],
    wires: [],
    rootOrder: ["e1"],
    componentDefs: {},
    designPage: { ...usePoiesisStore.getState().designPage, codeFlowVisible: false },
    debugValues: {
      main: { status: { text: '"error"', valid: true } },
    },
    debugOverridesEnabled: true,
  });
}

describe("useWasmCompile 调试覆盖接线", () => {
  beforeEach(() => {
    mockResolve.mockClear();
    baseline();
  });

  it("debugValues 覆盖后的源码传给 WASM（status → error）", async () => {
    renderHook(() => useWasmCompile());
    // 等 150ms 防抖 + 编译
    await act(async () => { await new Promise((r) => setTimeout(r, 200)); });
    const sources = mockResolve.mock.calls[0]?.[0] as Record<string, string>;
    expect(sources["src/main.omni"]).toContain('signal status: String = "error";');
  });

  it("总开关 off → 调试值不参与覆盖", async () => {
    renderHook(() => useWasmCompile());
    await act(async () => { await new Promise((r) => setTimeout(r, 200)); });
    // 切总开关 → hook 订阅触发重渲染 → 重新编译（覆盖不生效）
    await act(async () => {
      usePoiesisStore.setState({ debugOverridesEnabled: false });
      await new Promise((r) => setTimeout(r, 200));
    });
    const calls = mockResolve.mock.calls;
    expect(calls.length).toBeGreaterThanOrEqual(2);
    const sources = calls[calls.length - 1]?.[0] as Record<string, string>;
    expect(sources["src/main.omni"]).toContain('signal status: String = "loading";');
  });
});
