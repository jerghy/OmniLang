// poiesis/src/components/canvas/code/blocks/BlockSvg/AwaitBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { AwaitExprOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const AwaitBlock = React.forwardRef<SVGGElement, { op: AwaitExprOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.ASYNC} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <Keyword text="await" />
          <TextSlot slotId="VALUE" value={slotToText(op.value)} placeholder="promise" />
        </text>
      </BlockSvg>
    );
  },
);
AwaitBlock.displayName = "AwaitBlock";
