// Rust WASM compiler loader for the OmniLang editor.
// Loads the WASM-compiled omni-compiler and exposes compile_to_ir / compile_to_omniview.

import { Logger } from "../lib/utils/logger";

const log = new Logger({ prefix: "RustCompiler" });

type CompileResult = {
  success: boolean;
  ir?: IRProgram;
  errors?: Diagnostic[];
};

type OmniViewResult = {
  success: boolean;
  files?: { js: string; html: string; css: string };
  sourceMap?: SourceMap;
  errors?: Diagnostic[];
};

type IRProgram = {
  version: string;
  components: Record<string, unknown>;
  stores: Record<string, unknown>;
  source_map: Record<string, unknown>;
};

type Diagnostic = {
  severity: string;
  code: string;
  message: string;
  line: number;
  col: number;
  end_line: number;
  end_col: number;
  suggestion?: string;
};

type SourceMap = {
  entries: SourceMapEntry[];
};

type SourceMapEntry = {
  node_id: string;
  omni_span: { line: number; col: number; len: number };
  target_spans: { file: string; line: number; col: number; len: number; kind: string }[];
};

type ComponentCompileResult = {
  success: boolean;
  component?: { js: string; html: string; css: string };
  errors?: Diagnostic[];
};

type OmniViewFullResult = {
  success: boolean;
  ir?: IRProgram | null;
  files?: { js: string; html: string; css: string } | null;
  components?: Record<string, { js: string; html: string; css: string }> | null;
  sourceMap?: SourceMap | null;
  errors?: Diagnostic[];
};

type MultiCompileResult = {
  success: boolean;
  files: Record<string, string>;
  errors?: string[];
};

class RustCompiler {
  private wasm: {
    compile_to_ir: (source: string) => string;
    compile_to_omniview: (source: string, options_json: string) => string;
    compile_omniview_component?: (source: string, component_name: string, options_json: string) => string;
    version: () => string;
    compile_to_omniview_full?: (source: string, options_json: string) => string;
    compile_multi_to_omniview?: (sourcesJson: string, optionsJson: string) => string;
  } | null = null;
  private ready = false;

  async init(): Promise<void> {
    if (this.ready) return;

    try {
      // Dynamic import of the WASM JS glue — resolved at runtime, not at build time
      // eslint-disable-next-line
      const wasmModule = await import(
        // @ts-ignore — WASM module resolved at runtime, type decls may be missing on CI
        /* @vite-ignore */ '../../public/wasm/omnilang.js'
      );

      try {
        // Primary: fetch-based initialization (works in browser/dev server)
        await wasmModule.default();
      } catch (_fetchErr) {
        // Fallback: use initSync + readFileSync (works in test/Node environments
        // where fetch() is unavailable or fails)
        const nodeFs = 'node:fs' as string;
        const nodePath = 'node:path' as string;
        const nodeUrl = 'node:url' as string;
        const { readFileSync } = await import(nodeFs) as { readFileSync: (path: string) => Uint8Array };
        const { resolve, dirname } = await import(nodePath) as { resolve: (...args: string[]) => string; dirname: (path: string) => string };
        const { fileURLToPath } = await import(nodeUrl) as { fileURLToPath: (url: string) => string };

        const __filename = fileURLToPath(import.meta.url);
        const __dirname = dirname(__filename);
        const wasmPath = resolve(
          __dirname,
          '../../public/wasm/omnilang_bg.wasm',
        );
        const wasmBytes = readFileSync(wasmPath);
        wasmModule.initSync(wasmBytes.buffer as ArrayBuffer);
      }

      this.wasm = {
        compile_to_ir: wasmModule.compile_to_ir,
        compile_to_omniview: wasmModule.compile_to_omniview,
        compile_omniview_component: wasmModule.compile_omniview_component,
        version: wasmModule.version,
        compile_to_omniview_full: wasmModule.compile_to_omniview_full,
        compile_multi_to_omniview: wasmModule.compile_multi_to_omniview,
      };
      this.ready = true;

      log.info(
        `Loaded omni-compiler v${this.wasm.version()}`,
      );
    } catch (err) {
      log.warn('WASM not available, compilation will be unavailable:', err);
      this.ready = false;
    }
  }

  get isReady(): boolean {
    return this.ready;
  }

  compileToIR(source: string): CompileResult {
    if (!this.wasm) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: 'Rust compiler not initialized', line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      const json = this.wasm.compile_to_ir(source);
      return JSON.parse(json);
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `Compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
  }

  compileToOmniView(source: string, editorMode: boolean = false): OmniViewResult {
    if (!this.wasm || !this.wasm.compile_to_omniview) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: 'OmniView codegen not available', line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      const options = JSON.stringify({ editor_mode: editorMode });
      const json = this.wasm.compile_to_omniview(source, options);
      return JSON.parse(json);
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `OmniView compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
  }

  compileOmniViewComponent(source: string, componentName: string, editorMode: boolean = false): ComponentCompileResult {
    if (!this.wasm || !this.wasm.compile_omniview_component) {
      // Fallback: use full compile and extract on TS side
      const full = this.compileToOmniView(source, editorMode);
      if (!full.success || !full.files) {
        return { success: false, errors: full.errors };
      }
      return {
        success: true,
        component: {
          js: full.files.js,
          html: full.files.html,
          css: full.files.css,
        },
      };
    }
    try {
      const options = JSON.stringify({ editor_mode: editorMode });
      const json = this.wasm.compile_omniview_component(source, componentName, options);
      return JSON.parse(json) as ComponentCompileResult;
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `Component compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
  }

  getVersion(): string {
    if (!this.wasm) return 'unavailable';
    try {
      return this.wasm.version();
    } catch {
      return 'unknown';
    }
  }

  compileToOmniViewFull(source: string, editorMode: boolean = false): OmniViewFullResult {
    if (!this.wasm || !this.wasm.compile_to_omniview_full) {
      // Fallback: use existing compileToOmniView
      const omniResult = this.compileToOmniView(source, editorMode);
      return {
        success: omniResult.success,
        ir: null,
        files: omniResult.files ?? null,
        components: null,
        sourceMap: omniResult.sourceMap ?? null,
        errors: omniResult.errors,
      };
    }
    try {
      const options = JSON.stringify({ editor_mode: editorMode });
      const json = this.wasm.compile_to_omniview_full(source, options);
      return JSON.parse(json) as OmniViewFullResult;
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `Compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
  }

  compileMultiToOmniView(sources: Record<string, string>, editorMode: boolean = false): MultiCompileResult {
    if (!this.wasm || !this.wasm.compile_multi_to_omniview) {
      return {
        success: false,
        files: {},
        errors: ['compile_multi_to_omniview not available in WASM'],
      };
    }
    try {
      const options = JSON.stringify({ editor_mode: editorMode });
      const json = this.wasm.compile_multi_to_omniview(JSON.stringify(sources), options);
      return JSON.parse(json) as MultiCompileResult;
    } catch (e) {
      return {
        success: false,
        files: {},
        errors: [`Multi compile error: ${e}`],
      };
    }
  }
}

// Singleton
export const rustCompiler = new RustCompiler();
export type { CompileResult, OmniViewResult, OmniViewFullResult, MultiCompileResult, ComponentCompileResult, Diagnostic, SourceMap };
