# Task S4-A Report: TypeScript Type Hardening

## Summary

Implemented Section 3.4 of FIX_PLAN.md — hardened 5 type families using discriminated unions and interface splits. TypeScript `tsc --noEmit` passes with **zero errors**.

## Changes

### T1: ComponentPart → discriminated union (types.ts + 2 consumers)
- Split `ComponentPart` into `ValidComponentPart | PartialComponentPart`
- Added `SignalDef`, `ComputedDef`, `ConstDef`, `PropDef`, `EventDef`, `FnDef` helper interfaces
- Updated `LoweredCode.functions` to use `FnDef[]` instead of inline type
- **component-builder.ts**: Returns `ValidComponentPart` when view is non-null, `PartialComponentPart` when null; uses empty arrays instead of undefined for props/events/consts in the valid case
- **source-emitter.ts**: Added `|| []` / `|| {}` guards for fns, computeds, style on Partial variant

### T2: ComponentDef split (project-json.ts + 1 consumer)
- Split into `ComponentSourceDef` (required signal/prop/event/fn metadata) and `ComponentCanvasState` (optional elements/wires/rootOrder)
- `ComponentDef = ComponentSourceDef & Partial<ComponentCanvasState>`
- **App.tsx**: Fixed ComponentDef construction to provide default source fields when building from parsed .omni data

### T3: AnnotatedLoadResponse → discriminated union (annotation.ts)
- Changed to `{ success: true; source: string; ... } | { success: false; error: string }`
- No active consumers in TypeScript code (the only call site in useProjectInit.ts uses an inline cast type)

### T4: PidMap → discriminated union (pid.ts + 6 consumers)
- Changed to `{ status: "ok"; entries; lineToPid } | { status: "error"; error: string }`
- **omni-parser.ts**: Updated construction to local variables before wrapping in discriminated variant
- **annotation-api.ts**: Updated parsePidComments and readPidMap (both success and error returns)
- **pid-index.ts**: Updated buildPidMap construction; added status guards to resolvePid, resolvePidByLine, mergePidMaps, detectPidConflicts
- **annotation-data.ts**: Added `existingPidMap.status === "ok"` guard before iterating entries
- **tab-slice.ts**: Changed default pidMap from `{ entries: {}, lineToPid: {} }` to `{ status: "ok", entries: {}, lineToPid: {} }`

### T6: FileTypeMap / getTabType (design-page.ts)
- Added `FileTypeMap = Record<string, TabType>` type
- Added `getTabType(filePath, fileTypeMap?)` that prefers explicit metadata, falls back to heuristic `detectTabType`
- Backward compatible — no breakage

## Files Modified (Poiesis only)

| File | Change |
|------|--------|
| `poiesis/src/services/codegen/types.ts` | T1: ComponentPart discriminated union + helper types |
| `poiesis/src/services/codegen/component-builder.ts` | T1 consumer: return correct union variant |
| `poiesis/src/services/codegen/source-emitter.ts` | T1 consumer: safe optional field access |
| `poiesis/src/core/types/project-json.ts` | T2: ComponentDef split |
| `poiesis/src/App.tsx` | T2 consumer: default source fields |
| `poiesis/src/core/types/annotation.ts` | T3: AnnotatedLoadResponse discriminated union |
| `poiesis/src/core/types/pid.ts` | T4: PidMap discriminated union |
| `poiesis/src/services/codegen/omni-parser.ts` | T4 consumer: construction |
| `poiesis/src/commands/annotation-api.ts` | T4 consumer: construction |
| `poiesis/src/services/pid/pid-index.ts` | T4 consumer: construction + utility guards |
| `poiesis/src/services/codegen/annotation-data.ts` | T4 consumer: status guard |
| `poiesis/src/store/tab-slice.ts` | T4 consumer: construction |
| `poiesis/src/core/types/design-page.ts` | T6: FileTypeMap + getTabType |
