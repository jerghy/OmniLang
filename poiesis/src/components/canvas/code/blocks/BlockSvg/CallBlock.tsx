// poiesis/src/components/canvas/code/blocks/BlockSvg/CallBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { CallFnOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const CallBlock = React.forwardRef<SVGGElement, { op: CallFnOp }>(
  ({ op }, ref) => {
    const argsCount = op.args.length;
    const items: LayoutItem[] = [
      { kind: "slot", slotId: "FN_NAME", value: slotToText(op.fnName), placeholder: "fn" },
      { kind: "keyword", text: "(" },
      { kind: "slot", slotId: "ARGS", value: argsCount > 0 ? `${argsCount} args` : "", placeholder: "..." },
      { kind: "keyword", text: ")" },
    ];
    return (
      <BlockSvg colour={BLOCK_COLORS.FUNCTION} blockId={op.pid} ref={ref} items={items} />
    );
  },
);
CallBlock.displayName = "CallBlock";
