# PID Annotation System (Phase 1) - Report

## Status: DONE

## Files Changed

### CREATED
1. `poiesis/src/services/pid/pid-index.ts` — PID generation functions: `generatePid`, `buildPidMap`, `resolvePid`, `resolvePidByLine`, `mergePidMaps`
2. `poiesis/src/services/pid/index.ts` — Barrel export for PID service

### MODIFIED
3. `poiesis/src/core/types/pid.ts` — Added `"lib"` to `PidBlockType` union type
4. `poiesis/src/services/codegen/types.ts` — Replaced `EditAnnotation` with `PidAnnotation`; updated `AnnotationData.pidAnnotations` (was `editAnnotations`); updated `AnnotationStrategy.beforeViewNode` return type from `string | null` to `PidAnnotation | null`; added `pidMap` to `ParseResult`
5. `poiesis/src/services/codegen/annotation-data.ts` — Rewrote: `buildAnnotationData` renamed to `buildPidAnnotations`, generates `c<N>_v<seq>` PIDs instead of `// @edit` lines; removed `isFolded`/`renderMode` logic; `createAnnotationStrategy` returns `PidAnnotation | null`
6. `poiesis/src/services/codegen/source-emitter.ts` — Changed to emit `// @pid:<pid>` instead of `// @edit` line
7. `poiesis/src/services/codegen/omni-parser.ts` — Added `parsePidAnnotations` function to extract `// @pid:xxx` comments; `parseOmniSource` now includes `pidMap` in result
8. `poiesis/src/services/codegen/pipeline.ts` — Updated import from `buildAnnotationData` to `buildPidAnnotations`; call site passes `componentIndex: 1`
9. `poiesis/src/services/codegen/index.ts` — Exports `parsePidAnnotations`, `buildPidAnnotations`, `createAnnotationStrategy`
10. `poiesis/src/commands/annotation-api.ts` — Replaced `annotatedLoad`/`annotatedSave` with `readPidMap` and `writePidAnnotations`; fixed unused imports and `EditorTomlState` import path

## Verification Results

```
TypeScript errors in modified files: 0 (new)
TypeScript errors total (pre-existing): 175
```

- **Zero new errors** in the files I modified or created.
- Pre-existing errors (missing modules like `CodeTab`, `componentRegistry`, `styleRegistry`, etc.) remain unchanged.
- Expected new errors in `App.tsx` (line 158) and `useProjectInit.ts` (line 56) due to `annotatedLoad` removal — these callers need separate migration.

## Key Design Decisions

1. **PID format for view nodes**: `c<compIndex>_v<seq>` with sequential counter per tree walk. This is deterministic and stable across re-generations for the same tree structure.

2. **`generatePid` in pid-index.ts**: Handles top-level blocks (`c`, `s`, `l` prefixes). Sub-block view PIDs use the separate view-node format above.

3. **`parsePidAnnotations`**: A lightweight regex-based parser that scans `// @pid:xxx` comments. It populates `PidMap.lineToPid` for line-to-PID resolution but uses minimal `PidEntry` data (blockType defaults to "view"). Full entry enrichment happens on the emitter side.

4. **No backward compat**: The old `// @edit` system and `EditAnnotation` type are completely removed. `isFolded`/`renderMode` from old `@edit` are dropped from codegen (they move to project.json in a later phase).

5. **`writePidAnnotations`**: Saves both the .omni source and the PidMap envelope to `.poiesis/annotations/`, following the same pattern as the old `annotatedSave`.
