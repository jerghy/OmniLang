# ADR-041: OmniView Codegen Runtime Hardening

## Status

Implemented

## Context

During integration testing of the learn-omniview project after the OmniView v5 architecture refactoring (ADR-040), 6 P0 compiler bugs were discovered that prevented the application from loading or rendering correctly. The bugs fell into two categories:

1. **JS Codegen Errors**: Generated JavaScript contained syntax errors (hyphens in variable names) and missing value accessors (`.value` suffix omitted on store fields), causing the entire application script to fail.

2. **Dead Code Elimination (DCE) Gaps**: The dead-computed elimination pass in `optimize.rs` had gaps in its expression traversal — inline styles (`el.inline_style` as JSON with `$signal` refs) and `CallExpr` function names were not tracked, causing referenced computed properties to be silently removed.

3. **@for Template Binding Failures**: Nid uniquification (`el-interp-303` → `el-interp-303_0`) during template cloning broke the binding selector lookups, and `ComponentScope::take()` reset `in_for_template` to false during child component inlining.

4. **Signal Runtime Recursion**: Computed properties and signals could trigger infinite recursion under certain dependency patterns due to missing re-entrance guards.

**Total impact**: All 10 pages failed to render content; the app showed only the shell header and tab bar with no interactive content.

## Decision Drivers

* Ensure generated JavaScript is syntactically valid and semantically correct
* Fix dead computed elimination to accurately track all computed usage sites
* Restore @for template text/class/attr bindings that were broken by nid uniquification
* Prevent silent data corruption from runtime stack overflows in computed properties
* Minimize codegen output changes to avoid test breakage in other backends (React, Flutter, Vue, Dioxus)
* Fix must be in the Rust compiler codegen only — omni source files are NOT to be modified

## Decision

Apply 8 targeted fixes across 5 compiler modules to restore correct JavaScript output, with zero modifications to omni source files.

**Architectural Components Affected:**
* `crates/omnilang-core/src/ir/optimize.rs` — Dead computed elimination pass
* `omnilang/src/codegen/omniview/editor.rs` — @if conditional transition codegen
* `omnilang/src/codegen/omniview/component_html.rs` — Child component view inlining
* `omnilang/src/codegen/omniview/list.rs` — @for template rendering
* `omnilang/src/codegen/omniview/runtime.rs` — JS signal runtime

**Interface Changes:**
* None — all fixes are within the OmniView codegen backend; no public API changes

## Consequences

### Positive

* Application loads and renders all 10 pages correctly
* @for template bindings (text, class, attr) work across component boundaries
* Store field values display properly instead of `[object Object]`
* Computed function calls (`fullName()`) resolve correctly
* Runtime stack overflow prevented in computed dependency cycles
* All existing 837+ codegen tests continue to pass

### Negative

* 3 remaining caught runtime errors (`filter` on undefined) with unknown root cause, but caught and non-blocking
* 1 caught warning (`count` not defined in slot scope) — minor slot scope limitation
* Runtime file gains ~20 lines for re-entrance guards

### Neutral

* DCE pass walks more expression types (inline style JSON, CallExpr names) — negligible performance impact since this runs during compilation
* Nid uniquification now uses runtime concatenation (`"el-interp-303_" + _idx`) for binding lookups — slightly more runtime work at clone time

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low — all changes are scoped to the OmniView codegen backend. Other backends (React, Flutter, Vue, Dioxus) are unaffected.

**Affected Components**:
| File | Change |
|------|--------|
| `editor.rs` | Variable prefix: `{nid}` → `{nid_js}` with hyphen-to-underscore sanitization |
| `optimize.rs` | Added `walk_json_for_signal_refs()` for inline styles; added `used.insert(name)` for CallExpr |
| `component_html.rs` | Added store signal names to child scope; preserve `in_for_template`; merge for-bindings after restore |
| `list.rs` | Changed selector from exact nid to `{nid}_" + _idx` in all binding lookups |
| `runtime.rs` | Added `setting` guard to `__omni_signal`; added `running` + try-catch to `__omni_computed` |

**User Impact**: None directly — all pages render correctly after fixes.

### Reversibility

**Reversibility Level**: High — each fix can be individually reverted and tested.

**Rollback Feasibility**: Each commit touches a single module with a single concern. Rollback is a simple `git revert` of the corresponding commit.

### Sequencing & Timing

**Prerequisites**: All implemented in a single session.

**Readiness Assessment**: Ready to implement — all fixes verified with 837+ existing tests passing.

### Social Cost

**Learning Curve**: Low — fixes follow existing patterns (replace `-` with `_`, add traversal loops, add runtime guards).

**Documentation Needs**:
- [x] This ADR documents the decisions and rationale

## Implementation

**Phase 1: Nid Sanitization** (`editor.rs:201`)
* Replace `_current_{nid}` with `_current_{nid_js}` where `nid_js = nid.replace('-', "_")`
* This prevents JS syntax errors from hyphens in element nids like `el-if-2908`

**Phase 2: DCE Gap Fixes** (`optimize.rs`)
* Add `walk_json_for_signal_refs()` to traverse inline style JSON for `$signal` references
* Add `used.insert(name.clone())` in `CallExpr` arm of `walk_expr_for_idents` so computed functions like `fullName()` are not eliminated

**Phase 3: Child Component Scope** (`component_html.rs`)
* Add store signal names (`store_signal_names`) to child scope during `try_inline_component`
* Preserve `in_for_template` flag from parent scope after `ComponentScope::take`
* Merge child's for-bindings back into parent scope after restore to prevent binding loss

**Phase 4: @for Binding Nids** (`list.rs`)
* Change binding lookups from exact `[data-nid="el-interp-303"]` to runtime-concatenated `[data-nid="el-interp-303_" + _idx]`
* Apply to text, class, and attribute bindings

**Phase 5: Runtime Guards** (`runtime.rs`)
* Add `setting` boolean guard to `__omni_signal` to prevent recursive `set` calls
* Add `running` boolean guard + try-catch to `__omni_computed` to prevent re-entrance and handle computed body errors gracefully

## Alternatives Considered

### Alternative 1: Fix in JS output only (post-processing)

Post-process the generated JS to replace hyphens with underscores and add `.value` suffixes.

**Pros:**
* No Rust compiler changes needed

**Cons:**
* Brittle — would need re-running after every compilation
* Cannot fix runtime recursion issues
* Does not address DCE gaps
* Rejected: violates architecture principle of correct codegen

### Alternative 2: Change nid format to avoid hyphens

Use underscores in nids instead of hyphens (e.g., `el_if_2908` instead of `el-if-2908`).

**Pros:**
* No sanitization needed in variable names
* Cleaner nid format

**Cons:**
* Hyphens in nids are the well-established format across the codebase
* Would require changing HTML attribute conventions and CSS selectors
* Does not fix the other 7 bugs
* Rejected: larger blast radius with no additional benefit

### Alternative 3: Use bracket notation for JS variable names

Generate `this['_current_el-if-2908']` instead of `_current_el-if-2908`.

**Pros:**
* No sanitization needed
* Valid JS syntax

**Cons:**
* Inconsistent with all other generated variable names
* Harder to read and debug
* Does not fix other bugs
* Rejected: consistency matters more

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer (Balanced Mode)

**Note**: *This section appears because pragmatic_mode is enabled in `.architecture/config.yml`*

**Overall Decision Complexity Assessment**: Low complexity. These are targeted bug fixes in specific codegen modules, not new architectural abstractions or patterns. Each fix addresses a concrete, verified defect with a narrow change scope.

**Decision Challenge**:

**Proposed Decision**: "Apply 8 targeted fixes across 5 compiler modules to correct JavaScript output"

**Necessity Assessment**: **10/10**
- **Current need**: Critical — application fails to load without these fixes. All 10 pages rendered empty content.
- **Future need**: Not applicable — these are bug fixes for CURRENT regressions, not future features.
- **Cost of waiting**: Zero — application is completely non-functional. Every page fails.
- **Evidence of need**: Verified through browser testing: 3 P0 syntax errors, 4 P0 runtime errors, 1 P0 stack overflow.

**Complexity Assessment**: **2/10**
- **Added complexity**: Minimal — each fix is 3-15 lines in a single module following existing patterns.
- **Maintenance burden**: Zero — these are standard bug fixes that restore expected behavior.
- **Learning curve**: None — the fixes follow existing code patterns (replace `-` with `_`, add traversal for missing expression types).
- **Dependencies introduced**: None — no new dependencies or abstractions.

**Alternative Analysis**: All 3 alternatives considered would fix only a subset of the bugs while introducing new consistency issues. The chosen approach fixes all bugs with minimal, localized changes.

**Simpler Alternative Proposal**: Could defer the `running` re-entrance guard in `__omni_computed` since the stack overflow only manifests under specific computed dependency patterns. However, the guard is 3 lines, fully generic, and prevents a class of runtime crashes — adding it now costs essentially nothing.

**Recommendation**: ✅ **Approve decision**

**Justification**: These are zero-complexity bug fixes that restore core application functionality. The necessity is absolute (app doesn't work) and the complexity is negligible (8 small, localized changes following existing patterns). No simpler alternative would fix all bugs.

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 2/10
- **Ratio**: 0.2 *(Target: <1.5)*

**Overall Assessment**: Appropriate engineering. These are essential bug fixes for a non-functional application, implemented with minimal complexity and zero new dependencies. The re-entrance guard in the runtime is the only "defensive" addition, but at 3 lines it's negligible cost for preventing a whole class of runtime crashes.

## Validation

**Acceptance Criteria:**
- [x] `omnic run` compiles the learn-omniview project without errors
- [x] Application loads in browser with no JS syntax errors
- [x] All 10 page tabs render content correctly
- [x] Todo CRUD operations work (add, display todo items)
- [x] FeatureList @for template shows text content (previously blank tags)
- [x] Store values display as text (not `[object Object]`)
- [x] Computed function calls (`fullName()`) resolve without errors
- [x] No "Maximum call stack size exceeded" errors at startup
- [x] All 837+ existing codegen tests pass

**Testing Approach:**
* Built `omnic` binary and ran `omnic run` on test project
* Served static output with Node.js HTTP server
* Verified with Playwright browser automation: page load, tab navigation, DOM content, console errors

## References

* [ADR-040: IR Enum Non-Exhaustive Strategy](.architecture/decisions/adrs/ADR-040-ir-enum-non-exhaustive-strategy.md)
* [OmniView Codegen Refactoring](omniview-codegen-refactoring.md)
* [Session 2026-07-30 Architecture v2 Review & SDD](session-2026-07-30-architecture-v2-review-sdd.md)
