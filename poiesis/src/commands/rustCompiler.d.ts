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
  export function version(): string;
  export default init;
}
