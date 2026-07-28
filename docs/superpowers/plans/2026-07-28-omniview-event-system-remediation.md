# OmniView Event System — Post-ADR-018 Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 3 CRITICAL/HIGH data-loss bugs and 4 MEDIUM correctness/quality gaps discovered in the 2026-07-28 dual-specialist review (Compiler Architect Dr. Lin Wei + TS/React Specialist Emma Rodriguez) of OmniView's event system.

**Architecture:** 10 targeted fixes in the codegen layer only — no IR/semantic/lowering changes needed. Each fix is 1–20 lines. The core pattern: thread event payload data through the "last mile" of codegen (generated closure bodies in InlineFn handlers across 4 non-OmniView backends).

**Tech Stack:** Rust (omnilang compiler crate), OmniLang IR, 5 codegen backends

**Source Reviews:**
- [ADR-018: OmniView Event System Architecture](.architecture/decisions/adrs/ADR-018-omniview-event-system-architecture.md) — prior fixes applied (12 commits)
- Compiler & Language Architect Review (Dr. Lin Wei) — 2026-07-28
- TypeScript & React Specialist Review (Emma Rodriguez) — 2026-07-28

---

## File Map

| File | Purpose | Changes |
|------|---------|---------|
| `omnilang/src/codegen/react/utils.rs:396` | React custom event payload | 1 line fix |
| `omnilang/src/codegen/react/expr.rs:189` | React zero-arg emit no-op | 1 line fix |
| `omnilang/src/codegen/react/view.rs:306-329` | React InlineFn event extraction | ~10 lines |
| `omnilang/src/codegen/dioxus/expr.rs:260` | Dioxus custom event payload | 1 line fix |
| `omnilang/src/codegen/dioxus/expr.rs:55-88` | Dioxus `onTap` mapping | 1 line add |
| `omnilang/src/codegen/dioxus/view.rs:70-82` | Dioxus InlineFn event extraction | ~8 lines |
| `omnilang/src/codegen/vue/expr.rs:123-158` | Vue `onTap` mapping | 1 line add |
| `omnilang/src/codegen/vue/template.rs:68-84` | Vue InlineFn event extraction | ~8 lines |
| `omnilang/src/codegen/flutter/motion_widgets.rs:660` | Flutter custom event payload | 1 line fix |
| `omnilang/src/codegen/flutter/view.rs:176-189` | Flutter InlineFn event extraction | ~8 lines |
| `omnilang/src/codegen/omniview/view/element.rs:399-418` | @for data-event flags encoding | ~5 lines |
| `omnilang/src/codegen/omniview/list_render.rs:241-324` | @for delegate handler flag decode | ~10 lines |

---

## Task 1: Fix custom event payload — React/Dioxus/Flutter backends (P0) 🔴

**Files:**
- Modify: `omnilang/src/codegen/react/utils.rs:396`
- Modify: `omnilang/src/codegen/dioxus/expr.rs:260`
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs:660`

**Context:** The shared default `emit_event_object` at `crates/omnilang-core/src/codegen/event.rs:170` correctly returns `e.detail` for custom events (`event_data == None`). But React, Dioxus, and Flutter have their own `emit_event_object_*` variants with stale `None` branches that discard the payload.

**Evidence** (verified from source):

| Backend | File:Line | Current (broken) | Expected |
|---------|-----------|-------------------|----------|
| React | `react/utils.rs:396` | `None => return "{}".to_string()` | `e.detail` |
| Dioxus | `dioxus/expr.rs:260` | `None => return "FxHashMap::default()"` | `e.detail()` |
| Flutter | `flutter/motion_widgets.rs:660` | `None => return "{}".to_string()` | `e.detail` |

**Concrete scenario:** Component declares `event onItemSelected: Item`, child emits `emit onItemSelected(item)`. In OmniView backend → parent receives `item` via `e.detail`. In React/Dioxus/Flutter → parent receives `{}` (empty). This silently breaks cross-target event contracts.

- [ ] **Step 1: Run existing tests to establish baseline**

```bash
cargo test -p omnilang -- react --quiet
cargo test -p omnilang -- dioxus --quiet
cargo test -p omnilang -- flutter --quiet
```

Expected: All PASS

- [ ] **Step 2: Fix React `emit_event_object_react` — `omnilang/src/codegen/react/utils.rs:396`**

Replace:
```rust
None => return "{}".to_string(),
```
With:
```rust
None => return format!("{}.detail", raw_var),
```

- [ ] **Step 3: Fix Dioxus `emit_event_object_dioxus` — `omnilang/src/codegen/dioxus/expr.rs:260`**

Replace:
```rust
None => return "FxHashMap::default()".to_string(),
```
With:
```rust
None => return format!("{}.detail()", raw_var),
```

- [ ] **Step 4: Fix Flutter `emit_event_object_flutter` — `omnilang/src/codegen/flutter/motion_widgets.rs:660`**

Replace:
```rust
None => return "{}".to_string(),
```
With:
```rust
None => return format!("{}.detail", raw_var),
```

- [ ] **Step 5: Verify tests pass**

```bash
cargo test -p omnilang -- react --quiet
cargo test -p omnilang -- dioxus --quiet
cargo test -p omnilang -- flutter --quiet
cargo test --workspace --quiet
```

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/utils.rs omnilang/src/codegen/dioxus/expr.rs omnilang/src/codegen/flutter/motion_widgets.rs
git commit -m "fix(events): preserve custom event payload via .detail in React/Dioxus/Flutter

Custom component events (event_data=None) were returning {} or
FxHashMap::default() instead of the actual CustomEvent.detail
payload. The shared emit_event_object in omnilang-core handles
this correctly; these backend-specific variants had a stale
None branch that was not updated during the ADR-018 D4 fix
(CustomEvent dispatch model decision).
"
```

---

## Task 2: Thread InlineFn event extraction — Vue backend (P1) 🟠

**File:**
- Modify: `omnilang/src/codegen/vue/template.rs:68-84`

**Context:** `_event_obj` is computed (line 69: `let _event_obj = emit_event_object_js(...)`) but never integrated into the handler body. The body still uses raw `e` parameter, where `e.value` is `undefined` on a DOM event. The fix: augment the event object with extracted fields via `Object.assign`.

**Current output (broken):**
```html
@click="(e) => {
  count = e.value;  // undefined — e is raw Event, has no .value
}"
```

**Fixed output:**
```html
@click="(e) => {
  Object.assign(e, { value: e.target?.value ?? "" });
  count = e.value;  // now resolves to extracted value
}"
```

- [ ] **Step 1: Run existing Vue tests to establish baseline**

```bash
cargo test -p omnilang -- vue --quiet
```

Expected: All PASS

- [ ] **Step 2: Thread `_event_obj` into handler body — `omnilang/src/codegen/vue/template.rs:68-84`**

Replace lines 68-84:
```rust
if event_data.is_some() {
    let _event_obj = emit_event_object_js(event_data, used_fields, "e");
    let mut hw = CodeWriter::new();
    for stmt in &body.statements {
        generate_statement_vue(&mut hw, stmt);
    }
    let content = hw.into_string();
    let mut result = String::from("(e) => {\n");
    for line in content.lines() {
        result.push_str(&format!("        {}\n", line));
    }
    result.push_str("      }");
    attrs.push(format!("@{}={}", vue_event, result));
}
```
With:
```rust
if event_data.is_some() {
    let event_obj = emit_event_object_js(event_data, used_fields, "e");
    let mut hw = CodeWriter::new();
    hw.emit_fmt(format_args!("Object.assign(e, {});", event_obj));
    for stmt in &body.statements {
        generate_statement_vue(&mut hw, stmt);
    }
    let content = hw.into_string();
    let mut result = String::from("(e) => {\n");
    for line in content.lines() {
        result.push_str(&format!("        {}\n", line));
    }
    result.push_str("      }");
    attrs.push(format!("@{}={}", vue_event, result));
}
```

Key change: `_event_obj` → `event_obj` (used, not discarded) + added `Object.assign(e, ...)` as the first statement in the handler body.

- [ ] **Step 3: Verify Vue tests pass**

```bash
cargo test -p omnilang -- vue --quiet
```

Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/vue/template.rs
git commit -m "fix(events): thread InlineFn event data extraction into Vue handler body

Previously _event_obj was computed and discarded (underscore prefix
indicated unused). The handler body referenced e.value on the raw
DOM event parameter, where none of the extracted fields exist.

Fix: Use Object.assign(e, event_obj) to augment the event parameter
with extracted fields, so body references like e.value resolve to
e.target?.value ?? '' as intended by the event schema.
"
```

---

## Task 3: Thread InlineFn event extraction — Dioxus backend (P1) 🟠

**File:**
- Modify: `omnilang/src/codegen/dioxus/view.rs:70-82`

**Context:** Same pattern as Vue — `_event_obj` computed at line 71, never used. Body references `e.field` on raw web_sys event.

- [ ] **Step 1: Run existing Dioxus tests to establish baseline**

```bash
cargo test -p omnilang -- dioxus --quiet
```

Expected: All PASS

- [ ] **Step 2: Thread `_event_obj` into handler body — `omnilang/src/codegen/dioxus/view.rs:70-82`**

Replace lines 70-82:
```rust
if event_data.is_some() {
    let _event_obj = emit_event_object_dioxus(event_data, used_fields, "e");
    let mut hw = CodeWriter::new();
    for stmt in &body.statements {
        generate_statement_dioxus(&mut hw, stmt);
    }
    let content = hw.into_string();
    let mut result = String::from("move |e| {\n");
    for line in content.lines() {
        result.push_str(&format!("                {}\n", line));
    }
    result.push_str("            }");
    attrs.push(format!("{}: {},", dioxus_event, result));
}
```
With:
```rust
if event_data.is_some() {
    let event_obj = emit_event_object_dioxus(event_data, used_fields, "e");
    let mut hw = CodeWriter::new();
    // In Dioxus, instead of Object.assign (JS), use individual field bindings
    // since Dioxus handlers receive web_sys events with Rust-accessible fields.
    // However, custom extracted fields (value, checked, etc.) aren't on web_sys types.
    // Use a local shadow: let value = extract; let checked = extract;
    let fields: Vec<&str> = match used_fields {
        Some(ref f) if !f.is_empty() => f.iter().map(|s| s.as_str()).collect(),
        _ => vec![],
    };
    for field in &fields {
        // Extract just the field value from the event_obj expression and bind locally
        // Since we can't destructure FxHashMap easily, emit individual let bindings
        // that shadow the raw event field access.
        let extract = field_extract_dioxus(event_data.as_ref().unwrap(), field, "e");
        hw.emit_fmt(format_args!("let {} = {};", field, extract));
    }
    for stmt in &body.statements {
        generate_statement_dioxus(&mut hw, stmt);
    }
    let content = hw.into_string();
    let mut result = String::from("move |e| {\n");
    for line in content.lines() {
        result.push_str(&format!("                {}\n", line));
    }
    result.push_str("            }");
    attrs.push(format!("{}: {},", dioxus_event, result));
}
```

Note: The Dioxus backend generates Rust code (not JS), so `Object.assign` is not available. Instead, we emit `let field = extracted_value;` for each used field, which shadows the body's `e.field` references — because the body emits `e.field` at codegen level, but with local `let field = e.target().value().unwrap_or_default()` bindings, the reference becomes a local variable access. However, this requires the body to reference just `field` not `e.field`.

**⚠️  Complexity note:** Dioxus InlineFn requires deeper changes than the other backends because:
1. Dioxus generates Rust code (web_sys), not JS
2. The body IR emits `e.field` for field accesses, but web_sys doesn't have `.value` on raw events
3. Individual `let` bindings shadow the local scope, but the body still writes `e.value` not `value`

**Mitigation:** For Dioxus, the field accesses in the body need to be rewritten from `e.field` to just `field`. This requires intercepting the codegen of `FieldAccess` expressions within the InlineFn scope. Given the complexity, this task may require a follow-up design pass. The fix in this plan focuses on the immediate correct behavior: emit the field extraction bindings so the values are at least computed, and document the remaining body-reference issue.

**Alternative (more invasive but correct):** Add an `event_param_name: Option<&str>` parameter to `generate_statement_dioxus` and `format_expr_dioxus`, and check: when formatting `FieldAccess(VarRef(param), field)` where `param == event_param_name`, emit just the field name instead of `param.field`.

Given the complexity, this task should be attempted only after Tasks 1, 2, 4, 6, and 7 are complete and verified.

- [ ] **Step 3: Verify Dioxus tests pass**

```bash
cargo test -p omnilang -- dioxus --quiet
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/dioxus/view.rs
git commit -m "fix(events): emit event field extraction bindings in Dioxus InlineFn handlers

Previously _event_obj was computed and discarded. This adds individual
let bindings for each used event field to the handler body preamble.
Note: full fix requires intercepting FieldAccess codegen to redirect
e.field to the local binding — tracked as follow-up.
"
```

---

## Task 4: Thread InlineFn event extraction — Flutter backend (P1) 🟠

**File:**
- Modify: `omnilang/src/codegen/flutter/view.rs:176-189`

**Context:** Same pattern — `_event_obj` computed at line 177-178, never injected into handler body. Flutter generates Dart code.

- [ ] **Step 1: Run existing Flutter tests to establish baseline**

```bash
cargo test -p omnilang -- flutter --quiet
```

Expected: All PASS

- [ ] **Step 2: Thread event extraction into Flutter InlineFn handler — `omnilang/src/codegen/flutter/view.rs:176-189`**

Replace lines 176-189:
```rust
if event_data.is_some() {
    let _event_obj =
        emit_event_object_flutter(handler.event_data(), handler.used_fields(), "e");
    let mut hw = CodeWriter::new();
    hw.emit("(e) {");
    hw.indent();
    for stmt in &body.statements {
        generate_statement_flutter(&mut hw, stmt);
    }
    hw.dedent();
    hw.emit("}");
    let result = hw.into_string().trim().to_string();
```
With:
```rust
if event_data.is_some() {
    let event_obj =
        emit_event_object_flutter(handler.event_data(), handler.used_fields(), "e");
    let fields: Vec<String> = match handler.used_fields() {
        Some(ref f) if !f.is_empty() => f.clone(),
        _ => vec![],
    };
    let mut hw = CodeWriter::new();
    hw.emit("(e) {");
    hw.indent();
    // Emit individual field extractions as local variables
    // This shadows e.field references in the body with extracted values
    for field in &fields {
        let extract = field_extract_flutter(
            handler.event_data().as_ref().unwrap(),
            field,
            "e",
        );
        hw.emit_fmt(format_args!("final {} = {};", field, extract));
    }
    for stmt in &body.statements {
        generate_statement_flutter(&mut hw, stmt);
    }
    hw.dedent();
    hw.emit("}");
    let result = hw.into_string().trim().to_string();
```

Same complexity note as Dioxus: individual `final field = extract;` bindings shadow the local scope, but the body's `e.field` references need to be redirected to just `field`. The `field_extract_flutter` function is imported from `super::motion_widgets::field_extract_flutter`.

- [ ] **Step 3: Verify Flutter tests pass**

```bash
cargo test -p omnilang -- flutter --quiet
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs
git commit -m "fix(events): emit event field extraction bindings in Flutter InlineFn handlers

Previously _event_obj was computed and discarded. This adds final
variable bindings for each used event field to the handler body preamble.
Note: full fix requires FieldAccess redirection — tracked as follow-up.
"
```

---

## Task 5: Thread InlineFn event extraction — React backend (P1) 🟠

**File:**
- Modify: `omnilang/src/codegen/react/view.rs:306-329`

**Context:** The React InlineFn path has NO event extraction call at all (no `_event_obj` computation). The body directly references `e.value` on React's SyntheticEvent. The fix: add event extraction via `emit_event_object_react` + `Object.assign` to augment the synthetic event.

- [ ] **Step 1: Run existing React tests to establish baseline**

```bash
cargo test -p omnilang -- react --quiet
```

Expected: All PASS

- [ ] **Step 2: Add event extraction to React InlineFn handler — `omnilang/src/codegen/react/view.rs:306-329`**

The current React InlineFn path at lines 306-329 does NOT call `emit_event_object_react` for either the binding case or the non-binding case. We need to add event data extraction when `event_data.is_some()`.

Replace lines 306-329:
```rust
EventHandlerIR::InlineFn { body, params, .. } => {
    if let Some(setter_call) = binding_events_map.remove(&react_event) {
        // Generate body with binding setter prepended
        let mut merged_hw = CodeWriter::new();
        merged_hw.emit_fmt(format_args!("{};", setter_call));
        for stmt in &body.statements {
            generate_statement_tsx(&mut merged_hw, stmt, signal_names);
        }
        let content = merged_hw.into_string();
        let params_str = if params.is_empty() {
            "e".to_string()
        } else {
            params.join(", ")
        };
        let mut result = format!("({}) => {{\n", params_str);
        for line in content.lines() {
            result.push_str(&format!("        {}\n", line));
        }
        result.push_str("      }");
        attrs.push(format!("{}={{{}}}", react_event, result));
    } else {
        let handler_str = format_inline_handler(body, signal_names, params);
        attrs.push(format!("{}={{{}}}", react_event, handler_str));
    }
}
```
With:
```rust
EventHandlerIR::InlineFn { body, params, event_data, used_fields, .. } => {
    if let Some(setter_call) = binding_events_map.remove(&react_event) {
        let mut merged_hw = CodeWriter::new();
        merged_hw.emit_fmt(format_args!("{};", setter_call));
        // Emit event data extraction for InlineFn with event_data
        if event_data.is_some() {
            let event_obj = emit_event_object_react(event_data, used_fields, "e");
            merged_hw.emit_fmt(format_args!("Object.assign(e, {});", event_obj));
        }
        for stmt in &body.statements {
            generate_statement_tsx(&mut merged_hw, stmt, signal_names);
        }
        let content = merged_hw.into_string();
        let params_str = if params.is_empty() {
            "e".to_string()
        } else {
            params.join(", ")
        };
        let mut result = format!("({}) => {{\n", params_str);
        for line in content.lines() {
            result.push_str(&format!("        {}\n", line));
        }
        result.push_str("      }");
        attrs.push(format!("{}={{{}}}", react_event, result));
    } else {
        // Non-binding InlineFn: add event extraction before body
        if event_data.is_some() {
            let event_obj = emit_event_object_react(event_data, used_fields, "e");
            let mut merged_hw = CodeWriter::new();
            merged_hw.emit_fmt(format_args!("Object.assign(e, {});", event_obj));
            for stmt in &body.statements {
                generate_statement_tsx(&mut merged_hw, stmt, signal_names);
            }
            let content = merged_hw.into_string();
            let params_str = if params.is_empty() {
                "e".to_string()
            } else {
                params.join(", ")
            };
            let mut result = format!("({}) => {{\n", params_str);
            for line in content.lines() {
                result.push_str(&format!("        {}\n", line));
            }
            result.push_str("      }");
            attrs.push(format!("{}={{{}}}", react_event, result));
        } else {
            let handler_str = format_inline_handler(body, signal_names, params);
            attrs.push(format!("{}={{{}}}", react_event, handler_str));
        }
    }
}
```

Key addition: `use super::utils::emit_event_object_react;` must be imported at the top of `react/view.rs`.

- [ ] **Step 3: Verify React tests pass**

```bash
cargo test -p omnilang -- react --quiet
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/react/view.rs
git commit -m "fix(events): add event data extraction to React InlineFn handlers

React InlineFn handlers had no emit_event_object_react call at all,
unlike FnRef handlers. The handler body referenced e.value on React's
SyntheticEvent, where e.value is undefined for all event types except
those natively provided by React.

Fix: Add emit_event_object_react + Object.assign(e, eventObj) in both
the binding and non-binding InlineFn code paths, matching the pattern
used by Vue/OmniView backends.
"
```

---

## Task 6: Fix React zero-arg `emit` no-op (P0) 🟡

**File:**
- Modify: `omnilang/src/codegen/react/expr.rs:189`

**Context:** The shared default `format_event_emit_stmt` at `crates/omnilang-core/src/codegen/shared/stmt.rs:369` emits `String::from("()")` for empty args → `onCustomEvent();`. But React's override at `react/expr.rs:189` uses `String::new()` → `onCustomEvent;` (bare expression, no-op).

- [ ] **Step 1: Fix React `format_event_emit_stmt` — `omnilang/src/codegen/react/expr.rs:189`**

Replace:
```rust
let arg_str = if args.is_empty() {
    String::new()
```
With:
```rust
let arg_str = if args.is_empty() {
    String::from("()")
```

- [ ] **Step 2: Verify React tests pass**

```bash
cargo test -p omnilang -- react --quiet
```

Expected: All PASS (existing emit tests should verify the output is `onCustomEvent();` not `onCustomEvent;`)

- [ ] **Step 3: Check existing emit test snapshots**

```bash
cargo test -p omnilang -- emit --quiet
```

If any snapshot tests contain `onCustomEvent;` (bare reference), they should now show `onCustomEvent();`. Update snapshots if needed.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/react/expr.rs
git commit -m "fix(events): emit function call for zero-arg emit in React backend

The React format_event_emit_stmt override used String::new() for empty
args, producing 'onCustomEvent;' (bare expression, no-op). The shared
default correctly uses String::from('()') to produce 'onCustomEvent();'.

This was a React-specific regression from the ADR-018 D2 fix — the
shared default was changed from comment to real call, but React's
override was not updated to match.
"
```

---

## Task 7: Add `onTap` mapping to Vue and Dioxus event mappers (P1) 🟡

**Files:**
- Modify: `omnilang/src/codegen/vue/expr.rs:129` (add `"tap"` → `"click"`)
- Modify: `omnilang/src/codegen/dioxus/expr.rs:56` (add `"tap"` → `"onclick"`)

**Context:** OmniLang normalizes `@onTap` as a cross-platform touch/click event. OmniView maps it to `click` ✅, Flutter maps to `onPressed` ✅, React falls through to `onTap` (works as custom component event). But Vue produces `tap` (no native HTML `tap` event) and Dioxus produces `ontap` (no web-sys `ontap` attribute).

- [ ] **Step 1: Add `tap` mapping to Vue — `omnilang/src/codegen/vue/expr.rs:129`**

Change:
```rust
"click" | "press" | "tap" => "click".into(),
```
(Already correct! Line 129 reads `"click" | "press" | "tap" => "click".into(),`. The Vue mapping is already correct.)

**Correction:** Re-reading the source, `vue/expr.rs:129` already maps `"tap" => "click"`. The review finding was a false positive for Vue. Only Dioxus needs fixing.

- [ ] **Step 2: Add `tap` mapping to Dioxus — `omnilang/src/codegen/dioxus/expr.rs:56`**

Change:
```rust
"click" => "onclick".into(),
```
To:
```rust
"click" | "tap" | "press" => "onclick".into(),
```

- [ ] **Step 3: Verify tests pass**

```bash
cargo test -p omnilang -- dioxus --quiet
cargo test --workspace --quiet
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/dioxus/expr.rs
git commit -m "fix(events): add tap/press mapping to Dioxus event handler

OmniLang's @onTap is a normalized touch/click event. OmniView maps it
to click, Flutter to onPressed, React to onTap (custom component event),
and Vue already had the tap→click mapping. Dioxus was the only backend
missing it, producing 'ontap' which is not a valid web-sys event attribute.
"
```

---

## Task 8: Encode `.prevent`/`.stop` in `@for` data-event attributes (P2) 🟠

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:399-418` (encode flags)
- Modify: `omnilang/src/codegen/omniview/list_render.rs:241-324` (decode flags)

**Context:** The `data-event-*` attribute format (`fn:handlerName`) doesn't encode `needs_prevent_default` or `needs_stop_propagation` flags from `EventHandlerIR`. The `_forHandle` function only calls `preventDefault()` for hardcoded `submit` events (line 249). Outside `@for`, these flags work correctly via the `emit_element_event` function.

**Concrete scenario:** `@onClick.prevent="handler"` inside a `@for` block. Outside `@for`, `e.preventDefault()` is called. Inside `@for`, the `.prevent` modifier is silently ignored.

- [ ] **Step 1: Run existing OmniView tests**

```bash
cargo test -p omnilang -- omniview --quiet
```

- [ ] **Step 2: Encode flags in `data-event-*` — `element.rs:399-418`**

Current format: `fn:handlerName` or `store:handlerName`

New format: `fn:handlerName|pd|sp` where `pd` = needs_prevent_default, `sp` = needs_stop_propagation

Replace the `val` construction at lines 399-403:
```rust
let val = match handler {
    EventHandlerIR::FnRef { r#ref, .. } => {
        let prefixed = format!("{}_{}", self.current_component, r#ref);
        format!("fn:{}", escape_html(&prefixed))
    }
```
With:
```rust
let val = match handler {
    EventHandlerIR::FnRef { r#ref, needs_prevent_default, needs_stop_propagation } => {
        let prefixed = format!("{}_{}", self.current_component, r#ref);
        let flags = match (needs_prevent_default, needs_stop_propagation) {
            (true, true) => "|pd|sp",
            (true, false) => "|pd",
            (false, true) => "|sp",
            (false, false) => "",
        };
        format!("fn:{}{}", escape_html(&prefixed), flags)
    }
```
And the same for `EventHandlerIR::InlineFn` at lines 408-417:
```rust
EventHandlerIR::InlineFn { body, params, needs_prevent_default, needs_stop_propagation, .. } => {
    let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
    let params_str = params.join(", ");
    let _ = writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
    for stmt in &body.statements {
        self.emit_statement(stmt);
    }
    self.js.push_str("}\n");
    let flags = match (needs_prevent_default, needs_stop_propagation) {
        (true, true) => "|pd|sp",
        (true, false) => "|pd",
        (false, true) => "|sp",
        (false, false) => "",
    };
    format!("fn:{}{}", escape_html(&wrapper_name), flags)
}
```

- [ ] **Step 3: Decode flags in `_forHandle` — `list_render.rs:248-252`**

Replace the hardcoded submit-only check at line 249:
```rust
if (e.type === 'submit') e.preventDefault();
```
With:
```rust
const _flags = attr.includes('|') ? attr.split('|').slice(1) : [];
if (_flags.includes('pd')) e.preventDefault();
if (_flags.includes('sp')) e.stopPropagation();
```

And update the `fn:` prefix extraction at line 250-251 to handle the flags:
```rust
if (attr.startsWith('fn:')) {
    const fnKey = attr.includes('|') ? attr.slice(3, attr.indexOf('|')) : attr.slice(3);
    const fn = __omni_forHandlers[fnKey];
```

- [ ] **Step 4: Verify OmniView tests pass**

```bash
cargo test -p omnilang -- omniview --quiet
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs omnilang/src/codegen/omniview/list_render.rs
git commit -m "fix(events): encode preventDefault/stopPropagation flags in @for data-event attrs

Previously data-event-* attributes only stored handler names, causing
.prevent and .stop modifiers to be silently ignored inside @for loops.
Outside @for these flags worked correctly via emit_element_event.

Fix: Extend attribute format from 'fn:handlerName' to
'fn:handlerName|pd|sp', and decode flags in _forHandle to call
e.preventDefault() and e.stopPropagation() as appropriate.
"
```

---

## Task 9: Optimize `@for` event extraction — pass `used_fields` (P3) 🟡

**File:**
- Modify: `omnilang/src/codegen/omniview/list_render.rs:234`

**Context:** The `_extractEvent` switch function calls `emit_event_object(&Some(kind.clone()), &None, "e")` — the `&None` for `used_fields` means ALL fields in the event schema are extracted on every trigger. For a PointerEvent with 11 fields, this allocates an object with `x`, `y`, `button`, `pressure`, `tiltX`, `tiltY`, `pointerType`, `alt`, `ctrl`, `shift`, `meta` even when the handler only needs `x` and `y`.

The `collect_for_events` function already collects event kinds. This task extends it to also collect the union of used fields per event kind.

- [ ] **Step 1: Audit `collect_for_events` function**

Locate and read the `collect_for_events` function in `list_render.rs` to understand its current data collection. Identify where event handlers are scanned and where the per-kind used fields union can be accumulated.

```bash
grep -n "collect_for_events\|for_event_kinds\|for_event_types" omnilang/src/codegen/omniview/list_render.rs
```

- [ ] **Step 2: Collect used_fields union per event kind**

Add a `HashMap<EventDataKind, Vec<String>>` (or similar) to `collect_for_events` that accumulates all `used_fields` for each event kind. For each `EventHandlerIR` encountered in the @for body, call `handler.used_fields()` and merge into the per-kind set.

- [ ] **Step 3: Pass collected used_fields to `emit_event_object`**

In the `_extractEvent` switch generation at line 234, replace:
```rust
emit_event_object(&Some(kind.clone()), &None, "e")
```
With:
```rust
emit_event_object(&Some(kind.clone()), &Some(used_fields_for_kind), "e")
```
Where `used_fields_for_kind` is the union collected in Step 2.

- [ ] **Step 4: Verify tests pass**

```bash
cargo test -p omnilang -- omniview --quiet
cargo test --workspace --quiet
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/list_render.rs
git commit -m "perf(events): pass used_fields to @for _extractEvent to reduce allocations

Previously _extractEvent always emitted all fields from the event schema
(&None used_fields), allocating up to 11 properties per PointerEvent
trigger. This collects the union of used_fields per event kind across
all @for body handlers and passes it to emit_event_object.

Reduces per-event allocations for frequently-fired events (mousemove,
scroll, touchmove) inside @for loops.
"
```

---

## Task 10: Run full test suite + formatting check

- [ ] **Step 1: Full workspace test**

```bash
cargo test --workspace --quiet
```

Expected: All tests PASS (currently 801+)

- [ ] **Step 2: Clippy check**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: Zero warnings

- [ ] **Step 3: Format check**

```bash
cargo fmt --all -- --check
```

Expected: No formatting changes needed. If any, run `cargo fmt --all` and amend the most recent commit.

---

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| InlineFn Object.assign approach conflicts with native event properties | Low | Event schema fields (value, checked, x, y) do not exist on native DOM events at top level. Only `submit` event has a `.submitter` property that could conflict — not in our schema. |
| Dioxus/Flutter InlineFn fix is incomplete (body still references `e.field`) | High | Task 3/4 notes document this limitation. A follow-up plan should address FieldAccess redirection in the Dioxus/Flutter statement generators. |
| Snapshot tests need updating after emit fix | Medium | `--quiet` mode suppresses snapshot diff details. Use `cargo test -p omnilang` without `--quiet` to see full diff if failures occur, then review and accept snapshot changes via `cargo insta review`. |
| @for event flag format change breaks existing `data-event-*` parsing | Very Low | The `|` character is not valid in handler function names and won't appear in existing attribute values. The `attr.slice(3)` → `attr.slice(3, attr.indexOf('|'))` change handles both old and new formats cleanly. |

---

## Completion Checklist

- [ ] Task 1: Custom event payload fix — React/Dioxus/Flutter ✅
- [ ] Task 2: Vue InlineFn event extraction ✅
- [ ] Task 3: Dioxus InlineFn event extraction ✅
- [ ] Task 4: Flutter InlineFn event extraction ✅
- [ ] Task 5: React InlineFn event extraction ✅
- [ ] Task 6: React zero-arg emit no-op ✅
- [ ] Task 7: Dioxus `onTap` mapping ✅
- [ ] Task 8: `@for` .prevent/.stop flag encoding ✅
- [ ] Task 9: `@for` used_fields optimization ✅
- [ ] Task 10: Full test suite + fmt + clippy pass ✅
- [ ] All 10 commits pushed ✅
- [ ] Cross-backend consistency matrix updated ✅

## Remaining (deferred to follow-up plan)

1. **Dioxus/Flutter InlineFn body FieldAccess redirection** — The IR body generates `e.field` for field accesses. After emitting `let field = extract;`, the body should reference `field`, not `e.field`. This requires intercepting the codegen of `ExprIR::FieldAccess` when the object is the event parameter. Not tackled in this plan due to invasiveness.

2. **React event prop types** — Store event parameter types in `ComponentIR.event_names` and use them in generated TypeScript prop interfaces instead of `(value: any) => void`.

3. **Shared `field_extract` base trait** — Reduce ~400 lines of duplication across 4 `field_extract_*` functions.

4. **`onChange` → `input` vs `change` cross-target documentation** — Document the behavioral difference.

5. **Multi-arg CustomEvent detail** — Change `detail: args[0]` to `detail: args.length <= 1 ? args[0] : args` for multi-parameter events.
