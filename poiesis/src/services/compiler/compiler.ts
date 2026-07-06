// services/compiler/compiler.ts
// Compilation orchestration — wraps the generateOmniSource -> compile flow.
// Uses compileToOmniViewFull for single-pass validate+compile when generated
// source is valid, falling back to api.compile.compile() for the original content.

import { rustCompiler } from "../../commands/rustCompiler";
import { generateOmniSource } from "../codegen";
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
 * Generate OmniLang source from editor state, validate and compile it in a
 * single WASM pass (compileToOmniViewFull), then return the compilation result.
 *
 * When the generated source has errors, falls back to compiling the original
 * activeFileContent via the standard compile API.
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
  let compileResponse: CompileResponse;

  if (genResult.value) {
    generatedSource = genResult.value;
    // ONE WASM call: validate + compile in single pass
    const fullResult = rustCompiler.compileToOmniViewFull(genResult.value, false);

    const hasErrors = (fullResult.errors || []).some(e => e.severity === 'error');
    if (hasErrors || !fullResult.files) {
      // Generated source failed — fallback to original content
      validationErrors = (fullResult.errors || []) as unknown as CompileError[];
      content = params.activeFileContent;
      // Fallback: compile original content
      compileResponse = await api.compile.compile(content, params.target, undefined, params.isRunning);
    } else {
      // Generated source passed! Use the result directly
      validationPassed = true;
      content = genResult.value;
      compileResponse = {
        success: true,
        files: {
          'index.html': fullResult.files.html,
          'index.js': fullResult.files.js,
          'index.css': fullResult.files.css,
        },
        diff: {},
        errors: (fullResult.errors || []).map(e => ({
          file: '',
          line: e.line,
          col: e.col,
          severity: (e.severity as 'error' | 'warning'),
          code: e.code,
          message: e.message,
          suggestion: e.suggestion || null,
        })),
      };
    }
  } else {
    // No generated source — compile original
    compileResponse = await api.compile.compile(content, params.target, undefined, params.isRunning);
  }

  if (!content) return null;

  return {
    content,
    compileResponse,
    generatedSource,
    validationPassed,
    validationErrors,
  };
}
