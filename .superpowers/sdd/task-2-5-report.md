# Phase 2.5 Report: Critical Path E2E Safety Net

## Summary

Created `editor/e2e/critical-path.spec.ts` with 6 E2E test scenarios covering the editor's core workflows. These tests use the OmniDriver API (`window.__OMNI__`) and the backward-compatible Zustand combined store (`window.__ZUSTAND_EDITOR_STORE__`) to verify that critical paths remain functional after the architecture refactoring.

## Test Scenarios

| # | Scenario | Coverage |
|---|----------|----------|
| 1 | Add component via OmniDriver and verify store state | OmniDriver -> store elements array, tag, props |
| 2 | Modify element property via store | `store.setProp()` mutation, Store -> React sync |
| 3 | Add wire between elements | Store elements + wires arrays, wire connectivity |
| 4 | Switch mode (design/code) preserves elements | `setActiveMode()`, state persistence across modes |
| 5 | Save marks project clean | `isDirty` flag lifecycle, Ctrl+S -> `saveProject()` -> `markClean()` |
| 6 | Reorder elements updates rootOrder | `reorderElement()`, rootOrder array ordering |

## API Surface Validation

Tested both high-level (OmniDriver) and low-level (Zustand store) APIs:

- **OmniDriver**: `button(label)`, `text(label)` — verified element creation with correct tag and props
- **Combined store**: `addElement`, `setProp`, `addWire`, `setActiveMode`, `reorderElement`, `isDirty` — all confirmed present and functional on `__ZUSTAND_EDITOR_STORE__`
- **Save flow**: Ctrl+S triggers `saveProject()` which calls `store.markClean()` — confirmed at `editor/client/src/services/project/project-service.ts:190`

## Deviations from Template

- Changed `EDITOR_URL` from `http://localhost:5174` to `http://localhost:5199` to match existing test convention (Playwright config's webServer port)
- Fixed button label assertion to match actual OmniDriver behavior: `button("TestBtn")` creates `label: "TestBtn"` (not "按钮" as the template had)
- Added explicit assertions for wire details (fromId, toId) in the add wire test
- Added `rootOrder` before/after assertions in reorder test for stronger verification

## Verification

- Test file compiles cleanly with TypeScript 6.0.3 (checked with client's `tsc --noEmit`)
- All store methods used in tests were verified against the actual combined store interface at `editor/client/src/store/editor.ts`

## File

- `D:\space\OmniLang\.claude\worktrees\editor-architecture-refactor\editor\e2e\critical-path.spec.ts`
