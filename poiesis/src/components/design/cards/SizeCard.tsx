// components/design/cards/SizeCard.tsx
// Size/dimensions editing card with W/H scrub inputs, linked ratio toggle, and mini preview.

import React, { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils/id";
import { LockIcon, isPropLocked, LOCKED_INPUT_STYLE } from "./LockIcon";

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

function isTokenRef(value: string): boolean {
  return value.startsWith("token(");
}

const tokenTextStyle: React.CSSProperties = {
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  color: "var(--dp-accent, #6366f1)",
  cursor: "pointer",
};

const tokenRemoveBtn: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  fontSize: 14,
  padding: "0 2px",
  color: "var(--dp-text-muted, #a09db8)",
  lineHeight: 1,
  flexShrink: 0,
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "4px 6px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, #e8e5f2)",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, #fff)",
  color: "var(--dp-text, #1e1b4b)",
  boxSizing: "border-box",
  flex: 1,
  minWidth: 0,
};

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
  onPropSlash?: (propName: string, el: HTMLElement) => void;
  inheritedProps?: string[];
  onToggleInherited?: (propName: string) => void;
  readOnly?: boolean;
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
  transition: "border-color 150ms ease, background-color 150ms ease, opacity 150ms ease, 150ms ease",
};

// ── Component ──

export const SizeCard = React.memo(function SizeCard({ properties, onChange, onFocus, onPropSlash, inheritedProps, onToggleInherited, readOnly }: SizeCardProps) {
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
            <span style={isPropLocked("width", inheritedProps) ? LOCKED_INPUT_STYLE : undefined}>
            {readOnly && inheritedProps?.includes("width") ? (
              <span style={{ ...INPUT_STYLE, color: "var(--dp-text-muted, #a09db8)", flex: 1, textAlign: "right" }}>{widthStr || "—"}</span>
            ) : isTokenRef(widthStr) ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                <span
                  style={tokenTextStyle}
                  onClick={(e) => onPropSlash?.("width", e.currentTarget)}
                >{widthStr}</span>
                <button type="button"
                  style={tokenRemoveBtn}
                  title="Remove token"
                  onClick={(e) => {
                    e.stopPropagation();
                    onChange(properties.filter((p) => p.name !== "width" && p.name !== "Width"));
                  }}
                >&times;</button>
              </span>
            ) : (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                <span style={widthStr === "" ? { opacity: 0.4 } : {}}>
                  <ScrubInput
                    value={width}
                    unit="px"
                    min={0}
                    max={4000}
                    onChange={handleWidthChange}
                    onSlash={onPropSlash ? (el) => onPropSlash("width", el) : undefined}
                  />
                </span>
                {widthStr !== "" && (
                  <button type="button"
                    style={tokenRemoveBtn}
                    title="清除"
                    onClick={(e) => {
                      e.stopPropagation();
                      onChange(properties.filter((p) => p.name !== "width" && p.name !== "Width"));
                    }}
                  >&times;</button>
                )}
              </span>
            )}
            </span>
            <LockIcon propName="width" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
          </div>

          <button type="button"
            onClick={() => setLinked((v) => !v)}
            title={linked ? "Unlink dimensions" : "Link dimensions"}
            style={linkBtnStyle}
          >
            {linked ? "🔗" : "🔓"}
          </button>

          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
            <span style={labelStyle}>H</span>
            <span style={isPropLocked("height", inheritedProps) ? LOCKED_INPUT_STYLE : undefined}>
            {readOnly && inheritedProps?.includes("height") ? (
              <span style={{ ...INPUT_STYLE, color: "var(--dp-text-muted, #a09db8)", flex: 1, textAlign: "right" }}>{heightStr || "—"}</span>
            ) : isTokenRef(heightStr) ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                <span
                  style={tokenTextStyle}
                  onClick={(e) => onPropSlash?.("height", e.currentTarget)}
                >{heightStr}</span>
                <button type="button"
                  style={tokenRemoveBtn}
                  title="Remove token"
                  onClick={(e) => {
                    e.stopPropagation();
                    onChange(properties.filter((p) => p.name !== "height" && p.name !== "Height"));
                  }}
                >&times;</button>
              </span>
            ) : (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                <span style={heightStr === "" ? { opacity: 0.4 } : {}}>
                  <ScrubInput
                    value={height}
                    unit="px"
                    min={0}
                    max={4000}
                    onChange={handleHeightChange}
                    onSlash={onPropSlash ? (el) => onPropSlash("height", el) : undefined}
                  />
                </span>
                {heightStr !== "" && (
                  <button type="button"
                    style={tokenRemoveBtn}
                    title="清除"
                    onClick={(e) => {
                      e.stopPropagation();
                      onChange(properties.filter((p) => p.name !== "height" && p.name !== "Height"));
                    }}
                  >&times;</button>
                )}
              </span>
            )}
            </span>
            <LockIcon propName="height" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
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
                transition: "none",
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
