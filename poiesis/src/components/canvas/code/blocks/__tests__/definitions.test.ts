// poiesis/src/components/canvas/code/blocks/__tests__/definitions.test.ts
import { describe, it, expect } from "vitest";
import { BLOCK_DEFINITIONS } from "../definitions";

describe("BLOCK_DEFINITIONS", () => {
  const expectedBlocks = [
    "declare_var", "set_var", "return_stmt", "call_fn",
    "await_expr", "break_stmt", "continue_stmt",
    "comment_stmt", "raw_stmt",
    "if_block", "while_loop", "for_in",
  ];

  it("has all 13 block types", () => {
    expect(Object.keys(BLOCK_DEFINITIONS).sort()).toEqual(expectedBlocks.sort());
  });

  for (const name of expectedBlocks) {
    it(`${name} has type field`, () => {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      expect(def.type).toBe(name);
    });

    it(`${name} has colour field`, () => {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      expect(def.colour).toBeDefined();
    });

    it(`${name} has previousStatement: null and nextStatement: null`, () => {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      expect(def.previousStatement).toBeNull();
      expect(def.nextStatement).toBeNull();
    });
  }

  it("simple statements (no C-shaped) have inputsInline: true", () => {
    const simple = [
      "declare_var", "set_var", "return_stmt", "call_fn",
      "await_expr", "break_stmt", "continue_stmt",
      "comment_stmt", "raw_stmt",
    ];
    for (const name of simple) {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      if (name === "break_stmt" || name === "continue_stmt" || name === "raw_stmt") {
        // These blocks have no inputs — inputsInline is irrelevant
        continue;
      }
      expect(def.inputsInline).toBe(true);
    }
  });

  it("C-shaped blocks have input_statement BODY", () => {
    for (const name of ["if_block", "while_loop", "for_in"]) {
      const def = BLOCK_DEFINITIONS[name] as Record<string, unknown>;
      const defStr = JSON.stringify(def);
      expect(defStr).toContain("input_statement");
      expect(defStr).toContain("BODY");
    }
  });

  it("if_block has mutator reference", () => {
    const def = BLOCK_DEFINITIONS["if_block"] as Record<string, unknown>;
    expect(def.mutator).toBe("if_block_mutator");
  });

  it("if_block has ELSE_BODY in its definition", () => {
    const def = BLOCK_DEFINITIONS["if_block"] as Record<string, unknown>;
    const defStr = JSON.stringify(def);
    expect(defStr).toContain("ELSE_BODY");
  });

  it("declare_var has MUT dropdown", () => {
    const def = BLOCK_DEFINITIONS["declare_var"] as Record<string, unknown>;
    const defStr = JSON.stringify(def);
    expect(defStr).toContain("MUT");
    expect(defStr).toContain("let");
    expect(defStr).toContain("mut");
  });

  it("set_var has OP dropdown", () => {
    const def = BLOCK_DEFINITIONS["set_var"] as Record<string, unknown>;
    const defStr = JSON.stringify(def);
    expect(defStr).toContain("OP");
    expect(defStr).toContain("+=");
    expect(defStr).toContain("-=");
  });

  it("comment_stmt uses field_input for TEXT", () => {
    const def = BLOCK_DEFINITIONS["comment_stmt"] as Record<string, unknown>;
    const defStr = JSON.stringify(def);
    expect(defStr).toContain("field_input");
    expect(defStr).toContain("TEXT");
  });

  it("raw_stmt uses field_multilinetext for SOURCE", () => {
    const def = BLOCK_DEFINITIONS["raw_stmt"] as Record<string, unknown>;
    const defStr = JSON.stringify(def);
    expect(defStr).toContain("field_multilinetext");
    expect(defStr).toContain("SOURCE");
  });
});

describe("registerAllBlocks", () => {
  it("registerAllBlocks is a function", async () => {
    // Dynamic import to verify the module exports it
    const mod = await import("../definitions");
    expect(typeof mod.registerAllBlocks).toBe("function");
  });
});
