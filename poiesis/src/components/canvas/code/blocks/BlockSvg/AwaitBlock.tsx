// poiesis/src/components/canvas/code/blocks/BlockSvg/AwaitBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { AwaitExprOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const AwaitBlock = React.forwardRef<SVGGElement, { op: AwaitExprOp }>(
  ({ op }, ref) => {
    const items: LayoutItem[] = [
      { kind: "keyword", text: "await" },
      { kind: "slot", slotId: "VALUE", value: slotToText(op.value), placeholder: "promise" },
    ];
    return (
      <BlockSvg colour={BLOCK_COLORS.ASYNC} blockId={op.pid} ref={ref} items={items} />
    );
  },
);
AwaitBlock.displayName = "AwaitBlock";
