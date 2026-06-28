/// <reference types="vite/client" />

declare module '*.css' {
  const content: string;
  export default content;
}

// WASM compiler module - resolved by Vite at build time
declare module '*/omni_compiler.js' {
  const init: (module?: WebAssembly.Module) => Promise<void>;
  export function initSync(buffer: ArrayBuffer): void;
  export function compile_to_ir(source: string): string;
  export function compile_to_omniview(source: string, options_json: string): string;
  export function compile_omniview_component(source: string, component_name: string, options_json: string): string;
  export function version(): string;
  export default init;
}

// Also match relative paths like ../../../public/wasm/omni_compiler.js
declare module '.pnpm/omni_compiler.js' {
  export * from '*/omni_compiler.js';
}
