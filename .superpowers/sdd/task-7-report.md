# Task 7 Report: Save Pipeline Integration for Inline Stores

## Status: COMPLETED

## Changes Summary

The save pipeline now fully supports inline store definitions — stores defined inside component `.omni` files are persisted to disk when the user clicks save, and reloaded from disk when component files are opened.

### Modified: `poiesis/src/services/codegen/pipeline-types.ts`
- Added `inlineStores?: StoreDefinition[]` to `GenerateOmniSourceOptions`
- Added `inlineStores?: StoreDefinition[]` to `GenerateOmniFilesOptions`
- Added `import type { StoreDefinition } from "../../core/types/store";`

### Modified: `poiesis/src/services/codegen/pipeline.ts`
- Added `inlineStores?: StoreDefinition[]` to the inline options type in `generateOmniFiles` function
- Added `inlineStores: options.inlineStores ?? []` to the `EmitInput` construction in `generateOmniSource`, so inline stores flow from pipeline options through to the SourceEmitter
- Added `import type { StoreDefinition } from "../../core/types/store";`

### Modified: `poiesis/src/hooks/useSaveProject.ts`
- In `handleSave()`, the component tab save path now collects `inlineStoreDefs` from Zustand state that match the current component (keyed by `ComponentName.` prefix)
- Passes the collected `inlineStores` array to `generateOmniFiles`

### Modified: `poiesis/src/components/design/DesignTabBar.tsx`
- In `handleOpenFile()`, after opening a component file (non-design, non-store tab), reads the file, parses it, and loads any `inlineStoreDefs` from the parse result into Zustand via `loadInlineStoreDef`

### Created: `poiesis/src/services/codegen/__tests__/save-inline-store.test.ts`
- 7 integration tests for the save pipeline:
  1. `generateOmniSource` includes inline stores in component output
  2. `generateOmniFiles` includes inline stores in component output
  3. Multiple inline stores are emitted inside the component
  4. Backward compatibility: works without inline stores
  5. Backward compatibility for `generateOmniFiles`
  6. Inline stores with actions and getters survive full pipeline
  7. Inline stores do NOT generate standalone files under `src/stores/`

## Data Flow

```
User clicks Save
  → useSaveProject.handleSave()
    → Collects inlineStoreDefs from Zustand filtered by component name
    → generateOmniFiles({ inlineStores, ... })
      → generateOmniSource({ inlineStores, ... })
        → EmitInput { inlineStores, ... }
          → SourceEmitter.emitDocument()
            → emitInlineStore() for each store
              → Written to component .omni file

User opens file
  → DesignTabBar.handleOpenFile()
    → api.project.readFile()
    → parseOmniSource()
    → loadInlineStoreDef() for each parsed def
      → Zustand inlineStoreDefs populated
```

## Test Results

All 52 tests pass (7 test files, 52 test cases). Zero new TypeScript errors introduced by these changes (48 pre-existing errors remain unmodified).

## Files Changed

- `D:/space/OmniLang/poiesis/src/services/codegen/pipeline-types.ts`
- `D:/space/OmniLang/poiesis/src/services/codegen/pipeline.ts`
- `D:/space/OmniLang/poiesis/src/hooks/useSaveProject.ts`
- `D:/space/OmniLang/poiesis/src/components/design/DesignTabBar.tsx`
- `D:/space/OmniLang/poiesis/src/services/codegen/__tests__/save-inline-store.test.ts`
