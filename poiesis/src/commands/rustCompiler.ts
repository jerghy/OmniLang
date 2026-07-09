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
  components?: Record<string, { js: string; html: string; css: string }>;
  sourceMap?: SourceMap;
  errors?: Diagnostic[];
};

type IRProgram = {
  version: string;
  modules: Record<string, {
    components: Record<string, unknown>;
    stores: Record<string, unknown>;
    functions: unknown[];
    structs: unknown[];
    type_aliases: unknown[];
    imports: unknown[];
    exports: unknown[];
    motions: unknown[];
    timelines: unknown[];
  }>;
  /// IR-level source map (keyed by node/module). Distinct from the flat
  /// codegen SourceMap array — see SourceMap type below.
  source_map: Record<string, SourceMapEntry>;
  stdlib_imports: string[];
};

type Diagnostic = {
  severity: 'error' | 'warning' | 'info';
  code: string;
  message: string;
  line: number;
  col: number;
  end_line: number;
  end_col: number;
  suggestion?: string;
};

// SourceMap is a flat array — Rust serializes Vec<SourceMapEntry> directly,
// NOT wrapped in { entries: [...] }.
type SourceMap = SourceMapEntry[];

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

type ResolvedElementStyle = {
  path: string;
  tag: string;
  classes: string[];
  computed_css: Record<string, string>;
  html_attrs: Record<string, string>;
  events: Record<string, string>;
  bindings: Record<string, string>;
  inline_style: Record<string, string> | null;
  children: string[];
  source_line: number;
};

type DesignSummary = {
  name: string;
  tokens: Array<{ name: string; token_type: string; value: string }>;
  classes: Array<{ name: string; properties: Record<string, string> }>;
  variants: Array<{
    name: string;
    dimensions: Array<{ name: string; values: string[] }>;
    defaults: Record<string, string>;
  }>;
};

type StyleResolveResult = {
  success: boolean;
  components: Record<string, ResolvedElementStyle[]>;
  design: DesignSummary | null;
  errors: Diagnostic[];
};

type MultiCompileDiagnostic = {
  severity: 'error' | 'warning' | 'info';
  code: string;
  message: string;
  file?: string | null;
  line: number;
  col: number;
  end_line: number;
  end_col: number;
  suggestion?: string | null;
};

type MultiCompileResult = {
  success: boolean;
  files: Record<string, string>;
  errors?: MultiCompileDiagnostic[];
  warnings?: MultiCompileDiagnostic[];
};

class RustCompiler {
  private wasm: {
    compile_to_ir: (source: string) => any;
    compile_to_omniview: (source: string, editor_mode: boolean) => any;
    compile_omniview_component?: (source: string, component_name: string, editor_mode: boolean) => any;
    version: () => string;
    compile_to_omniview_full?: (source: string, editor_mode: boolean) => any;
    compile_multi_to_omniview?: (sources: Record<string, string>, editor_mode: boolean) => any;
    resolve_styles?: (source: string) => any;
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
        // Slice the backing ArrayBuffer to the exact byte range — a
        // Node.js Buffer may be a view into a larger pooled ArrayBuffer.
        const exactBuffer = wasmBytes.buffer.slice(
          wasmBytes.byteOffset,
          wasmBytes.byteOffset + wasmBytes.byteLength,
        );
        wasmModule.initSync(exactBuffer);
      }

      this.wasm = {
        compile_to_ir: wasmModule.compile_to_ir as unknown as (source: string) => any,
        compile_to_omniview: wasmModule.compile_to_omniview as unknown as (source: string, editor_mode: boolean) => any,
        compile_omniview_component: wasmModule.compile_omniview_component as unknown as (source: string, component_name: string, editor_mode: boolean) => any,
        version: wasmModule.version as unknown as () => string,
        compile_to_omniview_full: wasmModule.compile_to_omniview_full as unknown as (source: string, editor_mode: boolean) => any,
        compile_multi_to_omniview: wasmModule.compile_multi_to_omniview as unknown as (sources: Record<string, string>, editor_mode: boolean) => any,
        resolve_styles: wasmModule.resolve_styles as unknown as (source: string) => any,
      };
      this.ready = true;

      log.info(
        `Loaded omni-compiler v${this.wasm!.version()}`,
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
      return this.wasm.compile_to_ir(source) as CompileResult;
    } catch (e) {
      log.error('compileToIR failed:', e);
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
      return this.wasm.compile_to_omniview(source, editorMode) as OmniViewResult;
    } catch (e) {
      log.error('compileToOmniView failed:', e);
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
      return this.wasm.compile_omniview_component(source, componentName, editorMode) as ComponentCompileResult;
    } catch (e) {
      log.error('compileOmniViewComponent failed:', e);
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
    } catch (e) {
      log.error("WASM version() failed:", e);
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
      return this.wasm.compile_to_omniview_full(source, editorMode) as OmniViewFullResult;
    } catch (e) {
      log.error('compileToOmniViewFull failed:', e);
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
        errors: [{ severity: 'error' as const, code: 'WASM', message: 'compile_multi_to_omniview not available in WASM', file: null, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      return this.wasm.compile_multi_to_omniview(sources, editorMode) as MultiCompileResult;
    } catch (e) {
      log.error('compileMultiToOmniView failed:', e);
      return {
        success: false,
        files: {},
        errors: [{ severity: 'error' as const, code: 'WASM', message: `Multi compile error: ${e}`, file: null, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
  }

  resolveStyles(source: string): StyleResolveResult {
    if (!this.wasm || !this.wasm.resolve_styles) {
      return {
        success: false,
        components: {},
        design: null,
        errors: [{ severity: 'error', code: 'WASM', message: 'resolve_styles not available in WASM', line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      return this.wasm.resolve_styles(source) as StyleResolveResult;
    } catch (e) {
      log.error('resolveStyles failed:', e);
      return {
        success: false,
        components: {},
        design: null,
        errors: [{ severity: 'error', code: 'WASM', message: `resolve_styles error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
  }
}

// Singleton
export const rustCompiler = new RustCompiler();
export type { CompileResult, OmniViewResult, OmniViewFullResult, MultiCompileResult, MultiCompileDiagnostic, ComponentCompileResult, Diagnostic, SourceMap, StyleResolveResult, ResolvedElementStyle, DesignSummary };
