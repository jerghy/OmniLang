// poiesis/src/components/canvas/code/blocks/BlockSvg/SetVarBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { SetVarOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const SetVarBlock = React.forwardRef<SVGGElement, { op: SetVarOp }>(
  ({ op }, ref) => {
    const items: LayoutItem[] = [
      { kind: "slot", slotId: "NAME", value: slotToText(op.name), placeholder: "name" },
      { kind: "keyword", text: op.operator },
      { kind: "slot", slotId: "VALUE", value: slotToText(op.value), placeholder: "value" },
    ];
    return (
      <BlockSvg colour={BLOCK_COLORS.VARIABLE} blockId={op.pid} ref={ref} items={items} />
    );
  },
);
SetVarBlock.displayName = "SetVarBlock";
