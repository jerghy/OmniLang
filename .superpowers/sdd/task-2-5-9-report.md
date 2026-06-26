# Tasks 2.5-2.9 Report: Service Extraction

## Summary

Split the monolithic `api/client.ts` into domain-specific modules and extracted compiler, project, runtime, and parser services from `App.tsx`. TypeScript compiles clean (`tsc --noEmit` = zero errors), tests match baseline (190/192 pass, 2 pre-existing failures in `services/codegen/`).

## Files Created

### Task 2.5: API Domain Modules (`services/api/`)

| File | Description |
|------|-------------|
| `compile-api.ts` | `createCompileApi` factory — `compile()`, `startRun()`, `stopRun()`, `getRunStatus()`, `writeRunFiles()` |
| `project-api.ts` | `createProjectApi` factory — `listDir()`, `readFile()`, `writeFile()`, `getProject()`, `listDirectory()`, `setProjectDir()`, `createProject()` |
| `annotation-api.ts` | `createAnnotationApi` factory — `saveComponentDefinitions()`, `loadComponentDefinitions()`, `deleteComponentDefinitions()`, `annotatedSave()`, `annotatedLoad()`, `saveEditorToml()`, `loadEditorToml()` |
| `index.ts` | Composite `ApiClient` class merging domain factories + WebSocket methods (watch/debug). Exports singleton `api`. |

**Consumers updated (7 files):** App.tsx, OmniDriver.ts, CanvasTabBar.tsx, CanvasPanel.tsx, FileExplorer.tsx, ProjectWelcome.tsx, ComponentPalette.tsx

### Task 2.6: Compiler Service (`services/compiler/`)

| File | Description |
|------|-------------|
| `compiler.ts` | `compileProject()` — wraps generateOmniSource → validate → api.compile.compile flow. Returns `{content, compileResponse, generatedSource, validationPassed, validationErrors}`. |
| `cache.ts` | Moved from `src/omni-compiler-cache.ts` via `git mv`. Updated relative imports. |

### Task 2.7: Project Service (`services/project/`)

| File | Description |
|------|-------------|
| `project-service.ts` | `saveProject()` + `saveFileEntry()` (extracted 1:1 from App.tsx), `saveComponentDefinitionsToServer()`, `loadComponentDefinitionsFromServer()` |

### Task 2.8: Runtime Service (`services/runtime/`)

| File | Description |
|------|-------------|
| `run-service.ts` | `handleRun()` + `handleStop()` — compile → startRun / stopRun lifecycle |
| `debug-service.ts` | `handleDebugMessage()` — dispatch state-change, execution-trace, runtime-ready/paused/resumed/disconnected |

### Task 2.9: Parser Service (`services/parser/`)

| File | Description |
|------|-------------|
| `source-parser.ts` | `parseAndPopulate()` — validate → parseOmniSource → populate editor store |
| `search-index.ts` | Moved from `src/search/SearchIndex.ts` via `git mv` |
| `index-loader.ts` | Moved from `src/search/indexLoader.ts` via `git mv` |

## App.tsx Changes

- Removed ~100 lines of module-level functions (`saveComponentDefinitionsToServer`, `loadComponentDefinitionsFromServer`) → imported from `project-service`
- `handleCompile` (~135 lines) → delegates core generate/validate/compile to `compileProject()` from `compiler.ts`. Post-compile parsing/fnToNodeMap/WASM preview stays in App.tsx.
- `saveFileEntry` + `saveProject` (~160 lines) → delegated to `saveProjectToServer()` from `project-service.ts`
- `handleRun` + `handleStop` (~100 lines) → delegated to `run-service.ts`
- `handleDebugMessage` (~70 lines) → delegated to `debug-service.ts`
- Parse useEffect (~25 lines) → delegated to `parseAndPopulate()` from `source-parser.ts`

## Import Path Changes

All `import { api } from "./api/client"` → `import { api } from "./services/api"`. Method calls updated: `api.compile()` → `api.compile.compile()`, `api.listDir()` → `api.project.listDir()`, etc. WebSocket methods (`connectWatch`, `disconnect`, `connectDebugWs`, `disconnectDebug`, `sendDebugCommand`) remain directly on `api`.
