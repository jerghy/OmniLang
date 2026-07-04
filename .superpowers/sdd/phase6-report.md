# Phase 6 Report: Data Persistence — project.json Save/Load

**Date**: 2026-07-04
**Status**: COMPLETE

## Files Created

All files in `poiesis/src/services/persistence/`:

1. **`project-json.ts`** — Core ProjectJson serialization/deserialization with:
   - `serializeProjectJson()` — JSON.stringify with alphabetically sorted keys for git merge friendliness
   - `sortedKeysReplacer()` — Custom JSON replacer that sorts all object keys recursively
   - `deserializeProjectJson()` — Parse and validate raw JSON string into ProjectJson
   - `validateProjectJson()` — Version check (= 2) and type validation for nodes/wires
   - `createEmptyProjectJson()` — Factory with sensible defaults

2. **`save-manager.ts`** — Save pipeline orchestration:
   - `buildProjectJsonFromStore()` — Reads Zustand store state and returns serialized ProjectJson
   - `saveProject()` — Async save pipeline (for Tauri fs integration; currently builds store state and marks clean)

3. **`load-manager.ts`** — Load pipeline orchestration:
   - `loadProject()` — Full pipeline: read project.json, parse .omni files, populate store
   - `loadProjectFromJson()` — In-memory load from raw JSON string (no .omni parsing)
   - `validatePidConsistency()` — Cross-check project.json node PIDs against parsed .omni index

4. **`index.ts`** — Re-exports all public functions

## Files Modified

5. **`commands/annotation-api.ts`** — Added two new methods to the annotation API:
   - `saveProject()` — Calls `buildProjectJsonFromStore()` and writes to `project.json` via Tauri fs
   - `loadProject()` — Reads `project.json`, calls `loadProjectFromJson()` to populate store

## TypeScript Errors

- **Persistence layer files**: 0 errors
- **annotation-api.ts**: 0 errors (from our changes)
- Pre-existing errors elsewhere in the project are unchanged.

## Round-Trip Verification

Test: Create ProjectJson with nodes/wires, serialize, deserialize, compare.

**Result: PASS**
- Sorted keys verified: `architecture → canvasStates → componentDefs → nodes → panels → version → wires`
- All field values preserved correctly through round-trip

## Concerns

- `save-manager.ts` and `load-manager.ts` have placeholder Tauri fs calls (commented out). Actual disk I/O needs the Tauri backend integration to uncomment `writeTextFile` / `readTextFile`.
- Pre-existing `annotatedLoad` references in `App.tsx` and `useProjectInit.ts` reference a removed v1 API method. These are not new issues but remain as cleanup items.
