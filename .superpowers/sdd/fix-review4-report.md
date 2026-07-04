# Round 4 Code Review Findings - Fix Report

All 16 issues resolved. TypeScript check: 0 errors. Vite build: success. Store size: 571 lines.

## High (H1-H4)

| Issue | Description | What Was Done | Status |
|-------|------------|---------------|--------|
| **H1** | Split store/index.ts (894 lines) | Created `store/undo-slice.ts` (UndoAction, UndoStack, skipUndoGuard closure, buildUndoUpdate helper, createUndoSlice factory). Created `store/tab-slice.ts` (openTab, closeTab, switchTab, reorderTabs, markTabDirty via createTabSlice factory). Refactored `store/index.ts` to import and merge both slices. Line count: 894 -> 571. | Done |
| **H2** | Remove stale TODO on pipeline.ts:1 | Removed the `// TODO(poiesis-v2): remove @ts-nocheck` line (the @ts-nocheck was already removed). Also updated the stale path comment. | Done |
| **H3** | __skipUndo hardening | Encapsulated `__skipUndo` inside `createSkipUndoGuard()` closure in `undo-slice.ts`. Exported `skipUndoGuard` with `isSkipped()` and `runWithoutUndo()` (try/finally pattern). CRUD operations use `buildUndoUpdate()` which internally checks `skipUndoGuard.isSkipped()`. | Done |
| **H4** | ArchitectureGraph useEffect cleanup | Added `cancelled` ref pattern: `let cancelled = false; ... if (!cancelled) { ... } return () => { cancelled = true; }`. | Done |

## Medium (M5-M9)

| Issue | Description | What Was Done | Status |
|-------|------------|---------------|--------|
| **M5** | Replace inline import() with import type | Updated `pipeline-types.ts`, `pipeline.ts`, and `types.ts`. Replaced `import("../../core/types/pid").PidMap` with top-level `import type { PidMap, PidBlockType } from "../../core/types/pid"`. | Done |
| **M6** | Move tagMap to module-level const | Moved `tagMap` from inside `resolveTag()` function body to module-level `const` in `OmniViewRenderer.tsx`. | Done |
| **M7** | Unify defaultValue/default naming | Changed `default?: unknown` to `defaultValue?: unknown` in all inline type definitions in `types.ts`, `pipeline.ts`, `store-extractor.ts`, `parser-types.ts`. Updated all property accesses: `source-emitter.ts`, `store-extractor.ts`, `pipeline.ts`, `component-service.ts`, `parser.ts`. | Done |
| **M8** | Remove duplicated JSDoc in bubble-data.ts | Removed orphaned 4-line JSDoc block before `extractViewBlock` function (lines 161-164 in original). | Done |
| **M9** | Fix broken type assertion in ArchitectureGraph | Replaced `as unknown as Edge[]` with proper `: Edge` return type annotation on the map callback. Replaced `as unknown as Record<string, unknown>` with `as unknown as Record<string, unknown>` (kept for BubbleEdge which lacks index signature). | Done |

## Low (M10-M16)

| Issue | Description | What Was Done | Status |
|-------|------------|---------------|--------|
| **M10** | Extract magic position constants | Created `BUBBLE_LAYOUT = { xBase: 120, xSpread: 140, yBase: 180, yStep: 120 } as const` at module level in `bubble-data.ts`. | Done |
| **M11** | Extract "未命名项目" to constant | Created `DEFAULT_PROJECT_NAME = "未命名项目"` in `src/constants.ts`. Updated `store/index.ts` and `App.tsx` to import and use it. | Done |
| **M12** | Extract panel layout to constant | Added `DEFAULT_PANEL_LAYOUT = { leftWidth: 240, rightWidth: 300 } as const` to `src/constants.ts`. Updated `store/index.ts` (both initial state and `newProject()`). | Done |
| **M13** | pipeline.ts unnecessary any | Replaced `const registryDef: any = ...` with `const registryDef: unknown = ...` + proper narrowing via `Record<string, unknown>` accessors with `in` checks. | Done |
| **M14** | Window polyfill safety | Added `typeof (window as unknown as Record<string, unknown>).__ZUSTAND_STORE__ === "undefined"` check before assigning. Cast through `unknown` for type safety. | Done |
| **M15** | Update stale source path comments | Updated `pipeline-types.ts:1`, `pipeline.ts:2` (via H2), `annotation-data.ts:1`, `types.ts:1` from old `editor/client/src/` paths to `poiesis/src/services/codegen/`. | Done |
| **M16** | Document xyflow type casts | Added comment: `// @xyflow/react v12 uses broad Node/Edge types; cast to our specific component types` before nodeTypes/edgeTypes in `ArchitectureGraph.tsx`. | Done |

## Verification

```bash
$ npx tsc --noEmit --pretty false  # → 0 errors
$ npx vite build                    # → built in 122ms
$ wc -l src/store/index.ts          # → 571 lines
```

All 16 issues fixed. Zero type errors. Build passes.
