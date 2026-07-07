// components/design/render-utils.ts
// Shared rendering utilities for the OmniView renderer.

import type { ElementData, ViewElementData } from "../../core/types/element";
import type { StyleValueSegment } from "../../core/types/style";
import type { DesignToken, DesignClass, DesignDefinition } from "../../core/types/design";

export function isTextSegment(
  s: StyleValueSegment
): s is StyleValueSegment & { type: "text"; value: string } {
  return s.type === "text";
}

export const tagMap: Record<string, string> = {
  container: "div", flex: "div", grid: "div", card: "div",
  text: "span", heading: "h1", button: "button", input: "input",
  textarea: "textarea", select: "select", checkbox: "input",
  list: "ul", listitem: "li", image: "img", spinner: "div",
  progress: "progress", alert: "div", link: "a", form: "form",
  div: "div", span: "span", img: "img", a: "a", p: "p",
  h1: "h1", h2: "h2", h3: "h3", h4: "h4", h5: "h5", h6: "h6",
  section: "section", article: "article", header: "header",
  footer: "footer", nav: "nav", main: "main",
  ul: "ul", ol: "ol", li: "li", label: "label", hr: "hr",
};

export const blockTags = new Set([
  "div", "section", "article", "header", "footer", "nav", "main",
  "ul", "ol", "li", "form", "h1", "h2", "h3", "h4", "h5", "h6", "p",
]);

export function resolveTag(tag: string): string {
  return tagMap[tag.toLowerCase()] || tagMap[tag] || "div";
}

export function getDefaultInlineStyles(el: ViewElementData): Record<string, string> {
  const styles: Record<string, string> = {};
  const htmlTag = resolveTag(el.tag);
  if (blockTags.has(htmlTag)) {
    styles.display = "block";
    if (!el.props?.label && el.children.length === 0) {
      styles.minHeight = "32px";
    }
  }
  if (htmlTag === "span" && !el.props?.label) {
    styles.display = "inline-block";
    styles.minWidth = "32px";
    styles.minHeight = "20px";
  }
  return styles;
}

/** Resolve a style value from segments, following var-ref through tokens. */
export function resolveSegmentsValue(
  segments: StyleValueSegment[],
  tokens: DesignToken[],
): string | null {
  for (const seg of segments) {
    if (seg.type === "text") return seg.value;
    if (seg.type === "var-ref") {
      const token = tokens.find((t) => t.name === seg.name);
      if (token) return token.value;
    }
  }
  return null;
}

/**
 * Pre-compute inline styles for all elements in the expanded tree.
 * Uses O(1) Map lookups instead of O(n) Array.find() for class resolution.
 */
export function buildInlineStyleMap(
  expandedElements: ReadonlyMap<string, ElementData>,
  activeDesign: DesignDefinition,
): Map<string, Record<string, string>> {
  const map = new Map<string, Record<string, string>>();

  // Build O(1) class name lookup
  const classMap = new Map<string, DesignClass>(activeDesign.classes.map((c) => [c.name, c]));

  for (const [id, el] of expandedElements) {
    // Only view elements have styles
    if (!("children" in el && "tag" in el)) continue;

    const styles: Record<string, string> = { ...getDefaultInlineStyles(el) };

    if (el.style?.inlineProperties) {
      for (const prop of el.style.inlineProperties) {
        const textSeg = prop.segments.find(isTextSegment);
        if (textSeg) styles[prop.name] = textSeg.value;
      }
    }

    if (el.style?.classRefs) {
      for (const ref of el.style.classRefs) {
        // Strip namespace prefix (e.g. "AppDesign.Card" → "Card")
        const className = ref.includes(".") ? ref.split(".").slice(1).join(".") : ref;
        const classDef = classMap.get(className);
        if (!classDef) continue;
        for (const prop of classDef.properties) {
          const resolved = resolveSegmentsValue(prop.segments, activeDesign.tokens);
          if (resolved != null) {
            styles[prop.name] = resolved;
          }
        }
      }
    }

    map.set(id, styles);
  }

  return map;
}
