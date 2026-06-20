## EVAL: compiler-pipeline

### Purpose
End-to-end validation of the omni-compiler pipeline: lex → parse → semantic → IR → codegen across all 10 target backends.

### Baseline
Date: 2026-06-19
Git SHA baseline to be captured in baseline.json

---

### Phase 1: Build Integrity

**Grader**: Code-based (deterministic)

| # | Eval | Command | Expected |
|---|------|---------|----------|
| B1 | Debug build succeeds | `cargo build` | exit 0, no errors |
| B2 | Release build succeeds | `cargo build --release` | exit 0, no errors |
| B3 | No new compiler warnings | `cargo build 2>&1` | warning count ≤ 3 |

### Phase 2: Lexer

**Grader**: Code-based (`cargo test`)

| # | Eval | Test Suite | Expected |
|---|------|-----------|----------|
| L1 | Token lexing | `lexer_tests` | 13/13 passing |
| L2 | Token definitions | `lexer::token` inline | all passing |

### Phase 3: Parser

**Grader**: Code-based (`cargo test`)

| # | Eval | Test Suite | Expected |
|---|------|-----------|----------|
| P1 | Expression parsing | `parser` inline | all passing |
| P2 | Top-level declarations | `parser_tests` | 26/26 passing |
| P3 | View parsing | `parser::view` inline | all passing |
| P4 | AST round-trip | `parser::ast` inline | all passing |

### Phase 4: Semantic Analysis

**Grader**: Code-based (`cargo test`)

| # | Eval | Test Suite | Expected |
|---|------|-----------|----------|
| S1 | Symbol collection | `semantic_tests` | 3/3 passing |
| S2 | Error reporting | `semantic_tests::test_reports_undeclared_variable` | PASS |
| S3 | Cycle detection | `semantic::cycles` inline | 3/3 passing |
| S4 | Lint rules | `semantic::lints` inline | 10/10 passing |

### Phase 5: IR Lowering

**Grader**: Code-based (`cargo test`)

| # | Eval | Test Suite | Expected |
|---|------|-----------|----------|
| I1 | Logic IR lowering | `ir::lower_logic` inline | 2/2 passing |

### Phase 6: Code Generation (Logic)

**Grader**: Code-based (`cargo test`)

| # | Eval | Target | Test Suite | Expected |
|---|------|--------|-----------|----------|
| C1 | JavaScript | js | `codegen::javascript` inline | 52/52 passing |
| C2 | Python | py | `codegen::python` inline | all passing |
| C3 | Rust | rs | `codegen::rust` inline | 46/46 passing |
| C4 | Dart | dart | `codegen::dart` inline | 43/43 passing |
| C5 | Logic codegen tests | all | `logic_codegen_tests` | 7/7 passing |

### Phase 7: Code Generation (Views)

**Grader**: Code-based (`cargo test`)

| # | Eval | Target | Test Suite | Expected |
|---|------|--------|-----------|----------|
| V1 | React | react | `codegen::react` inline | 32/32 passing |
| V2 | Vue | vue | `codegen::vue` inline | 30/30 passing |
| V3 | Flutter | flutter | `codegen::flutter` inline | 39/39 passing |
| V4 | Dioxus | dioxus | `codegen::dioxus` inline | 30/30 passing |
| V5 | OmniView | omniview | `codegen::omniview` inline | 1/1 passing |
| V6 | Codegen integration | all | `codegen_tests` | 33/33 passing |

### Phase 8: Snapshots & Source Maps

**Grader**: Code-based (`cargo test`)

| # | Eval | Test Suite | Expected |
|---|------|-----------|----------|
| M1 | Codegen snapshots | `codegen_snapshots` | 3/3 passing |
| M2 | Source maps | `sourcemap` inline | 3/3 passing |

### Phase 9: LSP

**Grader**: Human review required

| # | Eval | Notes |
|---|------|-------|
| LSP1 | LSP server starts | Manual verification |
| LSP2 | Diagnostics reported | Manual verification |
| LSP3 | Format on save | Manual verification |

---

### Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Total tests passing | ≥ 410 | 410 |
| Test failures | 0 | 0 |
| Build errors | 0 | 0 |
| Build warnings | ≤ 3 | 3 |
| pass@1 (first attempt) | 100% | TBD |
| pass^3 (3 consecutive) | 100% | TBD |

### Regression Guards

These MUST NOT regress:
- Total test count must not decrease
- Build must stay green
- Warning count must not increase
- All 10 codegen backends must produce valid output
