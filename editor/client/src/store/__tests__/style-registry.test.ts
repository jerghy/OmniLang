import { describe, it, expect, beforeEach } from "vitest";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
import { uid } from "../../core/utils/id";

const APP = "App";

describe("useStyleRegistryStore", () => {
  beforeEach(() => {
    useStyleRegistryStore.setState({ registries: {} });
  });

  it("addVariable stores a CSS variable under component key", () => {
    useStyleRegistryStore.getState().addVariable(APP, {
      id: uid("v"),
      name: "--primary",
      segments: [{ type: "text", value: "#6366f1" }],
    });
    const reg = useStyleRegistryStore.getState().getRegistry(APP);
    expect(reg.variables).toHaveLength(1);
    expect(reg.variables[0].name).toBe("--primary");
  });

  it("removeVariable deletes by id", () => {
    const id = uid("v");
    useStyleRegistryStore.getState().addVariable(APP, {
      id, name: "--x", segments: [{ type: "text", value: "1" }],
    });
    useStyleRegistryStore.getState().removeVariable(APP, id);
    const reg = useStyleRegistryStore.getState().getRegistry(APP);
    expect(reg.variables).toHaveLength(0);
  });

  it("updateVariable applies partial patch", () => {
    const id = uid("v");
    useStyleRegistryStore.getState().addVariable(APP, {
      id, name: "--old", segments: [{ type: "text", value: "1" }],
    });
    useStyleRegistryStore.getState().updateVariable(APP, id, { name: "--new" });
    const v = useStyleRegistryStore.getState().getRegistry(APP).variables[0];
    expect(v.name).toBe("--new");
  });

  it("addClass stores a class definition", () => {
    useStyleRegistryStore.getState().addClass(APP, {
      id: uid("c"), name: "card", properties: [],
    });
    const reg = useStyleRegistryStore.getState().getRegistry(APP);
    expect(reg.classes).toHaveLength(1);
    expect(reg.classes[0].name).toBe("card");
  });

  it("renameClass updates the class name", () => {
    const id = uid("c");
    useStyleRegistryStore.getState().addClass(APP, {
      id, name: "old-card", properties: [],
    });
    useStyleRegistryStore.getState().renameClass(APP, id, "new-card");
    expect(useStyleRegistryStore.getState().getRegistry(APP).classes[0].name).toBe("new-card");
  });

  it("addProperty appends to class", () => {
    const cid = uid("c");
    useStyleRegistryStore.getState().addClass(APP, {
      id: cid, name: "btn", properties: [],
    });
    useStyleRegistryStore.getState().addProperty(APP, cid, {
      id: uid("p"), name: "padding", segments: [{ type: "text", value: "8px" }],
    });
    const cls = useStyleRegistryStore.getState().getRegistry(APP).classes[0];
    expect(cls.properties).toHaveLength(1);
  });

  it("removeProperty deletes by property id", () => {
    const cid = uid("c");
    const pid = uid("p");
    useStyleRegistryStore.getState().addClass(APP, {
      id: cid, name: "btn", properties: [
        { id: pid, name: "color", segments: [{ type: "text", value: "red" }] },
      ],
    });
    useStyleRegistryStore.getState().removeProperty(APP, cid, pid);
    const cls = useStyleRegistryStore.getState().getRegistry(APP).classes[0];
    expect(cls.properties).toHaveLength(0);
  });

  it("getRegistry returns empty default for unknown component", () => {
    const reg = useStyleRegistryStore.getState().getRegistry("UnknownComponent");
    expect(reg.variables).toEqual([]);
    expect(reg.classes).toEqual([]);
  });
});
