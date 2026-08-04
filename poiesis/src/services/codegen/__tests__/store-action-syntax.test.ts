import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import { emitStoreDefinition } from "../source-emitter";
import { parseOmniSource } from "../omni-parser";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { StoreDefinition } from "../../../core/types/store";

function el(id: string, tag: string, props: Record<string, unknown>): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source: "" };
}

describe("store action syntax-node emission", () => {
  const elements = [
    el("s1_n3_c1", "fn", { name: "newAction", params: "" }),
    el("assign-mse1uliz-7el95b-1l", "assign", { target: "newSignal2", operator: "=", value: "newSignal2 + 1" }),
    el("store-call-mse1utoy-7o6eoo-1m", "store-call", { storeName: "TodoStore", action: "add", args: "" }),
  ];
  const wires: WireData[] = [
    { id: "w1", fromId: "s1_n3_c1", fromPort: "flow-out", toId: "assign-mse1uliz-7el95b-1l", toPort: "flow-in" },
    { id: "w2", fromId: "assign-mse1uliz-7el95b-1l", fromPort: "flow-out", toId: "store-call-mse1utoy-7o6eoo-1m", toPort: "flow-in" },
  ];

  it("compileCodeFlowGraph produces the fn body", () => {
    const out = compileCodeFlowGraph(elements, wires, "s1_n3_c1");
    expect(out).toContain("fn newAction() {");
    expect(out).toContain("newSignal2 = newSignal2 + 1;");
    expect(out).toContain("TodoStore.add();");
  });

  it("emitStoreDefinition produces a well-formed action block", () => {
    const storeDef: StoreDefinition = {
      pid: "s1",
      name: "demostore",
      filePath: "src/stores/demostore.omni",
      imports: [],
      signals: [
        { pid: "s1_n1", name: "newSignal", type: "String", defaultValue: "few" },
        { pid: "s1_n2", name: "newSignal2", type: "i32", defaultValue: 12 },
      ],
      getters: [],
      actions: [
        { pid: "s1_n3", name: "newAction", params: [], returnType: undefined, isAsync: false, elements, wires },
      ],
    };
    const out = emitStoreDefinition(storeDef);
    expect(out).toContain("action newAction() {");
    expect(out).toContain("newSignal2 = newSignal2 + 1;");
    expect(out).toContain("TodoStore.add();");
    // Action bodies must NOT contain nested fn declarations (invalid
    // OmniLang — stmt parser rejects them with E0001).
    expect(out).not.toMatch(/fn\s+newAction\s*\(/);
    // Statement @pid comments preserved for round-trip reconstruction
    expect(out).toContain("// @pid:assign-mse1uliz-7el95b-1l");
    expect(out).toContain("// @pid:store-call-mse1utoy-7o6eoo-1m");
    // The action block must be balanced — one closing brace for the action,
    // one for the store, nothing else.
    const actionOpen = out.indexOf("action newAction() {");
    const firstClose = out.indexOf("}", actionOpen);
    expect(firstClose).toBeGreaterThan(actionOpen);
    const actionClose = out.indexOf("}", firstClose + 1);
    expect(actionClose).toBeGreaterThan(firstClose);
    expect(out.slice(actionClose + 1).trim()).toBe("");
  });

  it("emitted store source re-parses with fn/assign/store-call elements preserved", () => {
    const storeDef: StoreDefinition = {
      pid: "s1",
      name: "demostore",
      filePath: "src/stores/demostore.omni",
      imports: [],
      signals: [
        { pid: "s1_n1", name: "newSignal", type: "String", defaultValue: "few" },
        { pid: "s1_n2", name: "newSignal2", type: "i32", defaultValue: 12 },
      ],
      getters: [],
      actions: [
        { pid: "s1_n3", name: "newAction", params: [], returnType: undefined, isAsync: false, elements, wires },
      ],
    };
    const out = emitStoreDefinition(storeDef);
    const reparsed = parseOmniSource(out);
    const re = reparsed.storeDef!;
    const reAction = re.actions.find((a) => a.name === "newAction")!;
    expect(reAction.elements.map((e) => e.tag)).toEqual(["fn", "assign", "store-call"]);
    const reAssign = reAction.elements.find((e) => e.tag === "assign")!;
    expect(reAssign.props).toEqual({ target: "newSignal2", operator: "=", value: "newSignal2 + 1" });
    const reStoreCall = reAction.elements.find((e) => e.tag === "store-call")!;
    expect(reStoreCall.props).toEqual({ storeName: "TodoStore", action: "add", args: "" });
    // fn element synthesized from the action signature (round-trip carrier)
    const reFn = reAction.elements.find((e) => e.tag === "fn")!;
    expect(reFn.id).toBe("s1_n3_fn");
    expect(reFn.props).toEqual({ name: "newAction", params: "", returnType: "" });
    expect(reAction.elements[0].id).toBe(reFn.id);
    // Wiring preserved incl. the synthesized fn → first statement edge
    expect(reAction.wires.some((w) => w.fromId === reFn.id && w.toId === reAssign.id)).toBe(true);
    expect(reAction.wires.some((w) => w.fromId === reAssign.id && w.toId === reStoreCall.id)).toBe(true);
  });
});
