// poiesis/src/components/canvas/code/blocks/__tests__/KeyboardHandler.test.ts
// Task 7: KeyboardHandler — keyboard input interception and dispatch tests.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as Blockly from "blockly";
import { BlockAdapter } from "../BlockAdapter";
import { SelectionManager } from "../SelectionManager";
import { GapManager } from "../GapManager";
import { KeyboardHandler } from "../KeyboardHandler";
import type { CodeOp } from "../../../../../core/types/code-op";

// ── Test helpers ──

function declareVar(name: string): CodeOp {
  return {
    kind: "declare_var",
    pid: `pid-dv-${name}`,
    mutable: false,
    name: { kind: "text", value: name },
    scope: "fn",
  };
}

function setVar(name: string, value: string): CodeOp {
  return {
    kind: "set_var",
    pid: `pid-set-${name}`,
    name: { kind: "text", value: name },
    operator: "=",
    value: { kind: "text", value },
  };
}

interface TestContext {
  ws: Blockly.WorkspaceSvg;
  sm: SelectionManager;
  gm: GapManager;
  kh: KeyboardHandler;
  callbacks: {
    onFuzzySearch: ReturnType<typeof vi.fn>;
    onRawInput: ReturnType<typeof vi.fn>;
    onOpsChanged: ReturnType<typeof vi.fn>;
  };
  div: HTMLDivElement;
}

function setup(ops: CodeOp[] = []): TestContext {
  const div = document.createElement("div");
  div.style.width = "800px";
  div.style.height = "600px";
  div.style.position = "absolute";
  div.style.left = "0";
  div.style.top = "0";
  document.body.appendChild(div);

  const ws = Blockly.inject(div, {
    renderer: "zelos",
  }) as Blockly.WorkspaceSvg;

  if (ops.length > 0) {
    const adapter = new BlockAdapter();
    adapter.toBlocks(ops, ws);
  }

  const sm = new SelectionManager(ws);
  const gm = new GapManager(ws);
  gm.initializeForBlockCount(ops.length);

  const callbacks = {
    onFuzzySearch: vi.fn(),
    onRawInput: vi.fn(),
    onOpsChanged: vi.fn(),
  };

  const kh = new KeyboardHandler(ws, sm, gm, callbacks);

  return { ws, sm, gm, kh, callbacks, div };
}

function cleanup(ctx: TestContext): void {
  ctx.kh.dispose();
  ctx.sm.dispose();
  ctx.ws.dispose();
  if (ctx.div.parentElement) {
    ctx.div.parentElement.removeChild(ctx.div);
  }
}

function getSvg(ws: Blockly.WorkspaceSvg): SVGElement | null {
  return ws.getParentSvg();
}

function dispatchKey(
  svg: SVGElement | null,
  key: string,
  options: Partial<KeyboardEventInit> = {},
): void {
  if (!svg) return;
  svg.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...options }));
}

// ── Tests ──

describe("KeyboardHandler", () => {
  describe("navigation dispatch", () => {
    it("dispatches ArrowRight to selectionMgr.moveRight", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      const spy = vi.spyOn(ctx.sm, "moveRight");
      dispatchKey(getSvg(ctx.ws), "ArrowRight");
      expect(spy).toHaveBeenCalledOnce();
      cleanup(ctx);
    });

    it("dispatches ArrowLeft to selectionMgr.moveLeft", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      const spy = vi.spyOn(ctx.sm, "moveLeft");
      dispatchKey(getSvg(ctx.ws), "ArrowLeft");
      expect(spy).toHaveBeenCalledOnce();
      cleanup(ctx);
    });

    it("dispatches ArrowUp to selectionMgr.moveUp", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      const spy = vi.spyOn(ctx.sm, "moveUp");
      dispatchKey(getSvg(ctx.ws), "ArrowUp");
      expect(spy).toHaveBeenCalledOnce();
      cleanup(ctx);
    });

    it("dispatches ArrowDown to selectionMgr.moveDown", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      const spy = vi.spyOn(ctx.sm, "moveDown");
      dispatchKey(getSvg(ctx.ws), "ArrowDown");
      expect(spy).toHaveBeenCalledOnce();
      cleanup(ctx);
    });
  });

  describe("character keys", () => {
    it("typing on block does not trigger fuzzy search", () => {
      const ctx = setup([declareVar("x")]);
      // Cursor starts on block
      dispatchKey(getSvg(ctx.ws), "a");
      expect(ctx.callbacks.onFuzzySearch).not.toHaveBeenCalled();
      cleanup(ctx);
    });

    it("typing on gap triggers fuzzy search with character and gap context", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      // Move cursor to gap
      ctx.sm.moveRight();
      dispatchKey(getSvg(ctx.ws), "a");
      expect(ctx.callbacks.onFuzzySearch).toHaveBeenCalledWith("a", "gap");
      cleanup(ctx);
    });

    it("typing on gap triggers fuzzy search for each character typed", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      ctx.sm.moveRight();
      dispatchKey(getSvg(ctx.ws), "s");
      dispatchKey(getSvg(ctx.ws), "e");
      dispatchKey(getSvg(ctx.ws), "t");
      expect(ctx.callbacks.onFuzzySearch).toHaveBeenCalledTimes(3);
      expect(ctx.callbacks.onFuzzySearch).toHaveBeenNthCalledWith(1, "s", "gap");
      expect(ctx.callbacks.onFuzzySearch).toHaveBeenNthCalledWith(2, "e", "gap");
      expect(ctx.callbacks.onFuzzySearch).toHaveBeenNthCalledWith(3, "t", "gap");
      cleanup(ctx);
    });
  });

  describe("special key dispatch", () => {
    it("backslash triggers onRawInput", () => {
      const ctx = setup([declareVar("x")]);
      dispatchKey(getSvg(ctx.ws), "\\");
      expect(ctx.callbacks.onRawInput).toHaveBeenCalledOnce();
      cleanup(ctx);
    });

    it("Enter on block is a no-op (no callbacks fired)", () => {
      const ctx = setup([declareVar("x")]);
      dispatchKey(getSvg(ctx.ws), "Enter");
      expect(ctx.callbacks.onFuzzySearch).not.toHaveBeenCalled();
      expect(ctx.callbacks.onRawInput).not.toHaveBeenCalled();
      expect(ctx.callbacks.onOpsChanged).not.toHaveBeenCalled();
      cleanup(ctx);
    });

    it("Enter on gap inserts gap and opens fuzzy search", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      ctx.sm.moveRight(); // move to gap
      const insertSpy = vi.spyOn(ctx.gm, "insertGap");
      dispatchKey(getSvg(ctx.ws), "Enter");
      expect(insertSpy).toHaveBeenCalled();
      expect(ctx.callbacks.onFuzzySearch).toHaveBeenCalledWith("", "gap");
      cleanup(ctx);
    });

    it("Delete on block disposes block and fires onOpsChanged", () => {
      const ctx = setup([declareVar("x")]);
      dispatchKey(getSvg(ctx.ws), "Delete");
      expect(ctx.callbacks.onOpsChanged).toHaveBeenCalledOnce();
      // Block should be removed
      expect(ctx.ws.getTopBlocks(true).length).toBe(0);
      cleanup(ctx);
    });

    it("Backspace on block disposes block and fires onOpsChanged", () => {
      const ctx = setup([declareVar("x")]);
      dispatchKey(getSvg(ctx.ws), "Backspace");
      expect(ctx.callbacks.onOpsChanged).toHaveBeenCalledOnce();
      expect(ctx.ws.getTopBlocks(true).length).toBe(0);
      cleanup(ctx);
    });

    it("Delete on gap calls deleteGap", () => {
      const ctx = setup([declareVar("x"), setVar("x", "5")]);
      ctx.sm.moveRight(); // move to gap
      const deleteSpy = vi.spyOn(ctx.gm, "deleteGap");
      dispatchKey(getSvg(ctx.ws), "Delete");
      expect(deleteSpy).toHaveBeenCalled();
      cleanup(ctx);
    });
  });

  describe("clipboard operations", () => {
    it("Ctrl+C copies via Blockly.clipboard.copy", () => {
      const ctx = setup([declareVar("x")]);
      const copySpy = vi.spyOn(Blockly.clipboard, "copy").mockImplementation(() => {});
      dispatchKey(getSvg(ctx.ws), "c", { ctrlKey: true });
      expect(copySpy).toHaveBeenCalledWith(ctx.ws);
      copySpy.mockRestore();
      cleanup(ctx);
    });

    it("Meta+C copies via Blockly.clipboard.copy (Mac)", () => {
      const ctx = setup([declareVar("x")]);
      const copySpy = vi.spyOn(Blockly.clipboard, "copy").mockImplementation(() => {});
      dispatchKey(getSvg(ctx.ws), "c", { metaKey: true });
      expect(copySpy).toHaveBeenCalledWith(ctx.ws);
      copySpy.mockRestore();
      cleanup(ctx);
    });

    it("Ctrl+V pastes and fires onOpsChanged", () => {
      const ctx = setup([declareVar("x")]);
      const pasteSpy = vi.spyOn(Blockly.clipboard, "paste").mockImplementation(() => {});
      dispatchKey(getSvg(ctx.ws), "v", { ctrlKey: true });
      expect(pasteSpy).toHaveBeenCalledWith(ctx.ws);
      expect(ctx.callbacks.onOpsChanged).toHaveBeenCalledOnce();
      pasteSpy.mockRestore();
      cleanup(ctx);
    });
  });

  describe("empty workspace", () => {
    it("key events are no-ops when workspace is empty", () => {
      const ctx = setup([]);
      // No target — handler should return early without errors
      dispatchKey(getSvg(ctx.ws), "ArrowRight");
      dispatchKey(getSvg(ctx.ws), "a");
      dispatchKey(getSvg(ctx.ws), "Enter");
      dispatchKey(getSvg(ctx.ws), "Delete");
      expect(ctx.callbacks.onFuzzySearch).not.toHaveBeenCalled();
      expect(ctx.callbacks.onRawInput).not.toHaveBeenCalled();
      expect(ctx.callbacks.onOpsChanged).not.toHaveBeenCalled();
      cleanup(ctx);
    });
  });

  describe("lifecycle", () => {
    it("dispose removes event listener (no errors on double dispose)", () => {
      const ctx = setup([declareVar("x")]);
      ctx.kh.dispose();
      ctx.kh.dispose(); // should not throw
      cleanup(ctx);
    });
  });
});
