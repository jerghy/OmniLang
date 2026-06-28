// services/compiler/compiler.ts
// Compilation orchestration — wraps the generateOmniSource → api.compile flow.

import { generateOmniSource, validate } from "../codegen";
import { api } from "../../commands";
import type { CompileResponse, CompileError } from "../../commands/types";
import type { ElementData, WireData, CodeTab } from "../../core/types";

export interface CompileProjectParams {
  elements: ElementData[];
  wires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder: string[];
  activeFileContent: string;
  target: string;
  isRunning: boolean;
}

export interface CompileProjectResult {
  /** The content that was compiled (generated source or original fallback). */
  content: string;
  /** Response from the server-side compile API call. */
  compileResponse: CompileResponse;
  /** The generated OmniLang source, if any. */
  generatedSource: string | null;
  /** Whether the generated source passed validation. */
  validationPassed: boolean;
  /** Validation errors when generation failed validation. */
  validationErrors: CompileError[];
}

/**
 * Generate OmniLang source from editor state, validate it, and compile via
 * the server-side compile API. Returns the compilation result along with
 * metadata about whether generation/validation was used.
 *
 * When generation fails validation the original activeFileContent is used
 * as a fallback (same behavior as the original inline handleCompile).
 */
export async function compileProject(
  params: CompileProjectParams,
): Promise<CompileProjectResult | null> {
  const genResult = generateOmniSource({
    designElements: params.elements,
    designWires: params.wires,
    codeTabs: params.codeTabs,
    componentName: params.componentName,
    rootOrder: params.rootOrder,
  });

  let content = params.activeFileContent;
  let generatedSource: string | null = null;
  let validationPassed = false;
  let validationErrors: CompileError[] = [];

  if (genResult.value) {
    generatedSource = genResult.value;
    const validationResult = validate(genResult.value);
    if (!validationResult.valid) {
      // Fallback to original content when generated source fails validation
      validationErrors = validationResult.errors as unknown as CompileError[];
      content = params.activeFileContent;
    } else {
      validationPassed = true;
      content = genResult.value;
    }
  }

  if (!content) return null;

  const compileResponse = await api.compile.compile(
    content,
    params.target,
    undefined,
    params.isRunning,
  );

  return {
    content,
    compileResponse,
    generatedSource,
    validationPassed,
    validationErrors,
  };
}
