// poiesis/src/components/canvas/code/blocks/BlockSvg/ForInBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS, BLOCK_H, layoutItems } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { ForLoopOp, CodeOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

interface ForInBlockProps {
  op: ForLoopOp;
  renderBody: (ops: CodeOp[]) => React.ReactNode;
}

const BODY_BLOCK_SPACING = BLOCK_H + 4;
const ARM_W = 6;
const BODY_GAP = 3;

export const ForInBlock = React.forwardRef<SVGGElement, ForInBlockProps>(
  ({ op, renderBody }, ref) => {
    const itemName = slotToText(op.itemName);
    const collection = slotToText(op.collection);

    const items: LayoutItem[] = [
      { kind: "keyword", text: "for" },
      { kind: "slot", slotId: "ITEM", value: itemName, placeholder: "item" },
      { kind: "keyword", text: "in" },
      { kind: "slot", slotId: "COLLECTION", value: collection, placeholder: "collection" },
    ];

    const { contentWidth } = layoutItems(items);
    const headerW = Math.max(contentWidth, 60);

    const hasBody = op.body.length > 0;
    const bodyAreaStartY = BLOCK_H + BODY_GAP;
    const bodyContentH = hasBody ? op.body.length * BODY_BLOCK_SPACING : 0;
    const bodyAreaH = bodyContentH + 6;
    const armHeight = bodyAreaStartY + bodyAreaH;
    const capY = armHeight;

    const color = BLOCK_COLORS.CONTROL;

    return (
      <g ref={ref} data-block-id={op.pid}>
        {/* ── Left arm ── */}
        <rect x={0} y={0} width={ARM_W} height={armHeight} fill={color} />

        {/* ── Header shifted right by arm ── */}
        <g transform={`translate(${ARM_W}, 0)`}>
          <BlockSvg
            colour={color}
            blockId={`${op.pid}-header`}
            items={items}
            hasTopConnector={true}
            hasBottomConnector={false}
          />
        </g>

        {/* ── Body area ── */}
        {hasBody && (
          <g transform={`translate(${ARM_W + 8}, ${bodyAreaStartY})`}>
            {renderBody(op.body)}
          </g>
        )}

        {/* ── Bottom cap ── */}
        <g transform={`translate(0, ${capY})`}>
          <rect x={0} y={0} width={headerW + ARM_W} height={5}
            fill={color} stroke="rgba(0,0,0,0.12)" strokeWidth={1} rx={2} />
        </g>
      </g>
    );
  },
);
ForInBlock.displayName = "ForInBlock";
