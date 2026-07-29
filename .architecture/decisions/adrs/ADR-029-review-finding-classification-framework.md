# ADR-029: Review Finding Classification Framework — Bug / Intentional Trade-off / Deferrable

## Status

Accepted

## Context

The 2026-07-29 comprehensive OmniView architecture review (`.architecture/reviews/component-omniview.md`, 10 specialists) surfaced 46 findings spanning from CRITICAL bugs to cosmetic suggestions. During the review, it became apparent that many "findings" were not defects at all — they were intentional design trade-offs made to optimize OmniView for its debug/development runtime role (per ADR-012).

Without a clear classification framework, future reviews will:
1. Re-flag the same 14 intentional trade-offs as bugs, wasting review cycles
2. Fail to distinguish between "must fix now" (CRITICAL/HIGH bugs) and "nice to have eventually" (deferrable code quality)
3. Treat all findings with equal urgency, diluting focus

This ADR establishes the triage framework used in this review cycle so that future reviewers — whether human or automated — can apply the same classification consistently.

## Decision Drivers

- **Prevent review churn**: 14 of 46 findings (30%) were intentional design trade-offs per ADR-028. Without classification, each future review re-discovers and re-debates them.
- **Focus remediation on real bugs**: CRITICAL findings (BlockStmtIR dropped, activeEffect leak) deserve immediate attention. MEDIUM code-quality items (extract helpers, rename types) should not compete for the same urgency.
- **Respect architectural intent**: ADR-012 established OmniView as a debug tool. Design decisions that flow from this role (no minification, no batching, inline runtime) are architecture, not accidents.
- **Enable automated triage**: A structured framework can be applied by future AI reviewers without re-deriving the rationale each time.

## Decision

All review findings SHALL be classified into exactly one of three buckets:

### Bucket 1: Real Bugs (Fix Now)

Findings that represent incorrect behavior, security vulnerabilities, or runtime failures — regardless of the debug/production role.

**Characteristics**:
- Produces incorrect or broken output (e.g., BlockStmtIR silently dropped)
- Causes runtime state corruption (e.g., `__omni_activeEffect` leak)
- Creates a security vulnerability (e.g., missing CSP directives)
- Breaks a documented contract (e.g., prop immutability not enforced)
- Fails an existing test assertion (e.g., `test_resolve_styles_with_design_class`)

**Action**: Fix. Severity-driven prioritization (CRITICAL → HIGH → MEDIUM → LOW).

**Examples from this review**:
| Finding | Severity | Fix |
|---------|----------|-----|
| BlockStmtIR silently dropped | CRITICAL | Add match arm (15 lines) |
| `__omni_activeEffect` leak on exception | CRITICAL | try/catch/finally (3 lines) |
| CSP missing form-action/object-src/frame-src | HIGH | Add directives (1 line) |
| Known test failure (design_class_names ordering) | HIGH | Move population earlier (5 lines) |

### Bucket 2: Intentional Trade-offs (Document, Don't Fix)

Findings that represent deliberate architectural choices made to optimize OmniView for its debug/development runtime role. These are documented in ADR-028 and SHALL NOT be re-flagged as bugs.

**Characteristics**:
- Documented in ADR-012 or ADR-028 as intentional
- Trade-off favors developer experience over production optimization
- Has a clear alternative (e.g., "would be a bug if this were a production target")
- The production path (React/Flutter backends) handles this differently

**Action**: Reference ADR-028. Do NOT fix unless OmniView becomes a production target.

**Examples from this review**:
| Finding | ADR-028 Ref | Why Intentional |
|---------|-------------|-----------------|
| No signal batching | D3 | Synchronous = predictable = debuggable |
| No output minification | D4 | Human-readable output for debugging |
| Inline runtime (~200 lines) | D1 | Self-contained iframe-ready document |
| wrap_debug_html without CSP | D6 | Debug-only wrapper; production path has CSP |
| String-based codegen (not CodeWriter) | D2 | ADR-012: imperative pattern for one-shot output |
| (0, eval) hot-replace | D8 | Editor-mode only; production builds exclude this |
| debug_assert! phase guards (not production) | D12 | Zero-cost in release; catches dev mistakes |

### Bucket 3: Deferrable Improvements (Fix When Touching)

Findings that represent code quality, maintainability, or performance improvements that are real but not urgent. These do not block the current release and can be addressed opportunistically.

**Characteristics**:
- Code works correctly but could be better organized
- Represents duplication, dead code, or inconsistent style
- No behavioral change if fixed (pure refactoring)
- Fix is isolated and low-risk

**Action**: Record in technical debt backlog. Fix when the affected file is next modified for other reasons, or batch into a dedicated cleanup sprint.

**Examples from this review**:
| Finding | Severity | Fix |
|---------|----------|-----|
| 46-field God Object (OmniViewGenerator) | HIGH | Phased decomposition (L effort, separate plan) |
| Motion trigger duplication (~280 lines) | HIGH | Extract helpers (M effort) |
| VariantIR name clash | HIGH | Rename to DesignVariantIR (M effort) |
| push_str(&format!(...)) → write!() | MEDIUM | Mechanical conversion (M effort) |
| Dead field current_nid_path | LOW | Remove (XS effort, done) |
| Magic strings for HTML attributes | MEDIUM | Named const slices (S effort) |

### Classification Decision Tree

```
Is the behavior INCORRECT or INSECURE?
  ├─ YES → Bucket 1 (Real Bug). Fix now.
  └─ NO → Is it documented in ADR-028 as an intentional debug-runtime trade-off?
            ├─ YES → Bucket 2 (Intentional Trade-off). Reference ADR-028. Do not fix.
            └─ NO → Does fixing it change runtime behavior?
                      ├─ YES → Bucket 1 (it's a latent bug). Fix now.
                      └─ NO → Bucket 3 (Deferrable Improvement). Fix when touching.
```

**Architectural Components Affected:**
- Architecture review process (`.architecture/reviews/` documents)
- ADR registry (ADR-012, ADR-028, ADR-029)
- Future review dispatch prompts (include classification instructions)

**Interface Changes:**
- Review documents SHALL include a classification summary table showing bucket assignments
- Review documents SHALL reference ADR-028 for any Bucket 2 findings
- ADR-028 SHALL be updated when new intentional trade-offs are identified

## Consequences

### Positive

1. **Review efficiency**: Future reviewers (human or AI) can skip 14 known intentional trade-offs immediately by referencing ADR-028, saving ~30% of review effort.
2. **Clear prioritization**: The three buckets create a natural priority order: fix critical bugs → defer improvements → document trade-offs.
3. **Architectural self-awareness**: The framework forces reviewers to ask "is this a bug or an intentional choice?" before reporting findings.
4. **Prevents regression**: ADR-028 acts as a permanent record — even if team members change, the rationale for each trade-off is preserved.
5. **Automation-ready**: The decision tree can be applied by AI reviewers without re-deriving context.

### Negative

1. **Classification ambiguity**: Some findings straddle buckets (e.g., a MEDIUM code-quality issue that also has security implications). Resolution requires human judgment.
2. **ADR-028 maintenance burden**: New intentional trade-offs must be added to ADR-028, or they'll be misclassified as Bucket 1 or Bucket 3 in future reviews.
3. **Over-classification risk**: Aggressive Bucket 2 classification could hide real issues behind "it's intentional" without proper documentation.

### Neutral

1. **Bucket ratios will shift over time**: As the codebase matures, Bucket 1 (bugs) should decrease and Bucket 3 (improvements) should increase. Tracking these ratios provides a maturity metric.
2. **Framework is review-process-level, not code-level**: This ADR doesn't change any source code — it changes how we evaluate code during reviews.

## Pragmatic Enforcer Analysis

### Necessity Assessment (8/10)

- **Current need**: The 2026-07-29 review wasted ~30% of review capacity re-identifying known intentional trade-offs. Without classification, this repeats in every future review. Score: 8/10.
- **Future need**: As the codebase grows, the number of intentional trade-offs and deferrable improvements will increase. Without a framework, reviews become increasingly noisy. Score: 7/10.
- **Cost of waiting**: Each review cycle without this framework re-debates the same 14 findings. At 10 specialists per review, that's ~140 specialist-hours of duplicated analysis. Score: 8/10.
- **Evidence**: ADR-012 established OmniView's debug-tool role but didn't provide a classification framework. The 2026-07-29 review empirically demonstrated the need. Score: 9/10.

### Complexity Assessment (3/10)

- **Added complexity**: Three-bucket classification + decision tree. Fits on one page. Score: 2/10.
- **Maintenance cost**: ADR-028 must be updated when new intentional trade-offs are added. Low ongoing cost. Score: 3/10.
- **Learning curve**: Reviewers must read ADR-028 and ADR-029 before conducting reviews. One-time cost. Score: 2/10.
- **Dependencies**: Depends on ADR-012 (debug-tool role) and ADR-028 (trade-off catalog). Both already exist. Score: 1/10.

### Ratio

Necessity: 8.0 / Complexity: 2.0 = **Ratio: 0.25** (well under 1.5 threshold)

### Recommendation

**Approve**. The classification framework is minimal (one-page decision tree), addresses a demonstrated problem (30% review churn), and has zero code impact. It is the simplest thing that solves the problem — a YAGNI-compliant process improvement.

## Implementation Strategy

### Blast Radius

**Impact Scope**: Architecture review process only. No code changes. No runtime impact. No user-facing changes.

**Affected Components**:
- `.architecture/reviews/` — future review documents must include classification summary
- `.architecture/decisions/adrs/ADR-028.md` — must be maintained as the canonical trade-off catalog
- Review dispatch prompts — should include classification instructions

### Rollout Plan

1. This ADR is effective immediately upon acceptance
2. ADR-028 becomes the canonical reference for Bucket 2 (intentional trade-offs)
3. Future review documents include a classification summary table
4. No migration needed — the 2026-07-29 review already applied this framework

## Alternatives Considered

### Alternative A: Two-Bucket Classification (Bug / Not Bug)

Merge Bucket 2 (Intentional Trade-offs) and Bucket 3 (Deferrable) into a single "Not Bug" bucket.

**Rejected because**: Loses the critical distinction between "never fix this" (intentional trade-offs) and "fix eventually" (deferrable improvements). Future reviewers would need to re-derive which "not bugs" are intentional vs. deferrable, recreating the same problem.

### Alternative B: Severity-Only Classification (Current ADR-025/026 Approach)

Use only severity levels (CRITICAL/HIGH/MEDIUM/LOW) without bucket classification. This was the approach in ADR-025 and ADR-026.

**Rejected because**: Severity alone doesn't capture intent. A HIGH-severity finding like "God Object at 46 fields" and a HIGH-severity finding like "CSP missing directives" have the same label but completely different action profiles (deferrable refactoring vs. immediate security fix). The ADR-025/026 approach led to the very confusion this ADR resolves.

### Alternative C: Per-Finding ADR

Create an ADR for each intentional trade-off individually rather than cataloging them in ADR-028.

**Rejected because**: 14 separate ADRs for items that share a single root cause ("OmniView is a debug runtime") would be excessive ceremony. ADR-028 catalogs them efficiently in one document.

## Validation

**Success Criteria**:
1. The next OmniView architecture review classifies all findings using this framework
2. Bucket 2 findings are referenced to ADR-028, not re-debated
3. Review documents include a classification summary

**Verification**: The 2026-07-29 review (`.architecture/reviews/component-omniview.md`) already applied this framework retroactively — the "Acknowledged Design Trade-offs (ADR-028)" section and the reclassified cross-cutting themes demonstrate the framework in practice.

## References

- [ADR-012: OmniView Codegen as Debug Tool — Independent Implementation](ADR-012-omniview-codegen-debug-tool.md) — Establishes OmniView's debug-tool role
- [ADR-028: OmniView as Debug/Development Runtime — Intentional Design Trade-offs](ADR-028-omniview-debug-runtime-design-tradeoffs.md) — The canonical catalog of Bucket 2 items
- [OmniView Architecture Review (2026-07-29)](../../reviews/component-omniview.md) — First application of this framework
- [ADR-025: Review Remediation 2026-07-29](ADR-025-omniview-review-remediation-2026-07-29.md) — Pre-framework review (severity-only)
- [ADR-026: Review v2 Fixes](ADR-026-omniview-review-v2-fixes.md) — Pre-framework review (severity-only)
