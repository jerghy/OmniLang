// poiesis/src/components/canvas/code/blocks/__tests__/GapManager.test.ts
// Task 6: GapManager — gap row management tests.

import { describe, it, expect } from "vitest";
import { GapManager } from "../GapManager";

function makeGapManager(): GapManager {
  const div = document.createElement("div");
  document.body.appendChild(div);
  // Use minimal mock — GapManager only needs workspace for constructor
  return new GapManager({} as any);
}

describe("GapManager", () => {
  it("starts with no gaps", () => {
    const gm = makeGapManager();
    expect(gm.getGapPositions()).toEqual([]);
  });

  it("inserts a gap", () => {
    const gm = makeGapManager();
    expect(gm.insertGap(0)).toBe(true);
    expect(gm.hasGap(0)).toBe(true);
  });

  it("does not duplicate gaps", () => {
    const gm = makeGapManager();
    gm.insertGap(0);
    expect(gm.insertGap(0)).toBe(false);
  });

  it("deletes a gap", () => {
    const gm = makeGapManager();
    gm.insertGap(0);
    gm.deleteGap(0);
    expect(gm.hasGap(0)).toBe(false);
  });

  it("sorts gap positions", () => {
    const gm = makeGapManager();
    gm.insertGap(1.5);
    gm.insertGap(0);
    gm.insertGap(0.5);
    expect(gm.getGapPositions()).toEqual([0, 0.5, 1.5]);
  });

  it("initializeForBlockCount creates gaps", () => {
    const gm = makeGapManager();
    gm.initializeForBlockCount(2);
    // Should have: gap at 0, gap at 0.5 (between 0 and 1), gap at 1.5 (after block 1)
    const positions = gm.getGapPositions();
    expect(positions).toContain(0);
    expect(positions).toContain(0.5);
    expect(positions).toContain(1.5);
  });

  it("dispose clears all gaps", () => {
    const gm = makeGapManager();
    gm.insertGap(0);
    gm.dispose();
    expect(gm.getGapPositions()).toEqual([]);
  });
});
