// poiesis/src/components/canvas/code/blocks/BlockSvg/CommentBlock.tsx
import React from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { CommentOp } from "../../../../../core/types/code-op";

export const CommentBlock = React.forwardRef<SVGGElement, { op: CommentOp }>(
  ({ op }, ref) => {
    const items: LayoutItem[] = [
      { kind: "keyword", text: "// " + (op.text || "comment") },
    ];
    return (
      <BlockSvg colour={BLOCK_COLORS.COMMENT} blockId={op.pid} ref={ref} items={items} />
    );
  },
);
CommentBlock.displayName = "CommentBlock";
