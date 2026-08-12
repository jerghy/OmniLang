// poiesis/src/components/design/__tests__/action-verbs.test.ts
// M3 T1 行为卡动词集：最小动词表 + 动作定义契约（label/code 存在、核心动作齐备）。
import { describe, it, expect } from "vitest";
import { ACTION_VERBS, ACTIONS } from "../action-verbs";

describe("action-verbs", () => {
  it("动词表 ≥10 项且含核心动词 点击/加/减", () => {
    expect(ACTION_VERBS.length).toBeGreaterThanOrEqual(10);
    expect(ACTION_VERBS).toContain("点击");
    expect(ACTION_VERBS).toContain("加");
    expect(ACTION_VERBS).toContain("减");
  });

  it("每个动作定义都有中文 label 与 code 生成函数", () => {
    for (const action of Object.values(ACTIONS)) {
      expect(action.label).toBeTruthy();
      expect(typeof action.code).toBe("function");
    }
    expect(ACTIONS["add"]).toBeDefined();
  });

  it("核心动作键齐备：加/减/跳转", () => {
    const keys = Object.keys(ACTIONS);
    expect(keys).toContain("add");
    expect(keys).toContain("subtract");
    expect(keys).toContain("navigate");
  });
});
