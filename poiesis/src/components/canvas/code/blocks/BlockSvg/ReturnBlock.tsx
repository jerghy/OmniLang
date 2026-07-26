// poiesis/src/components/canvas/code/blocks/BlockSvg/ReturnBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { ReturnExprOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const ReturnBlock = React.forwardRef<SVGGElement, { op: ReturnExprOp }>(
  ({ op }, ref) => {
    const items: LayoutItem[] = [
      { kind: "keyword", text: "return" },
      { kind: "slot", slotId: "VALUE", value: op.value ? slotToText(op.value) : "", placeholder: "value" },
    ];
    return (
      <BlockSvg colour={BLOCK_COLORS.CONTROL} blockId={op.pid} ref={ref} items={items} />
    );
  },
);
ReturnBlock.displayName = "ReturnBlock";
