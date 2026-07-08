// poiesis/src/services/codegen/design-value-parser.ts
// Parse a raw CSS value string into StyleValueSegment[] for storage in the design store.
//
// "var(--color-primary)"  → [{ type: "var-ref", name: "--color-primary" }]
// "8px 16px"              → [{ type: "text", value: "8px 16px" }]
// "var(--color) 8px"      → [{ type: "var-ref", name: "--color" }, { type: "text", value: " 8px" }]
// "token(--color)"        → [{ type: "var-ref", name: "--color" }]

import type { StyleValueSegment } from "../../core/types/style";

export function parseStyleValue(value: string): StyleValueSegment[] {
  const segments: StyleValueSegment[] = [];
  // Match both var(--xxx) and token(--xxx) patterns
  const re = /(var|token)\(([^)]+)\)/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(value)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ type: "text", value: value.slice(lastIndex, match.index) });
    }
    segments.push({ type: "var-ref", name: match[2] });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < value.length) {
    segments.push({ type: "text", value: value.slice(lastIndex) });
  }
  if (segments.length === 0 && value) {
    segments.push({ type: "text", value });
  }
  return segments;
}
