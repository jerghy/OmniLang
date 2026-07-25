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
