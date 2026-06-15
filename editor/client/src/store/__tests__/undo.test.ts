/** Undo store 单元测试。*/

import { describe, it, expect, beforeEach, vi } from "vitest";
import { useUndoStore } from "../undo";
import type { Command, CommandContext } from "../../commands/types";

function makeMockCtx(): CommandContext {
  const elements: unknown[] = [];
  const wires: unknown[] = [];
  return {
    elements: elements as CommandContext["elements"],
    wires: wires as CommandContext["wires"],
    getElement: vi.fn(),
    addElement: vi.fn(),
    removeElement: vi.fn(),
    setProp: vi.fn(),
    getProp: vi.fn(),
    addWire: vi.fn(),
    removeWire: vi.fn(),
    setElementsAndWires: vi.fn(),
  };
}

function makeMockCommand(overrides: Partial<Command> = {}): Command {
  return {
    type: "TestCommand",
    description: "测试命令",
    execute: vi.fn(),
    undo: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  useUndoStore.setState({ undoStack: [], redoStack: [] });
});

describe("execute", () => {
  it("calls cmd.execute(ctx) and pushes to undoStack", () => {
    const ctx = makeMockCtx();
    const cmd = makeMockCommand();
    useUndoStore.getState().execute(cmd, ctx);

    expect(cmd.execute).toHaveBeenCalledWith(ctx);
    expect(useUndoStore.getState().undoStack).toHaveLength(1);
    expect(useUndoStore.getState().undoStack[0]).toBe(cmd);
  });

  it("clears redoStack on new execute", () => {
    const ctx = makeMockCtx();
    const cmd1 = makeMockCommand({ type: "A" });
    const cmd2 = makeMockCommand({ type: "B" });

    useUndoStore.getState().execute(cmd1, ctx);
    useUndoStore.getState().undo(ctx); // cmd1 moves to redo
    expect(useUndoStore.getState().redoStack).toHaveLength(1);

    useUndoStore.getState().execute(cmd2, ctx); // new action clears redo
    expect(useUndoStore.getState().redoStack).toHaveLength(0);
    expect(useUndoStore.getState().undoStack).toHaveLength(1);
  });

  it("caps undoStack at 200 entries", () => {
    const ctx = makeMockCtx();
    // Push 250 commands
    for (let i = 0; i < 250; i++) {
      const cmd = makeMockCommand({ type: `Cmd${i}` });
      useUndoStore.getState().execute(cmd, ctx);
    }
    const stack = useUndoStore.getState().undoStack;
    // Stack is capped at 200 (slice(-199) + 1 new = 200)
    expect(stack.length).toBeLessThanOrEqual(200);
    // Most recent commands should be at the end
    expect(stack[stack.length - 1].type).toBe("Cmd249");
  });
});

describe("undo", () => {
  it("calls cmd.undo(ctx) and moves to redoStack", () => {
    const ctx = makeMockCtx();
    const cmd = makeMockCommand();

    useUndoStore.getState().execute(cmd, ctx);
    useUndoStore.getState().undo(ctx);

    expect(cmd.undo).toHaveBeenCalledWith(ctx);
    expect(useUndoStore.getState().undoStack).toHaveLength(0);
    expect(useUndoStore.getState().redoStack).toHaveLength(1);
    expect(useUndoStore.getState().redoStack[0]).toBe(cmd);
  });

  it("is a no-op on empty undoStack", () => {
    const ctx = makeMockCtx();
    useUndoStore.getState().undo(ctx);
    expect(useUndoStore.getState().undoStack).toHaveLength(0);
    expect(useUndoStore.getState().redoStack).toHaveLength(0);
  });
});

describe("redo", () => {
  it("calls cmd.execute(ctx) and moves back to undoStack", () => {
    const ctx = makeMockCtx();
    const cmd = makeMockCommand();

    useUndoStore.getState().execute(cmd, ctx);
    useUndoStore.getState().undo(ctx);
    // Reset mock to check redo calls execute (not undo)
    const executeSpy = vi.fn();
    const redoCmd = { ...cmd, execute: executeSpy };

    // Manually put redoCmd on redoStack for isolated testing
    useUndoStore.setState({ undoStack: [], redoStack: [redoCmd] });
    useUndoStore.getState().redo(ctx);

    expect(executeSpy).toHaveBeenCalledWith(ctx);
    expect(useUndoStore.getState().undoStack).toHaveLength(1);
    expect(useUndoStore.getState().redoStack).toHaveLength(0);
  });

  it("is a no-op on empty redoStack", () => {
    const ctx = makeMockCtx();
    useUndoStore.getState().redo(ctx);
    expect(useUndoStore.getState().redoStack).toHaveLength(0);
  });
});

describe("clear", () => {
  it("clears both stacks", () => {
    const ctx = makeMockCtx();
    const cmd = makeMockCommand();
    useUndoStore.getState().execute(cmd, ctx);
    useUndoStore.getState().clear();
    expect(useUndoStore.getState().undoStack).toHaveLength(0);
    expect(useUndoStore.getState().redoStack).toHaveLength(0);
  });
});
