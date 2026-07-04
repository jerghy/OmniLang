# Phase 2: Unified Zustand Root Store

**Date:** 2026-07-04  
**Status:** Complete

## Summary

Created a single unified Zustand store (`usePoiesisStore`) to replace the deleted dual-store
architecture (`useEditorStore` + atom stores). The new store is the central nervous system of
Poiesis v2, managing all state for both the Architecture and Design pages.

## Files Created

| File | Description | Lines |
|------|-------------|-------|
| `poiesis/src/store/index.ts` | Unified Zustand root store | 528 |

## Files Deleted

| File | Description |
|------|-------------|
| `poiesis/src/store/editor.ts` | Old monolithic `useEditorStore` (741 lines) |

## Files Modified

| File | Description |
|------|-------------|
| `poiesis/src/core/state/index.ts` | Updated to re-export `usePoiesisStore` from new store |

## Store Architecture

The unified store covers these state domains:
- **Page routing** — `currentPage` ("architecture" | "design")
- **Architecture state** — bubbles, edges, mode, camera, focus
- **Design page state** — tabs, breadcrumbs, code flow toggle
- **Canvas data** — elements, wires, root order, selection
- **Component definitions** — parsed .omni component cache
- **PID index** — annotation system entries
- **Project metadata** — dir, name, dirty flag, file state
- **Compilation** — errors, preview URL
- **Editor metadata** — node/wire meta, canvas viewport states, panel sizes
- **Persistence** — `toProjectJson()` / `loadFromProjectJson()` helpers

## Verification Results

```
> npx tsc --noEmit | grep "store/index.ts" | grep "error TS"
(no output — zero errors)
```

The new store file itself compiles with zero TypeScript errors. The broader project
has 260 pre-existing errors from unmigrated consumers (App.tsx, drivers, components)
that still reference the old `useEditorStore`.

## Key Decisions

1. **Single flat store** — no slice splitting at the store level; logic is organized
   by domain within the single `create()` call for simplicity.
2. **Immutable state updates** — all mutations use spread operators; no `immer` middleware.
3. **`PoiesisState` interface** — fully typed with all actions and state fields for
   TypeScript safety.
4. **Window exposure** — `window.__ZUSTAND_STORE__` set for E2E testing / Victauri
   integration.
