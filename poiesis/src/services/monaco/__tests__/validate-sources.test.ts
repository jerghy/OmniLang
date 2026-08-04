// 门禁修复：compile_multi_to_omniview 返回 MultiCompileEnvelope（{ result, cache_state }），
// 错误在 result.result.errors（omnilang-wasm/src/lib.rs:81-85）。
// 旧实现读顶层 "errors" 恒空 → design/store/component 三处保存门禁全部静默放行。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { validateSources } from "../diagnostics";

// mock wasm 模块：diagnostics.ts 通过动态 import("../../wasm/omnilang_wasm.js")（相对其自身目录）
// 后直接调用 compile_multi_to_omniview（无 getWasmModule 包装）。
// 本测试文件位于 __tests__/，到 src/wasm 需三级上跳（../../../wasm/omnilang_wasm.js）→ 同一模块 id。
// wasm 绑定为同步返回（Rust 导出 JsValue，JS 胶水 takeObject 直接返回值，非 Promise）→ mockReturnValue。
const { mockCompile } = vi.hoisted(() => ({ mockCompile: vi.fn() }));
vi.mock("../../../wasm/omnilang_wasm.js", () => ({
  compile_multi_to_omniview: mockCompile,
}));

const sources = { "App.omni": "component App { }" };

describe("validateSources（MultiCompileEnvelope 适配）", () => {
  beforeEach(() => {
    mockCompile.mockReset();
  });

  it("从 result.result.errors 读取诊断（MultiCompileEnvelope 内层）", async () => {
    mockCompile.mockReturnValue({
      result: {
        success: false,
        files: [],
        errors: [
          { file: "App.omni", line: 3, column: 1, message: "Undeclared variable 'x'", code: "E0001" },
        ],
        warnings: [],
        components: [],
        source_map: [],
      },
      cache_state: {},
    });
    const errors = await validateSources(sources);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toMatchObject({ message: "Undeclared variable 'x'" });
  });

  it("无错误 → 空数组", async () => {
    mockCompile.mockReturnValue({
      result: { success: true, files: [], errors: [], warnings: [], components: [], source_map: [] },
      cache_state: {},
    });
    expect(await validateSources(sources)).toEqual([]);
  });

  it("非 envelope 返回值（旧形状兼容）→ 读顶层 errors", async () => {
    mockCompile.mockReturnValue({ errors: [{ message: "legacy" }] });
    expect(await validateSources(sources)).toHaveLength(1);
  });

  it("WASM 调用失败 → 空数组（门禁放行，记录可诊断）", async () => {
    mockCompile.mockRejectedValue(new Error("wasm down"));
    expect(await validateSources(sources)).toEqual([]);
  });
});
