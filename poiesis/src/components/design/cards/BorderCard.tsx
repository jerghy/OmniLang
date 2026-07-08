// components/design/cards/BorderCard.tsx
// Border editing card with width, style, and color controls.

import { useState, useRef, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { PropertyRow } from "../PropertyRow";
import { ScrubInput } from "../ScrubInput";
import { ColorHUD } from "../ColorHUD";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils";

// ── Helpers ──

function getProp(props: StyleProperty[], name: string): string {
  const prop = props.find((p) => p.name === name);
  if (!prop || prop.segments.length === 0) return "";
  const seg = prop.segments[0];
  if (seg.type === "var-ref") return seg.name;
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

function setColorProp(
  props: StyleProperty[],
  name: string,
  hex: string,
  tokenName: string | null,
): StyleProperty[] {
  const segments = tokenName
    ? [{ type: "var-ref" as const, name: tokenName }]
    : [{ type: "text" as const, value: hex }];
  const existing = props.find((p) => p.name === name);
  if (existing) {
    return props.map((p) => (p.name === name ? { ...p, segments } : p));
  }
  return [...props, { id: uid("prop"), name, segments }];
}

// ── Props ──

interface BorderCardProps {
  properties: StyleProperty[];
  tokens: DesignToken[];
  onChange: (props: StyleProperty[]) => void;
}

const BORDER_STYLES = ["solid", "dashed", "dotted", "double", "none"];

// ── Component ──

export function BorderCard({
  properties,
  tokens,
  onChange,
}: BorderCardProps) {
  const [showColorPicker, setShowColorPicker] = useState(false);
  const swatchRef = useRef<HTMLButtonElement>(null);

  const borderWidth = getProp(properties, "border");
  const borderStyle = getProp(properties, "borderStyle") || "solid";
  const borderColor = getProp(properties, "borderColor") || "#000000";

  // Detect if the border color value came from a token
  const borderColorProp = properties.find((p) => p.name === "borderColor");
  const isColorFromToken =
    borderColorProp?.segments[0]?.type === "var-ref";

  const widthNum = parseFloat(borderWidth) || 0;

  const summary = borderWidth
    ? `${borderWidth} ${borderStyle} ${
        isColorFromToken ? `var(--${borderColor})` : borderColor
      }`
    : "—";

  const handleWidthChange = useCallback(
    (v: number) => {
      onChange(setProp(properties, "border", `${v}px`));
    },
    [properties, onChange],
  );

  const handleStyleChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange(setProp(properties, "borderStyle", e.target.value));
    },
    [properties, onChange],
  );

  const handleColorChange = useCallback(
    (hex: string, tokenName: string | null) => {
      onChange(setColorProp(properties, "borderColor", hex, tokenName));
      setShowColorPicker(false);
    },
    [properties, onChange],
  );

  const handleColorSwatchClick = useCallback(() => {
    setShowColorPicker((v) => !v);
  }, []);

  const handleColorHUDClose = useCallback(() => {
    setShowColorPicker(false);
  }, []);

  return (
    <CollapsibleGroup title="边框" summary={summary} defaultOpen={true}>
      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        {/* 粗细 row */}
        <PropertyRow label="粗细">
          <ScrubInput
            value={widthNum}
            unit="px"
            min={0}
            max={100}
            onChange={handleWidthChange}
          />
        </PropertyRow>

        {/* 样式 row */}
        <PropertyRow label="样式">
          <select
            value={borderStyle}
            onChange={handleStyleChange}
            style={{
              width: "100%",
              height: 22,
              fontSize: 11,
              border: "1px solid #d0d0d0",
              borderRadius: 3,
              background: "#fff",
              fontFamily: "inherit",
            }}
          >
            {BORDER_STYLES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </PropertyRow>

        {/* 颜色 row */}
        <PropertyRow label="颜色">
          <button
            ref={swatchRef}
            onClick={handleColorSwatchClick}
            type="button"
            style={{
              width: 24,
              height: 24,
              borderRadius: 3,
              border: isColorFromToken
                ? "2px solid #6366f1"
                : "1px solid rgba(0,0,0,0.15)",
              backgroundColor: isColorFromToken ? undefined : borderColor,
              cursor: "pointer",
              padding: 0,
              flexShrink: 0,
              position: "relative",
            }}
            title={
              isColorFromToken ? `var(--${borderColor})` : borderColor
            }
          >
            {isColorFromToken && (
              <span
                style={{
                  fontSize: 10,
                  color: "#6366f1",
                  lineHeight: "24px",
                  display: "block",
                  textAlign: "center",
                }}
              >
                T
              </span>
            )}
          </button>
        </PropertyRow>
      </div>

      {showColorPicker && swatchRef.current && (
        <ColorHUD
          hex={isColorFromToken ? "#6366f1" : borderColor}
          tokens={tokens}
          onChange={handleColorChange}
          onClose={handleColorHUDClose}
          anchorRect={swatchRef.current.getBoundingClientRect()}
        />
      )}
    </CollapsibleGroup>
  );
}
