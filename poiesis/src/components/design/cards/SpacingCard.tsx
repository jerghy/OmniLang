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

const labelStyle: React.CSSProperties = {
  fontSize: 11,
  color: "#64748b",
  width: 14,
  flexShrink: 0,
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
      // Default to unified when nothing is set; switch to individual only
      // when the user has explicitly set individual corner values
      const hasShorthand = getProp(properties, shorthandName) !== "";
      if (hasShorthand) return true;
      const hasAnyCorner = corners.some(
        (c) => getProp(properties, c) !== "",
      );
      return !hasAnyCorner;
    },
  );

  const cornerLabels = ["上", "右", "下", "左"];

  const handleToggle = useCallback(() => {
    if (unified) {
      // unified → independent: spread shorthand to corners, remove shorthand
      const shorthandVal = getProp(properties, shorthandName);
      const px = parsePx(shorthandVal);
      const pix = px > 0 ? `${px}px` : "";
      let newProps = setProp(properties, shorthandName, "");
      for (const name of cornerNames) {
        newProps = setProp(newProps, name, pix);
      }
      onChange(newProps);
    } else {
      // independent → unified: read corners (use top as representative), remove corners
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

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {/* Section title */}
      <div style={{ fontSize: 12, fontWeight: 600, color: "#333" }}>
        {label}
      </div>

      {unified ? (
        /* Unified: single ScrubInput + 🔗 button */
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
            🔗
          </button>
        </div>
      ) : (
        /* Independent: 4 ScrubInputs in a 2×2 grid + 🔓 button */
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {/* Row 1: top, right */}
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                flex: 1,
              }}
            >
              <span style={labelStyle}>{cornerLabels[0]}</span>
              <ScrubInput
                value={cornerVals[0]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(0, v)}
              />
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                flex: 1,
              }}
            >
              <span style={labelStyle}>{cornerLabels[1]}</span>
              <ScrubInput
                value={cornerVals[1]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(1, v)}
              />
            </div>
            <button
              onClick={handleToggle}
              title="统一设置所有方向"
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
              🔓
            </button>
          </div>
          {/* Row 2: bottom, left */}
          <div style={{ display: "flex", gap: 8 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                flex: 1,
              }}
            >
              <span style={labelStyle}>{cornerLabels[2]}</span>
              <ScrubInput
                value={cornerVals[2]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(2, v)}
              />
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                flex: 1,
              }}
            >
              <span style={labelStyle}>{cornerLabels[3]}</span>
              <ScrubInput
                value={cornerVals[3]}
                unit="px"
                min={0}
                max={200}
                onChange={(v) => handleCornerChange(3, v)}
              />
            </div>
            {/* Spacer to align with the button above */}
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
}

export function SpacingCard({ properties, onChange }: SpacingCardProps) {
  // Compute summary from shorthand or individual top values
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

  // Derive spacing values for the preview
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

        <div style={{ borderTop: "1px solid #e0e0e0" }} />

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

        <BoxModelPreview margin={margin} padding={padding} />
      </div>
    </CollapsibleGroup>
  );
}
