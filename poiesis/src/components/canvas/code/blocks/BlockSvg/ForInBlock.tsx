// poiesis/src/components/canvas/code/blocks/BlockSvg/ForInBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { ForLoopOp, CodeOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

interface ForInBlockProps {
  op: ForLoopOp;
  renderBody: (ops: CodeOp[]) => React.ReactNode;
}

const BODY_BLOCK_HEIGHT = 48;

export const ForInBlock = React.forwardRef<SVGGElement, ForInBlockProps>(
  ({ op, renderBody }, ref) => {
    const itemName = slotToText(op.itemName);
    const collection = slotToText(op.collection);
    const bottomY = 36 + op.body.length * BODY_BLOCK_HEIGHT + 8;
    return (
      <g ref={ref} data-block-id={op.pid}>
        {/* Header */}
        <BlockSvg
          colour={BLOCK_COLORS.CONTROL}
          blockId={`${op.pid}-header`}
          hasTopConnector={true}
          hasBottomConnector={false}
        >
          <text dominantBaseline="middle">
            <Keyword text="for" />
            <TextSlot
              slotId="ITEM"
              value={itemName}
              placeholder="item"
            />
            <Keyword text="in" />
            <TextSlot
              slotId="COLLECTION"
              value={collection}
              placeholder="collection"
            />
          </text>
        </BlockSvg>

        {/* Body area — indented */}
        {op.body.length > 0 && (
          <g transform="translate(24, 36)">{renderBody(op.body)}</g>
        )}

        {/* Bottom connector — dynamically positioned after body blocks */}
        <rect
          x={0}
          y={bottomY}
          width={200}
          height={8}
          fill={BLOCK_COLORS.CONTROL}
          rx={2}
        />
      </g>
    );
  },
);
ForInBlock.displayName = "ForInBlock";
