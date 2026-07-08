// components/design/cards/SizeCard.tsx
// Size/dimensions editing card with W/H scrub inputs, linked ratio toggle, and mini preview.

import { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
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

interface SizeCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
  onFocus?: () => void;
}

// ── Component ──

export function SizeCard({ properties, onChange, onFocus }: SizeCardProps) {
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

  // Mini preview: scale dimensions by 1/8, clamped to 200x60
  const previewW = Math.min(Math.max(width / 8, 4), 200);
  const previewH = Math.min(Math.max(height / 8, 4), 60);

  return (
    <CollapsibleGroup title="尺寸" summary={summary} defaultOpen={true}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {/* W / H scrub row */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {/* Width */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              flex: 1,
            }}
          >
            <span
              style={{
                fontSize: 11,
                color: "#64748b",
                width: 14,
                flexShrink: 0,
              }}
            >
              W
            </span>
            <ScrubInput
              value={width}
              unit="px"
              min={0}
              max={4000}
              onChange={handleWidthChange}
            />
          </div>

          {/* Linked ratio button */}
          <button
            onClick={() => setLinked((v) => !v)}
            title={linked ? "Unlink dimensions" : "Link dimensions"}
            style={{
              border: linked ? "1px solid #6366f1" : "1px solid #d1d5db",
              borderRadius: 4,
              background: linked ? "#eef2ff" : "transparent",
              color: linked ? "#6366f1" : "#94a3b8",
              cursor: "pointer",
              fontSize: 14,
              lineHeight: 1,
              padding: "2px 5px",
              fontFamily: "inherit",
              flexShrink: 0,
            }}
          >
            {"🔗"}
          </button>

          {/* Height */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              flex: 1,
            }}
          >
            <span
              style={{
                fontSize: 11,
                color: "#64748b",
                width: 14,
                flexShrink: 0,
              }}
            >
              H
            </span>
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
            onDoubleClick={() => onFocus?.()}
            style={{
              width: "100%",
              height: 60,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              cursor: "pointer",
              borderRadius: 4,
              background: "#f8fafc",
            }}
          >
            <div
              style={{
                width: previewW,
                height: previewH,
                border: "2px solid #6366f1",
                borderRadius: 2,
                background: "rgba(99, 102, 241, 0.06)",
                transition: "width 100ms, height 100ms",
              }}
            />
          </div>
        )}
      </div>
    </CollapsibleGroup>
  );
}
