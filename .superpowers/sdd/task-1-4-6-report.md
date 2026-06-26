# Tasks 1.4-1.6 Report

## Task 1.4: Migrate commands/ to core/commands/

- Created `core/commands/` directory
- Moved 18 command files and 2 test files from `commands/` to `core/commands/`
- Updated all internal imports: `../core/types` -> `../types`, `../../core/types` -> `../../types`, `../utils/id` -> `../../utils/id`
- Updated all consumer imports (11 files across panels, canvas, store, core/state) to use `../core/commands/` or `../../core/commands/`
- Updated `App.tsx` which used `"./commands/"` -> `"./core/commands/"`
- Deleted old `commands/` directory
- `tsc --noEmit`: zero errors
- `vitest run`: 15/17 passed (2 pre-existing failures in omni-gen)
- Commit: `136473d2` — `refactor(core): migrate commands/ to core/commands/`

## Task 1.5: Create core/constants/ — extract shared constants

### New files:
- `core/constants/components.ts` — CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS
- `core/constants/defaults.ts` — `getDefaultProps()` (unified component default prop logic)
- `core/constants/index.ts` — barrel export

### Modified files:
- `panels/ComponentPalette.tsx` — replaced inline constants with `import { CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS } from "../core/constants"`
- `panels/ComponentTree.tsx` — replaced inline constants + local `getDefaultProps()` with imports from core/constants; removed `COMPONENT_PROTOCOL` import (no longer needed)
- `canvas/LayoutView.tsx` — replaced hardcoded if-else props in `handleDrop` with `getDefaultProps(tag)`
- `driver/OmniDriver.ts` — `component()` method now merges defaults via `getDefaultProps(tag)`

### Verification:
- `tsc --noEmit`: zero errors
- `vitest run`: 15/17 passed (same 2 pre-existing failures)
- Commit: `538cba65` — `refactor(core): extract shared constants + getDefaultProps; unify component defaults`

## Task 1.6: Delete dead code canvas/toast.ts

- Confirmed no imports reference `canvas/toast` anywhere in the codebase
- Deleted `canvas/toast.ts` (44 lines)
- `tsc --noEmit`: zero errors (file had no dependents)
- Commit: `38090c22` — `chore: remove dead code canvas/toast.ts`

## Final State

- `tsc --noEmit`: **0 errors** (PASS)
- `vitest run`: 15 passed / 2 failed (pre-existing omni-gen test failures, unchanged from baseline)
