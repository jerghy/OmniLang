// poiesis/src/services/codegen/design-value-parser.ts
// Parse a raw CSS value string into StyleValueSegment[] for storage in the design store.
//
// "var(--color-primary)"  → [{ type: "var-ref", name: "--color-primary" }]
// "8px 16px"              → [{ type: "text", value: "8px 16px" }]
// "var(--color) 8px"      → [{ type: "var-ref", name: "--color" }, { type: "text", value: " 8px" }]
// "token(primary)"        → [{ type: "var-ref", name: "--primary" }]  (-- prefix added)

import type { StyleValueSegment } from "../../core/types/style";

export function parseStyleValue(value: string): StyleValueSegment[] {
  // Strip surrounding quotes from OmniLang STRING tokens (e.g. `"10px"` → `10px`)
  let v = value;
  if (v.startsWith('"') && v.endsWith('"') && v.length >= 2) {
    v = v.slice(1, -1);
  } else if (v.startsWith("'") && v.endsWith("'") && v.length >= 2) {
    v = v.slice(1, -1);
  }

  const segments: StyleValueSegment[] = [];
  // Match both var(--xxx) and token(--xxx) patterns
  const re = /(var|token)\(([^)]+)\)/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(v)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ type: "text", value: v.slice(lastIndex, match.index) });
    }
    // Always normalize: ensure `--` prefix on CSS custom property names
    const varName = match[2].startsWith("--") ? match[2] : `--${match[2]}`;
    segments.push({ type: "var-ref", name: varName });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < v.length) {
    segments.push({ type: "text", value: v.slice(lastIndex) });
  }
  if (segments.length === 0 && v) {
    segments.push({ type: "text", value: v });
  }
  return segments;
}
