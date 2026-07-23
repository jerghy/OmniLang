# Task 6 Report: Context-aware autocomplete rules

**Status:** DONE

**Files created:**
- `poiesis/src/components/canvas/code/lexical/autocomplete/autocomplete-rules.ts`
- `poiesis/src/components/canvas/code/lexical/autocomplete/__tests__/autocomplete-rules.test.ts`

**Test results:**

## Step 2 (RED) -- FAIL:
```
 RUN  v4.1.9
 FAIL  src/components/canvas/code/lexical/autocomplete/__tests__/autocomplete-rules.test.ts
Error: Failed to resolve import "../autocomplete-rules" ... Does the file exist?
 Tests:  0 tests
```

## Step 4 (GREEN) -- all passed:
```
 RUN  v4.1.9

 Test Files  1 passed (1)
      Tests  6 passed (6)
```

**Commit:** `88e873a` -- "feat(poiesis): add context-aware autocomplete rules with prefix filtering"

**Summary:**
- Implemented `AutocompleteItem` and `AutocompleteContext` interfaces
- `getAutocompleteItems(token, context)` returns filtered candidates based on:
  - `slotType === "type"` -- type names (i32, String, f64, bool, etc.)
  - `slotType === "value" | "condition" | "arg"` -- expression literals (true, false, null)
  - Default -- statement-level items (let, if, while, for, return, etc.)
- Prefix filtering via `label.toLowerCase().startsWith(lower)`
- `detectCardKindFromItem()` maps autocomplete kinds to `CardKind` for card creation
