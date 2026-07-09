// components/design/render-utils.ts
// Shared rendering utilities for the OmniView renderer.

import type { StyleValueSegment } from "../../core/types/style";

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

