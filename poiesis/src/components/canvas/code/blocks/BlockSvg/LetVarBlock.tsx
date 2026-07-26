// poiesis/src/components/canvas/code/blocks/BlockSvg/LetVarBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { DeclareVarOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const LetVarBlock = React.forwardRef<SVGGElement, { op: DeclareVarOp }>(
  ({ op }, ref) => {
    const mutLabel = op.mutable ? "let mut" : "let";
    const items: LayoutItem[] = [
      { kind: "keyword", text: mutLabel },
      { kind: "slot", slotId: "NAME", value: slotToText(op.name), placeholder: "name" },
      { kind: "keyword", text: ":" },
      { kind: "slot", slotId: "VARTYPE", value: slotToText(op.varType || { kind: "empty" }), placeholder: "type" },
      { kind: "keyword", text: "=" },
      { kind: "slot", slotId: "VALUE", value: slotToText(op.initValue || { kind: "empty" }), placeholder: "value" },
    ];
    return (
      <BlockSvg colour={BLOCK_COLORS.VARIABLE} blockId={op.pid} ref={ref} items={items} />
    );
  },
);
LetVarBlock.displayName = "LetVarBlock";
