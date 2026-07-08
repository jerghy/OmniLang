// components/design/cards/SpacingCard.tsx
// Spacing editing card: padding and margin sections with unified/independent toggle
// and a live box-model preview.

import { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import { BoxModelPreview } from "./BoxModelPreview";
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
}

const cornerLabelStyle: React.CSSProperties = {
  fontSize: 11,
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
  transition: "all 150ms ease",
};

function SpacingSection({
  label,
  shorthandName,
  cornerNames,
  properties,
  onChange,
}: SpacingSectionProps) {
  const [unified, setUnified] = useState(
    () => {
      const hasShorthand = getProp(properties, shorthandName) !== "";
      if (hasShorthand) return true;
      const hasAnyCorner = cornerNames.some(
        (c) => getProp(properties, c) !== "",
      );
      return !hasAnyCorner;
    },
  );

  const cornerLabels = ["上", "右", "下", "左"];

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
    setUnified((v) => !v);
  }, [unified, properties, onChange, shorthandName, cornerNames]);

  const handleShorthandChange = useCallback(
    (v: number) => {
      const newProps = setProp(properties, shorthandName, `${v}px`);
      onChange(newProps);
    },
    [properties, onChange, shorthandName],
  );

  const handleCornerChange = useCallback(
    (index: number, v: number) => {
      const newProps = setProp(properties, cornerNames[index], `${v}px`);
      onChange(newProps);
    },
    [properties, onChange, cornerNames],
  );

  const shorthandVal = parsePx(getProp(properties, shorthandName));
  const cornerVals = cornerNames.map((n) =>
    parsePx(getProp(properties, n)),
  ) as [number, number, number, number];

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
      </div>

      {unified ? (
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ flex: 1 }}>
            <ScrubInput
              value={shorthandVal}
              unit="px"
              min={0}
              max={200}
              onChange={handleShorthandChange}
            />
          </div>
          <button
            onClick={handleToggle}
            title="独立设置各方向"
            style={linkedBtnStyle}
          >
            🔗
          </button>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
              <span style={cornerLabelStyle}>{cornerLabels[0]}</span>
              <ScrubInput
                value={cornerVals[0]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(0, v)}
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
              <span style={cornerLabelStyle}>{cornerLabels[1]}</span>
              <ScrubInput
                value={cornerVals[1]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(1, v)}
              />
            </div>
            <button onClick={handleToggle} title="统一设置所有方向" style={unlinkedBtnStyle}>
              🔓
            </button>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
              <span style={cornerLabelStyle}>{cornerLabels[2]}</span>
              <ScrubInput
                value={cornerVals[2]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(2, v)}
              />
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
              <span style={cornerLabelStyle}>{cornerLabels[3]}</span>
              <ScrubInput
                value={cornerVals[3]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(3, v)}
              />
            </div>
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
}

export function SpacingCard({ properties, onChange, onFocus }: SpacingCardProps) {
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
    <CollapsibleGroup title="间距" summary={summary} defaultOpen={true}>
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
        />

        {/* Box model preview — right-click for focus mode */}
        <div
          onContextMenu={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onFocus?.();
          }}
          style={{ cursor: onFocus ? "context-menu" : "default" }}
          title={onFocus ? "Right-click for focus mode" : undefined}
        >
          <BoxModelPreview margin={margin} padding={padding} />
        </div>
      </div>
    </CollapsibleGroup>
  );
}
