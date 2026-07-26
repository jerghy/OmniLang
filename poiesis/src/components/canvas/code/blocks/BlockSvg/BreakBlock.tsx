// poiesis/src/components/canvas/code/blocks/BlockSvg/BreakBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { BreakStmtOp } from "../../../../../core/types/code-op";

const BREAK_ITEMS: LayoutItem[] = [{ kind: "keyword", text: "break" }];

export const BreakBlock = React.forwardRef<SVGGElement, { op: BreakStmtOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.BREAK} blockId={op.pid} ref={ref} items={BREAK_ITEMS} />
    );
  },
);
BreakBlock.displayName = "BreakBlock";
