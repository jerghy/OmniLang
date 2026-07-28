# ADR-017: Elevate Design Validation Diagnostics to Error Severity

## Status

Implemented

## Context

The Compiler & Language Architect review of OmniView's style system (2026-07-28, maturity rating B) identified that `validate_design()` in the IR lowering pipeline emitted all validation failures as `Warning` severity with incorrect `DiagnosticKind::E0002` ("duplicate symbol"). This meant:

1. **Undefined token references** (e.g., `token(missing)` referencing a non-existent token) compiled successfully with just a warning
2. **Undeclared theme references** (e.g., `@themes light, dark;` + `token primary: color = "#fff" darkk "#000"`) produced dead CSS silently
3. **Invalid default themes** (e.g., `@default-theme highContrast` not in `@themes`) went unreported
4. All diagnostics used the wrong error code (E0002 = "duplicate symbol", not appropriate for validation errors)

Prior to the theme system, design validation was minimal (only undefined token refs) and Warning severity was arguably acceptable since the `var(--name)` fallback produced valid CSS. With the addition of theme switching, validation failures now produce genuinely broken output (dead `[data-theme]` blocks, mismatched runtime themes).

## Decision Drivers

- **Compile-time correctness**: Users should know immediately when their design block source is invalid, not discover dead CSS at runtime
- **Semantic accuracy**: DiagnosticKind should reflect the actual error (undefined reference, validation error), not "duplicate symbol"
- **Theme system maturity**: The theme switching pipeline (parser → AST → IR → CSS → JS runtime) is now end-to-end; validation should match this maturity
- **Precedent**: Other OmniLang validation passes (type checking, import resolution) use `Error` severity

## Decision

**Elevate design validation diagnostics from `Warning` → `Error` severity**, and use correct `DiagnosticKind` values based on error type:

| Error Type | Old Kind | New Kind | Old Severity | New Severity |
|---|---|---|---|---|
| Undefined token reference | E0002 | **E0005** (undefined reference) | Warning | **Error** |
| Undeclared theme name | N/A (not checked) | **E0006** (validation error) | N/A | **Error** |
| Invalid @default-theme | N/A (not checked) | **E0006** (validation error) | N/A | **Error** |
| Undeclared variant dimension | E0002 | **E0006** (validation error) | Warning | **Error** |

**Architectural Components Affected:**
- `crates/omnilang-core/src/ir/lower/mod.rs` — diagnostic emission (line 172-188)
- `crates/omnilang-core/src/ir/lower/design.rs` — `validate_design()` function
- `crates/omnilang-core/src/parser/ast/design.rs` — `DesignDef` (note: lacks `source_span`)

**Interface Changes:**
- Diagnostic severity changed for `E0002` → `E0005`/`E0006` in design validation path
- No new diagnostic kinds introduced; existing `E0005` and `E0006` used
- Source location (`line: 0, col: 0`) unchanged — AST `DesignDef` lacks `source_span` (deferred)

## Consequences

### Positive

- Users receive compile-time errors for invalid design blocks, catching typos before they produce dead CSS
- DiagnosticKind accurately reflects the error type, improving IDE/editor integration
- Theme name validation catches `@themes`/`@default-theme` mismatches at compile time
- Consistent with other OmniLang validation passes (type checker, import resolver)

### Negative

- Existing `.omni` source files with previously-ignored validation issues will now fail to compile
- Downstream tools that parse diagnostic JSON may need to handle `Error` severity for design validation
- `validate_design()` still returns plain `Vec<String>` without source location — users see `line:0, col:0` (deferred to future work)

### Neutral

- Error severity applies uniformly; there is no distinction between "hard error" and "warning" within design validation
- The `lower_design()` function still runs unconditionally after validation (producing `var(--undefined)` fallbacks), but the build now fails before codegen

## Implementation Strategy

### Blast Radius

**Impact Scope**: All `.omni` source files using `design` blocks with invalid token references, undeclared themes, or variant dimension mismatches will experience build failures instead of silent compilation.

**Affected Components**:
- `omnilang-core` crate — `ir/lower/mod.rs`, `ir/lower/design.rs`
- `omnilang` facade crate — re-exports diagnostic types
- CLI tooling — diagnostic display must handle Error severity for design blocks
- CI/CD pipelines — builds with design validation issues will now fail

**Affected Teams**: Compiler team only; no runtime/application impact.

**User Impact**: Developers using `design` blocks must ensure:
- All `token(name)` references point to declared tokens
- All `@themes` declarations match per-token theme overrides
- `@default-theme` (if set) is one of the declared themes
- Variant branches reference valid dimension names

**Risk Mitigation**:
- Integration test `full_pipeline_design_with_themes` verifies valid designs produce zero diagnostics
- New validation tests cover error cases: `validate_undeclared_theme_in_token`, `validate_undeclared_default_theme`, `validate_valid_themes_no_errors`
- `learn-omniview` example project was verified to pass with new validation

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: Trivial — change `Severity::Error` back to `Severity::Warning` in one location (`ir/lower/mod.rs:177`). No data migration or schema changes required.

**Migration Paths**:
- **Forward Migration**: Already implemented. Existing valid `.omni` sources are unaffected.
- **Rollback Migration**: Revert severity and DiagnosticKind changes in `ir/lower/mod.rs:172-188`.
- **Evolution Path**: Future: add `source_span` to `DesignDef` AST to enable accurate line/col in diagnostics.

**Options Preserved**:
- Can add per-error-type severity (some errors = Error, some = Warning) without structural changes
- Can add `source_span` to `DesignDef` independently of this decision

**Commitments Made**:
- Design validation is now a build-blocking step
- `DiagnosticKind::E0005` and `E0006` are used for design validation errors

### Sequencing & Timing

**Prerequisites**:
- [x] Theme validation logic added to `validate_design()` (Fix 1)
- [x] New theme validation tests pass
- [x] Full workspace test suite passes (816 tests, 0 failures)

**System Readiness**: Ready — no infrastructure changes needed.

**Team Readiness**: Compiler team is familiar with the diagnostic system. The change is localized to one function.

**Readiness Assessment**: Ready to implement ✅ (already done)

### Social Cost

**Learning Curve**: Low — developers already expect Error severity for invalid source code.

**Cognitive Load**: Minimal — one less thing to wonder about ("why did my build succeed with a typo in my theme name?").

**Documentation Needs**:
- [x] Code comments in `validate_design()` docstring
- [ ] Update OmniView docs if they reference design block validation behavior

### Confidence Assessment

**Model Correctness Confidence**: High — the change maps directly to existing OmniLang conventions (type checker, import resolver both use Error).

**Assumptions**:
1. No downstream tooling hardcodes `Warning` for E0002 diagnostics — **Validation**: Low risk; diagnostic code changed from E0002 to E0005/E0006, so E0002 consumers unaffected
2. No existing `.omni` projects rely on Warning-tolerance for design validation — **Validation**: `learn-omniview` verified clean

**Edge Cases**:
- Design with `@themes` declared but NO theme (0 themes, not just missing `@themes`) — `validate_design` correctly skips theme validation when `ast.themes.is_empty()`
- Design with `@themes a, b` but only one token has theme overrides for `b` — not an error; partial overrides are valid and fall back to base value

## Implementation

**Phase 1: Diagnostics Fix** (✅ Done)
- Changed `Severity::Warning` → `Severity::Error` in `ir/lower/mod.rs:177`
- Changed `DiagnosticKind::E0002` → `E0005` (undefined token) or `E0006` (validation) based on error message content
- Added 4 theme validation tests

**Phase 2: Source Location** (Deferred)
- Add `source_span: Option<Span>` to `DesignDef` AST node
- Thread spans from parser through `validate_design()` return type
- Replace `line: 0, col: 0` with actual source locations

## Alternatives Considered

### Keep Warning Severity

Keep design validation at `Warning` severity, only add the missing theme checks.

**Pros:**
- No breaking change for existing projects
- Lower blast radius

**Cons:**
- Theme name typos still produce dead CSS silently
- Inconsistent with other OmniLang validation passes
- Users must manually inspect build output to catch design errors
- Does not address the root cause identified in the architecture review

### Per-error-type Severity

Use `Error` for undefined token references but `Warning` for undeclared theme names.

**Pros:**
- More nuanced than uniform Error
- Undefined token refs are clearly wrong; undeclared themes could be intentional (unused themes)

**Cons:**
- Adds complexity without clear benefit
- Undeclared theme in `theme_values` is always a bug (produces dead CSS)
- Requires structured error types from `validate_design()` (larger refactor)

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer (YAGNI Guardian & Simplicity Advocate)
**Mode**: Balanced

**Overall Decision Complexity Assessment**:
This decision is a severity/config change on existing validation logic, not a new abstraction or dependency. It aligns with existing compiler conventions. No over-engineering concern — this is the simplest fix that addresses the root cause.

**Decision Challenge**:

**Proposed Decision**: "Elevate design validation diagnostics from Warning to Error severity"

**Necessity Assessment**: 9/10
- **Current need**: Architecture review identified this as a concrete defect — invalid `.omni` sources compile successfully
- **Future need**: Already needed now; theme system is shipping
- **Cost of waiting**: Users ship dead CSS and wonder why theme switching doesn't work. Harder to fix later when more projects depend on Warning behavior.
- **Evidence of need**: Compiler Architect review finding H1 (theme name cross-validation absent), confirmed via code exploration

**Complexity Assessment**: 1/10
- **Added complexity**: One field change in one location (`Warning` → `Error`)
- **Maintenance burden**: None — same code paths, same functions
- **Learning curve**: None — developer intuition already expects Error for invalid source
- **Dependencies introduced**: None

**Alternative Analysis**:
The "Keep Warning" alternative would avoid any breaking change but perpetuates the identified defect. The "Per-error-type Severity" alternative adds structural complexity (structured error types) without proportional benefit — all design validation errors equally indicate broken output.

**Simpler Alternative Proposal**:
N/A — the chosen approach is already the simplest fix. Changing one enum value is the minimal viable change.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 1/10
- **Ratio**: 0.11 (Target: <1.5 ✅)

**Overall Assessment**:
This is textbook appropriate engineering — fixing a real, current defect with the minimal possible change. No YAGNI violation, no premature optimization, no speculative architecture.

## Validation

**Acceptance Criteria:**
- [x] All design validation errors produce `Severity::Error` diagnostics
- [x] Undefined token refs use `DiagnosticKind::E0005`
- [x] Theme/variant validation errors use `DiagnosticKind::E0006`
- [x] Valid design blocks produce zero diagnostics
- [x] Full workspace test suite passes (816 tests, 0 failures)

**Testing Approach:**
- 4 new unit tests for theme validation (`validate_design()` tests module)
- 2 existing integration tests verified (`full_pipeline_design_with_themes`, `design_without_themes_still_works`)
- Full `cargo test --workspace` passes with 0 failures

## References

- [Compiler & Language Architect Review of OmniView Style System](../reviews/) (2026-07-28, maturity B)
- `crates/omnilang-core/src/ir/lower/design.rs` — `validate_design()` function
- `crates/omnilang-core/src/ir/lower/mod.rs:172-188` — diagnostic emission
- `crates/omnilang-core/src/error.rs` — `DiagnosticKind` enum (E0005, E0006)
- [OmniView Compiler Internals](../../memory/omniview-compiler-internals.md) — compiler module reference
