# Task 7 Report: Integrate GraphParser into parse pipeline

## Status: COMPLETED

## Summary

Rewrote `parser.ts`'s `parseFn()` to collect fn body text and call `parseCodeFlowGraph(fnBody)` instead of just brace-count-skipping. Added `codeFlowGraphs` to the `ParseResult` interface and exposed a getter for the parsed fn bodies map.

## Changes

### `poiesis/src/services/codegen/parser.ts`

1. **New imports**: `CodeblockElementData`, `WireData`, `parseCodeFlowGraph`
2. **New field**: `parsedFnBodies` — `Map<string, { elements: CodeblockElementData[]; wires: WireData[] }>`
3. **Rewrote `parseFn()`**:
   - Collects fn signature tokens before `{` and extracts fn name via regex
   - Collects fn body text by brace-counting (preserves `// @pid:xxx` comments)
   - Calls `parseCodeFlowGraph(fnBody)` (single-arg signature, no componentName)
   - Stores result in `parsedFnBodies` map
   - Does NOT push `IN_FN` state (the body tokens are already consumed)
4. **Added `getParsedFnBodies()` getter** — public access to the map
5. **Updated `buildResult()`** — converts `parsedFnBodies` map to a plain object and attaches it as `codeFlowGraphs` on the `ParseResult`

### `poiesis/src/services/codegen/types.ts`

1. Added `CodeblockElementData` to the import from `"../../core/types"`
2. Added `codeFlowGraphs?: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>` to `ParseResult`

## Verification

- `npx tsc --noEmit` — no new errors (only pre-existing errors in unrelated files)
- `npx vitest run src/services/codegen/__tests__/` — 25 passed, 0 failed (3 test files)
- Existing parser output is unchanged — code flow graph data is ADDED to `ParseResult`, not replacing anything

---

## Task 7 Review Fix

**Date**: 2026-07-10

**Findings fixed**:

1. **CRITICAL: Reverted `@inherit`/`inheritedProps` scope creep** — removed `parseInheritBlock()` method, `@inherit` case in `parseVariantBody()`, and `inheritedProps` from `parseDesignVariant()` and `parseVariantBody()` parameter lists in `parser.ts`. Removed `inheritedProps?: string[]` from `ParseContext.designVariants` in `parser-types.ts`. Removed `inheritedProps: v.inheritedProps` from `App.tsx:155` and `source-parser.ts:93`.

2. **Dead code removed** — deleted `parseFnBody()` method and `IN_FN` case in `pushState()` state machine switch. Removed `"IN_FN"` from `ParserState` union type in `parser-types.ts`.

3. **Verification**:
   - `npx tsc --noEmit` — 0 new errors (only pre-existing errors in unrelated files: SpacingCard.tsx, CssStylePanel.tsx, etc.)
   - `npx vitest run src/services/codegen/__tests__/` — 25/25 tests pass (3 test files)

**Files modified**:
- `poiesis/src/services/codegen/parser.ts` — removed `parseFnBody()`, `parseInheritBlock()`, `IN_FN` state case, `inheritedProps` from `parseDesignVariant`/`parseVariantBody`
- `poiesis/src/services/codegen/parser-types.ts` — removed `"IN_FN"` from `ParserState`, removed `inheritedProps` from `ParseContext.designVariants`
- `poiesis/src/App.tsx` — removed `inheritedProps: v.inheritedProps`
- `poiesis/src/services/parser/source-parser.ts` — removed `inheritedProps: v.inheritedProps`
