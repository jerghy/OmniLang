// poiesis/src/components/canvas/code/blocks/BlockSvg.tsx
// Block shell + layout engine for Scratch-style SVG blocks.
// Keywords are bold white text; slots get tinted background rects.
import React from "react";

// ── Colors (Zelos palette) ──

export const BLOCK_COLORS: Record<string, string> = {
  CONTROL: "#FFAB19",
  VARIABLE: "#FF8C1A",
  FUNCTION: "#995BA5",
  ASYNC: "#5B67B0",
  BREAK: "#CF8B17",
  COMMENT: "#5CB85C",
  RAW: "#AAAAAA",
};

// ── Layout item types ──

export interface KeywordItem {
  kind: "keyword";
  text: string;
}

export interface SlotItem {
  kind: "slot";
  slotId: string;
  value: string;
  placeholder: string;
}

export type LayoutItem = KeywordItem | SlotItem;

// ── Layout constants ──

const FONT_SIZE = 13;
const CHAR_W = 7.8; // approximate monospace char width at 13px
const SLOT_PAD_X = 5;
const SLOT_PAD_Y = 2;
const SLOT_RADIUS = 4;
const ITEM_GAP = 3;
export const BLOCK_PAD_X = 12;
const BLOCK_PAD_Y = 10;

// ── Connector dimensions ──

export const CONNECTOR_W = 14;
export const CONNECTOR_H = 8;
export const RADIUS = 6;

// ── Slot background colour helpers (Scratch-style: white bg + dark text) ──

const SLOT_BG = "rgba(255,255,255,0.92)";
const SLOT_BG_PLACEHOLDER = "rgba(255,255,255,0.40)";
const SLOT_BORDER = "rgba(0,0,0,0.15)";
const SLOT_TEXT = "rgba(0,0,0,0.80)";
const SLOT_TEXT_PLACEHOLDER = "rgba(0,0,0,0.35)";
const KEYWORD_FILL = "rgba(255,255,255,0.92)";

function slotBg(isPlaceholder: boolean): string {
  return isPlaceholder ? SLOT_BG_PLACEHOLDER : SLOT_BG;
}

function slotStroke(_isPlaceholder: boolean): string {
  return SLOT_BORDER;
}

function slotTextFill(isPlaceholder: boolean): string {
  return isPlaceholder ? SLOT_TEXT_PLACEHOLDER : SLOT_TEXT;
}

function slotTextWeight(isPlaceholder: boolean): "normal" | "bold" {
  return isPlaceholder ? "normal" : "bold";
}

// ── Block height ──

export const BLOCK_H = FONT_SIZE + BLOCK_PAD_Y * 2;

// ── Layout engine ──

export interface LayoutResult {
  elements: React.ReactNode[];
  contentWidth: number;
}

export function layoutItems(items: LayoutItem[], startX: number = BLOCK_PAD_X): LayoutResult {
  let cx = startX;
  const elements: React.ReactNode[] = [];

  items.forEach((item, i) => {
    if (item.kind === "keyword") {
      const w = item.text.length * CHAR_W;
      elements.push(
        <g key={`kw-${i}`} transform={`translate(${cx}, 0)`}>
          <text dominantBaseline="middle" style={{ userSelect: "none" }}>
            <tspan
              fill={KEYWORD_FILL}
              fontSize={FONT_SIZE}
              fontFamily="monospace"
              fontWeight="bold"
            >
              {item.text}
            </tspan>
          </text>
        </g>,
      );
      cx += w + ITEM_GAP;
    } else {
      const display = item.value || item.placeholder;
      const isPlaceholder = !item.value;
      const slotW = Math.max(display.length * CHAR_W + SLOT_PAD_X * 2, 28);

      elements.push(
        <g key={`sl-${i}`} transform={`translate(${cx}, 0)`}>
          {/* Slot background */}
          <rect
            x={0}
            y={-(FONT_SIZE + SLOT_PAD_Y)}
            width={slotW}
            height={(FONT_SIZE + SLOT_PAD_Y) * 2}
            rx={SLOT_RADIUS}
            ry={SLOT_RADIUS}
            fill={slotBg(isPlaceholder)}
            stroke={slotStroke(isPlaceholder)}
            strokeWidth={0.5}
          />
          {/* Slot text */}
          <text x={SLOT_PAD_X} dominantBaseline="middle">
            <tspan
              data-slot-id={item.slotId}
              fill={slotTextFill(isPlaceholder)}
              fontSize={FONT_SIZE}
              fontFamily="monospace"
              fontWeight={slotTextWeight(isPlaceholder)}
            >
              {display}
            </tspan>
          </text>
        </g>,
      );
      cx += slotW + ITEM_GAP;
    }
  });

  const contentWidth = cx - startX - ITEM_GAP + BLOCK_PAD_X;
  return { elements, contentWidth };
}

// ── BlockSvg shell ──

interface BlockSvgProps {
  colour: string;
  blockId: string;
  items: LayoutItem[];
  hasTopConnector?: boolean;
  hasBottomConnector?: boolean;
  children?: React.ReactNode;
}

export const BlockSvg = React.forwardRef<SVGGElement, BlockSvgProps>(
  ({ colour, blockId, items, hasTopConnector: _ht, hasBottomConnector: _hb, children }, ref) => {
    const { elements, contentWidth } = layoutItems(items);
    const totalW = Math.max(contentWidth, 60);

    return (
      <g ref={ref} data-block-id={blockId}>
        {/* Main body — clean rounded rect, no connectors */}
        <rect
          x={0}
          y={0}
          width={totalW}
          height={BLOCK_H}
          rx={RADIUS}
          ry={RADIUS}
          fill={colour}
          stroke="rgba(0,0,0,0.15)"
          strokeWidth={1}
        />

        {/* Content */}
        <g transform={`translate(0, ${BLOCK_H / 2})`}>
          {elements}
          {children}
        </g>
      </g>
    );
  },
);
BlockSvg.displayName = "BlockSvg";

// ── Legacy inline slot (fallback for contexts where layout engine can't be used) ──

export const TextSlotInline: React.FC<{
  slotId: string;
  value: string;
  placeholder: string;
}> = ({ slotId, value, placeholder }) => {
  const display = value || placeholder;
  const isPlaceholder = !value;
  return (
    <tspan
      data-slot-id={slotId}
      fill={slotTextFill(isPlaceholder)}
      fontSize={FONT_SIZE}
      fontFamily="monospace"
      fontWeight={slotTextWeight(isPlaceholder)}
    >
      {display}
    </tspan>
  );
};
