# ADR-007: Three-Layer Architecture Review Methodology

## Status

Accepted

## Context

The 2026-07-26 architecture review used 11 parallel specialist agents (Systems Architect, Domain Expert, Security Specialist, etc.) to evaluate the OmniLang workspace. The 2026-07-27 follow-up revealed that **6 factual claims in the original review were incorrect**:

| Original Claim | Actual (verified) | Root Cause |
|---------------|-------------------|------------|
| "165 production unwraps" | Almost all in `#[cfg(test)]` | Grep count without context |
| "ts-rs not consumed" | `CompileResult` + `Diagnostic` imported by frontend | Grep missed `.d.ts` imports |
| "TS golden tests not in CI" | Present in `ci.yml:86-87` | Read CI file name, not content |
| "39 Clippy warnings" | 41 (19 auto-fixable) | Outdated count |
| "OmniViewGenerator 55 fields" | 45 fields | Counted incorrectly |
| "4 core structs can be filled" | Only 2 (ModuleIR/IRProgram lack AST span fields) | Assumed without checking AST types |

Additionally, **2 of 6 subagent reviews** (33%) in the prior plan-vs-reality review contained **major misjudgments** (Sprint 2/3 agents claimed completed work was not done). See [[subagent-reliability-issues]].

## Decision Drivers

1. **Subagent-only reviews produce errors** at ~33% rate for complex verification tasks.
2. **Grep-based claims without code reading** are the primary failure mode.
3. **Cross-verification by a different agent or the main session** catches these errors.
4. **Code-level verification** (Read + Bash + grep with context) produces reliable findings.
5. The cost of incorrect findings is **wasted engineering effort** on non-issues and **missed real issues**.

## Decision

**Architecture reviews in this project will follow a three-layer methodology:**

### Layer 1: Broad Specialist Review (breadth)
- 11 experts dispatched in parallel
- Each reviews from their specialty perspective
- Produces initial findings, severity ratings, cross-cutting themes
- **Limitation acknowledged**: ~10-20% error rate on factual claims

### Layer 2: Code-Level Verification (depth)
- 4-6 focused technical specialists
- Each reads actual files with Read/Grep tools
- Verifies or refutes Layer 1 claims with exact file:line citations
- Produces corrected findings and concrete implementation approaches
- **Prerequisite** for any implementation plan

### Layer 3: Implementation Planning (execution)
- Based solely on Layer 2 verified findings
- Each task includes exact file paths, code changes, test steps
- No "TBD" or "TODO" placeholders
- Tasks sized for 2-5 minute execution steps

**When to Use**:
- Major architecture reviews: All 3 layers
- Focused technical audits: Layers 2 + 3
- Quick check-ins: Layer 2 only (code-level verification of specific claims)

**Architectural Components Affected:**
- Review process in `.architecture/` (methodology, not code)
- Agent dispatch patterns (parallel broad → focused deep)
- Implementation plan quality gates (must cite file:line from Layer 2)

## Consequences

### Positive

- **Eliminates factually incorrect review claims** before they reach implementation plans.
- **Layer 2 corrects Layer 1 errors**: 6 corrections in this review cycle.
- **Prevents wasted engineering on non-issues** (e.g., "fix 165 production unwraps" when only ~2 exist).
- **Discovers deeper issues** that broad reviews miss (source_map overwrite bug, XSS chain, TypeKind serialization).

### Negative

- **Increased review cost**: 3-layer review is ~2x the agent calls of 1-layer.
- **Longer review cycle**: Layer 1→2→3 takes 2-3 sessions instead of 1.
- **Requires human judgment** to decide when Layer 2 is sufficient vs. when full Layer 1 is needed.

### Neutral

- Layer 1 remains valuable for discovering **unknown unknowns** that Layer 2 wouldn't think to check.
- The 33% subagent error rate is consistent with prior measurements in [[subagent-reliability-issues]].

## Implementation Strategy

### Blast Radius

**Impact Scope**: Process only — no code changes. Affects how future architecture reviews are conducted.

**Affected Components**:
- Review methodology documentation
- Agent orchestration in review skills
- Implementation plan quality requirements

### Sequencing

1. Apply immediately to current review cycle (already done — this ADR documents the de facto process)
2. Update architecture-review skill documentation
3. Add Layer 2 verification requirement to `.architecture/templates/review-template.md`

### Reversibility

Fully reversible — revert to single-layer reviews if cost proves too high. Each layer is independently optional.

## Alternatives Considered

### Single-layer reviews only (status quo ante)
**Rejected**: 6 factual errors and 33% subagent misjudgment rate demonstrate this is insufficient for reliable findings.

### Skip Layer 1, go directly to Layer 2
**Rejected**: Layer 1 discovers unknown unknowns across broad scope that Layer 2 specialists wouldn't think to investigate.

### Add Layer 0: automated static analysis
**Considered but deferred**: `cargo clippy`, `cargo audit`, and TS lint would catch some issues automatically. Worth adding to CI but does not replace human-level architecture analysis.

## Pragmatic Enforcer Analysis

**Necessity**: 7/10 — Subagent errors waste real engineering time. Correction cost exceeds prevention cost after 2 major review cycles.

**Complexity**: 2/10 — Process change only. No code. No new tools. Just different agent dispatch patterns.

**Ratio**: 0.29 ✅

**Recommendation**: ✅ Approved. Three-layer review is the minimum viable process to produce reliable findings from AI-assisted architecture analysis.

## Validation

**Acceptance Criteria**:
- [x] Layer 1 (11 experts) completed for 2026-07-27 re-review
- [x] Layer 2 (6 experts) completed, 6 factual errors corrected from Layer 1
- [x] Layer 3 (implementation plan) produced with file:line citations from Layer 2
- [ ] Future reviews follow this methodology
- [ ] Review template updated with Layer 2 requirement

## References

- [Full Workspace Re-Review 2026-07-27](../reviews/full-workspace-review-2026-07-27.md)
- [Deep Technical Review 2026-07-27](../reviews/deep-review-2026-07-27.md)
- [Subagent Reliability Issues](../../../C:/Users/XHT/.claude/projects/D--space-OmniLang/memory/subagent-reliability-issues.md)
- [[ADR-004]] — Original review methodology
