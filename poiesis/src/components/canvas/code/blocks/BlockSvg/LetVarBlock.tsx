// poiesis/src/components/canvas/code/blocks/BlockSvg/LetVarBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { DeclareVarOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

export const LetVarBlock = React.forwardRef<SVGGElement, { op: DeclareVarOp }>(
  ({ op }, ref) => {
    const mutLabel = op.mutable ? "let mut" : "let";
    return (
      <BlockSvg colour={BLOCK_COLORS.VARIABLE} blockId={op.pid} ref={ref}>
        <text dominantBaseline="middle">
          <Keyword text={mutLabel} />
          <TextSlot slotId="NAME" value={slotToText(op.name)} placeholder="name" />
          <Keyword text=":" />
          <TextSlot slotId="VARTYPE" value={slotToText(op.varType || { kind: "empty" })} placeholder="type" />
          <Keyword text="=" />
          <TextSlot slotId="VALUE" value={slotToText(op.initValue || { kind: "empty" })} placeholder="value" />
        </text>
      </BlockSvg>
    );
  },
);
LetVarBlock.displayName = "LetVarBlock";
