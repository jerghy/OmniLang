// poiesis/src/components/canvas/code/blocks/BlockSvg/IfBlock.tsx
import React from "react";
import { BlockSvg, Keyword, TextSlot, BLOCK_COLORS } from "../BlockSvg";
import type { IfBlockOp, CodeOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

interface IfBlockProps {
  op: IfBlockOp;
  renderBody: (ops: CodeOp[]) => React.ReactNode;
}

export const IfBlock = React.forwardRef<SVGGElement, IfBlockProps>(
  ({ op, renderBody }, ref) => {
    return (
      <g ref={ref} data-block-id={op.pid}>
        {op.branches.map((branch, i) => {
          const isElse = branch.condition.kind === "empty";
          const prefix = i === 0 ? "if" : isElse ? "else" : "elif";
          const cond = slotToText(branch.condition);

          return (
            <g key={i} transform={`translate(0, ${i * 60})`}>
              {/* Branch header */}
              <BlockSvg
                colour={BLOCK_COLORS.CONTROL}
                blockId={`${op.pid}-br-${i}`}
                hasTopConnector={i === 0}
                hasBottomConnector={false}
              >
                <text dominantBaseline="middle">
                  <Keyword text={prefix} />
                  {!isElse && (
                    <TextSlot
                      slotId={`CONDITION_${i}`}
                      value={cond}
                      placeholder="condition"
                    />
                  )}
                </text>
              </BlockSvg>

              {/* Body area — indented */}
              {branch.body.length > 0 && (
                <g transform="translate(24, 36)">{renderBody(branch.body)}</g>
              )}
            </g>
          );
        })}

        {/* Bottom connector */}
        <rect
          x={0}
          y={op.branches.length * 60 + 24}
          width={200}
          height={8}
          fill={BLOCK_COLORS.CONTROL}
          rx={2}
        />
      </g>
    );
  },
);
IfBlock.displayName = "IfBlock";
