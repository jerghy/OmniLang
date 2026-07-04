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
    childBubbles.forEach((b) => {
      const parentPos = positions.get(b.parentId!) || { x: 400, y: 100 };
      const siblings = childBubbles.filter((c) => c.parentId === b.parentId);
      const idx = siblings.indexOf(b);
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
