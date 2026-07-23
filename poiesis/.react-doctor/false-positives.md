# react-doctor `unused-file` False Positives

Generated: 2026-07-23

## Summary

- **Total files flagged by react-doctor:** 61
- **True false positives (files with import references):** 33
- **Truly unused files (no import references):** 28

## False Positives

These 33 files are imported by other source files and should NOT be deleted.

| # | File | Importer(s) | Notes |
|---|------|-------------|-------|
| 1 | `src/commands/__generated__/CompileResult.ts` | rustCompiler.d.ts | Type generated from Rust compiler |
| 2 | `src/commands/__generated__/Diagnostic.ts` | rustCompiler.d.ts, CompileResult.ts | Type generated from Rust compiler |
| 3 | `src/commands/__generated__/Severity.ts` | Diagnostic.ts | Type generated from Rust compiler |
| 4 | `src/components/canvas/shared/connectionValidators.ts` | ConnectionDriver.ts, driver/store.ts | Shared canvas utility |
| 5 | `src/components/canvas/shared/layoutUtils.ts` | useProjectInit.ts | Auto-layout utility |
| 6 | `src/components/canvas/shared/nodeRegistry.ts` | CanvasDriver.ts, OmniDriver.ts | Node configuration registry |
| 7 | `src/components/panels/css-panel/FormulaValueInput.tsx` | CssStylePanel.tsx | Component sub-module |
| 8 | `src/core/utils/omni.ts` | core/utils/index.ts, useCompile.ts | Source parsing utility |
| 9 | `src/driver/CanvasDriver.ts` | OmniDriver.ts | Driver sub-module |
| 10 | `src/driver/CodeDriver.ts` | OmniDriver.ts | Driver sub-module |
| 11 | `src/driver/ConnectionDriver.ts` | ElementHandle.ts, OmniDriver.ts | Driver sub-module |
| 12 | `src/driver/ElementHandle.ts` | 7 driver files | Driver sub-module |
| 13 | `src/driver/FileDriver.ts` | OmniDriver.ts | Driver sub-module |
| 14 | `src/driver/OmniDriver.ts` | driver/index.ts, SnippetService.ts | Driver facade |
| 15 | `src/driver/SnippetService.ts` | OmniDriver.ts | Driver sub-module |
| 16 | `src/driver/StateDriver.ts` | OmniDriver.ts | Driver sub-module |
| 17 | `src/driver/VisualService.ts` | OmniDriver.ts | Driver sub-module |
| 18 | `src/driver/WireHandle.ts` | ConnectionDriver.ts, ElementHandle.ts, OmniDriver.ts, VisualService.ts | Driver sub-module |
| 19 | `src/driver/store.ts` | 9 driver sub-modules | Driver internal store |
| 20 | `src/driver/types.ts` | 8 driver files + omni-driver.d.ts | Driver type definitions |
| 21 | `src/lib/constants/style-properties.ts` | ComponentPalette.tsx, CssStylePanel.tsx, and many others | Shared style constants |
| 22 | `src/services/codegen/token-emitter.ts` | codegen/index.ts | Codegen sub-module |
| 23 | `src/services/codegen/validator.ts` | codegen/index.ts, source-parser.ts | Codegen sub-module |
| 24 | `src/services/codegraph/store.ts` | CodeDriver.ts, useCompile.ts, useProjectInit.ts, source-parser.ts, project-service.ts | Code graph store |
| 25 | `src/services/compiler/cache.ts` | useCompile.ts | Compiler cache |
| 26 | `src/services/compiler/compiler.ts` | useCompile.ts | Compiler orchestration |
| 27 | `src/services/parser/omni-regex-parser.ts` | OmniDriver.ts, useCompile.ts | Regex-based parser |
| 28 | `src/services/parser/search-index.ts` | index-loader.ts | Search index |
| 29 | `src/services/parser/source-parser.ts` | useAutoCompile.ts | Source parser |
| 30 | `src/services/project/component-service.ts` | useProjectInit.ts, project-service.ts | Component def service |
| 31 | `src/services/runtime/debug-service.ts` | useDebugSession.ts | Debug service |
| 32 | `src/services/runtime/run-service.ts` | useRun.ts | Run service |
| 33 | `src/snippets/builtin.ts` | search-index.ts | Built-in snippets |

## Truly Unused Files (for deletion consideration)

These 28 files have zero import references. **Note:** Many of these represent
dead code that was replaced by newer implementations (e.g., the driver module,
old hooks, legacy barrel files).

| # | File | Reason for unuse |
|---|------|-----------------|
| 1 | `src/components/canvas/shared/edges/DataEdge.tsx` | Edge component — never rendered |
| 2 | `src/components/canvas/shared/flowPortGenerator.ts` | Flow port generator — never called |
| 3 | `src/components/canvas/shared/socketTypes.ts` | Socket type definitions — never imported |
| 4 | `src/components/design/render-utils.ts` | Render utilities — replaced by @omni/animation runtime? |
| 5 | `src/components/panels/CssStylePanel.tsx` | CSS style panel — never rendered in UI |
| 6 | `src/components/panels/inspector/StylesSection.tsx` | Inspector style section — never rendered |
| 7 | `src/components/shell/Toolbar.tsx` | App toolbar — never rendered |
| 8 | `src/core/state/styleRegistry.ts` | Legacy style registry — replaced by unified store |
| 9 | `src/core/types/project.ts` | ProjectConfig type — never used |
| 10 | `src/driver/index.ts` | Driver barrel — no external consumers |
| 11 | `src/features/compiler/useAutoCompile.ts` | Auto-compile hook — never called |
| 12 | `src/features/compiler/useCompile.ts` | Compile hook — never called |
| 13 | `src/features/compiler/useRun.ts` | Run hook — never called |
| 14 | `src/features/debug/useDebugSession.ts` | Debug session hook — never called |
| 15 | `src/features/useProjectInit.ts` | Project init hook — never called |
| 16 | `src/hooks/useBreadcrumb.ts` | Breadcrumb hook — never called |
| 17 | `src/hooks/useBubbleExpand.ts` | Bubble expand hook — never called |
| 18 | `src/hooks/useDebugBridge.ts` | Debug bridge hook — never called |
| 19 | `src/hooks/useLiveCodePreview.ts` | Live preview hook — never called |
| 20 | `src/hooks/useOmniViewRender.ts` | OmniView render hook — never called |
| 21 | `src/hooks/useZAxisCanvas.ts` | Z-axis canvas hook — never called |
| 22 | `src/services/codegen/expand-custom-components.ts` | Custom component expander — never called |
| 23 | `src/services/codegen/index.ts` | Codegen barrel — files import from sub-modules directly, not through barrel |
| 24 | `src/services/monaco/index.ts` | Monaco editor integration — never initialized |
| 25 | `src/services/parser/index-loader.ts` | Index loader — never called |
| 26 | `src/services/persistence/index.ts` | Persistence barrel — files import from sub-modules directly |
| 27 | `src/services/pid/index.ts` | PID barrel — files import from sub-modules directly |
| 28 | `src/services/project/project-service.ts` | Project service — never called |

## Notes

- The driver module (`src/driver/`) is internally connected (all files import
  from each other) but has NO external consumers. The entire module appears to
  be dead code that was never wired into the app.
- Several hooks and feature files (`useCompile.ts`, `useRun.ts`, `useProjectInit.ts`,
  etc.) export React hooks but nothing imports them. Their implementations
  may have moved to other locations.
- Barrel files (`codegen/index.ts`, `persistence/index.ts`, `pid/index.ts`,
  `monaco/index.ts`, `driver/index.ts`) are unused because other files import
  from the specific sub-modules directly instead of using the barrel.
- Some files marked as false positives (e.g., those only imported by `useCompile.ts`,
  which is itself unused) are transitively dead code. However, they DO have
  import references, so `react-doctor`'s `unused-file` rule is technically
  incorrect about them.
