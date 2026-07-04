import { usePoiesisStore } from "../store";
import type { BubbleNode } from "../core/types/architecture";

/**
 * Hook for linked expansion logic.
 * When a top-level bubble is expanded, siblings are also expanded automatically.
 * The store's expandBubble action already implements this linked behavior.
 */
export function useBubbleExpand() {
  const expandedIds = usePoiesisStore((s) => s.architecture.expandedBubbleIds);
  const bubbles = usePoiesisStore((s) => s.architecture.bubbles);
  const expandBubble = usePoiesisStore((s) => s.expandBubble);
  const collapseBubble = usePoiesisStore((s) => s.collapseBubble);

  const isExpanded = (bubbleId: string): boolean => {
    return expandedIds.includes(bubbleId);
  };

  const toggleBubble = (bubbleId: string): void => {
    if (expandedIds.includes(bubbleId)) {
      collapseBubble(bubbleId);
    } else {
      expandBubble(bubbleId);
    }
  };

  const expandAll = (): void => {
    const allIds = bubbles.map((b: BubbleNode) => b.id);
    allIds.forEach((id: string) => {
      if (!expandedIds.includes(id)) {
        expandBubble(id);
      }
    });
  };

  const collapseAll = (): void => {
    const childIds = bubbles
      .filter((b: BubbleNode) => b.depth > 0)
      .map((b: BubbleNode) => b.id);
    // Only collapse children, keep top-level visible
    usePoiesisStore.setState((s) => ({
      architecture: {
        ...s.architecture,
        expandedBubbleIds: s.architecture.expandedBubbleIds.filter(
          (id) => !childIds.includes(id)
        ),
      },
    }));
  };

  return {
    expandedIds,
    isExpanded,
    toggleBubble,
    expandBubble,
    collapseBubble,
    expandAll,
    collapseAll,
  };
}
