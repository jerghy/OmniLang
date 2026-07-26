// poiesis/src/components/canvas/code/blocks/BlockSvg/RawBlock.tsx
// RawBlock — dual-mode display/edit component for RawCodeOp.

import React, { useState, useRef, useEffect } from "react";
import { BlockSvg, Keyword, BLOCK_COLORS } from "../BlockSvg";
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

    return (
      <g ref={ref} data-block-id={op.pid}>
        <BlockSvg
          colour={BLOCK_COLORS.RAW}
          blockId={op.pid}
        >
          {isEditing ? (
            <foreignObject x={0} y={-10} width={300} height={24}>
              <input
                ref={inputRef}
                defaultValue={op.source}
                onKeyDown={handleKeyDown}
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
          ) : (
            <text dominantBaseline="middle">
              <Keyword text="raw" />
              <tspan fill="#fff" fontSize="12" fontFamily="monospace">
                {op.source || "(empty)"}
              </tspan>
            </text>
          )}
        </BlockSvg>
      </g>
    );
  },
);
RawBlock.displayName = "RawBlock";
