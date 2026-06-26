# Task 2.1 Report: Migrate `omni-gen/` to `services/codegen/`

## Summary

Successfully migrated the `omni-gen/` directory to `services/codegen/` with updated import paths across source files, tests, and all consumers.

## Files Migrated

- **16 source files**: `services/codegen/*.ts`
- **10 test files**: `services/codegen/__tests__/*.ts`

## Import Updates

### Internal source imports (`../` -> `../../`)
- `../core/types` -> `../../core/types` (9 files)
- `../core/utils` -> `../../core/utils` (1 file)
- `../store/componentRegistry` -> `../../core/state/componentRegistry` (2 files)
- `../api/rustCompiler` -> `../../api/rustCompiler` (1 file)

### Internal test imports (`../../` -> `../../../`)
- `../../core/types` -> `../../../core/types` (4 files)
- `../../api/rustCompiler` -> `../../../api/rustCompiler` (2 files)

### Consumer imports (`omni-gen` -> `services/codegen`)
- `src/App.tsx`: `"./omni-gen"` -> `"./services/codegen"`
- `src/canvas/CanvasTabBar.tsx`: `"../omni-gen"` -> `"../services/codegen"`
- `src/panels/CanvasPanel.tsx`: `"../omni-gen"` -> `"../services/codegen"`
- `src/store/editor.ts`: `"../omni-gen"` -> `"../services/codegen"`
- `src/omni-compiler-cache.ts`: `"./omni-gen/pipeline"` -> `"./services/codegen/pipeline"`

### Test mock path fix
- `services/codegen/__tests__/validator.test.ts`: `vi.mock("../../api/rustCompiler"` -> `vi.mock("../../../api/rustCompiler"` (required because vitest resolves mock paths relative to the test file)

## Verification

| Check | Result |
|-------|--------|
| `tsc --noEmit` | Zero errors |
| `vitest run` | 190 passed / 192 total (2 pre-existing failures) |
| Old `omni-gen/` deleted | Confirmed no stale references |

## Commit

```
bf9ece9d refactor(services): migrate omni-gen/ to services/codegen/
```
