// Monaco services barrel export
export { getOmniCompiler, runDiagnostics, applyDiagnostics, validateSources } from "./diagnostics";
export { useMonacoEditor } from "./useMonacoEditor";
export type { UseMonacoEditorConfig, UseMonacoEditorResult } from "./useMonacoEditor";
export { initOmniLangLanguage } from "./omnilang-language";
export { initOmniLangSnippets } from "./omnilang-snippets";
export { injectOmniLangTypes } from "./omnilang-types";
