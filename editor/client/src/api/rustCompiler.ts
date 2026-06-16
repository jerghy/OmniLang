// Rust WASM compiler loader for the OmniLang editor.
// Loads the WASM-compiled omni-compiler and exposes compile_to_ir / compile_to_omniview.

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

class RustCompiler {
  private wasm: {
    compile_to_ir: (source: string) => string;
    compile_to_omniview?: (source: string) => string;
    version: () => string;
  } | null = null;
  private ready = false;

  async init(): Promise<void> {
    if (this.ready) return;

    try {
      // Dynamic import of the WASM JS glue
      const wasmModule = await import(
        /* webpackChunkName: "omni-compiler" */
        '../../../public/wasm/omni_compiler.js'
      );

      // Initialize WASM (calls the default export)
      await wasmModule.default();

      this.wasm = {
        compile_to_ir: wasmModule.compile_to_ir,
        compile_to_omniview: wasmModule.compile_to_omniview,
        version: wasmModule.version,
      };
      this.ready = true;

      console.log(
        `[RustCompiler] Loaded omni-compiler v${this.wasm.version()}`,
      );
    } catch (err) {
      console.warn('[RustCompiler] WASM not available, using Python fallback:', err);
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

  compileToOmniView(source: string): OmniViewResult {
    if (!this.wasm || !this.wasm.compile_to_omniview) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: 'OmniView codegen not available', line: 0, col: 0, end_line: 0, end_col: 0 }],
      };
    }
    try {
      const json = this.wasm.compile_to_omniview(source);
      return JSON.parse(json);
    } catch (e) {
      return {
        success: false,
        errors: [{ severity: 'error', code: 'WASM', message: `OmniView compile error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
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
}

// Singleton
export const rustCompiler = new RustCompiler();
export type { CompileResult, OmniViewResult, Diagnostic, SourceMap };
