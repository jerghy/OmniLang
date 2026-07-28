# OmniView Event System — Deferred Items Follow-Up

> **Source:** docs/superpowers/plans/2026-07-28-omniview-event-system-remediation.md (deferred items)

**Goal:** Complete the 4 deferred items from the event system remediation.

| # | Task | Complexity | LOC |
|---|------|-----------|-----|
| 1 | Multi-arg CustomEvent detail | Low | 1 |
| 2 | Dioxus InlineFn FieldAccess redirection | Medium | ~20 |
| 3 | Flutter InlineFn FieldAccess redirection | Medium | ~20 |
| 4 | React typed event props | High | ~30 |

---

### Task F1: Multi-arg CustomEvent detail

**File:** `omnilang/src/codegen/omniview/store.rs:248`

**Current:** `detail: args[0]` — only first argument passed.
**Fix:** `detail: args.length <= 1 ? args[0] : args` — all args passed.

When a component declares `event onDataChanged(id: i32, name: string)` and emits `emit onDataChanged(1, "Alice")`, currently only `1` is passed as detail. After fix, `[1, "Alice"]` is passed.

- [ ] Change `detail: args[0]` to `detail: args.length <= 1 ? args[0] : args` in `store.rs:248`
- [ ] `cargo test -p omnilang -- omniview` must pass
- [ ] Commit: `fix(events): pass all arguments in CustomEvent detail for multi-arg emits`

---

### Task F2: Dioxus InlineFn FieldAccess redirection

**File:** `omnilang/src/codegen/dioxus/view.rs:70-95`

The Dioxus InlineFn code already emits `let field = extract;` bindings, but the body IR still generates `e.field` (because the codegen doesn't know to redirect). Fix: thread the event_param_name through `FormatExpr` so when it encounters `FieldAccess(VarRef("e"), "value")`, it emits just `value` instead of `e.value()`.

**Approach:** Add `event_param_name: Option<&str>` parameter to the Dioxus expression formatter. In the FieldAccess case, check if the object matches the event param name → emit bare field.

- [ ] Add `event_param_name` field to the InlineFn generation context
- [ ] Intercept `FieldAccess` expressions on the event param → emit just `field_name`
- [ ] `cargo test -p omnilang -- dioxus` must pass
- [ ] Commit: `fix(events): redirect Dioxus InlineFn field access to local extraction bindings`

---

### Task F3: Flutter InlineFn FieldAccess redirection

**File:** `omnilang/src/codegen/flutter/view.rs:176-195`

Same pattern as Dioxus — thread event_param_name through the Flutter expression formatter so `e.value` emits as `value` (the local `final` binding).

- [ ] Add `event_param_name` parameter to Flutter InlineFn expression formatting
- [ ] Intercept `FieldAccess` on event param → emit bare field name
- [ ] `cargo test -p omnilang -- flutter` must pass
- [ ] Commit: `fix(events): redirect Flutter InlineFn field access to local extraction bindings`

---

### Task F4: React typed event props

**File:** `omnilang/src/codegen/react/component/mod.rs:141-143`

**Current:** All event callbacks are `(value: any) => void`.
**Fix:** Use the event's declared parameter type from the Omni source.

The `ComponentIR.event_names` field stores `Vec<String>` (event names only). Need to extend it to store parameter types, or add a parallel `event_types: Vec<(String, Option<String>)>` field.

- [ ] Extend IR: add event parameter types to ComponentIR
- [ ] Use types in React prop interface generation
- [ ] `cargo test -p omnilang -- react` must pass
- [ ] Commit: `feat(events): emit typed event callback props in React component interfaces`

---

## Global Constraints
- `cargo test --workspace` must pass after each task
- `cargo fmt` before each commit
- Conventional commit prefixes
