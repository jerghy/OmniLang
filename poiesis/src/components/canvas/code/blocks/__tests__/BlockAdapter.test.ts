// poiesis/src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts
// Roundtrip tests for BlockAdapter: CodeOp[] → Blockly blocks → CodeOp[]

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as Blockly from "blockly";
import { BlockAdapter } from "../BlockAdapter";
import type { CodeOp } from "../../../../../core/types/code-op";
import { createBodyId } from "../../../../../core/types/code-op";
import type { SlotValue } from "../../../../../core/types/slot-value";

// ── Helpers ──

/**
 * Create a simple non-SVG workspace for roundtrip tests.
 * Uses Blockly.Workspace (not WorkspaceSvg) to avoid DOM requirements.
 */
function makeWorkspace(): Blockly.Workspace {
  return new Blockly.Workspace();
}

/**
 * Recursively strip pid and bodyId fields from an operation tree,
 * returning a plain JSON-serializable structure for comparison.
 */
function stripMeta(ops: CodeOp[]): unknown[] {
  return ops.map((op) => {
    const stripped: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(op)) {
      if (key === "pid" || key === "bodyId") continue;
      if (key === "branches" && Array.isArray(value)) {
        stripped[key] = (value as NonNullable<unknown>[]).map(
          (b: unknown) => {
            const br = b as { condition: SlotValue; body: CodeOp[] };
            return {
              condition: br.condition,
              body: stripMeta(br.body),
            };
          },
        );
      } else if (key === "body" && Array.isArray(value)) {
        stripped[key] = stripMeta(value as CodeOp[]);
      } else if (key === "args" && Array.isArray(value)) {
        // args is SlotValue[] — keep as-is
        stripped[key] = value;
      } else {
        stripped[key] = value as unknown;
      }
    }
    return stripped;
  });
}

function roundtrip(ops: CodeOp[]): CodeOp[] {
  const adapter = new BlockAdapter();
  const ws = makeWorkspace();
  adapter.toBlocks(ops, ws);
  return adapter.fromBlocks(ws);
}

// ── Tests ──

describe("BlockAdapter roundtrip", () => {
  it("empty ops → empty result", () => {
    const ops: CodeOp[] = [];
    const result = roundtrip(ops);
    expect(result).toEqual([]);
  });

  it("declare_var (let) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "declare_var",
        pid: "p1",
        mutable: false,
        name: { kind: "text", value: "x" },
        scope: "fn",
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("declare_var (let mut) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "declare_var",
        pid: "p1",
        mutable: true,
        name: { kind: "text", value: "counter" },
        scope: "fn",
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("declare_var with type and value roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "declare_var",
        pid: "p1",
        mutable: false,
        name: { kind: "text", value: "msg" },
        varType: { kind: "text", value: "string" },
        initValue: { kind: "text", value: '"hello"' },
        scope: "fn",
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("set_var roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "set_var",
        pid: "p1",
        name: { kind: "text", value: "x" },
        operator: "=",
        value: { kind: "text", value: "42" },
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("set_var with += operator roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "set_var",
        pid: "p1",
        name: { kind: "text", value: "count" },
        operator: "+=",
        value: { kind: "text", value: "1" },
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("set_var with -= operator roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "set_var",
        pid: "p1",
        name: { kind: "text", value: "count" },
        operator: "-=",
        value: { kind: "text", value: "1" },
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("return with value roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "return",
        pid: "p1",
        value: { kind: "text", value: "result" },
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("return without value roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "return",
        pid: "p1",
      },
    ];
    const result = roundtrip(ops);
    // result should have no value field
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("call_fn roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "call_fn",
        pid: "p1",
        fnName: { kind: "text", value: "print" },
        args: [{ kind: "text", value: '"hello"' }],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("call_fn with no args roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "call_fn",
        pid: "p1",
        fnName: { kind: "text", value: "foo" },
        args: [],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("await_expr roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "await",
        pid: "p1",
        value: { kind: "text", value: "fetchData()" },
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("break roundtrip", () => {
    const ops: CodeOp[] = [
      { kind: "break", pid: "p1" },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("continue roundtrip", () => {
    const ops: CodeOp[] = [
      { kind: "continue", pid: "p1" },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("comment roundtrip", () => {
    const ops: CodeOp[] = [
      { kind: "comment", pid: "p1", text: "This is a comment" },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("raw roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "raw",
        pid: "p1",
        source: "someRawCode();",
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("while_loop with body roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "while",
        pid: "p1",
        condition: { kind: "text", value: "x < 10" },
        body: [
          {
            kind: "set_var",
            pid: "p2",
            name: { kind: "text", value: "x" },
            operator: "+=",
            value: { kind: "text", value: "1" },
          },
        ],
        bodyId: createBodyId(),
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("while_loop with empty body roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "while",
        pid: "p1",
        condition: { kind: "text", value: "true" },
        body: [],
        bodyId: createBodyId(),
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("for_in with body roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "for_in",
        pid: "p1",
        itemName: { kind: "text", value: "item" },
        collection: { kind: "text", value: "items" },
        body: [
          {
            kind: "call_fn",
            pid: "p2",
            fnName: { kind: "text", value: "print" },
            args: [{ kind: "text", value: "item" }],
          },
        ],
        bodyId: createBodyId(),
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("if_block (if only) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "if_block",
        pid: "p1",
        bodyId: createBodyId(),
        branches: [
          {
            condition: { kind: "text", value: "x > 0" },
            body: [
              {
                kind: "call_fn",
                pid: "p2",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"positive"' }],
              },
            ],
          },
        ],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("if_block (if + else) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "if_block",
        pid: "p1",
        bodyId: createBodyId(),
        branches: [
          {
            condition: { kind: "text", value: "x > 0" },
            body: [
              {
                kind: "call_fn",
                pid: "p2",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"positive"' }],
              },
            ],
          },
          {
            // else branch: condition is empty
            condition: { kind: "empty" },
            body: [
              {
                kind: "call_fn",
                pid: "p3",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"non-positive"' }],
              },
            ],
          },
        ],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("if_block (if + elif + else) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "if_block",
        pid: "p1",
        bodyId: createBodyId(),
        branches: [
          {
            condition: { kind: "text", value: "x > 0" },
            body: [
              {
                kind: "call_fn",
                pid: "p2",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"positive"' }],
              },
            ],
          },
          {
            condition: { kind: "text", value: "x == 0" },
            body: [
              {
                kind: "call_fn",
                pid: "p3",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"zero"' }],
              },
            ],
          },
          {
            condition: { kind: "empty" },
            body: [
              {
                kind: "call_fn",
                pid: "p4",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"negative"' }],
              },
            ],
          },
        ],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("if_block (if + multiple elifs + else) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "if_block",
        pid: "p1",
        bodyId: createBodyId(),
        branches: [
          {
            condition: { kind: "text", value: "grade >= 90" },
            body: [
              {
                kind: "call_fn",
                pid: "p2",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"A"' }],
              },
            ],
          },
          {
            condition: { kind: "text", value: "grade >= 80" },
            body: [],
          },
          {
            condition: { kind: "text", value: "grade >= 70" },
            body: [],
          },
          {
            condition: { kind: "empty" },
            body: [
              {
                kind: "call_fn",
                pid: "p3",
                fnName: { kind: "text", value: "print" },
                args: [{ kind: "text", value: '"F"' }],
              },
            ],
          },
        ],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("if_block (if + elif, no else) roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "if_block",
        pid: "p1",
        bodyId: createBodyId(),
        branches: [
          {
            condition: { kind: "text", value: "x > 0" },
            body: [],
          },
          {
            condition: { kind: "text", value: "x < 0" },
            body: [],
          },
        ],
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });

  it("multiple sequential ops roundtrip", () => {
    const ops: CodeOp[] = [
      {
        kind: "declare_var",
        pid: "p1",
        mutable: false,
        name: { kind: "text", value: "x" },
        initValue: { kind: "text", value: "10" },
        scope: "fn",
      },
      {
        kind: "while",
        pid: "p2",
        condition: { kind: "text", value: "x > 0" },
        body: [
          {
            kind: "call_fn",
            pid: "p3",
            fnName: { kind: "text", value: "print" },
            args: [{ kind: "text", value: "x" }],
          },
          {
            kind: "set_var",
            pid: "p4",
            name: { kind: "text", value: "x" },
            operator: "-=",
            value: { kind: "text", value: "1" },
          },
        ],
        bodyId: createBodyId(),
      },
      {
        kind: "raw",
        pid: "p5",
        source: "// done",
      },
    ];
    const result = roundtrip(ops);
    expect(stripMeta(result)).toEqual(stripMeta(ops));
  });
});
