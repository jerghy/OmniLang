// poiesis/src/components/canvas/code/blocks/BlockSvg/CommentBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { CommentOp } from "../../../../../core/types/code-op";

export const CommentBlock = React.forwardRef<SVGGElement, { op: CommentOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.COMMENT} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle" fill="#fff" fontSize="13" fontFamily="monospace" fontStyle="italic">
          <tspan>// </tspan>
          <tspan>{op.text || "comment"}</tspan>
        </text>
      </BlockSvg>
    );
  },
);
CommentBlock.displayName = "CommentBlock";
