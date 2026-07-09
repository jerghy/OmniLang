/* tslint:disable */
/* eslint-disable */

/**
 * Stateful compiler instance that caches sources and compilation results across calls.
 */
export class OmniCompiler {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Compile all registered sources to OmniView. Uses cached result when available.
     * editor_mode: enable debug wrappers for editor preview.
     */
    compile_to_omniview(editor_mode: boolean): any;
    /**
     * Run diagnostics on all registered sources without compiling to output.
     * Returns JsValue with `success` and `errors` arrays.
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
 * Compile multiple .omni source files to OmniView.
 * sources: JS object mapping file path to source content, e.g. {"path": "source content", ...}
 * editor_mode: enable debug wrappers for editor preview.
 * Returns JsValue with `success`, `files`, `errors`, and `warnings`.
 */
export function compile_multi_to_omniview(sources: any, editor_mode: boolean): any;

/**
 * Compile a single .omni component to OmniView output. Returns JsValue.
 * The source should contain the full .omni file; component_name selects which component to extract.
 * editor_mode: enable debug wrappers for editor preview.
 */
export function compile_omniview_component(source: string, component_name: string, editor_mode: boolean): any;

/**
 * Compile .omni source to IR JSON. Returns JsValue.
 */
export function compile_to_ir(source: string): any;

/**
 * Compile .omni source to OmniView (JS + HTML + CSS). Returns JsValue.
 * editor_mode: enable debug wrappers for editor preview.
 */
export function compile_to_omniview(source: string, editor_mode: boolean): any;

/**
 * Compile .omni source to OmniView output in a single pass (IR + codegen).
 * Returns JsValue with `ir`, `files`, `components`, `source_map`, and `errors`.
 * editor_mode: enable debug wrappers for editor preview.
 */
export function compile_to_omniview_full(source: string, editor_mode: boolean): any;

/**
 * Resolve per-element computed styles for editor canvas rendering.
 * Returns structured JSON with component -> element -> computed CSS.
 */
export function resolve_styles(source: string): any;

/**
 * Return compiler version.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_omnicompiler_free: (a: number, b: number) => void;
    readonly compile_multi_to_omniview: (a: number, b: number) => number;
    readonly compile_omniview_component: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly compile_to_ir: (a: number, b: number) => number;
    readonly compile_to_omniview: (a: number, b: number, c: number) => number;
    readonly compile_to_omniview_full: (a: number, b: number, c: number) => number;
    readonly omnicompiler_compile_to_omniview: (a: number, b: number) => number;
    readonly omnicompiler_diagnostics: (a: number) => number;
    readonly omnicompiler_new: () => number;
    readonly omnicompiler_remove_source: (a: number, b: number, c: number) => void;
    readonly omnicompiler_upsert_source: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly omnicompiler_version: (a: number, b: number) => void;
    readonly resolve_styles: (a: number, b: number) => number;
    readonly version: (a: number) => void;
    readonly omni_analyse: (a: number, b: number) => number;
    readonly omni_compile_multi_to_target: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
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
