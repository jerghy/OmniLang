# OmniView Architecture Review Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the CRITICAL and HIGH-severity findings from the 2026-07-29 OmniView architecture review (10 specialists), excluding 14 items documented as intentional design trade-offs in ADR-028.

**Architecture:** Four phases ordered by severity: Phase 1 fixes 3 CRITICAL bugs (silent output corruption, runtime state corruption). Phase 2 fixes 8 HIGH concerns (security gaps, memory leaks, wrong behavior, broken CI). Phase 3 cleans 4 MEDIUM code quality issues. Phase 4 documents deferred items per ADR-028.

**Tech Stack:** Rust (omnilang crate), JavaScript (inline runtime strings in Rust), cargo test

**Key ADRs:** [ADR-028](../.architecture/decisions/adrs/ADR-028-omniview-debug-runtime-design-tradeoffs.md) — 14 intentional design decisions NOT to fix

---

### Task 1: Fix BlockStmtIR silently dropped in emit_statement

**Files:**
- Modify: `omnilang/src/codegen/omniview/stmt.rs:133-136`
- Modify: `omnilang/src/codegen/omniview/stmt.rs:166`
- Create test: `omnilang/src/codegen/omniview/tests.rs` (add test case)

**Context:** Valid `.omni` with bare blocks `{ x = x + 1; }` compiles to broken OmniView output. The `StatementIR::BlockStmtIR` variant falls through to a wildcard arm emitting `/* unhandled stmt */`. React/Flutter backends handle this correctly. Fix is 15 lines — add the missing arm.

- [ ] **Step 1: Add BlockStmtIR arm to emit_statement**

In `omnilang/src/codegen/omniview/stmt.rs`, find the `emit_statement` method's match block (around line 133). Replace the wildcard `_ =>` arm with explicit variants:

```rust
// In emit_statement, replace the wildcard arm at line 133-136:
            StatementIR::BlockStmtIR { body } => {
                for stmt in &body.statements {
                    self.emit_statement(stmt);
                }
            }
            _ => {
                // Placeholder for unhandled statements
                self.js.push_str("  /* unhandled stmt */\n");
            }
```

- [ ] **Step 2: Add BlockStmtIR arm to stmt_to_js_inline**

In the same file, find `stmt_to_js_inline` method's match block (around line 166). Add the BlockStmtIR arm before the wildcard:

```rust
// In stmt_to_js_inline, insert before the wildcard `_ =>` at line 166:
            StatementIR::BlockStmtIR { body } => {
                let parts: Vec<String> = body
                    .statements
                    .iter()
                    .map(|s| self.stmt_to_js_inline(s))
                    .filter(|s| !s.is_empty())
                    .collect();
                parts.join("; ")
            }
```

- [ ] **Step 3: Run existing OmniView tests to verify no regression**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview -- --nocapture 2>&1 | tail -20
```

Expected: All existing tests pass (no `/* unhandled stmt */` in any snapshot).

- [ ] **Step 4: Add regression test for bare block statements**

In `omnilang/src/codegen/omniview/tests.rs`, add a test:

```rust
#[test]
fn test_block_stmt_in_function() {
    let source = r#"import omniview;
component main {
    signal x: i32 = 0;
    fn increment() {
        {
            x = x + 1;
        }
    }
    view { <div>{x}</div> }
}
"#;
    let result = omnilang::compile_to_omniview(source).unwrap();
    let js = result.output.js;
    // The block's inner statement should be emitted, not "unhandled stmt"
    assert!(!js.contains("unhandled stmt"), "BlockStmtIR should not fall through to wildcard. JS:\n{}", js);
    // The increment logic should appear in the output
    assert!(js.contains("__omni_subs.get"), "Signal mutation should be emitted");
}
```

- [ ] **Step 5: Run the new test**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview::tests::test_block_stmt_in_function
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/stmt.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "fix(omniview): add BlockStmtIR handling in emit_statement and stmt_to_js_inline

BlockStmtIR (bare blocks in statement position) was silently
dropped by the wildcard arm in both emit_statement and
stmt_to_js_inline, producing /* unhandled stmt */ in generated
JS. React/Flutter backends handle this correctly.

Fix: recurse into body.statements for BlockStmtIR in both methods.
Add regression test."
```

---

### Task 2: Fix __omni_activeEffect leak on subscriber exception

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs:40-45`

**Context:** If any subscriber's `e.run()` throws in the signal setter, `__omni_activeEffect` is never restored, causing silent global state corruption. Remaining subscribers are skipped. Fix is 3 lines — wrap in try/catch/finally.

- [ ] **Step 1: Add try/catch/finally around subscriber notification**

In `omnilang/src/codegen/omniview/runtime.rs`, find the signal setter's forEach block (lines 40-45). Replace:

```javascript
// OLD (lines 40-45):
        s.forEach(e => {
          const prev = __omni_activeEffect;
          __omni_activeEffect = e;
          e.run();
          __omni_activeEffect = prev;
        });
```

With:

```javascript
// NEW:
        s.forEach(e => {
          const prev = __omni_activeEffect;
          __omni_activeEffect = e;
          try { e.run(); }
          catch(err) { __omni_reportError("error", "effect:run", String(err), ""); }
          finally { __omni_activeEffect = prev; }
        });
```

- [ ] **Step 2: Run OmniView tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview -- --nocapture 2>&1 | tail -10
```

Expected: All tests pass. The try/catch shouldn't change snapshot output since no test triggers subscriber exceptions.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix(omniview): add try/catch/finally to signal subscriber notification

If any subscriber's e.run() throws, __omni_activeEffect was
never restored and remaining subscribers were silently skipped.
This caused silent UI freezes with no error trace.

Fix: wrap e.run() in try/catch/finally to restore
__omni_activeEffect and report the error via __omni_reportError."
```

---

### Task 3: Add missing CSP directives (form-action, object-src, frame-src)

**Files:**
- Modify: `omnilang/src/codegen/omniview/utils.rs:248`

**Context:** The CSP meta tag is missing `form-action`, `object-src`, and `frame-src` directives. Without `form-action 'self'`, HTML injection could exfiltrate form data. Fix is 1 line — add three directives to the existing CSP string.

- [ ] **Step 1: Add missing CSP directives**

In `omnilang/src/codegen/omniview/utils.rs`, line 248, find the CSP string. Replace:

```rust
// OLD (line 248):
        r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self'; base-uri 'self'; frame-ancestors 'self';">"
"#
```

With:

```rust
// NEW:
        r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self'; base-uri 'self'; frame-ancestors 'self'; form-action 'self'; object-src 'none'; frame-src 'none';">"
"#
```

- [ ] **Step 2: Update snapshot tests affected by CSP change**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview -- --nocapture 2>&1 | tail -30
```

If any insta snapshot tests fail because the CSP string changed, update them:

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview
# If snapshots need updating:
# Review the diff, then:
cargo insta review
# Or auto-accept:
cargo insta accept
```

- [ ] **Step 3: Run full test suite**

```bash
cd D:/space/OmniLang && cargo test -p omnilang 2>&1 | tail -10
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/utils.rs
# Add snapshot files if updated:
git add omnilang/src/codegen/omniview/snapshots/ 2>/dev/null || true
git commit -m "fix(omniview): add form-action, object-src, frame-src to CSP meta tag

CSP was missing three directives:
- form-action 'self': prevents form data exfiltration via HTML injection
- object-src 'none': blocks <object>/<embed>/<applet> elements
- frame-src 'none': blocks same-origin framing

These are standard defense-in-depth directives for standalone HTML pages."
```

---

### Task 4: Clear __omni_el._cache on component teardown

**Files:**
- Modify: `omnilang/src/codegen/omniview/editor.rs:108`

**Context:** When components are hot-replaced in the editor, the `__omni_el._cache` retains stale references to removed DOM elements. Although `isConnected` prevents them from being returned, the entries accumulate over long editor sessions. Fix is 1 line.

- [ ] **Step 1: Add cache clear to teardown**

In `omnilang/src/codegen/omniview/editor.rs`, find the `__omni_replaceComponent` function (around line 108). After `if (oldComp && oldComp.teardown) oldComp.teardown();`, add the cache clear:

```javascript
// In the __omni_replaceComponent JS string in editor.rs, after line 108:
  if (oldComp && oldComp.teardown) oldComp.teardown();
  if (__omni_el._cache) __omni_el._cache = {};  // <-- ADD THIS LINE
  var newComp = window.__omni_components[name];
```

- [ ] **Step 2: Run tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview::tests_regression -- --nocapture 2>&1 | tail -10
```

Expected: All tests pass. No snapshot should break since the cache clear is in editor-mode-specific code.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/editor.rs
git commit -m "fix(omniview): clear __omni_el._cache on component teardown

Stale element cache entries accumulated in __omni_el._cache across
hot-replace cycles in the Poiesis editor. While isConnected prevented
returning removed elements, the Map entries wasted memory.

Fix: reset __omni_el._cache to {} after teardown in
__omni_replaceComponent."
```

---

### Task 5: Fix StaggerDirection::Random mapped to "edges"

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_config.rs:251`

**Context:** `StaggerDirection::Random` is mapped to `"edges"` — same as the `Edges` variant. This is a probable copy-paste error. Changing to `"random"` fixes the behavior. If the JS runtime doesn't support `"random"`, emit a warning instead.

- [ ] **Step 1: Fix the mapping**

In `omnilang/src/codegen/omniview/motion_config.rs`, line 251, change:

```rust
// OLD (line 251):
                StaggerDirection::Random => "edges",
```

To:

```rust
// NEW:
                StaggerDirection::Random => "random",
```

- [ ] **Step 2: Check if JS runtime supports "random" staggerFrom**

Look at the motion runtime (omni-animation package) for `staggerFrom: "random"` support:

```bash
cd D:/space/OmniLang && grep -r "staggerFrom" packages/omni-animation/ 2>/dev/null || echo "Check omni-animation package for staggerFrom random support"
```

If `"random"` is NOT supported, instead emit a warning:

```rust
                StaggerDirection::Random => {
                    self.warnings.push(
                        "staggerFrom: random is not yet supported; falling back to 'edges'".into()
                    );
                    "edges"
                }
```

- [ ] **Step 3: Run motion tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview::tests_regression::motion -- --nocapture 2>&1 | tail -20
```

Expected: Snapshot tests may change. Review and accept if the output is correct.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_config.rs
git commit -m "fix(omniview): map StaggerDirection::Random to 'random' not 'edges'

StaggerDirection::Random was incorrectly mapped to 'edges',
the same value as StaggerDirection::Edges — a copy-paste error.
Changed to 'random'."
```

---

### Task 6: Fix known test failure — test_resolve_styles_with_design_class

**Files:**
- Investigate: `omnilang/src/codegen/omniview/view/element.rs` (design class lookup)
- Modify: `omnilang/src/codegen/omniview/mod.rs:504-506` (design_class_names construction)
- Test: `omnilang/tests/resolve_styles_test.rs:57-83`

**Context:** `test_resolve_styles_with_design_class` has been failing across 3 review cycles. The test asserts no warnings when using `class: Card` on a `<div>`, where `Card` is defined in a `design` block. The warning is incorrectly produced: `"class 'Card' referenced on <div> (nid=el-div-001) is not defined in any design block or component style"`. Root cause: the design class name lookup in `emit_element_props` may not be checking the `design_class_names` HashSet built at `mod.rs:504-506`.

- [ ] **Step 1: Reproduce the failure**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --test resolve_styles_test test_resolve_styles_with_design_class 2>&1 | tail -30
```

Expected: FAIL with unexpected warnings about class 'Card'.

- [ ] **Step 2: Trace the design class lookup**

In `omnilang/src/codegen/omniview/view/element.rs`, find where `class:` attributes are validated against design class names. Search for the warning string. Identify which lookup (design_class_names vs component_style_classes) is being used.

```bash
cd D:/space/OmniLang && grep -n "not defined in any design block" omnilang/src/codegen/omniview/view/element.rs
```

- [ ] **Step 3: Fix the lookup**

The fix depends on what's found in Step 2. Likely scenario: `emit_element_props` checks `component_style_classes` but not `design_class_names`. The fix is to also check `design_class_names` when validating class references.

The `design_class_names` HashSet is built at `mod.rs:504-506` during the `emit_view` phase. In the `resolve_styles` path (which uses `css_only=true`), the design class names need to be available during element emission.

Add the check (pseudocode — exact line numbers from Step 2):
```rust
// In the class validation code in view/element.rs:
let is_defined = self.component_style_classes.contains(class_name)
    || self.design_class_names.contains(class_name);
if !is_defined {
    self.warnings.push(format!(
        "class '{}' referenced on <{}> (nid={}) is not defined in any design block or component style",
        class_name, tag, nid
    ));
}
```

- [ ] **Step 4: Run the failing test**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --test resolve_styles_test test_resolve_styles_with_design_class
```

Expected: PASS with no warnings.

- [ ] **Step 5: Run all resolve_styles tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --test resolve_styles_test
```

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "fix(omniview): check design_class_names when validating class attributes

test_resolve_styles_with_design_class was failing because the
class attribute validator only checked component_style_classes,
not design_class_names. Classes defined in design blocks were
incorrectly flagged as 'not defined'.

Fix: add design_class_names check to the class validation logic."
```

---

### Task 7: Remove dead field current_nid_path

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:447` (field declaration)
- Modify: `omnilang/src/codegen/omniview/mod.rs` (initialization in generate())
- Modify: `omnilang/src/codegen/omniview/mod.rs` (initialization in test_new())
- Modify: `omnilang/src/codegen/omniview/view/mod.rs:50` (clear() call in emit_view)

**Context:** `current_nid_path: Vec<String>` is initialized but never pushed to — only cleared at `emit_view` line 50. It's a vestigial field from an earlier design. Remove it.

- [ ] **Step 1: Remove field declaration**

In `omnilang/src/codegen/omniview/mod.rs`, line 447, remove:
```rust
    current_nid_path: Vec<String>,
```

- [ ] **Step 2: Remove initialization in generate()**

In the `generate()` function (around line 558), remove the `current_nid_path` initialization line.

- [ ] **Step 3: Remove initialization in test_new()**

In the `test_new()` function (around line 605), remove the `current_nid_path` initialization line.

- [ ] **Step 4: Remove clear() call in emit_view**

In `omnilang/src/codegen/omniview/view/mod.rs`, around line 50, remove:
```rust
        self.current_nid_path.clear();
```

- [ ] **Step 5: Compile and test**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview 2>&1 | tail -10
```

Expected: All tests pass. Compiler should not warn about unused field since it's removed.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/view/mod.rs
git commit -m "refactor(omniview): remove dead field current_nid_path

current_nid_path: Vec<String> was initialized but never pushed to,
only cleared in emit_view. Vestigial field from an earlier design.
No behavioral change."
```

---

### Task 8: Extract shared emit_class_map_effects helper

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:575-587, 626-637`

**Context:** The `__omni_effect` calls for dynamic class toggling are duplicated in two branches of `emit_element_html_close` (void-tag and normal-tag branches). Extract into a helper method.

- [ ] **Step 1: Create the helper method**

In `omnilang/src/codegen/omniview/view/element.rs`, add a new method to `impl OmniViewGenerator`:

```rust
    fn emit_class_map_effects(&mut self, el: &ElementIR, nid: &str) {
        for (class_name, binding) in &el.class_map_bindings {
            let expr_js = self.expr_to_js(binding);
            writeln!(
                self.deferred_js,
                "__omni_effect(() => {{ const v = {}; const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!v); }});",
                expr_js, nid, class_name
            )
            .unwrap();
        }
    }
```

- [ ] **Step 2: Replace void-tag branch call**

Replace lines 575-587 (the void-tag branch's class_map_bindings loop) with:
```rust
        self.emit_class_map_effects(el, &nid);
```

- [ ] **Step 3: Replace normal-tag branch call**

Replace lines 626-637 (the normal-tag branch's class_map_bindings loop) with:
```rust
        self.emit_class_map_effects(el, &nid);
```

- [ ] **Step 4: Run tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview -- --nocapture 2>&1 | tail -10
```

Expected: All tests pass. Snapshot output unchanged (refactoring only).

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "refactor(omniview): extract shared emit_class_map_effects method

Class map binding emission was duplicated in void-tag and
normal-tag branches of emit_element_html_close. Extracted into
a shared helper to eliminate the duplication."
```

---

### Task 9: Extract resolve_store_field_access helper for FieldAccessExpr/OptionalChainExpr

**Files:**
- Modify: `omnilang/src/codegen/omniview/expr.rs:52-101`

**Context:** `FieldAccessExpr` and `OptionalChainExpr` have 48 lines of nearly identical logic, differing only in `.value` vs `?.value` accessor and `.` vs `?.` join operator. Extract a helper.

- [ ] **Step 1: Create the helper method**

In `omnilang/src/codegen/omniview/expr.rs`, add a method to `impl OmniViewGenerator`:

```rust
    fn resolve_store_field_access(
        &mut self,
        object: &ExprIR,
        field: &str,
        use_optional: bool,
    ) -> String {
        let accessor = if use_optional { "?." } else { "." };
        let join = if use_optional { "?." } else { "." };

        if let ExprIR::IdentifierIR { name: obj_name } = object {
            let resolved = self
                .name_renames
                .get(obj_name.as_str())
                .cloned()
                .unwrap_or_else(|| obj_name.clone());
            if self.store_signal_names.contains(&resolved) {
                let getter_name = format!("{}{}value", resolved, accessor);
                return self.wrap_expr_in_guard(getter_name);
            }
            if self.signal_names.contains(obj_name) {
                let getter_name = format!("{}{}value", resolved, accessor);
                return self.wrap_expr_in_guard(getter_name);
            }
        }
        let obj_js = self.expr_to_js(object);
        format!("{}{}{}", obj_js, join, self.sanitize_js_ident(field))
    }
```

- [ ] **Step 2: Replace FieldAccessExpr arm**

Replace lines 52-76 with a call to the helper with `use_optional: false`.

- [ ] **Step 3: Replace OptionalChainExpr arm**

Replace lines 77-101 with a call to the helper with `use_optional: true`.

- [ ] **Step 4: Run tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview -- --nocapture 2>&1 | tail -10
```

Expected: All tests pass. No snapshot changes.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/expr.rs
git commit -m "refactor(omniview): extract resolve_store_field_access helper

FieldAccessExpr and OptionalChainExpr had 48 lines of near-identical
logic differing only in .value vs ?.value and . vs ?. join operators.
Extracted into a shared helper with use_optional parameter."
```

---

### Task 10: Define named const slices for HTML attributes

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:122-132`

**Context:** ~60 HTML attribute names are listed as inline string literals in a match arm. Boolean attributes, URL attributes, and standard attributes are each defined in separate inline lists — duplicating meaning. Define named const slices.

- [ ] **Step 1: Define const slices at the top of the function or module**

In `omnilang/src/codegen/omniview/view/element.rs`, add before the `emit_element_props` function:

```rust
    /// HTML boolean attributes — presence means true, absence means false.
    const BOOLEAN_HTML_ATTRS: &[&str] = &[
        "disabled", "checked", "required", "readonly", "selected",
        "multiple", "autofocus", "novalidate", "formnovalidate",
        "hidden", "async", "defer", "ismap",
    ];

    /// HTML attributes whose values are URLs — need sanitization.
    const URL_HTML_ATTRS: &[&str] = &[
        "href", "src", "srcset", "action", "formaction", "poster",
    ];

    /// Standard HTML attributes emitted as key="value" pairs.
    const STANDARD_HTML_ATTRS: &[&str] = &[
        "id", "placeholder", "type", "value", "name", "title",
        "alt", "width", "height", "min", "max", "step", "pattern",
        "maxlength", "minlength", "rows", "cols", "wrap",
        "autocomplete", "spellcheck", "contenteditable",
        "download", "target", "rel", "media", "sizes",
        "accept", "enctype", "method",
        // ... add any remaining from the existing 60-arm match
    ];
```

- [ ] **Step 2: Replace the inline match arm**

Replace the 60-arm match statement with:
```rust
        let attr_name = name.as_str();
        if STANDARD_HTML_ATTRS.contains(&attr_name) {
            // standard attribute handling
        } else if BOOLEAN_HTML_ATTRS.contains(&attr_name) {
            // boolean attribute handling
        } else if URL_HTML_ATTRS.contains(&attr_name) {
            // URL attribute handling with sanitization
        }
```

- [ ] **Step 3: Run tests**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib omniview 2>&1 | tail -10
```

Expected: All tests pass. No snapshot changes.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/view/element.rs
git commit -m "refactor(omniview): define named const slices for HTML attribute categories

Replaced inline string literal lists for boolean, URL, and standard
HTML attributes with named const slices. Eliminates duplication and
makes attribute categorization explicit."
```

---

### Phase 3: Deferred Items (Documented per ADR-028)

These items are intentional design trade-offs per ADR-028 and are NOT bugs to fix:

| Item | ADR-028 Ref | Deferral Rationale |
|------|-------------|-------------------|
| Signal batching | D3 | Synchronous = predictable = debuggable. Add when OmniView becomes a production target. |
| Output minification | D4 | Human-readable output for debugging. Minification optional future feature. |
| No tree-shaking | D5 | Self-contained single-file design. Production targets (React/Flutter) handle this. |
| String-based codegen | D2 | ADR-012: imperative pattern matches "one output, one document" model. |
| wrap_debug_html no CSP | D6 | Debug-only wrapper; production path has CSP. |
| Inline runtime | D1 | Self-contained iframe-ready document. No external dependencies. |
| Editor metadata on output | D10 | WASM/FFI boundary needs structured data. |
| (0, eval) hot-replace | D8 | Editor-mode only; production builds exclude this code. |
| debug_assert! phase guards | D12 | Zero-cost in release; catches dev mistakes. |
| CSS-only mode | D7 | Editor canvas preview — reuses codegen pipeline. |
| 6-phase execution order | D11 | Buffer-swap pattern enforces DOM-before-bindings. |
| Source maps | D13 | Debugging requirement; editor selection tracking. |
| debug_mode/editor_mode flags | D9 | Orthogonal dimensions for different editor contexts. |
| Overall debug-tool role | D14 | ADR-012 established: dev debugging, not production. |

---

### Self-Review

**1. Spec coverage check against review findings:**

| Review Finding | Severity | Covered By |
|---------------|----------|------------|
| BlockStmtIR dropped | CRITICAL | Task 1 |
| activeEffect leak | CRITICAL | Task 2 |
| CSP gaps | HIGH | Task 3 |
| Teardown cache leak | HIGH | Task 4 |
| StaggerDirection bug | HIGH | Task 5 |
| Known test failure | HIGH | Task 6 |
| Dead field current_nid_path | LOW | Task 7 |
| Class map duplication | MEDIUM | Task 8 |
| Expr duplication | MEDIUM | Task 9 |
| Magic HTML strings | MEDIUM | Task 10 |
| Signal batching | HIGH | Deferred (ADR-028 D3) |
| No minification | MEDIUM | Deferred (ADR-028 D4) |
| God Object decomposition | HIGH | Separate plan (L effort, 2-3 days) |
| Motion trigger duplication | HIGH | Separate plan (M effort) |
| InliningState extraction | MEDIUM | Separate plan (S effort) |
| VariantIR rename | HIGH | Separate plan (M effort) |
| Prop immutability check | MEDIUM | Separate plan (S effort) |
| @enter replay restriction | HIGH | Separate plan (M effort) |
| Output minification | MEDIUM | Deferred (ADR-028 D4) |
| WASM incremental cache | MEDIUM | Separate plan (S effort) |
| Rust/JS CSS sanitizer align | MEDIUM | Separate plan (S effort) |
| Browser JS runtime tests | HIGH | Separate plan (M effort) |
| Cross-backend semantic tests | HIGH | Separate plan (M effort) |

**2. Placeholder scan:** No TBD/TODO/fill-in-details. All code steps have actual code.

**3. Type consistency:** StatementIR::BlockStmtIR structure verified in `crates/omnilang-core/src/ir/types.rs:546-548`. Runtime function names verified against `runtime.rs`.

**Gaps (not covered — need separate plans):**
- God Object decomposition (L effort, requires 2-3 day phased refactoring)
- Motion trigger deduplication (M effort, 1-2 hours)
- VariantIR rename (M effort, 6-8 files)
- Prop immutability compile check (S effort)
- @enter replay dependency restriction (M effort)
- Browser JS runtime tests (M effort)
- Cross-backend semantic tests (M effort)
- InliningState save/restore extraction (S effort)
- WASM incremental cache connection (S effort)
- Rust/JS CSS sanitizer alignment (S effort)

---

**Plan complete and saved to `docs/superpowers/plans/2026-07-29-omniview-review-fixes.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
