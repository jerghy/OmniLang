# learn-omniview Stdlib & Feature Upgrade Plan

> **Goal:** Demonstrate all newly-implemented stdlib modules and compiler fixes in the learn-omniview project. Every change must be independently compilable and testable.

**Source:** `D:/space/OmniLang/test/projects/learn-omniview/src/`
**Compile:** `cd D:/space/OmniLang && python -m omnilang build test/projects/learn-omniview/src/main.omni --target omniview`
**Test:** `cd D:/space/OmniLang/omni-compiler && cargo test`

---

## Task 1: Add persistence (storage) to all 3 stores

**Files:** `src/stores/TodoStore.omni`, `src/stores/ThemeStore.omni`, `src/stores/NotificationStore.omni`

**New stdlib:** `storage_get`, `storage_set`, `json_stringify`, `json_parse`

### TodoStore — persist tasks to localStorage
- Add `import { storage_get, storage_set } from "std/storage"`
- Add `import { json_parse, json_stringify } from "std/json"`
- Add `action load()`:
  ```
  action load() {
      let raw = storage_get("todo-items");
      if raw != null {
          items = json_parse(raw);
      }
  }
  ```
- Add `action save()` — `storage_set("todo-items", json_stringify(items));`
- Call `save()` at end of `add()`, `removeLast()`, `addTask()`

### ThemeStore — persist theme preference
- Add `import { storage_get, storage_set } from "std/storage"`
- Add `action load()` — reads `"theme-mode"` from storage, sets `mode` if found
- Call `storage_set("theme-mode", mode)` in `toggle()` and `setMode()`

### NotificationStore — persist notification history
- Add `import { storage_get, storage_set } from "std/storage"`
- Call `storage_set("notification-count", ...)` in `add()` after push
- Add `action clearAll()` that also clears storage key

**Verify:** `cargo test` passes; compile project to OmniView.

---

## Task 2: Use math stdlib in StatsDashboard

**File:** `src/components/StatsDashboard.omni`

- Add `import { math_abs, math_random, math_ceil, math_floor, math_sqrt } from "std/math"`
- Replace manual `counter % 2` with `math_abs(counter)` in a display context
- Add `signal randomValue: f64 = 0.0` — NOT computed (no deps → never re-computes)
- Add `fn rollDice()` — `randomValue = math_random()`; call from `@onPress` button
- Display `math_ceil(randomValue * 100)` and `math_floor(randomValue * 100)` in the template
- Use `math_sqrt(counter)` for a "square root display" in the template

**Verify:** `cargo test`; compile and check OmniView output contains `Math.abs`, `Math.random`, `Math.ceil`, `Math.floor`, `Math.sqrt`.

---

## Task 3: Use string stdlib in ValidationInput + FormPage

**Files:** `src/components/ValidationInput.omni`, `src/components/FormPage.omni`

### ValidationInput
- Add `import { string_length, string_contains, string_trim } from "std/string"`
- Replace `.len()` with `string_length(input)` in validation action
- Use `string_contains(input, "@")` for email validation
- Use `string_trim(input)` before validation

### FormPage  
- Add `import { string_length, string_trim, string_starts_with } from "std/string"`
- Validate `name` length >= 2 with `string_length(name)`
- Validate `email` with `string_contains(email, "@")`
- Use `string_trim(message)` to strip whitespace before display

**Verify:** `cargo test`; compile and check JS output.

---

## Task 4: Use array stdlib in DataTable + TodoStore

**Files:** `src/components/DataTable.omni`, `src/stores/TodoStore.omni`

### DataTable
- Add `import { array_sort, array_map, array_filter, array_length, array_contains } from "std/array"`
- Replace `.len()` with `array_length(rows)`
- Replace manual sort toggle with `rows = array_sort(rows)` (stdlib returns sorted copy) 
- Add a "filter active" function using `array_filter`
- Display filtered count

### TodoStore
- Add `import { array_push, array_pop, array_length, array_contains, array_is_empty } from "std/array"`
- Replace `.push()` with `items = array_push(items, text)` (stdlib returns the mutated array)
- Replace `.pop()` with `array_pop(items)` (returns popped element, same as built-in)
- Replace `.len()` with `array_length(items)`
- Use `array_is_empty(items)` in getter

**Verify:** `cargo test`; compile and check JS uses `[...arr].sort()`, `.filter()`, `.map()`.

---

## Task 5: Use console stdlib for structured logging

**Files:** `src/components/AddTodo.omni`, `src/components/Timer.omni`, `src/pages/DataFetching.omni`

- Add `import { console_log, console_warn, console_error } from "std/console"`
- AddTodo: replace `println "AddTodo mounted";` with `console_log("AddTodo mounted");`
- AddTodo: add `console_warn("Input is empty");` when trying to add empty todo
- Timer: add `console_log("Timer started");` / `console_log("Timer stopped");`
- DataFetching: use `console_error(errorMsg);` in error branch instead of plain display
- DataFetching: use `console_log("Fetch completed");` on success
- Note: `console_log/warn/error` are function calls (not statements), use as `console_log(msg);`

**Verify:** `cargo test`; compile and check JS uses `console.log`, `console.warn`, `console.error`.

---

## Task 6: Add @effect auto-save to UserSettings + TodoPage

**Files:** `src/pages/UserSettings.omni`, `src/pages/TodoPage.omni`

### UserSettings — auto-save on change
- Already has mount-based load and button-based save
- Add `@effect(username, fontSize) { ... }` that auto-saves when these signals change
- Keeps the explicit save button as well

### TodoPage — effect-based analytics
- Add `@effect(TodoStore.items) { console_log("Total items: " + TodoStore.totalCount.to_string()); }`
- Uses store signal `items` as dep (getter `totalCount` is not trackable — it's a computed property, not a signal)
- Note: effect body reads the getter, but tracking is on the signal

**Verify:** `cargo test`; compile and check JS contains `__omni_effect(`.

---

## Task 7: Use match expression in validation.omni

**File:** `src/lib/validation.omni`

- Currently defines `type ValidationResult` but uses it minimally
- Add `match result { Valid { message } => { ... }, Invalid { errors } => { ... }, Pending => { ... } }` 
- Add `pub fn format_result(result: ValidationResult) -> String` that uses `match` to return different messages
- Demonstrates `match` with variant binding in a pure-logic context

**Verify:** `cargo test`; compile and check JS contains `_variant` match logic.

---

## Task 8: Add closure-based sorting in FilterBar + DataTable

**Files:** `src/components/FilterBar.omni`, `src/components/DataTable.omni`

### DataTable
- Currently has a `|s| s + 5` closure example
- Add a signal `sortKey: String = "name"`  
- Use closure `|a, b| a.name` for custom sort comparison with `array_sort`

### FilterBar
- Add `array_filter` to filter by search text using a closure-like callback
- Demonstrates closure/arrow function in event context

**Verify:** `cargo test`; compile and check JS arrow function output.

---

## Task 9: Connect theme.omni to ThemeStore and SettingsPage

**Files:** `src/shared/theme.omni`, `src/stores/ThemeStore.omni`, `src/pages/SettingsPage.omni`

- ThemeStore already has hardcoded color getters. Add a note/comment referencing AppTheme tokens
- In SettingsPage, add a `@for color in themeColors { ... }` that displays color swatches
- Define `const themeColors` array referencing theme token names
- Demonstrates `const` + `@for` integration

**Verify:** `cargo test`; compile.

---

## Task 10: Add inline style and class-map demos to TabPanel

**File:** `src/components/TabPanel.omni`

- Already uses `style:` and `class:` extensively — good reference
- Add `computed activeStyle` that returns a style object
- Demonstrate dynamic inline style via computed signal
- Demonstrate class toggling with `class: { active: isActive("home"), ... }`

**Verify:** `cargo test`; compile and check CSS class toggling JS.

---

## Summary

| Task | Files Modified | New Features Exercised |
|------|---------------|----------------------|
| 1 | 3 stores | storage_get/set, json_parse/stringify |
| 2 | StatsDashboard | math_abs, _random, _ceil, _floor, _sqrt |
| 3 | ValidationInput, FormPage | string_length, _contains, _trim, _starts_with |
| 4 | DataTable, TodoStore | array_sort, _map, _filter, _length, _contains, _is_empty, _push, _pop |
| 5 | AddTodo, Timer, DataFetching | console_log, _warn, _error |
| 6 | UserSettings, TodoPage | @effect with signal deps |
| 7 | validation.omni | match with variant binding |
| 8 | FilterBar, DataTable | closures + array_sort/filter |
| 9 | theme.omni, ThemeStore, SettingsPage | theme + @for + const |
| 10 | TabPanel | computed style, class-map toggling |

**Total:** 10 tasks, ~17 files modified, 48 stdlib functions exercised across 4 new modules + compiler fixes verified via actual OmniView compilation.
