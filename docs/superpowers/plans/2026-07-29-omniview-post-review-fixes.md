# OmniView Post-ADR-026 Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 11 findings (5 P1 critical + 6 P2 important) from the post-ADR-026 remediation architecture review, closing security scope gaps, a nested @for correctness bug, and Rust edge cases.

**Architecture:** All fixes are localized to the OmniView codegen in `omnilang/src/codegen/omniview/`. No new modules, no trait changes, no cross-crate impacts. Each task is independently testable and committable. The fixes fall into three categories: (1) one-line correctness fixes (P1-1, P1-5), (2) runtime JS sanitization improvements (P1-2, P1-4, P2-1), (3) compile-time hardening (P1-3, P2-3, P2-4, P2-5, P2-6).

**Tech Stack:** Rust (omnilang crate), JavaScript (embedded runtime in Rust strings), insta snapshots for regression tests

## Global Constraints

- Zero `unwrap()`/`expect()` in production paths — use `debug_assert!` or `unreachable!` where invariants should never break
- ES5 compatibility for all runtime JS changes — `function`, `var`, `indexOf`, no arrow functions, no `const`/`let`
- All changes must pass `cargo test -p omnilang` and `cargo clippy` with zero new warnings
- Snapshot tests (`insta`) must be reviewed with `cargo insta review` after any JS output change
- Commit granularity: one commit per task, conventional commits format (`fix(omniview): ...`)

---

### Task 1: Fix direct nested @for `in_for_template` leak

**Files:**
- Modify: `omnilang/src/codegen/omniview/list.rs:58-60`
- Modify: `omnilang/src/codegen/omniview/tests_regression/list.rs` (add test at end)

**Interfaces:**
- Consumes: `OmniViewGenerator.in_for_template: bool`
- Produces: Correct template generation for nested @for loops — outer loop's template mode restored after inner loop returns

- [ ] **Step 1: Write the failing regression test**

Add at end of `tests_regression/list.rs`:

```rust
/// Tests that a direct nested @for loop preserves the outer @for's template
/// mode. Before the fix, the inner @for's `emit_list_state_init` unconditionally
/// set `in_for_template = false`, causing content after the inner loop to
/// generate non-@for JS (silent no-ops in cloned DOM).
#[test]
fn test_nested_for_preserves_outer_template_mode() {
    let mut gen = OmniViewGenerator::test_new();
    // Simulate outer @for entering template mode
    gen.in_for_template = true;

    // Inner @for: its emit_list_state_init should save/restore the flag
    let inner_list = ListRenderingIntent {
        iterable: ExprIR::LiteralExpr {
            value: serde_json::json!([1, 2]),
        },
        item_binding: BindingIR {
            name: "inner".into(),
            r#type: None,
        },
        body: Some(ir::ViewIR::TextNode {
            text: "x".into(),
            source_span: None,
        }),
        key_expr: None,
        empty_body: None,
    };

    gen.emit_list_rendering_html(&inner_list, 0);

    // After inner @for returns, outer template mode must be preserved
    assert!(
        gen.in_for_template,
        "outer @for template mode must be preserved after inner @for returns"
    );
}

/// Verifies that outside @for context, emit_list_state_init sets
/// in_for_template to false (the normal exit behavior).
#[test]
fn test_normal_for_sets_template_false_on_exit() {
    let mut gen = OmniViewGenerator::test_new();
    assert!(!gen.in_for_template);

    let list = ListRenderingIntent {
        iterable: ExprIR::LiteralExpr {
            value: serde_json::json!([1, 2, 3]),
        },
        item_binding: BindingIR {
            name: "item".into(),
            r#type: None,
        },
        body: Some(ir::ViewIR::TextNode {
            text: "y".into(),
            source_span: None,
        }),
        key_expr: None,
        empty_body: None,
    };

    gen.emit_list_rendering_html(&list, 0);

    assert!(
        !gen.in_for_template,
        "in_for_template must be false after non-nested @for exits"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omnilang -- test_nested_for_preserves_outer_template_mode`
Expected: FAIL — `assertion failed: gen.in_for_template` (inner @for set it to false)

- [ ] **Step 3: Implement the fix**

In `list.rs`, replace lines 58-60:

**Before:**
```rust
            self.in_for_template = true;
            let html = self.view_to_html(body, indent + 1);
            self.in_for_template = false;
```

**After:**
```rust
            // Save outer @for template flag to support direct nesting.
            // Inner @for loops set in_for_template = true → false on exit,
            // which would corrupt the outer loop's template generation for
            // any content after the inner loop. Restoring the saved value
            // ensures the outer loop continues in the correct mode.
            let outer_in_for = self.in_for_template;
            self.in_for_template = true;
            let html = self.view_to_html(body, indent + 1);
            self.in_for_template = outer_in_for;
```

- [ ] **Step 4: Run tests to verify fix**

Run: `cargo test -p omnilang -- test_nested_for_preserves_outer_template_mode test_normal_for_sets_template_false_on_exit`
Expected: both PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p omnilang`
Expected: all omniview tests pass, no regressions

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/list.rs omnilang/src/codegen/omniview/tests_regression/list.rs
git commit -m "fix(omniview): save/restore in_for_template for direct nested @for loops

The inner @for's emit_list_state_init unconditionally set
in_for_template = false, leaking state to the outer @for and causing
content after the inner loop to generate wrong JS patterns (non-@for
effects referencing template-only nids).

Use the same save/restore pattern already in try_inline_component:
save the outer value before entering template mode, restore after
template generation completes."
```

---

### Task 2: Extend URL sanitization to block all `data:` schemes

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:136`
- Modify: `omnilang/src/codegen/omniview/tests.rs` (add test or verify via existing fixture)

**Interfaces:**
- Consumes: None (standalone runtime JS function)
- Produces: `__omni_sanitizeUrl` now blocks all `data:` URI schemes, not just `data:text/html`

- [ ] **Step 1: Update the sanitization logic**

In `runtime.rs`, inside the `emit_signal_runtime` method, replace the `__omni_sanitizeUrl` condition on line 136:

**Before:**
```javascript
function __omni_sanitizeUrl(url) {
  if (typeof url !== 'string') return url;
  var trimmed = url.trim().toLowerCase();
  if (trimmed.indexOf('javascript:') === 0 || trimmed.indexOf('data:text/html') === 0 || trimmed.indexOf('vbscript:') === 0) {
    return '#blocked';
  }
  return url;
}
```

**After:**
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

Note: This also incorporates the P1-4 fix (`String(url)` coercion) — see Task 4 for the CSS counterpart.

- [ ] **Step 2: Review snapshot changes**

Run: `cargo test -p omnilang`
Expected: some insta snapshots will differ (the `__omni_sanitizeUrl` function body changed). Review with `cargo insta review` and accept the new function body.

- [ ] **Step 3: Verify new behavior via existing fixture tests**

Run: `cargo test -p omnilang -- regression_omniview`
Expected: all regression tests pass (the sanitizer function body changed but no call sites changed — existing snapshot output should update cleanly)

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git add omnilang/src/codegen/omniview/tests_regression/snapshots/
git commit -m "fix(omniview): block all data: URI schemes in __omni_sanitizeUrl

Previously only blocked data:text/html. data:text/javascript and other
data: subtypes bypass the sanitizer. Block the entire data: scheme
prefix since data: URIs can carry arbitrary executable content.

Also add String() coercion (Task 4) to prevent toString() bypass when
non-string values are passed to the sanitizer."
```

---

### Task 3: Add `base-uri` and `frame-ancestors` to CSP

**Files:**
- Modify: `omnilang/src/codegen/omniview/utils.rs:248`

**Interfaces:**
- Consumes: None (standalone string literal)
- Produces: CSP meta tag now includes `base-uri 'self'` and `frame-ancestors 'self'`

- [ ] **Step 1: Update the CSP string**

In `utils.rs`, replace the CSP meta tag on line 248:

**Before:**
```rust
let csp_meta = if include_csp {
    r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self';">
"#
} else {
    ""
};
```

**After:**
```rust
let csp_meta = if include_csp {
    r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self'; base-uri 'self'; frame-ancestors 'self';">
"#
} else {
    ""
};
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang`
Expected: snapshot tests that include CSP will differ. Accept with `cargo insta review`.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/utils.rs
git add omnilang/src/codegen/omniview/tests_regression/snapshots/
git commit -m "fix(omniview): add base-uri and frame-ancestors to CSP meta tag

base-uri 'self' prevents <base href> tag hijacking of resource URLs.
frame-ancestors 'self' prevents clickjacking via cross-origin iframe
embedding. Both are standard defense-in-depth CSP directives."
```

---

### Task 4: Add `String()` coercion to `__omni_sanitizeCss`

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:143`

**Interfaces:**
- Consumes: None
- Produces: `__omni_sanitizeCss` now coerces input to string before validation

> **Note:** The URL sanitizer coercion (`String(url)`) was already applied in Task 2. This task handles the CSS counterpart.

- [ ] **Step 1: Update `__omni_sanitizeCss`**

In `runtime.rs`, replace the `__omni_sanitizeCss` function (lines 142-153):

**Before:**
```javascript
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
```

**After:**
```javascript
function __omni_sanitizeCss(value) {
  var str = String(value);
  var lower = str.toLowerCase();
  if (lower.indexOf('javascript:') >= 0 || lower.indexOf('expression(') >= 0 ||
      lower.indexOf('url(') >= 0 || lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 || lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 || lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {
    return '';
  }
  return str;
}
```

Key change: `var str = String(value)` replaces `if (typeof value !== 'string') return value`, and returns `str` instead of the original `value`. This prevents `toString()` bypass when a computed signal returns an object with a custom `toString` method that produces `javascript:alert(1)`.

- [ ] **Step 2: Run tests and review snapshots**

Run: `cargo test -p omnilang`
Review: `cargo insta review` — the `__omni_sanitizeCss` function body changed in embedded runtime output.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix(omniview): add String() coercion to __omni_sanitizeCss

Prevents toString() bypass where a computed signal returns an object
with a custom toString method that produces dangerous CSS values.
The typeof !== 'string' guard returned the raw object, which would
be coerced later by the DOM API and potentially inject malicious CSS."
```

---

### Task 5: Fix UTF-8 byte boundary slice in `expr_default_display`

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:24-25`

**Interfaces:**
- Consumes: `gen.expr_to_js(expr) -> String`
- Produces: Truncated display string, char-boundary-safe

- [ ] **Step 1: Replace the unsafe byte slice**

In `store.rs`, replace lines 23-25:

**Before:**
```rust
            let js = gen.expr_to_js(expr);
            // Truncate long expressions
            if js.len() > 40 {
                format!("{}…", &js[..39])
            } else {
                js
            }
```

**After:**
```rust
            let js = gen.expr_to_js(expr);
            // Truncate long expressions — use char boundary to avoid UTF-8 panics.
            // Matches the pattern used in expr.rs:382-388.
            if js.chars().count() > 40 {
                let end = js
                    .char_indices()
                    .nth(39)
                    .map(|(i, _)| i)
                    .unwrap_or(js.len());
                format!("{}…", &js[..end])
            } else {
                js
            }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang`
Expected: all pass. No snapshot changes — this only affects the display string in debug/warning output.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs
git commit -m "fix(omniview): use char_indices for safe UTF-8 truncation in expr_default_display

The previous byte-index slice &js[..39] would panic on multi-byte
UTF-8 characters straddling the byte boundary. Use the same
char_indices() pattern already established in expr.rs:382-388."
```

---

### Task 6: Relax CSS `url()` blocking to allow safe URL schemes

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:142-153`

**Interfaces:**
- Consumes: None
- Produces: `__omni_sanitizeCss` now allows `url()` containing https://, http://, /, or data:image/ while still blocking `url(javascript:...)` and `url(vbscript:...)`

- [ ] **Step 1: Update `__omni_sanitizeCss`**

Replace the `__omni_sanitizeCss` function in `runtime.rs`:

**After:**
```javascript
function __omni_sanitizeCss(value) {
  var str = String(value);
  var lower = str.toLowerCase();
  // Block protocol-level injection patterns
  if (lower.indexOf('javascript:') >= 0 || lower.indexOf('expression(') >= 0 ||
      lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 || lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 || lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {
    return '';
  }
  // Allow url() only with safe schemes. url() is needed for
  // background-image and other standard CSS resource loading.
  // Block url() containing javascript: or vbscript: schemes.
  // The outer block already catches standalone javascript:/vbscript:,
  // but they could appear inside a url() value after stripping.
  var urlIdx = lower.indexOf('url(');
  if (urlIdx >= 0) {
    var afterUrl = lower.substring(urlIdx);
    if (afterUrl.indexOf('javascript:') >= 0 || afterUrl.indexOf('vbscript:') >= 0) {
      return '';
    }
  }
  return str;
}
```

Design rationale:
- First pass: block known injection patterns (unchanged from original)
- Second pass: if `url()` is present, check only the content after `url(` for dangerous protocols — this allows `url("https://example.com/bg.png")` and `url("/assets/bg.png")` while still blocking `url(javascript:alert(1))`

- [ ] **Step 2: Run tests and review snapshots**

Run: `cargo test -p omnilang`
Review: `cargo insta review` — the `__omni_sanitizeCss` function body changed significantly.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix(omniview): relax CSS url() blocking to allow safe URL schemes

The previous blanket url() block prevented legitimate signal-driven
background-image and other url()-based CSS values from working,
creating pressure on developers to bypass the sanitizer entirely.

Now url() is allowed as long as it does not contain javascript: or
vbscript: protocols. Known injection patterns (expression(), -moz-
binding, @import, etc.) are still blocked in the first pass."
```

---

### Task 7: Replace `debug_assert!(false)` with `unreachable!` for `LifecycleKind::Effect`

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs:332-337`

**Interfaces:**
- Consumes: None (match arm)
- Produces: Release builds now panic instead of silently skipping Effect emission

- [ ] **Step 1: Replace the debug-only assertion**

In `store.rs`, replace lines 332-337:

**Before:**
```rust
                    LifecycleKind::Effect => {
                        debug_assert!(
                            false,
                            "LifecycleKind::Effect should not reach codegen; filtered during IR lowering"
                        );
                    }
```

**After:**
```rust
                    LifecycleKind::Effect => {
                        // IR lowering must filter Effect before codegen.
                        // If this invariant breaks in production, panic loudly
                        // rather than silently skipping the effect emission.
                        unreachable!(
                            "LifecycleKind::Effect should not reach codegen; filtered during IR lowering"
                        );
                    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang`
Expected: all pass. No snapshot changes (this code path should never execute).

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/store.rs
git commit -m "fix(omniview): use unreachable! for LifecycleKind::Effect invariant

debug_assert!(false) is silent in release builds. If the IR lowering
filter ever breaks and lets Effect through, the codegen would
silently skip effect emission — producing subtly broken output with
missing reactive behavior. unreachable! panics in all build modes,
making the invariant violation immediately visible."
```

---

### Task 8: Add `srcset` and `poster` to URL sanitization scope

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:149`

**Interfaces:**
- Consumes: `is_url_attr` check in attribute emission
- Produces: `srcset` and `poster` dynamic bindings now pass through `__omni_sanitizeUrl`

- [ ] **Step 1: Extend the URL attribute list**

In `element.rs`, replace line 149:

**Before:**
```rust
                        let is_url_attr =
                            matches!(name.as_str(), "href" | "src" | "action" | "formaction");
```

**After:**
```rust
                        let is_url_attr =
                            matches!(name.as_str(), "href" | "src" | "srcset" | "action" | "formaction" | "poster");
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang`
Expected: all pass. Snapshot tests may update if any test fixture includes srcset or poster dynamic attributes.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "fix(omniview): add srcset and poster to URL sanitization scope

srcset (responsive images) and poster (video preview) accept URLs
but were not covered by the __omni_sanitizeUrl runtime sanitizer.
Dynamic signal bindings on these attributes could carry dangerous
URL schemes."
```

---

### Task 9: Add compile-time protocol sanitization for static URL attributes

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:175`

**Interfaces:**
- Consumes: `val_str` (static attribute value from `expr_to_static_str`)
- Produces: Static `href`/`src`/`action` values are checked for dangerous protocols at compile time

- [ ] **Step 1: Add protocol check helper**

In `element.rs`, add a module-level function before the `impl OmniViewGenerator` block that contains `emit_element_props`:

```rust
/// Check whether a static URL attribute value contains a dangerous protocol
/// scheme. Returns `true` if the value should be blocked.
///
/// This mirrors `__omni_sanitizeUrl` in the generated JS runtime, catching
/// dangerous URLs in literal attribute values at compile time as defense-in-depth
/// against supply-chain compromises where a third-party .omni component
/// contains literal `javascript:` or `data:` URLs.
fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
}
```

- [ ] **Step 2: Integrate into static attribute emission**

In `element.rs`, replace the static attribute emission on line 174-175:

**Before:**
```rust
                    } else if !val_str.is_empty() {
                        let _ = write!(attrs, " {}=\"{}\"", name, escape_html(&val_str));
```

**After:**
```rust
                    } else if !val_str.is_empty() {
                        let is_static_url = matches!(
                            name.as_str(),
                            "href" | "src" | "srcset" | "action" | "formaction" | "poster"
                        );
                        if is_static_url && is_dangerous_url(&val_str) {
                            // Block dangerous URL schemes in static attributes.
                            // This is defense-in-depth: dynamic bindings are protected
                            // by __omni_sanitizeUrl at runtime; this catches literal
                            // dangerous URLs at compile time (supply-chain defense).
                            self.warnings
                                .push(format!("Blocked dangerous URL in static {} attribute: scheme not allowed", name));
                            // Emit a safe fallback value
                            let _ = write!(attrs, " {}=\"#blocked\"", name);
                        } else {
                            let _ = write!(attrs, " {}=\"{}\"", name, escape_html(&val_str));
                        }
```

- [ ] **Step 3: Add unit test for the helper**

In `tests.rs`, add:

```rust
#[test]
fn test_is_dangerous_url() {
    use super::view::element::is_dangerous_url;
    assert!(is_dangerous_url("javascript:alert(1)"));
    assert!(is_dangerous_url("  javascript:void(0)"));
    assert!(is_dangerous_url("data:text/html,<script>alert(1)</script>"));
    assert!(is_dangerous_url("data:image/svg+xml,..."));
    assert!(is_dangerous_url("vbscript:msgbox(1)"));
    assert!(!is_dangerous_url("https://example.com"));
    assert!(!is_dangerous_url("/relative/path"));
    assert!(!is_dangerous_url("#anchor"));
    assert!(!is_dangerous_url("mailto:user@example.com"));
}
```

Note: `is_dangerous_url` needs to be `pub(super)` to be testable from `tests.rs`. Since `tests.rs` is in the parent module (`omniview`), and `element.rs` is in `view/`, use `pub(crate)` visibility.

- [ ] **Step 4: Run tests**

Run: `cargo test -p omnilang`
Expected: all pass including new URL test. Snapshot tests with static URL attributes may differ.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "fix(omniview): add compile-time protocol sanitization for static URL attributes

Dynamic URL bindings are protected by __omni_sanitizeUrl at runtime,
but literal URL values in .omni source bypassed sanitization entirely.
This is defense-in-depth against supply-chain compromises where a
third-party component package contains literal javascript: or data:
URLs."
```

---

### Task 10: Extract CSS-only capture to shared method

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs` — extract shared method, replace two call sites

**Interfaces:**
- Consumes: `el: &ElementIR`, `nid: &str`, `html_tag: &str`, `class_name_parts: &[String]`
- Produces: `emit_element_css_only_capture(el, nid, html_tag, class_name_parts)` — replaces 31 duplicated lines in two branches

- [ ] **Step 1: Extract the shared method**

In `element.rs`, add a new method to `impl OmniViewGenerator` (before the `emit_element_html_close` method):

```rust
    /// In css_only mode, capture element metadata for the editor's
    /// element tree: design ID, resolved CSS classes, HTML tag, and
    /// nid mapping. Called from both void-tag and normal-tag branches
    /// of `emit_element_html_close`.
    fn emit_element_css_only_capture(
        &mut self,
        el: &ElementIR,
        nid: &str,
        html_tag: &str,
        class_name_parts: &[String],
    ) {
        let design_id = el
            .props
            .get("id")
            .and_then(|v| {
                if let ExprIR::LiteralExpr { value } = v {
                    value.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| nid.to_string());
        // Resolve design class references ("AppDesign.card" → "card")
        let resolved_classes: Vec<String> = class_name_parts
            .iter()
            .map(|c| {
                if let Some(dot_pos) = c.find('.') {
                    c[dot_pos + 1..].to_string()
                } else {
                    c.clone()
                }
            })
            .collect();
        self.element_classes
            .insert(nid.to_string(), resolved_classes);
        self.element_tags
            .insert(nid.to_string(), html_tag.to_string());
        self.nid_map.insert(nid.to_string(), design_id);
        self.tree_pop();
    }
```

- [ ] **Step 2: Replace the void-tag CSS-only block**

Replace lines 490-520 (the entire `if self.css_only { ... return String::new(); }` block in the void-tag path):

**Before:**
```rust
            if self.css_only {
                let design_id = el
                    .props
                    .get("id")
                    .and_then(|v| {
                        if let ExprIR::LiteralExpr { value } = v {
                            value.as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| nid.to_string());
                // Resolve design class references ("AppDesign.card" → "card")
                let resolved_classes: Vec<String> = class_name_parts
                    .iter()
                    .map(|c| {
                        if let Some(dot_pos) = c.find('.') {
                            c[dot_pos + 1..].to_string()
                        } else {
                            c.clone()
                        }
                    })
                    .collect();
                self.element_classes
                    .insert(nid.to_string(), resolved_classes);
                self.element_tags
                    .insert(nid.to_string(), html_tag.to_string());
                // Record nid → design element ID mapping for editor selection
                self.nid_map.insert(nid.to_string(), design_id);
                self.tree_pop();
                return String::new();
            }
```

**After:**
```rust
            if self.css_only {
                self.emit_element_css_only_capture(el, nid, html_tag, &class_name_parts);
                return String::new();
            }
```

- [ ] **Step 3: Replace the normal-tag CSS-only block**

Replace lines 564-594 (the entire `if self.css_only { ... return String::new(); }` block in the normal-tag path) with the same single call:

**After:**
```rust
            if self.css_only {
                self.emit_element_css_only_capture(el, nid, html_tag, &class_name_parts);
                return String::new();
            }
```

- [ ] **Step 4: Run tests and verify zero snapshot changes**

Run: `cargo test -p omnilang`
Review: `cargo insta review` — expect **zero snapshot differences** (pure refactor, identical output).

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "refactor(omniview): extract shared emit_element_css_only_capture method

The 31-line CSS-only mode capture block was duplicated verbatim in
the void-tag and normal-tag branches of emit_element_html_close.
Extract to a single method. Zero behavioral change — verified by
identical snapshot output."
```

---

### Task 11: Add runtime sanitization JS tests (deferred I10)

**Files:**
- Create: `omnilang/src/codegen/omniview/tests_regression/sanitizers.rs`
- Modify: `omnilang/src/codegen/omniview/tests_regression/mod.rs` (add module)

**Interfaces:**
- Consumes: `OmniViewGenerator::test_new()`, `gen.js` (the generated JS output containing sanitizer function bodies)
- Produces: Tests that verify the embedded sanitizer JS function bodies contain correct logic

- [ ] **Step 1: Create the test file**

Create `omnilang/src/codegen/omniview/tests_regression/sanitizers.rs`:

```rust
//! Regression tests for runtime sanitization functions (__omni_sanitizeUrl
//! and __omni_sanitizeCss) embedded in the generated JS runtime.
//!
//! These tests verify that the generated function bodies contain the
//! correct blocking patterns. Full behavioral verification requires
//! a JS test runner; these tests provide compile-time regression
//! protection against accidental changes to the sanitizer logic.

use super::super::*;

/// Verify __omni_sanitizeUrl is emitted and blocks key dangerous schemes.
#[test]
fn test_sanitize_url_blocks_dangerous_schemes() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_signal_runtime();

    let js = &gen.js;

    // Function is defined
    assert!(
        js.contains("function __omni_sanitizeUrl"),
        "sanitizeUrl function must be emitted"
    );

    // Uses String() coercion for toString bypass prevention
    assert!(
        js.contains("String(url)"),
        "must coerce input to string to prevent toString() bypass"
    );

    // Blocks javascript: scheme
    assert!(
        js.contains("javascript:") && js.contains("'#blocked'"),
        "must block javascript: URLs"
    );

    // Blocks data: scheme (all data: URIs, not just data:text/html)
    assert!(
        js.contains("data:") && js.contains("=== 0"),
        "must block all data: URI schemes"
    );

    // Blocks vbscript: scheme
    assert!(
        js.contains("vbscript:") && js.contains("'#blocked'"),
        "must block vbscript: URLs"
    );

    // Returns #blocked on match
    assert!(
        js.contains("return '#blocked'"),
        "must return '#blocked' for dangerous URLs"
    );
}

/// Verify __omni_sanitizeCss is emitted and blocks key injection patterns.
#[test]
fn test_sanitize_css_blocks_injection_patterns() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_signal_runtime();

    let js = &gen.js;

    // Function is defined
    assert!(
        js.contains("function __omni_sanitizeCss"),
        "sanitizeCss function must be emitted"
    );

    // Uses String() coercion
    assert!(
        js.contains("String(value)"),
        "must coerce input to string"
    );

    // Blocks core CSS injection patterns
    assert!(
        js.contains("javascript:") && js.contains("return ''"),
        "must block javascript: in CSS values"
    );
    assert!(
        js.contains("expression("),
        "must block CSS expression() injection"
    );
    assert!(
        js.contains("-moz-binding"),
        "must block -moz-binding injection"
    );
    assert!(
        js.contains("@import"),
        "must block @import injection"
    );

    // Allows url() with safety check (Task 6)
    assert!(
        js.contains("url("),
        "must contain url() handling logic"
    );
    // After Task 6: should contain url() validation but not blanket block
    assert!(
        js.contains("urlIdx") || js.contains("afterUrl"),
        "must validate url() content rather than blanket-block"
    );
}

/// Verify both sanitizer functions exist in the emitted runtime.
#[test]
fn test_both_sanitizers_emitted() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_signal_runtime();

    let js = &gen.js;

    assert!(js.contains("function __omni_sanitizeUrl"));
    assert!(js.contains("function __omni_sanitizeCss"));
    // Both functions must be ES5-compatible (no arrow functions, no const/let)
    assert!(!js.contains("const __omni_sanitizeUrl"));
    assert!(!js.contains("let __omni_sanitizeUrl"));
    assert!(!js.contains("=> __omni_sanitizeUrl"));
}
```

- [ ] **Step 2: Register the module**

In `tests_regression/mod.rs` (or at the top of the existing test module), add:

```rust
mod sanitizers;
```

If `tests_regression` is a directory-based module, ensure `sanitizers.rs` is included.

- [ ] **Step 3: Run the new tests**

Run: `cargo test -p omnilang -- sanitize`
Expected: all 3 new sanitizer tests pass.

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p omnilang`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/tests_regression/sanitizers.rs omnilang/src/codegen/omniview/tests_regression/mod.rs
git commit -m "test(omniview): add regression tests for runtime sanitizer functions

Verifies that __omni_sanitizeUrl and __omni_sanitizeCss are emitted
with correct blocking patterns: javascript:, data:, vbscript: for
URLs; expression(), -moz-binding, @import for CSS. Also verifies
ES5 compatibility (no arrow functions, no const/let).

This addresses deferred item I10 from the v2 architecture review."
```

---
