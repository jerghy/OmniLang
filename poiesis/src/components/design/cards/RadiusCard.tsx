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
  onFocus?: () => void;
}

// ── Component ──

export function RadiusCard({ properties, onChange, onFocus }: RadiusCardProps) {
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

  const summary = useMemo(() => {
    if (mode === "unified") {
      return radiusValue > 0 ? `${radiusValue}px` : "—";
    }
    const vals = cornerValues.filter((v) => v > 0);
    return vals.length > 0
      ? cornerValues.map((v) => `${v}px`).join(" ")
      : "—";
  }, [mode, radiusValue, cornerValues]);

  const previewRadius = useMemo(() => {
    if (mode === "unified") {
      return `${radiusValue}px`;
    }
    return cornerValues.map((v) => `${v}px`).join(" ");
  }, [mode, radiusValue, cornerValues]);

  const toggleMode = useCallback(() => {
    if (mode === "unified") {
      const value = radiusValue > 0 ? `${radiusValue}px` : "";
      let newProps = properties;
      for (const name of CORNER_NAMES) {
        newProps = setProp(newProps, name, value);
      }
      newProps = setProp(newProps, "borderRadius", "");
      onChange(newProps);
      setMode("individual");
    } else {
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
    fontSize: 12,
    color: "var(--dp-text-secondary, #6b6893)",
    width: 16,
    flexShrink: 0,
    textAlign: "center",
  };

  const toggleBtnBase: React.CSSProperties = {
    borderRadius: "var(--dp-radius-sm, 6px)",
    cursor: "pointer",
    fontSize: 13,
    lineHeight: 1,
    padding: "3px 5px",
    fontFamily: "inherit",
    flexShrink: 0,
    transition: "all 150ms ease",
  };

  const linkedBtnStyle: React.CSSProperties = {
    ...toggleBtnBase,
    border: "1px solid var(--dp-accent, #6366f1)",
    background: "var(--dp-accent-subtle, #eef0ff)",
    color: "var(--dp-accent, #6366f1)",
  };

  const unlinkedBtnStyle: React.CSSProperties = {
    ...toggleBtnBase,
    border: "1px solid var(--dp-border, #e8e5f2)",
    background: "transparent",
    color: "var(--dp-text-muted, #a09db8)",
  };

  return (
    <CollapsibleGroup title="圆角" summary={summary} defaultOpen={true}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {mode === "unified" ? (
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
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
              style={linkedBtnStyle}
            >
              {"🔗"}
            </button>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <CornerInput
                label="↖"
                value={cornerValues[0]}
                onChange={(v) => handleCornerChange(0, v)}
              />
              <CornerInput
                label="↗"
                value={cornerValues[1]}
                onChange={(v) => handleCornerChange(1, v)}
              />
              <button
                onClick={toggleMode}
                title="Switch to unified radius"
                style={unlinkedBtnStyle}
              >
                {"🔓"}
              </button>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <CornerInput
                label="↙"
                value={cornerValues[3]}
                onChange={(v) => handleCornerChange(3, v)}
              />
              <CornerInput
                label="↘"
                value={cornerValues[2]}
                onChange={(v) => handleCornerChange(2, v)}
              />
              <div style={{ width: 30, flexShrink: 0 }} />
            </div>
          </div>
        )}

        {/* Mini preview — right-click for focus mode */}
        <div
          onContextMenu={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onFocus?.();
          }}
          style={{
            width: "100%",
            height: 48,
            borderRadius: previewRadius,
            border: "2px solid var(--dp-accent, #6366f1)",
            background: "rgba(99, 102, 241, 0.06)",
            transition: "border-radius 150ms ease",
            cursor: onFocus ? "context-menu" : "default",
          }}
          title={onFocus ? "Right-click for focus mode" : undefined}
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
}

function CornerInput({ label, value, onChange }: CornerInputProps) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
      <span
        style={{
          fontSize: 12,
          color: "var(--dp-text-secondary, #6b6893)",
          width: 16,
          flexShrink: 0,
          textAlign: "center",
        }}
      >
        {label}
      </span>
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
