// poiesis/src/components/canvas/code/blocks/BlockSvg/ContinueBlock.tsx
import React from "react";
import { BlockSvg, Keyword, BLOCK_COLORS } from "../BlockSvg";
import type { ContinueStmtOp } from "../../../../../core/types/code-op";

export const ContinueBlock = React.forwardRef<SVGGElement, { op: ContinueStmtOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.BREAK} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <Keyword text="continue" />
        </text>
      </BlockSvg>
    );
  },
);
ContinueBlock.displayName = "ContinueBlock";
