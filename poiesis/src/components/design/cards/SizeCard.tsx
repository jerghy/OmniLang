// components/design/cards/SizeCard.tsx
// Size/dimensions editing card with W/H scrub inputs, linked ratio toggle, and mini preview.

import React, { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils";

// ── Helpers ──

function getProp(props: StyleProperty[], name: string): string {
  const prop = props.find((p) => p.name === name);
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

interface SizeCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
  onFocus?: () => void;
}

// ── Styles ──

const labelStyle: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, #6b6893)",
  width: 14,
  flexShrink: 0,
};

const linkBtnBase: React.CSSProperties = {
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  background: "transparent",
  cursor: "pointer",
  fontSize: 13,
  lineHeight: 1,
  padding: "3px 5px",
  fontFamily: "inherit",
  flexShrink: 0,
  transition: "all 150ms ease",
};

// ── Component ──

export const SizeCard = React.memo(function SizeCard({ properties, onChange, onFocus }: SizeCardProps) {
  const [linked, setLinked] = useState(false);

  const widthStr = getProp(properties, "width");
  const heightStr = getProp(properties, "height");
  const width = parseFloat(widthStr) || 0;
  const height = parseFloat(heightStr) || 0;

  const handleWidthChange = useCallback(
    (v: number) => {
      let newProps = setProp(properties, "width", `${v}px`);
      if (linked && height > 0 && width > 0) {
        const ratio = width / height;
        const newH = Math.round(v / ratio);
        newProps = setProp(newProps, "height", `${newH}px`);
      }
      onChange(newProps);
    },
    [properties, onChange, linked, width, height],
  );

  const handleHeightChange = useCallback(
    (v: number) => {
      let newProps = setProp(properties, "height", `${v}px`);
      if (linked && width > 0 && height > 0) {
        const ratio = width / height;
        const newW = Math.round(v * ratio);
        newProps = setProp(newProps, "width", `${newW}px`);
      }
      onChange(newProps);
    },
    [properties, onChange, linked, width, height],
  );

  const hasSize = width > 0 || height > 0;
  const summary = hasSize
    ? `${widthStr || "auto"} × ${heightStr || "auto"}`
    : "—";

  const previewW = Math.min(Math.max(width / 8, 4), 200);
  const previewH = Math.min(Math.max(height / 8, 4), 60);

  const linkBtnStyle: React.CSSProperties = {
    ...linkBtnBase,
    borderColor: linked ? "var(--dp-accent, #6366f1)" : "var(--dp-border, #e8e5f2)",
    background: linked ? "var(--dp-accent-subtle, #eef0ff)" : "transparent",
    color: linked ? "var(--dp-accent, #6366f1)" : "var(--dp-text-muted, #a09db8)",
  };

  return (
    <CollapsibleGroup title="尺寸" summary={summary} defaultOpen={true} onDoubleClick={() => onFocus?.()}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {/* W / H scrub row */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
            <span style={labelStyle}>W</span>
            <ScrubInput
              value={width}
              unit="px"
              min={0}
              max={4000}
              onChange={handleWidthChange}
            />
          </div>

          <button
            onClick={() => setLinked((v) => !v)}
            title={linked ? "Unlink dimensions" : "Link dimensions"}
            style={linkBtnStyle}
          >
            {linked ? "🔗" : "🔓"}
          </button>

          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
            <span style={labelStyle}>H</span>
            <ScrubInput
              value={height}
              unit="px"
              min={0}
              max={4000}
              onChange={handleHeightChange}
            />
          </div>
        </div>

        {/* Mini preview */}
        {hasSize && (
          <div
            style={{
              width: "100%",
              height: 60,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              borderRadius: "var(--dp-radius-sm, 6px)",
              background: "var(--dp-bg, #f7f6fa)",
              border: "1px dashed var(--dp-border, #e8e5f2)",
            }}
          >
            <div
              style={{
                width: previewW,
                height: previewH,
                border: "2px solid var(--dp-accent, #6366f1)",
                borderRadius: 2,
                background: "rgba(99, 102, 241, 0.06)",
                transition: "width 100ms, height 100ms",
                position: "relative",
              }}
            >
              {/* Dimension labels */}
              {previewW > 30 && (
                <span
                  style={{
                    position: "absolute",
                    top: "50%",
                    left: "50%",
                    transform: "translate(-50%, -50%)",
                    fontSize: 8,
                    color: "var(--dp-text-muted, #a09db8)",
                    fontFamily: "var(--dp-font-mono, monospace)",
                    whiteSpace: "nowrap",
                  }}
                >
                  {`${widthStr} × ${heightStr}`}
                </span>
              )}
            </div>
          </div>
        )}
      </div>
    </CollapsibleGroup>
  );
});
