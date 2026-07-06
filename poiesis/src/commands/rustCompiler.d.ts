// Type declarations for the WASM-compiled Omni compiler.
// Resolved by Vite at build time.

declare module '../../public/wasm/omnilang.js' {
  const init: (module?: WebAssembly.Module) => Promise<void>;
  export function initSync(buffer: ArrayBuffer): void;
  export function compile_to_ir(source: string): string;
  export function compile_to_omniview(source: string, options_json: string): string;
  export function compile_omniview_component(
    source: string,
    component_name: string,
    options_json: string,
  ): string;
  export function compile_to_omniview_full(source: string, options_json: string): string;
  export function compile_multi_to_omniview(sources_json: string, options_json: string): string;
  export class OmniCompiler {
    constructor();
    upsert_source(path: string, source: string): void;
    remove_source(path: string): void;
    compile_to_omniview(options_json: string): string;
    diagnostics(): string;
    version(): string;
  }
  export function version(): string;
  export default init;
}
