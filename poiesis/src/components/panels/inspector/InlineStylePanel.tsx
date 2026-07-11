// components/panels/inspector/InlineStylePanel.tsx
// Visual inline style editor using the same card components as ClassEditor/VariantEditor.
// Properties are always visible; empty ones are grayed out (auto-locked),
// while set properties are normal with a × to clear.

import { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import { SizeCard } from "../../design/cards/SizeCard";
import { SpacingCard } from "../../design/cards/SpacingCard";
import { BackgroundCard } from "../../design/cards/BackgroundCard";
import { BorderCard } from "../../design/cards/BorderCard";
import { RadiusCard } from "../../design/cards/RadiusCard";
import { TypographyBasics } from "./TypographyBasics";
import type { ElementData, ViewElementData } from "../../../core/types/element";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";

// ── Property name sets for each card ──

const SIZE_PROPS = new Set([
  "width", "height", "minWidth", "maxWidth", "minHeight", "maxHeight",
]);

const SPACING_PROPS = new Set([
  "padding", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
  "margin", "marginTop", "marginRight", "marginBottom", "marginLeft",
]);

const BACKGROUND_PROPS = new Set([
  "background", "backgroundColor",
]);

const BORDER_PROPS = new Set([
  "border", "borderWidth", "borderStyle", "borderColor",
]);

const RADIUS_PROPS = new Set([
  "borderRadius",
  "borderTopLeftRadius", "borderTopRightRadius",
  "borderBottomRightRadius", "borderBottomLeftRadius",
]);

const TYPO_PROPS = new Set([
  "fontSize", "fontWeight", "color",
]);

// Text-like tags that get the typography section
const TEXT_TAGS = new Set([
  "p", "h1", "h2", "h3", "h4", "h5", "h6",
  "span", "a", "button", "label", "li",
]);

interface SectionDef {
  key: string;
  label: string;
  propSet: Set<string>;
}

const SECTIONS: SectionDef[] = [
  { key: "size", label: "尺寸", propSet: SIZE_PROPS },
  { key: "spacing", label: "间距", propSet: SPACING_PROPS },
  { key: "background", label: "背景", propSet: BACKGROUND_PROPS },
  { key: "border", label: "边框", propSet: BORDER_PROPS },
  { key: "radius", label: "圆角", propSet: RADIUS_PROPS },
  { key: "typo", label: "文字", propSet: TYPO_PROPS },
];

// ── Helpers ──

function filterProps(props: StyleProperty[], names: Set<string>): StyleProperty[] {
  return props.filter((p) => names.has(p.name));
}

function mergeCardProps(
  allProps: StyleProperty[],
  cardManaged: Set<string>,
  newCardProps: StyleProperty[],
): StyleProperty[] {
  const otherProps = allProps.filter((p) => !cardManaged.has(p.name));
  return [...otherProps, ...newCardProps];
}

// ── Styles ──

const sectionHeader: React.CSSProperties = {
  fontSize: 11,
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  marginBottom: 6,
  marginTop: 4,
};

// ── Main component ──

interface InlineStylePanelProps {
  element: ElementData;
}

export function InlineStylePanel({ element }: InlineStylePanelProps) {
  const setElementInlineStyles = usePoiesisStore((s) => s.setElementInlineStyles);
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const viewEl = element as ViewElementData;
  const allProps = viewEl.style?.inlineProperties ?? [];
  const tag = viewEl.tag ?? "";
  const isTextElement = TEXT_TAGS.has(tag);

  const makeCommit = useCallback(
    (cardManaged: Set<string>) => (newCardProps: StyleProperty[]) => {
      const merged = mergeCardProps(allProps, cardManaged, newCardProps);
      setElementInlineStyles(element.id, merged);
    },
    [allProps, element.id, setElementInlineStyles],
  );

  const visibleSections = SECTIONS.filter((s) => s.key !== "typo" || isTextElement);

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={sectionHeader}>内联样式</div>

      {visibleSections.map(({ key, propSet }) => {
        const cardProps = filterProps(allProps, propSet);

        return (
          <div key={key}>
            {key === "size" && (
              <SizeCard properties={cardProps} onChange={makeCommit(propSet)} />
            )}
            {key === "spacing" && (
              <SpacingCard properties={cardProps} onChange={makeCommit(propSet)} />
            )}
            {key === "background" && (
              <BackgroundCard
                properties={cardProps}
                tokens={tokens as DesignToken[]}
                onChange={makeCommit(propSet)}
              />
            )}
            {key === "border" && (
              <BorderCard
                properties={cardProps}
                tokens={tokens as DesignToken[]}
                onChange={makeCommit(propSet)}
              />
            )}
            {key === "radius" && (
              <RadiusCard properties={cardProps} onChange={makeCommit(propSet)} />
            )}
            {key === "typo" && (
              <TypographyBasics properties={cardProps} onChange={makeCommit(propSet)} />
            )}
          </div>
        );
      })}
    </div>
  );
}
