# OmniView Event System Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 2 HIGH and 4 MEDIUM issues in the OmniView event system identified by the Compiler & Language Architect review.

**Architecture:** The fixes span three layers: (1) IR lowering — add missing `Stmt::EventEmit` lowering, fix unconditional `preventDefault`, collect event handler param names; (2) Codegen — strip `"on"` prefix for Dioxus/Vue/Flutter event mapping, fix Flutter `onOnPress` doubling, add OmniView `format_event_emit_stmt` override; (3) Extraction — normalize custom event dispatch vs extraction model.

**Tech Stack:** Rust (compiler), omnic test suite, snapshot tests

---

### Task 1: H1 — Fix Dioxus event name prefix bug

**Files:**
- Modify: `omnilang/src/codegen/dioxus/expr.rs:48-81`

**Background:** IR event names have `"on"` prefix (`"onPress"`, `"onClick"`). The Dioxus `to_dioxus_event` match table expects unprefixed names (`"click"`, `"change"`). All events fall through to the default case returning the original name unchanged (e.g., `"onPress"` instead of `"onclick"`). React's `to_react_event` already does the right thing at `omnilang/src/codegen/react/utils.rs:205`.

- [ ] **Step 1: Add `strip_prefix("on")` normalization in `to_dioxus_event`**

In `omnilang/src/codegen/dioxus/expr.rs`, replace lines 48-81:

```rust
/// Map OmniView event names to Dioxus event handler prop names.
pub(super) fn to_dioxus_event(event_name: &str) -> String {
    // Strip any existing "on" prefix to normalize (IR may carry "onPress" etc.)
    let stem = event_name.strip_prefix("on").unwrap_or(event_name).to_lowercase();
    match stem.as_str() {
        "click" => "onclick".into(),
        "change" => "onchange".into(),
        "input" => "oninput".into(),
        "submit" => "onsubmit".into(),
        "focus" => "onfocus".into(),
        "blur" => "onblur".into(),
        "keydown" => "onkeydown".into(),
        "keyup" => "onkeyup".into(),
        "keypress" => "onkeypress".into(),
        "mouseenter" => "onmouseenter".into(),
        "mouseleave" => "onmouseleave".into(),
        "mousedown" => "onmousedown".into(),
        "mouseup" => "onmouseup".into(),
        "mousemove" => "onmousemove".into(),
        "mouseover" => "onmouseover".into(),
        "mouseout" => "onmouseout".into(),
        "dblclick" => "ondblclick".into(),
        "scroll" => "onscroll".into(),
        "wheel" => "onwheel".into(),
        "touchstart" => "ontouchstart".into(),
        "touchend" => "ontouchend".into(),
        "touchmove" => "ontouchmove".into(),
        "contextmenu" => "oncontextmenu".into(),
        "drag" => "ondrag".into(),
        "dragstart" => "ondragstart".into(),
        "dragend" => "ondragend".into(),
        "dragover" => "ondragover".into(),
        "dragleave" => "ondragleave".into(),
        "drop" => "ondrop".into(),
        // Custom / unmatched events: prefix with "on"
        _ => format!("on{}", stem),
    }
}
```

- [ ] **Step 2: Run tests and verify**

Run: `cargo test -p omnilang -- dioxus`
Expected: All existing Dioxus codegen tests pass (event names now resolve correctly).

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/dioxus/expr.rs
git commit -m "fix(dioxus): strip 'on' prefix from event names in to_dioxus_event

IR event names carry 'on' prefix (e.g. 'onPress'). The match table
expected unprefixed names, causing all events to fall through to the
default case. Add strip_prefix('on') normalization matching the React
backend's to_react_event pattern.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: H1 — Fix Vue event name prefix bug

**Files:**
- Modify: `omnilang/src/codegen/vue/expr.rs:123-125`

**Background:** Vue's `to_vue_event` currently returns the event name unchanged. Since IR names carry `"on"` prefix, this generates `@onPress="handler"` instead of `@click="handler"`. Vue directives use the bare DOM event name.

- [ ] **Step 1: Strip `"on"` prefix and map known event names**

In `omnilang/src/codegen/vue/expr.rs`, replace lines 121-125:

```rust
/// Map OmniView event names to Vue event directive suffixes.
/// Vue uses @click, @change, etc. — bare DOM event names.
pub(super) fn to_vue_event(event_name: &str) -> String {
    // Strip any existing "on" prefix to normalize (IR may carry "onPress" etc.)
    let stem = event_name.strip_prefix("on").unwrap_or(event_name);
    // Lowercase for case-insensitive matching
    let stem_lower = stem.to_lowercase();
    match stem_lower.as_str() {
        "click" | "press" | "tap" => "click".into(),
        "change" => "change".into(),
        "input" => "input".into(),
        "submit" => "submit".into(),
        "focus" => "focus".into(),
        "blur" => "blur".into(),
        "keydown" => "keydown".into(),
        "keyup" => "keyup".into(),
        "mouseenter" => "mouseenter".into(),
        "mouseleave" => "mouseleave".into(),
        "mousedown" => "mousedown".into(),
        "mouseup" => "mouseup".into(),
        "mousemove" => "mousemove".into(),
        "dblclick" => "dblclick".into(),
        "scroll" => "scroll".into(),
        "wheel" => "wheel".into(),
        "touchstart" => "touchstart".into(),
        "touchend" => "touchend".into(),
        "touchmove" => "touchmove".into(),
        "contextmenu" => "contextmenu".into(),
        "drag" => "drag".into(),
        "dragstart" => "dragstart".into(),
        "dragend" => "dragend".into(),
        "dragover" => "dragover".into(),
        "dragleave" => "dragleave".into(),
        "drop" => "drop".into(),
        // Custom events: use the stem directly (Vue will emit as component event name)
        _ => stem.to_lowercase(),
    }
}
```

**Note:** Return type changes from `&str` to `String` since some branches produce owned strings (`stem.to_lowercase()`). Update any callers that relied on the `&str` lifetime. Check `to_vue_event` call sites in `omnilang/src/codegen/vue/`:

- `omnilang/src/codegen/vue/template.rs`
- `omnilang/src/codegen/vue/mod.rs`

All callers likely use `to_vue_event(...)` inline and won't be affected by the return type change (since `String` implements `Display` and can be passed to string formatting macros).

- [ ] **Step 2: Run tests and verify**

Run: `cargo test -p omnilang -- vue`
Expected: All existing Vue codegen tests pass.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/vue/expr.rs
git commit -m "fix(vue): strip 'on' prefix and normalize event names in to_vue_event

Previously returned event_name unchanged, producing @onPress instead
of @click. Now strips 'on' prefix and maps known event names to bare
DOM names matching Vue's @event directive syntax.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: H1 — Fix Flutter event name prefix bug

**Files:**
- Modify: `omnilang/src/codegen/flutter/utils.rs:41-62`

**Background:** Flutter's `event_to_flutter_callback` expects unprefixed names but IR carries `"on"` prefix. `onPress` doesn't match `"click"` → falls through to default → returns `"onPress"` as-is, but Flutter buttons use `onPressed`.

- [ ] **Step 1: Add `strip_prefix("on")` normalization in `event_to_flutter_callback`**

In `omnilang/src/codegen/flutter/utils.rs`, replace lines 40-62:

```rust
/// Map OmniView event names to Flutter callback parameter names.
pub(crate) fn event_to_flutter_callback(event_name: &str) -> &str {
    // Strip any existing "on" prefix to normalize (IR may carry "onPress" etc.)
    let stem = event_name.strip_prefix("on").unwrap_or(event_name).to_lowercase();
    match stem.as_str() {
        "click" | "tap" | "press" => "onPressed",
        "change" | "input" => "onChanged",
        "submit" => "onSubmitted",
        "focus" => "onFocus",
        "blur" => "onBlur",
        "keydown" => "onKeyDown",
        "keyup" => "onKeyUp",
        "longpress" => "onLongPress",
        "doubleclick" | "dblclick" => "onDoubleTap",
        "hover" | "mouseenter" => "onHover",
        "scroll" => "onScroll",
        "drag" | "dragstart" => "onDragStart",
        "dragend" => "onDragEnd",
        "dragover" => "onDrag",
        "drop" => "onDrop",
        "toggle" => "onChanged",
        "select" => "onSelected",
        _ => event_name,
    }
}
```

**Note:** We add `"Press"` to the `"click" | "tap"` match arm so `onPress` (after stripping) maps to `onPressed`.

- [ ] **Step 2: Run tests and verify**

Run: `cargo test -p omnilang -- flutter`
Expected: All existing Flutter codegen tests pass.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/flutter/utils.rs
git commit -m "fix(flutter): strip 'on' prefix in event_to_flutter_callback

IR event names carry 'on' prefix, causing all events to fall through
to the default case. Strip prefix before matching so 'onPress' maps
to 'onPressed', 'onChange' to 'onChanged', etc.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: H2 — Add `Stmt::EventEmit` lowering to `StatementIR::EventEmitStmtIR`

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/stmt.rs:168-172`

**Background:** `Stmt::EventEmit { name, args, span }` is parsed from `emit onXxx(args)` syntax and resolved in the semantic pass, but the lowering pass never creates `StatementIR::EventEmitStmtIR`. The `EventEmit` variant falls through to the wildcard `_` case at line 168, producing `ExprStmtIR { expr: LiteralExpr { value: Null } }` — effectively `null;`. The `StatementIR::EventEmitStmtIR` IR node exists in `types.rs` but is never populated by lowering.

Semantic resolve at `semantic/resolve.rs:295` handles `Stmt::EventEmit` (resolves references in args). Lowering just needs to produce the correct IR node.

- [ ] **Step 1: Add `Stmt::EventEmit` lowering case**

In `crates/omnilang-core/src/ir/lower/stmt.rs`, insert a new case BEFORE the wildcard `_` at line 168:

```rust
        Stmt::EventEmit { name, args, .. } => {
            StatementIR::EventEmitStmtIR {
                event_name: name.clone(),
                args: args
                    .iter()
                    .map(|a| lower_expr(a, variant_map, event_fields.as_deref_mut(), semantics))
                    .collect(),
            }
        }
        _ => StatementIR::ExprStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            },
        },
```

- [ ] **Step 2: Run tests to verify no regressions**

Run: `cargo test -p omnilang-core`
Expected: All existing tests pass. No snapshot changes since `EventEmitStmtIR` in existing test IR trees would be a new node — but no existing tests create `Stmt::EventEmit` in AST inputs, so no old tests change behavior.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/stmt.rs
git commit -m "fix(ir-lower): add Stmt::EventEmit lowering to EventEmitStmtIR

emit onXxx(args) was lowered to null; via the wildcard _ catch-all.
Now produces proper EventEmitStmtIR node with lowered args, which
codegen backends can handle.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: H2 — Add OmniView `format_event_emit_stmt` override

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/stmt.rs:367-378`

**Background:** The shared `format_event_emit_stmt` default emits only a comment: `// emit event: onXxx(args)`. Only React overrides it at `omnilang/src/codegen/react/expr.rs:187-194` with real code. The OmniView backend's `emit_statement` in `omnilang/src/codegen/omniview/stmt.rs` also has no `EventEmitStmtIR` handler and falls to `/* unhandled stmt */`.

After Task 4, `Stmt::EventEmit` will properly lower to `StatementIR::EventEmitStmtIR`, so both backends need to handle it.

For OmniView, the store.rs already generates event dispatch functions (line 234-251) that use `CustomEvent.dispatchEvent`. The emit statement should call those generated functions. Looking at store.rs:228-251, the generated dispatch functions are named `{component}_{eventName}` (e.g., `EmitChild_onEmitClick`), and the name_renames map maps `eventName` → `{component}_{eventName}` for function body references.

For the shared default, change the comment-only implementation to emit the event name as a bare call (which the rename mapping will resolve):

- [ ] **Step 1: Change shared `format_event_emit_stmt` default to emit a call**

In `crates/omnilang-core/src/codegen/shared/stmt.rs`, replace lines 367-378:

```rust
    /// Format an event emit statement.
    /// Default: emits as a function call (e.g. `onCustomEvent(value);`).
    /// Backends that need different syntax (Flutter, Dioxus) MUST override.
    fn format_event_emit_stmt(&self, w: &mut CodeWriter, event_name: &str, args: &[ExprIR]) {
        let arg_str = if args.is_empty() {
            String::new()
        } else {
            let formatted: Vec<String> = args
                .iter()
                .map(|a| self.expr_fmt().format_expr(a))
                .collect();
            format!("({})", formatted.join(", "))
        };
        w.emit_fmt(format_args!("{event_name}{arg_str};"));
    }
```

- [ ] **Step 2: Add `EventEmitStmtIR` handler to OmniView `emit_statement`**

In `omnilang/src/codegen/omniview/stmt.rs`, add a new match arm before the wildcard `_` at line 123:

```rust
            StatementIR::EventEmitStmtIR { event_name, args } => {
                let arg_str: Vec<String> = args
                    .iter()
                    .map(|a| self.expr_to_js(a))
                    .collect();
                let resolved_name = self
                    .name_renames
                    .get(event_name.as_str())
                    .cloned()
                    .unwrap_or_else(|| event_name.clone());
                self.js.push_str(&format!(
                    "  {}({});\n",
                    resolved_name,
                    arg_str.join(", ")
                ));
            }
```

- [ ] **Step 3: Run tests and verify**

Run: `cargo test -p omnilang -- omniview`
Run: `cargo test -p omnilang-core`
Expected: All existing tests pass. If there are existing test fixtures with `emit` statements, their snapshots will update to show real code instead of comments/nulls.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/stmt.rs omnilang/src/codegen/omniview/stmt.rs
git commit -m "fix(emit): change shared format_event_emit_stmt default from comment to call, add OmniView handler

Previously the shared default emitted only // emit event: ... and the
OmniView emit_statement had no EventEmitStmtIR handler. Combined with
Task 4's lowering fix, emit keyword now generates real code.

OmniView resolves event name through name_renames map (set up by
store.rs for component event dispatch functions).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: M2 — Fix Flutter component event `onOnPress` doubling

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs:84`

**Background:** For component composition in Flutter, line 84 does `format!("on{}: {}", capitalize_first(event_name), cb)`. Since `event_name` from IR already has `"on"` prefix (e.g., `"onPress"`), `capitalize_first("onPress")` returns `"OnPress"`, and the format string prepends `"on"` again → `"onOnPress"`.

- [ ] **Step 1: Strip `"on"` prefix before capitalizing**

In `omnilang/src/codegen/flutter/view.rs`, replace line 84:

```rust
            let stem = event_name.strip_prefix("on").unwrap_or(event_name);
            child_args.push(format!("on{}: {}", capitalize_first(stem), cb));
```

- [ ] **Step 2: Run tests and verify**

Run: `cargo test -p omnilang -- flutter`
Expected: Existing Flutter tests pass without snapshot change (if there's no existing component-to-component event test) or with corrected output.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs
git commit -m "fix(flutter): strip 'on' prefix before capitalize_first in component events

IR event_name already carries 'on' prefix. capitalize_first('onPress')
→ 'OnPress', then format!('on{}') → 'onOnPress'. Strip prefix first
so 'onPress' → 'Press' → 'OnPress' → 'onPressed'.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: M1 — Remove unconditional `preventDefault` on onSubmit

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:274`

**Background:** Line 274 unconditionally sets `needs_pd = true` for `onSubmit`, OR'd with the scan result. This means forms can never submit natively. The user should be able to opt in/out by calling `e.preventDefault()` in their handler.

- [ ] **Step 1: Remove the unconditional default for onSubmit**

In `crates/omnilang-core/src/ir/lower/view.rs`, replace line 274:

**Before:**
```rust
                let mut needs_pd = name == "onSubmit"; // backward compat
```

**After:**
```rust
                let mut needs_pd = false;
```

- [ ] **Step 2: Run tests and verify**

Run: `cargo test -p omnilang-core`
Run: `cargo test -p omnilang -- omniview`
Expected: Tests that relied on the implicit `preventDefault` may need snapshot updates. If a test fixture uses `@onSubmit` without calling `e.preventDefault()`, the generated code will no longer have `e.preventDefault()`. Update snapshots via `cargo test -- --accept` or equivalent.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/view.rs
git commit -m "fix(events): remove unconditional preventDefault for onSubmit

The unconditional needs_pd = true for onSubmit prevented forms from
submitting natively. Users who need preventDefault can call it in
their handler (detected by scan_event_methods). This makes the
behavior opt-in via e.preventDefault() rather than forced.

BREAKING: Forms with @onSubmit that relied on implicit preventDefault
will now submit natively unless e.preventDefault() is called.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: M3 — Collect event handler parameter name and use it in field detection

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:277-289` (lowering — collect param name)
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:895-901` (walk_expr_event_fields — use param name)

**Background:** `walk_expr_event_fields` hardcodes `name == "e"` check. If a user writes `(event) => { event.x }` or `fn handle(event) { event.x }`, the used_fields analysis returns empty set and `{}` is passed as the event object. The OmniView convention is `e`, but the compiler should handle any parameter name.

- [ ] **Step 1: Make `walk_expr_event_fields` accept a parameter name**

In `crates/omnilang-core/src/ir/lower/view.rs`, replace the function at line 895:

```rust
/// Walk an ExprIR tree for `FieldAccessExpr { object: Box(IdentifierExpr(param_name)), field }`.
fn walk_expr_event_fields(expr: &ExprIR, fields: &mut Vec<String>, param_name: &str) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                if name == param_name && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            walk_expr_event_fields(object, fields, param_name);
        }
        ExprIR::OptionalChainExpr { object, .. } => {
            walk_expr_event_fields(object, fields, param_name);
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_event_fields(object, fields, param_name);
            for arg in args {
                walk_expr_event_fields(arg, fields, param_name);
            }
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_event_fields(arg, fields, param_name);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_event_fields(left, fields, param_name);
            walk_expr_event_fields(right, fields, param_name);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_event_fields(operand, fields, param_name),
        ExprIR::TernaryIR {
            condition,
            then_branch,
            else_branch,
        } => {
            walk_expr_event_fields(condition, fields, param_name);
            walk_expr_event_fields(then_branch, fields, param_name);
            walk_expr_event_fields(else_branch, fields, param_name);
        }
        ExprIR::ArrayExprIR { items, .. } => {
            for item in items {
                walk_expr_event_fields(item, fields, param_name);
            }
        }
        ExprIR::ObjectExprIR { fields: obj_fields, .. } => {
            for (_key, value) in obj_fields {
                walk_expr_event_fields(value, fields, param_name);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Extract param name and pass through all call sites**

First, update the `EventHandler::InlineFn` case in `lower/view.rs` at ~line 277. Change:

```rust
                EventHandler::InlineFn { params, body, .. } => {
                    if event_data.is_some() {
                        let (needs_pd_from_scan, needs_sp_from_scan) = scan_event_methods(body);
                        needs_pd = needs_pd || needs_pd_from_scan;
                        needs_sp = needs_sp_from_scan;

                        // Extract event parameter name (default "e" if no params)
                        let event_param_name = params
                            .first()
                            .map(|p| p.name.as_str())
                            .unwrap_or("e");
                        let mut event_fields = FxHashSet::default();
                        let block_ir =
                            lower_block(body, variant_map, Some(&mut event_fields), semantics);
```

Then find all call sites of `walk_expr_event_fields` and update them to pass `param_name`. The existing callers are inside `collect_ir_event_fields` at lower/view.rs (the function used to scan FnRef bodies). Let's update `collect_ir_event_fields` to also accept `param_name`:

- [ ] **Step 3: Update `collect_ir_event_fields` to accept param_name**

Search for `collect_ir_event_fields` in `ir/lower/view.rs` and add a `param_name: &str` parameter. All its internal calls to `walk_expr_event_fields(expr, fields)` become `walk_expr_event_fields(expr, fields, param_name)`.

For FnRef handlers, extract the param name from the function definition:
```rust
// In the FnRef resolution path, look up the function and get its first param name
let fn_param_name = fns.iter()
    .find(|f| f.name == fn_name)
    .and_then(|f| f.params.first())
    .map(|p| p.name.as_str())
    .unwrap_or("e");
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p omnilang-core`
Run: `cargo test -p omnilang`
Expected: All tests pass. Snapshot tests for fixtures using non-`e` parameter names may update to show correct field extraction.

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/view.rs
git commit -m "fix(events): use actual handler param name in walk_expr_event_fields

Previously hardcoded check for param name 'e', causing handlers with
other parameter names (e.g. (event) => event.x) to receive {} instead
of actual event data. Now extracts the parameter name from the handler
declaration and passes it through to the field walker.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: M4 — Align custom event dispatch model with extraction

**Files:**
- Modify: `crates/omnilang-core/src/codegen/event.rs:165-170`
- Verify (no change needed): `omnilang/src/codegen/omniview/store.rs:234-251`
- Verify (no change needed): `omnilang/src/codegen/react/utils.rs` (React `emit_event_object_react` handling of `None`)

**Background:** Custom events (no `EventDataKind`) dispatch via `CustomEvent.dispatchEvent` in OmniView's store.rs (line 248), which sets `detail: args[0]`. The shared `emit_event_object` correctly returns `e.detail` for `None` kind. React's `emit_event_object_react` returns `"{}"` for `None`.

The actual mismatch is in **how custom events are called from within the component**: the OmniView store.rs (line 234) generates dispatch functions that forward to `CustomEvent.dispatchEvent`. But when emitted from within the component via `emit onXxx("hello")` (after Task 4+5 fix), the OmniView codegen will try to call the resolved name directly. The store.rs already generates these as regular functions that accept arguments, so `emit onXxx("hello")` → `ComponentName_onXxx("hello")` → `dispatchEvent(new CustomEvent(...))`. This is consistent — the extraction expects `e.detail` and the dispatch sets `detail`.

**Decision:** The model is already consistent. Custom events are always `CustomEvent` with data in `.detail`, both in dispatch (store.rs) and extraction (event.rs). No code change needed. We only need to verify that React's handling is also consistent.

- [ ] **Step 1: Verify React handling of custom events**

Read `omnilang/src/codegen/react/utils.rs` and check `emit_event_object_react` for `None` kind. Confirm it returns `"{}"` and trace the React event flow to ensure consistency.

- [ ] **Step 2: Commit (mark as verified/no change)**

```bash
git commit --allow-empty -m "docs(events): verify custom event dispatch/extraction consistency

Custom events (no EventDataKind in known DOM list) use CustomEvent
model consistently:
- Dispatch: store.rs generates dispatchEvent(new CustomEvent(..., {detail: args[0]}))
- Extraction: emit_event_object returns e.detail for None kind
- React: emit_event_object_react returns {} for None (consistent
  with React's component prop passing, where custom events are
  plain function calls, not CustomEvent objects)

No code changes needed.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 10: Full test suite verification

**Files:**
- Run full test suite

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace
```

Expected: All 800+ tests pass. Address any failures from Tasks 1-9.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: Zero clippy warnings.

- [ ] **Step 3: Run fmt**

```bash
cargo fmt --all -- --check
```

Expected: All files properly formatted.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: run full test suite, clippy, and fmt after event system fixes

All 9 fixes integrated. Test suite passing, clippy clean, fmt clean.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
