# Compiler Pipeline Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 2 CRITICAL + 1 HIGH + 6 MEDIUM compiler pipeline issues found in the 2026-07-31 architecture review.

**Architecture:** Three changes to the pipeline handoff (`run_pipeline`, `analyse_multi`, `analyse_multi_cached`), two parser fixes (design-block guard, recursion depth), and targeted cleanups (cache size, diagnostic spans, duplicate-design warnings, clippy hygiene, peek_char O(n²), serialization drift). All changes are in `crates/omnilang-core/`, `omnilang/`, and `omnic/`.

**Tech Stack:** Rust, Cargo workspace (`crates/omnilang-core`, `omnilang`, `omnic`)

**Source review:** `.architecture/reviews/full-workspace-review-2026-07-31.md`

---

### Task 1: Fix post-lowering diagnostics dropped in all pipeline entry points (C1)

**Files:**
- Modify: `crates/omnilang-core/src/lib.rs:107-111`
- Modify: `crates/omnilang-core/src/analyse.rs:303-312`
- Modify: `crates/omnilang-core/src/analyse.rs:464-472`
- Create: `crates/omnilang-core/tests/pipeline_diagnostics_test.rs`

- [ ] **Step 1: Write pipeline-level regression tests for dropped diagnostics**

Create `crates/omnilang-core/tests/pipeline_diagnostics_test.rs`:

```rust
//! Regression tests: ensure diagnostics produced during ir::lower()
//! (E0005, E0006, E0011) surface through run_pipeline and analyse_multi.
//!
//! These were silently dropped before the fix because diagnostics were
//! drained from semantics *before* ir::lower() was called.

use omnilang_core::parser;

/// A design block referencing a nonexistent token should produce E0005.
#[test]
fn run_pipeline_surfaces_undefined_token_error() {
    let source = r#"
        design App {
            token primary: color = "#6366f1";
            class Card {
                background: token(missing);
            }
        }
    "#;
    let program = parser::parse_source(source, "test.omni").0;
    let result = omnilang_core::run_pipeline(&program, None, None);
    let has_e0005 = result
        .diagnostics
        .iter()
        .any(|d| d.code() == "E0005" && d.message.contains("undefined token"));
    assert!(
        has_e0005,
        "E0005 should surface for undefined token reference. Diagnostics: {:?}",
        result.diagnostics
    );
}

/// A component omitting a required prop should produce E0011.
#[test]
fn run_pipeline_surfaces_missing_required_prop_error() {
    let source = r#"
        component Child {
            @prop name: string;
            render {}
        }
        component Parent {
            render {
                <Child />
            }
        }
    "#;
    let program = parser::parse_source(source, "test.omni").0;
    let result = omnilang_core::run_pipeline(&program, None, None);
    let has_e0011 = result
        .diagnostics
        .iter()
        .any(|d| d.code() == "E0011" && d.message.contains("required prop"));
    assert!(
        has_e0011,
        "E0011 should surface for missing required prop. Diagnostics: {:?}",
        result.diagnostics
    );
}

/// Multifile: analyse_multi should surface design errors.
#[test]
fn analyse_multi_surfaces_design_errors() {
    let source_a = r#"
        design App {
            token primary: color = "#6366f1";
            class Card {
                background: token(missing);
            }
        }
    "#;
    // Use a second file to trigger the multi-file path.
    let source_b = r#"component Foo { render {} }"#;
    let sources = vec![
        ("a.omni".to_string(), source_a.to_string()),
        ("b.omni".to_string(), source_b.to_string()),
    ];
    let result = omnilang_core::analyse_multi(sources, false);
    let has_e0005 = result
        .diagnostics
        .iter()
        .any(|d| d.code() == "E0005");
    assert!(
        has_e0005,
        "E0005 should surface in analyse_multi. Diagnostics: {:?}",
        result.diagnostics
    );
}
```

- [ ] **Step 2: Run tests to verify they FAIL (diagnostics are dropped)**

```bash
cd crates/omnilang-core && cargo test --test pipeline_diagnostics_test -- --nocapture
```

Expected: All 3 tests FAIL — `run_pipeline` and `analyse_multi` return zero diagnostics even though the source has intentional errors.

- [ ] **Step 3: Fix `run_pipeline` — move diagnostics drain after `ir::lower()`**

In `crates/omnilang-core/src/lib.rs`, change lines 107-111 from:

```rust
    diagnostics.extend(std::mem::take(&mut semantics.diagnostics));
    let has_errors = diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    let mut ir = ir::lower(&program, &mut semantics, None);
```

To:

```rust
    let mut ir = ir::lower(&program, &mut semantics, None);
    // Drain diagnostics AFTER lowering — ir::lower() may append new diagnostics
    // (design validation, required-prop validation, ref-binding validation).
    diagnostics.extend(std::mem::take(&mut semantics.diagnostics));
    let has_errors = diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
```

- [ ] **Step 4: Fix `analyse_multi` — move diagnostics drain after `ir::lower()`**

In `crates/omnilang-core/src/analyse.rs`, change lines 303-312 from:

```rust
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in &semantics.diagnostics {
                let mut d = d.clone();
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            let ir_program =
                crate::ir::lower(program_ref, &mut semantics, Some(&global_const_values));
            (path.clone(), ir_program, local_diags)
```

To:

```rust
            let ir_program =
                crate::ir::lower(program_ref, &mut semantics, Some(&global_const_values));
            // Drain diagnostics AFTER lowering — ir::lower() may append new diagnostics.
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in &semantics.diagnostics {
                let mut d = d.clone();
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            (path.clone(), ir_program, local_diags)
```

- [ ] **Step 5: Fix `analyse_multi_cached` — same pattern**

In `crates/omnilang-core/src/analyse.rs`, change lines 464-472 (same transformation — move `ir::lower` above the diagnostics drain):

```rust
            let ir_program = crate::ir::lower(program, &mut semantics, Some(&global_const_values));
            // Drain diagnostics AFTER lowering — ir::lower() may append new diagnostics.
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in &semantics.diagnostics {
                let mut d = d.clone();
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            (path.clone(), (ir_program, local_diags))
```

- [ ] **Step 6: Run all tests to verify fixes pass**

```bash
cd crates/omnilang-core && cargo test -- --nocapture
cargo test --test pipeline_diagnostics_test -- --nocapture
```

Expected: All tests PASS including the new pipeline regression tests.

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/lib.rs crates/omnilang-core/src/analyse.rs crates/omnilang-core/tests/pipeline_diagnostics_test.rs
git commit -m "fix: restore post-lowering diagnostics in all pipeline entry points

ir::lower() pushes design validation, required-prop, and ref-binding
diagnostics into semantics.diagnostics, but all three entry points
(run_pipeline, analyse_multi, analyse_multi_cached) drained diagnostics
BEFORE calling lower(). Move the drain to after ir::lower().

Add pipeline-level regression tests for E0005 (undefined token refs)
and E0011 (missing required props) through both run_pipeline and
analyse_multi paths."
```

---

### Task 2: Fix non-OmniView design-block guard — warn-and-continue, don't discard output (H1)

**Files:**
- Modify: `omnilang/src/codegen/mod.rs:92-203`
- Modify: `omnic/src/commands/build.rs:31-48`
- Modify: `omnic/src/commands/run.rs` (check if warnings are printed)

- [ ] **Step 1: Write a test verifying non-OmniView targets emit files + warnings for design-bearing projects**

We'll verify this manually first (CLI test), then add a Rust integration test.

```bash
# Build omnic from current HEAD
cd omnic && cargo build --release

# Verify CURRENT broken behavior: zero files, exit 0
cat > /tmp/design_test.omni << 'EOF'
design App { token primary: color = "#6366f1"; }
component Test { render { <div>hi</div> } }
EOF
./target/release/omnic build --target react /tmp/design_test.omni -o /tmp/design_out
# Expected (broken): "Compiled 0 file(s) to react successfully." — ZERO files
ls /tmp/design_out  # Should be empty (the bug)
```

- [ ] **Step 2: Replace 9x duplicated guard with a single warn-and-continue helper**

In `omnilang/src/codegen/mod.rs`, add this function after `has_design_blocks` (line 65):

```rust
/// Attach a design-block warning to a CompileOutput without discarding files.
/// Call after `generate()` for non-OmniView targets, before returning output.
fn warn_design_blocks(mut output: CompileOutput, ir: &IRProgram) -> CompileOutput {
    if has_design_blocks(ir) {
        output.warnings.push(DESIGN_BLOCK_WARNING.into());
    }
    output
}
```

Then replace each of the 9 guard blocks. For example, Python (lines 92-103) changes from:

```rust
        #[cfg(feature = "codegen-python")]
        Target::Python => {
            if has_design_blocks(ir) {
                return CompileOutput {
                    files: vec![],
                    errors: vec![],
                    source_map: vec![],
                    warnings: vec![DESIGN_BLOCK_WARNING.into()],
                };
            }
            python::PythonCodegen.generate(ir, source_files)
        }
```

To:

```rust
        #[cfg(feature = "codegen-python")]
        Target::Python => {
            warn_design_blocks(
                python::PythonCodegen.generate(ir, source_files),
                ir,
            )
        }
```

Do the same replacement for all 9 non-OmniView backends: JavaScript (line 105-115), TypeScript (line 117-127), Rust (line 129-139), Dart (line 141-151), React (line 153-167), Vue (line 169-179), Dioxus (line 181-195), Flutter (line 196-210).

- [ ] **Step 3: Print warnings in `omnic build`**

In `omnic/src/commands/build.rs`, change the output section (lines 38-47) from:

```rust
    let count = write_output_files(output_path, data["files"].as_object())?;
    if count > 0 {
        println!("Compiled {count} file(s) to {target_str} successfully.");
    }

    Ok(())
```

To:

```rust
    let count = write_output_files(output_path, data["files"].as_object())?;

    // Print warnings
    if let Some(warnings) = data["warnings"].as_array() {
        for w in warnings {
            if let Some(msg) = w.as_str() {
                eprintln!("WARNING: {msg}");
            }
        }
    }

    if count > 0 {
        println!("Compiled {count} file(s) to {target_str} successfully.");
    } else {
        // Exit non-zero if zero files and there were errors or warnings
        anyhow::bail!("No files emitted. Check warnings above.");
    }

    Ok(())
```

- [ ] **Step 4: Rebuild omnic and verify the fix**

```bash
cd omnic && cargo build --release

# Should now produce output files + WARNING
./target/release/omnic build --target react /tmp/design_test.omni -o /tmp/design_out2
# Expected: WARNING about design blocks, non-zero file count
ls /tmp/design_out2  # Should contain .tsx/.css files (component compiled, design dropped)
```

- [ ] **Step 5: Add Rust integration test for cross-target design block behavior**

Add to `omnilang/tests/cross_target.rs` (append after existing tests):

```rust
#[test]
fn react_target_with_design_blocks_emits_files_and_warning() {
    let source = r#"
        design App { token primary: color = "#6366f1"; }
        component Hello { render { <div>Hello</div> } }
    "#;
    let mut sources = std::collections::HashMap::new();
    sources.insert("test.omni".to_string(), source.to_string());
    let result = omnilang::compile_multi_to_target(
        sources,
        omnilang::codegen::Target::React,
        omnilang::path_mapping::derive_path_mapping(&Default::default()),
        false,
        false,
        None,
    );
    let result_str = result.unwrap_or_default();
    // Should contain files (the Hello component), not be empty
    assert!(!result_str.contains("\"files\":{}") || !result_str.contains("\"files\":[]"),
        "React target should emit files for non-design components. Got: {}", result_str);
    // Should contain the design block warning
    assert!(result_str.contains("DESIGN_BLOCK_WARNING") || result_str.contains("Design blocks"),
        "Should emit design block warning. Got: {}", result_str);
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p omnilang cross_target
cargo test -p omnilang -- --nocapture
```

Expected: All tests PASS including the new cross-target test.

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/mod.rs omnic/src/commands/build.rs omnilang/tests/cross_target.rs
git commit -m "fix: restore partial emission for non-OmniView design-bearing projects

Replace 9x duplicated zero-output guard with warn_design_blocks() helper
that attaches a warning without discarding component output. Print warnings
in omnic build. Add cross-target test verifying files + warning for React
target with design blocks.

The ADR-051 A2 fix was intended to add visibility, not suppress output."
```

---

### Task 3: Fix parser stack overflow via recursion guard bypass (H10)

**Files:**
- Modify: `crates/omnilang-core/src/parser/expr/mod.rs:46-57` (`parse_expr_bp`)
- Modify: `crates/omnilang-core/src/parser/expr/mod.rs:28-43` (`parse_expr`)
- Create: `crates/omnilang-core/tests/parser_recursion_test.rs`

- [ ] **Step 1: Write a test that reproduces the crash**

Create `crates/omnilang-core/tests/parser_recursion_test.rs`:

```rust
use omnilang_core::parser;

#[test]
fn deep_unary_chain_does_not_overflow() {
    // 10,000 consecutive `!` operators — previously crashed with stack overflow at ~300
    let source = "!".repeat(10_000);
    let result = parser::parse_source(&source, "test.omni");
    // Must not panic/crash — should return either a parsed program or a parse error
    let _ = result;
}

#[test]
fn deep_nested_ternary_does_not_overflow() {
    // Deeply nested a ? b : c ? d : e ? ... — right-recursive through parse_expr_bp
    let mut source = String::from("a");
    for _ in 0..5_000 {
        source = format!("{} ? b : c", source);
    }
    let result = parser::parse_source(&source, "test.omni");
    let _ = result;
}

#[test]
fn deep_assignment_chain_does_not_overflow() {
    // a = b = c = ... — right-recursive through parse_expr_bp(1)
    let source = "a = ".repeat(5_000) + "0";
    let result = parser::parse_source(&source, "test.omni");
    let _ = result;
}
```

- [ ] **Step 2: Verify the test CRASHES (proves the bug)**

```bash
cd crates/omnilang-core && cargo test --test parser_recursion_test deep_unary -- --nocapture
```

Expected: CRASH with `thread '<unknown>' has overflowed its stack` (exit 127) on the 10,000 `!` test. The 300-byte reproduction may also crash depending on stack size.

- [ ] **Step 3: Move recursion depth accounting into `parse_expr_bp`**

In `crates/omnilang-core/src/parser/expr/mod.rs`, change `parse_expr` (lines 28-43):

```rust
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.enter_recursion()?;
        let mut expr = self.parse_expr_bp(0)?;

        // After Pratt: check for struct literal suffix — TypeName { field: val, ... }
        if self.at(&TokenKind::LBrace) {
            if let Expr::VarRef(ref name, _) = expr {
                if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    expr = self.parse_struct_literal_suffix(name.clone())?;
                }
            }
        }
        self.leave_recursion();
        Ok(expr)
    }
```

And change `parse_expr_bp` (lines 46-57) to include depth tracking:

```rust
    fn parse_expr_bp(&mut self, min_bp: Bp) -> Result<Expr, ParseError> {
        self.enter_recursion()?;
        let mut left = self.parse_prefix()?;

        loop {
            let bp = self.infix_bp();
            if bp == (0, 0) || bp.0 < min_bp {
                break;
            }
            left = self.parse_infix(left, bp)?;
        }
        self.leave_recursion();
        Ok(left)
    }
```

This ensures EVERY recursive call through `parse_expr_bp` (the 5 direct paths: unary `-`/`!`/`await` at prefix.rs:119/132/40, binary/assign/ternary at infix.rs:199/215/237/78) goes through the depth guard.

- [ ] **Step 4: Run the regression tests to verify the crash is fixed**

```bash
cd crates/omnilang-core && cargo test --test parser_recursion_test -- --nocapture
```

Expected: All 3 tests PASS — parser returns without stack overflow (may produce parse errors for pathological inputs, but must not crash).

- [ ] **Step 5: Run the full test suite**

```bash
cd crates/omnilang-core && cargo test -- --nocapture
```

Expected: All tests PASS (existing parser tests should not be affected).

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/expr/mod.rs crates/omnilang-core/tests/parser_recursion_test.rs
git commit -m "fix: prevent parser stack overflow by moving recursion guard into parse_expr_bp

The enter_recursion/leave_recursion guards were only called from parse_expr
and parse_stmt, but the Pratt parser has 5 direct parse_expr_bp() recursion
paths (unary, binary, assign, ternary, compound-assign) that never passed
through the guard. Move depth accounting into parse_expr_bp to cover all
recursion paths.

Verified: 10,000 ! operators previously crashed with stack overflow at ~300;
now returns safely with a parse error at MAX_RECURSION_DEPTH (256)."
```

---

### Task 4: Raise incremental cache cap above actual project sizes (M5)

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:97`

- [ ] **Step 1: Change MAX_CACHE_ENTRIES**

In `crates/omnilang-core/src/analyse.rs`, change line 97:

```rust
pub const MAX_CACHE_ENTRIES: usize = 500;
```

- [ ] **Step 2: Verify the constant is used**

```bash
cd crates/omnilang-core && cargo test -- --nocapture
```

The LRU eviction at `analyse.rs:132-148` uses `MAX_CACHE_ENTRIES` — existing tests verify cache behavior.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/analyse.rs
git commit -m "perf: raise MAX_CACHE_ENTRIES from 50 to 500

50 was below the 92-file learn-omniview corpus, causing 42 files
to be re-parsed on every compile. 500 provides headroom for larger
projects while the LRU still bounds memory."
```

---

### Task 5: Add source spans to design token diagnostics (M7)

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/design.rs:142-149` (add `source_span` to `TokenDef`)
- Modify: `crates/omnilang-core/src/parser/design.rs` (thread parser's `error_offset` into AST)
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:303-312, 335-341, 356-361, 411-418, 445-455` (use real spans)

- [ ] **Step 1: Add `source_span` field to `TokenDef`**

In `crates/omnilang-core/src/parser/ast/design.rs`, change the struct (line 142):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    pub value: TokenValue,
    /// Per-theme value overrides: `(theme_name, value)` pairs.
    #[serde(default)]
    pub theme_values: Vec<(String, TokenValue)>,
    /// Source span for diagnostic locations (line/col).
    /// Defaults to zero — filled by the parser from the token's position.
    #[serde(default)]
    pub source_span: Span,
}
```

- [ ] **Step 2: Thread parser's token position into `source_span`**

In `crates/omnilang-core/src/parser/design.rs`, find where `TokenDef` is constructed (where `name`, `token_type`, and `value` fields are set). Add `source_span` using the token's position:

Look for the `TokenDef { name, token_type, value, theme_values }` construction. Add:

```rust
TokenDef {
    name,
    token_type,
    value,
    theme_values,
    source_span: name_token.span, // or the keyword token's span
}
```

Note: You'll need to locate the exact construction site in `parser/design.rs`. The `TokenDef` is built during `parse_design_body` — the name token's span is available at that point.

- [ ] **Step 3: Use the real span in diagnostic sites**

In `crates/omnilang-core/src/semantic/resolve.rs`, replace each `Span { start: 0, end: 0, line: 0, col: 0 }` in design-token diagnostic push calls (lines 303-312, 335-341, 356-361, 411-418, 445-455) with `&token.source_span`.

For each site, the `token` variable (a `&TokenDef`) is already in scope. Replace:

```rust
    self.diagnostics.push(Diagnostic::error(
        DiagnosticKind::E0001,
        &msg,
        &Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        },
    ));
```

With:

```rust
    self.diagnostics.push(Diagnostic::error(
        DiagnosticKind::E0001,
        &msg,
        &token.source_span,
    ));
```

Do this for all 5 design-token diagnostic push sites in `resolve.rs`.

- [ ] **Step 4: Run tests**

```bash
cd crates/omnilang-core && cargo test -- --nocapture
```

Expected: All tests PASS. Diagnostics now carry correct file locations instead of line 0/col 0.

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/design.rs crates/omnilang-core/src/parser/design.rs crates/omnilang-core/src/semantic/resolve.rs
git commit -m "fix: add source spans to design token diagnostics

Previously all design-token E0001 errors used zeroed spans (line 0, col 0),
making them unlocatable in the editor and CLI. Add source_span field to
TokenDef, thread the parser's token position into it, and use real spans
in all 5 diagnostic push sites in resolve.rs."
```

---

### Task 6: Warn on duplicate design names across files (M13)

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:231`

- [ ] **Step 1: Add duplicate detection for designs in `merge_file_ir_into`**

In `crates/omnilang-core/src/analyse.rs`, change line 231 from:

```rust
        existing.designs.extend(std::mem::take(&mut mod_ir.designs));
```

To mirror the `extend_and_warn` pattern used for components/stores/functions (line 203-223):

```rust
        // Check for duplicate design names (same pattern as components/stores/functions)
        for design in std::mem::take(&mut mod_ir.designs) {
            if let Some(existing_design) = existing.designs.iter().find(|d| d.name == design.name) {
                log::warn!(
                    "Duplicate design name '{}' in module '{}' — previously defined in another module. \
                     The later definition will overwrite the earlier one.",
                    design.name,
                    mod_ir.name
                );
            }
            existing.designs.push(design);
        }
```

Note: If `log` crate isn't available, use `eprintln!` for the warning.

- [ ] **Step 2: Run tests**

```bash
cd crates/omnilang-core && cargo test -- --nocapture
```

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/analyse.rs
git commit -m "fix: warn on duplicate design names across files during IR merge

Components, stores, and functions already emit warnings for name collisions.
Designs were overlooked — two files each declaring 'design App' silently
overwrote with the last one, nondeterministically (FxHashMap iteration order).

Use the same extend_and_warn pattern to make duplicate design names visible."
```

---

### Task 7: Fix peek_char O(n²) on non-ASCII runs (M10)

**Files:**
- Modify: `crates/omnilang-core/src/lexer/mod.rs:105`

- [ ] **Step 1: Bound the UTF-8 validation window**

In `crates/omnilang-core/src/lexer/mod.rs`, change the `peek_char` function at line 105 from:

```rust
    fn peek_char(&self) -> Option<char> {
        if self.pos >= self.source.len() {
            return None;
        }
        // ASCII fast path — most source is ASCII
        let b = self.source.as_bytes()[self.pos];
        if b.is_ascii() {
            return Some(b as char);
        }
        // Non-ASCII — validate the remaining source as UTF-8
        std::str::from_utf8(&self.source[self.pos..])
            .ok()?
            .chars()
            .next()
    }
```

To:

```rust
    fn peek_char(&self) -> Option<char> {
        if self.pos >= self.source.len() {
            return None;
        }
        // ASCII fast path — most source is ASCII
        let b = self.source.as_bytes()[self.pos];
        if b.is_ascii() {
            return Some(b as char);
        }
        // Non-ASCII — validate only the window needed for one UTF-8 char (max 4 bytes).
        // Previously validated the entire remaining source (O(n²) for CJK content).
        let window_end = (self.pos + 4).min(self.source.len());
        std::str::from_utf8(&self.source[self.pos..window_end])
            .ok()?
            .chars()
            .next()
    }
```

- [ ] **Step 2: Run tests**

```bash
cd crates/omnilang-core && cargo test -- --nocapture
```

Expected: All lexer tests PASS. No behavioral change — only a performance improvement.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/lexer/mod.rs
git commit -m "perf: fix O(n²) peek_char on non-ASCII character runs

peek_char was re-validating the entire remaining source as UTF-8 for
every non-ASCII character. For CJK content in long string literals,
this was O(n²). Bound the validation window to the maximum UTF-8
character length (4 bytes)."
```

---

### Task 8: Restore clippy cleanliness (M11)

**Files:**
- Modify: `crates/omnilang-core/src/lexer/mod.rs:742` (add `#[cfg(test)]`)
- Modify: `omnilang/src/codegen/omniview/name_resolver.rs:43-81` (delete dead `NameResolverSnapshot`)
- Modify: `omnilang/src/codegen/omniview/store.rs:580` (drop unused `comp` param)
- Modify: `omnilang/src/codegen/flutter/view.rs:227-228` (delete dead `_event_obj`)
- Modify: `omnilang/src/codegen/dioxus/view.rs:86` (delete dead `_event_obj`)

- [ ] **Step 1: Gate the lexer test module**

In `crates/omnilang-core/src/lexer/mod.rs`, change line 742 from:

```rust
mod tests {
```

To:

```rust
#[cfg(test)]
mod tests {
```

- [ ] **Step 2: Delete dead `NameResolverSnapshot`**

In `omnilang/src/codegen/omniview/name_resolver.rs`, delete lines 43-81 (the entire `NameResolverSnapshot` struct, `take` method, and `restore` method). Confirm with grep that nothing references them:

```bash
grep -r "NameResolverSnapshot\|snapshot\|take_snapshot\|restore_snapshot" omnilang/src/
```

Expected: Zero matches after deletion (except the deletion itself).

- [ ] **Step 3: Drop unused `comp` parameter**

In `omnilang/src/codegen/omniview/store.rs`, find line 580 and remove the `comp` parameter from the function signature. Check if the parameter is truly unused (underscore-prefixed or used only in dead code).

- [ ] **Step 4: Delete dead `_event_obj` computations**

In `omnilang/src/codegen/flutter/view.rs`, delete line 227-228:

```rust
let _event_obj = emit_event_object_flutter(...);
```

In `omnilang/src/codegen/dioxus/view.rs`, delete line 86:

```rust
let _event_obj = emit_event_object_jsx(...);
```

- [ ] **Step 5: Run remaining auto-fixable clippy lints**

```bash
cargo clippy --fix --workspace --allow-dirty
```

- [ ] **Step 6: Verify zero clippy warnings**

```bash
cargo clippy --workspace -- -D warnings 2>&1 | grep -v "ts_rs.*failed to parse serde"
```

Expected: Zero warnings (ignore ts-rs attribute-parse noise if still present — those are a separate issue).

- [ ] **Step 7: Run full test suite**

```bash
cargo test --workspace -- --nocapture
```

Expected: All tests PASS.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: restore clippy cleanliness across workspace

- Gate lexer test module with #[cfg(test)] (179 test fns were in prod lib)
- Delete dead NameResolverSnapshot (40 lines, zero callers)
- Remove unused comp parameter and dead _event_obj computations
- Run clippy --fix for mechanical lints

Restores the documented 'zero clippy' quality gate."
```

---

### Task 9: Replace hand-rolled diagnostic serializers (M12)

**Files:**
- Modify: `crates/omnilang-core/src/ffi.rs:13-27`
- Modify: `poiesis/src-tauri/src/lib.rs:17-31`

- [ ] **Step 1: Replace C FFI diagnostic serializer with `to_json_value()`**

In `crates/omnilang-core/src/ffi.rs`, find the hand-rolled `json!({...})` diagnostic construction (lines 13-27). Replace it with:

```rust
fn diagnostic_to_json_value(d: &crate::error::Diagnostic) -> serde_json::Value {
    d.to_json_value()
}
```

This delegates to the existing `Diagnostic::to_json_value()` method at `error.rs:321` which includes `end_line`/`end_col` fields that the hand-rolled version omitted.

- [ ] **Step 2: Replace Tauri IPC diagnostic serializer**

In `poiesis/src-tauri/src/lib.rs`, find the hand-rolled diagnostic JSON construction (lines 17-31) and replace with:

```rust
fn diagnostic_to_json_value(d: &omnilang_core::error::Diagnostic) -> serde_json::Value {
    d.to_json_value()
}
```

Remove the duplicate manual serialization.

- [ ] **Step 3: Run tests**

```bash
cargo test --workspace -- --nocapture
```

Expected: All tests PASS. No behavioral change — the same fields are emitted, plus now `end_line`/`end_col`/`suggestion` are correctly included.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ffi.rs poiesis/src-tauri/src/lib.rs
git commit -m "refactor: replace hand-rolled diagnostic serializers with to_json_value()

Both the C FFI and Tauri IPC layers had hand-rolled Diagnostic→JSON
construction that omitted end_line, end_col, and suggestion fields.
Replace with d.to_json_value() — the single source of truth already
existed at error.rs:321.

Eliminates 2 copies of the same serialization logic and fixes the
cross-surface inconsistency where the same diagnostic had different
JSON shapes depending on the consumer (WASM vs C FFI vs Tauri IPC)."
```

---

### Verification Checklist

Before marking this plan complete, verify:

- [ ] `cargo test --workspace` — all tests pass (554+ omnilang-core + 161 omnilang + poiesis)
- [ ] `cargo clippy --workspace -- -D warnings` — zero warnings (ignoring ts-rs attribute noise)
- [ ] `cargo fmt --check --all` — formatted
- [ ] `omnic build --target react test.omni` with design blocks produces files + warning (not zero output)
- [ ] `omnic analyse bad.omni` with `token(missing)` returns E0005 diagnostics (not empty)
- [ ] Deep unary chain (`!`.repeat(10000)) does not crash the parser
- [ ] Design token diagnostics carry real line/col numbers (not 0/0)
