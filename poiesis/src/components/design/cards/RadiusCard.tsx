// components/design/cards/RadiusCard.tsx
// Border-radius editing card with unified/individual modes and mini preview.

import { useState, useMemo, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils";

// ── Helpers ──

const CORNER_NAMES = [
  "borderTopLeftRadius",
  "borderTopRightRadius",
  "borderBottomRightRadius",
  "borderBottomLeftRadius",
] as const;

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
  // When value is empty, remove the property entirely
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
    {
      id: uid("prop"),
      name,
      segments: [{ type: "text", value }],
    },
  ];
}

function parsePx(value: string): number {
  const num = parseFloat(value);
  return isNaN(num) ? 0 : num;
}

// ── Props ──

interface RadiusCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
}

// ── Component ──

export function RadiusCard({ properties, onChange }: RadiusCardProps) {
  // Determine initial mode based on whether individual corner properties exist
  const hasCornerProps = CORNER_NAMES.some(
    (n) => getProp(properties, n) !== "",
  );
  const [mode, setMode] = useState<"unified" | "individual">(
    hasCornerProps ? "individual" : "unified",
  );

  const radiusValue = parsePx(getProp(properties, "borderRadius"));

  const cornerValues = useMemo(
    () => CORNER_NAMES.map((n) => parsePx(getProp(properties, n))),
    [properties],
  );

  const [tl, tr, br, bl] = cornerValues;

  // Summary
  const summary = useMemo(() => {
    if (mode === "unified") {
      return radiusValue > 0 ? `${radiusValue}px` : "—";
    }
    const vals = cornerValues.filter((v) => v > 0);
    return vals.length > 0
      ? cornerValues.map((v) => `${v}px`).join(" ")
      : "—";
  }, [mode, radiusValue, cornerValues]);

  // Preview border-radius value
  const previewRadius = useMemo(() => {
    if (mode === "unified") {
      return `${radiusValue}px`;
    }
    return cornerValues.map((v) => `${v}px`).join(" ");
  }, [mode, radiusValue, cornerValues]);

  // Toggle between unified and individual mode
  const toggleMode = useCallback(() => {
    if (mode === "unified") {
      // Unified → Individual: copy unified value to all 4 corners
      const value = radiusValue > 0 ? `${radiusValue}px` : "";
      let newProps = properties;
      for (const name of CORNER_NAMES) {
        newProps = setProp(newProps, name, value);
      }
      // Remove the unified property
      newProps = setProp(newProps, "borderRadius", "");
      onChange(newProps);
      setMode("individual");
    } else {
      // Individual → Unified: take average of the 4 corners
      const avg = Math.round((tl + tr + br + bl) / 4);
      let newProps = properties;
      newProps = setProp(
        newProps,
        "borderRadius",
        avg > 0 ? `${avg}px` : "",
      );
      for (const name of CORNER_NAMES) {
        newProps = setProp(newProps, name, "");
      }
      onChange(newProps);
      setMode("unified");
    }
  }, [mode, properties, onChange, radiusValue, tl, tr, br, bl]);

  const handleUnifiedChange = useCallback(
    (v: number) => {
      const newProps = setProp(properties, "borderRadius", `${v}px`);
      onChange(newProps);
    },
    [properties, onChange],
  );

  const handleCornerChange = useCallback(
    (index: number, v: number) => {
      const name = CORNER_NAMES[index];
      const newProps = setProp(properties, name, `${v}px`);
      onChange(newProps);
    },
    [properties, onChange],
  );

  const labelStyle: React.CSSProperties = {
    fontSize: 11,
    color: "#64748b",
    width: 14,
    flexShrink: 0,
    textAlign: "center",
  };

  return (
    <CollapsibleGroup title="圆角" summary={summary} defaultOpen={true}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {mode === "unified" ? (
          /* Unified mode: single borderRadius ScrubInput + toggle button */
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                flex: 1,
              }}
            >
              <span style={labelStyle}>R</span>
              <ScrubInput
                value={radiusValue}
                unit="px"
                min={0}
                max={200}
                onChange={handleUnifiedChange}
              />
            </div>
            <button
              onClick={toggleMode}
              title="Switch to individual corners"
              style={{
                border: "1px solid #6366f1",
                borderRadius: 4,
                background: "#eef2ff",
                color: "#6366f1",
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
          </div>
        ) : (
          /* Individual mode: 2x2 grid of corner inputs + toggle button */
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {/* Top row: TL, TR */}
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <CornerInput
                label="↖"
                value={cornerValues[0]}
                onChange={(v) => handleCornerChange(0, v)}
                labelStyle={labelStyle}
              />
              <CornerInput
                label="↗"
                value={cornerValues[1]}
                onChange={(v) => handleCornerChange(1, v)}
                labelStyle={labelStyle}
              />
              <button
                onClick={toggleMode}
                title="Switch to unified radius"
                style={{
                  border: "1px solid #d1d5db",
                  borderRadius: 4,
                  background: "transparent",
                  color: "#94a3b8",
                  cursor: "pointer",
                  fontSize: 14,
                  lineHeight: 1,
                  padding: "2px 5px",
                  fontFamily: "inherit",
                  flexShrink: 0,
                }}
              >
                {"🔓"}
              </button>
            </div>
            {/* Bottom row: BL, BR */}
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <CornerInput
                label="↙"
                value={cornerValues[3]}
                onChange={(v) => handleCornerChange(3, v)}
                labelStyle={labelStyle}
              />
              <CornerInput
                label="↘"
                value={cornerValues[2]}
                onChange={(v) => handleCornerChange(2, v)}
                labelStyle={labelStyle}
              />
              {/* Spacer to align with toggle button above */}
              <div style={{ width: 30, flexShrink: 0 }} />
            </div>
          </div>
        )}

        {/* Mini preview */}
        <div
          style={{
            width: "100%",
            height: 50,
            borderRadius: previewRadius,
            border: "2px solid #6366f1",
            background: "rgba(99, 102, 241, 0.06)",
            transition: "border-radius 100ms",
          }}
        />
      </div>
    </CollapsibleGroup>
  );
}

// ── Sub-component: CornerInput ──

interface CornerInputProps {
  label: string;
  value: number;
  onChange: (v: number) => void;
  labelStyle: React.CSSProperties;
}

function CornerInput({ label, value, onChange, labelStyle }: CornerInputProps) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 4,
        flex: 1,
      }}
    >
      <span style={labelStyle}>{label}</span>
      <ScrubInput
        value={value}
        unit="px"
        min={0}
        max={200}
        onChange={onChange}
      />
    </div>
  );
}
