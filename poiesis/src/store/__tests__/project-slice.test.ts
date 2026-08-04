// poiesis/src/store/__tests__/project-slice.test.ts
// 项目切片重置语义：resetEditorState / newProject 共享 createCleanState，
// 必须清空跨项目残留状态（persistedPositions——pid 可跨项目重复，不清会套旧位置）。

import { describe, it, expect } from "vitest";
import { usePoiesisStore } from "../index";

describe("project slice（重置语义）", () => {
  it("resetEditorState 清空 persistedPositions（pid 跨项目可重复不套旧位置）", () => {
    usePoiesisStore.setState({ persistedPositions: { fn_a1: { x: 120, y: 340 } } });
    usePoiesisStore.getState().resetEditorState();
    expect(usePoiesisStore.getState().persistedPositions).toEqual({});
  });

  it("newProject 清空 persistedPositions 并重置项目名", () => {
    usePoiesisStore.setState({ persistedPositions: { fn_a1: { x: 120, y: 340 } } });
    usePoiesisStore.getState().newProject();
    expect(usePoiesisStore.getState().persistedPositions).toEqual({});
    expect(usePoiesisStore.getState().projectName).toBe("未命名项目");
    expect(usePoiesisStore.getState().projectDir).toBeNull();
  });
});
