// components/design/cards/BackgroundCard.tsx
// Background color editing card with HSV panel for inline color picking.

import React, { useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { HSVPanel, hexToHsv, hsvToHex } from "../HSVPanel";
import type { HSVColor } from "../HSVPanel";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils";

// ── Helpers ──

function getProp(props: StyleProperty[], name: string): string {
  const prop = props.find((p) => p.name === name);
  if (!prop || prop.segments.length === 0) return "";
  const seg = prop.segments[0];
  return seg.type === "text" ? seg.value : "";
}

function setProp(
  props: StyleProperty[],
  name: string,
  value: string,
): StyleProperty[] {
  const existing = props.find((p) => p.name === name);
  if (existing) {
    return props.map((p) =>
      p.name === name
        ? { ...p, segments: [{ type: "text" as const, value }] }
        : p,
    );
  }
  return [
    ...props,
    {
      id: uid("prop"),
      name,
      segments: [{ type: "text", value }],
    },
  ];
}

// ── Props ──

interface BackgroundCardProps {
  properties: StyleProperty[];
  tokens: DesignToken[];
  onChange: (props: StyleProperty[]) => void;
  onFocus?: () => void;
}

// ── Component ──

export const BackgroundCard = React.memo(function BackgroundCard({
  properties,
  tokens,
  onChange,
  onFocus,
}: BackgroundCardProps) {
  const bgValue = getProp(properties, "background");

  const isHex = /^#[0-9a-fA-F]{6}$/.test(bgValue);
  const summary = bgValue || "none";

  const hex = isHex ? bgValue : "#ffffff";
  const hsv = hexToHsv(hex);
  const hsvColor: HSVColor = { h: hsv.h, s: hsv.s, v: hsv.v, a: 1 };

  const handleColorChange = useCallback(
    (c: HSVColor) => {
      const newHex = hsvToHex(c.h, c.s, c.v);
      onChange(setProp(properties, "background", newHex));
    },
    [properties, onChange],
  );

  return (
    <CollapsibleGroup title="背景" summary={summary} defaultOpen={false} onDoubleClick={() => onFocus?.()}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
        }}
      >
        {/* Current color indicator */}
        {isHex && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              marginBottom: 2,
            }}
          >
            <span
              style={{
                width: 18,
                height: 18,
                borderRadius: "var(--dp-radius-sm, 6px)",
                background: bgValue,
                border: "1px solid var(--dp-border, #e8e5f2)",
                flexShrink: 0,
              }}
            />
            <span
              style={{
                fontSize: 12,
                fontFamily: "var(--dp-font-mono, monospace)",
                color: "var(--dp-text, #1e1b4b)",
              }}
            >
              {bgValue}
            </span>
          </div>
        )}

        <HSVPanel color={hsvColor} onChange={handleColorChange} showHexInput />
      </div>
    </CollapsibleGroup>
  );
});
