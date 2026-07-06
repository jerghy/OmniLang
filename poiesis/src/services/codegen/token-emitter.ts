// poiesis/src/omni-gen/token-emitter.ts
// Phase C: Emit theme/token blocks as OmniLang source.

import type { ThemeDef } from "./types";

/**
 * Emit a list of theme definitions as OmniLang source.
 *
 * Each theme produces a block like:
 *
 *   theme myTheme {
 *     colors {
 *       primary: "#4F46E5";
 *     }
 *   }
 */
export function emitTokens(themes: ThemeDef[]): string {
  if (themes.length === 0) return "";
  const lines: string[] = [];
  for (const theme of themes) {
    lines.push(`theme ${theme.name} {`);
    for (const [category, entries] of Object.entries(theme.categories)) {
      lines.push(`  ${category} {`);
      for (const [key, value] of Object.entries(entries)) {
        lines.push(`    ${key}: "${value}";`);
      }
      lines.push(`  }`);
    }
    lines.push(`}`);
  }
  return lines.join("\n") + "\n";
}
