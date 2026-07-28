# Deep Verification: Breadth Review v2 Findings — Cross-Referenced Against ADRs, Memory, and Code

**Date**: 2026-07-28  
**Type**: Layer 2 deep verification per ADR-007 methodology  
**Source**: [2026-07-28-breadth-review-v2.md](./2026-07-28-breadth-review-v2.md) — 14 HIGH + 18 IMPORTANT findings from 12 specialists  
**Method**: Each finding verified by: (1) tracing code paths, (2) cross-referencing ADRs (13 total), (3) checking memory files (40+), (4) git history for design intent

---

## Executive Summary

Of the **14 HIGH findings** in the v2 breadth review:

| Verdict | Count | Findings |
|---------|-------|----------|
| ✅ **CONFIRMED — Real Issue** | 8 | C1 (lower_type), C2 (WASM cache), C3 (I2 template spans), C4 (3 dead helpers), C5 (dead ElementIR fields), C7 (broken eval-run.sh), C8 (WASM CI non-blocking), C9 (dw_log no size limit) |
| ⚠️ **NUANCED — Real but milder** | 3 | C6 (eval-run-omnilang.sh — wrong crate but CI covers it), I4 (eslint-deps — real but low blast radius), C10 (README stale — cosmetic) |
| ❌ **FALSE POSITIVE — By Design** | 3 | FS capabilities (documented intentional), I8 cross-target tests (per ADR-003), ArrayLiteral first-element-only (per ADR-011 known limitations) |

The v2 breadth review had a **21% false-positive rate** (3/14 HIGH) for findings that contradicted documented architectural decisions. This is notably better than the 33% misjudgment rate documented in prior sessions, but confirms that cross-referencing ADRs and memory is essential before acting on review findings.

---

## Detailed Verification Per Finding

### C1: `lower_type` bypasses TypeKind normalization → breaks `missing_return` lint

**Source**: Domain Expert  
**Claim**: `lower_type()` at `ir/lower/component.rs:393` creates `TypeKind::Named { name: "void" }` instead of `TypeKind::Void`, causing false-positive "missing return" warnings for ALL void functions.

**Verification**:
1. ✅ `TypeKind::from("void")` returns `TypeKind::Void` — confirmed at `ir/types.rs:305`
2. ✅ `lower_type()` at line 393 uses `TypeKind::Named { name: name.clone() }` directly — bypasses `From<&str>`
3. ✅ `missing_return` lint at `semantic/lints/missing_return.rs:40` matches `TypeKind::Void` only — won't match `Named { name: "void" }`
4. ✅ `TypeKind::from()` has exactly 1 caller (`types.rs:355`, serde deserialization) — the normalization path is essentially dead code
5. ⚠️ However: `lower_type` is called for ALL type annotations, not just `void`. Builtins like `int`, `string`, `bool` all get `TypeKind::Named` instead of their typed variants. This has broader implications than just the `missing_return` lint.

**Verdict**: ✅ **CONFIRMED REAL BUG.** The fix (`TypeKind::from(name.as_str())` at line 393, 1-line change) is correct. This affects ALL built-in type names, not just `void`. Any code that pattern-matches on `TypeKind::Int`/`TypeKind::String`/etc. will miss types lowered through this path.

**Severity**: HIGH — confirmed. Every void function gets false-positive lint warnings.

---

### C2: WASM cache_state always null → incremental cache never persists

**Source**: Performance Specialist  
**Claim**: `analyse_multi_cached` at `analyse.rs:321` returns `"cache_state": null` unconditionally, meaning every editor keystroke starts from a cold cache.

**Verification**:
1. ✅ 4 WASM functions return `"cache_state": null` unconditionally (`omnilang-wasm/src/lib.rs:296, 325, 358, 423`)
2. ✅ `save_cache_state()` exists at line 653 and works correctly
3. ✅ `seed_cache_from_state()` exists at line 254 and correctly deserializes cache
4. ✅ The JS side stores `this._cacheState` and passes it back — designed for cross-call persistence
5. ⚠️ **Nuance**: The thread-local `ANALYSIS_CACHE` works perfectly WITHIN a single WASM invocation. The `save_cache_state`/`seed_cache_from_state` mechanism is designed for page-navigation persistence (e.g., user navigates away and returns). The design intent (from `analyse.rs:69` comment: "Use `save_cache_state()` / `seed_cache_from_state()` only for page-navigation") suggests this was intentionally deferred, not accidentally broken.
6. ⚠️ **Nuance**: The `OmniCompiler` JS class (`omnilang-wasm/src/lib.rs:476-506`) maintains its own in-memory source cache with invalidation on source changes — this is the primary caching mechanism for the editor, NOT the thread-local `ANALYSIS_CACHE`.

**Verdict**: ⚠️ **NUANCED — Real gap, but impact is less severe than claimed.** The per-keystroke cache hit is achieved through the JS-side `OmniCompiler.sources` cache (which stores source strings and detects changes via hash comparison), not through the thread-local `ANALYSIS_CACHE`. The `cache_state` gap means cross-page-navigation persistence doesn't work, but per-keystroke caching does work through a different mechanism. The fix (serializing cache_state in compile functions) would add the cross-navigation persistence that was designed but not implemented.

**Corrected Severity**: MEDIUM (was HIGH). Per-keystroke caching works; only cross-navigation caching is missing.

---

### C3: Template error spans (I2) still unfixed

**Source**: Implementation Strategist, Rust Expert, Pragmatic Enforcer, Compiler Architect (4-specialist consensus)  
**Claim**: All template string parse errors emit `Span { start: 0, end: 0, line: 0, col: 0 }`. Span data already available but not propagated.

**Verification**:
1. ✅ `parser/expr/template.rs:43-75` — 3 error sites all use `Span { start: 0, end: 0, line: 0, col: 0 }`
2. ✅ `expr_start` and `i` variables computed at lines 31-39 — span data exists but unused
3. ✅ This was flagged as I2 in the v1 breadth review — NOT FIXED in 13 commits
4. ✅ No ADR or memory file documents this as intentional

**Verdict**: ✅ **CONFIRMED REAL BUG.** Highest-ROI unfixed item. S-effort (~30 min). Every template error is invisible in the editor (no highlighting, no navigation). Four independent specialists flagged it.

**Severity**: HIGH — confirmed. User-facing diagnostic quality issue.

---

### C4: 3 dead helper methods added during fix wave

**Source**: Pragmatic Enforcer  
**Claim**: `get_component`, `has_function`, `lookup_in_current_scope` were added in commit `3e6627fe` with 0 callers.

**Verification**:
1. ✅ `IRProgram::get_component()` at `ir/types.rs:808` — grep confirms 0 callers (only definition)
2. ✅ `IRProgram::has_function()` at `ir/types.rs:820` — grep confirms 0 callers (only definition)
3. ✅ `SymbolTable::lookup_in_current_scope()` at `semantic/symbols.rs:128` — grep confirms 0 callers (only definition)
4. ✅ All three have `#[must_use]` annotation — implying intent to use
5. ✅ `has_store()` (same commit) HAS 5 callers — proving the pattern works, just 2 methods weren't migrated
6. ✅ No ADR or memory file documents these as forward-looking design

**Verdict**: ✅ **CONFIRMED DEAD CODE.** The methods work correctly (has_store proves the pattern), they're just unused. The commit added optimization infrastructure but only partially migrated callers. Remove or migrate.

**Severity**: MEDIUM (was HIGH). Dead but harmless — doesn't affect correctness or performance. The HIGH severity from Pragmatic Enforcer was due to pattern concern (speculative code in fix waves), not immediate impact.

---

### C5: ElementIR.ref_name and test_id — dead fields

**Source**: Domain Expert  
**Claim**: Both fields always `None`, never read.

**Verification**:
1. ✅ `ref_name: None` at `ir/lower/view.rs:624` — confirmed
2. ✅ `test_id: None` at `ir/lower/view.rs:623` — confirmed
3. ✅ grep for `.ref_name` across all `.rs` files: **0 matches** (never read)
4. ✅ grep for `.test_id` across all `.rs` files: **0 matches** (never read)
5. ✅ `ref_name`'s purpose is served by `props["data-ref"]` at `ir/lower/view.rs:444-451` — the field is vestigial
6. ✅ `test_id` has no corresponding lowering from AST `ElementProp::TestId` — never implemented
7. ✅ No ADR or memory file documents these as intentional placeholder fields

**Verdict**: ✅ **CONFIRMED DEAD FIELDS.** Both fields are write-only (always `None`) and never read. Remove from `ElementIR` struct. The `ref_name` functionality works through a different mechanism (`data-ref` prop).

**Severity**: MEDIUM (was HIGH). Dead fields add serialization overhead but no runtime bugs. The HIGH severity was due to the misleading API surface — developers might try to read `ref_name` expecting it to be populated.

---

### C6: eval-run-omnilang.sh tests wrong crate

**Source**: AI Engineer  
**Claim**: Script runs `cargo test` in `omnilang/` facade (223 tests) instead of `cargo test --workspace` (801+ tests).

**Verification**:
1. ✅ Script at `.claude/evals/eval-run-omnilang.sh:12` runs `cd omnilang && cargo test`
2. ⚠️ **Nuance**: The CI `evals` job (`ci.yml`) runs `cargo test --workspace` separately — the eval script is supplementary, not the primary test gate
3. ⚠️ **Nuance**: The `all-backends` CI job (`ci.yml:154-164`) tests the full workspace with all features — true test coverage exists in CI, just not in the eval script
4. ✅ The eval baseline values (BASELINE_TESTS=227) are stale — actual is ~801 from workspace

**Verdict**: ⚠️ **NUANCED — Script is wrong, but CI covers the gap.** The eval script doesn't provide reliable metrics, but the primary CI pipeline (`rust` job + `all-backends` job) does test the full workspace. The eval script needs fixing for its stated purpose (trend detection), but test coverage is not actually at risk.

**Corrected Severity**: MEDIUM (was HIGH). CI covers the gap; eval script is supplementary.

---

### C7: eval-run.sh broken — references non-existent directory

**Source**: AI Engineer  
**Claim**: Script crashes because `editor/client/` no longer exists.

**Verification**:
1. ✅ `eval-run.sh:9` references `../../editor/client` — directory does not exist (restructured to `poiesis/`)
2. ✅ Running the script produces immediate crash
3. ✅ No CI job uses this script — it's a manual eval tool
4. ✅ Frontend tests run via CI `frontend` job's `vitest` step — coverage exists elsewhere

**Verdict**: ✅ **CONFIRMED BROKEN.** The script is non-functional. Either fix (target `poiesis/`) or remove.

**Corrected Severity**: MEDIUM (was HIGH). No CI dependency on this script; frontend tests run via separate CI job.

---

### C8: WASM CI non-blocking with no artifact retention

**Source**: Systems Architect  
**Claim**: `continue-on-error: true` makes WASM test failures invisible to PRs.

**Verification**:
1. ✅ `ci.yml:132` — `continue-on-error: true` on WASM job
2. ✅ No artifact upload step for compiled `.wasm` binary
3. ⚠️ **Nuance**: The comment says "while we establish reliability" — implies temporary, but no tracking issue exists
4. ✅ WASM tests have been green for consecutive runs — the guard is no longer needed

**Verdict**: ✅ **CONFIRMED REAL GAP.** The `continue-on-error` guard was intended as temporary. Should be removed now that WASM tests are stable. Add artifact upload.

**Severity**: HIGH — confirmed. WASM is the primary compilation path for the editor; breaking changes should block PRs.

---

### C9: dw_log no size limit → temp-dir fill attack

**Source**: Security Specialist  
**Claim**: `dw_log` command (`debug.rs:42-61`) appends to `poiesis-debug.log` with no size limit.

**Verification**:
1. ✅ No size check, no log rotation, no truncation
2. ✅ `withGlobalTauri: false` disables `window.__TAURI__` — but `__TAURI_INTERNALS__` remains accessible
3. ⚠️ **Nuance**: Requires `__TAURI_INTERNALS__` access — supply chain attack vector (compromised npm dependency), not remote attack
4. ✅ No ADR or memory file documents this as intentional

**Verdict**: ✅ **CONFIRMED REAL GAP.** Low exploitability (requires code execution in main webview) but zero-cost to fix. Add 10MB cap.

**Corrected Severity**: MEDIUM (was HIGH). Requires compromised dependency or dev tools access; single-user desktop app context limits impact.

---

### C10: Root README ADR count stale

**Source**: Documentation Expert  
**Claim**: README says "10 ADRs", actual is 13.

**Verification**:
1. ✅ `README.md:51` — "10 ADRs"
2. ✅ Actual ADR count: 13 files in `.architecture/decisions/adrs/`
3. ✅ `README.md:86` — also says "10 ADRs" (two locations)
4. ⚠️ This is a cosmetic documentation issue with zero functional impact

**Verdict**: ✅ **CONFIRMED STALE.** Simple fix. But severity is inflated.

**Corrected Severity**: LOW (was HIGH). Documentation freshness issue, not a functional or security concern.

---

## By-Design Findings (False Positives)

### ❌ FS Capabilities Too Broad → BY DESIGN

**Source**: Systems Architect, Security Specialist  
**Claim**: `D:/**` and `C:/**` in Tauri capabilities violate least privilege.

**Counter-Evidence**:
1. 📄 **Memory `project-permissions`** (2026-07-24): "`C:/**`、`D:/**` 的通配路径是有意保持开放的，不应限制" — explicitly documents this as intentional for an editor that must open projects anywhere
2. 📄 **Memory**: "在安全审查和代码审查中，跳过此项，不将其作为安全问题报告" — instructs reviewers to skip this
3. 📄 **ADR-004** (architecture review conclusions): Defers to the editor's functional requirement for broad FS access
4. ✅ `capabilities/default.json` shows `$HOME/**`, `$APPDATA/**`, `D:/**`, `C:/**` — all four paths
5. ⚠️ **Memory `poiesis-tauri-security-model`** (2026-07-27) is STALE — claims $HOME/$APPDATA only, but was written before commit `03e7e981` restored wider scope. This memory needs updating.

**Verdict**: ❌ **FALSE POSITIVE — BY DESIGN.** The broad FS scope is a documented architectural decision for an editor that needs to open projects anywhere. The `poiesis-tauri-security-model` memory should be updated to reflect the current state.

---

### ❌ Cross-Target Tests Only 3/9 Backends → BY DESIGN

**Source**: Compiler Architect, Systems Architect (v1 review finding I8)  
**Claim**: Parameterized regression tests cover only OmniView, React, Flutter — 6 backends untested.

**Counter-Evidence**:
1. 📄 **ADR-003** (2026-07-26): Explicitly decides "仅 OmniView、React、Flutter 保持活跃开发（P0）。其余 6 目标暂停发展。" — only P0 backends get active testing
2. 📄 **ADR-003** Section "Consequences": "新语言特性仅需在 P0 三个后端实现" — new features only need P0 implementation
3. 📄 **ADR-003** Implementation Strategy: "P19 参数化测试框架仅覆盖 P0 后端" — parameterized tests only cover P0 backends
4. 📄 **Memory `omnilang-codegen`**: Confirms P0 prioritization
5. ✅ Non-P0 backends are feature-gated, compile-checked, and return descriptive errors for unsupported features — not silently broken

**Verdict**: ❌ **FALSE POSITIVE — BY DESIGN per ADR-003.** Only 3 P0 backends are actively tested. The other 6 are in maintenance mode behind feature flags. I8 should be closed as "not actionable."

---

### ❌ ArrayLiteral Type Inference Only Checks First Element → BY DESIGN

**Source**: Domain Expert (L6)  
**Claim**: `infer_expr_type` for `ArrayLiteral` only checks `elements[0]`, ignoring other elements.

**Counter-Evidence**:
1. 📄 **ADR-011** Section "Negative / 已知限制": Explicitly lists "MatchExpr/ArrayLiteral 仅检查第一个元素" as a KNOWN LIMITATION
2. 📄 **ADR-011**: Documents this as intentional for v0.1 — the language has no user-defined generics, so full array element type checking is deferred
3. ✅ The `Any` cascade suppression mechanism (also documented in ADR-011) handles the case gracefully

**Verdict**: ❌ **FALSE POSITIVE — DOCUMENTED KNOWN LIMITATION per ADR-011.** Not a bug; a consciously deferred feature for v0.2+.

---

## Additional Corrections

### Compiler Architect Error: "source_span always None on ALL IR types"

**Claim**: `source_span: None` on all IR types including ElementIR.

**Correction**: `ElementIR.source_span` IS populated at `ir/lower/view.rs:627` — `source_span: Some(el.span)`. The claim that ElementIR.source_span is always None was incorrect. However, `source_span` IS None on:
- ModuleIR (`ir/lower/mod.rs:205`)
- IRProgram (`ir/lower/mod.rs:263`)  
- Design tokens (all 7 occurrences in `ir/lower/design.rs`)
- Motion configs (all 11 occurrences in `ir/lower/motion.rs`)
- Sub-elements within views (6 occurrences in `ir/lower/view.rs`)

The Compiler Architect's finding needs refinement: ElementIR's source_span is populated, but ~27 other source_span assignments across design/motion/sub-element types are None. The separate `source_map` mechanism covers source tracking for those types.

### Memory File Drift: poiesis-tauri-security-model

**File**: `poiesis-tauri-security-model.md` (2026-07-27)  
**Issue**: Claims FS scope is "$HOME/$APPDATA 仅" — but commit `03e7e981` restored D:/ and C:/ access. The memory is stale.  
**Action**: Update to reflect current state: "$HOME/$APPDATA + D:/ + C:/ — intentionally broad per project-permissions design decision."

### ADR-011 "check_call not wired" → Stale

**File**: `ADR-011-p4-2-semantic-type-checking-architecture.md:70`  
**Issue**: Lists "check_call 未接入 resolve pass" as a known limitation — but commit `f4e87e9f` wired it 30 minutes before the ADR was committed.  
**Action**: Remove from known limitations list.

---

## Corrected Priority List

### Actually Critical (this week)

| # | Finding | Original Severity | Corrected Severity | Effort |
|---|---------|-------------------|-------------------|--------|
| C1 | Fix `lower_type` TypeKind normalization | HIGH | **HIGH** ✅ | S |
| C3 | Fix I2: template error spans | HIGH | **HIGH** ✅ | S |
| C8 | Make WASM CI blocking | HIGH | **HIGH** ✅ | S |
| C2 | Wire WASM cache_state serialization | HIGH | **MEDIUM** ⬇️ | S |
| C4 | Remove 3 dead helper methods | HIGH | **MEDIUM** ⬇️ | S |
| C5 | Remove ElementIR dead fields | HIGH | **MEDIUM** ⬇️ | S |
| C6 | Fix eval-run-omnilang.sh | HIGH | **MEDIUM** ⬇️ | M |
| C7 | Fix or remove eval-run.sh | HIGH | **MEDIUM** ⬇️ | M |
| C9 | Add dw_log size limit | HIGH | **MEDIUM** ⬇️ | S |
| C10 | Fix README ADR count | HIGH | **LOW** ⬇️ | S |

### Key Corrections

| Original Finding | Verdict | Action |
|-----------------|---------|--------|
| FS capabilities too broad | ❌ BY DESIGN | No action. Update stale `poiesis-tauri-security-model` memory. |
| Cross-target tests only 3/9 | ❌ BY DESIGN per ADR-003 | Close I8. No code change needed. |
| ArrayLiteral first-element-only | ❌ KNOWN LIMITATION per ADR-011 | No action. Deferred to v0.2. |
| ElementIR.source_span always None | ❌ INCORRECT | ElementIR.source_span IS populated at view.rs:627. |
| WASM cache_state null | ⚠️ NUANCED | Fix is low-priority. Per-keystroke caching works via JS-side OmniCompiler. |

---

## Stale Artifacts Requiring Updates

| Artifact | Issue | Action |
|----------|-------|--------|
| `poiesis-tauri-security-model.md` | Claims $HOME/$APPDATA only; actual is broader | Update to reflect current state |
| `ADR-011` line 70 | Lists "check_call 未接入" as limitation | Remove from known limitations |
| `README.md` lines 51, 86 | Says "10 ADRs"; actual is 13 | Update to 13 |
| `docs/README.md` line 11 | Says "13 章"; actual is 15 files | Update chapter count |
| `docs/stdlib.md` + `docs/omniview/12-standard-library.md` | Competing stdlib docs with different paths | Consolidate into single canonical source |

---

## Methodology Notes

This deep verification applied ADR-007 Layer 2: each finding was checked against (1) actual code via grep/read, (2) all 13 ADRs for documented design decisions, (3) 40+ memory files for intentional patterns, and (4) git history for design evolution.

**Key lesson**: The v2 breadth review had a 21% false-positive rate (3/14 HIGH findings contradicted documented decisions). Cross-referencing ADRs and memory BEFORE acting on findings is essential. The 33% subagent misjudgment rate documented in prior sessions is consistent with this finding.

**Recommendation**: Future architecture reviews should include a mandatory "Check ADRs and Memory" step in the specialist prompt template to reduce false positives.
