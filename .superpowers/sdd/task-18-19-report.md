# Task 18 + 19 Report: TokenEmitter, CustomComponentEmitter, App.tsx Integration

## Status: COMPLETE

## Commit

```
b435c965 feat(omni-gen): add TokenEmitter, CustomComponentEmitter, and App.tsx integration
```

## Changes

### Task 18: TokenEmitter + CustomComponentEmitter

**`src/omni-gen/token-emitter.ts`** (new)
- `emitTokens(themes: ThemeDef[]): string` — emits theme blocks as OmniLang source
- Handles empty input (returns `""`), multiple categories, and entries per category
- Output format:
```
theme myTheme {
  colors {
    primary: "#4F46E5";
  }
}
```

**`src/omni-gen/custom-component-emitter.ts`** (new)
- `emitCustomComponents(...): StageResult<Record<string, string>>` — emits custom component definitions
- Reuses `buildViewTree` + `resolveStyle` + `SourceEmitter` from the existing pipeline
- Each component becomes `src/components/<Name>.omni`
- Error handling via `StageResult` with fatal error codes (`E_CC_EMIT`)

### Task 19: App.tsx Integration

Four surgical edits to `src/App.tsx`:

1. **Import** — added `parseOmniSource`, `generateOmniSource`, `generateOmniFiles`, `validate` from `./omni-gen`

2. **openFile** — integrated into the auto-compile effect: when a `.omni` file is opened, `parseOmniSource` populates `useEditorStore` with elements/wires and `useCodeGraphStore` with code tabs, then triggers compile

3. **handleCompile** — before the existing `api.compile` call: generates source from design elements via `generateOmniSource`, validates with `validate`, updates editor content with generated source; shows errors and returns if validation fails

4. **saveProject** — after the existing JSON save: calls `generateOmniFiles`, validates each generated file first, then writes valid files via `api.writeFile`; handles fatal generation errors and per-file validation failures

### Index exports

Added `emitTokens` and `emitCustomComponents` to `src/omni-gen/index.ts`

## Tests

```
 Test Files  9 passed (9)
      Tests  85 passed (85)
```

All 9 test files and 85 tests pass with no regressions.

## TypeScript

`npx tsc --noEmit` — clean, no errors.

## Concerns

- TokenEmitter and CustomComponentEmitter have no dedicated unit tests yet. They are exercised indirectly through the existing test suite and pipeline integration.
- `handleCompile` may trigger a re-render when `setSourceCode` updates `activeFileContent`, but the `prevFilePath` guard in the auto-compile effect prevents the compile from re-entering.

## Review Fixes (2026-06-21)

Two review findings addressed:

1. **CustomComponentEmitter multi-root warning** — added `W_MULTI_ROOT` warning push to errors when `roots.length > 1`, indicating additional roots are silently dropped.

2. **App.tsx saveProject race condition** — changed `Promise.all(...).catch(...)` to `await Promise.all(...)` inside a try/catch block so the "save succeeded" status message only appears after all file writes complete. Failure during writes now exits early with an error status.

### Commit
```
<commit-sha> fix(omni-gen): address review — multi-root warning in CC emitter, await file writes in saveProject
```

### Tests
```
 Test Files  9 passed (9)
      Tests  85 passed (85)
```

`npx tsc --noEmit` — clean, no errors.
