import type { Meta, StoryObj } from "@storybook/react";
import { CodeCardEditor } from "./CodeCardEditor";
import type { CodeOp } from "../../../core/types/code-op";

const meta: Meta<typeof CodeCardEditor> = {
  title: "CodeCard/CodeCardEditor",
  component: CodeCardEditor,
  parameters: {
    layout: "centered",
  },
  argTypes: {
    onSave: { action: "saved" },
    onClose: { action: "closed" },
  },
};

export default meta;
type Story = StoryObj<typeof CodeCardEditor>;

// ── Helpers ──

function pid(tag: string) {
  return `pid-${tag}-${Math.random().toString(36).slice(2, 8)}`;
}

// ── Mock data ──

const emptyOps: CodeOp[] = [];

const letAndSet: CodeOp[] = [
  {
    kind: "declare_var" as const,
    pid: pid("dv"),
    name: "count",
    mutable: false,
    varType: "i32",
    initValue: "0",
    scope: "fn",
  },
  {
    kind: "set_var" as const,
    pid: pid("sv"),
    name: "count",
    operator: "+=",
    value: "1",
  },
];

const ifBlockWithBranches: CodeOp[] = [
  {
    kind: "if_block" as const,
    pid: pid("if"),
    branches: [
      {
        condition: "x > 0",
        body: [
          {
            kind: "call_fn" as const,
            pid: pid("call"),
            fnName: "print",
            args: ['"positive"'],
          },
          {
            kind: "return" as const,
            pid: pid("ret"),
            value: "1",
          },
        ],
      },
      {
        condition: "x < 0",
        body: [
          {
            kind: "return" as const,
            pid: pid("ret2"),
            value: "-1",
          },
        ],
      },
      {
        condition: "",
        body: [
          {
            kind: "return" as const,
            pid: pid("ret3"),
            value: "0",
          },
        ],
      },
    ],
  },
];

const whileLoop: CodeOp[] = [
  {
    kind: "declare_var" as const,
    pid: pid("dv"),
    name: "sum",
    mutable: true,
    varType: "f64",
    initValue: "0.0",
    scope: "fn",
  },
  {
    kind: "declare_var" as const,
    pid: pid("dv2"),
    name: "i",
    mutable: true,
    varType: "i32",
    initValue: "0",
    scope: "fn",
  },
  {
    kind: "while" as const,
    pid: pid("wh"),
    condition: "i < values.len()",
    body: [
      {
        kind: "set_var" as const,
        pid: pid("sv"),
        name: "sum",
        operator: "+=",
        value: "values.get(i)",
      },
      {
        kind: "set_var" as const,
        pid: pid("sv2"),
        name: "i",
        operator: "+=",
        value: "1",
      },
    ],
  },
  {
    kind: "return" as const,
    pid: pid("ret"),
    value: "sum",
  },
];

const forLoop: CodeOp[] = [
  {
    kind: "for_in" as const,
    pid: pid("for"),
    itemName: "item",
    collection: "items",
    body: [
      {
        kind: "call_fn" as const,
        pid: pid("call"),
        fnName: "print",
        args: ["item"],
      },
    ],
  },
];

const breakAndContinue: CodeOp[] = [
  {
    kind: "while" as const,
    pid: pid("wh"),
    condition: "true",
    body: [
      {
        kind: "if_block" as const,
        pid: pid("if"),
        branches: [
          {
            condition: 'x == "stop"',
            body: [
              {
                kind: "break" as const,
                pid: pid("br"),
              },
            ],
          },
          {
            condition: 'x == "skip"',
            body: [
              {
                kind: "continue" as const,
                pid: pid("ct"),
              },
            ],
          },
        ],
      },
    ],
  },
];

const awaitOps: CodeOp[] = [
  {
    kind: "await" as const,
    pid: pid("aw"),
    value: "fetchUser(id)",
  },
  {
    kind: "call_fn" as const,
    pid: pid("call"),
    fnName: "process",
    args: ["user"],
  },
];

const commentAndRaw: CodeOp[] = [
  {
    kind: "comment" as const,
    pid: pid("cm"),
    text: "this is a comment",
  },
  {
    kind: "raw" as const,
    pid: pid("rw"),
    source: "// legacy code: do not touch",
  },
];

const fullExample: CodeOp[] = [
  ...letAndSet,
  ...ifBlockWithBranches,
  ...whileLoop,
];

// ── Stories ──

export const Empty: Story = {
  args: {
    elementId: "story-empty",
    initialOps: emptyOps,
    initialSource: "",
    language: "omnilang",
  },
  name: "Empty editor",
};

export const LetAndSet: Story = {
  args: {
    elementId: "story-let-set",
    initialOps: letAndSet,
    initialSource: `let count: i32 = 0;\ncount += 1;`,
    language: "omnilang",
  },
  name: "let + set operations",
};

export const IfBlock: Story = {
  args: {
    elementId: "story-if",
    initialOps: ifBlockWithBranches,
    initialSource: `if x > 0 {\n  print("positive");\n  return 1;\n} else if x < 0 {\n  return -1;\n} else {\n  return 0;\n}`,
    language: "omnilang",
  },
  name: "if / elif / else block",
};

export const WhileLoop: Story = {
  args: {
    elementId: "story-while",
    initialOps: whileLoop,
    initialSource: `let mut sum: f64 = 0.0;\nlet mut i: i32 = 0;\nwhile i < values.len() {\n  sum += values.get(i);\n  i += 1;\n}\nreturn sum;`,
    language: "omnilang",
  },
  name: "while loop",
};

export const ForLoop: Story = {
  args: {
    elementId: "story-for",
    initialOps: forLoop,
    initialSource: `for item in items {\n  print(item);\n}`,
    language: "omnilang",
  },
  name: "for..in loop",
};

export const BreakContinue: Story = {
  args: {
    elementId: "story-break-continue",
    initialOps: breakAndContinue,
    initialSource: `while true {\n  if x == "stop" {\n    break;\n  } else if x == "skip" {\n    continue;\n  }\n}`,
    language: "omnilang",
  },
  name: "break / continue",
};

export const AwaitCall: Story = {
  args: {
    elementId: "story-await",
    initialOps: awaitOps,
    initialSource: `await fetchUser(id);\nprocess(user);`,
    language: "omnilang",
  },
  name: "await + call",
};

export const CommentRaw: Story = {
  args: {
    elementId: "story-comment-raw",
    initialOps: commentAndRaw,
    initialSource: `// this is a comment\n// legacy code: do not touch`,
    language: "omnilang",
  },
  name: "comment + raw fallback",
};

export const FullExample: Story = {
  args: {
    elementId: "story-full",
    initialOps: fullExample,
    initialSource: `let count: i32 = 0;\ncount += 1;\n\nif x > 0 {\n  print("positive");\n  return 1;\n} else if x < 0 {\n  return -1;\n} else {\n  return 0;\n}\n\nlet mut sum: f64 = 0.0;\nlet mut i: i32 = 0;\nwhile i < values.len() {\n  sum += values.get(i);\n  i += 1;\n}\nreturn sum;`,
    language: "omnilang",
  },
  name: "Full example",
};

export const TypescriptMode: Story = {
  args: {
    elementId: "story-ts",
    initialOps: letAndSet,
    initialSource: `let count: number = 0;\ncount += 1;`,
    language: "typescript",
  },
  name: "TypeScript mode",
};
