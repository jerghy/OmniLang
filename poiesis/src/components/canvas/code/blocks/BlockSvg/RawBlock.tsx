// poiesis/src/components/canvas/code/blocks/BlockSvg/RawBlock.tsx
// RawBlock — dual-mode display/edit component for RawCodeOp.

import React, { useRef, useEffect } from "react";
import { BlockSvg, BLOCK_COLORS } from "../BlockSvg";
import type { LayoutItem } from "../BlockSvg";
import type { RawCodeOp } from "../../../../../core/types/code-op";

interface RawBlockProps {
  op: RawCodeOp;
  onEdit: (newSource: string) => void;
  isEditing: boolean;
  onToggleEdit: () => void;
}

export const RawBlock = React.forwardRef<SVGGElement, RawBlockProps>(
  ({ op, onEdit, isEditing, onToggleEdit }, ref) => {
    const inputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
      if (isEditing) inputRef.current?.focus();
    }, [isEditing]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        if (inputRef.current) onEdit(inputRef.current.value);
        onToggleEdit();
      } else if (e.key === "Escape") {
        onToggleEdit();
      }
    };

    const items: LayoutItem[] = [
      { kind: "keyword", text: "raw " + (op.source || "") },
    ];

    return (
      <g ref={ref} data-block-id={op.pid}>
        {isEditing ? (
          <BlockSvg colour={BLOCK_COLORS.RAW} blockId={op.pid} items={items}>
            <foreignObject x={40} y={-11} width={300} height={22}>
              <input
                ref={inputRef}
                defaultValue={op.source}
                onKeyDown={handleKeyDown}
                aria-label="Edit raw code"
                style={{
                  width: "100%",
                  height: "100%",
                  border: "none",
                  background: "transparent",
                  color: "#fff",
                  fontSize: "13px",
                  fontFamily: "monospace",
                  outline: "none",
                }}
              />
            </foreignObject>
          </BlockSvg>
        ) : (
          <BlockSvg colour={BLOCK_COLORS.RAW} blockId={op.pid} ref={ref} items={items} />
        )}
      </g>
    );
  },
);
RawBlock.displayName = "RawBlock";
