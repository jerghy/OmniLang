// commands/index.ts
// Composite API client — merges domain-specific API modules (compile, project, annotation)
// Powered by Tauri plugins: fs, store, shell + WASM Rust compiler
// Replaces Python FastAPI backend

import { createCompileApi } from "./compile-api";
import { createProjectApi } from "./project-api";
import { createAnnotationApi } from "./annotation-api";
import { invoke } from "@tauri-apps/api/core";

// ── Tauri backend commands ─────────────────────────

export interface CompileResult {
  success: boolean;
  files: Record<string, string>;
  errors: string[];
}

export interface Diagnostic {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  file: string;
  line: number;
  col: number;
}

export interface AnalyseResult {
  success: boolean;
  diagnostics: Diagnostic[];
}

export interface ExportResult {
  success: boolean;
  output_dir: string;
  file_count: number;
  errors: string[];
}

/**
 * Compile a project directory via the Tauri Rust backend.
 * Discovers .omni files and compiles them using the omnilang crate.
 */
export async function compileProject(projectPath: string, target: string): Promise<CompileResult> {
  return invoke<CompileResult>("compile_project", { projectPath, target });
}

/**
 * Run static analysis on .omni source via the Tauri Rust backend.
 * Returns diagnostics including errors and warnings.
 */
export async function analyseSource(source: string, filePath: string): Promise<AnalyseResult> {
  return invoke<AnalyseResult>("analyse_source", { source, filePath });
}

/**
 * Export a compiled project to an output directory.
 * Compiles .omni files then writes the generated targets to the specified directory.
 */
export async function projectExport(projectPath: string, outputDir: string, target: string): Promise<ExportResult> {
  return invoke<ExportResult>("project_export", { projectPath, outputDir, target });
}

// ── Composite API Client ────────────────────────────

class ApiClient {
  compile = createCompileApi();
  project = createProjectApi();
  annotation = createAnnotationApi();

  // ── Watch (polling-based replacement for WebSocket) ──
  // The old WebSocket /api/watch is replaced with a polling-based approach
  // since we now use Tauri plugins. For real-time file watching, the Rust
  // file_watch command can be used.

  private watchTimer: number | null = null;
  private onStatusChange: ((status: string, isError: boolean) => void) | null = null;

  connectWatch(
    _onCompiled: (data: unknown) => void,
    onStatus: (status: string, isError: boolean) => void,
  ): void {
    this.onStatusChange = onStatus;
    // Polling is light — the compile is triggered explicitly by the user,
    // so we just report connection status
    this.onStatusChange("就绪", false);
  }

  disconnect(): void {
    if (this.watchTimer !== null) {
      clearInterval(this.watchTimer);
      this.watchTimer = null;
    }
    this.onStatusChange = null;
  }

  // ── Debug WebSocket replacement ──
  // Debug communication now happens through Tauri events or direct invocation

  private onDebugMessage: ((msg: Record<string, unknown>) => void) | null = null;
  private onDebugStatusChange: ((connected: boolean) => void) | null = null;

  connectDebugWs(
    onMessage: (msg: Record<string, unknown>) => void,
    onStatusChange: (connected: boolean) => void,
  ): void {
    this.onDebugMessage = onMessage;
    this.onDebugStatusChange = onStatusChange;
    // Report connected immediately — debug commands come via sendDebugCommand
    this.onDebugStatusChange(true);
  }

  sendDebugCommand(_command: Record<string, unknown>): void {
    // TODO: Route debug commands via Tauri events or direct Rust calls
    if (this.onDebugMessage) {
      this.onDebugMessage({ type: "command_echo", command: _command });
    }
  }

  disconnectDebug(): void {
    this.onDebugMessage = null;
    this.onDebugStatusChange = null;
  }
}

export const api = new ApiClient();
