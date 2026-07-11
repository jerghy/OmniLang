// WASM compiler diagnostics → Monaco markers integration.
// Uses the WASM OmniCompiler class for fast parse+semantic+lint (no codegen).

import { editor, MarkerSeverity } from "monaco-editor";
import type { editor as IEditor } from "monaco-editor";

// ── Diagnostic types (matches WASM Diagnostic shape) ────────────────

export interface WasmDiagnostic {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  line: number;
  col: number;
  end_line: number | null;
  end_col: number | null;
  suggestion: string | null;
  file?: string | null;
}

// ── OmniCompiler wrapper ────────────────────────────────────────────

let omniCompilerInstance: import("../../wasm/omnilang.js").OmniCompiler | null =
  null;

/**
 * Get or create the stateful OmniCompiler WASM instance.
 * The WASM must already be initialized via rustCompiler.init().
 */
export async function getOmniCompiler(): Promise<import("../../wasm/omnilang.js").OmniCompiler | null> {
  if (omniCompilerInstance) return omniCompilerInstance;

  try {
    const wasmModule = await import("../../wasm/omnilang.js");
    omniCompilerInstance = new wasmModule.OmniCompiler();
    return omniCompilerInstance;
  } catch {
    console.warn("[OmniLang] OmniCompiler not available — WASM not loaded");
    return null;
  }
}

/**
 * Run diagnostics on registered sources using the WASM OmniCompiler.
 * This is fast — it only does parse + semantic + lint, no codegen.
 */
export function runDiagnostics(
  compiler: import("../../wasm/omnilang.js").OmniCompiler,
): WasmDiagnostic[] {
  try {
    const result = compiler.diagnostics();
    if (result && typeof result === "object") {
      const diag = result as { success?: boolean; errors?: WasmDiagnostic[] };
      return diag.errors ?? [];
    }
    return [];
  } catch (e) {
    console.warn("[OmniLang] diagnostics() failed:", e);
    return [];
  }
}

// ── Monaco marker mapping ───────────────────────────────────────────

const severityMap: Record<string, MarkerSeverity> = {
  error: MarkerSeverity.Error,
  warning: MarkerSeverity.Warning,
  info: MarkerSeverity.Info,
};

/**
 * Convert a WASM Diagnostic to a Monaco IMarkerData.
 */
export function wasmDiagnosticToMarker(
  d: WasmDiagnostic,
): IEditor.IMarkerData {
  const severity = severityMap[d.severity] ?? MarkerSeverity.Error;
  const message = d.suggestion
    ? `${d.message}\n💡 ${d.suggestion}`
    : d.message;

  return {
    severity,
    message,
    startLineNumber: d.line > 0 ? d.line : 1,
    startColumn: d.col > 0 ? d.col : 1,
    endLineNumber: d.end_line && d.end_line > 0 ? d.end_line : d.line > 0 ? d.line : 1,
    endColumn: d.end_col && d.end_col > 0 ? d.end_col : d.col > 0 ? d.col + 1 : 2,
    code: d.code,
  };
}

/**
 * Apply WASM diagnostics to a Monaco editor model as markers.
 *
 * @param model  The Monaco text model to attach markers to.
 * @param diagnostics  Array of WASM Diagnostic objects.
 * @param owner  Marker owner ID (default: "omnilang").
 */
export function applyDiagnostics(
  model: IEditor.ITextModel,
  diagnostics: WasmDiagnostic[],
  owner = "omnilang",
): void {
  const markers = diagnostics.map(wasmDiagnosticToMarker);
  editor.setModelMarkers(model, owner, markers);
}
