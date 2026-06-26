# Task 3.1 & 3.2 Report: Shell Components + Canvas Reorganization

## Summary

Both tasks completed in a single commit with 37 files changed, 423 insertions, 219 deletions. TypeScript compiles cleanly (tsc exit 0). All tests pass (190/192; 2 pre-existing failures in untouched `services/codegen/`).

## Task 3.1: Extract shared Toolbar + StatusBar to `ui/shell/`

### Files Created

- **`editor/client/src/ui/shell/theme.ts`** — Shared styles (`S`, `btnStyle`, `iconBtn`) extracted from App.tsx
- **`editor/client/src/ui/shell/Toolbar.tsx`** — Shared toolbar component with mode toggle, undo/redo/save, run/stop/compile, and a `children` slot for mode-specific buttons
- **`editor/client/src/ui/shell/DesignToolbar.tsx`** — Design-mode-specific toolbar: open project, viewport selector, state preview dropdown, token/wire/HUD/focus-order toggles
- **`editor/client/src/ui/shell/CodeToolbar.tsx`** — Code-mode-specific toolbar: quick template dropdown, target selector, connector toggle, auto-layout button
- **`editor/client/src/ui/shell/StatusBar.tsx`** — Shared status bar: shows status text, project name, dirty indicator, project dir, optional compile error count and PerfBadge

### Changes to App.tsx

- Removed duplicated toolbar JSX (~80 lines \* 2 copies) — now uses `<Toolbar>` + `<DesignToolbar>` / `<CodeToolbar>`
- Removed duplicated status bar JSX (~6 lines \* 2 copies) — now uses `<StatusBar>` with mode-specific props
- `S` object, `btnStyle`, `iconBtn` moved to `ui/shell/theme.ts`; imported by App.tsx and shell components
- All 6 canvas imports updated to new `ui/canvas/*` paths

## Task 3.2: Reorganize `canvas/` to `ui/canvas/{design,code,shared}`

### File Moves (git mv)

| New Location | Files |
|---|---|
| `ui/canvas/design/` | LayoutView, CanvasPreview, CanvasTabBar, ComponentCanvasHeader, NodePalette |
| `ui/canvas/code/` | FlowCanvas, CodeTabBar, nodes/BaseNode, nodes/CodeBlockNode, nodes/index |
| `ui/canvas/shared/` | FlowEngine, connectionValidators, layoutUtils, elementsToNodes, wiresToEdges, socketTypes, edges/, fnPortGenerator, nodeRegistry, __tests__/\* |
| `ui/canvas/` (root) | DebugPanel, StateHUD, PerfBadge, PerfMonitor, VariablePanel |

### Import Path Updates

- External imports from `../canvas/` → `../ui/canvas/` (with appropriate subdirectory)
- All `./canvas/` imports in App.tsx → `./ui/canvas/` paths
- Internal cross-imports between moved files updated (e.g., `../nodeRegistry` → `../../shared/nodeRegistry`)
- Fixed pre-existing broken `./toast` import in `FlowCanvas.tsx` → `../../../notifications/toast`

## Verification

- `npx tsc --noEmit` — exit 0, zero errors
- `npx vitest run` — 190/192 pass (2 pre-existing failures in untouched `services/codegen/`)
