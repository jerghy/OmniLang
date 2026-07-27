// components/design/cards/BackgroundCard.tsx
// Background color editing card with HSV panel for inline color picking.

import React, { useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { HSVPanel } from "../HSVPanel";
import { hexToHsv, hsvToHex } from "../hsv-utils";
import type { HSVColor } from "../hsv-utils";
import { LockIcon } from "./LockIcon";
import { isPropLocked, LOCKED_INPUT_STYLE } from "./lock-icon-utils";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils/id";

const CLEAR_BTN: React.CSSProperties = {
  background: "none",
  border: "none",
  cursor: "pointer",
  color: "var(--dp-text-muted, var(--ol-gray-500))",
  fontSize: 14,
  padding: "0 4px",
  lineHeight: 1,
};

// ── Helpers ──

function getProp(props: StyleProperty[], name: string): string {
  let prop = props.find((p) => p.name === name);
  if (!prop) {
    const camel = name.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase());
    prop = props.find((p) => p.name === camel);
  }
  if (!prop || prop.segments.length === 0) return "";
  return prop.segments.map((s) => {
    if (s.type === "var-ref") return `token(${s.name})`;
    if (s.type === "text") return s.value;
    return "";
  }).join("");
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
  inheritedProps?: string[];
  onToggleInherited?: (propName: string) => void;
  readOnly?: boolean;
}

// ── Component ──

export const BackgroundCard = React.memo(function BackgroundCard({
  properties,
  tokens,
  onChange,
  onFocus,
  inheritedProps,
  onToggleInherited,
  readOnly,
}: BackgroundCardProps) {
  const bgValue = getProp(properties, "background");

  const isHex = /^#[0-9a-fA-F]{6}$/.test(bgValue);
  const summary = bgValue || "none";

  const hex = isHex ? bgValue : "var(--ol-surface)";
  const hsv = hexToHsv(hex);
  const hsvColor: HSVColor = { h: hsv.h, s: hsv.s, v: hsv.v, a: 1 };

  const handleColorChange = useCallback(
    (c: HSVColor) => {
      const newHex = hsvToHex(c.h, c.s, c.v);
      onChange(setProp(properties, "background", newHex));
    },
    [properties, onChange],
  );

  const isBgInherited = readOnly && inheritedProps?.includes("background");
  const isEmpty = bgValue === "";

  const handleClear = () => onChange(properties.filter((p) => p.name !== "background"));

  return (
    <CollapsibleGroup title="背景" summary={summary} defaultOpen={false} onDoubleClick={() => onFocus?.()}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
        }}
      >
        <LockIcon propName="background" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
        <span style={isPropLocked("background", inheritedProps) ? LOCKED_INPUT_STYLE : (isEmpty ? { opacity: 0.4 } : {})}>
        {isBgInherited ? null : (<>
        {/* Current color indicator + clear button */}
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
                border: "1px solid var(--dp-border, var(--ol-border))",
                flexShrink: 0,
              }}
            />
            <span
              style={{
                fontSize: 12,
                fontFamily: "var(--dp-font-mono, monospace)",
                color: "var(--dp-text, var(--ol-gray-700))",
              }}
            >
              {bgValue}
            </span>
            <button type="button" style={CLEAR_BTN} title="清除" onClick={(e) => { e.stopPropagation(); handleClear(); }}>×</button>
          </div>
        )}

        <HSVPanel color={hsvColor} onChange={handleColorChange} showHexInput />
        </>)}
        </span>
      </div>
    </CollapsibleGroup>
  );
});
