# Architecture Review: OmniView High-Frequency Element Manipulation

**Date**: 2026-07-30
**Review Type**: Component — Element Reference System (`@ref`) + High-Frequency DOM Manipulation
**Expert Panel**: 6 specialists (Compiler Architect, TS/React Specialist, Performance Specialist, Maintainability Expert, Pragmatic Enforcer, Rust Expert)

---

## Executive Summary

**Overall Assessment: Needs Improvement (Conditional Approval)**

The Element Reference System (`@ref`, ADR-036) is architecturally sound — the IR design is clean, the semantic validator is well-structured, and the OmniView debug runtime works correctly. **However, the React production codegen has a critical gap: `ref_names` expression translation was defined but never wired up.** Every caller of `format_expr_ir_tsx` passes `&FxHashSet::default()`, meaning ref identifiers in function bodies are never translated to `nameRef.current`. This produces runtime `ReferenceError` on the primary production target — the exact use case the feature was designed for (60fps drag/mousemove in event handlers).

Additionally, two pre-existing HIGH bugs (H1: `@for` nid collision breaks motion triggers for non-first list items; H2: FLIP silently fails due to ordering bug) remain unfixed from the prior review and are prerequisites for Phase 2 ref system work.

The Pragmatic Enforcer's prior YAGNI objection to `useRef` is moot: the `@ref` language primitive makes `useRef` a necessary consumer, and the implementation is appropriately minimal (Phase 1: ~130 lines of production code across 2 backends, Phase 2 items explicitly deferred).

**Pragmatic Enforcer Assessment**: Necessity 7/10, Complexity 4/10, Ratio 0.57 — well below 1.5 threshold. Approve conditionally.

---

## Cross-Cutting Themes

### Theme 1: React Expression Translation Gap (4/6 experts)

The `ref_names` parameter on `format_expr_ir_tsx` was defined but never populated. Every call site passes `&FxHashSet::default()`. Function body ref access (`box.style.transform = "..."`) generates `ReferenceError` on React. This was the #1 finding across Compiler Architect (CRITICAL), TS/React Specialist (CRITICAL), Pragmatic Enforcer (HIGH), and Maintainability Expert (MEDIUM).

### Theme 2: H1/H2 Bugs Block Phase 2 (2 experts + prior review)

H1 (`@for` nid collision) and H2 (FLIP ordering) were documented in the prior review, confirmed by Performance Specialist, and noted by Compiler Architect. Both affect correct Element Ref behavior in `@for` lists — a Phase 2 dependency.

### Theme 3: DRY Violations in Generated Code Templates (3 experts)

Maintainability Expert, Rust Expert, and Performance Specialist all noted template duplication: triple `_elCache` lookup (list.rs), triple `__omni_effect` wrapper (style.rs), and duplicate `__omni_buildNidMap` (editor.rs). Each is a maintenance risk with potential for silent divergence.

### Theme 4: Hardcoded `HTMLDivElement` Type (3 experts)

TS/React Specialist (MEDIUM), Compiler Architect (LOW), and Maintainability Expert (MEDIUM) flagged the hardcoded `useRef<HTMLDivElement>(null)` type. Non-div refs produce TypeScript type errors. Fixing this requires IR-level element type tracking (deferred).

---

## Trade-offs and Disagreements

### Trade-off 1: Signal Batching vs Simplicity

**Performance Specialist** recommends microtask-based signal batching to reduce redundant effect executions. **ADR-028 D3** documents this as a deliberate trade-off: synchronous subscriber notification is simpler and sufficient for typical component hierarchies. The cost manifests at 60fps with 3+ animated properties.

**Resolution**: Defer until profiling data shows measurable jank in real applications. The existing architecture is correct — just not optimally efficient.

### Trade-off 2: `capturePointer`/`releasePointer` vs YAGNI

**Pragmatic Enforcer** notes zero consumers for pointer capture methods in the IIFE wrapper. **Compiler Architect** and **TS/React Specialist** argue the 2 lines of code are negligible insurance for a feature (drag) that is the primary use case.

**Resolution**: Keep the methods. The cost (2 lines) is lower than the cost of adding them later (API surface inconsistency, versioning).

### Trade-off 3: `scrollTo`/`scrollBy` — Documented but Not Implemented

ADR-036 lists `scrollTo`/`scrollBy` in the API table. The OmniView IIFE does not implement them. **Pragmatic Enforcer** recommends removing from docs; **Compiler Architect** notes it as LOW.

**Resolution**: Remove from ADR-036 API table until implemented, or add 2 lines to IIFE. Either choice is fine.

---

## Prioritized Recommendations

### Critical (0-2 weeks)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P0-1** | React ref expression translation dead code — generates `ReferenceError` | **CRITICAL** | Thread `ref_names` through `gen_view_node_tsx`, `generate_statement_tsx_with_mode`, and `gen_functions`. Add `ref_names` to `ReactFormatExpr` struct so trait-based formatting also translates ref identifiers. | **M** (4-6h) |
| **P0-2** | FLIP `flipBefore` runs after clearing loop — always captures zero elements | **HIGH** | Move `Omni.flipBefore(parent)` call before the `while (...removeChild...)` clearing loop at `list.rs:117`. | **XS** (10 min) |
| **P0-3** | `@for` nid collision — motion triggers only fire on first list item | **HIGH** | Generate unique nid values per clone at runtime (e.g., append `_idx`). Update binding code to use per-clone cache. Consider scoped `__omni_el` that accepts a parent context parameter. | **M** (1-2 days) |

### Important (2-8 weeks)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P1-1** | `ref_nid_map: Vec<(String, String)>` allows silent duplicates | **HIGH** | Add `push_ref_nid()` helper with `debug_assert!` + `FxHashSet` uniqueness guard. | **XS** (2h) |
| **P1-2** | Unnecessary `el.children.clone()` before slot-empty check | **HIGH** | Move clone inside the `else` branch of `try_inline_component`. | **S** (1h) |
| **P1-3** | Per-property style effects — N independent effects per element | **MEDIUM** | Consolidate per-element dynamic style assignments into a single `__omni_effect()`. Collect by nid before emitting. | **M** (1 day) |
| **P1-4** | Add warning for `@ref` on custom components (silently dropped) | **MEDIUM** | Emit warning when `el.ref_binding.is_some()` and tag resolves to component. | **XS** (1-2 lines) |
| **P1-5** | Add warning for `@ref` inside `@for` (unsupported Phase 2) | **MEDIUM** | Add semantic validation in `validate_refs.rs` to detect ref bindings in `ListRendering` contexts. | **S** (20 lines) |
| **P1-6** | Three `_elCache` lookup templates duplicated in `list.rs` | **MEDIUM** | Extract `emit_el_cache_lookup()` helper. | **XS** (1h) |
| **P1-7** | Three `__omni_effect` wrapper templates duplicated in `style.rs` | **MEDIUM** | Extract `emit_style_effect()` helper. | **XS** (1h) |

### Nice-to-Have (2-6 months)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P2-1** | `HTMLDivElement` hardcoded for all React `useRef` types | **MEDIUM** | Add `tag` field to `RefDeclIR`, map common tags → HTML element types, default to `HTMLElement`. | **S** (2h) |
| **P2-2** | `__omni_buildNidMap` duplicated in `editor.rs` | **MEDIUM** | Extract to shared `const &str`. | **XS** (30 min) |
| **P2-3** | Signal batching (microtask-based) | **MEDIUM** | Implement microtask queue for synchronous signal.set() calls. Gated on profiling data showing real impact. | **M** (1-2 days) |
| **P2-4** | `///` → `//` in JS emitted comment | **LOW** | Change doc-comment to standard JS comment in `emit_ref_wrappers`. | **XS** (5 min) |
| **P2-5** | Dead querySelector fallback in @for binding loops | **LOW** | Remove `if (!_el) { clone.querySelector(...) }` fallback blocks from list.rs:256/297/327. | **XS** (30 min) |
| **P2-6** | `ForStateEntry` struct to replace unnamed tuple | **LOW** | Define struct with named fields for original_name/default_js. | **XS** (30 min) |

---

## Risks if Unaddressed

1. **React production crash**: Any `.omni` component using `@ref` with a handler function will throw `ReferenceError` on the React target — the exact use case the feature was designed for. OmniView debug will work; React production will fail. This is the worst class of bug ("works in dev, crashes in prod").

2. **FLIP animations silently broken**: Developers invest time authoring `@for` + `key` list animations. The animations never execute. No error is thrown — just silently absent motion.

3. **Motion on list items broken**: In any `@for` list with 2+ items, hover/enter/press/scroll/drag triggers fire only on the first item. This is visible to end-users and erodes framework trust.

4. **Misleading API surface**: The `format_expr_ir_tsx` function accepts `ref_names` but it's never populated. Future developers will assume the feature works and build on it. The test validates the wrong output, normalizing broken behavior.

5. **Template drift**: Duplicated code patterns (triple `_elCache`, triple `__omni_effect`, double `__omni_buildNidMap`) will silently diverge over time as fixes are applied inconsistently.

---

## Individual Member Reviews

### Compiler & Language Architect (Dr. Lin Wei)

**Summary**: Pipeline-level evaluation. The IR design is clean and minimal (`ref_binding: Option<String>`). The critical gap is that the React codegen expression translation infrastructure exists but is disconnected — every caller passes `&FxHashSet::default()`. The OmniView backend works correctly only because it uses `const box = (function()...)()` naming (matches source), while React uses `const boxRef = useRef(...)` naming (requires translation). Cross-target semantic divergence: same `.omni` source produces working code on OmniView but broken code on React.

**Severity findings**: CRITICAL: React ref expression translation dead code. HIGH: cross-target inconsistency. MEDIUM: component ref forwarding silently ignored, source span missing from `RefDeclIR`, `@ref` inside `@for` unguarded. LOW: API surface mismatch (scrollTo/scrollBy missing), HTMLDivElement hardcoded.

---

### TypeScript & React Specialist (Emma Rodriguez)

**Summary**: React-specific evaluation. The architecture is well-planned but execution is incomplete. The function `format_expr_ir_tsx` correctly contains the `nameRef.current` translation logic, but `ref_names` is never populated at any of ~25+ call sites. Worse, `ReactStmtFormatter` uses a separate trait-based expression formatting path (`ReactFormatExpr.format_identifier`) that has no ref awareness at all. Two independent fixes needed: thread `ref_names` through callers AND add ref awareness to the trait-based path.

**Severity findings**: CRITICAL: function body ref references generate `ReferenceError`. HIGH: `format_expr_ir_tsx` is dead code, `ReactStmtFormatter` bypasses ref translation. MEDIUM: HTMLDivElement hardcoded, test validates broken output. LOW: `@for` + `@ref` unresolved.

---

### Performance Specialist

**Summary**: Performance evaluation of the runtime execution model. `__omni_el()` cache with `isConnected` guard is well-designed (O(1) common case). However, three performance issues: (1) synchronous unbatchted signal propagation (each `signal.set()` runs all subscribers immediately), (2) @for wipe-and-rebuild strategy (O(items × subtree × bindings) per re-render), (3) per-property style effects generating N independent signal subscriptions per element. The FLIP ordering bug (H2) and @for nid collision (H1) are confirmed as HIGH.

**Severity findings**: HIGH: FLIP before-snapshot negated, @for nid collision. MEDIUM: no signal batching, per-property style effects. LOW: dead querySelector fallback, unnecessary clone.

---

### Maintainability Expert

**Summary**: Code quality and maintenance risk evaluation. The architectural spine is clean but template duplication is the largest DRY violation. Three `_elCache` lookup patterns (list.rs), three `__omni_effect` wrappers (style.rs), and duplicated `__omni_buildNidMap` (editor.rs) create drift risks. The `format_expr_ir_tsx` `ref_names` parameter is misleading — it exists but is never used. Six low-effort extraction/cleanup fixes would pay off quickly.

**Severity findings**: MEDIUM ×6: `__omni_buildNidMap` duplication, `_elCache` triplication, `__omni_effect` triplication, `ref_names` dead parameter misleading, HTMLDivElement hardcoded, ref forwarding silent drop.

---

### Pragmatic Enforcer (YAGNI Guardian)

**Summary**: YAGNI evaluation. Overall Phase 1 Necessity 7/10, Complexity 4/10, Ratio 0.57 — well below 1.5 threshold. The IIFE wrapper API surface (8 properties) seems large relative to current consumers but the 35-line code cost is negligible. Key finding: two Phase 2 items (ref_names threading for view nodes and statements) should be promoted to Phase 1 because they're the critical path to making React backend functional. Flutter GlobalKey codegen scores Necessity 1/10, Complexity 5/10, Ratio 5.0 — true YAGNI, defer indefinitely.

**Per-item Ratio assessment**: Phase 1 overall 0.57 (Approve), IIFE wrapper 0.60 (Accept), React useRef declaration 0.11 (Required), React expression translation 3.0 (High — half-complete), P2: ref_names threading 0.38 (Promote to P1), P2: Flutter GlobalKey 5.0 (True YAGNI).

**Severity findings**: HIGH: React expression translation unconnected. MEDIUM: scrollTo/scrollBy documented but not implemented.

---

### Rust Expert (Katya Volkov)

**Summary**: Rust code quality evaluation. ComponentScope atomic save/restore (ADR-034) is the standout architectural win. Two HIGH Rust concerns: (1) unconditional `el.children.clone()` in `try_inline_component` — wasted deep allocation for the common case, (2) `ref_nid_map: Vec<(String, String)>` allows silent duplicates — type doesn't enforce uniqueness invariant. The triple `_elCache` patterns are a MEDIUM DRY concern. Most concerns would not be caught by Clippy — they require semantic understanding of the codegen execution model.

**Severity findings**: HIGH: unnecessary view tree clone, `ref_nid_map` Vec type without uniqueness. MEDIUM ×4: `_elCache` triplication, `///` in JS output, redundant `comp.refs.is_empty()` check, complex signal resolution chain. LOW ×2: unnamed tuple in `for_state_names`, `Vec::contains` for theme dedup.

---

## Success Metrics and Follow-up

**Pre-merge checks**:
- All 837+ workspace tests pass with zero regressions
- React integration tests updated to assert `boxRef.current.style.transform` (not `box.style.transform`)
- New test: React ref in handler → correct `nameRef.current` translation
- New test: React ref in effect/lifecycle/computed → correct translation
- Clippy clean, rustfmt clean

**Follow-up plan**:
1. Execute P0-1 through P0-3 (estimated 2-3 days total)
2. Update `test_react_ref_in_handler` to assert correct output
3. Run `cargo test` full suite
4. Create ADR-037 for signal batching decision (P2-3)
5. Update ADR-036 status to "Implemented (Phase 1 Complete)" after P0 fixes
6. Schedule specialist-review for Flutter/Vue ref support when those backends are activated

---

## References

- ADR-036: Element Reference System
- ADR-028: OmniView Debug Runtime Trade-offs (D3: synchronous subscriber notification)
- ADR-034: ComponentScope Inlining Isolation
- Prior review: `.architecture/reviews/component-omniview-high-frequency-element-manipulation.md` (2026-07-29)
- Memory: `omniview-review-high-freq-dom-2026-07-29`
- Memory: `omnilang-codegen` — Element Reference System implementation
