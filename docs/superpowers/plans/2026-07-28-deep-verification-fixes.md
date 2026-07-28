# Deep Verification Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 8 confirmed real issues + 5 documentation/memory corrections from the 2026-07-28 deep verification v2.

**Architecture:** Eight independent Rust fixes (compiler + security), two CI fixes, and five documentation updates. All changes are S-effort (1-line to few-lines each). No behavioral change to correct programs — only bug fixes, dead code removal, and stale doc corrections. Confirm each fix with `cargo test --workspace` (expect 801+ tests, 0 failures) and `cargo clippy --workspace -- -D warnings`.

**Tech Stack:** Rust (omnilang-core, omnilang, omnilang-wasm, poiesis), Bash (CI/eval), Markdown (docs/ADR/memory)

---

### Task 1: Fix `lower_type` TypeKind Normalization

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/component.rs:393`

- [ ] **Step 1: Apply the 1-line fix**

Replace line 393 in `crates/omnilang-core/src/ir/lower/component.rs`:

```rust
// Before (line 393):
                kind: TypeKind::Named { name: name.clone() },

// After:
                kind: TypeKind::from(name.as_str()),
```

This uses the existing `From<&str>` impl at `ir/types.rs:298-311` which maps `"void"` → `TypeKind::Void`, `"int"` → `TypeKind::Int`, etc. All built-in type names are normalized instead of being wrapped in `TypeKind::Named`.

- [ ] **Step 2: Verify compilation and tests**

Run: `cargo test --workspace`
Expected: 801+ tests, 0 failures.

The `missing_return` lint (`semantic/lints/missing_return.rs:40`) uses `matches!(ret_type.kind, TypeKind::Void)` which will now correctly match void return types instead of missing the `TypeKind::Named { name: "void" }` form.

- [ ] **Step 3: Verify clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/component.rs
git commit -m "fix(ir): normalize built-in type names in lower_type via TypeKind::from

lower_type was directly constructing TypeKind::Named { name }
for all type annotations, bypassing the From<&str> impl that
maps 'void'→Void, 'int'→Int, 'string'→String, etc. This caused
the missing_return lint to miss void functions (the lint matches
TypeKind::Void only, not Named { name: \"void\" }).

Fix: use TypeKind::from(name.as_str()) which normalizes through
the existing conversion table at ir/types.rs:298-311."
```

---

### Task 2: Fix Template Error Spans (I2)

**Files:**
- Modify: `crates/omnilang-core/src/parser/expr/template.rs:12-92`
- Modify: `crates/omnilang-core/src/parser/expr/prefix.rs:25`

- [ ] **Step 1: Add `base_span` parameter to `parse_template_parts`**

In `crates/omnilang-core/src/parser/expr/template.rs`, change the function signature at line 12:

```rust
// Before:
pub(super) fn parse_template_parts(s: &str) -> Result<Vec<TemplatePart>, ParseError> {

// After:
pub(super) fn parse_template_parts(s: &str, base_span: Span) -> Result<Vec<TemplatePart>, ParseError> {
```

- [ ] **Step 2: Replace zeroed spans with computed spans**

In the same file, replace the 3 error sites that use `Span { start: 0, end: 0, line: 0, col: 0 }`:

**Error site 1 (line 43-51, "Unterminated template expression"):**
```rust
// Before:
                return Err(ParseError {
                    message: "Unterminated template expression".into(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                });

// After:
                return Err(ParseError {
                    message: "Unterminated template expression".into(),
                    span: Span {
                        start: base_span.start + expr_start,
                        end: base_span.start + i,
                        line: base_span.line,
                        col: base_span.col + expr_start,
                    },
                });
```

**Error site 2 (line 55-63, "Empty template expression"):**
```rust
// Before:
                return Err(ParseError {
                    message: "Empty template expression".into(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                });

// After:
                return Err(ParseError {
                    message: "Empty template expression".into(),
                    span: Span {
                        start: base_span.start + expr_start,
                        end: base_span.start + i,
                        line: base_span.line,
                        col: base_span.col + expr_start,
                    },
                });
```

**Error site 3 (line 69-74, "In template expression"):**
```rust
// Before:
            let expr = parse_simple_template_expr(expr_str).map_err(|e| ParseError {
                message: format!("In template expression '{}': {}", expr_str, e),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    col: 0,
                },
            })?;

// After:
            let expr = parse_simple_template_expr(expr_str).map_err(|e| ParseError {
                message: format!("In template expression '{}': {}", expr_str, e),
                span: Span {
                    start: base_span.start + expr_start,
                    end: base_span.start + i,
                    line: base_span.line,
                    col: base_span.col + expr_start,
                },
            })?;
```

- [ ] **Step 3: Update caller in prefix.rs**

In `crates/omnilang-core/src/parser/expr/prefix.rs`, the token span is already captured at line 18 (`let span = t.span;`) and is in scope at line 25. Change the call:

```rust
// Before (line 25):
                let parts = parse_template_parts(s)?;

// After:
                let parts = parse_template_parts(s, span)?;
```

- [ ] **Step 4: Verify compilation and tests**

Run: `cargo test --workspace`
Expected: 801+ tests, 0 failures.

- [ ] **Step 5: Verify clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/expr/template.rs crates/omnilang-core/src/parser/expr/prefix.rs
git commit -m "fix(parser): propagate template token span to template parse errors

All template string parse errors were emitting Span(0,0,0,0),
making template diagnostics invisible in the editor (no
highlighting, no navigation). The span data (expr_start, i) was
already computed but not propagated into ParseError.

Fix: add base_span parameter to parse_template_parts, compute
relative offsets for all 3 error sites."
```

---

### Task 3: Remove 3 Dead Helper Methods

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:808-824` (remove `get_component`, `has_function`)
- Modify: `crates/omnilang-core/src/semantic/symbols.rs:128-130` (remove `lookup_in_current_scope`)

- [ ] **Step 1: Remove `get_component` and `has_function` from IRProgram**

In `crates/omnilang-core/src/ir/types.rs`, delete lines 807-824:

```rust
// Remove these 18 lines (807-824):
    /// Look up a component by name across all modules (short-circuit on first match).
    #[must_use]
    pub fn get_component(&self, name: &str) -> Option<&ComponentIR> {
        self.modules.values().find_map(|m| m.components.get(name))
    }

    /// Check whether a store exists by name across all modules (short-circuit).
    #[must_use]
    pub fn has_store(&self, name: &str) -> bool {
        self.modules.values().any(|m| m.stores.contains_key(name))
    }

    /// Check whether a function exists by name across all modules (short-circuit).
    pub fn has_function(&self, name: &str) -> bool {
        self.modules
            .values()
            .any(|m| m.functions.contains_key(name))
    }
```

Note: Keep `has_store` (lines 813-816) — it has 5 legitimate callers. Only remove `get_component` and `has_function`.

- [ ] **Step 2: Remove `lookup_in_current_scope` from SymbolTable**

In `crates/omnilang-core/src/semantic/symbols.rs`, delete lines 127-130:

```rust
// Remove these 4 lines (127-130):
    /// Look up a symbol name in the current scope only (not parent scopes).
    pub fn lookup_in_current_scope(&self, name: &str) -> Option<&Symbol> {
        self.scopes[self.current_scope].symbols.get(name)
    }
```

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test --workspace`
Expected: 801+ tests, 0 failures.

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/semantic/symbols.rs
git commit -m "chore: remove 3 dead helper methods (0 callers each)

get_component, has_function, and lookup_in_current_scope were
added speculatively in commit 3e6627fe with #[must_use] but
never wired to any caller. has_store (same commit) has 5 active
callers and is retained."
```

---

### Task 4: Remove Dead `ref_name` and `test_id` Fields from ElementIR

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:625-626` (remove field definitions)
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:623-624` (remove construction, line ~617-624)
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:1008-1009` (remove construction, second site)
- Modify: `crates/omnilang-core/src/ir/optimize.rs:806-807, 841-842, 872-873, 1013-1014` (4 construction sites)
- Modify: `crates/omnilang-core/src/codegen/tests_regression/fixtures_shared.rs:114-115, 283-284` (2 test sites)
- Modify: `crates/omnilang-core/src/semantic/lints/invalid_event_field.rs:178-179, 236-237, 277-278` (3 test sites)
- Modify: `crates/omnilang-core/src/semantic/lints/store_action_event.rs:150-151, 199-200, 240-241` (3 test sites)

- [ ] **Step 1: Remove field definitions from struct**

In `crates/omnilang-core/src/ir/types.rs`, remove lines 625-626:

```rust
// Before (lines 620-627):
    pub style_classes: Option<serde_json::Value>,
    pub inline_style: Option<serde_json::Value>,
    pub children: Vec<ViewTree>,
    pub test_id: Option<String>,
    pub ref_name: Option<String>,
    pub construction_kind: ConstructionKind,

// After:
    pub style_classes: Option<serde_json::Value>,
    pub inline_style: Option<serde_json::Value>,
    pub children: Vec<ViewTree>,
    pub construction_kind: ConstructionKind,
```

- [ ] **Step 2: Remove from all 14 construction sites**

Remove `test_id: None,` and `ref_name: None,` lines from every ElementIR construction. The sites are:

1. `ir/lower/view.rs:623-624` — remove `test_id: None,` and `ref_name: None,`
2. `ir/lower/view.rs:1008-1009` — remove `test_id: None,` and `ref_name: None,`
3. `ir/optimize.rs:806-807` — remove `test_id: None,` and `ref_name: None,`
4. `ir/optimize.rs:841-842` — remove `test_id: None,` and `ref_name: None,`
5. `ir/optimize.rs:872-873` — remove `test_id: None,` and `ref_name: None,`
6. `ir/optimize.rs:1013-1014` — remove `test_id: None,` and `ref_name: None,`
7. `codegen/tests_regression/fixtures_shared.rs:114-115` — remove `test_id: None,` and `ref_name: None,`
8. `codegen/tests_regression/fixtures_shared.rs:283-284` — remove `test_id: None,` and `ref_name: None,`
9. `semantic/lints/invalid_event_field.rs:178-179` — remove `test_id: None,` and `ref_name: None,`
10. `semantic/lints/invalid_event_field.rs:236-237` — remove `test_id: None,` and `ref_name: None,`
11. `semantic/lints/invalid_event_field.rs:277-278` — remove `test_id: None,` and `ref_name: None,`
12. `semantic/lints/store_action_event.rs:150-151` — remove `test_id: None,` and `ref_name: None,`
13. `semantic/lints/store_action_event.rs:199-200` — remove `test_id: None,` and `ref_name: None,`
14. `semantic/lints/store_action_event.rs:240-241` — remove `test_id: None,` and `ref_name: None,`

Each removal is simply deleting the two lines for that construction site.

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test --workspace`
Expected: 801+ tests, 0 failures.

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs \
        crates/omnilang-core/src/ir/lower/view.rs \
        crates/omnilang-core/src/ir/optimize.rs \
        crates/omnilang-core/src/codegen/tests_regression/fixtures_shared.rs \
        crates/omnilang-core/src/semantic/lints/invalid_event_field.rs \
        crates/omnilang-core/src/semantic/lints/store_action_event.rs
git commit -m "chore(ir): remove dead ref_name and test_id fields from ElementIR

Both fields were always None (14 construction sites) and never
read (0 readers anywhere in the codebase). ref_name functionality
is served by props[\"data-ref\"] (lowered at view.rs:444-451).
test_id was never implemented (no lowering from ElementProp::TestId)."
```

---

### Task 5: Add dw_log Size Limit

**Files:**
- Modify: `poiesis/src-tauri/src/commands/debug.rs:42-61`

- [ ] **Step 1: Add size check before appending**

In `poiesis/src-tauri/src/commands/debug.rs`, modify the `dw_log` function:

```rust
/// Append a line to the debug log file for frontend debugging.
#[tauri::command]
pub(crate) fn dw_log(message: String) -> Result<(), String> {
    // Reject excessively long messages
    if message.len() > 4096 {
        return Err("Log message exceeds 4KB limit".to_string());
    }
    let path = std::env::temp_dir().join("poiesis-debug.log");
    // Check current file size, truncate if over 10MB
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 10 * 1024 * 1024 {
            // Truncate: keep last 1MB
            if let Ok(content) = std::fs::read_to_string(&path) {
                let keep_start = content.len().saturating_sub(1024 * 1024);
                let tail = &content[keep_start..];
                std::fs::write(&path, tail)
                    .map_err(|e| format!("Failed to truncate log: {e}"))?;
            }
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open log file: {e}"))?;
    // ... rest of existing function unchanged ...
```

This adds two guards:
1. Per-message 4KB limit — prevents abuse via single large writes
2. 10MB file limit with truncation — prevents disk fill via repeated writes

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p poiesis`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src-tauri/src/commands/debug.rs
git commit -m "fix(security): add size limits to dw_log debug command

dw_log had no size cap — a compromised dependency or dev tools
script could fill the temp directory via repeated writes.

Added: per-message 4KB limit + 10MB file limit with truncation."
```

---

### Task 6: Make WASM CI Blocking + Add Artifact Upload

**Files:**
- Modify: `.github/workflows/ci.yml:129-142`

- [ ] **Step 1: Remove continue-on-error and add artifact upload**

In `.github/workflows/ci.yml`, modify the WASM job:

```yaml
# Before (lines 129-142):
  wasm:
    runs-on: ubuntu-latest
    needs: rust
    continue-on-error: true  # Non-blocking while we establish reliability
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          targets: wasm32-unknown-unknown
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Test omnilang-wasm
        run: wasm-pack test --node omnilang-wasm

# After:
  wasm:
    runs-on: ubuntu-latest
    needs: rust
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          targets: wasm32-unknown-unknown
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Test omnilang-wasm
        run: wasm-pack test --node omnilang-wasm
      - name: Build WASM (release)
        run: wasm-pack build --release omnilang-wasm
      - name: Upload WASM artifact
        uses: actions/upload-artifact@v4
        with:
          name: omnilang-wasm
          path: omnilang-wasm/pkg/
          retention-days: 7
```

Changes:
1. Remove `continue-on-error: true` — WASM tests are stable, should block PRs
2. Add `wasm-pack build --release` step — produce optimized WASM binary
3. Add artifact upload — preserve the compiled `.wasm` for deployment/inspection

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: make WASM job blocking, add artifact upload

WASM tests have been stable for consecutive runs — the temporary
continue-on-error guard is no longer needed. Added release build
and artifact upload to preserve compiled .wasm binary."
```

---

### Task 7: Fix eval-run-omnilang.sh

**Files:**
- Modify: `.claude/evals/eval-run-omnilang.sh:12,57-60,129-131`

- [ ] **Step 1: Fix crate scope**

In `.claude/evals/eval-run-omnilang.sh`, change the test command from facade-only to workspace-wide:

Line 57:
```bash
# Before:
  if out=$(cargo test 2>&1); then
# After:
  if out=$(cargo test --workspace 2>&1); then
```

- [ ] **Step 2: Update baseline values**

Line 12 and surrounding baseline definitions:
```bash
# Before:
BASELINE_TESTS=227
# After:
BASELINE_TESTS=800
```

And wherever clippy baseline is defined:
```bash
# Before:
BASELINE_CLIPPY=24
# After:
BASELINE_CLIPPY=0
```

- [ ] **Step 3: Fix C4 WASM check**

If the script has a C4 check that references `cargo build --features wasm`, update it to use `wasm-pack test --node omnilang-wasm` instead, matching the CI job.

- [ ] **Step 4: Verify script runs**

Run: `bash .claude/evals/eval-run-omnilang.sh`
Expected: C1 should show 800+ tests (PASS), C2-C5 should reflect current state.

- [ ] **Step 5: Commit**

```bash
git add .claude/evals/eval-run-omnilang.sh
git commit -m "fix(evals): update eval-run-omnilang.sh for workspace scope

Changed cargo test to --workspace (was only testing facade crate,
223 tests vs actual 801+). Updated baselines: TESTS=800, CLIPPY=0."
```

---

### Task 8: Remove Broken eval-run.sh

**Files:**
- Delete: `.claude/evals/eval-run.sh`
- Modify: `.claude/evals/editor-client.md` (if it references eval-run.sh)

- [ ] **Step 1: Delete the broken script**

Run: `rm .claude/evals/eval-run.sh`

This script crashes at line 9 because `editor/client/` was restructured into `poiesis/`. Frontend tests run via CI `frontend` job's `vitest` step — this manual eval script has no consumers.

- [ ] **Step 2: Check for references**

Run: `grep -r "eval-run.sh" .claude/ --include='*.md'`
Expected: Only references in the script itself and possibly `editor-client.md`.

If `editor-client.md` references `eval-run.sh`, update it to reference the CI `frontend` job instead.

- [ ] **Step 3: Commit**

```bash
git rm .claude/evals/eval-run.sh
# If editor-client.md was modified:
git add .claude/evals/editor-client.md
git commit -m "chore(evals): remove broken eval-run.sh (editor/client/ no longer exists)

Frontend tests run via CI frontend job's vitest step. The manual
eval-run.sh referenced a pre-P35 directory structure."
```

---

### Task 9: Update README ADR Count

**Files:**
- Modify: `README.md:51,86`

- [ ] **Step 1: Update both occurrences**

In `README.md`, change:
```markdown
# Line 51:
  decisions/adrs/  # 10 ADRs
# To:
  decisions/adrs/  # 13 ADRs

# Line 86 (if it exists):
# Search for "10 ADRs" and replace with "13 ADRs"
```

Run to find all occurrences: `grep -n "10 ADR" README.md`

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README ADR count from 10 to 13"
```

---

### Task 10: Update Stale ADR-011

**Files:**
- Modify: `.architecture/decisions/adrs/ADR-011-p4-2-semantic-type-checking-architecture.md:70`

- [ ] **Step 1: Remove stale limitation**

In `ADR-011-p4-2-semantic-type-checking-architecture.md`, change line 70:

```markdown
# Before:
* **已知限制**: `check_call` 未接入 resolve pass、MatchExpr/ArrayLiteral 仅检查第一个元素、...

# After:
* **已知限制**: MatchExpr/ArrayLiteral 仅检查第一个元素、...
```

Remove "`check_call` 未接入 resolve pass、" since commit `f4e87e9f` wired it before the ADR was committed.

- [ ] **Step 2: Commit**

```bash
git add .architecture/decisions/adrs/ADR-011-p4-2-semantic-type-checking-architecture.md
git commit -m "docs(adr): remove stale check_call limitation from ADR-011

check_call was wired into resolve_expr_refs by commit f4e87e9f,
committed 30 minutes before ADR-011. The ADR documented the
pre-fix state."
```

---

### Task 11: Update Stale poiesis-tauri-security-model Memory

**Files:**
- Modify: `C:\Users\XHT\.claude\projects\D--space-OmniLang\memory\poiesis-tauri-security-model.md`

- [ ] **Step 1: Update FS scope description**

In the memory file, update the FS scope row in the security posture table (around line 45):

```markdown
# Before:
| FS 范围 | ✅ 已限制 | $HOME/$APPDATA 仅 |
# After:
| FS 范围 | ✅ 已限制 | $HOME/$APPDATA + D:/ + C:/ — intentionally broad per project-permissions design |
```

Also update the "v5 修复中已解决的安页面" section to note that commit `03e7e981` restored drive-root access by design (per `project-permissions` memory from 2026-07-24).

- [ ] **Step 2: Add note about intentional design**

Append a note at the end of the file:
```markdown
## 2026-07-28 Update

Commit `03e7e981` (2026-07-28) restored `D:/**` and `C:/**` to Tauri capabilities.
This is intentional per [[project-permissions]] — an editor must open projects anywhere.
The $HOME-only restriction was from v5 P1-1 and was subsequently relaxed to support
projects on non-system drives. The `open_project` path validation and `sanitize_path`
canonicalization remain as defense-in-depth.
```

- [ ] **Step 3: No commit needed** (memory files are in user directory, not repo)

---

### Task 12: Close I8 as By-Design Per ADR-003

**Files:**
- Create/Modify: Document the closure in the v1 breadth review or deferrals log

- [ ] **Step 1: Mark I8 as closed**

In the v1 breadth review at `.architecture/reviews/2026-07-28-breadth-review.md`, find I8 ("Extend cross-target regression tests") and add a note:

```markdown
| **I8** | **CLOSED — By design per ADR-003.** Only P0 backends (OmniView, React, Flutter) get active testing. Other 6 are in maintenance mode behind feature flags. |
```

Or add to `.architecture/deferrals.md` if one exists.

- [ ] **Step 2: Commit**

```bash
git add .architecture/reviews/2026-07-28-breadth-review.md
git commit -m "docs: close I8 as by-design per ADR-003 (P0-only backend testing)"
```

---

### Task 13: Final Verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace
```
Expected: 801+ tests, 0 failures.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```
Expected: 0 warnings.

- [ ] **Step 3: Run cargo fmt**

```bash
cargo fmt --all -- --check
```
Expected: No formatting issues.

- [ ] **Step 4: Verify git status**

```bash
git status
git diff --stat
```
Confirm: only intended files modified, no unintended changes.

---
