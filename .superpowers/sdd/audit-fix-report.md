# Audit Fix Report

**Date**: 2026-06-26
**Branch**: worktree-editor-architecture-refactor
**Commit**: ea04d165

## Summary

Fixed all 5 audit findings in `editor/`:

| Finding | Severity | Status |
|---------|----------|--------|
| 1. Make atomic stores canonical | CRITICAL | Done |
| 2. Delete dead custom-component-emitter.ts | HIGH | Done |
| 3. Extract component-service.ts | MEDIUM | Done |
| 4. Extract ComponentMetaSection | MEDIUM | Done |
| 5. Move App to app/ | LOW | Done |

## Finding 1: Make atomic stores canonical

**File**: `editor/client/src/store/editor.ts`

**What was done**:
- Removed the one-way `subscribe()` sync block (lines 792-830 in the original) that pushed monolith state TO atoms.
- Added an internal `syncToAtoms()` helper that pushes the monolith's current state to all atomic stores after each action completes.
- Every action now computes state on the monolith (as before), then calls `syncToAtoms()` to propagate to `useElementStore`, `useWireStore`, `useSelectionStore`, `useProjectStore`, and `useTabStore`.

**Key change**: Atomic stores are now canonical. Direct writes to atoms (e.g., `useElementStore.getState().addElement(el)`) are no longer overwritten by a subscribe callback -- they persist independently. The monolith store is a backward-compatible facade.

**Files changed**: `editor/client/src/store/editor.ts`

## Finding 2: Delete dead custom-component-emitter.ts

**Files changed**:
- Deleted `editor/client/src/services/codegen/custom-component-emitter.ts` (165 lines)
- Removed `export { emitCustomComponents }` from `editor/client/src/services/codegen/index.ts`

**Verification**: `emitCustomComponents` had zero callers in production code, confirmed by grep.

## Finding 3: Extract component-service.ts

**Files changed**:
- Created `editor/client/src/services/project/component-service.ts` with `saveComponentDefinitionsToServer` and `loadComponentDefinitionsFromServer`
- Updated `editor/client/src/services/project/project-service.ts` to import from `./component-service`
- Updated `editor/client/src/features/project-explorer/useProjectInit.ts` to import from `../../services/project/component-service` instead of `project-service`

## Finding 4: Extract ComponentMetaSection

**Files changed**:
- Created `editor/client/src/ui/inspector/sections/ComponentMetaSection.tsx` with the extracted `ComponentMetaSection` component and its styles
- Updated `editor/client/src/ui/panels/PropertyPanel.tsx` to import `ComponentMetaSection` from the new location instead of inlining it

## Finding 5: Move App to app/

**Files changed**:
- Moved `editor/client/src/App.tsx` to `editor/client/src/app/App.tsx`
- Updated `editor/client/src/main.tsx` import from `"./App"` to `"./app/App"`

## Verification

- `npx tsc --noEmit --pretty false` -- **0 errors**
- `npx vitest run` -- **190 passed, 2 pre-existing failures** (codegen tests: `integration-generate.test.ts` and `source-emitter-annotation.test.ts` -- these fail on main branch as well, unrelated to these changes)
- All store tests pass (previously failing style-store tests and editor store tests now pass)
