// WASM compiler diagnostics → Monaco markers integration.
// Uses the WASM OmniCompiler class for fast parse+semantic+lint (no codegen).

import type { editor as IEditor, MarkerSeverity } from "monaco-editor";

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

let omniCompilerInstance: import("../../wasm/omnilang_wasm.js").OmniCompiler | null =
  null;
let wasmInitialized = false;

/**
 * Initialize the WASM module (idempotent). Must be called once before any
 * OmniCompiler usage. In the Tauri app, rustCompiler.init() handles this;
 * in Storybook/test contexts, this function self-initializes.
 */
async function ensureWasmInit(): Promise<boolean> {
  if (wasmInitialized) return true;
  try {
    const wasmModule = await import("../../wasm/omnilang_wasm.js");
    await wasmModule.default(); // wasm-pack __wbg_init
    wasmInitialized = true;
    return true;
  } catch {
    return false;
  }
}

/**
 * Get or create the stateful OmniCompiler WASM instance.
 * In the Tauri app this reuses the already-initialized WASM;
 * in Storybook/test it self-initializes on first call.
 */
export async function getOmniCompiler(): Promise<import("../../wasm/omnilang_wasm.js").OmniCompiler | null> {
  if (omniCompilerInstance) return omniCompilerInstance;

  try {
    const initialized = await ensureWasmInit();
    if (!initialized) return null;

    const wasmModule = await import("../../wasm/omnilang_wasm.js");
    omniCompilerInstance = new wasmModule.OmniCompiler();
    return omniCompilerInstance;
  } catch {
    console.warn("[OmniLang] OmniCompiler not available — WASM not loaded");
    return null;
  }
}

/**
 * Run diagnostics on registered sources using the WASM OmniCompiler.
 *
 * When `allSources` is provided (multi-file mode), uses compile_to_omniview
 * which internally calls analyse_multi for cross-file semantic validation.
 * Falls back to single-file diagnostics() when no extra sources given.
 *
 * @param compiler   The stateful OmniCompiler instance.
 * @param allSources Optional map of ALL project .omni sources for multi-file analysis.
 */
export function runDiagnostics(
  compiler: import("../../wasm/omnilang_wasm.js").OmniCompiler,
  allSources?: Record<string, string>,
): WasmDiagnostic[] {
  try {
    // Multi-file: upsert all sources, then compile_to_omniview (uses analyse_multi)
    if (allSources && Object.keys(allSources).length > 1) {
      for (const [path, source] of Object.entries(allSources)) {
        if (source) compiler.upsert_source(path, source);
      }
      const result = compiler.compile_to_omniview(false, false);
      if (result && typeof result === "object" && "errors" in result) {
        return (result as { errors: WasmDiagnostic[] }).errors ?? [];
      }
      return [];
    }

    // Single-file: fast parse + semantic + lint, no codegen
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

/**
 * Convert a WASM Diagnostic to a Monaco IMarkerData.
 */
function wasmDiagnosticToMarker(
  d: WasmDiagnostic,
  severityMap: Record<string, MarkerSeverity>,
  fallbackSeverity: MarkerSeverity,
): IEditor.IMarkerData {
  const severity = severityMap[d.severity] ?? fallbackSeverity;
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
 * Validate multiple .omni source files through the WASM compiler's multi-file
 * pipeline (parse + semantic + cross-file lint, no codegen output consumed).
 *
 * Uses compile_multi_to_omniview (→ analyse_multi) for true cross-file analysis,
 * catching import errors and cross-file type mismatches that single-file
 * diagnostics() cannot detect.
 *
 * Creates a fresh compiler instance so it doesn't interfere with Monaco.
 *
 * @param sources  Map of file path → source content.
 * @returns        Array of WASM diagnostics. Empty array = no errors.
 */
export async function validateSources(
  sources: Record<string, string>,
): Promise<WasmDiagnostic[]> {
  try {
    const wasmModule = await import("../../wasm/omnilang_wasm.js");
    // compile_multi_to_omniview: standalone multi-file with analyse_multi
    // editor_mode=false, debug_mode=false → bare compile, just for validation
    const result = wasmModule.compile_multi_to_omniview(sources, false, false);
    if (result && Array.isArray(result)) {
      return result as WasmDiagnostic[];
    }
    if (result && typeof result === "object" && "errors" in result) {
      return (result as { errors: WasmDiagnostic[] }).errors ?? [];
    }
    return [];
  } catch (e) {
    console.warn("[OmniLang] validateSources failed:", e);
    return [];
  }
}

/**
 * Apply WASM diagnostics to a Monaco editor model as markers.
 *
 * @param model  The Monaco text model to attach markers to.
 * @param diagnostics  Array of WASM Diagnostic objects.
 * @param owner  Marker owner ID (default: "omnilang").
 */
export async function applyDiagnostics(
  model: IEditor.ITextModel,
  diagnostics: WasmDiagnostic[],
  owner = "omnilang",
): Promise<void> {
  const { editor, MarkerSeverity } = await import("monaco-editor");
  const severityMap: Record<string, MarkerSeverity> = {
    error: MarkerSeverity.Error,
    warning: MarkerSeverity.Warning,
    info: MarkerSeverity.Info,
  };
  const markers = diagnostics.map((d) =>
    wasmDiagnosticToMarker(d, severityMap, MarkerSeverity.Error),
  );
  editor.setModelMarkers(model, owner, markers);
}
