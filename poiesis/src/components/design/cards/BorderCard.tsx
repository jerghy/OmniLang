// components/design/cards/BorderCard.tsx
// Border editing card with width, style, and color controls.

import React, { useState, useRef, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { PropertyRow } from "../PropertyRow";
import { ScrubInput } from "../ScrubInput";
import { ColorHUD } from "../ColorHUD";
import { LockIcon } from "./LockIcon";
import { isPropLocked, LOCKED_INPUT_STYLE } from "./lock-icon-utils";
import type { StyleProperty } from "../../../core/types/style";

const CLEAR_BTN: React.CSSProperties = {
  background: "none",
  border: "none",
  cursor: "pointer",
  color: "var(--dp-text-muted, var(--ol-gray-500))",
  fontSize: 14,
  padding: "0 4px",
  lineHeight: 1,
};
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils/id";

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
  onFocus?: () => void;
  inheritedProps?: string[];
  onToggleInherited?: (propName: string) => void;
  readOnly?: boolean;
}

const BORDER_STYLES = ["solid", "dashed", "dotted", "double", "none"];

// ── Styles ──

const selectStyle: React.CSSProperties = {
  width: "100%",
  height: 24,
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, var(--ol-border))",
  borderRadius: "var(--dp-radius-sm, 6px)",
  background: "var(--dp-surface, var(--ol-surface))",
  color: "var(--dp-text, var(--ol-gray-700))",
  outline: "2px solid transparent",
  cursor: "pointer",
  padding: "0 4px",
};

// ── Component ──

export const BorderCard = React.memo(function BorderCard({
  properties,
  tokens,
  onChange,
  onFocus,
  inheritedProps,
  onToggleInherited,
  readOnly,
}: BorderCardProps) {
  const [showColorPicker, setShowColorPicker] = useState(false);
  const swatchRef = useRef<HTMLButtonElement>(null);

  const borderWidth = getProp(properties, "border");
  const borderStyleFromProp = getProp(properties, "borderStyle");
  const borderStyle = borderStyleFromProp || "solid";
  const borderStyleSet = borderStyleFromProp !== "";
  const borderColorFromProp = getProp(properties, "borderColor");
  const borderColor = borderColorFromProp || "var(--ol-gray-950)";
  const borderColorSet = borderColorFromProp !== "";

  const borderColorProp = properties.find((p) => p.name === "borderColor");
  const isColorFromToken =
    borderColorProp?.segments[0]?.type === "var-ref";

  const widthNum = parseFloat(borderWidth) || 0;
  const borderWidthEmpty = borderWidth === "";

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

  const handleClearWidth = useCallback(() => {
    onChange(properties.filter((p) => p.name !== "border"));
  }, [properties, onChange]);

  const handleStyleChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange(setProp(properties, "borderStyle", e.target.value));
    },
    [properties, onChange],
  );

  const handleClearStyle = useCallback(() => {
    onChange(properties.filter((p) => p.name !== "borderStyle"));
  }, [properties, onChange]);

  const handleColorChange = useCallback(
    (hex: string, tokenName: string | null) => {
      onChange(setColorProp(properties, "borderColor", hex, tokenName));
      setShowColorPicker(false);
    },
    [properties, onChange],
  );

  const handleClearColor = useCallback(() => {
    onChange(properties.filter((p) => p.name !== "borderColor"));
  }, [properties, onChange]);

  const handleColorSwatchClick = useCallback(() => {
    setShowColorPicker((v) => !v);
  }, []);

  const handleColorHUDClose = useCallback(() => {
    setShowColorPicker(false);
  }, []);

  const swatchStyle: React.CSSProperties = {
    width: 24,
    height: 24,
    borderRadius: "var(--dp-radius-sm, 6px)",
    border: isColorFromToken
      ? "2px solid var(--dp-accent, var(--ol-color-primary))"
      : "1px solid var(--dp-border, var(--ol-border))",
    backgroundColor: isColorFromToken ? undefined : borderColor,
    cursor: "pointer",
    padding: 0,
    flexShrink: 0,
    position: "relative",
    transition: "box-shadow 150ms ease",
    boxShadow: "0 1px 2px rgba(0,0,0,0.06)",
  };

  return (
    <CollapsibleGroup title="边框" summary={summary} defaultOpen={true} onDoubleClick={() => onFocus?.()}>
      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        <PropertyRow label="粗细">
          <span style={isPropLocked("border", inheritedProps) ? LOCKED_INPUT_STYLE : (borderWidthEmpty ? { opacity: 0.4 } : {})}>
          {readOnly && inheritedProps?.includes("border") ? (
            <span style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))" }}>{borderWidth || "—"}</span>
          ) : (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
              <ScrubInput
                value={widthNum}
                unit="px"
                min={0}
                max={100}
                onChange={handleWidthChange}
              />
              {!borderWidthEmpty && (
                <button type="button" style={CLEAR_BTN} title="清除" onClick={(e) => { e.stopPropagation(); handleClearWidth(); }}>×</button>
              )}
            </span>
          )}
          </span>
          <LockIcon propName="border" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
        </PropertyRow>

        <PropertyRow label="样式">
          <span style={isPropLocked("borderStyle", inheritedProps) ? LOCKED_INPUT_STYLE : undefined}>
          {readOnly && inheritedProps?.includes("borderStyle") ? (
            <span style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))" }}>{borderStyle}</span>
          ) : (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
              <select value={borderStyle} onChange={handleStyleChange} style={selectStyle}>
                {BORDER_STYLES.map((s) => (<option key={s} value={s}>{s}</option>))}
              </select>
              {borderStyleSet && (
                <button type="button" style={CLEAR_BTN} title="清除" onClick={(e) => { e.stopPropagation(); handleClearStyle(); }}>×</button>
              )}
            </span>
          )}
          </span>
          <LockIcon propName="borderStyle" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
        </PropertyRow>

        <PropertyRow label="颜色">
          <span style={isPropLocked("borderColor", inheritedProps) ? LOCKED_INPUT_STYLE : (!borderColorSet ? { opacity: 0.4 } : {})}>
          {readOnly && inheritedProps?.includes("borderColor") ? (
            <span style={{ fontSize: 12, color: "var(--dp-text-muted, var(--ol-text-muted))" }}>{borderColor}</span>
          ) : (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
              <button ref={swatchRef} onClick={handleColorSwatchClick} type="button" style={swatchStyle}
                title={isColorFromToken ? `var(--${borderColor})` : borderColor}>
                {isColorFromToken && (<span style={{ fontSize: 12, color: "var(--dp-accent, var(--ol-color-primary))", lineHeight: "24px", display: "block", textAlign: "center", fontWeight: 700 }}>T</span>)}
              </button>
              {borderColorSet && (
                <button type="button" style={CLEAR_BTN} title="清除" onClick={(e) => { e.stopPropagation(); handleClearColor(); }}>×</button>
              )}
            </span>
          )}
          </span>
          <LockIcon propName="borderColor" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
        </PropertyRow>

        {/* Border preview strip */}
        <div
          style={{
            marginTop: 6,
            height: 20,
            borderRadius: "var(--dp-radius-sm, 6px)",
            borderStyle: borderStyle as React.CSSProperties["borderStyle"],
            borderColor: isColorFromToken
              ? "var(--dp-accent, var(--ol-color-primary))"
              : borderColor,
            borderWidth: Math.min(widthNum, 4) || 1,
            background: "var(--dp-bg, var(--ol-gray-50))",
            transition: "background 100ms ease",
          }}
        />
      </div>

      {showColorPicker && swatchRef.current && (
        <ColorHUD
          hex={isColorFromToken ? "var(--ol-color-primary)" : borderColor}
          tokens={tokens}
          onChange={handleColorChange}
          onClose={handleColorHUDClose}
          anchorRect={swatchRef.current.getBoundingClientRect()}
        />
      )}
    </CollapsibleGroup>
  );
});
