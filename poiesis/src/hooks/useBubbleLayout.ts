import { useMemo } from "react";
import type { BubbleNode } from "../core/types/architecture";

export function useBubbleLayout(
  bubbles: BubbleNode[],
  expandedIds: string[]
): Map<string, { x: number; y: number }> {
  return useMemo(() => {
    const positions = new Map<string, { x: number; y: number }>();
    const expanded = new Set(expandedIds);
    const visible = bubbles.filter(
      (b) => b.depth === 0 || (b.parentId && expanded.has(b.parentId))
    );

    // Top-level bubbles: horizontal spread
    const topLevel = visible.filter((b) => b.depth === 0);
    topLevel.forEach((b, i) => {
      positions.set(b.id, { x: 150 + i * 300, y: 100 });
    });

    // Children: grid below parent
    const childBubbles = visible.filter((b) => b.depth > 0);
    // Pre-compute sibling groups for O(n) layout
    const siblingGroups = new Map<string, BubbleNode[]>();
    childBubbles.forEach((b) => {
      const key = b.parentId ?? "__root__";
      if (!siblingGroups.has(key)) siblingGroups.set(key, []);
      siblingGroups.get(key)!.push(b);
    });
    childBubbles.forEach((b) => {
      const parentPos = positions.get(b.parentId!) || { x: 400, y: 100 };
      const siblings = siblingGroups.get(b.parentId!) ?? [];
      // Build a Map for O(1) position lookup instead of indexOf (O(n^2))
      const idxMap = new Map<string, number>();
      siblings.forEach((sib, i) => idxMap.set(sib.id, i));
      const idx = idxMap.get(b.id) ?? 0;
      const cols = Math.ceil(Math.sqrt(siblings.length));
      const row = Math.floor(idx / cols);
      const col = idx % cols;
      positions.set(b.id, {
        x: parentPos.x - (cols - 1) * 60 + col * 120,
        y: parentPos.y + 120 + row * 100,
      });
    });

    return positions;
  }, [bubbles, expandedIds]);
}
