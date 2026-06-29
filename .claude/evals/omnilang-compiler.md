# EVAL: omnilang-compiler

> **Scope**: `omnilang/` — OmniLang compiler (.omni → Intent IR → 10-target codegen)
> **Stack**: Rust 2021 + serde + thiserror + wasm-bindgen (optional)
> **Baseline**: 2026-06-29 (SHA: main branch, 375 tests, 30 source files)
> **Target**: pass@3 ≥ 90% for capability evals, pass^3 = 100% for regression evals

---

## Baseline Measurements

```
cargo test:       375 passed, 0 failed, 0 ignored
cargo clippy:     220 warnings (73 redundant closure, 45 useless format!, 44 map_or, ...)
cargo build:      passes (release: lto=true, opt-level="s", strip=true)
File count:       30 .rs source files
File sizes:       18 files > 800 lines (largest: omniview.rs 5084, flutter.rs 3654, ir/mod.rs 3643)
Codegen targets:  10 (omniview, python, javascript, rust, dart, react, vue, dioxus, flutter, + IR JSON)
Audit status:     60 issues found (8 CRITICAL, 14 HIGH, 24 MEDIUM, 14 LOW) — 2026-06-18 baseline
                  CVE-4 progress: `_ => "null"` reduced from 7/10 targets → 2/10 (dart.rs:417, flutter.rs:1138)
```

---

## Capability Evals

### C1: Test Suite — All 375 Tests Pass

```
[CAPABILITY EVAL: cargo-test]
Task: All compiler tests pass with zero failures
Command: cd omnilang && cargo test 2>&1
Success Criteria:
  - [ ] cargo test exits with code 0
  - [ ] 375+ tests pass (count must not decrease)
  - [ ] 0 failed, 0 ignored
  - [ ] Doc-tests pass (currently 0)
Grader: code-based (exit code + test count)
Baseline: 375 passed
```

### C2: Clippy Warning Count (Tracked Reduction)

```
[CAPABILITY EVAL: clippy-warnings]
Task: Track and reduce clippy warnings over time
Command: cd omnilang && cargo clippy 2>&1 | grep -c "warning:"
Baseline: 220 warnings
Success Criteria:
  - [ ] Warning count does not increase (≤ 220)
  - [ ] No new clippy categories introduced
  - [ ] Goal: reduce by ≥10 warnings per refactoring session
  - [ ] New code adds 0 new warnings
Grader: code-based (count comparison)
```

### C3: Release Build

```
[CAPABILITY EVAL: cargo-build-release]
Task: Release build succeeds with optimizations
Command: cd omnilang && cargo build --release 2>&1
Success Criteria:
  - [ ] Build completes without error (exit code 0)
  - [ ] cdylib and rlib outputs exist in target/release/
  - [ ] LTO and strip optimizations applied
Grader: code-based (exit code + output existence)
```

### C4: WASM Build (Optional Feature)

```
[CAPABILITY EVAL: wasm-build]
Task: WASM target builds successfully
Command: cd omnilang && cargo build --features wasm --target wasm32-unknown-unknown 2>&1
Success Criteria:
  - [ ] Build completes without error (exit code 0)
  - [ ] WASM binary output exists
  - [ ] compile_to_omniview, compile_to_ir, version WASM exports present
Grader: code-based (exit code)
Note: Requires wasm32-unknown-unknown target installed. SKIP if unavailable.
```

### C5: Compiler Pipeline — .omni → IR → JSON (Smoke Test)

```
[CAPABILITY EVAL: compile-pipeline]
Task: Verify the full compile pipeline works end-to-end
Test input: "fn add(a: int, b: int) -> int { return a + b; }"
Command: cd omnilang && cargo test compile_pipeline_smoke -- --nocapture 2>&1
Success Criteria:
  - [ ] Lexer tokenizes without panic
  - [ ] Parser produces valid AST
  - [ ] Semantic analysis completes
  - [ ] IR lowering produces valid IRProgram
  - [ ] JSON serialization succeeds
  - [ ] Result.success == true
Grader: code-based (test pass/fail)
```

### C6: All 10 Codegen Targets Produce Output

```
[CAPABILITY EVAL: codegen-targets]
Task: Each codegen target produces non-empty output for a basic program
Test input: simple function + struct + component (omni source)
Success Criteria:
  - [ ] omniview generates JS + HTML + CSS (non-empty)
  - [ ] python generates valid .py output
  - [ ] javascript generates valid .js output
  - [ ] rust generates valid .rs output
  - [ ] dart generates valid .dart output
  - [ ] react generates valid JSX output
  - [ ] vue generates valid .vue SFC output
  - [ ] dioxus generates valid .rs (rsx!) output
  - [ ] flutter generates valid .dart output
  - [ ] No target emits "null" as a fallback for expr variants
Grader: code-based (test pass/fail + content checks)
```

### C7: Multi-File Compilation

```
[CAPABILITY EVAL: multi-file]
Task: Cross-file import resolution and compilation works
Test input: 2+ .omni files with import statements
Success Criteria:
  - [ ] Cross-file imports resolved correctly
  - [ ] Pub-visibility enforced (non-pub symbols rejected)
  - [ ] Cycle detection reports circular imports
  - [ ] Logic→view boundary enforced
  - [ ] Merged IR contains all modules
Grader: code-based (test pass/fail)
```

---

## Regression Evals

### R1: No Unsafe Without Safety Comment

```
[REGRESSION EVAL: unsafe-safety-comments]
Baseline: 2026-06-29 main
Command: cd omnilang && grep -rn "unsafe" src/ --include="*.rs" | grep -v "// SAFETY:" | grep -v "///" | grep -v "#\[cfg(test)\]" || echo "PASS: all unsafe blocks documented"
Success Criteria:
  - [ ] Every `unsafe` block has a `// SAFETY:` comment
  - [ ] Every `unsafe fn` has a `# Safety` doc comment
Grader: code-based (grep)
```

### R2: No `.unwrap()` in Non-Test Code

```
[REGRESSION EVAL: no-unwrap-production]
Baseline: 2026-06-29 main
Command: cd omnilang && grep -rn "\.unwrap()" src/ --include="*.rs" | grep -v "#\[cfg(test)\]" | grep -v "mod tests" || echo "PASS"
Success Criteria:
  - [ ] Zero `.unwrap()` calls outside test modules and test cfg blocks
  - [ ] All fallible operations use `?` or explicit match
Grader: code-based (grep)
```

### R3: CSS/JS Injection Prevention

```
[REGRESSION EVAL: injection-prevention]
Baseline: CVEs from audit 2026-06-18
Command: cd omnilang && cargo test injection css_injection xss -- --nocapture 2>&1
Success Criteria:
  - [ ] CSS values rejected if containing javascript: or data: URLs
  - [ ] JS template literals escape user content
  - [ ] HTML element content properly escaped
  - [ ] No raw user text injected into _effect() callback bodies
Grader: code-based (test pass/fail)
```

### R4: Cross-File Error Messages — Clear Diagnostics

```
[REGRESSION EVAL: error-diagnostics]
Baseline: 2026-06-29 main
Command: cd omnilang && cargo test diagnostic error_message parse_error -- --nocapture 2>&1
Success Criteria:
  - [ ] Parse errors include file, line, col
  - [ ] Semantic errors include suggestion where applicable
  - [ ] Import resolution errors name both files
  - [ ] Cycle errors list the full cycle path
Grader: code-based (test pass/fail)
```

### R5: File Size Limits (Tracked Reduction)

```
[REGRESSION EVAL: file-sizes]
Baseline: 2026-06-29 (18 files > 800 lines)
Command: cd omnilang && find src -name "*.rs" -exec wc -l {} \; | awk '$1 > 800 {print $1, $2}' | sort -rn
Files over 800 lines:
  - omniview.rs: 5084 (target: split into sub-modules)
  - flutter.rs: 3654
  - ir/mod.rs: 3643
  - lints.rs: 3600
  - semantic/mod.rs: 2942
  - react.rs: 2897
  - javascript.rs: 2220
  - top_level.rs: 2085
  - view.rs: 1993
  - dioxus.rs: 1885
  - vue.rs: 1824
  - rust.rs: 1812
  - dart.rs: 1737
Success Criteria:
  - [ ] No file grows beyond current size
  - [ ] Count of files > 800 lines does not increase (>13)
  - [ ] Goal: each refactoring extracts ≥1 module from oversized files
Grader: code-based (wc -l comparison)
```

### R6: Codegen Output Completeness — No `_ => "null"` fallbacks

```
[REGRESSION EVAL: codegen-no-null-fallback]
Baseline: CVE-4 from audit 2026-06-18 (7/10 targets discard expr variants)
Command: cd omnilang && cargo test null_fallback method_call_expr array_expr struct_expr -- --nocapture 2>&1
Success Criteria:
  - [ ] All 10 codegen targets handle MethodCallExpr
  - [ ] All 10 codegen targets handle ArrayExpr
  - [ ] All 10 codegen targets handle StructExpr
  - [ ] All 10 codegen targets handle BlockExpr
  - [ ] No wildcard match arm emits "null" for unhandled expr variants
Grader: code-based (test pass/fail)
```

---

## Audit Issue Tracking

Counts from 2026-06-18 audit (60 total). Track resolution:

| Level | Initial | Resolved | Remaining |
|-------|---------|----------|-----------|
| CRITICAL | 8 | 0 | 8 |
| HIGH | 14 | 0 | 14 |
| MEDIUM | 24 | 0 | 24 |
| LOW | 14 | 0 | 14 |

Key CVEs to track:
- **CVE-1**: CSS value injection (omniview.rs, react.rs, vue.rs)
- **CVE-2**: Runtime name collision (_signal/_effect/_computed)
- **CVE-3**: @for template interpolation broken for compound expressions
- **CVE-4**: ~~7~~→2/10 targets: wildcard `_ => "null"` discards expr variants (progress: 5 targets fixed, dart.rs + flutter.rs remain)
- **CVE-5**: Dart/Flutter: `name: Type` → should be `Type name`
- **CVE-6**: Dioxus: spurious `{} div` line in rsx! macro
- **CVE-7**: ViewTree::Text fallback → JS code injection via `_effect()` body
- **CVE-8**: `<`/`>` chars globally truncate string literals in lexer

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Test pass rate | 100% (375/375) |
| Clippy warnings | ≤ 220 (trending ↓) |
| Release build | PASS |
| Codegen target coverage | 10/10 |
| Security CVEs resolved | 8/8 CRITICAL |
| Files ≤ 800 lines | trending ↑ |
| `unsafe` without comment | 0 |

---

## Eval Script

See `.claude/evals/eval-run-omnilang.sh` for automated execution of all code-based graders.
