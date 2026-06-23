/* tslint:disable */
/* eslint-disable */

/**
 * Compile .omni source to IR JSON. Returns JSON string.
 */
export function compile_to_ir(source: string): string;

/**
 * Compile .omni source to OmniView (JS + HTML + CSS). Returns JSON string.
 * options_json: {"editor_mode": true}
 */
export function compile_to_omniview(source: string, options_json: string): string;

/**
 * Return compiler version.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile_to_ir: (a: number, b: number, c: number) => void;
    readonly compile_to_omniview: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly omni_analyse: (a: number, b: number) => number;
    readonly omni_compile_multi_to_target: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly omni_free_string: (a: number) => void;
    readonly omni_version: () => number;
    readonly version: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
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
