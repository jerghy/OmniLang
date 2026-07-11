// Monaco services for OmniLang editor IntelliSense.
// Import this module to register OmniLang language support,
// snippets, type definitions, and diagnostic integration.

export { initOmniLangLanguage } from "./omnilang-language";
export {
  injectOmniLangTypes,
  generateStoreTypes,
  OMNILANG_RUNTIME_DTS,
} from "./omnilang-types";
export { initOmniLangSnippets, disposeSnippets } from "./omnilang-snippets";
export {
  getOmniCompiler,
  runDiagnostics,
  wasmDiagnosticToMarker,
  applyDiagnostics,
} from "./diagnostics";
export type { WasmDiagnostic } from "./diagnostics";
