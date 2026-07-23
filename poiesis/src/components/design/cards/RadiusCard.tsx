// components/design/cards/RadiusCard.tsx
// Border-radius editing card with unified/individual modes and mini preview.

import React, { useState, useMemo, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils";
import { LockIcon, isPropLocked, LOCKED_INPUT_STYLE } from "./LockIcon";

// ── Helpers ──

const CORNER_NAMES = [
  "borderTopLeftRadius",
  "borderTopRightRadius",
  "borderBottomRightRadius",
  "borderBottomLeftRadius",
] as const;

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
  onPropSlash?: (propName: string, el: HTMLElement) => void;
  inheritedProps?: string[];
  onToggleInherited?: (propName: string) => void;
  readOnly?: boolean;
}

// ── Component ──

export const RadiusCard = React.memo(function RadiusCard({ properties, onChange, onFocus, onPropSlash, inheritedProps, onToggleInherited, readOnly }: RadiusCardProps) {
  const hasCornerProps = CORNER_NAMES.some(
    (n) => getProp(properties, n) !== "",
  );
  const [mode, setMode] = useState<"unified" | "individual">(
    hasCornerProps ? "individual" : "unified",
  );

  const radiusRaw = getProp(properties, "borderRadius");
  const isRadiusToken = isTokenRef(radiusRaw);
  const radiusValue = isRadiusToken ? 0 : parsePx(radiusRaw);

  const cornerValues = useMemo(
    () => CORNER_NAMES.map((n) => {
      const raw = getProp(properties, n);
      return isTokenRef(raw) ? 0 : parsePx(raw);
    }),
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
    transition: "border-color 150ms ease, background-color 150ms ease, opacity 150ms ease, 150ms ease",
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
    <CollapsibleGroup title="圆角" summary={summary} defaultOpen={true} onDoubleClick={() => onFocus?.()}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {mode === "unified" ? (
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
              <span style={labelStyle}>R</span>
              <LockIcon propName="borderRadius" inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
              <span style={isPropLocked("borderRadius", inheritedProps) ? LOCKED_INPUT_STYLE : undefined}>
              {readOnly && inheritedProps?.includes("borderRadius") ? (
                <span style={{ ...tokenTextStyle, cursor: "default", flex: 1 }}>{radiusRaw || "—"}</span>
              ) : isRadiusToken ? (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                  <span
                    style={tokenTextStyle}
                    onClick={(e) => onPropSlash?.("borderRadius", e.currentTarget)}
                  >{radiusRaw}</span>
                  <button type="button"
                    style={tokenRemoveBtn}
                    title="Remove token"
                    onClick={(e) => {
                      e.stopPropagation();
                      onChange(properties.filter((p) => p.name !== "borderRadius"));
                    }}
                  >&times;</button>
                </span>
              ) : (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
                  <span style={radiusRaw === "" ? { opacity: 0.4 } : {}}>
                    <ScrubInput
                      value={radiusValue}
                      unit="px"
                      min={0}
                      max={200}
                      onChange={handleUnifiedChange}
                      onSlash={onPropSlash ? (el) => onPropSlash("borderRadius", el) : undefined}
                    />
                  </span>
                  {radiusRaw !== "" && !isRadiusToken && (
                    <button type="button" style={tokenRemoveBtn} title="清除"
                      onClick={(e) => {
                        e.stopPropagation();
                        onChange(properties.filter((p) => p.name !== "borderRadius"));
                      }}
                    >×</button>
                  )}
                </span>
              )}
              </span>
            </div>
            <button type="button"
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
                rawValue={getProp(properties, CORNER_NAMES[0])}
                onChange={(v) => handleCornerChange(0, v)}
                onSlash={onPropSlash ? (el) => onPropSlash(CORNER_NAMES[0], el) : undefined}
                onRemoveToken={() => onChange(properties.filter((p) => p.name !== CORNER_NAMES[0]))}
              />
              <CornerInput
                label="↗"
                value={cornerValues[1]}
                rawValue={getProp(properties, CORNER_NAMES[1])}
                onChange={(v) => handleCornerChange(1, v)}
                onSlash={onPropSlash ? (el) => onPropSlash(CORNER_NAMES[1], el) : undefined}
                onRemoveToken={() => onChange(properties.filter((p) => p.name !== CORNER_NAMES[1]))}
              />
              <button type="button"
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
                rawValue={getProp(properties, CORNER_NAMES[3])}
                onChange={(v) => handleCornerChange(3, v)}
                onSlash={onPropSlash ? (el) => onPropSlash(CORNER_NAMES[3], el) : undefined}
                onRemoveToken={() => onChange(properties.filter((p) => p.name !== CORNER_NAMES[3]))}
              />
              <CornerInput
                label="↘"
                value={cornerValues[2]}
                rawValue={getProp(properties, CORNER_NAMES[2])}
                onChange={(v) => handleCornerChange(2, v)}
                onSlash={onPropSlash ? (el) => onPropSlash(CORNER_NAMES[2], el) : undefined}
                onRemoveToken={() => onChange(properties.filter((p) => p.name !== CORNER_NAMES[2]))}
              />
              <div style={{ width: 30, flexShrink: 0 }} />
            </div>
          </div>
        )}

        {/* Mini preview */}
        <div
          style={{
            width: "100%",
            height: 48,
            borderRadius: previewRadius,
            border: "2px solid var(--dp-accent, #6366f1)",
            background: "rgba(99, 102, 241, 0.06)",
            transition: "border-radius 150ms ease",
          }}
        />
      </div>
    </CollapsibleGroup>
  );
});

// ── Sub-component: CornerInput ──

interface CornerInputProps {
  label: string;
  value: number;
  rawValue?: string;
  onChange: (v: number) => void;
  onSlash?: (el: HTMLElement) => void;
  onRemoveToken?: () => void;
}

function CornerInput({ label, value, rawValue, onChange, onSlash, onRemoveToken }: CornerInputProps) {
  const isToken = rawValue && isTokenRef(rawValue);
  const isEmpty = !rawValue || rawValue === "";
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
      {isToken ? (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
          <span style={tokenTextStyle} onClick={(e) => onSlash?.(e.currentTarget)}>{rawValue}</span>
          <button type="button" style={tokenRemoveBtn} title="Remove token"
            onClick={(e) => { e.stopPropagation(); onRemoveToken?.(); }}
          >&times;</button>
        </span>
      ) : (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
          <span style={isEmpty ? { opacity: 0.4 } : {}}>
            <ScrubInput
              value={value}
              unit="px"
              min={0}
              max={200}
              onChange={onChange}
              onSlash={onSlash}
            />
          </span>
          {!isEmpty && (
            <button type="button" style={tokenRemoveBtn} title="清除"
              onClick={(e) => { e.stopPropagation(); onRemoveToken?.(); }}
            >×</button>
          )}
        </span>
      )}
    </div>
  );
}
