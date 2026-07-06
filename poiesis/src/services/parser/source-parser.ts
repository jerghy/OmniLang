// services/parser/source-parser.ts
// Parse + validate + populate editor state from OmniLang source.
// Extracted 1:1 from App.tsx's useEffect that calls validate() then parseOmniSource().

import { validate, parseOmniSource } from "../codegen";
import { usePoiesisStore } from "../../store";
import { loadCodeTabs } from "../../services/codegraph/store";
import type { ElementData, WireData } from "../../core/types";

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

    return {
      elements: parseResult.designElements,
      wires: parseResult.designWires || [],
    };
  }

  return null;
}
