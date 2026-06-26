# Task 1.7 & 1.8 Report: core/utils/ + undo bypass fix

## Task 1.7: Create core/utils/ -- unify ID generation + extractComponentName

### New files created

- `editor/client/src/core/utils/id.ts` -- `uid()` function with `{prefix}-{ts}-{rand}-{seq}` format
- `editor/client/src/core/utils/omni.ts` -- `extractComponentName()` utility
- `editor/client/src/core/utils/index.ts` -- barrel re-export

### ID generators replaced (all use `uid()`)

| File | Old pattern | New pattern |
|------|-------------|-------------|
| OmniDriver.ts `_nextId()` | `\`el-${Date.now()}-${Math.random()...}\`` | `uid("el")` |
| OmniDriver.ts `cloneTab` | inline `newId()` with `Date.now()+Math.random` | `uid()` |
| codegraph/types.ts `newTabId()` | `\`tab-${Date.now()}-${_tabUid++}\`` | `uid("tab")` + removed `_tabUid` counter |
| core/state/codegraph.ts `uid()` | `Date.now().toString(36)+'-'+Math.random()` | delegates to `_uid()` from core/utils |
| BlockPalette.tsx | `\`cb-${tag}-${Date.now()}-${_seq++}\`` | `uid(\`cb-${tag}\`)` + removed `_seq` counter |
| pipeline.ts `generateId()` | `\`s_${Date.now()}_${++_idCounter}\`` | `uid("s")` + removed `_idCounter` |
| PropertyPanel.tsx | `id: \`s_${Date.now()}\`` | `id: uid("s")` |
| StatePanel.tsx (x2) | `\`cb-${kind}-${Date.now()}-${Math.random()...}\`` | `uid(\`cb-${kind}\`)` |
| StatePanel.tsx (store) | `\`cb-store-${Date.now()}-${Math.random()...}\`` | `uid("cb-store")` |
| FunctionLibrary.tsx | `\`cb-fn-${Date.now()}-${Math.random()...}\`` | `uid("cb-fn")` |

### Inline extractComponentName replaced

| File | Locations | Before | After |
|------|-----------|--------|-------|
| App.tsx | 5 (lines 438, 531, 624, 675, 797) | `source.match(/component\s+(\w+)\s*\{/)?.[1] \|\| "App"` | `extractComponentName(source)` |
| LayoutView.tsx | 1 (line 78) | inline match + fallback logic | `extractComponentName(source, "App")` |

## Task 1.8: Fix Undo bypass

### OmniDriver.ts

- **`component()` method** (line 587): replaced `this._store().addElement(el)` with `new AddElement(el).execute(cmdCtx)` -- builds a `CommandContext` from `useEditorStore.getState()`
- **`connect()` method** (line 870): replaced `store.addWire({...})` with `new AddWire(wire).execute(wireCtx)` -- wraps the `_activeStore()` with a `CommandContext`

### StatePanel.tsx

- **`createState()`** (line 399): replaced `ctx.addElement(el)` with `useUndoStore.getState().execute(new AddElement(el), ctx)` -- makes signal/computed creation undoable
- **`createStore()`** (line 420): replaced `ctx.addElement(el)` with `useUndoStore.getState().execute(new AddElement(el), ctx)` -- makes store creation undoable

### Import changes

- OmniDriver.ts: added imports for `uid`, `AddElement`, `AddWire`, `CommandContext`
- StatePanel.tsx: added import for `uid`; `useUndoStore` already imported
- codegraph/types.ts: added import for `uid`
- core/state/codegraph.ts: added import for `uid` (aliased as `_uid`)
- BlockPalette.tsx: changed `uid` import from `../utils/id` to `../core/utils`
- PropertyPanel.tsx: changed `uid` import from `../utils/id` to `../core/utils`
- pipeline.ts: added import for `uid`
- FunctionLibrary.tsx: added import for `uid`
- App.tsx: added import for `extractComponentName`
- LayoutView.tsx: changed `uid` import from `../utils/id` to `../core/utils`; added `extractComponentName`

## Verification

- `npx tsc --noEmit --pretty false` -- **zero errors**
- `npx vitest run` -- **matches baseline** (2 pre-existing failures in omni-gen source emitter tests, 190 passed)
