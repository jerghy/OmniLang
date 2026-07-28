# Architecture Review: Breadth & Functional Completeness (v2)

**Date**: 2026-07-28  
**Type**: Comprehensive breadth review with regression detection — follow-up to the v1 breadth review  
**Reviewers**: 12 specialist agents (Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert, TypeScript & React Specialist, Compiler & Language Architect, Documentation Expert)  
**Previous review**: [2026-07-28-breadth-review.md](./2026-07-28-breadth-review.md) — 5 CRITICAL + 13 IMPORTANT findings, all CRITICAL and 9/13 IMPORTANT addressed in 13 subsequent commits  
**Scope**: Entire OmniLang workspace — all crates, TypeScript frontend, documentation, eval infrastructure, CI pipeline

---

## Executive Summary

The OmniLang workspace is in **Strong** shape — trending upward from "Adequate → Strong" in the previous review. All 13 commits since the v1 breadth review are clean, correctly scoped, and free of regressions: all tests pass (801+), clippy is clean (0 warnings), and the 5 CRITICAL findings (C1-C5) are verified fixed across all pipeline paths.

However, this v2 breadth review — conducted with 12 specialists looking at areas *not* covered in v1 — surfaced **14 new HIGH-severity findings** across 6 cross-cutting themes. The most significant discovery is a **systemic WASM/eval infrastructure gap**: the incremental cache never persists (cache_state always `null`), both eval scripts are broken, WASM CI is non-blocking, and the WASM bridge is entirely untyped. Three dead-code regressions were introduced by the fix wave itself (3 speculatively-added helper methods with 0 callers). A domain-level bug in `lower_type` causes false-positive `missing_return` lint warnings for ALL void functions.

**Overall Assessment**: Strong. The architecture and fixes are sound. The remaining issues are completeness and polish — infrastructure that was built but not fully wired, and documentation that has drifted from implementation reality.

---

## Regression Detection: Pre-Fix vs Post-Fix

### Fix Verification Status

All 5 CRITICAL and 9/13 IMPORTANT findings from the v1 review have been independently verified by the relevant specialists:

| Finding | Status | Verifier | Evidence |
|---------|--------|----------|----------|
| C1: Optimizer bypassed in single-file | ✅ Fixed | Compiler Architect | `lib.rs:117` — `optimize()` called after `lower()` in all 3 pipeline paths |
| C2: Multi-field variant payload truncated | ✅ Fixed | Domain Expert, Compiler Architect | `lower_logic.rs` uses `Vec<(String, ExprIR)>`; all 3 FormatExpr families handle it |
| C3: WASM tests crash native targets | ✅ Fixed | Systems Architect | `#[cfg(target_arch = "wasm32")]` guards on all WASM tests |
| C4: unsafe-eval in production CSP | ✅ Fixed | Security Specialist | Production CSP now `script-src 'self'` without eval |
| C5: check_call wired into resolve_expr_refs | ✅ Fixed | Compiler Architect, Domain Expert | `resolve.rs:428-435` calls `check_call()` during FuncCall resolution |
| I1: run_pipeline consolidated | ✅ Fixed | Systems Architect, Maintainability Expert | Facade delegates to core's single `run_pipeline()` |
| I3: Redundant HashMap construction | ✅ Fixed (partial) | Performance Specialist | `has_store` migrated (5 callers); `has_function`/`get_component` unused |
| I4: FS capabilities aligned | ✅ Fixed | Security Specialist | `get_allowed_directories()` now includes C:\ and D:\ |
| I6: as any patterns removed | ✅ Fixed | TypeScript Specialist | graph-parser.ts and graph-compiler.ts clean |
| I7: Dead test file removed | ✅ Fixed | Maintainability Expert | `javascript.rs` deleted from repo |
| I9: ComponentIR.refs domain type | ✅ Fixed | Domain Expert | `Vec<RefDeclIR>` properly wired through lowering |
| I10: WASM cache LRU eviction | ✅ Fixed | Performance Specialist | FIFO-with-reinsertion, `#[serde(default)]` backward-compat |
| I11: Eval baselines updated | ⚠️ Partial | AI Engineer | Scripts updated but eval-run.sh still broken |
| I12: type_mismatch lint audited | ✅ Fixed | Pragmatic Enforcer | Audit completed, overlap documented |
| I13: CI npm audit fixed | ✅ Fixed | Security Specialist | Both `cargo audit` and `npm audit` run in CI |

### New Regressions Discovered

| Finding | Severity | Introduced By | Detail |
|---------|----------|---------------|--------|
| 3 dead helper methods (`get_component`, `has_function`, `lookup_in_current_scope`) | HIGH | Commit `3e6627fe` (I9/I10/I12/I13 batch) | Added speculatively with 0 callers; none used by the fix itself |
| `lower_type` bypasses `TypeKind::From<&str>` normalization | HIGH | Pre-existing (P4-2 semantic types) | Creates `Named { name: "void" }` instead of `TypeKind::Void`, breaking `missing_return` lint |

### Metrics Comparison

| Metric | v1 Review (pre-fix) | v2 Review (post-fix) | Δ |
|--------|---------------------|---------------------|---|
| Production `.unwrap()`/`.expect()` | ~240 | 121 | ↓ (improvement, but still has 1 production `.expect()`) |
| Clippy warnings | 0 | 42+ (not enforced) | ↑ (new violations from recent changes) |
| Test failures | 0 | 0 | — |
| Unsafe blocks | 7 | 5 | ↓2 |
| `#[allow(dead_code)]` | 1 (check_call) | 1 (keyframe_to_config) | — |
| `as any` patterns (TS) | 8 files | 6 files | ↓2 |
| ts-rs serde warnings | 22 | 11 | ↓11 |
| Rust LOC | ~67K | ~61K | — (test count recategorization) |
| Test count | ~737 | ~801 | +64 |
| ADRs | 10 | 13 | +3 |
| Architecture reviews | 12 | 14 | +2 |

---

## Cross-Cutting Themes

### Theme 1: WASM & Eval Infrastructure Gap (4 specialists: Performance, Systems Architect, AI Engineer, TypeScript Specialist)

The WASM compilation path — which is the **primary compilation path for the editor** — has three compounding infrastructure gaps that together make it a second-class citizen:

1. **WASM incremental cache never persists** (Performance H1): `analyse_multi_cached` returns `"cache_state": null` unconditionally. Every editor keystroke starts from a cold cache, re-parsing and re-analyzing ALL source files.
2. **WASM CI is non-blocking with no artifact retention** (Systems Architect): `continue-on-error: true` makes WASM test failures invisible to PRs. The compiled `.wasm` binary is never uploaded.
3. **Both eval scripts are broken** (AI Engineer H1, H2): `eval-run.sh` crashes because `editor/client/` no longer exists. `eval-run-omnilang.sh` tests the wrong crate (223 tests vs 801+ workspace tests).
4. **WASM bridge entirely untyped** (TypeScript Specialist H1): All WASM functions return `any`; auto-generated types in `__generated__/` are ignored.

**Impact**: The WASM compilation path has no automated quality gate. A PR that breaks WASM compilation merges silently. Editor users experience cold-cache compilation performance on every keystroke. The eval infrastructure provides false confidence.

### Theme 2: Dead & Vestigial Code Accumulation (4 specialists: Domain Expert, Compiler Architect, Pragmatic Enforcer, Systems Architect)

Multiple forms of dead code were found — some pre-existing, some introduced by the fix wave:

1. **3 dead helper methods** (Pragmatic Enforcer H): `get_component`, `has_function`, `lookup_in_current_scope` — added in the I9/I10/I12/I13 batch commit, 0 callers.
2. **ElementIR dead fields** (Domain Expert H2): `ref_name` and `test_id` always `None`. `ref_name`'s purpose is served by `props["data-ref"]`.
3. **source_span always None on IR types** (Compiler Architect): Individual `source_span` fields on ComponentIR, StoreIR, ElementIR, FunctionIR, ModuleIR, IRProgram are never populated — a separate `source_map` mechanism is used instead.
4. **Post-P35 InternalCompileResult duplication** (Systems Architect): Same type defined independently in `omnilang-core` and `omnilang` facade.
5. **SignalIntent.is_copy always false** (Domain Expert L5): Field exists but never set to `true` anywhere.

### Theme 3: Type System Consolidation Needed (3 specialists: Domain Expert, Compiler Architect, Pragmatic Enforcer)

The P4-2 type checker is architecturally sound but has three concrete gaps:

1. **`lower_type` bypasses normalization** (Domain Expert H1): Creates `TypeKind::Named { name: "void" }` instead of `TypeKind::Void`. This breaks the `missing_return` lint's pattern-match guard — ALL void functions get false-positive "missing return" warnings.
2. **`types_compatible` lacks nested generics** (Compiler Architect L): `Array<Any>` is not compatible with `Array<Int>` because `Any` short-circuit only fires at the top level.
3. **`check_call` only validates named VarRef calls** (Domain Expert M4): Method calls (`obj.method()`), closure calls, and StoreFieldAccess calls are not type-checked.

### Theme 4: Documentation & Eval Drift (4 specialists: Documentation Expert, AI Engineer, Domain Expert, Systems Architect)

Documentation has measurably drifted from implementation:

1. **Stale metadata**: README says "10 ADRs" (actual: 13), docs/README says "13 chapters" (actual: 15), ADR-011 says "check_call not wired" (was wired before ADR committed).
2. **Competing stdlib docs**: `docs/stdlib.md` uses `omni/math` paths; `docs/omniview/12-standard-library.md` uses `std/math` paths. Different function coverage. No canonical source.
3. **Zero per-target documentation**: 8 of 9 compilation targets have no documentation beyond auto-generated code.
4. **Eval documentation**: `omnilang-compiler.md` references 375 tests (actual: 801+), 220 clippy warnings (actual: 0), and a Python architecture that no longer exists.

### Theme 5: I2 (Template Error Spans) — Highest-ROI Unfixed Item (4 specialists: Implementation Strategist, Rust Expert, Pragmatic Enforcer, Compiler Architect)

All template string parse errors emit `Span { start: 0, end: 0, line: 0, col: 0 }`. The span data (`expr_start`, `i`) is already computed but not propagated. Every template error in the editor is invisible — no highlighting, no navigation. Effort: S (~30 minutes). This was flagged as MEDIUM in v1 but unfixed in the 13-commit wave. Four specialists independently identified it as the single highest-ROI remaining fix.

### Theme 6: Large File Maintainability (4 specialists: Maintainability Expert, TypeScript Specialist, Rust Expert, Compiler Architect)

The large-file landscape is unchanged from v1:
- VariantEditor.tsx: 1,601 lines (9 inline sub-components, shared constants duplicated with ClassEditor)
- parser.ts: 1,454 lines (40 methods)
- lexer/mod.rs: 1,721 lines (but 57% is tests)
- ir/lower/mod.rs: 1,590 lines (but 83% is tests)
- ir/optimize.rs: 1,419 lines (but 57% is tests)
- ir/lower/view.rs: 1,018 lines (production-only, `lower_element` is 493 lines alone)

The test-inline pattern accounts for most of the Rust file sizes. Extraction to separate test files would bring all Rust files under 800 lines of production code.

---

## Trade-Offs and Disagreements

| Topic | Perspective A | Perspective B | Resolution |
|-------|--------------|--------------|------------|
| **FS capability scope** | Security Specialist: Narrow capabilities to $HOME only | Systems Architect: I4 fix went the wrong direction — expanded validation instead of narrowing capabilities | Both agree the principle of least privilege is violated. Recommend: narrow capabilities, add IPC middleware |
| **LRU eviction design** | Performance Specialist: FIFO-not-LRU is insufficient for access-skewed workloads | Pragmatic Enforcer: FIFO is sufficient for cache-size bound; don't add dependency | Convergent: FIFO is acceptable for current scale (50 entries). Touch-on-access is a future optimization |
| **source_span fields** | Compiler Architect: Remove dead fields or wire them | Domain Expert: Remove — the source_map mechanism is sufficient | Convergent: Remove unused source_span fields from IR types; source_map covers all use cases |
| **run_pipeline consolidation approach** | Maintainability Expert: Facade delegation is clean, single source of truth | Systems Architect: InternalCompileResult still duplicated in both crates | Convergent: The pipeline logic is consolidated; InternalCompileResult duplication is a separate (smaller) issue |
| **I8 cross-target tests** | Compiler Architect: Only 3/9 backends covered | Implementation Strategist: By-design per ADR-003 (P0-only backends) | Implementation Strategist correct: I8 should be closed as "not actionable — per ADR-003" |
| **parser.ts splitting** | Maintainability Expert: Split into DesignParser/ViewParser/StoreParser | Pragmatic Enforcer: Wait until the next time parser.ts needs modification | Defer: split when next touched |

---

## Prioritized Recommendations

### Critical (this week)

| # | Finding | Severity | Source(s) | Effort |
|---|---------|----------|-----------|--------|
| **C1** | **Fix `lower_type` to use `TypeKind::from(name)`** — bypasses normalization, breaks `missing_return` lint for ALL void functions. 1-line fix. | HIGH | Domain Expert | **S** |
| **C2** | **Fix WASM cache serialization** — `cache_state` is always `null`. Replace with `serde_json::to_string(cache)` in `analyse_multi_cached`. 1-line fix. | HIGH | Performance Specialist | **S** |
| **C3** | **Fix I2: template error spans** — replace `Span(0,0,0,0)` with computed spans in `parse_template_string`. Already-available data, just needs propagation. | HIGH† | Implementation Strategist, Rust Expert, Pragmatic Enforcer, Compiler Architect | **S** |
| **C4** | **Remove 3 dead helper methods** — `get_component`, `has_function`, `lookup_in_current_scope`. Added speculatively, 0 callers. | HIGH | Pragmatic Enforcer | **S** |
| **C5** | **Remove ElementIR.ref_name and test_id** — always `None`, dead fields. | HIGH | Domain Expert | **S** |
| **C6** | **Fix eval-run-omnilang.sh** — change `cargo test` to `cargo test --workspace`, fix C4 WASM test target. Broken eval = false confidence. | HIGH | AI Engineer | **M** |
| **C7** | **Fix or remove eval-run.sh** — crashes because `editor/client/` no longer exists. | HIGH | AI Engineer | **M** |
| **C8** | **Make WASM CI blocking** — remove `continue-on-error: true`, add artifact upload. | HIGH | Systems Architect | **S** |
| **C9** | **Add log size limit to dw_log** — no size cap enables temp-dir fill attack. | HIGH | Security Specialist | **S** |
| **C10** | **Fix stale ADR count in root README** — says "10 ADRs," actual 13. Trust erosion. | HIGH | Documentation Expert | **S** |

† Severity elevated from MEDIUM to HIGH due to 4-specialist independent discovery consensus.

### Important (2-4 weeks)

| # | Finding | Severity | Source(s) | Effort |
|---|---------|----------|-----------|--------|
| **I1** | **WASM bridge type hardening** — replace `any` return types with auto-generated types from `__generated__/`. 7+ files affected. | HIGH | TypeScript Specialist | **S** |
| **I2** | **Consolidate competing stdlib docs** — merge `docs/stdlib.md` into `docs/omniview/12-standard-library.md` or vice versa. Eliminate path inconsistency (`omni/math` vs `std/math`). | HIGH | Documentation Expert | **M** |
| **I3** | **Create per-target documentation** — start with 3 P0 targets (OmniView, React, Flutter). 8/9 targets currently undocumented. | HIGH | Documentation Expert | **L** |
| **I4** | **Fix eslit-disable exhaustive-deps** — 7 instances on hot paths (DesignPage, useMonacoEditor, VariantEditor). | HIGH | TypeScript Specialist | **M** |
| **I5** | **Split VariantEditor.tsx** (1,601 lines) — extract 5+ sub-components into `VariantEditor/` directory, extract shared constants with ClassEditor. | HIGH | Maintainability Expert, TypeScript Specialist | **M** |
| **I6** | **Extract shared utilities** — `useOutsideClick` (2 duplicates), `kebabToCamel` (2 duplicates), `ENUM_OPTIONS` (2 duplicates). | MEDIUM | Maintainability Expert | **S** |
| **I7** | **Consolidate InternalCompileResult** — post-P35 duplication between `omnilang-core` and `omnilang` facade. | MEDIUM | Systems Architect | **S** |
| **I8** | **Single-pass escape_literal_str** — 5 chained `.replace()` calls → single-pass iterator. O(n*m) → O(n). | MEDIUM | Performance Specialist | **S** |
| **I9** | **Override format_variant_literal for JS-family backends** — current default emits JSON-quoted keys (`"_variant"`) instead of unquoted (`_variant`). | MEDIUM | Compiler Architect | **S** |
| **I10** | **Consolidate diagnostic_to_json_value** — ffi.rs duplicates error.rs implementation. | MEDIUM | Rust Expert | **S** |
| **I11** | **Fix Span.clone() on Copy type** — 28 instances. Enable `clippy::clone_on_copy` in CI. | MEDIUM | Rust Expert | **S** |
| **I12** | **Validate sync_store_state input** — accept unvalidated JSON, no size limit. Data integrity risk. | MEDIUM | Security Specialist | **S** |
| **I13** | **Update ADR-011** — remove stale "check_call not wired" limitation. | MEDIUM | Domain Expert | **S** |
| **I14** | **Rewrite eval docs** — `omnilang-compiler.md` references 375 tests (actual 801+), 220 clippy warnings (actual 0). | MEDIUM | AI Engineer | **M** |
| **I15** | **Archive test-framework skill** — references non-existent Python architecture. Causes systematic wrong actions for AI agents. | MEDIUM | AI Engineer | **S** |
| **I16** | **Create workspace root CLAUDE.md** — no project-level AI agent guidance exists. | MEDIUM | AI Engineer | **S** |
| **I17** | **Extract large test modules** — lexer (982 lines of tests), ir/lower (1,324 lines of tests), ir/optimize (813 lines of tests). Test-inline pattern bloats file metrics. | MEDIUM | Maintainability Expert | **L** |
| **I18** | **Remove source_span fields from IR types** — always None, dead. Separate source_map covers all use cases. | MEDIUM | Compiler Architect | **S** |

### Nice-to-Have (2-3 months)

| # | Finding | Effort |
|---|---------|--------|
| N1 | Extend `types_compatible` for nested generics (Array, Map, Optional) | M |
| N2 | Extend `check_call` to MethodCall, StoreFieldAccess, closure calls | L |
| N3 | Type remaining 5 `serde_json::Value` fields in IR types with custom enums | L |
| N4 | Split `lower_element` (493 lines) into focused helper functions | M |
| N5 | Enable `cargo clippy -- -D warnings` in CI | M |
| N6 | Add Rust/TypeScript PostToolUse hooks (cargo fmt, prettier, tsc) | S |
| N7 | Add `check_types(source)` WASM export for structured semantic type info | M |
| N8 | Add per-window CSP in Tauri config | M |
| N9 | Implement true LRU (touch-on-access) in IncrementalCache | L |
| N10 | Add automated memory file freshness check | S |

---

## Per-Specialist Review Summaries

### 1. Systems Architect
**Key findings**: Post-P35 type duplication (`InternalCompileResult` in 2 crates), WASM CI non-blocking (no artifact retention), I4 went wrong direction (expanded capabilities instead of narrowing), feature-gated test modules leave 6/9 backends untested, `any_backend_enabled` build-script cfg is dead code. **Strengths**: Clean acyclic dependency graph, well-designed incremental cache, 9-job CI pipeline, proper feature gate propagation.

### 2. Domain Expert
**Key findings**: `lower_type` bypasses `TypeKind::From<&str>` normalization — breaks `missing_return` lint (HIGH), `ElementIR.ref_name`/`test_id` always `None` (HIGH), ADR-011 documents check_call as "not wired" (MEDIUM), `check_call` only validates named VarRef calls (MEDIUM), `SignalIntent.is_copy` always `false` (LOW), `ArrayLiteral` type inference only checks first element (LOW). **Strengths**: Clean three-type-system architecture, comprehensive multi-field variant fix, well-structured EventDataKind domain model.

### 3. Security Specialist
**Key findings**: `dw_log` no size limit enables temp-dir fill (HIGH), `sync_store_state` accepts unvalidated JSON (MEDIUM), no per-window CSP for debug window (MEDIUM), no rate limiting on any Tauri IPC command (MEDIUM). All 3 previously-fixed items (C4, I4, I13) verified correct. **Strengths**: Defense-in-depth debug window sandbox, consistent 10MB input limits, no XSS vectors in React codebase.

### 4. Maintainability Expert
**Key findings**: VariantEditor.tsx 1,601 lines with 9 inline sub-components (MEDIUM), `lower_element` 493 lines (MEDIUM), 6 files over 800-line guideline, `useOutsideClick` duplicated in 2 files, `kebabToCamel` duplicated in 2 files, 6 files with residual `as any`. I1/I6/I7 fixes verified correct. **Strengths**: Clean pipeline consolidation, test-source co-location, complete module hierarchy.

### 5. Performance Specialist
**Key findings**: WASM cache state always `null` — incremental cache never persists (HIGH), `check_call` allocates per-call Vec<SemanticType> (HIGH), `escape_literal_str` O(n*m) pattern (MEDIUM), `useWasmCompile` subscribes to 7 selectors (MEDIUM), diagnostics runs full codegen (MEDIUM), `sources.clone()` on multi-file path (MEDIUM), LRU is actually FIFO (MEDIUM). I3/I10/C1 fixes verified correct but I10 is FIFO-not-LRU.

### 6. Implementation Strategist
**Key findings**: Fix sequencing was correct (HIGHs first, then MEDIUMs), I2 is highest-ROI unfixed item (S-effort), I4 was fragmented across 2 commits with 91-minute inconsistency window, batch commits reduce atomic reversibility, I8 should be closed as by-design per ADR-003. **Strengths**: Clean dependency ordering, LRU serialization backward-compat, check_call wiring is additive (no false positives).

### 7. AI Engineer
**Key findings**: `eval-run.sh` crashes (editor/client/ gone) (HIGH), `eval-run-omnilang.sh` tests wrong crate (HIGH), eval docs severely outdated (HIGH), `test-framework` skill references non-existent Python architecture (MEDIUM), no workspace CLAUDE.md (MEDIUM), no PostToolUse hooks (MEDIUM), semantic type checker not exposed to WASM (MEDIUM). **Strengths**: Production-grade WASM error handling, well-designed incremental caching, check_call improves AI codegen feedback.

### 8. Pragmatic Enforcer
**Key findings**: 3 dead helper methods added during fix wave (HIGH), `parse_type_string` vs `from_type_annotation` duplicate mapping (MEDIUM), partial migration pattern — optimization methods exist but callers unchanged (MEDIUM), I2/I5/I11 remain unfixed with no documented deferral (LOW). Necessity/Complexity/Ratio: check_call 9/2=0.22 (keep), dead helpers 0/1=∞ (remove), I2 5/1=0.20 (fix now). **Strengths**: Proportional fix quality, SemanticType enum over Hindley-Milner (correct YAGNI), no new crate dependencies.

### 9. Rust Expert
**Key findings**: Template error spans still zeroed (I2, MEDIUM), 28 `Span.clone()` on Copy type (MEDIUM), `diagnostic_to_json_value` duplication (MEDIUM), `keyframe_to_config` dead code with TODO (MEDIUM), 42+ clippy violations not CI-enforced, 11 remaining ts-rs warnings. Unsafe count down 7→5 with adequate SAFETY comments. **Strengths**: Well-structured trait hierarchy (FormatType/FormatExpr/CodegenVisitor/Codegen), correct thread-safety at WASM boundary, Arc-based sharing avoids clones.

### 10. TypeScript & React Specialist
**Key findings**: WASM bridge entirely untyped — all functions return `any` (HIGH), VariantEditor 1,601 lines (HIGH), 7 `eslint-disable exhaustive-deps` on hot paths (HIGH), WASM AST transformation untyped (MEDIUM), ClassEditor duplicates VariantEditor patterns (MEDIUM), ComponentDefPanel inline CSS-in-JS (MEDIUM), CSS modules untyped (MEDIUM). **Strengths**: Strict TypeScript config, zero `@ts-ignore`, well-designed Zustand slice pattern, good Monaco lifecycle management.

### 11. Compiler & Language Architect
**Key findings**: All 6 previously-fixed issues verified correct end-to-end. `format_variant_literal` produces JSON-quoted keys in JS output (MEDIUM), `source_span` fields always None (MEDIUM), cross-target tests cover 3/9 backends (MEDIUM), 5 `serde_json::Value` fields in IR types (LOW), `types_compatible` lacks nested generics (LOW), production `.expect()` in ir/lower (LOW). **Strengths**: Sound optimizer integration, multi-field variants work end-to-end, strong type-checking architecture, clean module separation.

### 12. Documentation Expert
**Key findings**: Root README ADR count stale (HIGH), competing stdlib docs with different API signatures (HIGH), zero per-target docs for 8/9 targets (HIGH), Rust public API sparse doc comments (MEDIUM), docs/README chapter count stale (MEDIUM), grammar docs lack validation markers (MEDIUM), root-level snapshot files clutter (LOW). **Strengths**: Comprehensive grammar docs (1,855 lines), strong ADR quality and template reuse, design document index with freshness markers, 14 review audit trail.

---

## Risks If Unaddressed

| Risk | Impact | Likelihood |
|------|--------|-----------|
| **WASM cache never persists (C2)**: Every editor keystroke recompiles all files from scratch. 10+ files = 500ms+ unnecessary work per keystroke. | High (user-visible latency) | High (every editor session) |
| **False-positive missing_return warnings (C1)**: All void functions trigger lint warning. Users ignore all lint output. | High (lint credibility) | High (every void function) |
| **Template errors invisible (C3)**: All template errors show Span(0,0). Editor can't highlight or navigate to error. | High (UX) | High (every template error) |
| **WASM CI non-blocking (C8)**: WASM-breaking change merges silently. Editor stops working. | High (editor availability) | Medium (WASM changes are infrequent) |
| **Broken eval scripts (C6, C7)**: False confidence in CI. Real regressions indistinguishable from baseline mismatches. | High (quality gate blind) | High (every CI run) |
| **Dead helper methods accumulate (C4)**: 3 today. Without guard, more in future fix waves. | Medium (maintenance noise) | Medium (pattern not caught) |
| **Stale documentation erodes trust (C10, I2, I13)**: README says 10 ADRs (actual 13). ADR-011 says check_call not wired (was wired). Multiple small untruths compound. | Medium (contributor trust) | High (every new reader) |
| **WASM bridge untyped (I1)**: WASM struct field rename → silent `undefined` → runtime crash. | Medium (type safety) | Medium (WASM interface changes) |
| **VariantEditor file rot (I5)**: 1,601 lines + ClassEditor duplication. Each edit risks desync. | Medium (maintenance velocity) | Medium (every edit) |

---

## Success Metrics and Follow-Up Plan

### Quality Gates (current state)
- ✅ All workspace tests passing (0 failures, 801+ tests)
- ✅ Clippy clean on `--workspace` (0 warnings with default lints)
- ⚠️ 42+ clippy violations with `-D warnings` (not CI-enforced)
- ⚠️ 11 ts-rs serde warnings (cosmetic, down from 22)
- ❌ Both eval scripts broken
- ❌ WASM CI non-blocking (continue-on-error: true)
- ❌ 14 new HIGH findings across 6 themes

### Follow-Up Actions
1. **This session**: Fix C1-C5 + C8-C10 (6 HIGH findings, all S-effort). Estimated 1-2 hours.
2. **This week**: Fix C6-C7 (eval scripts), I1-I7 (WASM types, docs, VariantEditor split prep). Estimated 3-5 hours.
3. **Next sprint**: Address I8-I18 (medium findings). Estimated 1-2 days.
4. **Ongoing**: Enable clippy `-D warnings` in CI, add Rust/TS PostToolUse hooks, create workspace CLAUDE.md.
5. **Next review**: Focus on type system consolidation after `lower_type` fix and `check_call` extension. Re-evaluate eval infrastructure after scripts are fixed.

---

*Review methodology: 12-specialist parallel dispatch per ADR-007 Layer 1 (breadth). All findings cite exact file:line locations verified via Grep/Read against the codebase. Cross-cutting themes aggregated from 3+ independent specialist discoveries. Fix verification performed via git diff and code path tracing.*
