// poiesis/src/store/__tests__/sleep-awake.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";

// 睡/醒状态机（spec §17.3 状态规则）——瞬态视图状态，不入 undo、不持久化。
describe("睡/醒状态机", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false, mainPanelTab: "structure" },
      selectedElementIds: [],
    });
  });

  it("呼吸点：setAwake 翻转整页醒", () => {
    usePoiesisStore.getState().setAwake(true);
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
    usePoiesisStore.getState().setAwake(false);
    expect(usePoiesisStore.getState().designPage.awake).toBe(false);
  });

  it("[内容] 标签自动醒：setMainPanelTab('content') → awake = true", () => {
    usePoiesisStore.getState().setMainPanelTab("content");
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
    expect(usePoiesisStore.getState().designPage.mainPanelTab).toBe("content");
  });

  it("切回 [结构]/[行为] 不睡：睡/醒正交于标签切换（Q12-A）", () => {
    usePoiesisStore.getState().setMainPanelTab("content"); // 醒
    usePoiesisStore.getState().setMainPanelTab("structure");
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
  });

  it("选中集合 = 醒区：取消选中睡回", () => {
    usePoiesisStore.setState({ selectedElementIds: ["a", "b"] });
    expect(usePoiesisStore.getState().selectedElementIds).toHaveLength(2);
    usePoiesisStore.getState().clearSelection();
    expect(usePoiesisStore.getState().selectedElementIds).toHaveLength(0);
  });
});
