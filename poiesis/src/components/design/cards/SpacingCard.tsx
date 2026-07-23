// components/design/cards/SpacingCard.tsx
// Spacing editing card: padding and margin sections with unified/independent toggle
// and a live box-model preview.

import React, { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import { BoxModelPreview } from "./BoxModelPreview";
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

/** Check if a value string is a token reference (not a plain numeric value). */
function isTokenRef(value: string): boolean {
  return value.startsWith("token(");
}

const tokenValueStyle: React.CSSProperties = {
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  color: "var(--dp-accent, #6366f1)",
  flex: 1,
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

/** Render a ScrubInput for numeric values, or a text label for token refs.
 *  Empty values are grayed-out but interactive; set values show normal with × to clear. */
function SpacingValue({
  value,
  rawValue,
  onChange,
  onSlash,
  onTokenClick,
  onRemoveToken,
  locked,
}: {
  value: number;
  rawValue: string;
  onChange: (v: number) => void;
  onSlash?: (el: HTMLElement) => void;
  onTokenClick?: (el: HTMLElement) => void;
  onRemoveToken?: () => void;
  locked?: boolean;
}) {
  const isEmpty = rawValue === "";

  if (isTokenRef(rawValue)) {
    return (
      <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
        <span role="button" tabIndex={0} style={tokenValueStyle} onClick={(e) => onTokenClick?.(e.currentTarget)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onTokenClick?.(e.currentTarget); } }}>{rawValue}</span>
        <button type="button" style={tokenRemoveBtn} title="Remove token"
          onClick={(e) => { e.stopPropagation(); onRemoveToken?.(); }}
        >&times;</button>
      </span>
    );
  }

  // Variant-locked: fully disabled (pointerEvents: none). Empty: grayed but interactive.
  const wrapperStyle: React.CSSProperties = locked
    ? LOCKED_INPUT_STYLE
    : isEmpty
      ? { opacity: 0.4 }
      : {};

  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
      <span style={wrapperStyle}>
        <ScrubInput value={value} unit="px" min={0} max={200} onChange={onChange} onSlash={onSlash} />
      </span>
      {!isEmpty && !locked && (
        <button type="button"
          style={tokenRemoveBtn}
          title="清除"
          onClick={(e) => { e.stopPropagation(); onRemoveToken?.(); }}
        >&times;</button>
      )}
    </span>
  );
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

// ── SpacingSection (padding or margin sub-section) ──

interface SpacingSectionProps {
  label: string;
  shorthandName: string;
  cornerNames: [string, string, string, string];
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
  onPropSlash?: (propName: string, el: HTMLElement) => void;
  inheritedProps?: string[];
  onToggleInherited?: (propName: string) => void;
  readOnly?: boolean;
}

const cornerLabelStyle: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, #6b6893)",
  width: 14,
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

const CORNER_LABELS = ["上", "右", "下", "左"];

const LINKED_BTN_STYLE: React.CSSProperties = {
  ...toggleBtnBase,
  border: "1px solid var(--dp-accent, #6366f1)",
  background: "var(--dp-accent-subtle, #eef0ff)",
  color: "var(--dp-accent, #6366f1)",
};

const UNLINKED_BTN_STYLE: React.CSSProperties = {
  ...toggleBtnBase,
  border: "1px solid var(--dp-border, #e8e5f2)",
  background: "transparent",
  color: "var(--dp-text-muted, #a09db8)",
};

function SpacingSection({
  label,
  shorthandName,
  cornerNames,
  properties,
  onChange,
  onPropSlash,
  inheritedProps,
  onToggleInherited,
  readOnly,
}: SpacingSectionProps) {
  // Derive unified/individual mode from current properties so switching
  // branches (e.g. base → size.sm) correctly reflects the new props.
  const [userToggled, setUserToggled] = useState(false);
  const deducedUnified = (() => {
    const hasShorthand = getProp(properties, shorthandName) !== "";
    if (hasShorthand) return true;
    const hasAnyCorner = cornerNames.some(
      (c) => getProp(properties, c) !== "",
    );
    if (hasAnyCorner) return false;
    return true;
  })();
  const unified = userToggled ? !deducedUnified : deducedUnified;

  const handleToggle = useCallback(() => {
    if (unified) {
      const shorthandVal = getProp(properties, shorthandName);
      const px = parsePx(shorthandVal);
      const pix = px > 0 ? `${px}px` : "";
      let newProps = setProp(properties, shorthandName, "");
      for (const name of cornerNames) {
        newProps = setProp(newProps, name, pix);
      }
      onChange(newProps);
    } else {
      const vals = cornerNames.map((n) => parsePx(getProp(properties, n)));
      const pix = vals[0] > 0 ? `${vals[0]}px` : "";
      let newProps = [...properties];
      for (const name of cornerNames) {
        newProps = setProp(newProps, name, "");
      }
      newProps = setProp(newProps, shorthandName, pix);
      onChange(newProps);
    }
    setUserToggled((v) => !v);
  }, [unified, properties, onChange, shorthandName, cornerNames]);

  const handleShorthandChange = useCallback(
    (v: number) => {
      onChange(setProp(properties, shorthandName, `${v}px`));
    },
    [properties, onChange, shorthandName],
  );

  const handleCornerChange = useCallback(
    (index: number, v: number) => {
      onChange(setProp(properties, cornerNames[index], `${v}px`));
    },
    [properties, onChange, cornerNames],
  );

  const shorthandRaw = getProp(properties, shorthandName);
  const isShorthandToken = isTokenRef(shorthandRaw);
  const shorthandVal = isShorthandToken ? 0 : parsePx(shorthandRaw);
  const cornerVals = cornerNames.map((n) => {
    const raw = getProp(properties, n);
    return isTokenRef(raw) ? 0 : parsePx(raw);
  }) as [number, number, number, number];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <div
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: "var(--dp-text-secondary, #6b6893)",
          textTransform: "uppercase",
          letterSpacing: "0.3px",
        }}
      >
        {label}
        <LockIcon propName={shorthandName} inheritedProps={inheritedProps} onToggleInherited={onToggleInherited} readOnly={readOnly} />
      </div>

      {unified ? (
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ flex: 1 }}>
            {isShorthandToken ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                <span
                  role="button"
                  tabIndex={0}
                  style={tokenValueStyle}
                  onClick={(e) => onPropSlash?.(shorthandName, e.currentTarget)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onPropSlash?.(shorthandName, e.currentTarget); } }}
                >{shorthandRaw}</span>
                <button type="button" style={tokenRemoveBtn} title="Remove token"
                  onClick={(e) => {
                    e.stopPropagation();
                    onChange(properties.filter((p) => p.name !== shorthandName));
                  }}
                >&times;</button>
              </span>
            ) : (
              (() => {
                const isEmptyShorthand = shorthandRaw === "";
                const lockedShorthand = isPropLocked(shorthandName, inheritedProps);
                const wrapperS: React.CSSProperties = lockedShorthand
                  ? LOCKED_INPUT_STYLE
                  : isEmptyShorthand
                    ? { opacity: 0.4 }
                    : {};
                return (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 2, flex: 1 }}>
                    <span style={wrapperS}>
                      <ScrubInput
                        value={shorthandVal}
                        unit="px"
                        min={0}
                        max={200}
                        onChange={handleShorthandChange}
                        onSlash={onPropSlash ? (el) => onPropSlash(shorthandName, el) : undefined}
                      />
                    </span>
                    {!isEmptyShorthand && !lockedShorthand && (
                      <button type="button" style={tokenRemoveBtn} title="清除"
                        onClick={(e) => {
                          e.stopPropagation();
                          onChange(properties.filter((p) => p.name !== shorthandName));
                        }}
                      >&times;</button>
                    )}
                  </span>
                );
              })()
            )}
          </div>
          <button type="button"
            onClick={handleToggle}
            title="独立设置各方向"
            style={LINKED_BTN_STYLE}
          >
            🔗
          </button>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {[0, 1].map((i) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
                <span style={cornerLabelStyle}>{CORNER_LABELS[i]}</span>
                <SpacingValue
                  value={cornerVals[i]}
                  rawValue={getProp(properties, cornerNames[i])}
                  onChange={(v) => handleCornerChange(i, v)}
                  onSlash={onPropSlash ? (el) => onPropSlash(cornerNames[i], el) : undefined}
                  onTokenClick={onPropSlash ? (el) => onPropSlash(cornerNames[i], el) : undefined}
                  onRemoveToken={() => onChange(properties.filter((p) => p.name !== cornerNames[i]))}
                  locked={isPropLocked(cornerNames[i], inheritedProps)}
                />
              </div>
            ))}
            <button type="button" onClick={handleToggle} title="统一设置所有方向" style={UNLINKED_BTN_STYLE}>🔓</button>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            {[2, 3].map((i) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
                <span style={cornerLabelStyle}>{CORNER_LABELS[i]}</span>
                <SpacingValue
                  value={cornerVals[i]}
                  rawValue={getProp(properties, cornerNames[i])}
                  onChange={(v) => handleCornerChange(i, v)}
                  onSlash={onPropSlash ? (el) => onPropSlash(cornerNames[i], el) : undefined}
                  onTokenClick={onPropSlash ? (el) => onPropSlash(cornerNames[i], el) : undefined}
                  onRemoveToken={() => onChange(properties.filter((p) => p.name !== cornerNames[i]))}
                  locked={isPropLocked(cornerNames[i], inheritedProps)}
                />
              </div>
            ))}
            <div style={{ width: 25, flexShrink: 0 }} />
          </div>
        </div>
      )}
    </div>
  );
}

// ── Main component ──

interface SpacingCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
  onFocus?: () => void;
  onPropSlash?: (propName: string, el: HTMLElement) => void;
  inheritedProps?: string[];
  onToggleInherited?: (propName: string) => void;
  readOnly?: boolean;
}

export const SpacingCard = React.memo(function SpacingCard({ properties, onChange, onFocus, onPropSlash, inheritedProps, onToggleInherited, readOnly }: SpacingCardProps) {
  const padShorthand = getProp(properties, "padding");
  const padTop = getProp(properties, "padding-top");
  const marShorthand = getProp(properties, "margin");
  const marTop = getProp(properties, "margin-top");

  const summaryParts: string[] = [];
  if (padShorthand) summaryParts.push(`P:${padShorthand}`);
  else if (padTop) summaryParts.push(`P:${padTop}`);
  if (marShorthand) summaryParts.push(`M:${marShorthand}`);
  else if (marTop) summaryParts.push(`M:${marTop}`);

  const summary = summaryParts.length > 0 ? summaryParts.join(" ") : "—";

  function spacingForPreview(
    shorthand: string,
    corners: [string, string, string, string],
  ): { top: number; right: number; bottom: number; left: number } {
    const sh = parsePx(getProp(properties, shorthand));
    if (sh > 0) return { top: sh, right: sh, bottom: sh, left: sh };
    return {
      top: parsePx(getProp(properties, corners[0])),
      right: parsePx(getProp(properties, corners[1])),
      bottom: parsePx(getProp(properties, corners[2])),
      left: parsePx(getProp(properties, corners[3])),
    };
  }

  const padding = spacingForPreview("padding", [
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
  ]);
  const margin = spacingForPreview("margin", [
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
  ]);

  return (
    <CollapsibleGroup title="间距" summary={summary} defaultOpen={true} onDoubleClick={() => onFocus?.()}>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        <SpacingSection
          label="Padding"
          shorthandName="padding"
          cornerNames={[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
          ]}
          properties={properties}
          onChange={onChange}
          onPropSlash={onPropSlash}
          inheritedProps={inheritedProps}
          onToggleInherited={onToggleInherited}
          readOnly={readOnly}
        />

        <div
          style={{
            borderTop: "1px solid var(--dp-border, #e8e5f2)",
            margin: "2px 0",
          }}
        />

        <SpacingSection
          label="Margin"
          shorthandName="margin"
          cornerNames={[
            "margin-top",
            "margin-right",
            "margin-bottom",
            "margin-left",
          ]}
          properties={properties}
          onChange={onChange}
          onPropSlash={onPropSlash}
          inheritedProps={inheritedProps}
          onToggleInherited={onToggleInherited}
          readOnly={readOnly}
        />

        {/* Box model preview */}
        <BoxModelPreview margin={margin} padding={padding} />
      </div>
    </CollapsibleGroup>
  );
});
