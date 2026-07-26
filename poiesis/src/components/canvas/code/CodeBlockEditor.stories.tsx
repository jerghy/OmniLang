// CodeBlockEditor.stories.tsx
// Storybook stories for the CodeBlockEditor v5 component.
// Design: docs/superpowers/specs/2026-07-26-codeblock-text-editor-design.md §7

import "../../../styles/tokens.css";
import type { Meta, StoryObj } from "@storybook/react";
import { CodeBlockEditor } from "./CodeBlockEditor";
import type { CodeBlockEditorContext } from "./CodeBlockEditor";

const meta: Meta<typeof CodeBlockEditor> = {
  title: "CodeBlockEditor/CodeBlockEditor",
  component: CodeBlockEditor,
  parameters: {
    layout: "fullscreen",
  },
};

export default meta;
type Story = StoryObj<typeof CodeBlockEditor>;

// ── Shared mock handlers ──

const noopSave = (source: string) => {
  console.log("[Storybook] onSave:", source.slice(0, 60));
};
const noopClose = () => {
  console.log("[Storybook] onClose");
};

// ── Mock WASM analysis context ──
// In Storybook, WASM is not available, so the dependency panel
// will show "解析中..." (analyzing...). This is expected — the
// stories verify Monaco rendering and layout, not WASM analysis.

const mockContext: CodeBlockEditorContext = {
  fnParams: ["items", "filter", "count"],
  componentName: "TodoList",
  omniSourcePath: "src/components/TodoList.omni",
  signals: ["activeFilter", "completedCount"],
  stores: ["TodoStore"],
  allSources: {},
  designElements: [],
  designWires: [],
  codeTabs: [],
};

// ── Stories ──

export const Empty: Story = {
  name: "空编辑器",
  args: {
    elementId: "s-empty",
    initialSource: "",
    context: mockContext,
    onSave: noopSave,
    onClose: noopClose,
  },
};

export const SimpleVarOps: Story = {
  name: "变量操作 + 返回",
  args: {
    elementId: "s-ops",
    initialSource: "let count: i32 = 0;\ncount += 1;\nreturn count;",
    context: mockContext,
    onSave: noopSave,
    onClose: noopClose,
  },
};

export const IfElseBlock: Story = {
  name: "if/else 分支",
  args: {
    elementId: "s-if",
    initialSource: "if x > 0 {\n  return 1;\n} else {\n  return 0;\n}",
    context: mockContext,
    onSave: noopSave,
    onClose: noopClose,
  },
};

export const WhileLoop: Story = {
  name: "while 循环",
  args: {
    elementId: "s-while",
    initialSource: "let i = 0;\nwhile i < 10 {\n  i += 1;\n}",
    context: mockContext,
    onSave: noopSave,
    onClose: noopClose,
  },
};

export const StoreCall: Story = {
  name: "Store 调用 + 赋值",
  args: {
    elementId: "s-store",
    initialSource: "let items = TodoStore.getItems();\nfiltered = items.filter(fn);",
    context: mockContext,
    onSave: noopSave,
    onClose: noopClose,
  },
};
