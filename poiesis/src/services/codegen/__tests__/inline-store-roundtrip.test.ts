// poiesis/src/services/codegen/__tests__/inline-store-roundtrip.test.ts
// Round-trip verification: inline store definitions inside component bodies
// survive parse -> emit -> parse cycle.

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { SourceEmitter } from "../source-emitter";
import type { EmitInput } from "../types";
import type { StoreDefinition } from "../../../core/types/store";

// ---------------------------------------------------------------------------
// Helper: emit an EmitInput with inlineStores back to source via SourceEmitter.
// ---------------------------------------------------------------------------

function emitWithInlineStore(source: string): string {
  // First parse to extract inline store defs and component structure
  const r1 = parseOmniSource(source);

  // Build EmitInput from the parsed result
  const inlineStores = Object.values(r1.inlineStoreDefs ?? {});
  const input: EmitInput = {
    codeTabType: "logic",
    imports: r1.imports ?? [],
    stores: [],
    component: {
      status: "valid",
      name: r1.inlineStoreDefs
        ? Object.keys(r1.inlineStoreDefs)[0]?.split(".")[0] ?? "App"
        : "App",
      signals: {},
      computeds: {},
      storeRefs: [],
      fns: [],
      style: {},
      view: { id: "v_root", path: "root", kind: "text", text: "" },
      consts: [],
      props: r1.props ?? [],
      events: r1.events ?? [],
    },
    inlineStores: inlineStores.length > 0 ? inlineStores : undefined,
  };

  const emitter = new SourceEmitter();
  return emitter.emitDocument(input, new Map());
}

describe("Inline store round-trip", () => {
  it("emitInlineStore produces valid store block inside component", () => {
    const source = `component App {
    store Cart {
        signal items: Array<String> = [];
        getter count: i32 = items.len();
        action add(name: String) {
        }
    }
    view {
        <div></div>
    }
}`;
    const emitted = emitWithInlineStore(source);
    expect(emitted).toContain("store Cart");
    expect(emitted).toContain("signal items: Array<String>");
    expect(emitted).toContain("getter count: i32");
    expect(emitted).toContain("action add(name: String)");
    // Store should appear inside component body
    const compOpen = emitted.indexOf("component App {");
    const storeOpen = emitted.indexOf("store Cart");
    expect(compOpen).toBeGreaterThanOrEqual(0);
    expect(storeOpen).toBeGreaterThan(compOpen);
  });

  it("roundtrip: inline store survives parse -> emit -> parse", () => {
    const source = `component App {
    store Cart {
        signal items: Array<String> = [];
        getter count: i32 = items.len();
        action add(name: String) {
        }
    }
    view {
        <div></div>
    }
}`;
    // Parse -> gets inlineStoreDefs
    const r1 = parseOmniSource(source);
    expect(r1.inlineStoreDefs).toBeDefined();

    // Emit via SourceEmitter
    const emitted = emitWithInlineStore(source);
    expect(emitted).toBeTruthy();

    // Re-parse emitted source
    const r2 = parseOmniSource(emitted);
    expect(r2.inlineStoreDefs).toBeDefined();
    expect(r2.inlineStoreDefs!["App.Cart"]).toBeDefined();

    const store = r2.inlineStoreDefs!["App.Cart"];
    expect(store.name).toBe("App.Cart");
    expect(store.signals).toHaveLength(1);
    expect(store.signals[0].name).toBe("items");
    expect(store.getters).toHaveLength(1);
    expect(store.getters[0].name).toBe("count");
    expect(store.actions).toHaveLength(1);
    expect(store.actions[0].name).toBe("add");
  });

  it("roundtrip: multiple inline stores survive", () => {
    const source = `component App {
    store A { signal x: String = ""; }
    store B { signal y: i32 = 0; }
    view { <div></div>
    }
}`;
    const r1 = parseOmniSource(source);
    expect(Object.keys(r1.inlineStoreDefs ?? {})).toHaveLength(2);

    const emitted = emitWithInlineStore(source);
    const r2 = parseOmniSource(emitted);
    expect(Object.keys(r2.inlineStoreDefs ?? {})).toHaveLength(2);
    expect(r2.inlineStoreDefs!["App.A"]).toBeDefined();
    expect(r2.inlineStoreDefs!["App.B"]).toBeDefined();
  });

  it("roundtrip: inline store with no signals/getters/actions survives", () => {
    const source = `component App {
    store EmptyStore {
    }
    view { <div></div>
    }
}`;
    const emitted = emitWithInlineStore(source);
    const r2 = parseOmniSource(emitted);
    expect(r2.inlineStoreDefs!["App.EmptyStore"]).toBeDefined();
    const store = r2.inlineStoreDefs!["App.EmptyStore"];
    expect(store.signals).toHaveLength(0);
    expect(store.getters).toHaveLength(0);
    expect(store.actions).toHaveLength(0);
  });
});
