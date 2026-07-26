// poiesis/src/components/canvas/code/blocks/GapManager.ts
// Task 6: Gap row management for the Block Editor.
//
// Blockly has no native "gap row" concept. GapManager simulates insertion
// points between blocks, allowing the SelectionManager to position the
// cursor on a "gap" and enabling the user to insert new blocks there.

import * as Blockly from "blockly";

/**
 * Manages gap rows (insertion points) between blocks in a Blockly workspace.
 *
 * Gaps are identified by their position index (0 = before first block,
 * 0.5 = between block 0 and block 1, etc.)
 */
export class GapManager {
  private gaps = new Set<number>();
  private workspace: Blockly.WorkspaceSvg;

  constructor(workspace: Blockly.WorkspaceSvg) {
    this.workspace = workspace;
  }

  /**
   * Check if a position has a gap.
   */
  hasGap(position: number): boolean {
    return this.gaps.has(position);
  }

  /**
   * Create a gap at the given position.
   * Returns true if created, false if already exists.
   */
  insertGap(position: number): boolean {
    if (this.gaps.has(position)) return false;
    this.gaps.add(position);
    return true;
  }

  /**
   * Delete a gap at the given position.
   * Removes the gap entry from the set.
   */
  deleteGap(position: number): boolean {
    return this.gaps.delete(position);
  }

  /**
   * Get all gap positions, sorted.
   */
  getGapPositions(): number[] {
    return [...this.gaps].sort((a, b) => a - b);
  }

  /**
   * Clear all gaps.
   */
  clear(): void {
    this.gaps.clear();
  }

  /**
   * Ensure initial gaps exist: one before first block, gaps between all blocks.
   */
  initializeForBlockCount(count: number): void {
    this.clear();
    // Gap before first block
    this.gaps.add(0);
    // Gaps between blocks
    for (let i = 1; i <= count; i++) {
      this.gaps.add(i - 0.5);
    }
  }

  dispose(): void {
    this.gaps.clear();
  }
}
