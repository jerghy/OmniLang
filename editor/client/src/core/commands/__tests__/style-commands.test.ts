import { describe, it, expect, beforeEach } from "vitest";
import { useStyleRegistryStore } from "../../state/styleRegistry";
import { uid } from "../../utils/id";
import { AddCssVariable } from "../AddCssVariable";
import { RemoveCssVariable } from "../RemoveCssVariable";
import { UpdateCssVariable } from "../UpdateCssVariable";
import { AddStyleClass } from "../AddStyleClass";
import { RemoveStyleClass } from "../RemoveStyleClass";
import { RenameStyleClass } from "../RenameStyleClass";
import { AddClassProperty } from "../AddClassProperty";
import { RemoveClassProperty } from "../RemoveClassProperty";
import { UpdateClassProperty } from "../UpdateClassProperty";
import type { CommandContext } from "../types";

const APP = "App";
const mockCtx: CommandContext = {
  getElement: () => undefined,
  elements: [], wires: [],
  addElement: () => {}, removeElement: () => {},
  setProp: () => {}, getProp: () => undefined,
  addWire: () => {}, removeWire: () => {},
  setElementsAndWires: () => {},
  reorderElement: () => {},
  moveElement: () => {},
};

beforeEach(() => {
  useStyleRegistryStore.setState({ registries: {} });
});

describe("style commands", () => {
  it("AddCssVariable → execute adds, undo removes", () => {
    const v = { id: uid("v"), name: "--primary", segments: [{ type: "text" as const, value: "#333" }] };
    const cmd = new AddCssVariable(APP, v);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(1);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(0);
  });

  it("RemoveCssVariable → execute removes, undo adds", () => {
    const v = { id: uid("v"), name: "--x", segments: [{ type: "text" as const, value: "1" }] };
    useStyleRegistryStore.getState().addVariable(APP, v);
    const cmd = new RemoveCssVariable(APP, v);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(0);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables).toHaveLength(1);
  });

  it("UpdateCssVariable → execute updates, undo reverts", () => {
    const vid = uid("v");
    useStyleRegistryStore.getState().addVariable(APP, { id: vid, name: "--old", segments: [{ type: "text", value: "1" }] });
    const cmd = new UpdateCssVariable(APP, vid, { name: "--old" }, { name: "--new" });
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables[0].name).toBe("--new");
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).variables[0].name).toBe("--old");
  });

  it("AddStyleClass → execute adds, undo removes", () => {
    const c = { id: uid("c"), name: "card", properties: [] };
    const cmd = new AddStyleClass(APP, c);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(1);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(0);
  });

  it("RemoveStyleClass → execute removes, undo adds", () => {
    const c = { id: uid("c"), name: "card", properties: [] };
    useStyleRegistryStore.getState().addClass(APP, c);
    const cmd = new RemoveStyleClass(APP, c);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(0);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes).toHaveLength(1);
  });

  it("RenameStyleClass → execute renames, undo reverts", () => {
    const cid = uid("c");
    useStyleRegistryStore.getState().addClass(APP, { id: cid, name: "old", properties: [] });
    const cmd = new RenameStyleClass(APP, cid, "old", "new");
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].name).toBe("new");
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].name).toBe("old");
  });

  it("AddClassProperty → execute adds, undo removes", () => {
    const cid = uid("c");
    useStyleRegistryStore.getState().addClass(APP, { id: cid, name: "btn", properties: [] });
    const p = { id: uid("p"), name: "padding", segments: [{ type: "text" as const, value: "8px" }] };
    const cmd = new AddClassProperty(APP, cid, p);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(1);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(0);
  });

  it("RemoveClassProperty → execute removes, undo adds", () => {
    const cid = uid("c");
    const p = { id: uid("p"), name: "color", segments: [{ type: "text" as const, value: "red" }] };
    useStyleRegistryStore.getState().addClass(APP, { id: cid, name: "btn", properties: [p] });
    const cmd = new RemoveClassProperty(APP, cid, p);
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(0);
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties).toHaveLength(1);
  });

  it("UpdateClassProperty → execute updates, undo reverts", () => {
    const cid = uid("c");
    const pid = uid("p");
    useStyleRegistryStore.getState().addClass(APP, {
      id: cid, name: "btn",
      properties: [{ id: pid, name: "padding", segments: [{ type: "text", value: "8px" }] }],
    });
    const cmd = new UpdateClassProperty(APP, cid, pid, { name: "padding" }, { name: "margin" });
    cmd.execute(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties[0].name).toBe("margin");
    cmd.undo(mockCtx);
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].properties[0].name).toBe("padding");
  });
});
