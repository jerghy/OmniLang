// poiesis/src/components/canvas/code/blocks/BlockSvg/BreakBlock.tsx
import React from "react";
import { BlockSvg, Keyword, BLOCK_COLORS } from "../BlockSvg";
import type { BreakStmtOp } from "../../../../../core/types/code-op";

export const BreakBlock = React.forwardRef<SVGGElement, { op: BreakStmtOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.BREAK} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <Keyword text="break" />
        </text>
      </BlockSvg>
    );
  },
);
BreakBlock.displayName = "BreakBlock";
