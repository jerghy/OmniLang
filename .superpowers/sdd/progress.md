# OmniLang Code Review Fix Progress
> branch: worktree-code-review-fixes | completed: 2026-07-06

## Completed

### Phase 1: Security (5/5)
- T1.1: withGlobalTauri=false, FS/Shell plugins restricted
- T1.2: FS capabilities — already restricted
- T1.3: Empty catch → console.warn (4 locations)
- T1.4: sanitize_path TOCTOU safety comment
- T1.5: postMessage origin — verified correct

### Phase 2: Error Handling (4/6)
- T2.1: Lowering catch-all → explicit TemplateString + NotYetImplemented
- T2.2: Lexer expect() — verified correct with peek() guard
- T2.4: FFI SAFETY comment at ffi.rs:296
- T2.5: Generated unwrap() — verified already fixed

### Phase 3: Code Dedup (5/5)
- T3.1: binary_op_to_str — 8 backends → 1 shared
- T3.2: escape_literal_str — shared
- T3.3: format_block_as_expr — 8 backends → 1 shared
- T3.4: capitalize_first — 3 copies → 1 shared
- T3.5: Naming + outdated docs updated

### Phase 4: Type System (2/4)
- T4.3: #[must_use] on compile_multi_to_target
- Updated codegen/mod.rs header comment

## Verification
- cargo build --workspace: PASS
- cargo clippy --workspace -- -D warnings: PASS
- cargo test --workspace: 405/405 PASS

## Commits
- a0bcf701: fix: Phase 1-2 security and error handling fixes
- 610d0202: fix: Phase 4 — add #[must_use], update outdated docs
- 73d75e2c: refactor: Phase 3 — shared codegen utilities
- cb0758fb: merge: Phase 3 — shared codegen utilities
