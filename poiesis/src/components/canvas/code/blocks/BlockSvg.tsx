// poiesis/src/components/canvas/code/blocks/BlockSvg.tsx
import React from "react";

export const BLOCK_COLORS: Record<string, string> = {
  CONTROL: "#FFAB19",
  VARIABLE: "#FF8C1A",
  FUNCTION: "#995BA5",
  ASYNC: "#5B67B0",
  BREAK: "#CF8B17",
  COMMENT: "#5CB85C",
  RAW: "#AAAAAA",
};

interface BlockSvgProps {
  colour: string;
  blockId: string;
  children: React.ReactNode;
  /** Whether to render top connector (previousStatement). */
  hasTopConnector?: boolean;
  /** Whether to render bottom connector (nextStatement). */
  hasBottomConnector?: boolean;
}

const H_PX = 32; // block height
const CONNECTOR_W = 12;
const CONNECTOR_H = 8;
const RADIUS = 6;

/** Top puzzle connector: notch (convex bump). */
function TopConnector({ x }: { x: number }) {
  return (
    <path
      d={`M ${x} 0 L ${x + CONNECTOR_W} 0 L ${x + CONNECTOR_W} -${CONNECTOR_H}`}
      fill="none"
      stroke="rgba(0,0,0,0.15)"
      strokeWidth={1}
    />
  );
}

/** Bottom puzzle connector: indentation (concave notch). */
function BottomConnector({ x, y }: { x: number; y: number }) {
  return (
    <path
      d={`M ${x} ${y} L ${x + CONNECTOR_W} ${y} L ${x + CONNECTOR_W} ${
        y + CONNECTOR_H
      }`}
      fill="none"
      stroke="rgba(0,0,0,0.15)"
      strokeWidth={1}
    />
  );
}

export const BlockSvg = React.forwardRef<SVGGElement, BlockSvgProps>(
  ({ colour, blockId, children, hasTopConnector = true, hasBottomConnector = true }, ref) => {
    const textWidth = 200;
    const totalW = textWidth + 20;
    const connectorX = 20;

    return (
      <g ref={ref} data-block-id={blockId}>
        {/* Top connector notch */}
        {hasTopConnector && <TopConnector x={connectorX} />}

        {/* Main body: rounded rect */}
        <rect
          x={0}
          y={0}
          width={totalW}
          height={H_PX}
          rx={RADIUS}
          ry={RADIUS}
          fill={colour}
          stroke="rgba(0,0,0,0.15)"
          strokeWidth={1}
        />

        {/* Content */}
        <g transform={`translate(8, ${H_PX / 2})`}>{children}</g>

        {/* Bottom connector indentation */}
        {hasBottomConnector && (
          <BottomConnector x={connectorX} y={H_PX} />
        )}
      </g>
    );
  },
);

BlockSvg.displayName = "BlockSvg";

/** A text slot within a block — wrapped in <g> so it has a measurable bounding box for selection. */
export const TextSlot: React.FC<{
  slotId: string;
  value: string;
  placeholder: string;
}> = ({ slotId, value, placeholder }) => {
  const display = value || placeholder;
  const isPlaceholder = !value;
  return (
    <g data-slot-id={slotId}>
      <tspan
        fill={isPlaceholder ? "#999" : "#fff"}
        fontSize="13"
        fontFamily="monospace"
        fontWeight={isPlaceholder ? "normal" : "bold"}
      >
        {display}
      </tspan>
    </g>
  );
};

/** A keyword label within a block. */
export const Keyword: React.FC<{ text: string; children?: never }> = ({ text }) => (
  <tspan fill="#fff" fontSize="13" fontFamily="monospace" fontWeight="bold">
    {text}{" "}
  </tspan>
);
