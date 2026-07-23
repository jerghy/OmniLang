// services/parser/source-parser.ts
// Parse + validate + populate editor state from OmniLang source.
// Extracted 1:1 from App.tsx's useEffect that calls validate() then parseOmniSource().

import { validate } from "../codegen/validator";
import { parseOmniSource } from "../codegen/omni-parser";
import { parseStyleValue } from "../codegen/design-value-parser";
import { usePoiesisStore } from "../../store";
import { loadCodeTabs } from "../../services/codegraph/store";
import { uid } from "../../core/utils/id";
import type { ElementData, WireData } from "../../core/types";
import type { DesignDefinition, TokenType } from "../../core/types/design";
import { tokenTypeToCategory } from "../../core/types/design";

export interface ParseResult {
  elements: ElementData[];
  wires: WireData[];
}

/**
 * Validate and parse OmniLang source, then populate the editor store with
 * design elements, wires, and code graph tabs.
 *
 * Returns the parsed elements and wires on success, or null if validation
 * failed (caller should check compile errors in the store).
 */
export function parseAndPopulate(
  source: string,
  setCompileErrors: (errors: unknown[]) => void,
  setStatus: (s: { text: string; isError: boolean }) => void,
): ParseResult | null {
  // ── Validate before parse (spec: validate first, if not valid → report error) ──
  const validation = validate(source);
  if (!validation.valid) {
    setCompileErrors(validation.errors);
    setStatus({
      text: `❌ 验证失败: ${validation.errors.length} 个错误`,
      isError: true,
    });
    return null;
  }

  // ── Parse `.omni` source to populate design elements and code tabs ──
  const parseResult = parseOmniSource(source);
  if (parseResult.designElements) {
    usePoiesisStore.setState({
      elements: parseResult.designElements,
      wires: parseResult.designWires,
    });
    loadCodeTabs(parseResult.codeTabs);

    // ── If the parsed source contains design items, populate the design store ──
    if ((parseResult.designTokens?.length ?? 0) > 0
        || (parseResult.designClasses?.length ?? 0) > 0
        || (parseResult.designVariants?.length ?? 0) > 0) {
      const design: DesignDefinition = {
        name: parseResult.designName || "AppDesign",
        tokens: (parseResult.designTokens ?? []).map((t) => ({
          id: t.id,
          name: t.name,
          category: tokenTypeToCategory(t.tokenType),
          tokenType: t.tokenType as TokenType,
          value: t.value,
        })),
        classes: parseResult.designClasses
          ? parseResult.designClasses.map((c) => ({
              id: c.id,
              name: c.name,
              properties: (c.properties ?? []).map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              })),
              pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
                id: uid("ps"),
                pseudo: ps.pseudo,
                properties: ps.properties.map((p) => ({
                  id: uid("sp"),
                  name: p.name,
                  segments: parseStyleValue(p.value),
                })),
              })),
            }))
          : [],
        variants: parseResult.designVariants
          ? parseResult.designVariants.map((v) => ({
              id: v.id,
              name: v.name,
              dimensions: (v.dimensions ?? []).map((d) => ({
                name: d.name,
                values: d.values.map((dv) => dv.name),
              })),
              defaults: v.defaults ?? {},
              baseProperties: (v.baseProperties ?? []).map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              })),
              branches: (v.branches ?? []).map((b) => {
                const dotIdx = b.name.indexOf(".");
                const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
                const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
                const props = b.properties.map((p) => ({
                  id: uid("sp"),
                  name: p.name,
                  segments: parseStyleValue(p.value),
                }));
                return { id: uid("vb"), dimension: dim, value: val, properties: props };
              }),
            }))
          : [],
      };
      usePoiesisStore.getState().loadDesign(design);
    }

    return {
      elements: parseResult.designElements,
      wires: parseResult.designWires || [],
    };
  }

  return null;
}
