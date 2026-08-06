// debugValues 数据模型测试：ownerKey 键控 + 校验态 + 总开关。
import { describe, it, expect } from "vitest";
import { usePoiesisStore } from "../index";

function reset() {
  usePoiesisStore.setState({ debugValues: {}, debugOverridesEnabled: true });
}

describe("debugValues", () => {
  it("setDebugValue 写入 ownerKey 分组条目（含校验态）", () => {
    reset();
    usePoiesisStore.getState().setDebugValue("main", "status", "loading", true);
    const v = usePoiesisStore.getState().debugValues["main"]?.["status"];
    expect(v).toEqual({ text: "loading", valid: true });
    // 不 dirty 项目
    expect(usePoiesisStore.getState().isDirty).toBe(false);
  });

  it("不同 ownerKey 同名互不覆盖", () => {
    reset();
    const s = usePoiesisStore.getState();
    s.setDebugValue("main", "status", "loading", true);
    s.setDebugValue("TodoStore", "status", "done", true);
    expect(usePoiesisStore.getState().debugValues["main"]?.["status"]?.text).toBe("loading");
    expect(usePoiesisStore.getState().debugValues["TodoStore"]?.["status"]?.text).toBe("done");
  });

  it("clearDebugValues 清空全部", () => {
    reset();
    const s = usePoiesisStore.getState();
    s.setDebugValue("main", "count", "3", true);
    s.setDebugValue("TodoStore", "items", "a,b", true);
    s.clearDebugValues();
    expect(usePoiesisStore.getState().debugValues).toEqual({});
  });

  it("总开关默认开启，可切换", () => {
    reset();
    expect(usePoiesisStore.getState().debugOverridesEnabled).toBe(true);
    usePoiesisStore.getState().setDebugOverridesEnabled(false);
    expect(usePoiesisStore.getState().debugOverridesEnabled).toBe(false);
  });
});
