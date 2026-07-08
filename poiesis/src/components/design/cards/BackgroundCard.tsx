// components/design/cards/BackgroundCard.tsx
// Background color editing card with HSV panel for inline color picking.

import { useCallback } from "react";
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
}

// ── Component ──

export function BackgroundCard({
  properties,
  tokens,
  onChange,
}: BackgroundCardProps) {
  const bgValue = getProp(properties, "background");
  const summary = bgValue || "none";

  // Parse hex value to HSV; default to white if not a valid hex.
  const hex = /^#[0-9a-fA-F]{6}$/.test(bgValue) ? bgValue : "#ffffff";
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
    <CollapsibleGroup title="背景" summary={summary} defaultOpen={false}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
          padding: "4px 0",
        }}
      >
        <HSVPanel color={hsvColor} onChange={handleColorChange} showHexInput />
      </div>
    </CollapsibleGroup>
  );
}
