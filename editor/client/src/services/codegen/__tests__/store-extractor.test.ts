import { describe, it, expect } from "vitest";
import { extractStores } from "../store-extractor";
import type { ElementData, WireData } from "../../../core/types";

describe("extractStores", () => {
  it("groups signals by parentStore", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "TodoStore" }, children: [], parentId: null },
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "todos", parentStore: "TodoStore", typeAnnotation: "Array" }, children: [], parentId: "s1" },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(1);
    expect(r.stores[0].name).toBe("TodoStore");
    expect(r.stores[0].signals["todos"].type).toBe("Array");
  });

  it("standalone signals have no parentStore", () => {
    const els: ElementData[] = [
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "counter", typeAnnotation: "i32" }, children: [], parentId: null },
    ];
    const r = extractStores(els, []);
    expect(r.standaloneSignals).toHaveLength(1);
    expect(r.standaloneSignals[0].name).toBe("counter");
  });

  it("defaults type to unknown", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "S" }, children: ["sig1"], parentId: null },
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "x", parentStore: "S" }, children: [], parentId: "s1" },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(1);
    expect(r.stores[0].signals["x"].type).toBe("unknown");
  });

  it("finds parent store via parentId chain when parentStore missing", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "SettingsStore" }, children: ["sig1"], parentId: null },
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "volume", typeAnnotation: "f64" }, children: [], parentId: "s1" },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(1);
    expect(r.stores[0].signals["volume"].type).toBe("f64");
    expect(r.standaloneSignals).toHaveLength(0);
  });

  it("groups actions by parent store", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "CartStore" }, children: [], parentId: null },
      { id: "act1", nodeType: "codeblock", tag: "action", props: { name: "addItem", parentStore: "CartStore", params: [{ name: "item", type: "string" }] }, children: [], parentId: "s1" },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(1);
    expect(r.stores[0].actions).toHaveLength(1);
    expect(r.stores[0].actions[0].name).toBe("addItem");
  });

  it("handles multiple stores independently", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "A" }, children: [], parentId: null },
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "x", parentStore: "A", typeAnnotation: "i32" }, children: [], parentId: "s1" },
      { id: "s2", nodeType: "codeblock", tag: "store", props: { name: "B" }, children: [], parentId: null },
      { id: "sig2", nodeType: "codeblock", tag: "signal", props: { name: "y", parentStore: "B", typeAnnotation: "string" }, children: [], parentId: "s2" },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(2);
    expect(r.stores[0].signals["x"].type).toBe("i32");
    expect(r.stores[1].signals["y"].type).toBe("string");
  });

  it("handles computed getters with output port type as returnType", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "DataStore" }, children: [], parentId: null },
      { id: "comp1", nodeType: "codeblock", tag: "computed", props: { name: "sortedItems" }, children: [], parentId: "s1", ports: [{ name: "value", direction: "out", type: "Array<string>" }] },
    ];
    const wires: WireData[] = [];
    const r = extractStores(els, wires);
    expect(r.stores).toHaveLength(1);
    expect(r.stores[0].getters["sortedItems"].returnType).toBe("Array<string>");
  });

  it("defaults computed returnType to unknown when no output port", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "S" }, children: [], parentId: null },
      { id: "comp1", nodeType: "codeblock", tag: "computed", props: { name: "derived" }, children: [], parentId: "s1" },
    ];
    const r = extractStores(els, []);
    expect(r.stores[0].getters["derived"].returnType).toBe("unknown");
  });

  it("extracts body from action codeblock source", () => {
    const els: ElementData[] = [
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "Store" }, children: [], parentId: null },
      { id: "act1", nodeType: "codeblock", tag: "action", props: { name: "doStuff", parentStore: "Store" }, children: [], parentId: "s1", source: "this.x = 1;" },
    ];
    const r = extractStores(els, []);
    expect(r.stores[0].actions[0].body).toBe("this.x = 1;");
  });

  it("skips action with no parent store", () => {
    const els: ElementData[] = [
      { id: "act1", nodeType: "codeblock", tag: "action", props: { name: "orphan" }, children: [], parentId: null },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(0);
    expect(r.standaloneSignals).toHaveLength(0);
  });

  it("handles standalone computed (no parent store)", () => {
    const els: ElementData[] = [
      { id: "comp1", nodeType: "codeblock", tag: "computed", props: { name: "doubled" }, children: [], parentId: null, source: "count * 2", ports: [{ name: "value", direction: "out", type: "i32" }] },
    ];
    const r = extractStores(els, []);
    expect(r.standaloneComputeds).toHaveLength(1);
    expect(r.standaloneComputeds[0].name).toBe("doubled");
    expect(r.standaloneComputeds[0].type).toBe("i32");
    expect(r.standaloneComputeds[0].body).toBe("count * 2");
    expect(r.standaloneSignals).toHaveLength(0);
  });

  it("returns empty result for no codeblocks", () => {
    const r = extractStores([], []);
    expect(r.stores).toHaveLength(0);
    expect(r.standaloneSignals).toHaveLength(0);
  });

  it("ignores view elements", () => {
    const els: ElementData[] = [
      { id: "v1", nodeType: "view", tag: "button", props: { label: "Click" }, children: [], parentId: null },
    ];
    const r = extractStores(els, []);
    expect(r.stores).toHaveLength(0);
    expect(r.standaloneSignals).toHaveLength(0);
  });
});
