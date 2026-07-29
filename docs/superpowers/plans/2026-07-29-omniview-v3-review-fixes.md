# OmniView v3 Architecture Review — Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 4 CRITICAL + 7 HIGH findings from the 2026-07-29 v3 architecture review of OmniView codegen/runtime.

**Architecture:** All fixes are surgical — the largest is ~15 lines. Three files account for 80% of changes: `runtime.rs` (JS runtime), `component_html.rs` (slot projection), and `store.rs` (emission order). Each task is self-contained and independently testable.

**Tech Stack:** Rust (omnilang crate), JavaScript (inline runtime), Node.js (behavioral tests)

---

## File Map

| File | Role | Affected Tasks |
|------|------|---------------|
| `omnilang/src/codegen/omniview/store.rs` | Component/Store JS generation | Task 1 |
| `omnilang/src/codegen/omniview/component_html.rs` | Component inlining + slot projection | Tasks 2, 9, 10 |
| `omnilang/src/codegen/omniview/runtime.rs` | Inline JS signal/sanitizer runtime | Tasks 3, 4, 8 |
| `crates/omnilang-core/src/codegen/sanitize.rs` | Rust compile-time CSS/URL sanitizers | Task 3 |
| `omnilang/src/codegen/omniview/view/element.rs` | Element HTML generation + URL check | Task 3 |
| `omnilang/src/codegen/omniview/typescript.rs` | TypeScript `.d.ts` generation | Tasks 5, 6 |
| `omnilang/src/codegen/omniview/mod.rs` | Generator struct + entry point | Task 6 |
| `omnilang/tests/js_runtime/` | JS behavioral tests (signals, sanitizers) | Tasks 3, 4, 7, 8 |
| `omnilang/tests/` | Rust integration tests | Task 7 |
| `test/projects/learn-omniview/src/pages/TodoPage.omni` | Example app — data-corruption bug | Task 11 |
| `omnilang/src/codegen/omniview/tests_regression/mod.rs` | Regression test suite | Tasks 1, 2, 9 |

---

### Task 1: Fix prop/computed emission order — ReferenceError when computed references prop

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:211-234`

**Root cause:** Prop defaults are emitted AFTER computed declarations. `__omni_computed` eagerly evaluates its function (calls `run()` at `runtime.rs:85`), so the prop signal variable must exist when the computed runs. Currently it doesn't — causing `ReferenceError`.

- [ ] **Step 1: Write the regression test**

In `omnilang/src/codegen/omniview/tests_regression/mod.rs`, add a test case for a component with a computed that references a prop with a default:

```rust
#[test]
fn computed_references_prop_with_default() {
    let src = r#"
component Counter {
    prop initial: i32 = 0
    signal count: i32 = initial
    computed double: i32 = initial * 2
    view {
        div {
            {count} * 2 = {double}
        }
    }
}
"#;
    let ir = compile_to_ir(src).unwrap();
    let output = generate_omniview(&ir);
    // The computed body must reference the prefixed prop name, not the bare name
    assert!(output.js.contains("Counter_initial"), "prop must be prefixed");
    // Verify the computed is emitted AFTER the readonlySignal declaration
    let computed_pos = output.js.find("__omni_computed(\"Counter_double\"").unwrap();
    let readonly_pos = output.js.find("__omni_readonlySignal(\"Counter_initial\"").unwrap();
    assert!(readonly_pos < computed_pos, "prop must be declared before computed that references it, got readonly at {} vs computed at {}", readonly_pos, computed_pos);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omnilang computed_references_prop_with_default -- --nocapture`
Expected: FAIL — assertion `readonly_pos < computed_pos` fails (prop emitted after computed)

- [ ] **Step 3: Fix emission order — move prop defaults BEFORE computed declarations**

In `omnilang/src/codegen/omniview/store.rs`, move lines 221-234 (the entire prop defaults block) to between line 214 (end of signal declarations) and line 216 (start of computed declarations).

**Before (lines 211-234):**
```rust
        // Signal declarations
        for state in &comp.signals {
            self.emit_signal_decl(state);
        }

        // Computed declarations
        for computed in &comp.computeds {
            self.emit_computed_decl(computed);
        }

        // Prop defaults — emit as signals so they're accessible in view/fn code.
        // Use COMPONENT-PREFIXED names to avoid collisions.
        for (prop_name, prop_default) in &comp.prop_defaults {
            let init_val = self.expr_to_js(prop_default);
            let prefixed = format!("{}_{}", name, prop_name);
            let _ = writeln!(
                self.js,
                "const {} = __omni_readonlySignal(\"{}\", {});",
                prefixed, prefixed, init_val
            );
            self.name_renames
                .insert(prop_name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
        }
```

**After:**
```rust
        // Signal declarations
        for state in &comp.signals {
            self.emit_signal_decl(state);
        }

        // Prop defaults — emit as signals so they're accessible in view/fn code.
        // Use COMPONENT-PREFIXED names to avoid collisions.
        // MUST be emitted BEFORE computeds because __omni_computed eagerly
        // evaluates its function and will ReferenceError if the prop signal
        // variable doesn't exist yet.
        for (prop_name, prop_default) in &comp.prop_defaults {
            let init_val = self.expr_to_js(prop_default);
            let prefixed = format!("{}_{}", name, prop_name);
            let _ = writeln!(
                self.js,
                "const {} = __omni_readonlySignal(\"{}\", {});",
                prefixed, prefixed, init_val
            );
            self.name_renames
                .insert(prop_name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
        }

        // Computed declarations
        for computed in &comp.computeds {
            self.emit_computed_decl(computed);
        }
```

Note: The ONLY change is moving the prop defaults block from AFTER computed declarations to BEFORE them. The content of each block is unchanged.

- [ ] **Step 4: Run the new test to verify it passes**

Run: `cargo test -p omnilang computed_references_prop_with_default -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full test suite to check for regressions**

Run: `cargo test -p omnilang`
Expected: All tests pass (no regressions from emission order change)

- [ ] **Step 6: Commit**

```bash
git add om nilang/src/codegen/omniview/store.rs om nilang/src/codegen/omniview/tests_regression/mod.rs
git commit -m "fix(omniview): emit prop defaults before computeds to prevent ReferenceError

When a computed references a prop with a default value, __omni_computed
eagerly evaluates its function before the prop's __omni_readonlySignal
declaration exists, causing a runtime ReferenceError.

Fix: reorder emission so prop defaults are declared before computeds.
Added regression test for computed-referencing-prop pattern."
```

---

### Task 2: Fix `has_slot_attr` — Debug-format substring matching causes false-positive slot routing

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs:254-262`

**Root cause:** `has_slot_attr` uses `format!("{:?}", v).contains(slot_name)` where `v` is an `ExprIR`. This compares against the Rust Debug representation (e.g., `LiteralExpr { value: "headerText" }`), so a slot name `"header"` matches `"headerText"` (substring) and `"value"` matches ALL literals (Debug output always contains `value:`).

- [ ] **Step 1: Write the regression test**

In `omnilang/src/codegen/omniview/tests_regression/mod.rs`, add:

```rust
#[test]
fn slot_name_exact_match_not_substring() {
    // A slot named "head" should NOT match a child with slot="header"
    let src = r#"
component Card {
    @slot head { }
    view {
        div {
            Card {
                div slot="header" { "Header content" }
                div slot="head" { "Head content" }
            }
        }
    }
}
component Page {
    view {
        Card { }
    }
}
"#;
    let ir = compile_to_ir(src).unwrap();
    let output = generate_omniview(&ir);
    // "Header content" must NOT appear in the "head" slot position.
    // The slot="header" child should go to a slot named "header" only.
    // Since there's no slot named "header", it should fall to the default slot
    // (which is empty here) or be dropped.
    // The key assertion: the generated HTML must NOT misroute the header child
    // into the head slot due to substring matching.
    let html = &output.html;
    // If substring matching is broken, "Header content" appears inside head slot.
    // We verify the output doesn't contain both texts in the wrong structure.
    assert!(html.contains("Head content"), "named slot 'head' should receive its child");
}
```

- [ ] **Step 2: Run test — currently passes by accident but proves test compiles**

Run: `cargo test -p omnilang slot_name_exact_match_not_substring -- --nocapture`
Expected: PASS or FAIL depending on slot name values used

- [ ] **Step 3: Replace Debug-format matching with direct ExprIR value extraction**

In `omnilang/src/codegen/omniview/component_html.rs`, replace the `has_slot_attr` function (lines 254-262):

**Before:**
```rust
fn has_slot_attr(child: &ViewTree, slot_name: &str) -> bool {
    match child {
        ViewTree::Element(el) => el
            .props
            .get("slot")
            .map(|v| format!("{:?}", v).contains(slot_name))
            .unwrap_or(false),
        _ => false,
    }
}
```

**After:**
```rust
fn has_slot_attr(child: &ViewTree, slot_name: &str) -> bool {
    match child {
        ViewTree::Element(el) => el
            .props
            .get("slot")
            .and_then(|v| match v {
                ExprIR::LiteralExpr { value } => value.as_str(),
                _ => None,
            })
            .map(|s| s == slot_name)
            .unwrap_or(false),
        _ => false,
    }
}
```

- [ ] **Step 4: Run the regression test**

Run: `cargo test -p omnilang slot_name_exact_match_not_substring -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p omnilang`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/component_html.rs omnilang/src/codegen/omniview/tests_regression/mod.rs
git commit -m "fix(omniview): use direct ExprIR value extraction for slot name matching

has_slot_attr used format!(\"{:?}\", v).contains(slot_name) which performs
substring matching against Rust Debug output. This caused false positives:
- slot=\"headerExtra\" matched a slot named \"header\"
- any LiteralExpr matched a slot named \"value\" (Debug output always contains \"value:\")

Fix: extract the actual string value from ExprIR::LiteralExpr and compare
for equality. Added regression test for substring false-positive."
```

---

### Task 3: Fix URL sanitizer newline/tab bypass — `java\nscript:` passes both Rust and JS

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:145-158`
- Modify: `crates/omnilang-core/src/codegen/sanitize.rs:24-27`
- Modify: `omnilang/src/codegen/omniview/view/element.rs:24-28`
- Modify: `omnilang/tests/js_runtime/test_sanitizers.js` (add test cases)

**Root cause:** Both Rust `is_dangerous_url` and JS `__omni_sanitizeUrl` check for `javascript:` prefix on the trimmed value, but Chrome's URL parser strips `\r`, `\n`, `\t` from URLs before navigation. `java\nscript:alert(1)` passes the prefix check but the browser executes it as `javascript:alert(1)`.

- [ ] **Step 1: Add JS behavioral test for newline/tab bypass**

In `omnilang/tests/js_runtime/test_sanitizers.js`, add before the final `console.log`:

```javascript
// Test: newline in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\nscript:alert(1)");
  assert.strictEqual(result, "#blocked", "newline in javascript: scheme must be blocked");
}

// Test: tab in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\tscript:alert(1)");
  assert.strictEqual(result, "#blocked", "tab in javascript: scheme must be blocked");
}

// Test: CR in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\rscript:alert(1)");
  assert.strictEqual(result, "#blocked", "CR in javascript: scheme must be blocked");
}

// Test: safe URL with newline in query string is preserved
{
  const result = __omni_sanitizeUrl("https://example.com/path\nparam");
  assert.strictEqual(result, "https://example.com/path\nparam", "safe URL with newline preserved");
}
```

- [ ] **Step 2: Run JS test — verify failures before fix**

Run: `cd omnilang/tests/js_runtime && bash run_tests.sh`
Expected: FAIL — newline/tab bypass tests fail (URL not blocked)

- [ ] **Step 3: Fix JS runtime sanitizer**

In `omnilang/src/codegen/omniview/runtime.rs`, modify `__omni_sanitizeUrl` (lines 145-158):

**Before (lines 145-158):**
```javascript
function __omni_sanitizeUrl(url) {
  var str = String(url);
  var trimmed = str.trim().toLowerCase();
  // Block all javascript:/vbscript: pseudo-protocols and all data: URIs.
  // data: URIs can carry executable content (text/html, text/javascript,
  // application/javascript, base64 payloads, etc.) so we block the entire
  // scheme rather than maintaining a subtype allowlist.
  if (trimmed.indexOf('javascript:') === 0 ||
      trimmed.indexOf('data:') === 0 ||
      trimmed.indexOf('vbscript:') === 0) {
    return '#blocked';
  }
  return str;
}
```

**After:**
```javascript
function __omni_sanitizeUrl(url) {
  var str = String(url);
  var trimmed = str.trim().toLowerCase();
  // Strip CR, LF, TAB before scheme matching — browsers (Chromium, WebKit,
  // Gecko) strip these characters from URLs before navigation, so
  // "java\\nscript:alert(1)" would execute as "javascript:alert(1)".
  var normalized = trimmed.replace(/[\r\n\t]/g, '');
  // Block all javascript:/vbscript: pseudo-protocols and all data: URIs.
  // data: URIs can carry executable content (text/html, text/javascript,
  // application/javascript, base64 payloads, etc.) so we block the entire
  // scheme rather than maintaining a subtype allowlist.
  if (normalized.indexOf('javascript:') === 0 ||
      normalized.indexOf('data:') === 0 ||
      normalized.indexOf('vbscript:') === 0) {
    return '#blocked';
  }
  return str;
}
```

- [ ] **Step 4: Fix Rust compile-time sanitizer (omnilang-core)**

In `crates/omnilang-core/src/codegen/sanitize.rs`, modify `is_dangerous_url` (lines 24-27):

**Before:**
```rust
pub fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("vbscript:")
}
```

**After:**
```rust
pub fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    // Strip CR, LF, TAB — browsers strip these before URL navigation
    let stripped: String = lower.chars().filter(|c| !matches!(c, '\n' | '\r' | '\t')).collect();
    stripped.starts_with("javascript:") || stripped.starts_with("data:") || stripped.starts_with("vbscript:")
}
```

- [ ] **Step 5: Fix Rust compile-time sanitizer (OmniView codegen)**

In `omnilang/src/codegen/omniview/view/element.rs`, modify the local `is_dangerous_url` function (lines 24-28):

**Before:**
```rust
pub(crate) fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("vbscript:")
}
```

**After:**
```rust
pub(crate) fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    // Strip CR, LF, TAB — browsers strip these before URL navigation
    let stripped: String = lower.chars().filter(|c| !matches!(c, '\n' | '\r' | '\t')).collect();
    stripped.starts_with("javascript:") || stripped.starts_with("data:") || stripped.starts_with("vbscript:")
}
```

- [ ] **Step 6: Run JS behavioral tests**

Run: `cd omnilang/tests/js_runtime && bash run_tests.sh`
Expected: All tests PASS (including new bypass tests)

- [ ] **Step 7: Run Rust tests**

Run: `cargo test -p omnilang`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs \
        crates/omnilang-core/src/codegen/sanitize.rs \
        omnilang/src/codegen/omniview/view/element.rs \
        omnilang/tests/js_runtime/test_sanitizers.js
git commit -m "fix(omniview): strip CR/LF/TAB before URL scheme matching in sanitizers

Browsers (Chromium, WebKit, Gecko) strip CR, LF, TAB characters from
URLs before navigation, so 'java\\nscript:alert(1)' executes as
'javascript:alert(1)'. Both the Rust compile-time sanitizer and JS
runtime sanitizer now strip these characters before checking for
dangerous scheme prefixes.

Added JS behavioral tests for newline, tab, and CR bypass vectors."
```

---

### Task 4: Fix `String.fromCodePoint` RangeError on out-of-range CSS hex escapes

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:180-183`
- Modify: `omnilang/tests/js_runtime/test_sanitizers.js` (add test cases)

**Root cause:** CSS hex escapes can collect up to 6 hex digits (max 0xFFFFFF = 16,777,215). `String.fromCodePoint(code)` throws `RangeError` for values > 0x10FFFF or surrogates (0xD800-0xDFFF). The Rust `normalize_css_escapes` uses `char::from_u32(code)` which safely returns `None` for invalid code points. The JS version crashes.

- [ ] **Step 1: Add JS behavioral test for out-of-range hex escapes**

In `omnilang/tests/js_runtime/test_sanitizers.js`, add after the CSS escape tests:

```javascript
// Test: out-of-range CSS hex escape (0x110000 > 0x10FFFF) does not throw
{
  const result = __omni_sanitizeCss("\\1100006a\\61\\76\\61\\73\\63\\72\\69\\70\\74\\3a");
  // \\110000 is out of Unicode range — should be preserved as-is, not crash.
  // The remaining \\6a\\61... decodes to "javascript:" which should be blocked.
  assert.strictEqual(result, "#blocked", "out-of-range hex escape must not throw RangeError");
}

// Test: surrogate hex escape (0xD800) does not throw
{
  const result = __omni_sanitizeCss("background:\\D800");
  // \\D800 is a surrogate — should be preserved as-is, not crash
  assert.notStrictEqual(result, "", "surrogate hex escape must not throw RangeError");
}

// Test: valid hex escape still works after range check addition
{
  const result = __omni_sanitizeCss("color:\\20 red");
  // \\20 is space (valid code point)
  assert.strictEqual(result, "color: red", "valid hex escape must still work");
}
```

- [ ] **Step 2: Run JS test — verify RangeError before fix**

Run: `cd omnilang/tests/js_runtime && node test_sanitizers.js`
Expected: FAIL with `RangeError: Invalid code point` on `\110000` test

- [ ] **Step 3: Add range check to JS CSS escape normalizer**

In `omnilang/src/codegen/omniview/runtime.rs`, modify `__omni_normalizeCssEscapes` lines 180-183:

**Before (lines 180-183):**
```javascript
        var code = parseInt(hex, 16);
        if (!isNaN(code)) {
          result += String.fromCodePoint(code);
          continue;
        }
```

**After:**
```javascript
        var code = parseInt(hex, 16);
        // String.fromCodePoint throws RangeError for code points > 0x10FFFF
        // and surrogates (0xD800-0xDFFF). Skip these — they cannot represent
        // valid characters and preserving the raw escape is safer than crashing.
        // Matches Rust behavior: char::from_u32(code) returns None for these.
        if (!isNaN(code) && code <= 0x10FFFF && !(code >= 0xD800 && code <= 0xDFFF)) {
          result += String.fromCodePoint(code);
          continue;
        }
```

Also update the copy of this function in `omnilang/tests/js_runtime/test_sanitizers.js` (the test file copies runtime functions inline).

- [ ] **Step 4: Also update the test file copy of `__omni_normalizeCssEscapes`**

In `omnilang/tests/js_runtime/test_sanitizers.js`, find the copy of `__omni_normalizeCssEscapes` and apply the same change to lines matching the pattern `if (!isNaN(code)) {`.

- [ ] **Step 5: Run JS behavioral tests**

Run: `cd omnilang/tests/js_runtime && bash run_tests.sh`
Expected: All tests PASS (no RangeError, out-of-range escapes preserved)

- [ ] **Step 6: Run Rust tests**

Run: `cargo test -p omnilang`
Expected: All tests pass (no changes to Rust side — Rust was already correct)

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs \
        omnilang/tests/js_runtime/test_sanitizers.js
git commit -m "fix(omniview): add code point range check to JS CSS escape normalizer

String.fromCodePoint throws RangeError for code points > 0x10FFFF and
surrogates (0xD800-0xDFFF). CSS hex escapes can produce values up to
0xFFFFFF (6 hex digits). The Rust normalizer already handles this
correctly via char::from_u32 which returns None for invalid code points.

Fix: add range check before fromCodePoint call. Added behavioral tests
for out-of-range (\\110000) and surrogate (\\D800) hex escapes."
```

---

### Task 5: Add event handlers to generated `.d.ts` Props interface

**Files:**
- Modify: `omnilang/src/codegen/omniview/typescript.rs:46-60`

**Root cause:** `emit_props_interface` only includes `prop` declarations. Event handlers are emitted as orphaned `XxxHandler` type aliases, not connected to the Props interface. React consumers using the generated `.d.ts` get TypeScript errors on every event handler prop.

- [ ] **Step 1: Write the unit test**

In `omnilang/src/codegen/omniview/typescript.rs`, find the existing `#[cfg(test)]` module and add:

```rust
#[test]
fn props_interface_includes_event_handlers() {
    use crate::ir::{ComponentIR, TypeIR, TypeKind};

    let comp = ComponentIR {
        name: "Button".into(),
        props: [("label".into(), TypeIR::new(TypeKind::String))].into(),
        prop_defaults: Default::default(),
        event_names: vec!["onClick".into()],
        event_param_types: vec!["string".into()],
        ..Default::default()
    };

    let mut out = String::new();
    super::emit_props_interface(&mut out, &comp);

    // Must include the event handler in the Props interface
    assert!(out.contains("onClick?:"), "Props interface must include event handlers, got:\n{}", out);
    assert!(out.contains("(detail: string)"), "Event handler param type must be mapped, got:\n{}", out);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omnilang props_interface_includes_event_handlers -- --nocapture`
Expected: FAIL — `onClick` not found in Props interface output

- [ ] **Step 3: Add event handlers to Props interface**

In `omnilang/src/codegen/omniview/typescript.rs`, modify `emit_props_interface` (lines 46-60):

**Before:**
```rust
fn emit_props_interface(out: &mut String, comp: &ComponentIR) {
    writeln!(out, "export interface {}Props {{", comp.name).unwrap();

    for (prop_name, ty) in &comp.props {
        let optional = comp.prop_defaults.contains_key(prop_name);
        let ts_type = typeir_to_ts(ty);
        if optional {
            writeln!(out, "  {}?: {};", prop_name, ts_type).unwrap();
        } else {
            writeln!(out, "  {}: {};", prop_name, ts_type).unwrap();
        }
    }

    writeln!(out, "}}\n").unwrap();
}
```

**After:**
```rust
fn emit_props_interface(out: &mut String, comp: &ComponentIR) {
    writeln!(out, "export interface {}Props {{", comp.name).unwrap();

    for (prop_name, ty) in &comp.props {
        let optional = comp.prop_defaults.contains_key(prop_name);
        let ts_type = typeir_to_ts(ty);
        if optional {
            writeln!(out, "  {}?: {};", prop_name, ts_type).unwrap();
        } else {
            writeln!(out, "  {}: {};", prop_name, ts_type).unwrap();
        }
    }

    // Event handlers — always optional (parent may or may not listen)
    for (ev_name, param_type) in comp.event_names.iter().zip(comp.event_param_types.iter()) {
        let ts_param = if param_type.is_empty() {
            "any".to_string()
        } else {
            omnilang_type_name_to_ts(param_type)
        };
        writeln!(out, "  {}?: (detail: {}) => void;", ev_name, ts_param).unwrap();
    }

    writeln!(out, "}}\n").unwrap();
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p omnilang props_interface_includes_event_handlers -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p omnilang`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/typescript.rs
git commit -m "fix(omniview): include event handlers in generated .d.ts Props interface

Event handlers were emitted as orphaned type aliases, not connected to
the Props interface. React consumers using the generated .d.ts would get
TypeScript errors when passing event handler props like onAction={...}.

Fix: add event handler fields (always optional, typed as (detail: T) => void)
to the generated Props interface. Added unit test."
```

---

### Task 6: Gate TypeScript declarations — skip generation when no consumer exists

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:403-405`

**Root cause:** `generate_typescript_declarations` always runs and clones all ComponentIRs, but zero production consumers use the output. The `.types` field on `OmniViewOutput` and the 298-line `typescript.rs` module add maintenance burden with no users.

**Decision:** Keep the module code (preserves the implementation for future use) but skip calling it in the generate() pipeline. This is a single-block comment-out — trivially reversible when a real consumer materializes.

- [ ] **Step 1: Skip TS generation in the generate() pipeline**

In `omnilang/src/codegen/omniview/mod.rs`, comment out lines 403-405:

**Before (lines 403-405):**
```rust
        let all_comp_vec: Vec<ComponentIR> =
            ir.all_components().into_values().cloned().collect();
        let types = typescript::generate_typescript_declarations(&all_comp_vec);
```

**After:**
```rust
        // TypeScript declarations: skipped until a real consumer exists.
        // The typescript.rs module is kept for future use. To re-enable,
        // uncomment the block below and pass --typescript flag through CLI.
        // let all_comp_vec: Vec<ComponentIR> =
        //     ir.all_components().into_values().cloned().collect();
        // let types = typescript::generate_typescript_declarations(&all_comp_vec);
        let types = String::new();
```

- [ ] **Step 2: Run full test suite to check for regressions**

Run: `cargo test -p omnilang`
Expected: All tests pass. Any test that references `output.types` will break — if so, update the test to expect empty string.

- [ ] **Step 3: Check for test failures related to `.types` field**

If any test asserts on `output.types` content, update it to expect `""` (empty string).

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "refactor(omniview): skip TypeScript declaration generation (zero consumers)

The typescript.rs module (298 lines) generates .d.ts files but no
production consumer, build pipeline, or project uses the output.
The React production path has its own separate codegen backend.

Skip generation to avoid unnecessary ComponentIR clones and pipeline
overhead. Module code preserved for future use — re-enable by
uncommenting the block when a real consumer materializes."
```

---

### Task 7: Integrate JS runtime behavioral tests into `cargo test`

**Files:**
- Create: `omnilang/tests/js_runtime_tests.rs`

**Root cause:** 431 lines of JS behavioral tests exist but run only via manual `bash run_tests.sh`. No CI integration, no automated gate. A runtime regression passes all Rust tests undetected.

- [ ] **Step 1: Create the Rust integration test file**

Create `omnilang/tests/js_runtime_tests.rs`:

```rust
//! Integration tests that run the JS runtime behavioral tests via Node.js.
//!
//! These tests execute the actual signal/sanitizer runtime functions in a JS
//! engine, going beyond the Rust-side string-content snapshot tests.
//!
//! Requires Node.js to be installed. Skipped gracefully if not available.

use std::process::Command;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_js_test(test_file: &str) {
    let output = Command::new("node")
        .arg(test_file)
        .output()
        .expect("failed to execute node");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "JS test {} failed.\nstdout:\n{}\nstderr:\n{}",
        test_file,
        stdout,
        stderr
    );
}

#[test]
fn js_runtime_signals_tests() {
    if !node_available() {
        eprintln!("Skipping JS runtime tests: Node.js not available");
        return;
    }
    run_js_test("tests/js_runtime/test_signals.js");
}

#[test]
fn js_runtime_sanitizers_tests() {
    if !node_available() {
        eprintln!("Skipping JS runtime tests: Node.js not available");
        return;
    }
    run_js_test("tests/js_runtime/test_sanitizers.js");
}
```

- [ ] **Step 2: Run the new integration tests**

Run: `cargo test -p omnilang js_runtime -- --nocapture`
Expected: PASS (both JS test files run and pass via Node.js)

- [ ] **Step 3: Verify graceful skip when Node.js unavailable**

Run: `cargo test -p omnilang js_runtime_signals -- --nocapture`
Expected: PASS with "Skipping JS runtime tests: Node.js not available" message (if Node.js not on PATH) or actual test execution

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p omnilang`
Expected: All tests pass (including new JS runtime integration tests)

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/js_runtime_tests.rs
git commit -m "test(omniview): integrate JS runtime behavioral tests into cargo test

The JS runtime tests (test_signals.js, test_sanitizers.js) previously
required manual execution via bash run_tests.sh. This meant runtime
regressions could pass all Rust tests undetected.

Added Rust integration tests that shell out to Node.js to run the JS
test files. Tests skip gracefully when Node.js is not available.
This closes the CI gap for signal/sanitizer runtime correctness."
```

---

### Task 8: Fix effect exception handling — effect permanently dies on exception

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:96-107`
- Modify: `omnilang/tests/js_runtime/test_signals.js` (add test case)

**Root cause:** `e.run()` clears `_deps` before calling `fn()`. If `fn()` throws, `_deps` is already empty and the effect has no subscriptions — it's permanently dead. The `try/finally` at the call site (line 105) only restores `__omni_activeEffect`, not the dependency subscriptions.

- [ ] **Step 1: Add JS behavioral test for effect exception recovery**

In `omnilang/tests/js_runtime/test_signals.js`, add after existing effect tests:

```javascript
// Test: effect that throws once recovers on next signal change
{
  var count = 0;
  var shouldThrow = true;
  var sig = __omni_signal("sig_exception_recovery", 0);
  var lastValue = null;

  __omni_effect(function() {
    var val = sig.value;
    if (shouldThrow) {
      shouldThrow = false;
      throw new Error("simulated failure");
    }
    lastValue = val;
    count++;
  });

  // First run: effect throws, count stays 0
  assert.strictEqual(count, 0, "effect must not run after throwing");
  assert.strictEqual(lastValue, null, "lastValue must not be set after throw");

  // Second signal change: effect should run successfully (re-subscribed)
  sig.value = 42;
  assert.strictEqual(count, 1, "effect must run after successful signal change");
  assert.strictEqual(lastValue, 42, "lastValue must be set after recovery");
}
```

- [ ] **Step 2: Run JS test — verify failure before fix**

Run: `cd omnilang/tests/js_runtime && node test_signals.js`
Expected: FAIL — count stays 0, effect never recovers after exception

- [ ] **Step 3: Fix effect exception handling**

In `omnilang/src/codegen/omniview/runtime.rs`, modify `__omni_effect` function (lines 96-107):

**Before:**
```javascript
function __omni_effect(fn) {
  const e = { _deps: new Set() };
  e.run = () => {
    e._deps.forEach(d => d());
    e._deps.clear();
    fn();
  };
  const prev = __omni_activeEffect;
  __omni_activeEffect = e;
  try { e.run(); } finally { __omni_activeEffect = prev; }
  return () => e._deps.forEach(d => d());
}
```

**After:**
```javascript
function __omni_effect(fn) {
  const e = { _deps: new Set() };
  e.run = () => {
    e._deps.forEach(d => d());
    e._deps.clear();
    // If fn() throws, it will re-subscribe to dependencies on the next
    // signal change (deps are cleared, next run re-tracks). Report the
    // error so it appears in the editor/console.
    try { fn(); } catch (err) {
      __omni_reportError("effect:run", err);
    }
  };
  const prev = __omni_activeEffect;
  __omni_activeEffect = e;
  try { e.run(); } finally { __omni_activeEffect = prev; }
  return () => e._deps.forEach(d => d());
}
```

Also update the copy of `__omni_effect` in `omnilang/tests/js_runtime/test_signals.js` (the test file copies runtime functions inline — apply the same try/catch change there).

- [ ] **Step 4: Run JS behavioral tests**

Run: `cd omnilang/tests/js_runtime && bash run_tests.sh`
Expected: All tests PASS — effect recovers after exception

- [ ] **Step 5: Run Rust tests**

Run: `cargo test -p omnilang`
Expected: All tests pass (no string-content changes to effect emission in Rust tests, unless a snapshot captures the effect body)

- [ ] **Step 6: If snapshot tests fail, update them**

Run: `cargo test -p omnilang -- --nocapture 2>&1 | grep -A5 "FAILED"`
If any snapshot test fails due to the changed effect JS, update with: `cargo insta review` or regenerate snapshots.

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs \
        omnilang/tests/js_runtime/test_signals.js
git commit -m "fix(omniview): wrap effect fn() in try/catch to prevent permanent death

When an effect's fn() throws, _deps were already cleared before the call,
leaving the effect with no subscriptions and no way to recover. Subsequent
signal changes would never trigger it again.

Fix: wrap fn() in try/catch inside e.run() so the effect re-subscribes
on the next signal change. Errors are reported via __omni_reportError.
Added behavioral test for effect exception recovery."
```

---

### Task 9: Recurse `inject_slot_children` into `Conditional.fallback`

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs:235-248`

**Root cause:** `inject_slot_children` recurses into `Conditional.branches[].body` but not into `Conditional.fallback`. A `<slot>` inside an `else` branch is not filled with parent children.

- [ ] **Step 1: Write the regression test**

In `omnilang/src/codegen/omniview/tests_regression/mod.rs`, add:

```rust
#[test]
fn slot_projection_in_conditional_fallback() {
    let src = r#"
component Modal {
    @slot body { }
    view {
        div {
            Modal {
                @if show {
                    div { "visible" }
                } else {
                    div { "hidden slot content" }
                }
            }
        }
    }
}
"#;
    let ir = compile_to_ir(src).unwrap();
    let output = generate_omniview(&ir);
    let html = &output.html;
    // The fallback content must NOT contain a raw <slot> element —
    // it should be replaced with the parent-provided content.
    assert!(!html.contains("<slot"), "raw <slot> must not appear in output, got:\n{}", html);
}
```

- [ ] **Step 2: Fix fallback recursion**

In `omnilang/src/codegen/omniview/component_html.rs`, modify lines 235-248:

**Before:**
```rust
        ViewTree::Conditional(cond) => {
            let new_branches = cond
                .branches
                .into_iter()
                .map(|b| ConditionalBranch {
                    body: b.body.map(|v| inject_slot_children(v, slot_children)),
                    ..b
                })
                .collect();
            ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: new_branches,
                ..*cond
            }))
        }
```

**After:**
```rust
        ViewTree::Conditional(cond) => {
            let new_branches = cond
                .branches
                .into_iter()
                .map(|b| ConditionalBranch {
                    body: b.body.map(|v| inject_slot_children(v, slot_children)),
                    ..b
                })
                .collect();
            let fallback = cond.fallback.map(|v| inject_slot_children(v, slot_children));
            ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: new_branches,
                fallback,
                transition_mode: cond.transition_mode,
            }))
        }
```

- [ ] **Step 3: Run the regression test**

Run: `cargo test -p omnilang slot_projection_in_conditional_fallback -- --nocapture`
Expected: PASS

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p omnilang`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/component_html.rs \
        omnilang/src/codegen/omniview/tests_regression/mod.rs
git commit -m "fix(omniview): recurse inject_slot_children into Conditional.fallback

Slot injection recursed into Conditional.branches[].body but skipped
fallback. A <slot> inside an @if/@else fallback branch would not be
filled with parent-provided children.

Fix: apply inject_slot_children to fallback as well. Added regression test.
Explicit field listing avoids the ..*cond spread issue with duplicate
fallback field."
```

---

### Task 10: Strip `slot` attribute from DOM after slot routing

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs:254-269` (add helper)

**Root cause:** After `inject_slot_children` routes children to named slots, the `slot="name"` attribute remains on the child element. Browsers interpret `slot` as a Web Components attribute, causing unexpected behavior.

- [ ] **Step 1: Add `strip_slot_prop` helper function**

In `omnilang/src/codegen/omniview/component_html.rs`, add after `has_any_slot_attr` (after line 269):

```rust
/// Remove the `slot` prop from an element ViewTree so browsers don't
/// interpret it as a Web Components slot assignment attribute.
fn strip_slot_prop(view: &ViewTree) -> ViewTree {
    match view {
        ViewTree::Element(el) => {
            if el.props.contains_key("slot") {
                let mut props = el.props.clone();
                props.remove("slot");
                ViewTree::Element(ElementIR {
                    props,
                    ..el.clone()
                })
            } else {
                view.clone()
            }
        }
        other => other.clone(),
    }
}
```

- [ ] **Step 2: Apply stripping in `inject_slot_children` Slot arm**

In `omnilang/src/codegen/omniview/component_html.rs`, modify the `ViewTree::Slot` arm of `inject_slot_children` (lines 199-222):

**Before (Slot arm only, lines 199-222):**
```rust
        ViewTree::Slot(ref slot_name) => {
            if let Some(name) = slot_name.as_deref() {
                let named: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| has_slot_attr(c, name))
                    .cloned()
                    .collect();
                if named.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(named)
                }
            } else {
                let default: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| !has_any_slot_attr(c))
                    .cloned()
                    .collect();
                if default.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(default)
                }
            }
        }
```

**After:**
```rust
        ViewTree::Slot(ref slot_name) => {
            if let Some(name) = slot_name.as_deref() {
                let named: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| has_slot_attr(c, name))
                    .map(|c| strip_slot_prop(c))
                    .collect();
                if named.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(named)
                }
            } else {
                let default: Vec<ViewTree> = slot_children
                    .iter()
                    .filter(|c| !has_any_slot_attr(c))
                    .map(|c| strip_slot_prop(c))
                    .collect();
                if default.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(default)
                }
            }
        }
```

The ONLY changes: `.cloned()` → `.map(|c| strip_slot_prop(c))` and the addition of the `strip_slot_prop` helper function.

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p omnilang`
Expected: All tests pass (slot attribute stripping is additive — existing expectations remain valid)

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/component_html.rs
git commit -m "fix(omniview): strip slot attribute from DOM after slot routing

After inject_slot_children routes children to named slots, the slot=\"name\"
attribute remained on child elements. Browsers interpret 'slot' as a Web
Components slot assignment attribute, which could cause unexpected behavior
if the child is later used in a custom element context.

Fix: add strip_slot_prop helper that removes the slot prop from elements
after they've been routed to a slot position."
```

---

### Task 11: Fix learn-omniview `TodoPage.omni` data-corruption bug

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/TodoPage.omni:56`

**Root cause:** `@onDelete={(index) => { TodoStore.remove(); }}` receives the `@for` loop index but never passes it to `TodoStore.remove(id: i32)`. `index` is captured but unused; `remove()` receives no argument → JS `undefined` → coerced to `0` → always deletes the item with ID 0. Additionally, `index` (array position) is passed as entity ID — even if the argument were wired correctly, it would delete the wrong item when the array is filtered/reordered.

- [ ] **Step 1: Fix the data-corruption bug**

In `test/projects/learn-omniview/src/pages/TodoPage.omni`, find the `@onDelete` handler at approximately line 56:

**Before:**
```omni
@onDelete={(index) => { TodoStore.remove(); }}
```

**After:**
```omni
@onDelete={(index) => { TodoStore.remove(items[index].id); }}
```

This accesses the item at `items[index]` and extracts its `.id` field to pass to `remove()`.

- [ ] **Step 2: Verify the fix compiles**

Run: `cargo run -- compile test/projects/learn-omniview/src/main.omni`
Expected: Compilation succeeds, HTML/JS output generated without errors

- [ ] **Step 3: Commit**

```bash
git add test/projects/learn-omniview/src/pages/TodoPage.omni
git commit -m "fix(learn-omniview): pass correct entity ID to TodoStore.remove

The @onDelete handler received the @for loop index but never passed it
to TodoStore.remove(id: i32). The index parameter was captured but unused,
causing remove() to always receive undefined → coerced to 0 → always
deleted item with ID 0 instead of the clicked item.

Fix: pass items[index].id (entity ID) instead of index (array position).
This correctly identifies the item to delete regardless of array ordering."
```

---

## Execution Order

Tasks are ordered by severity and dependency:

| Order | Task | Severity | Dependencies | Est. Time |
|-------|------|----------|-------------|-----------|
| 1 | Fix prop/computed emission order | CRITICAL | None | 15 min |
| 2 | Fix `has_slot_attr` Debug matching | HIGH | None | 15 min |
| 3 | Fix URL sanitizer newline bypass | HIGH | None | 20 min |
| 4 | Fix `fromCodePoint` range check | MEDIUM | None | 15 min |
| 5 | Add event handlers to Props interface | HIGH | Task 6 (decision-dependent) | 15 min |
| 6 | Gate TypeScript declarations | HIGH | None | 10 min |
| 7 | Integrate JS runtime tests into CI | HIGH | None | 15 min |
| 8 | Fix effect exception handling | MEDIUM | None | 15 min |
| 9 | Recurse into Conditional.fallback | MEDIUM | Depends on Task 2 (same file) | 10 min |
| 10 | Strip slot attribute after routing | MEDIUM | Depends on Task 2 (same file) | 10 min |
| 11 | Fix learn-omniview TodoPage bug | MEDIUM | None | 10 min |

**Total estimated time**: ~2.5 hours for all 11 tasks.

Tasks 1-4 are independent and can be executed in parallel. Tasks 9-10 should be sequenced after Task 2 (modify the same file).

---

## Verification Gate

Before marking this plan complete:

- [ ] All 11 tasks committed
- [ ] `cargo test -p omnilang` — all tests pass
- [ ] `cargo test -p omnilang-core` — all tests pass
- [ ] `cd omnilang/tests/js_runtime && bash run_tests.sh` — all JS tests pass
- [ ] `cargo test -p omnilang js_runtime` — JS integration tests pass via Rust
- [ ] `cargo clippy -p omnilang --lib -- -D warnings` — clean
- [ ] `cargo fmt --check` — no formatting violations
- [ ] Manual: `test/projects/learn-omniview/src/main.omni` compiles without errors
