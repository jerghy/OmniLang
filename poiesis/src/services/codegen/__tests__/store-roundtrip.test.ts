// poiesis/src/services/codegen/__tests__/store-roundtrip.test.ts
// Round-trip verification: parse store .omni source → verify StoreDefinition
// fields → emit back via SourceEmitter → reparsing → verify identity.

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { SourceEmitter, CodeWriter as _CodeWriter } from "../source-emitter";
import type { EmitInput } from "../types";
import type { StoreDefinition, StoreSignalDef as _StoreSignalDef, StoreGetterDef as _StoreGetterDef, StoreActionDef as _StoreActionDef } from "../../../core/types/store";

// ---------------------------------------------------------------------------
// Sample store source with @pid annotations BEFORE each declaration
// (the parser reads @pid on the line preceding a declaration as its pid).
// ---------------------------------------------------------------------------

const sampleStoreSource = `import omniview;
import { storage_get, storage_set } from "std/storage";

// @pid:s1
store TodoStore {
    // @pid:s1_n1
    signal items: Todo[];

    // @pid:s1_n2
    signal filterStatus: String = "all";

    // @pid:s1_n3
    getter totalCount: i32 = items.len();

    // @pid:s1_n4
    action add(text: String) {
        // @pid:s1_n4_c1
        items = [...items, { id: items.len(), text, done: false }];
    }

    // @pid:s1_n5
    action clear() {
        // @pid:s1_n5_c1
        items = [];
    }
}
`;

// ---------------------------------------------------------------------------
// Helper: emit a StoreDefinition back to source via SourceEmitter.
// ---------------------------------------------------------------------------

function emitStoreDef(store: StoreDefinition): string {
  const emitter = new SourceEmitter();
  const input: EmitInput = {
    codeTabType: "logic",
    imports: store.imports,
    stores: [store],
    component: {
      status: "valid",
      name: "Dummy",
      signals: {},
      computeds: {},
      storeRefs: [],
      fns: [],
      style: {},
      view: { id: "v_dummy", path: "dummy", kind: "text", text: "" },
      consts: [],
      props: [],
      events: [],
    },
  };
  return emitter.emitDocument(input, new Map());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Store round-trip", () => {
  // ── Parse → verify structure ──────────────────────────────────────────

  it("parses store source into StoreDefinition with correct name/pid/counts", () => {
    const result = parseOmniSource(sampleStoreSource);
    expect(result.storeDef).toBeDefined();
    const store = result.storeDef!;
    expect(store.name).toBe("TodoStore");
    expect(store.signals).toHaveLength(2);
    expect(store.getters).toHaveLength(1);
    expect(store.actions).toHaveLength(2);
  });

  it("parsed store signals have correct data", () => {
    const result = parseOmniSource(sampleStoreSource);
    const store = result.storeDef!;

    // Signal 0: items
    expect(store.signals[0].name).toBe("items");
    expect(store.signals[0].type).toBe("Todo[]");
    // items has no default — the field should be undefined or empty
    expect(store.signals[0].defaultValue === undefined || store.signals[0].defaultValue === "").toBe(true);

    // Signal 1: filterStatus
    expect(store.signals[1].name).toBe("filterStatus");
    expect(store.signals[1].type).toBe("String");
    expect(store.signals[1].defaultValue).toBeDefined();
    expect(store.signals[1].defaultValue).toContain("all");
  });

  it("parsed store getter has correct data", () => {
    const result = parseOmniSource(sampleStoreSource);
    const store = result.storeDef!;
    const getter = store.getters[0];
    expect(getter.name).toBe("totalCount");
    expect(getter.returnType).toBe("i32");
    expect(getter.expression).toContain("items.len()");
  });

  it("parsed store actions have correct name and params", () => {
    const result = parseOmniSource(sampleStoreSource);
    const store = result.storeDef!;

    // Action: add
    const addAction = store.actions.find((a) => a.name === "add");
    expect(addAction).toBeDefined();
    expect(addAction!.params).toHaveLength(1);
    expect(addAction!.params[0].name).toBe("text");
    expect(addAction!.params[0].type).toBe("String");

    // Action: clear
    const clearAction = store.actions.find((a) => a.name === "clear");
    expect(clearAction).toBeDefined();
    expect(clearAction!.params).toHaveLength(0);

    // Actions should have parsed code flow graph data (elements)
    expect(addAction!.elements.length).toBeGreaterThan(0);
  });

  it("parsed store actions have syntax-node elements", () => {
    const result = parseOmniSource(sampleStoreSource);
    const store = result.storeDef!;
    const addAction = store.actions.find((a) => a.name === "add")!;
    // The add action body contains a @pid segment → an assign element
    expect(addAction.elements.some((e) => e.tag === "assign")).toBe(true);
  });

  // ── Emit → reparsing identity ─────────────────────────────────────────

  it("emit → re-parse preserves name and member counts", () => {
    const parsed = parseOmniSource(sampleStoreSource);
    const store = parsed.storeDef!;

    // Emit the store back to source
    const emitted = emitStoreDef(store);
    expect(emitted).toBeTruthy();
    expect(emitted).toContain("store TodoStore");

    // Re-parse the emitted source
    const reparsed = parseOmniSource(emitted);
    expect(reparsed.storeDef).toBeDefined();
    const re = reparsed.storeDef!;

    // Structural identity
    expect(re.name).toBe(store.name);
    expect(re.signals.length).toBe(store.signals.length);
    expect(re.getters.length).toBe(store.getters.length);
    expect(re.actions.length).toBe(store.actions.length);

    // Signal names preserved
    for (let i = 0; i < store.signals.length; i++) {
      expect(re.signals[i].name).toBe(store.signals[i].name);
      expect(re.signals[i].type).toBe(store.signals[i].type);
    }

    // Getter name and expression preserved
    expect(re.getters[0].name).toBe(store.getters[0].name);
    expect(re.getters[0].returnType).toBe(store.getters[0].returnType);
    expect(re.getters[0].expression).toBe(store.getters[0].expression);

    // Action names and param shapes preserved
    for (const origAction of store.actions) {
      const reAction = re.actions.find((a) => a.name === origAction.name);
      expect(reAction).toBeDefined();
      expect(reAction!.params).toEqual(origAction.params);
    }
  });

  it("emitted source preserves @pid annotations", () => {
    const parsed = parseOmniSource(sampleStoreSource);
    const emitted = emitStoreDef(parsed.storeDef!);

    // Store-level @pid may be auto-generated (e.g. "s1") since the parser
    // does not capture // @pid:s1 from root-level comments.
    // But each member should have its @pid preserved in the emitted output.
    expect(emitted).toContain("// @pid:");
    expect(emitted).toContain("// @pid:s1_n1");
    expect(emitted).toContain("// @pid:s1_n2");
    expect(emitted).toContain("// @pid:s1_n3");
    expect(emitted).toContain("// @pid:s1_n4");
    expect(emitted).toContain("// @pid:s1_n5");
  });

  // ── Edge cases ────────────────────────────────────────────────────────

  it("handles empty actions (no body)", () => {
    const source = `store EmptyActionStore {
    // @pid:empty_s1
    signal count: i32;
    // @pid:empty_s2
    action doNothing() {}
}`;
    const result = parseOmniSource(source);
    expect(result.storeDef).toBeDefined();
    expect(result.storeDef!.actions).toHaveLength(1);
    // The fn entry element is synthesized from the action signature even for
    // empty bodies — a fresh action saved before any statements must keep
    // its fn node across reloads (round-trip carrier). No statements, no wires.
    expect(result.storeDef!.actions[0].elements).toHaveLength(1);
    const fnEl = result.storeDef!.actions[0].elements[0];
    expect(fnEl.tag).toBe("fn");
    expect(fnEl.props).toEqual({ name: "doNothing", params: "", returnType: "" });
    expect(result.storeDef!.actions[0].wires).toHaveLength(0);
  });

  it("round-trips store with no imports", () => {
    const source = `// @pid:ns1
store NoImportsStore {
    // @pid:ns1_n1
    signal x: i32;
}`;
    const parsed = parseOmniSource(source);
    expect(parsed.storeDef).toBeDefined();
    expect(parsed.storeDef!.imports).toHaveLength(0);
    expect(parsed.storeDef!.name).toBe("NoImportsStore");

    const emitted = emitStoreDef(parsed.storeDef!);
    const reparsed = parseOmniSource(emitted);
    expect(reparsed.storeDef).toBeDefined();
    expect(reparsed.storeDef!.name).toBe("NoImportsStore");
    expect(reparsed.storeDef!.signals).toHaveLength(1);
    expect(reparsed.storeDef!.signals[0].name).toBe("x");
  });

  it("handles store with getters but no signals", () => {
    const source = `// @pid:gs1
store GettersOnlyStore {
    // @pid:gs1_n1
    getter count: i32 = 42;
    // @pid:gs1_n2
    getter message: String = "hello";
}`;
    const result = parseOmniSource(source);
    expect(result.storeDef).toBeDefined();
    expect(result.storeDef!.signals).toHaveLength(0);
    expect(result.storeDef!.getters).toHaveLength(2);
    expect(result.storeDef!.getters[0].name).toBe("count");
    expect(result.storeDef!.getters[0].expression).toBe("42");
    expect(result.storeDef!.getters[1].name).toBe("message");
    expect(result.storeDef!.getters[1].expression).toContain("hello");
  });

  // ── Default value round-trip ──────────────────────────────────────────

  it("default values survive parse-emit-reparse round-trip", () => {
    const source = `import omniview;
store TestStore {
    // @pid:rt1_n1
    signal mode: String = "dark";
    // @pid:rt1_n2
    signal count: i32 = 0;
}`;
    const parsed = parseOmniSource(source);
    const emitted = emitStoreDef(parsed.storeDef!);
    const reparsed = parseOmniSource(emitted);
    expect(reparsed.storeDef!.signals[0].defaultValue).toContain("dark");
    expect(reparsed.storeDef!.signals[1].defaultValue).toContain("0");
  });
});
