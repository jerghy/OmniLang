// poiesis/src/services/codegen/__tests__/parser-inline-store.test.ts
// Tests for inline store blocks inside component { ... } body.

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("Inline store parsing", () => {
  it("parses inline store inside component", () => {
    const source = `
component TodoApp {
    store CartStore {
        signal items: Array<String> = [];
        getter count: i32 = items.len();
        action add(name: String) {
            // body
        }
    }
    view {
        <div></div>
    }
}`;
    const result = parseOmniSource(source);
    expect(result.inlineStoreDefs).toBeDefined();
    expect(result.inlineStoreDefs!["TodoApp.CartStore"]).toBeDefined();
    const store = result.inlineStoreDefs!["TodoApp.CartStore"];
    expect(store.name).toBe("TodoApp.CartStore");
    expect(store.signals).toHaveLength(1);
    expect(store.signals[0].name).toBe("items");
    expect(store.getters).toHaveLength(1);
    expect(store.getters[0].name).toBe("count");
    expect(store.actions).toHaveLength(1);
    expect(store.actions[0].name).toBe("add");
  });

  it("parses multiple inline stores", () => {
    const source = `
component App {
    store A { signal x: String = ""; }
    store B { signal y: i32 = 0; }
    view { <div></div> }
}`;
    const result = parseOmniSource(source);
    expect(Object.keys(result.inlineStoreDefs ?? {})).toHaveLength(2);
  });

  it("inline store does not affect standalone storeDef", () => {
    const source = `
component App {
    store Cart { signal items: Array<String> = []; }
    view { <div></div> }
}`;
    const result = parseOmniSource(source);
    expect(result.storeDef).toBeUndefined();
    expect(result.inlineStoreDefs).toBeDefined();
  });

  it("root-level store still populates storeDef", () => {
    const source = `
store TodoStore {
    signal items: Array<String> = [];
    getter count: i32 = items.len();
}`;
    const result = parseOmniSource(source);
    expect(result.storeDef).toBeDefined();
    expect(result.storeDef!.name).toBe("TodoStore");
    expect(result.inlineStoreDefs).toBeUndefined();
  });

  it("parses inline store with no signals/getters/actions", () => {
    const source = `
component App {
    store EmptyStore {
    }
    view { <div></div> }
}`;
    const result = parseOmniSource(source);
    expect(result.inlineStoreDefs!["App.EmptyStore"]).toBeDefined();
    const store = result.inlineStoreDefs!["App.EmptyStore"];
    expect(store.signals).toHaveLength(0);
    expect(store.getters).toHaveLength(0);
    expect(store.actions).toHaveLength(0);
  });
});
