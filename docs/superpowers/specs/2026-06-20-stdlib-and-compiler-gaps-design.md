# Standard Library & Compiler Gap Closure — Design Spec

**Date:** 2026-06-20
**Status:** Design approved, ready for implementation plan

---

## Part 1: Standard Library Design

### Current State

3 modules, 7 functions, mature registration system:

| Module | Functions | Async | Available Targets |
|--------|-----------|-------|-------------------|
| `http` | http_send, http_get, http_post | Yes | All 9 |
| `json` | json_parse, json_stringify | No | All 9 |
| `storage` | storage_get, storage_set | No | JS/React/Vue/OmniView/Flutter (5) |

### Registration Pattern

Every function follows: `StdlibFn { module, name, available_targets, codegen: fn(&[String], Target) -> String, is_async }`.

Import syntax: `import { fn_name } from "std/<module>"`.

### New Modules

#### Batch 1 (P0) — Runtime Essentials

**`math`** — All 9 targets, all sync:
`abs`, `min`, `max`, `ceil`, `floor`, `round`, `random`, `sqrt`, `pow`
(JS: `Math.*`, Python: `math.*`, Rust: `f64::` methods, Dart: `dart:math`)

**`string`** — All 9 targets, all sync:
`length`, `concat`, `split`, `join`, `replace`, `substring`, `contains`, `to_upper`, `to_lower`, `trim`, `starts_with`, `ends_with`

**`array`** — All 9 targets, all sync:
`length`, `get`, `push`, `pop`, `remove`, `map`, `filter`, `find`, `sort`, `contains`, `is_empty`
*(Note: map/filter/find require closure codegen — defer to P1 if blockers)*

**`console`** — All 9 targets, all sync:
`log`, `warn`, `error`
(JS: `console.*`, Python: `print`/`logging`, Rust: `println!`/`eprintln!`, Dart: `print`)

#### Batch 2 (P1) — Common Utilities

**`datetime`** — All 9 targets:
`now` (sync, ISO 8601), `format` (sync), `timestamp` (sync, Unix ms)

**`crypto`** — All 9 targets:
`sha256` (sync, hex digest), `random_uuid` (sync, UUID v4), `random_bytes` (sync, base64)

#### Batch 3 (P2) — Platform-Specific

**`fs`** — Python/Rust/Dart/JS/TS only (5 logic targets):
`read` (async), `write` (async), `exists` (sync)

**`timer`** — OmniView/React/Vue/Flutter/Dioxus only (5 UI targets):
`delay` (callback), `interval` (callback), `cancel`

### Effort Estimate

| Batch | Modules | Functions | Est. Days |
|-------|---------|-----------|-----------|
| P0 | 4 | 36 | 3-4 |
| P1 | 2 | 6 | 2-3 |
| P2 | 2 | 6 | 1-2 |
| **Total** | **8** | **48** | **6-9** |

---

## Part 2: Compiler Gap Closure

### P0 — Critical (semantic holes, expression codegen gaps)

| ID | Gap | Files | Impact |
|----|-----|-------|--------|
| P0-1 | Semantic Pass 4 no-op: bind-on-signals, @for key, break/continue in loops, await-in-non-async | `semantic/mod.rs:786` | Silent acceptance of invalid programs |
| P0-2 | TypeMismatch lint: always returns empty vec | `semantic/lints.rs:172-198` | No type checking at all |
| P0-3 | IndexExpr returns "null" in 6 backends: React, Vue, Flutter, Dioxus, Dart, Rust, JS/TS | 7 codegen files | `arr[0]` broken on 7/9 targets |
| P0-4 | ClosureExpr returns "null" in 7 backends: React, Vue, Flutter, Dioxus, Dart, Rust, JS/TS | 7 codegen files | Closures broken on 7/9 targets |
| P0-5 | BlockExpr preceding statements silently dropped in ALL backends | All 9 codegen files | Side-effecting statements in block exprs lost |
| P0-6 | MatchExpr returns "null"/"None" in 5 backends: React, Vue, Flutter, Dart, Dioxus, Python, Rust | 7 codegen files | Pattern matching broken on 7/9 targets |

### P1 — High (cross-backend inconsistencies)

| ID | Gap | Files | Impact |
|----|-----|-------|--------|
| P1-1 | Mutation ops (mul/div/mod/push/pop) missing in Python, Dart, Rust, React, Vue, Flutter | 7 codegen files | Compound assignment silent failure |
| P1-2 | EventEmitStmt: comment-only placeholder in ALL non-OmniView backends | 8 codegen files | Custom events don't work outside OmniView |
| P1-3 | Slot content projection: placeholder in React, Vue, Dioxus, Flutter | 4 codegen files | Named slots broken on 4 UI targets |
| P1-4 | Inline styles: placeholder in React, Vue | `react.rs:347`, `vue.rs:361` | style: prop ignored |
| P1-5 | Store action event handlers: TODO in React, Vue, Dioxus, Flutter | 4 codegen files | @click={store.action} broken |
| P1-6 | Lifecycle hooks (@mount/@unmount/@effect): TODO in React, Vue, Dioxus, Flutter | 4 codegen files | Component lifecycles missing |
| P1-7 | Rust getter return type inference missing | `rust.rs:216` | Rust store getters lack types |

### P2 — Medium (lint rules, edge cases)

| ID | Gap | Files | Impact |
|----|-----|-------|--------|
| P2-1 | Lint Batch 2: unused signals/functions/computed/props | `lints.rs:310` | 4 missing dead-code checks |
| P2-2 | MatchExpr variant bindings ignored | `omniview.rs` | Ok(value) pattern doesn't extract value |
| P2-3 | Dynamic class toggling not supported in JS runtime | `ir/mod.rs:919` | class: { active: isActive } static only |
| P2-4 | BlockStmtIR falls through to unhandled in OmniView | `omniview.rs:639` | Bare block statements produce placeholder |

### P3 — Long-term

| ID | Gap |
|----|-----|
| P3-1 | @for uses `new Function()` eval — should be pre-compiled |
| P3-2 | SSR/hydration (full P0 roadmap item, separate spec) |

### Effort Estimate

| Priority | Items | Est. Days |
|----------|-------|-----------|
| P0 | 6 | 13-19 |
| P1 | 7 | 9-14 |
| P2 | 4 | 3 |
| P3 | 2 | exploratory |
| **Total** | **19** | **25-36** |

---

## Combined Phasing

### Phase 1: Stdlib Batch 1 + P0 Compiler Gaps
- 36 stdlib functions across math/string/array/console
- 6 critical compiler gaps (semantic validation, type checking, expression codegen)
- Est: 16-23 days

### Phase 2: Stdlib Batch 2 + P1 Compiler Gaps
- 6 stdlib functions across datetime/crypto
- 7 high-priority compiler gaps
- Est: 11-17 days

### Phase 3: Stdlib Batch 3 + P2 Compiler Gaps
- 6 stdlib functions across fs/timer
- 4 medium compiler gaps
- Est: 4-5 days

### Phase 4: P3 Long-term
- @for optimization, SSR exploration
- Est: exploratory
