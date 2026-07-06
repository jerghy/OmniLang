# Task 2.4: Fix TypeScript data integrity issues

## Summary

Three data-integrity fixes in the Poiesis engine: readPidMap now surfaces errors with an `error` field; lineToPid detects conflicting PID annotations on the same line; removeComponent uses a `set()` callback to avoid stale closure bugs.

## Changes Made

### 1. readPidMap — distinguish "file not found" vs "read error"

- **`poiesis/src/core/types/pid.ts`**: Added optional `error?: string` field to the `PidMap` interface
- **`poiesis/src/commands/annotation-api.ts`** (line 149): Changed catch-block return from `{ entries: {}, lineToPid: {} }` to `{ entries: {}, lineToPid: {}, error: String(e) }`, so callers can inspect the error field to decide whether to trigger automatic regeneration

### 2. lineToPid — conflict detection warning

- **`poiesis/src/services/codegen/omni-parser.ts`** (lines 58-60): Added a `console.warn` before overwriting `lineToPid[line]` when the existing value differs from the new entry's PID. This catches e.g. two `// @pid:c1` and `// @pid:c2` on the same line

### 3. removeComponent — stale closure fix

- **`poiesis/src/store/index.ts`** (lines 505-548): Rewrote `removeComponent` from a `get()` + external-mutation pattern to a single `set()` callback. The tab-close logic from `tab-slice.ts` `closeTab` is inlined inside the same `set()` callback to ensure `componentDefs` removal and tab cleanup always see consistent, fresh state

## Verification

- `cd poiesis && npx tsc --noEmit`: Zero errors
