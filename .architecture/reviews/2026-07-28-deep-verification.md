# Architecture Review: Deep Verification — ADR Cross-Reference

**Date**: 2026-07-28  
**Type**: Layer 2 Deep Verification (per ADR-007)  
**Method**: Cross-reference all 39 Round 1 findings against 10 ADRs + code-level verification  
**Previous**: [2026-07-28-breadth-review.md](2026-07-28-breadth-review.md) (Layer 1, 11 specialists)

---

## Executive Summary

Of the 39 findings from the Layer 1 breadth review, this deep verification **confirms 31 as genuine issues**, identifies **5 as intentional design** per ADRs, and marks **3 as false positives** (findings that contradict explicit architectural decisions). All 5 HIGH-severity findings from Round 1 are **confirmed genuine** — none are false positives.

The most significant correction: **3 findings were downgraded** after ADR cross-reference revealed them as intentional:
- "Only 3/9 backends in regression tests" → **By design** per ADR-003 (P0 backends only)
- "Tauri FS capabilities grant D:/** and C:/**" → **By design** per ADR-004 Decision 2 (FS全覆盖是编辑器必要条件)
- "6 backend test coverage gap" → **By design** per ADR-003 (suspended backends)

---

## Verified Against ADRs: Finding-by-Finding

### HIGH Severity (5/5 CONFIRMED)

| # | Finding | ADR Cross-Reference | Code Verification | Verdict |
|---|---------|---------------------|-------------------|---------|
| **C1** | Optimizer bypassed in single-file compilation | ADR-006/P39 activated IR optimizer. ADR-010 implemented MVP. But `run_pipeline` (`lib.rs:116`) never calls `optimize()`, while `analyse_multi` (`analyse.rs:322`) does. | ✅ `lib.rs:115-116`: `let ir = ir::lower(...)` → returns without optimization. `analyse.rs:322`: `crate::ir::optimize::optimize(&mut merged_ir)` → does run. **Gap confirmed.** | **✅ CONFIRMED** — genuine bug |
| **C2** | Multi-field tagged union variant payload truncated | No ADR justifies single-field-only. AST has `Variant.fields: Vec<StructField>`; IR has `VariantIR.payload: Option<TypeIR>` (single). | ✅ `lower_logic.rs:58`: `payload: v.fields.first().map(...)` — drops fields[1..]. P4-1 grammar spec defines multi-field variants. | **✅ CONFIRMED** — data-loss bug |
| **C3** | WASM tests crash `cargo test --workspace` | ADR-003 mentions wasm-pack CI as Linux-only, non-blocking. Intentional to limit platform scope, but not intentional to crash native workspace test. | ✅ `lib.rs:660+` tests call `serde_wasm_bindgen` unconditionally. Panics on non-wasm32. | **✅ CONFIRMED** — needs `#[cfg(target_arch = "wasm32")]` |
| **C4** | Production CSP includes `'unsafe-eval'` | ADR-010 (P38b) addresses XSS chain but not CSP. ADR-004 Decision 2 emphasizes defense-in-depth — which `unsafe-eval` undermines. | ✅ `tauri.conf.json:22`: production CSP has `'unsafe-eval'`. Dev CSP (`:23`) has it too — can't differentiate. `devCsp` field exists separately. | **✅ CONFIRMED** — move to devCsp only |
| **C5** | `check_call` dead — function args never validated | No ADR mentions this. P4-2 was incremental delivery — function exists but was never wired. | ✅ `validate_types.rs:240-267`: `#[allow(dead_code)] pub fn check_call(...)` — full impl, 0 call sites. `resolve.rs:359-363`: FuncCall handler only resolves refs. | **✅ CONFIRMED** — wire it in |

### MEDIUM Severity (10/16 confirmed, 3 intentional, 3 false-positive/downgraded)

| # | Finding | ADR Cross-Reference | Verdict |
|---|---------|---------------------|---------|
| **I1** | `run_pipeline` duplication | ADR-001 describes single pipeline. P35 was meant to eliminate duplication. Having two is a cleanup remnant, not intentional. | **✅ CONFIRMED** |
| **I2** | Template parser error-span bug | No ADR addresses this. `template.rs:45-107`: All errors use `Span { start: 0, end: 0 }`. | **✅ CONFIRMED** |
| **I3** | 31× redundant HashMap construction | ADR-001 notes `build_component_map` was migrated to borrowed refs. But `all_components()` still builds HashMap each call. | **✅ CONFIRMED** |
| **I4** | FS capabilities vs path validation mismatch | **ADR-004 Decision 2**: `C:/**` and `D:/**` are **intentional design** — "FS 全覆盖是编辑器必要条件". But `open_project` (`project.rs:189-215`) rejects non-HOME paths. The inconsistency (capabilities allow D:, validation rejects D:) IS a real gap. | **⚠️ PARTIAL** — broad FS scope is intentional, but validation should match |
| **I5** | VariantEditor.tsx 1,601 lines | No ADR. Code verified: 1,601 lines, 10+ inline sub-components. | **✅ CONFIRMED** |
| **I6** | `as any` hidden property patterns | No ADR. Code verified: `graph-parser.ts:492,508`, `graph-compiler.ts:118`. | **✅ CONFIRMED** |
| **I7** | Dead P35-remnant test file in core | P35 completed per ADR-006. `javascript.rs` gated on nonexistent `codegen-javascript` feature in core crate. Cleanup remnant. | **✅ CONFIRMED** |
| **I8** | Only 3/9 backends in regression tests | **ADR-003**: "仅 OmniView、React、Flutter 保持活跃开发（P0）。其余 6 目标暂停发展。" Parameterized tests intentionally only cover P0. | **❌ FALSE POSITIVE** — by design |
| **I9** | `ComponentIR.refs` dead domain surface | No ADR justifies `serde_json::Value` or always-empty state. AST has `RefDecl` — lowering drops it. | **✅ CONFIRMED** |
| **I10** | WASM cache unbounded growth | ADR-001 mentions incremental cache. ADR-002 mentions P8 (WASM cache). No eviction policy is acknowledged gap. | **✅ CONFIRMED** |
| **I11** | Eval baselines outdated | No ADR. Code: `BASELINE_TESTS=227`, actual ~737+. `BASELINE_CLIPPY=24`, actual 0. `BASELINE_UNWRAP=0`, actual ~240. | **✅ CONFIRMED** |
| **I12** | `type_mismatch` lint overlaps with `validate_types` | ADR-001 describes Before-IR + After-IR lint. The lint predates the type checker. Overlap is a natural artifact of incremental development. | **✅ CONFIRMED** — audit overlap |
| **I13** | CI `npm audit` broken | No ADR. Registry mirror (`npmmirror.com`) doesn't support audit API. | **✅ CONFIRMED** |

### LOW Severity — ADR-Impacted

| # | Finding | ADR Cross-Reference | Verdict |
|---|---------|---------------------|---------|
| L1 | "6 backends not in regression coverage" | **ADR-003**: Suspended backends. Code preserved under feature flags, not in CI. | **❌ FALSE POSITIVE** |
| L2 | "TS pipeline as compiler" misconception | **ADR-005**: TS pipeline is editor round-trip translator, NOT a compiler. Explicitly forbidden terminology. | **❌ FALSE POSITIVE** (corrected) |
| L3 | P36 `compile_multi_target` concern | **ADR-009**: SUPERSEDED — function was removed as YAGNI. No longer exists. | **⬇️ MOOT** — already resolved |
| L4 | P38b TAURI_INTERNALS strip concern | **ADR-010**: TAURI_INTERNALS strip was rejected. XSS chain fix (innerHTML→textContent) was the actual implementation. | **⬇️ INTENTIONAL** — XSS chain fix is sufficient |
| L5 | `source_span` not populated on 29 IR structs | **ADR-008**: "Selective Fill" — only 6 of 37 structs populated. 29 intentionally deferred until consumers emerge. | **⬇️ INTENTIONAL** — selective fill strategy |
| L6 | TypeKind flattening (IR can't represent compound types) | **ADR-001 Consequences (Negative)**: "TypeKind 过于简单 — Named { name: String } 存储类型为不透明字符串，无递归验证" — explicitly documented as known limitation. | **⬇️ KNOWN LIMITATION** — documented in ADR-001 |

---

## ADR-Anchored Design Decisions (Not Bugs)

These findings from Round 1 are **intentional architecture**, confirmed by ADRs:

| Design Decision | ADR | Rationale |
|----------------|-----|-----------|
| Only 3 P0 backends tested | ADR-003 | 单人团队无法维持9个后端。其余6个暂停，feature-flag保留 |
| FS scope `C:/**` and `D:/**` | ADR-004 Decision 2 | 编辑器必要条件。安全重点在纵深防御（CSP、沙箱、路径验证） |
| TS pipeline is not a compiler | ADR-005 | 编辑器往返翻译层，不是第二个编译器 |
| P36 `compile_multi_target` removed | ADR-009 | SUPERSEDED — removed as YAGNI in remediation cleanup |
| P38b XSS chain fix, not TAURI_INTERNALS strip | ADR-010 | Sandbox already isolates user code. XSS chain fix is more effective |
| source_span selective fill (6/37) | ADR-008 | 29 structs have no consumers; defer until needed |
| TypeKind simplicity (opaque strings) | ADR-001 | Known limitation — "过于简单" documented, improvement deferred to Sprint 4 |

---

## Revised Prioritization

After ADR cross-reference, the 39 findings resolve to:

| Severity | Count | Change from Round 1 |
|----------|-------|---------------------|
| **HIGH** (genuine bugs) | 5 | Unchanged — all 5 confirmed |
| **MEDIUM** (genuine issues) | 13 | I8/I12 downgraded, I4 partial |
| **LOW** (genuine issues) | 13 | L1/L2/L3 removed |
| **BY DESIGN** (intentional) | 5 | New category |
| **FALSE POSITIVE** | 3 | I8, L1, L2 |

### Critical Path (unchanged from Round 1)

1. **C1**: Add `optimize()` to `run_pipeline` — **S effort, 5 min**
2. **C2**: Fix variant payload truncation — **M effort, 1-2 hrs**
3. **C3**: Guard WASM tests with `#[cfg(target_arch = "wasm32")]` — **S effort, 15 min**
4. **C4**: Move `'unsafe-eval'` from production CSP to devCsp — **S effort, 5 min**
5. **C5**: Wire `check_call` into `resolve_expr_refs` — **M effort, 1-2 hrs**

### Key Recalibrations

| Original Severity | Finding | New Assessment | Reason |
|-------------------|---------|---------------|--------|
| HIGH (Security) | Tauri FS D:/** C:/** scope | **BY DESIGN** | ADR-004 Decision 2: FS全覆盖是编辑器必要条件 |
| MEDIUM | 6 backends not in regression | **FALSE POSITIVE** | ADR-003: P0-only testing is intentional |
| MEDIUM | P36 compile_multi_target | **MOOT** | ADR-009: Function already removed |
| LOW | TS pipeline as compiler | **FALSE POSITIVE** | ADR-005: TS is editor translator, not compiler |
| LOW | source_span gap on 29 structs | **BY DESIGN** | ADR-008: Selective fill strategy |
| LOW | TAURI_INTERNALS strip | **BY DESIGN** | ADR-010: XSS chain fix, not strip |

---

## ADR Quality Assessment

All 10 ADRs are well-structured with Status, Context, Decision Drivers, Decision, Consequences, Implementation Strategy, Alternatives, Pragmatic Enforcer Analysis, and Validation sections. Quality assessment:

| ADR | Quality | Notes |
|-----|---------|-------|
| ADR-001 | **A** | Comprehensive pipeline architecture. Negative consequences honestly documented. Validation criteria measurable. |
| ADR-002 | **A** | Excellent evolution narrative (v3→v4→v4.1→v5). Clearly documents rejected alternatives and why. |
| ADR-003 | **A** | Pragmatic decision with clear YAGNI justification. Ratio 0.22. High reversibility. |
| ADR-004 | **B+** | Good summary of review conclusions. Decision 2 (FS scope) should cite the security model document more prominently. |
| ADR-005 | **A** | Terminology correction with enforcement mechanism. Prevents future review false positives. |
| ADR-006 | **A** | Clear reclassification from DEFERRED→ACTIVE with per-item Pragmatic Enforcer scores. |
| ADR-007 | **A** | Methodology ADR with empirical data (33% subagent error rate, 6 Layer 1 corrections). Meta-cognitive. |
| ADR-008 | **A** | Selective fill strategy with concrete counts (6 sites, 29 deferred). Well-reasoned alternatives. |
| ADR-009 | **B** | Good decision but superseded. The cleanup note is clear. Could benefit from a post-mortem on why it was removed. |
| ADR-010 | **A** | Excellent technical detail on exploit chain and incremental fix strategy. Clear P0/P1/P2 prioritization. |

**ADR gaps identified**:
- No ADR exists for the `run_pipeline` duplication (P35 cleanup remnant)
- No ADR exists for the eval system design or baseline policy
- ADR-004 Decision 2 (FS scope) could use a more prominent security-model cross-reference
- ADR-009 supersession should record *why* it was removed (speed impact? complexity? YAGNI?)

---

## Regression Detection (ADR-Guided)

Using ADRs as ground truth for "what should exist":

| Capability | ADR Reference | Current State | Status |
|-----------|---------------|---------------|--------|
| P0 backends (OmniView/React/Flutter) | ADR-003 | ✅ All compiling, tested | OK |
| Suspended backends (6) | ADR-003 | ✅ Feature-gated, compile-checked | OK |
| P35 crate split | ADR-006 | ✅ Completed, facade re-exports from core | OK |
| P38b XSS chain fix | ADR-010 | ✅ innerHTML→textContent, origin validation, atomic capabilities | OK |
| P39 dead computed MVP | ADR-010 | ✅ `ir/optimize.rs` implemented, called in `analyse_multi` | OK |
| P39 full optimization | ADR-010 | ❌ Deferred — `ComputedIntent.deps` still `vec![]` | Known deferred |
| P36 parallel codegen | ADR-009 | ❌ SUPERSEDED — removed | Known removed |
| IR source_map end-to-end | ADR-001 | ⚠️ Partial — per-element works, per-struct selective | Known limitation |
| TypeKind recursive types | ADR-001 | ❌ Deferred to Sprint 4 | Known deferred |
| P8 WASM incremental cache | ADR-002 | ⚠️ Memory-only, no eviction policy | Known limitation |

---

## Conclusion

The Layer 1 breadth review was **~92% accurate** (31/39 findings confirmed genuine, 5 intentional design, 3 false positives). The 3 false positives all stem from not cross-referencing ADRs — specifically ADR-003 (P0-only backends), ADR-004 (FS scope design), and ADR-005 (TS pipeline terminology).

**All 5 HIGH-severity findings are confirmed genuine.** None were false positives. The ADRs validate that the project's architecture is intentional and well-documented — the issues found are implementation gaps, not design flaws.

### Recommended Next Actions

1. Fix C1-C5 immediately (HIGH findings, all confirmed genuine)
2. Update eval baselines (I11)
3. Audit `type_mismatch` lint vs `validate_types` overlap (I12)
4. Create ADR for eval baseline policy
5. Update `.architecture/templates/review-template.md` to require ADR cross-reference before filing findings
