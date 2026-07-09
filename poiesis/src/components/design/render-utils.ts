// components/design/render-utils.ts
// Shared rendering utilities for the OmniView renderer.

import type { ViewElementData } from "../../core/types/element";
import type { StyleValueSegment } from "../../core/types/style";
import type { DesignToken } from "../../core/types/design";

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

