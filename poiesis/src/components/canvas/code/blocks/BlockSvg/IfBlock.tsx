// poiesis/src/components/canvas/code/blocks/BlockSvg/IfBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS, BLOCK_H, layoutItems } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { IfBlockOp, CodeOp } from "../../../../../core/types/code-op";
import { slotToText } from "../../../../../core/types/slot-value";

interface IfBlockProps {
  op: IfBlockOp;
  renderBody: (ops: CodeOp[]) => React.ReactNode;
}

const BODY_BLOCK_SPACING = BLOCK_H + 4;
const ARM_W = 6;
const BRANCH_GAP = 2;
const BODY_GAP = 3;

export const IfBlock = React.forwardRef<SVGGElement, IfBlockProps>(
  ({ op, renderBody }, ref) => {
    const color = BLOCK_COLORS.CONTROL;

    // Compute widest branch header
    let maxHeaderW = 0;
    op.branches.forEach((branch) => {
      const isLastElse = branch.condition.kind === "empty" && op.branches.indexOf(branch) === op.branches.length - 1;
      const prefix = op.branches.indexOf(branch) === 0 ? "if"
        : branch.condition.kind === "empty" ? "else" : "elif";
      const cond = slotToText(branch.condition);
      const items: LayoutItem[] = isLastElse
        ? [{ kind: "keyword", text: prefix }]
        : [
            { kind: "keyword", text: prefix },
            { kind: "slot", slotId: "CONDITION", value: cond, placeholder: "condition" },
          ];
      const { contentWidth } = layoutItems(items);
      if (contentWidth > maxHeaderW) maxHeaderW = contentWidth;
    });
    const headerW = Math.max(maxHeaderW, 60);

    // Compute branch Y positions and total height
    let cumY = 0;
    const branchY: number[] = [];
    op.branches.forEach((branch) => {
      branchY.push(cumY);
      cumY += BLOCK_H + BODY_GAP + branch.body.length * BODY_BLOCK_SPACING + 4 + BRANCH_GAP;
    });

    const armHeight = cumY + 5;
    const capY = armHeight;

    return (
      <g ref={ref} data-block-id={op.pid}>
        {/* ── Left arm spanning entire block height ── */}
        <rect x={0} y={0} width={ARM_W} height={armHeight} fill={color} />

        {/* ── Branches ── */}
        {op.branches.map((branch, i) => {
          const isFirst = i === 0;
          const isLastElse = branch.condition.kind === "empty" && i === op.branches.length - 1;
          const prefix = isFirst ? "if" : isLastElse ? "else" : "elif";
          const cond = slotToText(branch.condition);

          const items: LayoutItem[] = isLastElse
            ? [{ kind: "keyword", text: prefix }]
            : [
                { kind: "keyword", text: prefix },
                { kind: "slot", slotId: "CONDITION", value: cond, placeholder: "condition" },
              ];

          const branchHeaderY = branchY[i];
          const bodyY = branchHeaderY + BLOCK_H + BODY_GAP;

          return (
            <g key={i}>
              {/* Branch header — shifted right by arm */}
              <g transform={`translate(${ARM_W}, ${branchHeaderY})`}>
                {isFirst ? (
                  <BlockSvg
                    colour={color}
                    blockId={`${op.pid}-br-${i}`}
                    items={items}
                    hasTopConnector={true}
                    hasBottomConnector={false}
                  />
                ) : (
                  <g>
                    <rect x={0} y={0} width={headerW} height={BLOCK_H}
                      fill={color} stroke="rgba(0,0,0,0.12)" strokeWidth={1} />
                    <line x1={0} y1={0} x2={headerW} y2={0}
                      stroke="rgba(255,255,255,0.25)" strokeWidth={1} />
                    <g transform={`translate(0, ${BLOCK_H / 2})`}>
                      {layoutItems(items).elements}
                    </g>
                  </g>
                )}
              </g>

              {/* Branch body — to the right of left arm */}
              {branch.body.length > 0 && (
                <g transform={`translate(${ARM_W + 8}, ${bodyY})`}>
                  {renderBody(branch.body)}
                </g>
              )}
            </g>
          );
        })}

        {/* ── Bottom cap ── */}
        <g transform={`translate(0, ${capY})`}>
          <rect x={0} y={0} width={headerW + ARM_W} height={5}
            fill={color} stroke="rgba(0,0,0,0.12)" strokeWidth={1} rx={2} />
        </g>
      </g>
    );
  },
);
IfBlock.displayName = "IfBlock";
