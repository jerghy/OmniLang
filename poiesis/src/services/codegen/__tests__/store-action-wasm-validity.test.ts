import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync } from "fs";
import path from "path";
import { emitStoreDefinition } from "../source-emitter";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";
import type { StoreDefinition } from "../../../core/types/store";

// ── WASM compiler bootstrap ──────────────────────────────────────────────
// The wasm-pack module self-initializes via fetch in browsers, but vitest
// runs in Node where fetch of the .wasm file fails — pass the bytes directly.
let wasmModule: typeof import("../../../wasm/omnilang_wasm.js") | null = null;

async function loadWasm(): Promise<typeof import("../../../wasm/omnilang_wasm.js")> {
  if (wasmModule) return wasmModule;
  const mod = await import("../../../wasm/omnilang_wasm.js");
  const wasmPath = path.join(process.cwd(), "src/wasm/omnilang_wasm_bg.wasm");
  await mod.default(readFileSync(wasmPath));
  wasmModule = mod;
  return mod;
}

interface WasmDiag {
  severity?: string;
  code?: string;
  message?: string;
  line?: number;
  file?: string;
}

/**
 * Compile store sources through the REAL WASM compiler (the same
 * compile_multi_to_omniview pipeline the app's save gate uses).
 * Navigates the current FFI envelope: { result: { success, errors, ... } }.
 */
async function compileStoreSources(sources: Record<string, string>): Promise<WasmDiag[]> {
  const mod = await loadWasm();
  // 4th arg (cache_state) is required by the wasm binding signature — pass
  // undefined for a bare validation compile.
  const envelope = mod.compile_multi_to_omniview(sources, false, false, undefined) as {
    result?: { errors?: WasmDiag[] };
    errors?: WasmDiag[];
  };
  return envelope?.result?.errors ?? envelope?.errors ?? [];
}

function el(id: string, tag: string, props: Record<string, unknown>): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source: "" };
}

function makeStore(elements: CodeblockElementData[], wires: WireData[]): StoreDefinition {
  return {
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
}

const fnEl = el("s1_n3_c1", "fn", { name: "newAction", params: "" });

describe("store action semantic validity (real WASM compile)", () => {
  beforeAll(async () => {
    await loadWasm();
  });

  it("emitted store source (assign-only body) compiles with zero errors", async () => {
    const elements = [
      fnEl,
      el("assign-x", "assign", { target: "newSignal2", operator: "=", value: "newSignal2 + 1" }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "s1_n3_c1", fromPort: "flow-out", toId: "assign-x", toPort: "flow-in" },
    ];
    const out = emitStoreDefinition(makeStore(elements, wires));
    // No nested fn declaration in the action body
    expect(out).not.toMatch(/fn\s+newAction\s*\(/);

    const diagnostics = await compileStoreSources({ "src/stores/demostore.omni": out });
    const errors = diagnostics.filter((d) => d.severity === "error");
    expect(errors).toEqual([]);
  });

  it("store-call in a store action is rejected by the language (cross-store call limitation, E0001 on the store name — not on the emitted syntax)", async () => {
    const elements = [
      fnEl,
      el("assign-x", "assign", { target: "newSignal2", operator: "=", value: "newSignal2 + 1" }),
      el("sc-x", "store-call", { storeName: "TodoStore", action: "add", args: "" }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "s1_n3_c1", fromPort: "flow-out", toId: "assign-x", toPort: "flow-in" },
      { id: "w2", fromId: "assign-x", fromPort: "flow-out", toId: "sc-x", toPort: "flow-in" },
    ];
    const out = emitStoreDefinition(makeStore(elements, wires));
    expect(out).not.toMatch(/fn\s+newAction\s*\(/);

    const diagnostics = await compileStoreSources({
      "src/stores/demostore.omni": out,
      "src/stores/TodoStore.omni": `import omniview;\nstore TodoStore {\n    signal items: Array<String> = [];\n}\n`,
    });
    const errors = diagnostics.filter((d) => d.severity === "error");
    // Every error must be the documented cross-store reference limitation
    // (E0001 on the store NAME) — never a structural/nested-fn failure.
    for (const e of errors) {
      expect(e.message).toContain("Undeclared variable 'TodoStore'");
    }
  });

  it("negative control: nested fn inside an action body is rejected (E0001 Undeclared variable 'fn')", async () => {
    const invalid = `import omniview;
store s {
    action a() {
        fn a() {
            q = q + 1;
        }
    }
}
`;
    const diagnostics = await compileStoreSources({ "src/stores/s.omni": invalid });
    const errors = diagnostics.filter((d) => d.severity === "error");
    expect(errors.length).toBeGreaterThan(0);
    expect(errors.some((e) => (e.message || "").includes("Undeclared variable 'fn'"))).toBe(true);
  });
});
