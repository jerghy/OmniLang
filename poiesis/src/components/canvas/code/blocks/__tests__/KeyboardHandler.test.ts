// poiesis/src/components/canvas/code/blocks/__tests__/KeyboardHandler.test.ts
// Task 5: Pure DOM KeyboardHandler tests — no Blockly dependency.

import { describe, it, expect, vi } from "vitest";
import { KeyboardHandler } from "../KeyboardHandler";
import { BlockModel } from "../BlockModel";

describe("KeyboardHandler", () => {
  const makeContainer = () => {
    const div = document.createElement("div");
    document.body.appendChild(div);
    return div;
  };

  const dummyGetElement = () => null;

  it("ArrowLeft calls model.moveLeft", () => {
    const model = new BlockModel([{ kind: "break", pid: "b-1" }]);
    model.moveRight(); // move to block
    const container = makeContainer();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch: vi.fn(),
      onRawInput: vi.fn(),
      onOpsChanged: vi.fn(),
    });
    expect(model.getCurrentTarget()?.type).toBe("block");
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft" }));
    expect(model.getCurrentTarget()?.type).toBe("gap");
    kh.dispose();
    container.remove();
  });

  it("Delete on block removes it", () => {
    const model = new BlockModel([{ kind: "break", pid: "b-1" }]);
    model.moveRight(); // on block
    const container = makeContainer();
    const onOpsChanged = vi.fn();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch: vi.fn(),
      onRawInput: vi.fn(),
      onOpsChanged,
    });
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete" }));
    expect(model.getOps()).toHaveLength(0);
    expect(onOpsChanged).toHaveBeenCalled();
    kh.dispose();
    container.remove();
  });

  it("character key on gap triggers fuzzy search", () => {
    const model = new BlockModel([]);
    const container = makeContainer();
    const onFuzzySearch = vi.fn();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch,
      onRawInput: vi.fn(),
      onOpsChanged: vi.fn(),
    });
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "l" }));
    expect(onFuzzySearch).toHaveBeenCalledWith("l", "gap");
    kh.dispose();
    container.remove();
  });

  it("backslash triggers raw input", () => {
    const model = new BlockModel([]);
    const container = makeContainer();
    const onRawInput = vi.fn();
    const kh = new KeyboardHandler(container, model, dummyGetElement, {
      onFuzzySearch: vi.fn(),
      onRawInput,
      onOpsChanged: vi.fn(),
    });
    container.dispatchEvent(new KeyboardEvent("keydown", { key: "\\" }));
    expect(onRawInput).toHaveBeenCalled();
    kh.dispose();
    container.remove();
  });
});
