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

type ComponentCompileResult = {
  success: boolean;
  component?: { js: string; html: string; css: string };
  errors?: Diagnostic[];
};

class RustCompiler {
  private wasm: {
    compile_to_ir: (source: string) => string;
    compile_to_omniview: (source: string, options_json: string) => string;
    compile_omniview_component?: (source: string, component_name: string, options_json: string) => string;
    version: () => string;
  } | null = null;
  private ready = false;

  async init(): Promise<void> {
    if (this.ready) return;

    try {
      // Dynamic import of the WASM JS glue
      const wasmModule = await import(
        '../../../public/wasm/omni_compiler.js'
      );

      // Initialize WASM (calls the default export)
      await wasmModule.default();

      this.wasm = {
        compile_to_ir: wasmModule.compile_to_ir,
        compile_to_omniview: wasmModule.compile_to_omniview,
        compile_omniview_component: wasmModule.compile_omniview_component,
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
}

// Singleton
export const rustCompiler = new RustCompiler();
export type { CompileResult, OmniViewResult, ComponentCompileResult, Diagnostic, SourceMap };
