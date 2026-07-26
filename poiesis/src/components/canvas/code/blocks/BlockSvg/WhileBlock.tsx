// poiesis/src/components/canvas/code/blocks/BlockSvg/WhileBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { WhileLoopOp, CodeOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

interface WhileBlockProps {
  op: WhileLoopOp;
  renderBody: (ops: CodeOp[]) => React.ReactNode;
}

export const WhileBlock = React.forwardRef<SVGGElement, WhileBlockProps>(
  ({ op, renderBody }, ref) => {
    const cond = slotToText(op.condition);
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
            <Keyword text="while" />
            <TextSlot
              slotId="CONDITION"
              value={cond}
              placeholder="condition"
            />
          </text>
        </BlockSvg>

        {/* Body area — indented */}
        {op.body.length > 0 && (
          <g transform="translate(24, 36)">{renderBody(op.body)}</g>
        )}

        {/* Bottom connector */}
        <rect
          x={0}
          y={84}
          width={200}
          height={8}
          fill={BLOCK_COLORS.CONTROL}
          rx={2}
        />
      </g>
    );
  },
);
WhileBlock.displayName = "WhileBlock";
