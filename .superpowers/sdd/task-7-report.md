# Task 7 Report: AutocompletePlugin — Text Monitoring + Menu Trigger

**Status:** DONE

## Summary

Implemented the AutocompletePlugin — text monitoring + menu trigger for the lexical editor.

## Files Created

1. **`poiesis/src/components/canvas/code/lexical/autocomplete/AutocompleteMenu.tsx`**
   - Copied from `poiesis/src/components/canvas/code/cards/AutocompleteMenu.tsx`
   - Updated import: now imports `AutocompleteItem` type from `./autocomplete-rules` instead of defining locally
   - Removed the local `getAutocompleteItems` function (now lives in `autocomplete-rules.ts`)

2. **`poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx`**
   - Lexical plugin React component
   - **Token detection**: reads editor state on each text change, extracts current token at cursor, filters autocomplete candidates via `getAutocompleteItems()`
   - **Context awareness**: detects whether cursor is in a `SlotTextNode` (gets `slotType` via `(node as any).__slot`) or regular `TextNode`, walks up to find parent `CardElementNode` for `parentKind`
   - **Menu display**: renders `AutocompleteMenu` at approximate position below cursor
   - **Keyboard handlers**: Enter (execute selection), Escape (close), ArrowUp/ArrowDown (navigate items)
   - **Selection execution**: removes typed token, creates CardElementNode via `$transformTextToCard`, special-cases `declare_var_mut` to insert "mut " after "let ", places cursor in first slot
   - **Type simplifications**: used `(node as any).__slot` instead of the complex type assertion from the plan; `$createTextNode` imported from `lexical` (not `require`)

## Deviations from Plan

- Renamed from `.ts` to `.tsx` (required for JSX rendering)
- Removed unused `$getRoot` import
- Fixed `while` loop in `declare_var_mut` handler (extracted `getNextSibling()` to separate variable with null check to satisfy TS)
- Used `React.ReactNode` return type instead of `null` or `JSX.Element` (JSX namespace unavailable without explicit React import)

## Compilation

- `tsc --noEmit`: 0 new errors (93 total, all pre-existing)
- Our files have zero TS errors

## Integration Points

- Consumes: `autocomplete-rules.ts` (getAutocompleteItems, detectCardKindFromItem), `SlotTextNode`, `CardElementNode`, `$transformTextToCard`
- Produces: `AutocompletePlugin` component ready to register in LexicalComposer

## Post-Review Fixes

**Status:** DONE

Three issues from Task 7 review were fixed on 2026-07-23:

### Fix 1 (Critical) — `card.__cardKind` mutation
- **File:** `AutocompletePlugin.tsx`
- **Change:** Deleted `card.__cardKind = "declare_var";` (was line 141)
- **Rationale:** `$transformTextToCard` already sets the correct `cardKind` via the `CardElementNode` constructor. The redundant line bypassed `getWritable()` and dangerously mutated a Lexical node internal property directly.

### Fix 2 (High) — `as any` property access on Lexical nodes
- **File:** `AutocompletePlugin.tsx`
- **Changes:**
  - `(node as any).__slot` → `node.getSlotType()` (line 56, in token detection)
  - `(node as any).setTextContent(...)` → `node.setTextContent(...)` (line 132, in selection execution)
- **Rationale:** Using `getSlotType()` (a public API on `SlotTextNode`) instead of accessing the internal `__slot` property. Using `TextNode`'s public `setTextContent()` API instead of `(node as any)` cast.

### Fix 3 (High) — Duplicate React keys in AutocompleteMenu
- **File:** `AutocompleteMenu.tsx`
- **Change:** `key={item.kind}` → `key={\`${item.kind}-${item.label}\`}`
- **Rationale:** `TYPE_ITEMS` all have `kind: "type"`, causing duplicate React keys. Including `item.label` makes each key unique.

### Verification

| Check | Result |
|-------|--------|
| Vitest (unchanged files) | 26/29 pass, 189/195 pass — 3 failing files (6 tests) are pre-existing in `code-op-roundtrip.test.ts` |
| TS errors in fixed files | 0 |
| Total TS errors (unchanged) | 93 (pre-existing baseline) |
