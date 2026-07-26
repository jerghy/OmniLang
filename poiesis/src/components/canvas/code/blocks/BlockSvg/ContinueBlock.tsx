// poiesis/src/components/canvas/code/blocks/BlockSvg/ContinueBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { ContinueStmtOp } from "../../../../../core/types/code-op";

const CONTINUE_ITEMS: LayoutItem[] = [{ kind: "keyword", text: "continue" }];

export const ContinueBlock = React.forwardRef<SVGGElement, { op: ContinueStmtOp }>(
  ({ op }, ref) => {
    return (
      <BlockSvg colour={BLOCK_COLORS.BREAK} blockId={op.pid} ref={ref} items={CONTINUE_ITEMS} />
    );
  },
);
ContinueBlock.displayName = "ContinueBlock";
