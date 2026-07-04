# Round 2 Code Review Fix Report

## Verification
- `npx tsc --noEmit`: 0 errors (was 6 after initial edits, fixed all)
- `npx vite build`: Build succeeded (123ms)

---

## Critical Fixes

### C1: closeTab data loss — DONE
**File:** `src/store/index.ts` — `closeTab` action
**What was done:** Before destructuring `tabId` out of `s.tabData`, the current canvas state (`elements`, `wires`, `rootOrder`, `selectedElementIds`) is now saved into `tabData[tabId]` via a temporary `saveEntry` merged into a `tabDataWithSave` copy. This ensures the closing tab's latest edits are persisted before removal.

### C2: Undo stack corruption — DONE
**File:** `src/store/index.ts` — `undo`, `redo`, `addElement`, `removeElement`
**What was done:** Added a module-level `__skipUndo` flag. Before calling store actions during `undo()`/`redo()`, the flag is set to `true` (inside a `try/finally` block for safety). All actions (`addElement`, `removeElement`, `setProp`, `addWire`, `removeWire`, `moveElement`) check `__skipUndo` at the top of their set-callback and return early without recording undo. This prevents inner `set()` calls from contaminating the undo stack.

### C3: saveProject no-op — DONE
**File:** `src/services/persistence/save-manager.ts`
**What was done:** Removed the `store.markClean()` call (no actual write was happening). Added a `// TODO: wire Tauri fs writeFile` comment where the write should go. Returned `jsonStr` in the result so callers can use it. Updated return type to `Promise<{ success: boolean; error?: string; jsonStr?: string }>`.

### C4: Context menu passes PID as filePath — DONE
**File:** `src/components/architecture/context-menu/BubbleContextMenu.tsx`
**What was done:** In `handleOpenDesign`, the `openTab` call now resolves the file path through `pidIndex[bubble.pid].sourceFile` (falling back to `bubble.pid` if no entry found), matching the pattern used by `ArchitectureGraph.handleNodeDoubleClick`.

### C5: loadProjectFromJson doesn't reset store — DONE
**File:** `src/services/persistence/load-manager.ts`
**What was done:** Added `store.newProject()` call before `store.loadFromProjectJson(projectJson)` in `loadProjectFromJson`, ensuring stale state is cleared before loading.

### C6: useProjectInit writes to non-existent projectPath — DONE
**File:** `src/features/useProjectInit.ts`
**What was done:** Changed `projectPath: dir` to `projectDir: dir` to match the actual `PoiesisState` field name.

---

## Important Fixes

### I1: First action never recorded in undo — DONE
**File:** `src/store/index.ts` — `addElement`, `removeElement`, `setProp`, `addWire`, `removeWire`, `moveElement`
**What was done:** Changed the undo stack initialization from `tabId && s.undoStacks[tabId] ? { ... } : undefined` to `tabId ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] } : undefined`. This uses the `existingStack` variable (which may be undefined for the first action) with a `?? []` fallback, ensuring the first action for a tab is always recorded.

### I2: closeTab doesn't save closing tab's canvas — DONE
(Part of C1 fix above)
**What was done:** Same resolution as C1. Additionally, the fallback for `restoreEntry` was changed from `s.elements`/`s.wires`/etc. to `[]` when an active neighbor tab exists but has no saved data (fresh state). Current state fallback only applies when no active tab remains (all tabs closed).

### I3: collapseAll bypasses store contract — DONE
**File:** `src/hooks/useBubbleExpand.ts`
**What was done:** Replaced `usePoiesisStore.setState()` direct call with a loop calling the store's `collapseBubble(id)` action for each child bubble, respecting the store contract.

### I4: Module-level pidCounter in buildPidAnnotations — DONE
**File:** `src/services/codegen/annotation-data.ts`
**What was done:** Moved `pidCounter` variable and `nextPidSeq` function inside `buildPidAnnotations` function body, eliminating module-level mutable state that could be corrupted by concurrent calls.

### I5: orphaned element silent on reorder — DONE
**File:** `src/store/index.ts` — `reorderElement` action
**What was done:** Added a `parent` lookup and `console.warn('[store] reorderElement: parent not found for', elementId)` before returning `s` when the declared parent doesn't exist. Changed sibling resolution to use the found `parent` object rather than re-looking up in the array.

### I6: setProp/addWire/removeWire/moveElement have no undo — DONE
**File:** `src/store/index.ts`
**What was done:** Added undo recording to all four actions:
- `setProp`: captures `oldValue` before mutation, records `{ type: "setProp", id, prop, oldValue }`
- `addWire`: records `{ type: "removeWire", id: w.id }` as inverse
- `removeWire`: captures the `wire` data before removal, records `{ type: "addWire", wire }`
- `moveElement`: captures `oldParentId` before the move, records `{ type: "moveElement", elementId, oldParentId }`

All operations respect the `__skipUndo` flag to prevent contamination during undo/redo.

The `UndoAction` type was extended with the new discriminated union members. The `undo()` and `redo()` handlers were extended with a `switch` statement handling all action types, including direct `set()` for `moveElement` (which restores the element to its old parent/root).

### I7: O(n^2) bubble layout — DONE
**File:** `src/hooks/useBubbleLayout.ts`
**What was done:** Pre-computed a `siblingGroups` (`Map<string, BubbleNode[]>`) before iterating child bubbles, replacing the O(n) `childBubbles.filter(...)` inside the O(n) `forEach` loop. Sibling index lookup is now O(1) map access + O(1) `indexOf` within a pre-grouped array.

### I8: ArchitectureGraph always loads demo data — DONE
**File:** `src/components/architecture/ArchitectureGraph.tsx`
**What was done:** The `buildBubbleGraph` call now reads `projectDir` from the store via `usePoiesisStore.getState().projectDir` instead of always passing an empty string. This allows the graph to load from the actual project directory when one is set (still falls back to demo data if no files found).

---

## Minor Fixes

### M2: setElementSource/setElementPosition set isDirty: true — DONE
**File:** `src/store/index.ts`
**What was done:** Added `isDirty: true` to both `setElementSource` and `setElementPosition` return objects, ensuring these mutations mark the project as dirty.

---

## Files Modified (9 total)
1. `src/store/index.ts` — C1, C2, I1, I2, I5, I6, M2
2. `src/services/persistence/save-manager.ts` — C3
3. `src/services/persistence/load-manager.ts` — C5
4. `src/components/architecture/context-menu/BubbleContextMenu.tsx` — C4
5. `src/features/useProjectInit.ts` — C6
6. `src/hooks/useBubbleExpand.ts` — I3
7. `src/services/codegen/annotation-data.ts` — I4
8. `src/hooks/useBubbleLayout.ts` — I7
9. `src/components/architecture/ArchitectureGraph.tsx` — I8
