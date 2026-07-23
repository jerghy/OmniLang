// components/panels/inspector/TypographyBasics.tsx
// Lightweight typography card for the inline style panel.
// Only fontSize (scrub + presets), fontWeight (select), and color (picker).
// Designed for frequent tweaking with instant canvas feedback.

import React, { useState, useRef, useCallback } from "react";
import { CollapsibleGroup } from "../../design/CollapsibleGroup";
import { ScrubInput } from "../../design/ScrubInput";
import { ColorHUD } from "../../design/ColorHUD";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils/id";
import styles from "./TypographyBasics.module.css";

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
  if (value === "") {
    return props.filter((p) => p.name !== name);
  }
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
    { id: uid("prop"), name, segments: [{ type: "text" as const, value }] },
  ];
}

function parsePx(value: string): number {
  const num = parseFloat(value);
  return isNaN(num) ? 0 : num;
}

function isTokenRef(value: string): boolean {
  return value.startsWith("token(");
}

// ── Constants ──

const FONT_SIZE_PRESETS = [12, 14, 16, 18, 24, 32, 48];

const FONT_WEIGHTS: { label: string; value: string }[] = [
  { label: "Thin (100)", value: "100" },
  { label: "Light (300)", value: "300" },
  { label: "Normal (400)", value: "400" },
  { label: "Medium (500)", value: "500" },
  { label: "Semibold (600)", value: "600" },
  { label: "Bold (700)", value: "700" },
  { label: "Black (900)", value: "900" },
];

// ── Styles ──

const selectStyle: React.CSSProperties = {
  width: "100%",
  padding: "4px 6px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, #fff)",
  color: "var(--dp-text, #1e1b4b)",
};

const labelStyle: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, #6b6893)",
  minWidth: 56,
  flexShrink: 0,
};

const presetChipBase: React.CSSProperties = {
  padding: "2px 8px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  background: "transparent",
  cursor: "pointer",
  color: "var(--dp-text-secondary, #6b6893)",
};

// ── Props ──

interface TypographyBasicsProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
}

export function TypographyBasics({ properties, onChange }: TypographyBasicsProps) {
  const swatchRef = useRef<HTMLButtonElement>(null);
  const [showColorPicker, setShowColorPicker] = useState(false);

  const fontSizeRaw = getProp(properties, "fontSize");
  const fontSize = parsePx(fontSizeRaw);
  const fontWeightRaw = getProp(properties, "fontWeight");
  const colorRaw = getProp(properties, "color");
  const colorIsToken = isTokenRef(colorRaw);

  const handleFontSizeChange = useCallback(
    (v: number) => onChange(setProp(properties, "fontSize", `${v}px`)),
    [properties, onChange],
  );

  const handleWeightChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      onChange(setProp(properties, "fontWeight", e.target.value));
    },
    [properties, onChange],
  );

  const handleColorChange = useCallback(
    (hex: string, _tokenName: string | null) => {
      onChange(setProp(properties, "color", hex));
      setShowColorPicker(false);
    },
    [properties, onChange],
  );

  const hasAny = fontSize > 0 || fontWeightRaw !== "" || colorRaw !== "";
  const summary = hasAny
    ? [fontSizeRaw ? `${fontSizeRaw}` : "", fontWeightRaw || "", colorRaw || ""]
        .filter(Boolean)
        .join(" · ")
    : "—";

  const swatchStyle: React.CSSProperties = {
    width: 24,
    height: 24,
    borderRadius: "var(--dp-radius-sm, 6px)",
    border: colorIsToken
      ? "2px solid var(--dp-accent, #6366f1)"
      : "1px solid var(--dp-border, #e8e5f2)",
    backgroundColor: colorIsToken ? undefined : colorRaw || "transparent",
    cursor: "pointer",
    padding: 0,
    flexShrink: 0,
    boxShadow: "0 1px 2px rgba(0,0,0,0.06)",
  };

  return (
    <CollapsibleGroup title="文字" summary={summary} defaultOpen={hasAny}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {/* Font size: scrub + preset chips */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={labelStyle}>字号</span>
          <div style={{ flex: 1, display: "flex", alignItems: "center", gap: 4 }}>
            <span style={fontSizeRaw === "" ? { opacity: 0.4, flex: 1 } : { flex: 1 }}>
              <ScrubInput
                value={fontSize}
                unit="px"
                min={8}
                max={200}
                onChange={handleFontSizeChange}
              />
            </span>
            {fontSizeRaw !== "" && (
              <button type="button"
                className={styles.clearBtn}
                title="清除"
                onClick={(e) => { e.stopPropagation(); onChange(properties.filter((p) => p.name !== "fontSize")); }}
              >×</button>
            )}
          </div>
        </div>
        <div style={{ display: "flex", gap: 4, flexWrap: "wrap", paddingLeft: 64 }}>
          {FONT_SIZE_PRESETS.map((size) => (
            <button type="button"
              key={size}
              onClick={() => onChange(setProp(properties, "fontSize", `${size}px`))}
              style={{
                ...presetChipBase,
                background: fontSize === size ? "var(--dp-accent-subtle, #eef0ff)" : "transparent",
                borderColor: fontSize === size ? "var(--dp-accent, #6366f1)" : "var(--dp-border, #e8e5f2)",
                color: fontSize === size ? "var(--dp-accent, #6366f1)" : "var(--dp-text-secondary, #6b6893)",
              }}
            >
              {size}
            </button>
          ))}
        </div>

        {/* Font weight: select */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={labelStyle}>字重</span>
          <select
            aria-label="Font weight"
            value={fontWeightRaw || "400"}
            onChange={handleWeightChange}
            style={selectStyle}
          >
            <option value="">默认</option>
            {FONT_WEIGHTS.map((w) => (
              <option key={w.value} value={w.value}>
                {w.label}
              </option>
            ))}
          </select>
        </div>

        {/* Color: swatch + ColorHUD */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={labelStyle}>颜色</span>
          <span style={colorRaw === "" ? { opacity: 0.4 } : {}}>
            <button
              ref={swatchRef}
              onClick={() => setShowColorPicker((v) => !v)}
              type="button"
              style={swatchStyle}
              title={colorRaw || "选择颜色"}
            >
              {colorIsToken && (
                <span
                  style={{
                    fontSize: 12,
                    color: "var(--dp-accent, #6366f1)",
                    lineHeight: "24px",
                    display: "block",
                    textAlign: "center",
                    fontWeight: 700,
                  }}
                >
                  T
                </span>
              )}
            </button>
          </span>
          <span
            style={{
              fontSize: 12,
              fontFamily: "var(--dp-font-mono, monospace)",
              color: "var(--dp-text-muted, #a09db8)",
            }}
          >
            {colorRaw || "—"}
          </span>
          {colorRaw !== "" && (
            <button type="button"
              className={styles.clearBtn}
              title="清除"
              onClick={(e) => { e.stopPropagation(); onChange(properties.filter((p) => p.name !== "color")); }}
            >×</button>
          )}
        </div>

        {/* Color picker popup */}
        {showColorPicker && swatchRef.current && (
          <ColorHUD
            hex={colorIsToken ? "#6366f1" : colorRaw || "#000000"}
            tokens={[] as DesignToken[]}
            onChange={handleColorChange}
            onClose={() => setShowColorPicker(false)}
            anchorRect={swatchRef.current.getBoundingClientRect()}
          />
        )}

        {/* Mini text preview */}
        {hasAny && (
          <div
            style={{
              padding: "8px 12px",
              borderRadius: "var(--dp-radius-sm, 6px)",
              background: "var(--dp-bg, #f7f6fa)",
              border: "1px dashed var(--dp-border, #e8e5f2)",
              textAlign: "center",
            }}
          >
            <span
              style={{
                fontSize: fontSizeRaw || "14px",
                fontWeight: (fontWeightRaw || "400") as React.CSSProperties["fontWeight"],
                color: colorRaw || "var(--dp-text, #1e1b4b)",
                fontFamily: "var(--dp-font-mono, monospace)",
              }}
            >
              Aa 字体预览
            </span>
          </div>
        )}
      </div>
    </CollapsibleGroup>
  );
}
