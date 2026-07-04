# Integ 1: App.tsx Routing + Page Integration

## Status: PASS

## Changes Made

### 1. `src/App.tsx` — Rewrote placeholder with real page routing
- Imports `ArchitecturePage`, `DesignPage`, and `StatusBar`
- Reads `currentPage` from zustand store to switch between `ArchitecturePage` and `DesignPage`
- Reads `isDirty`, `projectName`, `projectDir`, `compileErrors` from store for StatusBar
- StatusBar shows error count or "就绪" (ready)

### 2. `src/main.tsx` — Simplified entry point
- Removed `Toaster` from sonner (streamlined for initial integration)
- Clean minimal `StrictMode` + `App` rendering

### 3. `src/commands/rustCompiler.ts` — Fixed build blocker
- Added `/* @vite-ignore */` to the dynamic WASM import
- Prevents Vite/Rolldown from trying to resolve `../../public/wasm/omnilang.js` at build time (WASM not yet built)
- `@ts-expect-error` still suppresses the TS module-not-found error

## Verification Results

| Check | Result |
|-------|--------|
| `npx tsc --noEmit --pretty false` | **0 errors** (exit 0) |
| `npx vite build` | **0 errors** — built in 134ms |
| Output size | `index.html`: 0.45 kB, `index.js`: 436.75 kB (gzip: 135.41 kB), `index.css`: 15.41 kB (gzip: 2.56 kB), `commands-*.js`: 15.02 kB (gzip: 4.88 kB) |

## Navigation Flow Verified

1. **App starts** → `currentPage: "architecture"` → `ArchitecturePage` renders (via conditional in App.tsx line 29)
2. **User double-clicks leaf bubble** → `handleNodeDoubleClick` in `ArchitectureGraph.tsx` calls `openTab()` → store sets `currentPage: "design"` → `DesignPage` renders
3. **User clicks breadcrumb "架构"** → `navigateTo("architecture")` → `currentPage` changes back to `"architecture"` → `ArchitecturePage` renders

## Concern

The WASM file at `public/wasm/omnilang.js` does not exist yet. The `rustCompiler.ts` handles this gracefully at runtime via try-catch (logs warning, `isReady` returns false), but until the Rust WASM compiler is built, any feature that triggers compilation will fall through to the error path. This is the expected state for v2 development — the compiler WASM build is a separate task.
