// poiesis/src/components/canvas/code/blocks/BlockSvg/SetVarBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { SetVarOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const SetVarBlock = React.forwardRef<SVGGElement, { op: SetVarOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.VARIABLE} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <TextSlot slotId="NAME" value={slotToText(op.name)} placeholder="name" />
          <Keyword text={op.operator} />
          <TextSlot slotId="VALUE" value={slotToText(op.value)} placeholder="value" />
        </text>
      </BlockSvg>
    );
  },
);
SetVarBlock.displayName = "SetVarBlock";
