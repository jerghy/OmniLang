// poiesis/src/components/canvas/code/blocks/__tests__/SelectionManager.test.ts
// Tests for SelectionManager: grid model, spatial navigation, position querying.

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as Blockly from "blockly";
import { SelectionManager, type SelectionTarget } from "../SelectionManager";
import { BlockAdapter } from "../BlockAdapter";
import type { CodeOp } from "../../../../../core/types/code-op";
import { createBodyId } from "../../../../../core/types/code-op";

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

function returnStmt(value?: string): CodeOp {
  return {
    kind: "return",
    pid: "pid-ret",
    ...(value ? { value: { kind: "text", value } } : {}),
  };
}

/**
 * Create a Blockly SVG workspace and populate it with the given ops.
 * Returns the workspace and a SelectionManager bound to it.
 */
function setupWorkspace(
  ops: CodeOp[],
): { ws: Blockly.WorkspaceSvg; sm: SelectionManager } {
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

  const adapter = new BlockAdapter();
  adapter.toBlocks(ops, ws);

  const sm = new SelectionManager(ws);
  return { ws, sm };
}

// ── Tests ──

describe("SelectionManager — grid model", () => {
  it("initial target is first block with single block", () => {
    const { sm, ws } = setupWorkspace([declareVar("x")]);
    const target = sm.getCurrentTarget();
    expect(target).not.toBeNull();
    expect(target!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("initial target is first block with multiple blocks", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    const target = sm.getCurrentTarget();
    expect(target).not.toBeNull();
    expect(target!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("returns null for empty workspace", () => {
    const { sm, ws } = setupWorkspace([]);
    expect(sm.getCurrentTarget()).toBeNull();
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("grid has correct structure (block, slots, gap, block, slots, gap, block, slots) for 3 blocks", () => {
    const { sm, ws } = setupWorkspace([
      declareVar("x"),
      setVar("x", "5"),
      returnStmt("x"),
    ]);
    // declare_var has 3 slots (NAME, VARTYPE, VALUE), set_var has 2, return_stmt has 1
    // Grid: block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv), slot(NAME-sv), slot(VALUE-sv), gap, block(ret), slot(VALUE-ret)
    expect(sm.getCurrentTarget()!.type).toBe("block");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("gap");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("gap");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    sm.moveRight();
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});

describe("SelectionManager — moveLeft / moveRight", () => {
  it("moveRight on single block goes to first slot", () => {
    const { sm, ws } = setupWorkspace([declareVar("x")]);
    // declare_var has 3 slots: NAME, VARTYPE, VALUE
    sm.moveRight();
    const after = sm.getCurrentTarget();
    expect(after!.type).toBe("slot");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveLeft on first block stays", () => {
    const { sm, ws } = setupWorkspace([declareVar("x")]);
    sm.moveLeft();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveRight on first block goes to first slot (2 blocks)", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    // Grid: block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv), ...
    sm.moveRight();
    const target = sm.getCurrentTarget();
    expect(target!.type).toBe("slot");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveRight through slots and gap reaches second block", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    // Grid: block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv)
    sm.moveRight(); // slot(NAME)
    sm.moveRight(); // slot(VARTYPE)
    sm.moveRight(); // slot(VALUE)
    sm.moveRight(); // gap
    sm.moveRight(); // block(sv)
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveLeft from second block goes to gap", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    // Grid: block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv)
    sm.moveRight(); // slot(NAME)
    sm.moveRight(); // slot(VARTYPE)
    sm.moveRight(); // slot(VALUE)
    sm.moveRight(); // gap
    sm.moveRight(); // block(sv)
    sm.moveLeft(); // block(sv) -> gap
    expect(sm.getCurrentTarget()!.type).toBe("gap");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveLeft from gap goes to last slot of preceding block", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    // Grid: block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv)
    sm.moveRight(); // slot(NAME)
    sm.moveRight(); // slot(VARTYPE)
    sm.moveRight(); // slot(VALUE)
    sm.moveRight(); // gap
    sm.moveLeft(); // gap -> slot(VALUE)
    expect(sm.getCurrentTarget()!.type).toBe("slot");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});

describe("SelectionManager — moveUp / moveDown", () => {
  it("moveUp on first block stays", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    sm.moveUp();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveDown skips gap to next block", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    sm.moveDown();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveDown on last block stays", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    // Grid: block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv)
    sm.moveRight(); // slot(NAME)
    sm.moveRight(); // slot(VARTYPE)
    sm.moveRight(); // slot(VALUE)
    sm.moveRight(); // gap
    sm.moveRight(); // block(sv — last)
    sm.moveDown();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveUp skips slots and gap to previous block", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    sm.moveRight(); // slot(NAME)
    sm.moveUp(); // should skip slots+gap and go to first block
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveDown skips multiple gaps with 3 blocks", () => {
    const { sm, ws } = setupWorkspace([
      declareVar("a"),
      setVar("a", "1"),
      setVar("b", "2"),
    ]);
    // Start at block 0
    // moveDown skips gap 0.5 and goes to block 1
    sm.moveDown();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    // moveDown again skips gap 1.5 and goes to block 2
    sm.moveDown();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moveUp skips multiple gaps with 3 blocks", () => {
    const { sm, ws } = setupWorkspace([
      declareVar("a"),
      setVar("a", "1"),
      setVar("b", "2"),
    ]);
    // Grid: block(da), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sa), slot(NAME), slot(VALUE), gap, block(sb), slot(VALUE)
    // Navigate to last block (index 9)
    for (let i = 0; i < 9; i++) sm.moveRight();
    // moveUp skips gap+slots and goes to block 1
    sm.moveUp();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    // moveUp again skips gap+slots and goes to block 0
    sm.moveUp();
    expect(sm.getCurrentTarget()!.type).toBe("block");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});

describe("SelectionManager — moveTo", () => {
  it("moves to a specific block by id", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    const blocks = ws.getTopBlocks(true);
    const secondBlock = blocks[1];
    sm.moveTo({ type: "block", blockId: secondBlock.id });
    const target = sm.getCurrentTarget()!;
    expect(target.type).toBe("block");
    expect((target as SelectionTarget & { type: "block" }).blockId).toBe(
      secondBlock.id,
    );
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("moves to a specific gap by position", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    sm.moveTo({ type: "gap", parentBlockId: null, position: 0.5 });
    const target = sm.getCurrentTarget()!;
    expect(target.type).toBe("gap");
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});

describe("SelectionManager — onChange callback", () => {
  it("fires onChange after moveRight", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    let changed = false;
    sm.onChange = (_target: SelectionTarget | null) => {
      changed = true;
    };
    sm.moveRight();
    expect(changed).toBe(true);
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("fires onChange after moveLeft", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    let changed = false;
    sm.onChange = (_target: SelectionTarget | null) => {
      changed = true;
    };
    sm.moveRight(); // to gap
    sm.moveLeft(); // back to block
    expect(changed).toBe(true);
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("fires onChange after moveUp", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    let changed = false;
    sm.onChange = (_target: SelectionTarget | null) => {
      changed = true;
    };
    sm.moveUp();
    expect(changed).toBe(true);
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("fires onChange after moveDown", () => {
    const { sm, ws } = setupWorkspace([declareVar("x"), setVar("x", "5")]);
    let changed = false;
    sm.onChange = (_target: SelectionTarget | null) => {
      changed = true;
    };
    sm.moveDown();
    expect(changed).toBe(true);
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});

describe("SelectionManager — refresh", () => {
  it("updates grid when blocks change", () => {
    const { sm, ws } = setupWorkspace([declareVar("x")]);
    expect(sm.getCurrentTarget()).not.toBeNull();

    // Add a new block directly on the workspace
    const newBlock = ws.newBlock("set_var");
    newBlock.setFieldValue("set", "OP");
    const valueBlock = ws.newBlock("_value");
    valueBlock.setFieldValue("5", "TEXT");
    const nameInput = newBlock.getInput("NAME");
    const valueInput = newBlock.getInput("VALUE");
    if (nameInput?.connection && valueBlock.outputConnection) {
      nameInput.connection.connect(valueBlock.outputConnection);
    }
    const valueChild = ws.newBlock("_value");
    valueChild.setFieldValue("10", "TEXT");
    if (valueInput?.connection && valueChild.outputConnection) {
      valueInput.connection.connect(valueChild.outputConnection);
    }

    sm.refresh();
    // After refresh with 2 blocks, grid is:
    // block(dv), slot(NAME), slot(VARTYPE), slot(VALUE), gap, block(sv), slot(NAME), slot(VALUE)
    const first = sm.getCurrentTarget()!;
    expect(first.type).toBe("block");
    sm.moveRight();
    const second = sm.getCurrentTarget()!;
    expect(second.type).toBe("slot"); // first slot after the block
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});

describe("SelectionManager — dispose", () => {
  it("dispose cleans up DOM and callback", () => {
    const { sm, ws } = setupWorkspace([declareVar("x")]);
    sm.dispose();
    expect(sm.onChange).toBeNull();
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });

  it("is safe to call dispose multiple times", () => {
    const { sm, ws } = setupWorkspace([declareVar("x")]);
    sm.dispose();
    sm.dispose(); // should not throw
    ws.dispose();
    const parent = ws.getParentSvg()?.parentElement;
    if (parent) parent.remove();
  });
});
