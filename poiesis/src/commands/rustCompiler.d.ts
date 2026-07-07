// Type declarations for the WASM-compiled Omni compiler.
// Resolved by Vite at build time.

declare module '../../public/wasm/omnilang.js' {
  const init: (module?: WebAssembly.Module) => Promise<void>;
  export function initSync(buffer: ArrayBuffer): void;
  export function compile_to_ir(source: string): any;
  export function compile_to_omniview(source: string, editor_mode: boolean): any;
  export function compile_omniview_component(
    source: string,
    component_name: string,
    editor_mode: boolean,
  ): any;
  export function compile_to_omniview_full(source: string, editor_mode: boolean): any;
  export function compile_multi_to_omniview(sources: Record<string, string>, editor_mode: boolean): any;
  export class OmniCompiler {
    constructor();
    upsert_source(path: string, source: string): void;
    remove_source(path: string): void;
    compile_to_omniview(editor_mode: boolean): any;
    diagnostics(): any;
    version(): string;
  }
  export function version(): string;
  export default init;
}
