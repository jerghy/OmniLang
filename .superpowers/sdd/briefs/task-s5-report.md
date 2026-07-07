# Task S5 Report: Zustand Store Split (Phase 1-2)

## Summary

Extracted two self-contained domain stores from the monolithic `PoiesisState` (80+ fields) and added tab data helper utilities. The monolithic store remains fully functional and backward compatible — all existing `usePoiesisStore` consumers continue to work unchanged.

## Files Created

### `poiesis/src/store/architecture-store.ts`
Standalone Zustand store managing the architecture bubble-graph state:
- State: `architecture` (ArchitectureState — mode, bubbles, edges, focusComponentId, expandedBubbleIds, selectedPid, camera)
- Actions: `setArchMode`, `expandBubble`, `collapseBubble`, `focusComponent`, `selectBubbleByPid`, `setArchCamera`, `setBubblesAndEdges`, `setArchitectureState` (bulk update for project.json load)
- Exports `createInitialArchitectureState()` helper

### `poiesis/src/store/design-system-store.ts`
Standalone Zustand store managing design tokens, classes, variants, and element variant bindings:
- State: `activeDesign`, `elementVariants`
- Actions: `loadDesign`, `addToken`, `updateToken`, `removeToken`, `addDesignClass`, `updateDesignClass`, `removeDesignClass`, `addVariant`, `updateVariant`, `removeVariant`, `bindVariantToElement`, `unbindVariantFromElement`
- Does NOT manage `isDirty` — handled by the monolithic store subscription

### `poiesis/src/store/tab-helpers.ts`
Pure utility functions extracted from duplicated tabData save/restore logic:
- `buildTabSaveEntry(source)` — creates a TabDataEntry from current state
- `tabRestoreCanvasPartial(entry, fallbackElements?)` — returns `{elements, wires, rootOrder, selectedElementIds}` partial
- `tabRestoreDesignPagePartial(entry)` — returns `{codeFlowVisible, codeFlowElementId}` partial

## Files Modified

### `poiesis/src/store/index.ts`
- Added imports from `architecture-store`, `design-system-store`, `tab-helpers`
- Removed local `createInitialArchitectureState()` (replaced by import)
- Removed `createDesignSlice` factory import (kept `DesignSlice` type for `extends`)
- Added `useArchitectureStore.subscribe(...)` — syncs architecture state reactively
- Added `useDesignSystemStore.subscribe(...)` — syncs design state reactively + marks isDirty
- Replaced inline architecture actions with delegation to `useArchitectureStore`
- Replaced inline design actions (`...createDesignSlice(set)`) with delegation to `useDesignSystemStore`
- Updated `resetEditorState` and `newProject` to reset domain stores alongside monolithic state
- Updated `loadFromProjectJson` to delegate architecture state to domain store
- Updated `removeComponent` to use `buildTabSaveEntry` / `tabRestoreCanvasPartial` / `tabRestoreDesignPagePartial`
- Initial state reads from domain stores: `architecture: useArchitectureStore.getState().architecture`, etc.

### `poiesis/src/store/tab-slice.ts`
- Added imports from `tab-helpers`
- Replaced 3 inline save-entry constructions with `buildTabSaveEntry(s)`
- Replaced inline restore logic with `tabRestoreCanvasPartial` + `tabRestoreDesignPagePartial`
- Behavior unchanged — all 4 tab lifecycle actions (openTab, closeTab, switchTab, removeComponent) produce identical state

## Verification

- `npx tsc --noEmit` passes (exit code 0)
- DesignSlice interface still extended by PoiesisState — full type backward compatibility
- Domain stores are standalone and testable independently
- No consumer code changes required — only the store implementation changed
