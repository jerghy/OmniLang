/* tslint:disable */
/* eslint-disable */

/**
 * Stateful compiler instance that caches sources and compilation results across calls.
 */
export class OmniCompiler {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Run semantic analysis on all registered sources and return
     * symbols + dependencies in addition to diagnostics.
     * Returns JsValue with { symbols, dependencies, diagnostics }.
     *
     * Routes through the multi-file analysis path (cross-file aware
     * diagnostics). Symbols/dependencies are extracted from the pre-parsed
     * programs via a per-file semantic pass — parsing, cross-file
     * validation, IR lowering and codegen all run once in the shared
     * multi-file path, not per-file in isolation. Lint/merge warnings are
     * folded into `diagnostics` like the pre-refactor single-file path.
     */
    analyse_dependencies(): any;
    /**
     * Compile all registered sources to OmniView. Uses cached result when available.
     * editor_mode: enable debug wrappers for editor preview.
     */
    compile_to_omniview(editor_mode: boolean, debug_mode: boolean): any;
    /**
     * Run diagnostics on all registered sources without compiling to output.
     * Returns JsValue with `success` and `errors` arrays (both error- and
     * warning-severity diagnostics, matching the pre-refactor single-file
     * path where lint warnings folded into `compile().errors`).
     *
     * Routes through the multi-file analysis path (`analyse_multi_cached`)
     * so diagnostics are cross-file aware (import errors, cross-file type
     * mismatches) instead of per-file isolated compiles. Diagnostics cross
     * the boundary in the unified `DiagnosticJson` shape.
     */
    diagnostics(): any;
    /**
     * Create a new OmniCompiler with no source files registered.
     */
    constructor();
    /**
     * Remove a source file. Invalidates cached compilation results.
     */
    remove_source(path: string): void;
    /**
     * Add or update a source file. Invalidates cached compilation results.
     */
    upsert_source(path: string, source: string): void;
    /**
     * Return the compiler version string.
     */
    version(): string;
}

/**
 * Multi-file analysis with incremental cache.
 *
 * `sources`: JS object `{ "file/path.omni": "source content", ... }`
 * `path_mapping`: JS object `{ "import/path": "resolved/file/path.omni", ... }`
 * `cache_state`: serialized cache from previous call (or null/undefined)
 *
 * Returns `{ success, errors, warnings, cache_state }`.
 */
export function analyse_multi_cached(sources: any, path_mapping: any, cache_state: any): any;

/**
 * Compile multiple .omni source files to OmniView with incremental cache.
 *
 * `sources`: JS object mapping file path to source content, e.g. {"path": "source content", ...}
 * `editor_mode`: enable debug wrappers for editor preview.
 * `cache_state`: serialized incremental cache from previous call (or null/undefined).
 *
 * Returns `{ result: InternalCompileResult, cache_state: string }`.
 */
export function compile_multi_to_omniview(sources: any, editor_mode: boolean, debug_mode: boolean, cache_state: any): any;

/**
 * Compile a single .omni component to OmniView output. Returns JsValue.
 * The source should contain the full .omni file; component_name selects which component to extract.
 * editor_mode: enable debug wrappers for editor preview.
 */
export function compile_omniview_component(source: string, component_name: string, editor_mode: boolean, debug_mode: boolean): any;

/**
 * Compile .omni source to IR JSON. Returns JsValue.
 */
export function compile_to_ir(source: string): any;

/**
 * Compile .omni source to OmniView (JS + HTML + CSS). Returns JsValue.
 * editor_mode: enable debug wrappers for editor preview.
 */
export function compile_to_omniview(source: string, editor_mode: boolean, debug_mode: boolean): any;

/**
 * Compile .omni source to OmniView output in a single pass (IR + codegen).
 * Returns JsValue with `ir`, `files`, `components`, `source_map`, and `errors`.
 * editor_mode: enable debug wrappers for editor preview.
 */
export function compile_to_omniview_full(source: string, editor_mode: boolean, debug_mode: boolean): any;

/**
 * Parse a code-block source string into structured AST statements.
 * Returns a JsValue with `{ success, stmts, errors }`.
 */
export function parse_code_block(source: string): any;

/**
 * Parse complete .omni source text (lex + parse in one call).
 *
 * Returns `{ success: bool, program: Program | null, errors: ParseError[] }` as JsValue.
 */
export function parse_source(source: string): any;

/**
 * Resolve per-element computed styles for editor canvas rendering.
 * Returns structured JSON with component -> element -> computed CSS.
 */
export function resolve_styles(source: string): any;

/**
 * Resolve styles for multiple .omni source files (cross-file design imports).
 *
 * `sources`: JS object `{ "file/path.omni": "source content", ... }`
 * `path_mapping`: JS object `{ "import/path": "resolved/file/path.omni", ... }`
 *
 * Returns `StyleResolveResult` with compiled CSS + element mappings.
 */
export function resolve_styles_multi(sources: any, path_mapping: any): any;

/**
 * Resolve styles for multiple .omni source files using incremental cache.
 *
 * Same interface as `resolve_styles_multi` but accepts `cache_state` and
 * returns `{ result: StyleResolveResult, cache_state: string }`.
 */
export function resolve_styles_multi_cached(sources: any, path_mapping: any, cache_state: any): any;

/**
 * Serialize the thread-local analysis cache to a JSON string for explicit persistence.
 *
 * Call this from JS on `beforeunload` to preserve the incremental cache across
 * page navigations. Returns the full cached state as a JSON string, or empty
 * string on serialization failure.
 *
 * The cache is NOT serialized automatically on every analysis call — it lives
 * in WASM memory for the page lifetime. Use this function only when you need
 * to persist the cache externally (session storage, beforeunload, etc.).
 */
export function save_cache_state(): any;

/**
 * Return compiler version.
 */
export function version(): string;

/**
 * Wrap generated OmniView output in a standalone debug-ready HTML document.
 */
export function wrap_debug_html(js: string, css: string, body_html: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_omnicompiler_free: (a: number, b: number) => void;
    readonly analyse_multi_cached: (a: number, b: number, c: number) => number;
    readonly compile_multi_to_omniview: (a: number, b: number, c: number, d: number) => number;
    readonly compile_omniview_component: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly compile_to_ir: (a: number, b: number) => number;
    readonly compile_to_omniview: (a: number, b: number, c: number, d: number) => number;
    readonly compile_to_omniview_full: (a: number, b: number, c: number, d: number) => number;
    readonly omnicompiler_analyse_dependencies: (a: number) => number;
    readonly omnicompiler_compile_to_omniview: (a: number, b: number, c: number) => number;
    readonly omnicompiler_diagnostics: (a: number) => number;
    readonly omnicompiler_new: () => number;
    readonly omnicompiler_remove_source: (a: number, b: number, c: number) => void;
    readonly omnicompiler_upsert_source: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly omnicompiler_version: (a: number, b: number) => void;
    readonly parse_code_block: (a: number, b: number) => number;
    readonly parse_source: (a: number, b: number) => number;
    readonly resolve_styles: (a: number, b: number) => number;
    readonly resolve_styles_multi: (a: number, b: number) => number;
    readonly resolve_styles_multi_cached: (a: number, b: number, c: number) => number;
    readonly save_cache_state: () => number;
    readonly version: (a: number) => void;
    readonly wrap_debug_html: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly omni_compile_multi_to_target: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
    readonly omni_analyse: (a: number, b: number) => number;
    readonly omni_free_string: (a: number) => void;
    readonly omni_version: () => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
