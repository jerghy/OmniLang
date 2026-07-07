/// <reference types="vite/client" />

declare module '*.css' {
  const content: string;
  export default content;
}

// WASM compiler module - resolved by Vite at build time
declare module '*/omni_compiler.js' {
  const init: (module?: WebAssembly.Module) => Promise<void>;
  export function initSync(buffer: ArrayBuffer): void;
  export function compile_to_ir(source: string): any;
  export function compile_to_omniview(source: string, editor_mode: boolean): any;
  export function compile_omniview_component(source: string, component_name: string, editor_mode: boolean): any;
  export function version(): string;
  export default init;
}

// Also match relative paths like ../../../public/wasm/omni_compiler.js
declare module '.pnpm/omni_compiler.js' {
  export * from '*/omni_compiler.js';
}
