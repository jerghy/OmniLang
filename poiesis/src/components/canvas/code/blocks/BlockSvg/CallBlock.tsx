// poiesis/src/components/canvas/code/blocks/BlockSvg/CallBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { CallFnOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const CallBlock = React.forwardRef<SVGGElement, { op: CallFnOp }>(
  ({ op }, ref) => {
    const argsCount = op.args.length;
    return (
      <BlockSvg colour={BLOCK_COLORS.FUNCTION} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <TextSlot slotId="FN_NAME" value={slotToText(op.fnName)} placeholder="fn" />
          <Keyword text="(" />
          <TextSlot
            slotId="ARGS"
            value={argsCount > 0 ? `${argsCount} args` : ""}
            placeholder="..."
          />
          <Keyword text=")" />
        </text>
      </BlockSvg>
    );
  },
);
CallBlock.displayName = "CallBlock";
