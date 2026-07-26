// poiesis/src/components/canvas/code/blocks/BlockSvg/ReturnBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { ReturnExprOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const ReturnBlock = React.forwardRef<SVGGElement, { op: ReturnExprOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.CONTROL} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <Keyword text="return" />
          <TextSlot slotId="VALUE" value={op.value ? slotToText(op.value) : ""} placeholder="value" />
        </text>
      </BlockSvg>
    );
  },
);
ReturnBlock.displayName = "ReturnBlock";
