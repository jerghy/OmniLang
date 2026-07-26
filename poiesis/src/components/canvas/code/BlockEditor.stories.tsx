import type { Meta, StoryObj } from "@storybook/react";
import { BlockEditor } from "./BlockEditor";
import type { CodeOp } from "../../../core/types/code-op";
import { createBodyId } from "../../../core/types/code-op";

const meta: Meta<typeof BlockEditor> = {
  title: "BlockEditor/BlockEditor",
  component: BlockEditor,
  parameters: { layout: "fullscreen" },
};

export default meta;
type Story = StoryObj<typeof BlockEditor>;

const noop = { onSave: () => {}, onClose: () => {} };

// ── Stories ──

export const Empty: Story = {
  args: { elementId: "story-empty", initialOps: [], initialSource: "", ...noop },
  name: "Empty editor",
};

export const LetAndSet: Story = {
  args: {
    elementId: "story-let-set",
    initialOps: [
      { kind: "declare_var", pid: "dv-1", mutable: false, name: { kind: "text", value: "count" }, varType: { kind: "text", value: "i32" }, initValue: { kind: "text", value: "0" }, scope: "fn" },
      { kind: "set_var", pid: "sv-1", name: { kind: "text", value: "count" }, operator: "+=", value: { kind: "text", value: "1" } },
    ] as CodeOp[],
    initialSource: "let count: i32 = 0;\ncount += 1;",
    ...noop,
  },
  name: "let + set",
};

export const IfBlock: Story = {
  args: {
    elementId: "story-if",
    initialOps: [{
      kind: "if_block", pid: "if-1", bodyId: createBodyId(),
      branches: [
        { condition: { kind: "text", value: "x > 0" }, body: [{ kind: "return", pid: "ret-1", value: { kind: "text", value: "1" } }] },
        { condition: { kind: "empty" }, body: [{ kind: "return", pid: "ret-2", value: { kind: "text", value: "0" } }] },
      ],
    }] as CodeOp[],
    initialSource: "if x > 0 { return 1; } else { return 0; }",
    ...noop,
  },
  name: "if / else block",
};

export const WhileLoop: Story = {
  args: {
    elementId: "story-while",
    initialOps: [{
      kind: "while", pid: "wh-1", bodyId: createBodyId(),
      condition: { kind: "text", value: "i < 10" },
      body: [{ kind: "set_var", pid: "sv-1", name: { kind: "text", value: "i" }, operator: "+=", value: { kind: "text", value: "1" } }],
    }] as CodeOp[],
    initialSource: "while i < 10 { i += 1; }",
    ...noop,
  },
  name: "while loop",
};

export const ForLoop: Story = {
  args: {
    elementId: "story-for",
    initialOps: [{
      kind: "for_in", pid: "for-1", bodyId: createBodyId(),
      itemName: { kind: "text", value: "item" },
      collection: { kind: "text", value: "items" },
      body: [{ kind: "call_fn", pid: "call-1", fnName: { kind: "text", value: "print" }, args: [{ kind: "text", value: "item" }] }],
    }] as CodeOp[],
    initialSource: "for item in items { print(item); }",
    ...noop,
  },
  name: "for..in loop",
};

export const BreakContinue: Story = {
  args: {
    elementId: "story-break-continue",
    initialOps: [{
      kind: "while", pid: "wh-1", bodyId: createBodyId(),
      condition: { kind: "text", value: "true" },
      body: [{
        kind: "if_block", pid: "if-1", bodyId: createBodyId(),
        branches: [
          {
            condition: { kind: "text", value: 'x == "stop"' },
            body: [{ kind: "break", pid: "br-1" }],
          },
          {
            condition: { kind: "text", value: 'x == "skip"' },
            body: [{ kind: "continue", pid: "ct-1" }],
          },
        ],
      }],
    }] as CodeOp[],
    initialSource: 'while true {\n  if x == "stop" {\n    break;\n  } else if x == "skip" {\n    continue;\n  }\n}',
    ...noop,
  },
  name: "break / continue",
};

export const AwaitCall: Story = {
  args: {
    elementId: "story-await",
    initialOps: [
      { kind: "await", pid: "aw-1", value: { kind: "text", value: "fetchUser(id)" } },
      { kind: "call_fn", pid: "call-1", fnName: { kind: "text", value: "process" }, args: [{ kind: "text", value: "user" }] },
    ] as CodeOp[],
    initialSource: "await fetchUser(id);\nprocess(user);",
    ...noop,
  },
  name: "await + call",
};

export const CommentRaw: Story = {
  args: {
    elementId: "story-comment-raw",
    initialOps: [
      { kind: "comment", pid: "cm-1", text: "this is a comment" },
      { kind: "raw", pid: "rw-1", source: "// legacy code: do not touch" },
    ] as CodeOp[],
    initialSource: "// this is a comment\n// legacy code: do not touch",
    ...noop,
  },
  name: "comment + raw fallback",
};

export const FullExample: Story = {
  args: {
    elementId: "story-full",
    initialOps: [
      // let + set
      { kind: "declare_var", pid: "dv-1", mutable: false, name: { kind: "text", value: "count" }, varType: { kind: "text", value: "i32" }, initValue: { kind: "text", value: "0" }, scope: "fn" },
      { kind: "set_var", pid: "sv-1", name: { kind: "text", value: "count" }, operator: "+=", value: { kind: "text", value: "1" } },
      // if / elif / else
      { kind: "if_block", pid: "if-1", bodyId: createBodyId(),
        branches: [
          { condition: { kind: "text", value: "x > 0" }, body: [
            { kind: "call_fn", pid: "call-1", fnName: { kind: "text", value: "print" }, args: [{ kind: "text", value: '"positive"' }] },
            { kind: "return", pid: "ret-1", value: { kind: "text", value: "1" } },
          ]},
          { condition: { kind: "text", value: "x < 0" }, body: [
            { kind: "return", pid: "ret-2", value: { kind: "text", value: "-1" } },
          ]},
          { condition: { kind: "empty" }, body: [
            { kind: "return", pid: "ret-3", value: { kind: "text", value: "0" } },
          ]},
        ],
      },
      // while loop (with nested operations)
      { kind: "declare_var", pid: "dv-2", mutable: true, name: { kind: "text", value: "sum" }, varType: { kind: "text", value: "f64" }, initValue: { kind: "text", value: "0.0" }, scope: "fn" },
      { kind: "declare_var", pid: "dv-3", mutable: true, name: { kind: "text", value: "i" }, varType: { kind: "text", value: "i32" }, initValue: { kind: "text", value: "0" }, scope: "fn" },
      { kind: "while", pid: "wh-1", bodyId: createBodyId(),
        condition: { kind: "text", value: "i < values.len()" },
        body: [
          { kind: "set_var", pid: "sv-2", name: { kind: "text", value: "sum" }, operator: "+=", value: { kind: "text", value: "values.get(i)" } },
          { kind: "set_var", pid: "sv-3", name: { kind: "text", value: "i" }, operator: "+=", value: { kind: "text", value: "1" } },
        ],
      },
      { kind: "return", pid: "ret-4", value: { kind: "text", value: "sum" } },
    ] as CodeOp[],
    initialSource: `let count: i32 = 0;\ncount += 1;\n\nif x > 0 {\n  print("positive");\n  return 1;\n} else if x < 0 {\n  return -1;\n} else {\n  return 0;\n}\n\nlet mut sum: f64 = 0.0;\nlet mut i: i32 = 0;\nwhile i < values.len() {\n  sum += values.get(i);\n  i += 1;\n}\nreturn sum;`,
    ...noop,
  },
  name: "Full example",
};

export const TypescriptMode: Story = {
  args: {
    elementId: "story-ts",
    initialOps: [
      { kind: "declare_var", pid: "dv-1", mutable: false, name: { kind: "text", value: "count" }, varType: { kind: "text", value: "number" }, initValue: { kind: "text", value: "0" }, scope: "fn" },
      { kind: "set_var", pid: "sv-1", name: { kind: "text", value: "count" }, operator: "+=", value: { kind: "text", value: "1" } },
    ] as CodeOp[],
    initialSource: "let count: number = 0;\ncount += 1;",
    language: "typescript",
    ...noop,
  },
  name: "TypeScript mode",
};
