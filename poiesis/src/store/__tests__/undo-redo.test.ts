// poiesis/src/store/__tests__/undo-redo.test.ts
// redo 方向性回归（E2E 缺陷 1）：
// undo 栈记录正向动作，computeInverse 按方向（undo/redo）应用——
// add/remove/wire/setProp 各自 undo → redo → 状态恢复 的完整循环。
// 旧实现只存逆动作/单方向数据，redo 方向与 undo 相同 → redo 全部失效。

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { ConditionalBlockData } from "../../core/types/element";
import { createInitialDesignPageState } from "../operations/tab-lifecycle";

function setupStore(): void {
  usePoiesisStore.setState({
    elements: [
      { id: "root", nodeType: "view", tag: "div", props: {}, children: [], parentId: null },
    ],
    wires: [],
    selectedElementIds: [],
    rootOrder: ["root"],
    isDirty: false,
    undoStacks: {},
    designPage: {
      ...createInitialDesignPageState(),
      activeTabId: "tab-1",
      openTabs: [{ id: "tab-1", filePath: "TestComp.omni", tabType: "component", componentName: "TestComp", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} } }],
    },
  });
}

describe("undo/redo 完整循环", () => {
  beforeEach(setupStore);

  it("addElement: undo 移除 → redo 恢复", () => {
    const store = usePoiesisStore.getState();
    store.addElement({ id: "el-1", nodeType: "codeblock", tag: "let", props: { name: "x" }, parentId: null, source: "let x;" });
    expect(usePoiesisStore.getState().elements.some((e) => e.id === "el-1")).toBe(true);

    usePoiesisStore.getState().undo();
    expect(usePoiesisStore.getState().elements.some((e) => e.id === "el-1")).toBe(false);

    usePoiesisStore.getState().redo();
    const s = usePoiesisStore.getState();
    const el = s.elements.find((e) => e.id === "el-1");
    expect(el).toBeDefined();
    expect(el!.nodeType).toBe("codeblock");
    expect("tag" in el! && el!.tag).toBe("let");
    expect("props" in el! && el!.props).toMatchObject({ name: "x" });
    expect(s.rootOrder).toContain("el-1");
  });

  it("removeElement: undo 恢复 → redo 再移除（含 wire 清理）", () => {
    const store = usePoiesisStore.getState();
    store.addElement({ id: "el-1", nodeType: "codeblock", tag: "let", props: { name: "x" }, parentId: null, source: "let x;" });
    store.addWire({ id: "w-1", fromId: "root", fromPort: "flow-out", toId: "el-1", toPort: "flow-in" });
    // 清空栈后独立验证 remove 循环
    usePoiesisStore.setState({ undoStacks: {} });

    usePoiesisStore.getState().removeElement("el-1");
    expect(usePoiesisStore.getState().elements.some((e) => e.id === "el-1")).toBe(false);
    expect(usePoiesisStore.getState().wires).toHaveLength(0);

    usePoiesisStore.getState().undo();
    expect(usePoiesisStore.getState().elements.some((e) => e.id === "el-1")).toBe(true);
    expect(usePoiesisStore.getState().wires.some((w) => w.id === "w-1")).toBe(true);

    usePoiesisStore.getState().redo();
    expect(usePoiesisStore.getState().elements.some((e) => e.id === "el-1")).toBe(false);
    expect(usePoiesisStore.getState().wires).toHaveLength(0);
  });

  it("addWire: undo 移除 → redo 恢复", () => {
    const store = usePoiesisStore.getState();
    store.addElement({ id: "el-1", nodeType: "codeblock", tag: "let", props: { name: "x" }, parentId: null, source: "let x;" });
    usePoiesisStore.setState({ undoStacks: {} });

    usePoiesisStore.getState().addWire({ id: "w-1", fromId: "root", fromPort: "flow-out", toId: "el-1", toPort: "flow-in" });
    expect(usePoiesisStore.getState().wires).toHaveLength(1);

    usePoiesisStore.getState().undo();
    expect(usePoiesisStore.getState().wires).toHaveLength(0);

    usePoiesisStore.getState().redo();
    expect(usePoiesisStore.getState().wires).toHaveLength(1);
    expect(usePoiesisStore.getState().wires[0]).toMatchObject({ id: "w-1", toId: "el-1" });
  });

  it("removeWire: undo 恢复 → redo 再移除", () => {
    const store = usePoiesisStore.getState();
    store.addElement({ id: "el-1", nodeType: "codeblock", tag: "let", props: { name: "x" }, parentId: null, source: "let x;" });
    store.addWire({ id: "w-1", fromId: "root", fromPort: "flow-out", toId: "el-1", toPort: "flow-in" });
    usePoiesisStore.setState({ undoStacks: {} });

    usePoiesisStore.getState().removeWire("w-1");
    expect(usePoiesisStore.getState().wires).toHaveLength(0);

    usePoiesisStore.getState().undo();
    expect(usePoiesisStore.getState().wires.some((w) => w.id === "w-1")).toBe(true);

    usePoiesisStore.getState().redo();
    expect(usePoiesisStore.getState().wires).toHaveLength(0);
  });

  it("setProp: undo 还原旧值 → redo 恢复新值", () => {
    const store = usePoiesisStore.getState();
    store.addElement({ id: "el-1", nodeType: "codeblock", tag: "let", props: { name: "x" }, parentId: null, source: "let x;" });
    usePoiesisStore.setState({ undoStacks: {} });

    const propsOf = (): Record<string, unknown> => {
      const el = usePoiesisStore.getState().elements.find((e) => e.id === "el-1")!;
      return "props" in el ? el.props : {};
    };

    usePoiesisStore.getState().setProp("el-1", "name", "y");
    expect(propsOf()).toMatchObject({ name: "y" });

    usePoiesisStore.getState().undo();
    expect(propsOf()).toMatchObject({ name: "x" });

    usePoiesisStore.getState().redo();
    expect(propsOf()).toMatchObject({ name: "y" });
  });

  it("链式：add → remove → undo×2 → redo×2 状态收敛", () => {
    const store = usePoiesisStore.getState();
    store.addElement({ id: "el-1", nodeType: "codeblock", tag: "let", props: { name: "x" }, parentId: null, source: "let x;" });
    store.addElement({ id: "el-2", nodeType: "codeblock", tag: "assign", props: {}, parentId: null, source: "x = 1;" });
    store.removeElement("el-1");

    // 当前：只有 el-2
    expect(usePoiesisStore.getState().elements.map((e) => e.id).sort()).toEqual(["el-2", "root"]);

    // undo#1（removeElement el-1）：恢复 el-1 → [root, el-2, el-1]
    usePoiesisStore.getState().undo();
    // undo#2（addElement el-2）：移除 el-2 → [root, el-1]
    usePoiesisStore.getState().undo();
    expect(usePoiesisStore.getState().elements.map((e) => e.id)).toEqual(["root", "el-1"]);

    // redo#1（addElement el-2）：恢复 el-2 → [root, el-1, el-2]
    usePoiesisStore.getState().redo();
    // redo#2（removeElement el-1）：再删 el-1 → 收敛回 [root, el-2]
    usePoiesisStore.getState().redo();
    expect(usePoiesisStore.getState().elements.map((e) => e.id).sort()).toEqual(["el-2", "root"]);
  });

  it("conditional: addConditionalBranch undo → redo 分支结构恢复", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");
    const condId = usePoiesisStore.getState().elements.find(
      (e) => e.nodeType === "conditional"
    )!.id;
    usePoiesisStore.setState({ undoStacks: {} });

    store.addConditionalBranch(condId, "elif");
    const withElif = usePoiesisStore.getState().elements.find(
      (e) => e.id === condId
    ) as ConditionalBlockData;
    expect(withElif.branches.map((b) => b.type)).toEqual(["if", "elif", "else"]);

    usePoiesisStore.getState().undo();
    const afterUndo = usePoiesisStore.getState().elements.find(
      (e) => e.id === condId
    ) as ConditionalBlockData;
    expect(afterUndo.branches.map((b) => b.type)).toEqual(["if", "else"]);

    usePoiesisStore.getState().redo();
    const afterRedo = usePoiesisStore.getState().elements.find(
      (e) => e.id === condId
    ) as ConditionalBlockData;
    expect(afterRedo.branches.map((b) => b.type)).toEqual(["if", "elif", "else"]);
  });
});
