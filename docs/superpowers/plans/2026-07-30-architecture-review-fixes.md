# Architecture Review Fixes — OmniView Codegen System

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the 6 highest-priority issues (3 CRITICAL security + 3 HIGH correctness) identified in the 11-specialist architecture review of the OmniView codegen system, plus 5 infrastructure cleanups.

**Architecture:** The fixes span two codegen backends: React (`omnilang/src/codegen/react/`) and OmniView (`omnilang/src/codegen/omniview/`), plus shared infrastructure (`crates/omnilang-core/`). The root cause pattern is asymmetry — the OmniView debug runtime has security sanitization that the React production backend lacks. Fixes are surgical: import one function, add one parameter, add one line.

**Tech Stack:** Rust (compiler), JavaScript (generated runtime code embedded as Rust strings), TypeScript (generated output tested via snapshots)

**Source:** `.architecture/reviews/component-omniview-2026-07-30.md` — 11-specialist architecture review, 2026-07-30

---

## File Structure

| File | Responsibility | Change |
|------|---------------|--------|
| `omnilang/src/codegen/react/view.rs` | React JSX generation — attributes, events, styles | Add URL/CSS sanitization, add ref_names param to style formatter |
| `omnilang/src/codegen/react/component/effect.rs` | React lifecycle/effect codegen (@mount, @unmount, @effect) | Add body_has_await check to @unmount, fix motion trigger filter |
| `omnilang/src/codegen/omniview/runtime.rs` | OmniView JS runtime — sanitizers, element cache, signal helpers | Add CR/LF/TAB normalization to __omni_sanitizeCss |
| `omnilang/src/codegen/omniview/editor.rs` | OmniView editor bridge — postMessage, hot-replace | Add typeof string guard before eval |
| `omnilang/src/codegen/omniview/mod.rs` | OmniViewGenerator struct, generate() entry point | Fix assert! → debug_assert! doc mismatch |
| `crates/omnilang-core/src/codegen/sanitize.rs` | Shared CSS/URL sanitization (compile-time) | No change — already has sanitize_css_value, is_dangerous_url |
| `.architecture/deferrals.md` | Deferred items tracking | Add ~10 active Phase 2 deferrals |
| `omnilang/src/codegen/omniview/scope.rs` | ComponentScope struct | Derive Default |
| `omnilang/src/codegen/omniview/typescript.rs` | TypeScript declaration generation (disabled) | Gate behind #[cfg(test)] |
| `crates/omnilang-core/src/codegen/shared/ui_codegen.rs` | UiCodegen trait (dead) | Remove |
| `omnilang/src/codegen/react/mod.rs` | React codegen entry — UiCodegen impl | Remove UiCodegen impl block |
| `omnilang/src/codegen/vue/mod.rs` | Vue codegen entry — UiCodegen impl | Remove UiCodegen impl block |
| `omnilang/src/codegen/omniview/utils.rs` | HTML wrapper utilities | Add console.warn to wrap_debug_html |

---

## Tests

Snapshot tests exist at:
- `omnilang/src/codegen/react/tests.rs` — React backend snapshot tests
- `omnilang/src/codegen/omniview/tests.rs` — OmniView backend tests
- `omnilang/src/codegen/omniview/tests_regression/` — Regression tests by category

Each task includes the specific test command. Run `cargo test -p omnilang` after each batch.

---

## Phase 1: Production Security (CRITICAL Priority)

### Task 1: Add URL attribute sanitization to React codegen

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:145-153`

**Why:** The OmniView debug runtime checks `is_dangerous_url()` at compile time for static URL attributes (`href`, `src`, `srcset`, `action`, `formaction`, `poster`). The React production backend — which ships to end users — has zero URL sanitization. A malicious third-party `.omni` component with `href="javascript:alert(1)"` passes through React codegen unsanitized.

- [ ] **Step 1: Read the current React view.rs static attribute emission**

The static string branch at line 146-148 emits attribute values verbatim after backslash/quote escaping:

```rust
// Current code at line 145-148:
} else {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    attrs.push(format!("{}=\"{}\"", jsx_attr, escaped));
}
```

- [ ] **Step 2: Add is_dangerous_url import at the top of view.rs**

At the top of `omnilang/src/codegen/react/view.rs`, find the existing import block (around line 1-20) and add:

```rust
use crate::codegen::omniview::view::element::is_dangerous_url;
```

If this creates a circular dependency or the function is not publicly accessible, instead copy the function inline or move it to `crates/omnilang-core/src/codegen/sanitize.rs` as a shared utility. Check the current visibility:

Run: `grep -n "pub(crate) fn is_dangerous_url" omnilang/src/codegen/omniview/view/element.rs`
Expected: `24:pub(crate) fn is_dangerous_url`

Since it's `pub(crate)` within the `omnilang` crate, the import path works because `react` and `omniview` are both modules within the same crate.

- [ ] **Step 3: Define the URL attribute name set and add the check**

Modify the static string branch at line 145-148 in `view.rs`. The URL attributes that need checking are: `href`, `src`, `srcset`, `action`, `formaction`, `poster`.

```rust
} else {
    // Check URL attributes for dangerous schemes (javascript:, data:, vbscript:)
    // Mirrors the OmniView backend's compile-time is_dangerous_url() defense-in-depth
    // against supply-chain compromises where a third-party .omni component
    // contains literal javascript:/data: URLs.
    let url_attrs: &[&str] = &["href", "src", "srcset", "action", "formaction", "poster"];
    if url_attrs.contains(&jsx_attr) && is_dangerous_url(s) {
        attrs.push(format!("{}=\"#blocked\"", jsx_attr));
    } else {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        attrs.push(format!("{}=\"{}\"", jsx_attr, escaped));
    }
}
```

- [ ] **Step 4: Run Rust compilation check**

Run: `cargo check -p omnilang --features codegen-react,codegen-omniview 2>&1`
Expected: Compilation succeeds (no errors)

- [ ] **Step 5: Run existing React snapshot tests to check for regressions**

Run: `cargo test -p omnilang --features codegen-react,codegen-omniview -- react::tests 2>&1`
Expected: All existing tests pass (no snapshot changes expected since existing fixtures don't use dangerous URLs)

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/view.rs
git commit -m "fix(react): add compile-time URL attribute sanitization to React codegen

Port is_dangerous_url() check from OmniView backend to React production
codegen. Static URL attributes (href, src, srcset, action, formaction,
poster) containing javascript:/data:/vbscript: schemes are now blocked
with '#blocked' value at compile time, matching OmniView behavior.

Security review finding C1: React codegen had zero URL sanitization
while the OmniView debug runtime had three-layer defense-in-depth."
```

---

### Task 2: Add CSS inline style sanitization to React codegen

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:834`

**Why:** The React codegen's `format_style_value_expr` emits static string style values verbatim without calling `sanitize_css_value()`. The OmniView backend wraps both static and dynamic CSS values with sanitizers. While CSS injection is lower risk than URL injection, this is a defense-in-depth gap.

- [ ] **Step 1: Add sanitize_css_value import**

At the top of `omnilang/src/codegen/react/view.rs`, add:

```rust
use omnilang_core::codegen::sanitize::sanitize_css_value;
```

- [ ] **Step 2: Wrap static string style values with sanitize_css_value**

In `format_style_value_expr` at line 834, replace the static string branch:

```rust
// Current:
serde_json::Value::String(s) => format!("'{}'", s),

// Fix:
serde_json::Value::String(s) => {
    let sanitized = sanitize_css_value(s);
    if sanitized.is_empty() {
        // Value was blocked — emit empty string as safe fallback
        "''".to_string()
    } else {
        // Escape single quotes in the sanitized value
        let escaped = sanitized.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{}'", escaped)
    }
}
```

- [ ] **Step 3: Run Rust compilation check**

Run: `cargo check -p omnilang --features codegen-react,codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 4: Run React snapshot tests**

Run: `cargo test -p omnilang --features codegen-react,codegen-omniview -- react::tests 2>&1`
Expected: All existing tests pass

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs
git commit -m "fix(react): add CSS value sanitization to React codegen inline styles

Call sanitize_css_value() on static string style values in
format_style_value_expr, matching the OmniView backend's behavior.
Dangerous CSS patterns (javascript:, expression(), -moz-binding,
data:text/html, @import) are blocked with an empty string fallback.

Security review finding C2: React codegen had zero CSS sanitization
while OmniView debug runtime wraps all style values."
```

---

### Task 3: Fix CSS sanitizer CR/LF/TAB bypass (one-line fix)

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:210-213`

**Why:** The URL sanitizer `__omni_sanitizeUrl` strips CR, LF, TAB characters before pattern matching (line 153: `normalized.replace(/[\r\n\t]/g, '')`). The CSS sanitizer `__omni_sanitizeCss` does NOT strip these characters. An attacker can embed `\r\n` to break keyword substring matching — `jav\r\nascript:` passes through because the literal substring `javascript:` doesn't appear. The browser's CSS parser strips CRLF per spec and processes the injected pattern. This is a one-line fix that has been deferred across two ADRs (ADR-027, ADR-031).

- [ ] **Step 1: Add CR/LF/TAB normalization at the top of __omni_sanitizeCss**

At line 211-213 of `omnilang/src/codegen/omniview/runtime.rs`, the current code is:

```javascript
function __omni_sanitizeCss(value) {
  var str = String(value);
  // Normalize CSS hex escapes before checking for dangerous patterns
  str = __omni_normalizeCssEscapes(str);
```

Add CR/LF/TAB stripping before the hex escape normalization (the strip must come first since a `\r\n` between hex escape characters would break normalization too):

```javascript
function __omni_sanitizeCss(value) {
  var str = String(value);
  // Strip CR, LF, TAB before any pattern matching — browsers strip these
  // characters during CSS tokenization (CSS Syntax §4.3.7), so
  // "jav\r\nascript:" would be normalized by the browser to "javascript:".
  // This mirrors the same normalization in __omni_sanitizeUrl.
  str = str.replace(/[\r\n\t]/g, '');
  // Normalize CSS hex escapes before checking for dangerous patterns
  str = __omni_normalizeCssEscapes(str);
```

- [ ] **Step 2: Run OmniView tests including sanitizer regression tests**

Run: `cargo test -p omnilang --features codegen-omniview -- omniview::tests_regression::sanitizers 2>&1`
Expected: All sanitizer tests pass

- [ ] **Step 3: Run full OmniView test suite**

Run: `cargo test -p omnilang --features codegen-omniview 2>&1`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix(omniview): add CR/LF/TAB normalization to __omni_sanitizeCss

Strip carriage return, line feed, and tab characters before CSS
pattern matching, mirroring the same normalization already present
in __omni_sanitizeUrl. Browsers strip these characters during CSS
tokenization per CSS Syntax §4.3.7, so an attacker could use them
to break keyword substring matching (e.g. 'jav\\r\\nascript:').

Security review finding C3: deferred across ADR-027 and ADR-031."
```

---

## Phase 2: Runtime Correctness (HIGH Priority)

### Task 4: Fix inline style $signal ref-name translation

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:846-848` (format_style_value_expr $signal branch)
- Modify: `omnilang/src/codegen/react/view.rs:467` (call site — add ref_names parameter)

**Why:** Every expression formatter in the React codegen accepts `ref_names: &FxHashSet<String>` to translate ref identifiers to `nameRef.current`. The sole exception is `format_style_value_expr`, which returns bare signal names. If a developer writes `style:width={box.offsetWidth}` where `box` is an `@ref`, the generated code produces `box` instead of `boxRef.current`, causing a runtime crash (`Cannot read properties of undefined`).

- [ ] **Step 1: Change the function signature of format_style_value_expr**

At line 831 of `omnilang/src/codegen/react/view.rs`, change:

```rust
// Current:
pub(crate) fn format_style_value_expr(val: &serde_json::Value) -> String {

// Fix:
pub(crate) fn format_style_value_expr(val: &serde_json::Value, ref_names: &FxHashSet<String>) -> String {
```

- [ ] **Step 2: Add ref-name check in the $signal branch**

At line 847-848, change the `$signal` handler to check ref_names:

```rust
// Current:
if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
    return sig_name.to_string();
}

// Fix:
if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
    if ref_names.contains(sig_name) {
        return format!("{}Ref.current", sig_name);
    }
    return sig_name.to_string();
}
```

- [ ] **Step 3: Update the recursive call in the $concat branch**

At line 876, the recursive call `format_style_value_expr(part)` needs `ref_names`:

```rust
// Current:
let inner = format_style_value_expr(part);

// Fix:
let inner = format_style_value_expr(part, ref_names);
```

- [ ] **Step 4: Update the Array branch recursive calls**

At line 894:

```rust
// Current:
let items: Vec<_> = arr.iter().map(format_style_value_expr).collect();

// Fix:
let items: Vec<_> = arr.iter().map(|v| format_style_value_expr(v, ref_names)).collect();
```

- [ ] **Step 5: Update the call site in inline style emission**

At line 467, where `format_style_value_expr` is called from the inline style block:

```rust
// Current:
let js_val = format_style_value_expr(val);

// Fix:
let js_val = format_style_value_expr(val, ref_names);
```

- [ ] **Step 6: Run Rust compilation check**

Run: `cargo check -p omnilang --features codegen-react,codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 7: Run React snapshot tests**

Run: `cargo test -p omnilang --features codegen-react,codegen-omniview -- react::tests 2>&1`
Expected: All existing tests pass

- [ ] **Step 8: Commit**

```bash
git add omnilang/src/codegen/react/view.rs
git commit -m "fix(react): add ref_names parameter to format_style_value_expr

Thread ref_names through the inline style expression formatter so
that @ref identifiers in style values are correctly translated to
nameRef.current. Previously, style:width={box.offsetWidth} would
emit 'box' instead of 'boxRef.current', causing a runtime crash.

This was the only expression formatter in the React codegen that
did not accept ref_names. Every other formatter (format_expr_ir_tsx,
generate_statement_tsx, gen_view_node_tsx) already handles this.

TS/React review finding C1."
```

---

### Task 5: Fix async @unmount lifecycle — add body_has_await check

**Files:**
- Modify: `omnilang/src/codegen/react/component/effect.rs:112-129`

**Why:** `@mount` (line 77) and `@effect` (line 132) both check `body_has_await` and wrap async bodies in an async IIFE. `@unmount` (line 112) does not. If a developer writes `await` in an `@unmount` block, the generated cleanup function would contain `await` inside a non-async function, producing semantically broken React code. Cleanup timing guarantees would be violated.

- [ ] **Step 1: Read the @mount handler for reference pattern**

At line 70-83 of `effect.rs`, the `@mount` handler shows the pattern:

```rust
LifecycleKind::Mount => {
    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
    w.emit("useEffect(() => {");
    w.indent();
    if body_has_await {
        w.emit("(async () => {");
        w.indent();
    }
    // ... emit statements ...
    if body_has_await {
        w.dedent();
        w.emit("})();");
    }
    w.dedent();
    w.emit("}, []);");
}
```

- [ ] **Step 2: Add body_has_await check to the @unmount handler**

At line 112-129 of `effect.rs`, change the `@unmount` handler:

```rust
// Current (lines 112-129):
LifecycleKind::Unmount => {
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("return () => {");
    w.indent();
    for stmt in &lc.body.statements {
        expr::generate_statement_tsx_with_mode(
            &mut *w,
            stmt,
            signal_names,
            self.store_mode,
            ref_names,
        );
    }
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
}

// Fix:
LifecycleKind::Unmount => {
    let body_has_await = lc.body.statements.iter().any(stmt_contains_await);
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("return () => {");
    w.indent();
    if body_has_await {
        // Async cleanup: React's cleanup function signature is () => void,
        // so await inside it must be wrapped in an async IIFE that runs eagerly.
        // React does not await the cleanup's return value, but the IIFE ensures
        // await expressions execute in the correct microtask context.
        w.emit("(async () => {");
        w.indent();
    }
    for stmt in &lc.body.statements {
        expr::generate_statement_tsx_with_mode(
            &mut *w,
            stmt,
            signal_names,
            self.store_mode,
            ref_names,
        );
    }
    if body_has_await {
        w.dedent();
        w.emit("})();");
    }
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
}
```

- [ ] **Step 3: Run Rust compilation check**

Run: `cargo check -p omnilang --features codegen-react,codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 4: Run React tests**

Run: `cargo test -p omnilang --features codegen-react,codegen-omniview -- react::tests 2>&1`
Expected: All existing tests pass (existing fixtures don't use async in @unmount)

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/component/effect.rs
git commit -m "fix(react): add body_has_await check to @unmount lifecycle codegen

Match the @mount and @effect handlers which wrap async bodies in an
async IIFE. Without this check, await expressions in @unmount blocks
would produce broken React cleanup functions — React's cleanup
signature is () => void and does not await the return value.

TS/React review finding C2."
```

---

### Task 6: Add input validation to editor bridge eval

**Files:**
- Modify: `omnilang/src/codegen/omniview/editor.rs:94-108`

**Why:** The `__omni_replaceComponent` function calls `(0, eval)(factoryCode)` with the `factoryCode` from `e.data.js` without validating that it's a string. While the primary protection is the `postMessage` origin check (line 68), a same-origin XSS in the parent editor page could send a crafted message with non-string `js` data, causing unexpected behavior. This is defense-in-depth: the iframe sandbox is the primary protection.

- [ ] **Step 1: Add typeof string guard before eval**

At line 94 of `editor.rs`, find the `__omni_replaceComponent` function. Add a typeof check at the beginning of the function body:

Current function start (around line 94):
```javascript
function __omni_replaceComponent(name, factoryCode) {
  var oldComp = window.__omni_components[name];
```

Add validation before the first line of the function body:
```javascript
function __omni_replaceComponent(name, factoryCode) {
  // Defense-in-depth: validate factoryCode is a string before eval.
  // The primary protection is the postMessage origin check, but this
  // guards against same-origin XSS in the parent editor page sending
  // a crafted message with non-string js payload.
  if (typeof factoryCode !== 'string' || factoryCode.length === 0) {
    return;
  }
  var oldComp = window.__omni_components[name];
```

- [ ] **Step 2: Run OmniView tests**

Run: `cargo test -p omnilang --features codegen-omniview 2>&1`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/editor.rs
git commit -m "fix(omniview): add typeof string guard to editor bridge eval

Validate that factoryCode is a non-empty string before passing it to
(0, eval) in __omni_replaceComponent. This is defense-in-depth: the
primary protection is the postMessage origin check, but this guards
against same-origin XSS in the parent editor page.

Security review finding C6."
```

---

## Phase 3: Bug Fixes (MEDIUM Priority, High ROI)

### Task 7: Fix any(|_| true) defeating useEffect import optimization

**Files:**
- Modify: `omnilang/src/codegen/react/component/effect.rs:307`

**Why:** `view_has_motion_or_effects` uses `any(|_| true)` which always returns true for any trigger type. The comment explicitly says `@timeline` and `@animate` triggers do NOT generate useEffect, but the code doesn't filter them. Components using only `@timeline` animations unnecessarily import `useEffect`.

- [ ] **Step 1: Read the TriggerKind enum to confirm variant names**

Run: `grep -n "pub enum TriggerKind" crates/omnilang-core/src/ir/types_motion.rs`
Expected: Find the enum definition. We need the exact variant names for `Timeline` and `Animate`.

- [ ] **Step 2: Add the trigger kind import**

At the top of `effect.rs`, check if `TriggerKind` is already imported. If not, add:

```rust
use omnilang_core::ir::TriggerKind;
```

- [ ] **Step 3: Replace any(|_| true) with a trigger kind filter**

At line 307 of `effect.rs`:

```rust
// Current:
if motion.triggers.iter().any(|_| true) {
    return true;
}

// Fix:
// @timeline and @animate triggers do NOT generate useEffect (they're WAAPI-only)
if motion.triggers.iter().any(|t| {
    !matches!(t.kind, TriggerKind::Timeline | TriggerKind::Animate)
}) {
    return true;
}
```

- [ ] **Step 4: Run Rust compilation check**

Run: `cargo check -p omnilang --features codegen-react,codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 5: Run React motion tests**

Run: `cargo test -p omnilang --features codegen-react,codegen-omniview -- react::tests 2>&1`
Expected: All tests pass. Verify the `@timeline`-only snapshot no longer imports `useEffect`.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/component/effect.rs
git commit -m "fix(react): filter Timeline/Animate triggers in useEffect detection

Replace any(|_| true) with a proper trigger kind filter in
view_has_motion_or_effects. @timeline and @animate triggers are
WAAPI-only and do not generate useEffect hooks — the comment
documented this but the implementation always returned true.

Performance review finding: any(|_| true) defeated the import opt."
```

---

### Task 8: Fix assert! → debug_assert! doc mismatch in push_ref_nid

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:617`

**Why:** The doc comment says "Panics (debug_assert)" but the code uses `assert!` (which panics in release builds too). If the semantic validator has a bug, the compiler crashes in production instead of producing a user-facing diagnostic. The semantic validator already catches duplicates before codegen — this is defense-in-depth, not a primary check.

- [ ] **Step 1: Change assert! to debug_assert!**

At line 617 of `mod.rs`, the current code:

```rust
    /// Panics (debug_assert) on duplicates — the semantic validator
    /// should catch duplicates before codegen, but this is defense-in-depth.
    pub(super) fn push_ref_nid(&mut self, ref_name: String, nid: String) {
        assert!(
            !self.ref_nid_set.contains(&ref_name),
            "duplicate ref binding '{}' — semantic validator should have caught this",
            ref_name
        );
```

Change `assert!` to `debug_assert!` to match the doc comment:

```rust
    /// Panics (debug_assert) on duplicates — the semantic validator
    /// should catch duplicates before codegen, but this is defense-in-depth.
    pub(super) fn push_ref_nid(&mut self, ref_name: String, nid: String) {
        debug_assert!(
            !self.ref_nid_set.contains(&ref_name),
            "duplicate ref binding '{}' — semantic validator should have caught this",
            ref_name
        );
```

- [ ] **Step 2: Run Rust compilation check**

Run: `cargo check -p omnilang --features codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 3: Run OmniView tests**

Run: `cargo test -p omnilang --features codegen-omniview 2>&1`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "fix(omniview): use debug_assert! in push_ref_nid to match doc

The doc comment says 'Panics (debug_assert)' but the code used
assert! which panics in release builds. The semantic validator
catches duplicates before codegen — this is defense-in-depth, not
a primary check, so debug_assert! is the correct level.

Rust review finding C1."
```

---

## Phase 4: Infrastructure & Cleanup (LOW Priority)

### Task 9: Update deferrals.md with active Phase 2 items

**Files:**
- Modify: `.architecture/deferrals.md:69` (replace "无" section)

**Why:** `deferrals.md` says "no active deferrals" but ~10 active Phase 2 items are scattered across ADR-036, ADR-038, ADR-021, ADR-037, and session memory files. The Implementation Strategist and AI Engineer both flagged this as a tracking gap.

- [ ] **Step 1: Replace the "当前活跃推迟项" section**

Replace the "无" at line 69 with the actual deferred items list:

```markdown
## 当前活跃推迟项

以下推迟项来自 ADR-036、ADR-038、ADR-021、ADR-037 及其他来源：

### 来自 ADR-036/038 (Element Reference System)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 1 | `@ref` inside `@for` loop bodies (Phase 2) | User demand for per-item refs | Currently emits compiler warning; unsupported |
| 2 | Flutter `GlobalKey` codegen for `@ref` | Flutter P0 commitment | All Flutter test fixtures pass `ref_binding: None` |
| 3 | Custom component ref forwarding (`forwardRef`) | Cross-component ref patterns | Not yet designed |
| 4 | `ref_names` threading through React store codegen | User reports store + ref combination bugs | 16 call sites pass `&FxHashSet::default()` |
| 5 | `element_tag` population from ElementIR tag | Non-div elements with refs | Currently uses tag→type mapping (ADR-038 D3) |

### 来自 ADR-037 (Signal Batching)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 6 | Signal batching with microtask deferral | Profiling gate: real app, measurable frame drops, bottleneck confirmed | Synchronous notification on every set() |

### 来自 ADR-021 (Cross-Target Feature Parity)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 7 | Cross-target regression test matrix | New backend added or paused backend reactivated | 4 confirmed silent-drop bugs fixed; no automated guard |

### 来自 ADR-001 (Pipeline Architecture)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 8 | TypeKind recursive validation for `Named` types | Language adds generic structs with custom type params | Unvalidated type names pass through IR silently |
| 9 | Source map pipeline completion (non-OmniView backends) | React/Flutter debugging needs | target_spans empty for React/Flutter |
| 10 | Codegen trait full adoption (OmniView) | Next major IR refactoring | OmniView bypasses standard compile() path |
| 11 | IR `#[non_exhaustive]` migration | Next ExprIR/StatementIR variant addition | Cascading compile errors across 10 backends |

### 来自 ADR-032 (Animation Scoping) — Parked

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 12 | Dead @focus JS codegen in React backend | React focus animation feature | mod.rs:127-144 unused code |
| 13 | Removed trigger tokens still in lexer | Lexer cleanup pass | token.rs, keywords.rs dead tokens |
| 14 | Stale doc comments referencing removed features | Doc audit | 5+ files with outdated animation docs |

### 来自 ADR-034/035 (v4 Review Structural Fixes)

| # | 项目 | 触发条件 | 影响 |
|---|------|---------|------|
| 15 | OmniViewGenerator decomposition into phase structs | Next major OmniView feature | 47-field struct with coupled phases |
| 16 | Complete React FormatExpr migration (remove direct variant handlers) | Next ExprIR variant addition | 3 variants handled directly, rest delegated to trait |
```

- [ ] **Step 2: Commit**

```bash
git add .architecture/deferrals.md
git commit -m "docs: update deferrals.md with 16 active Phase 2 items

Replace 'no active deferrals' with actual list sourced from
ADR-036/038, ADR-037, ADR-021, ADR-001, ADR-032, and ADR-034/035.
Each item includes trigger condition and impact if never addressed.

Implementation Strategist + AI Engineer review findings."
```

---

### Task 10: Remove dead UiCodegen trait (zero consumers)

**Files:**
- Delete content from: `crates/omnilang-core/src/codegen/shared/ui_codegen.rs`
- Modify: `crates/omnilang-core/src/codegen/shared/mod.rs` (remove re-export)
- Modify: `omnilang/src/codegen/react/mod.rs:864-888` (remove impl block)
- Modify: `omnilang/src/codegen/vue/mod.rs:365-380` (remove impl block)

**Why:** The `UiCodegen` trait has zero polymorphic consumers — confirmed by codebase-wide grep (`dyn UiCodegen`, `.generate_component_code(`). It adds ~103 lines of dead code (definition + 2 impl blocks). Concrete methods remain untouched.

- [ ] **Step 1: Remove the React UiCodegen impl block**

In `omnilang/src/codegen/react/mod.rs`, find and remove lines ~864-888:
```rust
impl UiCodegen for ReactCodegen {
    fn generate_component_code(&self, comp: &ComponentIR, ...) -> String { ... }
    fn generate_store_code(&self, store: &StoreIR, ...) -> String { ... }
}
```

- [ ] **Step 2: Remove the Vue UiCodegen impl block**

In `omnilang/src/codegen/vue/mod.rs`, find and remove lines ~365-380:
```rust
impl UiCodegen for VueCodegen { ... }
```

- [ ] **Step 3: Remove the trait definition**

In `crates/omnilang-core/src/codegen/shared/ui_codegen.rs`, replace the entire file content with a comment:
```rust
// UiCodegen trait removed 2026-07-30 — zero polymorphic consumers.
// Concrete methods on each backend remain unchanged.
// If a polymorphic consumer materializes, the trait can be re-extracted.
```

- [ ] **Step 4: Remove re-export from shared/mod.rs**

In `crates/omnilang-core/src/codegen/shared/mod.rs`, remove the line:
```rust
pub mod ui_codegen;
```
And any `pub use ui_codegen::UiCodegen;` re-export.

- [ ] **Step 5: Remove UiCodegen imports from React and Vue mod.rs**

Check for and remove `use omnilang_core::codegen::shared::UiCodegen;` from both files.

- [ ] **Step 6: Check for any other references**

Run: `grep -rn "UiCodegen" omnilang/src/ crates/ --include="*.rs" 2>&1`
Expected: No results (all references removed)

- [ ] **Step 7: Run compilation check**

Run: `cargo check -p omnilang --features codegen-react,codegen-vue,codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 8: Run full test suite**

Run: `cargo test -p omnilang --features codegen-react,codegen-vue,codegen-omniview 2>&1`
Expected: All tests pass

- [ ] **Step 9: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/ui_codegen.rs
git add crates/omnilang-core/src/codegen/shared/mod.rs
git add omnilang/src/codegen/react/mod.rs
git add omnilang/src/codegen/vue/mod.rs
git commit -m "refactor: remove dead UiCodegen trait (zero consumers)

The UiCodegen trait had zero polymorphic call sites — confirmed by
codebase-wide grep for 'dyn UiCodegen' and '.generate_component_code('.
The concrete methods on ReactCodegen and VueCodegen remain unchanged.
If a polymorphic consumer materializes, the trait can be re-extracted
from the existing impl methods.

Pragmatic Enforcer review finding: Necessity 0/10, ~103 lines dead."
```

---

### Task 11: Gate typescript.rs behind #[cfg(test)]

**Files:**
- Modify: `omnilang/src/codegen/omniview/typescript.rs:1` (add cfg gate)
- Modify: `omnilang/src/codegen/omniview/mod.rs:1-20` (gate the mod declaration)

**Why:** `typescript.rs` (351 lines) has an explicit "zero production consumers" comment (ADR-031 D6) but is compiled unconditionally in every OmniView build. The production call is commented out at mod.rs:393-395.

- [ ] **Step 1: Gate the mod declaration in mod.rs**

In `omnilang/src/codegen/omniview/mod.rs`, find the `mod typescript;` declaration and change it to:

```rust
#[cfg(test)]
mod typescript;
```

- [ ] **Step 2: Gate the production call site**

The call site at mod.rs:390-396 is already commented out. Verify:

```rust
// Skip TypeScript declaration generation — zero production consumers (ADR-031 D6).
// Re-enable when TypeScript type generation is needed for Poiesis or external consumers.
// let types = self.emit_typescript_declarations();
let types = String::new();
```

This is fine — no change needed here since the module itself is now gated.

- [ ] **Step 3: Add #[cfg(test)] to the typescript.rs file**

At the top of `omnilang/src/codegen/omniview/typescript.rs`, verify the existing test module at the bottom of the file (around line 223) uses `#[cfg(test)]`. The tests inside the file don't need changes since the whole file is now test-only.

- [ ] **Step 4: Run compilation check**

Run: `cargo check -p omnilang --features codegen-omniview 2>&1`
Expected: Compilation succeeds (typescript.rs not compiled in non-test builds)

- [ ] **Step 5: Run tests to verify test-only code still compiles and passes**

Run: `cargo test -p omnilang --features codegen-omniview 2>&1`
Expected: All tests pass, including typescript tests

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "refactor(omniview): gate typescript.rs behind #[cfg(test)]

typescript.rs (351 lines) has zero production consumers since
ADR-031 D6. Gate it behind #[cfg(test)] to avoid compiling it
in release builds. The production call site was already commented
out at mod.rs:393-395.

Pragmatic Enforcer review finding: Necessity 1/10."
```

---

### Task 12: Derive Default for OmniViewGenerator and ComponentScope

**Files:**
- Modify: `omnilang/src/codegen/omniview/scope.rs:21` (add Default derive)
- Modify: `omnilang/src/codegen/omniview/mod.rs:439` (add Default derive)
- Modify: `omnilang/src/codegen/omniview/mod.rs:548-591` (replace test_new with default)

**Why:** `test_new()` (43 lines) manually initializes all 33 fields with default values. `#[derive(Default)]` is behaviorally equivalent and eliminates the maintenance duplicate — every new field must be added to both `test_new()` and the `generate()` constructor.

- [ ] **Step 1: Add Default derive to ComponentScope**

In `omnilang/src/codegen/omniview/scope.rs`, line 21, the struct:

```rust
// Current:
pub(super) struct ComponentScope {

// Fix:
#[derive(Default)]
pub(super) struct ComponentScope {
```

- [ ] **Step 2: Add Default derive to OmniViewGenerator**

In `omnilang/src/codegen/omniview/mod.rs`, line 439, the struct:

```rust
// Current:
pub(super) struct OmniViewGenerator {

// Fix:
#[derive(Default)]
pub(super) struct OmniViewGenerator {
```

- [ ] **Step 3: Replace test_new() with OmniViewGenerator::default()**

In `omnilang/src/codegen/omniview/mod.rs`, find all call sites of `test_new()` (45+ test sites). Replace each:

```rust
// Current:
let mut gen = OmniViewGenerator::test_new();

// Fix:
let mut gen = OmniViewGenerator::default();
```

- [ ] **Step 4: Remove the test_new() function body**

Replace the `test_new()` function (lines 548-591) with:

```rust
// test_new() removed 2026-07-30 — use OmniViewGenerator::default() instead.
// Derive(Default) provides the same zero-initialization without the
// 43-line maintenance duplicate that required updating on every field change.
```

Or remove it entirely if no remaining callers.

- [ ] **Step 5: Run compilation check**

Run: `cargo check -p omnilang --features codegen-omniview 2>&1`
Expected: Compilation succeeds

- [ ] **Step 6: Run tests to verify Default produces correct test state**

Run: `cargo test -p omnilang --features codegen-omniview 2>&1`
Expected: All tests pass with `::default()` instead of `::test_new()`

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/omniview/scope.rs
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "refactor(omniview): derive Default for OmniViewGenerator and ComponentScope

Replace the 43-line test_new() manual initialization with
#[derive(Default)]. Every new field required updating test_new()
in addition to the generate() constructor — Default eliminates
this maintenance duplicate.

Pragmatic Enforcer review finding."
```

---

### Task 13: Add CSP warning to wrap_debug_html output

**Files:**
- Modify: `omnilang/src/codegen/omniview/utils.rs:191-224`

**Why:** `wrap_debug_html` omits CSP protection (documented in ADR-028 D6). If someone copies the debug HTML output to a production deployment, the page has no Content-Security-Policy. A console.warn in the generated output alerts developers.

- [ ] **Step 1: Add console.warn to the generated HTML**

In `omnilang/src/codegen/omniview/utils.rs`, find the `wrap_debug_html` function (around line 191). In the generated `<script>` block, add a console.warn at the beginning:

The function generates HTML with embedded script tags. Add to the first `<script>` block:

```javascript
console.warn(
  '%cOmniView Debug Mode%c\n' +
  'This page was generated in DEBUG mode and has NO Content-Security-Policy.\n' +
  'Do NOT deploy this HTML to production. Use the React production backend instead.',
  'font-weight: bold; font-size: 14px;', ''
);
```

Locate the exact emission point in the function and add the console.warn after the opening `<script>` tag.

- [ ] **Step 2: Run OmniView tests to verify snapshot changes**

Run: `cargo test -p omnilang --features codegen-omniview 2>&1`
Expected: Some snapshot tests may need updating (the console.warn will appear in debug output)

- [ ] **Step 3: Update snapshots if needed**

Run: `cargo test -p omnilang --features codegen-omniview -- omniview::tests 2>&1`
If snapshots fail, review the diff to confirm only the console.warn was added, then update:

Run: `cargo insta review` or set `INSTA_UPDATE=always` for the test run.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/utils.rs
git add omnilang/src/codegen/omniview/snapshots/  # if snapshots changed
git commit -m "fix(omniview): add CSP warning to wrap_debug_html output

Emit a console.warn in debug-mode HTML output alerting developers
that the page has no Content-Security-Policy and should not be
deployed to production. The production path (build_file_output →
html_document_wrapper) correctly includes CSP.

Security review finding C5."
```

---

## Final Verification

After all tasks are complete:

- [ ] **Run full test suite with all backends**

```bash
cargo test -p omnilang --features codegen-react,codegen-omniview,codegen-flutter 2>&1
```
Expected: All tests pass

- [ ] **Run clippy**

```bash
cargo clippy -p omnilang --features codegen-react,codegen-omniview -- -D warnings 2>&1
```
Expected: No warnings

- [ ] **Run cargo fmt**

```bash
cargo fmt -p omnilang --check 2>&1
```
Expected: All files formatted

- [ ] **Review git diff**

```bash
git diff --stat HEAD~13..HEAD
git log --oneline -13
```

---

## Self-Review Checklist

- [x] **Spec coverage**: All 6 HIGH/CRITICAL findings + 5 infrastructure items are covered
- [x] **Placeholder scan**: No TBD, TODO, "implement later", or "add appropriate error handling"
- [x] **Type consistency**: `ref_names: &FxHashSet<String>` used consistently across all tasks
- [x] **File paths**: All paths are absolute from workspace root
- [x] **Code shown**: Every code change includes the actual code to write
- [x] **Commands shown**: Every test step includes the exact command and expected output

---

*Plan generated 2026-07-30 from the 11-specialist architecture review at `.architecture/reviews/component-omniview-2026-07-30.md`*
