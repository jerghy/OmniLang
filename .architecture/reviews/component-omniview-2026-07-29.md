# Architecture Review: OmniView Codegen

**Date**: 2026-07-29
**Review Type**: Component
**Reviewers**: Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer

## Executive Summary

OmniView continues to be a well-architected multi-target UI compiler framework. The Language-First design philosophy (signals, design blocks, components as language keywords) remains its strongest differentiator. The recent wave of refactoring (element.rs split, shared view-tree traversal, canonical easing, cross-target parity fixes) has materially improved code quality. The test suite at 800+ tests with zero clippy warnings provides a strong safety net.

However, the review surfaced one structural correctness issue (**HIGH**: multi-instance component state collision in non-@for context), two systemic process gaps (**HIGH**: cross-target feature parity has no automated guard; **MEDIUM**: runtime memory leaks in `__omni_effect` and `__omni_runtimeErrors`), and a consistent theme of Generator struct complexity that is approaching a maintainability threshold.

**Overall Assessment**: Strong

**Key Findings**:
- Non-@for multi-instance components share global signals, producing silently wrong UI when the same component appears twice with different props
- Cross-target feature parity is enforced only by manual review — 4 features were found silently dropped in 3 backends during the last review cycle
- `__omni_effect` has an unbounded dependency leak (matching the pattern just fixed in `__omni_computed`), causing O(n) memory growth per signal change
- 46-field Generator struct with 3 implicit mode flags creates a state machine that spans 15+ source files

**Critical Actions**:
- Document the multi-instance component limitation and plan per-instance state isolation
- Fix the `__omni_effect` dependency leak (4-line change in runtime.rs)
- Add cross-target output equivalence tests to CI

---

## System Overview

OmniView is the primary codegen backend of the OmniLang compiler. It translates `.omni` source files through a pipeline (Lexer → Parser → Semantic Analysis → IR Lowering → Codegen) into HTML/CSS/JS with an inline signal runtime. It is the reference implementation for 9 total codegen targets.

**Key technologies**: Rust compiler (omnilang crate), inline JavaScript signal runtime (~200 lines), CSS variable-based design system, compile-time component inlining, Deferred JS swap mechanism.

**Architecture style**: Single-pass stateful code generation with 46-field accumulator struct. Module boundaries follow feature decomposition (motion, design, store, view, editor, debug).

**Scope**: 27 source files (~4800 lines), ~2300 test lines, 800+ tests, 24 ADRs from recent review/fix cycles.

---

## Individual Member Reviews

### Systems Architect — Systems Architecture

**Perspective**: Examines how components (codegen backends, IR modules, runtime) interact as a coherent whole, focusing on data flow direction, coupling mechanisms, boundary integrity, and architecture scalability toward 9+ compilation targets.

#### Key Observations
- 9-target architecture with OmniView as feature-complete reference (2.6x larger than next backend)
- Stateful 46-field Generator with implicit state machine spanning 15 files
- Two fundamentally different instance-isolation strategies (per-instance @for vs shared global for non-@for)
- Deferred JS swap pattern creates implicit output routing
- Cross-target shared infrastructure converging (ADR-023 eliminated 8 duplicate tree-walkers)

#### Strengths
1. **Clean compiler pipeline separation**: Strictly linear Lexer→Parser→IR→Semantic→Codegen with no circular dependencies
2. **Feature-gated backends**: Each backend behind its own Cargo feature flag, isolating bugs and keeping compilation fast
3. **Shared cross-target infrastructure**: `view_tree_any`, canonical easing, trait-based formatters prevent backend drift
4. **Per-instance state for @for templates**: `__omni_forGetState` with DOM-scoped `__omniState` correctly isolates loop instances
5. **Deferred JS mechanism**: Correctly solves the DOM-availability problem without two-pass traversal

#### Concerns

**H1: Multi-instance component state collision in non-@for context** (HIGH)
- **Location**: `component_html.rs:142-168`, `store.rs:199-202`
- **Issue**: Same component type appearing twice in a view shares global signals prefixed by component name. Second instance's props overwrite first instance's signal values — both instances reflect the last-set value.
- **Why it matters**: `\<TabPanel color="red"/\>\<TabPanel color="blue"/\>` produces two "blue" panels. Non-crashing incorrect behavior is the hardest to debug.
- **Recommendation**: Document limitation immediately; plan per-instance isolation using existing @for pattern (XL effort) or unique per-instance signal names (L effort).

**H2: Cross-target feature parity without compiler catching gaps** (HIGH)
- **Location**: `codegen/mod.rs:8-33`, ADR-021
- **Issue**: New `ElementIR` fields or `ViewTree` variants don't cause compile errors in backends that don't handle them. Features silently drop.
- **Why it matters**: Each new feature requires manual 9-backend verification. 4 gaps already found in one review session.
- **Recommendation**: Add cross-target output equivalence test matrix in CI (M effort).

**M1: Generator as monolithic state accumulator** (MEDIUM)
- **Location**: `mod.rs:436-535` (struct), behavior across ~15 files
- **Issue**: Mode flags (`in_for_template`, `css_only`, `editor_mode`) create implicit invariants with 5-method fan-out.
- **Recommendation**: Consolidate flags into `GenerationMode` enum; extract @for-specific accumulators into sub-struct.

**M2: Deferred JS buffer swap as implicit output routing** (MEDIUM)
- **Location**: `view/mod.rs:36-60` (swap), `mod.rs:379-388` (flush)
- **Issue**: `std::mem::swap(self.js, self.deferred_js)` creates invisible redirect through entire call stack.
- **Recommendation**: Replace swap with explicit buffer parameter in function signatures (M effort).

**M3: css_only dual-path drift** (MEDIUM)
- **Location**: `view/element.rs:481-510` vs `556-586`
- **Issue**: Two near-identical `if self.css_only` blocks (void-tag vs normal-tag paths).
- **Recommendation**: Extract `capture_css_only_metadata()` helper (S effort).

**M4: No shared ViewTree transformation utility** (MEDIUM)
- **Location**: Each backend independently implements view-to-X transformation
- **Issue**: New `ViewTree` variant requires 9 independent backend edits.
- **Recommendation**: Create shared `ViewTreeVisitor` trait (M-L effort).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Document multi-instance limitation (H1) | S |
| Immediate | Extract css_only metadata helper (M3) | S |
| Short-term | Replace deferred JS swap with explicit buffer param (M2) | M |
| Short-term | Shared ViewTree visitor trait (M4) | M |
| Long-term | Cross-target integration test matrix (H2) | L |
| Long-term | Generator decomposition (M1) | L |
| Long-term | Non-@for per-instance state isolation (H1) | XL |

---

### Domain Expert — Domain-Driven Design

**Perspective**: Evaluates whether the codegen's architecture faithfully represents the language's domain concepts (component, signal, design, motion) without leaking infrastructure concerns, and whether ubiquitous language remains consistent across the compilation pipeline.

#### Key Observations
- Clean conceptual mapping from language keywords to generated code at every pipeline stage
- "Component" concept undergoes fundamental transformation at codegen layer — boundaries erased for signal isolation
- 46-field Generator conflates 6 distinct concerns (signal, view, motion, design, editor, @for)
- Module-level motion/timeline emission unintentionally duplicated (N+1 times for N components)
- Text-node interpolation shorthand conflates "literal text" and "expression"

#### Strengths
1. **Ubiquitous language consistency**: `signal→SignalIntent→__omni_signal()`, `design→DesignIR→:root variables` — concept tracing is unambiguous
2. **Design/theme at correct layer**: ADR-016 correctly identified CSS cascade as theme-switching layer, avoiding Store-based `setProperty` mutation
3. **Well-structured event data model**: `EventDataKind` enum with typed schema, field extraction only for used fields
4. **Phase separation respected**: Each stage produces well-typed, validated output; IR is single source of truth
5. **Source map and element tree bridge output-to-source gap**: Structured mapping between DOM and .omni source

#### Concerns

**M1: Module-level motion/timeline duplicated in per-component emission** (MEDIUM)
- **Location**: `mod.rs:256-280` (first emission), `store.rs:271-288` (re-emitted per component)
- **Issue**: Each motion/timeline definition appears N+1 times in generated JS for N components. Non-editor mode produces duplicate function declarations in same scope.
- **Recommendation**: Remove motion emission from `generate_component()` or gate behind `editor_mode` (S effort).

**L1: Conditional/@for logic split across files by mode, not domain concept** (LOW)
- **Location**: `view/condition.rs`, `editor.rs:122`, `view/list.rs` (empty), `list.rs`, `list_render.rs`
- **Issue**: `@if` conditional split between view/condition.rs (css_only) and editor.rs (live); `view/list.rs` is empty placeholder.
- **Recommendation**: Co-locate conditional rendering in `view/condition.rs`; populate or remove `view/list.rs` (S effort).

**L2: css_only/editor_mode/debug_mode as implicit untyped state machine** (LOW)
- **Location**: `mod.rs:129-134, 177-179`
- **Issue**: Three booleans create 8-state space with only 4 valid combinations. Invariant only checked at debug_assert (release builds unchecked).
- **Recommendation**: Replace with `CodegenMode` enum (M effort).

**L3: Text-node interpolation shorthand conflates literal text and expression** (LOW)
- **Location**: `view/element.rs:716-736`
- **Issue**: `{expr}` pattern match is fragile — can't display literal `{count}` text without being reinterpreted.
- **Recommendation**: Document behavior or require explicit `ViewTree::Interpolation` variant.

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Fix motion/timeline duplicate emission (M1) | S |
| Short-term | Co-locate conditional rendering logic (L1) | S |
| Short-term | Populate or remove `view/list.rs` (L1) | S |
| Long-term | Model compilation mode as typed enum (L2) | M |

---

### Security Specialist — Security Analysis

**Perspective**: Evaluates attack surface and defense-in-depth: vulnerabilities in code generation pipeline, Tauri/FFI boundaries, runtime bridge code, and CSP/capability configuration.

#### Key Observations
- Architecture is security-conscious by design with consistent input validation and output escaping
- Production/debug CSP split is correct (`script-src 'self'` in production, `'unsafe-inline'` and `'unsafe-eval'` in dev only)
- Eval usage is sandbox-contained within opaque-origin iframe
- Filesystem capabilities intentionally broad for editor functionality
- All 6 `unsafe` blocks confined to FFI boundaries with `// SAFETY:` comments

#### Strengths
1. **Consistent output sanitization**: `escape_html()` handles `&<>"'`, `sanitize_css_value()` blocks `javascript:`, `expression()`, `url()`, etc.
2. **Defense-in-depth for debug window eval chain**: Sandboxed iframe + postMessage origin validation + production CSP without `'unsafe-eval'`
3. **Strong path traversal protection**: `sanitize_path()` canonicalizes, resolves symlinks, verifies containment
4. **Input size limits at every boundary**: WASM exports (10MB), Tauri commands (10MB), `dw_log` (4KB), log file (10MB cap)
5. **Identifier-based nid validation**: Lexer restricts identifiers to safe characters, preventing CSS selector injection in `__omni_el()`

#### Concerns

**M1: Runtime global cross-contamination across compilations** (MEDIUM)
- **Location**: `runtime.rs:20-21`, `editor.rs:43-44`, `debug.rs:19-30`
- **Issue**: Hot-replace re-assigns `window.__omni_subs` without clearing old signal entries. `__omni_forHandlers` recreated as new `Map` inside IIFE each compile — old event handlers on existing DOM elements reference stale entries.
- **Recommendation**: Clear and re-populate handler map across recompilations (M effort).

**M2: `window.__omni_runtimeErrors` grows unboundedly** (MEDIUM)
- **Location**: `runtime.rs:96`
- **Issue**: Every runtime error appends to array with no size cap. Long-running editor session accumulates thousands of entries.
- **Recommendation**: Cap at 100-1000 entries with oldest-eviction (S effort).

**M3: State sync command accepts unbounded data** (MEDIUM)
- **Location**: `poiesis/src-tauri/src/commands/state_sync.rs:16-24`
- **Issue**: `store_snapshot` and `pid_index` have no size limits. Multi-GB strings could cause OOM.
- **Recommendation**: Add size limit consistent with `MAX_SOURCE_SIZE` pattern (S effort).

**L1: `__omni_reportError` posts to parent without destination validation** (LOW)
- **Location**: `runtime.rs:98-105`
- **Issue**: Error data sent to `parent` via postMessage could leak in non-sandboxed contexts. Currently mitigated by sandboxed iframe.
- **Recommendation**: Feature-gate or remove from production runtime path.

**L2: WASM analysis cache persists across page navigations** (LOW)
- **Location**: `omnilang-wasm/src/lib.rs:246-248`
- **Issue**: Thread-local cache lives for WASM module lifetime; no explicit clear function from JS side.
- **Recommendation**: Add `clear_cache()` WASM export (S effort).

**L3: Debug window CSS selector injection via nid** (LOW)
- **Location**: Debug window `querySelector("[data-nid=\""+m.nid+"\"]")`
- **Issue**: nid value from postMessage concatenated into CSS selector without sanitization.
- **Recommendation**: Apply CSS-identifier sanitizer for defense-in-depth (S effort).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Cap `__omni_runtimeErrors` array (M2) | S |
| Immediate | Add size limits to `sync_store_state` (M3) | S |
| Short-term | Sanitize nid in debug window CSS selector (L3) | S |
| Short-term | Fix `__omni_forHandlers` across hot-replace (M1) | M |
| Short-term | Add `clear_cache()` WASM export (L2) | S |
| Long-term | Replace `(0, eval)` with structured loading | L |
| Long-term | Audit filesystem capability scope | M |

---

### Maintainability Expert — Code Quality & Technical Debt

**Perspective**: Evaluates how the OmniView compiler's architecture facilitates long-term evolution, developer onboarding, and safe modification. Focus on code organization, complexity hotspots, duplication, and cost of change.

#### Key Observations
- Single-pass accumulation pattern is necessary but cognitively costly (7 buffers, interleaved writes)
- Four ViewTree walkers existed; three are already consolidated (ADR-023); one standalone remains
- `generate_component` (410 lines) has at least 8 distinguishable responsibilities
- Three instances of copy-paste with minor variation (mutation operators, motion triggers, effect cleanup)
- Module boundaries are clean with tree-structured dependencies, no circular imports

#### Strengths
1. **Excellent module decomposition**: 27 files, each with focused concern; recent extract-refactors well-executed
2. **Thorough tests**: 800+ tests with targeted unit and regression coverage
3. **ADR-023 high-impact consolidation**: Eliminated 8 duplicate tree-walkers
4. **Consistent naming and structure**: Uniform `impl OmniViewGenerator` patterns
5. **Positive refactoring trajectory**: element.rs trending from 839→762 lines, sub-module extraction ongoing

#### Concerns

**M1: `generate_component` has too many responsibilities — 410 lines** (MEDIUM)
- **Location**: `store.rs:115-525`
- **Issue**: 17 sequential responsibilities including state prep, signal registration, store wiring, function emission, view emission, CSS emission, lifecycle hooks, enter replay, factory return.
- **Recommendation**: Extract `prepare_component_context()` (lines 128-196) and `finalize_component()` (lines 481-524); ~80 lines of entry/exit cleanup separated (M effort).

**M2: Named/inline motion trigger dispatch structurally duplicated** (MEDIUM)
- **Location**: `motion_triggers.rs:349-599`
- **Issue**: Two adjacent `match trigger.kind` blocks (10 arms named, 6 arms inline) emitting identical JS structure where only anim_call differs.
- **Recommendation**: Build `anim_call` string before single match; eliminates ~150 lines of duplicated dispatch (M effort).

**M3: `emit_mutation` repeats signal-vs-non-signal pattern 6 times** (MEDIUM)
- **Location**: `stmt.rs:218-346`
- **Issue**: Six arithmetic mutation variants each independently repeat the same signal-detection and value-resolution pattern. Only operator symbol and default value differ.
- **Recommendation**: Extract `emit_mutation_arithmetic` helper with operator and default as parameters (S effort).

**M4: Effect cleanup chaining duplicated for editor/non-editor modes** (MEDIUM)
- **Location**: `store.rs:388-436`
- **Issue**: Lines 388-412 and 413-436 are identical except for target buffer (`comp_mount_js` vs `mount_js`).
- **Recommendation**: Assign target buffer to local variable before duplicated block (S effort).

**M5: `emit_element_props` is 183 lines with 5 side-effects** (MEDIUM)
- **Location**: `view/element.rs:82-265`
- **Issue**: Writes to `self.js`, `self.for_attr_bindings`, `self.warnings`, `self.for_class_bindings`, `self.element_classes`.
- **Recommendation**: Extract class validation and design variant resolution into self-contained methods.

**L1: `emit_element_html_close` takes 9 parameters** (LOW)
- **Location**: `view/element.rs:462-631`
- **Issue**: Fragile parameter list. css_only blocks duplicated within.
- **Recommendation**: Extract `record_element_mapping()` to reduce parameter count.

**L2: `collect_for_events` duplicates ViewTree traversal** (LOW)
- **Location**: `utils.rs:50-115`
- **Issue**: Cannot use `view_tree_any` due to mutable reference capture conflict.
- **Recommendation**: Add `view_tree_walk` variant that passes `&ElementIR` to callback (deferred, L effort).

**L3: Inline JS template strings are hard to maintain** (LOW)
- **Location**: 7 files with ~150+ `writeln!`/`push_str` calls
- **Issue**: Generated JS invisible to JS linters/formatters; runtime API changes require fragile grep-and-replace.
- **Recommendation**: Extract common JS patterns into const templates or shared generation functions (L effort).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Extract arithmetic helper in stmt.rs (M3) | S |
| Immediate | Unify effect cleanup target buffer (M4) | S |
| Short-term | Consolidate motion trigger dispatch (M2) | M |
| Short-term | Extract prepare/finalize from generate_component (M1) | M |
| Long-term | Add `view_tree_walk` utility (L2) | L |
| Long-term | JS template macro or codegen DSL (L3) | L |

---

### Performance Specialist — Runtime & Compile Performance

**Perspective**: Focuses on generated runtime behavior and compiler throughput — runtime memory footprint, reactivity performance, DOM manipulation patterns, and compile-time allocation.

#### Key Observations
- Fine-grained reactivity with direct DOM manipulation — no virtual DOM, no diffing (architecturally sound for UI perf)
- `__omni_computed` subscriber leak recently fixed (commit `8ecbaa04`), but same pattern persists in `__omni_effect`
- `__omni_el()` does no caching — repeated `querySelector` calls from effects on same element
- Compile-time `escape_html` uses 5 chained `.replace()` calls (5 intermediate allocations per call)
- 7 independent IR scans in `generate()` cause CPU cache misses for large programs

#### Strengths
1. **Fine-grained reactivity avoids VDOM overhead**: Direct DOM patching, minimal mutation per signal change
2. **Conditional motion runtime embedding**: `project_uses_motion()` guards 46KB animation runtime
3. **Deferred JS eliminates FOUC**: DOM inserted before event binding prevents "element not found" errors
4. **Pre-compiled @for bindings avoid eval**: Compile-time binding compilation safer and faster than `new Function()`
5. **Event delegation in @for**: Single listener per event type on container, not per-item listeners

#### Concerns

**M1: `__omni_effect` dependency leak** (MEDIUM)
- **Location**: `runtime.rs:84-89`
- **Issue**: `e.run = fn` directly without wrapper that clears `e._deps` before re-execution. Each signal change adds new cleanup closure without removing stale ones. Same bug pattern just fixed in `__omni_computed` (commit `8ecbaa04`).
- **Concrete estimate**: 10 reactive bindings × 60fps signal change × 64 bytes per closure = ~1.4MB leaked per hour of continuous animation.
- **Recommendation**: Wrap `fn` in cleanup-before-exec wrapper matching `__omni_computed` pattern. 4-line change (S effort).

**M2: Compile-time view tree clone on every component inlining** (MEDIUM)
- **Location**: `component_html.rs:118`
- **Issue**: `child_view.clone()` for every inlining instance. For @for over 100 items with nested sub-components, clones compound.
- **Recommendation**: Refactor borrow structure to avoid clone; move `view_to_html` into non-`&mut self` helper (M effort).

**M3: `__omni_el()` repeated querySelector calls** (MEDIUM)
- **Location**: Every `__omni_effect` in generated output, `runtime.rs:109-111`
- **Issue**: No element reference caching. N effects on M unique elements = N querySelector calls per signal change even when N > M.
- **Recommendation**: Cache element reference on first query in a closure-level map; clear on @for re-render (S effort).

**L1: `escape_html` uses 5 chained replace calls** (LOW)
- **Location**: `utils.rs:177-183`
- **Issue**: 5 intermediate String allocations per escaped value. Measurable in WASM compilation.
- **Recommendation**: Single-pass character scan (S effort).

**L2: 7 independent IR scans in `generate()`** (LOW)
- **Location**: `mod.rs:229-388`
- **Issue**: Stores, functions, motions, components, designs each scanned independently. Working set evicted and reloaded 7×.
- **Recommendation**: Fuse scans where possible (M effort).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Fix `__omni_effect` dependency leak (M1) | S |
| Immediate | Cache `__omni_el()` results (M3) | S |
| Short-term | Rewrite `escape_html` as single-pass (L1) | S |
| Short-term | Replace format! + push_str with write! in non-hot paths | S |
| Long-term | Eliminate view tree clone in try_inline_component (M2) | M |
| Long-term | Fuse IR scans in generate() (L2) | M |

---

### Implementation Strategist — Change Sequencing & Risk

**Perspective**: Evaluates HOW and WHEN changes should be implemented — blast radius analysis, reversibility design, technical and team readiness, and migration planning.

#### Key Observations
- OmniView is 2.6x larger than next backend (React), creating asymmetric feature gap
- 4-buffer JS architecture is most operationally risky subsystem
- `name_renames`/`signal_names` save/restore protocol repeated in 3 locations with 2 modes
- Cross-target safety net incomplete and deferred without timeline (ADR-021)
- 30+ commits landed in single batch merge window including refactoring, features, and bugfixes

#### Strengths
1. **Good modular extraction trajectory**: element.rs split into view/element, view/events, view/style — right pattern for managing blast radius
2. **Comprehensive motion snapshot tests**: 20 tests with 13 insta snapshots covering all trigger paths
3. **Cross-target compilation tests enforced in CI**: 50+ golden fixtures compiled on all P0 backends
4. **Mode isolation via css_only and debug_mode flags**: Clean separation with assertions
5. **ADR documentation culture**: Blast radius, reversibility, sequencing documented in decisions

#### Concerns

**M1: Cross-target output equivalence has no automated guard** (MEDIUM)
- **Location**: `tests/cross_target.rs`, ADR-021 deferred items
- **Issue**: Fixtures compile without errors but output is not compared for semantic equivalence. 4 silent feature drops found in manual review.
- **Recommendation**: Add output-level structural invariants to cross_target tests (S effort for initial coverage).

**M2: Generator struct coupling — 3-file minimum for any new field** (MEDIUM)
- **Location**: `mod.rs:436-535` (struct + test_new + generate)
- **Issue**: New field requires struct definition, generate() initialization, test_new() initialization updates across 17+ impl blocks.
- **Recommendation**: Group 5 output-collection fields into `GeneratorOutput` sub-struct (S effort).

**M3: Component inlining has highest bug surface per change** (MEDIUM)
- **Location**: `component_html.rs:17-239`
- **Issue**: Two execution paths (normal/@for) with different state management rules. No unit test for restore invariants.
- **Recommendation**: Add pre/post state comparison tests for try_inline_component (S effort).

**M4: Name collision risk between component and store signal namespace** (MEDIUM)
- **Location**: `store.rs:40-82`, `mod.rs:152-195`
- **Issue**: Store and component signals both stored in `signal_names` set. Colliding names produce wrong `.value` access.
- **Recommendation**: Add name collision diagnostic during semantic analysis (S effort).

**L1: Deferred JS lacks execution ordering documentation** (LOW)
- **Location**: `view/mod.rs:31-93`
- **Issue**: 6-step execution order implicit in code flow but undocumented.
- **Recommendation**: Add module-level doc comment (S effort).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Add cross-target output equivalence assertions (M1) | S |
| Immediate | Document JS execution order in view/mod.rs (L1) | S |
| Immediate | Add try_inline_component restore invariant tests (M3) | S |
| Short-term | Group output fields into GeneratorOutput sub-struct (M2) | S |
| Short-term | Add name collision warning (M4) | S |
| Long-term | Build deferred cross-target test matrix (M1) | L |
| Long-term | Establish capability budget per backend | M |

---

### AI Engineer — AI Integration & Observability

**Perspective**: Evaluates observability, evaluation, instrumentation, and readiness for AI-assisted development workflows. System has zero AI/LLM integration — review focuses on runtime introspection quality.

#### Key Observations
- Zero AI/LLM integration exists anywhere in the codebase
- `__omni_reportError` has multi-channel reporting but incomplete coverage
- `__omni_runtimeErrors` array has no size bound, no deduplication
- Evaluation framework is traditional compiler testing, not AI eval
- System architecture (structured outputs, source maps, signal introspection) is AI-ready

#### Strengths
1. **`__omni_reportError` multi-channel**: Console, structured array, parent-frame postMessage — well-designed minimal observability substrate
2. **`__omni_effect` try/finally for state restoration**: Prevents cascaded tracking corruption on subscriber throw
3. **Signal introspection via `__omni_subs` Map**: Complete access to values via `.peek()`, debug bridge demonstrates querying
4. **Editor bridge structured UI introspection**: nidMap + element_tree enables full UI tree reconstruction without HTML parsing
5. **SourceMap output-to-source traceability**: Every output span mapped to .omni source location — critical for AI-assisted debugging

#### Concerns

**M1: Event handler errors silently swallowed by browser** (MEDIUM)
- **Location**: `view/events.rs:136-169` (all addEventListener emission points)
- **Issue**: Event handlers lack try/catch wrapper. Browser catches and console-logs but `__omni_reportError` never invoked — errors invisible to monitoring.
- **Recommendation**: Wrap each addEventListener callback in try/catch → `__omni_reportError("event:" + name, e)` (S effort).

**M2: `__omni_runtimeErrors` unbounded growth** (MEDIUM)
- **Location**: `runtime.rs:95-96`
- **Issue**: No limit, no deduplication. Long editor sessions accumulate memory from recurring errors.
- **Recommendation**: Cap at 100 entries with oldest-eviction (S effort).

**M3: No runtime performance instrumentation** (MEDIUM)
- **Location**: Entire generated JS runtime
- **Issue**: Zero timing measurements for effect duration, list re-render, animation frames, event handlers.
- **Recommendation**: Opt-in `performance.now()` wrapping in `__omni_effect`, gated behind `window.__omni_enableMetrics` (M effort).

**L1: Eval framework has no runtime behavior coverage** (LOW)
- **Location**: `.claude/evals/omnilang-compiler.md`
- **Issue**: Evals test compilation success but not runtime DOM behavior.
- **Recommendation**: Add 1-2 runtime-in-a-box evals using jsdom/headless test (M effort).

**L2: No standardized error context taxonomy** (LOW)
- **Location**: `view/mod.rs:172`, `list_render.rs:91,134,164,375`, `list.rs:120`
- **Issue**: Ad-hoc string literals for error contexts — no registry or enum.
- **Recommendation**: Define `error_contexts.rs` constants; document in generated JS header (S effort).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Wrap event handlers in try/catch + reportError (M1) | S |
| Immediate | Cap `__omni_runtimeErrors` array (M2) | S |
| Short-term | Add opt-in performance measurement (M3) | M |
| Short-term | Standardize error context strings (L2) | S |
| Long-term | Add runtime behavior evals (L1) | M |
| Long-term | Define structured telemetry JSON schema | L |

---

### Pragmatic Enforcer — YAGNI & Simplicity

**Perspective**: Rigorously questions whether proposed solutions, abstractions, and features are actually needed right now, pushing for the simplest approach that solves the immediate problem. **Mode**: Balanced (config.yml pragmatic_mode.intensity).

#### Key Observations
- 23-file module split largely justified — each file has clear responsibility
- Three boolean flags all used in production (no dead code)
- Deferred-JS pattern is good engineering, not over-engineering
- Pre-compiled @for bindings use shared mutable state — fragile ordering dependency
- Team shows healthy awareness of dead code (recent `keyframe_to_config` removal)

#### Strengths
1. **Motion runtime lazy loading**: `project_uses_motion()` guards 46KB runtime — don't pay for what you don't use
2. **Per-component JS deltas for hot-replace**: Appropriate complexity solving real performance problem
3. **`view_tree_any` shared utility**: Rare case where abstraction REDUCED total complexity (net -119 lines)
4. **Single struct with field-based state**: No traits, generics, or DI — simplest approach that works
5. **No trait abstraction within OmniView backend**: Direct `impl` methods avoid 1-impl-per-trait anti-pattern

#### Pragmatic Analysis

| Concern | Necessity | Complexity | Ratio | Verdict |
|---------|-----------|------------|-------|---------|
| `view/list.rs` empty module | 0/10 | 2/10 | N/A | Remove |
| `list.rs`/`list_render.rs` split | 3/10 | 4/10 | 1.33 | Merge |
| `debug.rs` standalone file | 4/10 | 3/10 | 0.75 | Optional merge |
| `render_branch_body` visibility | 2/10 | 1/10 | 0.50 | Downgrade to private |

#### Concerns

**M1: Empty module placeholder `view/list.rs`** (MEDIUM — YAGNI violation)
- **Location**: `omnilang/src/codegen/omniview/view/list.rs:1-9`
- **Issue**: File contains only a doc comment explaining it's a placeholder. Zero code. Added speculatively.
- **Recommendation**: Remove file and `mod list;` declaration (S effort — minutes).

**L1: Split `list.rs`/`list_render.rs` — tightly coupled two-phase pipeline** (LOW)
- **Location**: `list.rs` (156 lines) + `list_render.rs` (385 lines) = 541 total
- **Issue**: Two methods always called together, share 5 struct fields. File boundary adds indirection without enabling reuse.
- **Recommendation**: Merge `list_render.rs` into `list.rs` (S effort).

**L2: `debug.rs` single-method file** (LOW)
- **Location**: `debug.rs` (129 lines)
- **Issue**: One method, one caller, disabled-by-default flag. Implicit protocol with editor.rs yield pattern.
- **Recommendation**: Merge into `editor.rs` as private method (S effort — optional).

**L3: `render_branch_body` as `pub(super)` — misleading visibility** (LOW)
- **Location**: `view/condition.rs:59-71`
- **Issue**: Marked `pub(super)` but only called within same file.
- **Recommendation**: Downgrade to `fn` (private). (S effort — 5 minutes).

#### Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| Immediate | Remove empty `view/list.rs` module (M1) | S |
| Short-term | Merge `list_render.rs` into `list.rs` (L1) | S |
| Short-term | Merge `debug.rs` into `editor.rs` (L2) | S |
| Short-term | Downgrade `render_branch_body` visibility (L3) | S |

---

## Collaborative Discussion

### Cross-Cutting Themes

Four themes appeared across 3+ expert reviews, indicating systemic architectural concerns:

#### Theme 1: Cross-Target Feature Parity Is Unenforced (4 experts)

**Systems Architect (H2)**, **Implementation Strategist (M1)**, **Domain Expert (M1)**, **Maintainability Expert**

The same `.omni` source produces different behavior across backends because there is no automated output equivalence testing. ADR-021 documented 4 silent feature drops (class_map_bindings in Flutter/Vue/Dioxus, empty_body in Vue/Dioxus, slot in Vue, key_expr in Vue/Dioxus) — found only through manual review. OmniView at 4800 lines (2.6× the nearest backend) creates an asymmetric development pattern where features land first in OmniView and the gap to other backends grows invisibly.

**Synthesis**: This is the highest-leverage process improvement. Each new language feature creates an invisible migration debt to 8 other backends. The fix is a CI-level cross-target output equivalence test matrix — structural checks, not brittle exact-output comparisons.

#### Theme 2: Generator Struct Complexity Approaching Threshold (5+ experts)

**Systems Architect (M1)**, **Maintainability Expert (M1,M2,M3,M4)**, **Domain Expert (L2)**, **Implementation Strategist (M2)**, **Pragmatic Enforcer (M1,L1,L2)**

The 46-field `OmniViewGenerator` struct with 3 implicit mode flags (`css_only`, `editor_mode`, `debug_mode`) creates a state machine spanning 15+ files. Adding a new field requires touching at least 3 locations (struct definition, generate() init, test_new()), and understanding any method requires knowing which flags affect its behavior. Multiple sub-structures are candidates for extraction: `GeneratorOutput` (5 collection fields), `ForLoopState` (5 binding fields), `CodegenMode` (3 booleans → typed enum).

**Synthesis**: The struct is not yet unmaintainable — it's well-documented and tests are thorough. But the growth trajectory is concerning. The recommended approach is gradual sub-struct extraction rather than a big-bang refactor: start with `GeneratorOutput` and `ForLoopState`, both low-risk extractions with clear boundaries.

#### Theme 3: Runtime Memory/Leak Issues (3 experts)

**Performance Specialist (M1)**, **AI Engineer (M2)**, **Security Specialist (M2)**

Three independent experts identified unbounded memory growth in the generated runtime:
- `__omni_effect` subscriber leak (same pattern just fixed in `__omni_computed`)
- `__omni_runtimeErrors` array unbounded growth
- `__omni_el()` repeated querySelector without caching

All three are S-effort fixes in `runtime.rs` — a single 150-line file. The `__omni_effect` leak is the most urgent because it's on the hot path (every reactive binding) and was already recognized as a bug in the sibling `__omni_computed` code.

#### Theme 4: Component Instance Isolation Gap (2 experts but HIGH severity)

**Systems Architect (H1)**, **Implementation Strategist (M3)**

The fundamental architectural limitation: non-@for component instances share global signals. Two instances of the same component type with different props produce identical, silently wrong output. This is a correctness bug, not a performance or style issue. The existing @for pattern demonstrates the solution (DOM-scoped `__omniState`), but extending it to all component inlining is an XL effort that touches the core runtime and codegen.

### Areas of Debate

**Topic: Deferred JS swap vs. explicit buffer parameter**

- **Systems Architect**: Recommends replacing `std::mem::swap(self.js, self.deferred_js)` with explicit buffer parameter — "invisible redirect through entire call stack"
- **Performance Specialist**: Notes the swap correctly solves DOM-before-events problem — "good engineering, not over-engineering"
- **Implementation Strategist**: Middle ground — keep the mechanism, add documentation about execution ordering guarantees (S effort) before considering refactoring (M effort)

**Resolution**: Document first (S effort, all agree). Defer refactoring decision based on whether new contributors actually struggle with the pattern. The mechanism has been stable across 800+ tests.

**Topic: Pragmatic enforcer's file merge recommendations vs. maintainability expert's module praise**

- **Pragmatic Enforcer**: Merge `list_render.rs` into `list.rs` (541 lines total, well under 800-line limit), merge `debug.rs` into `editor.rs`, remove empty `view/list.rs`
- **Maintainability Expert**: Module decomposition is "excellent" — 27 files with focused concerns
- **Domain Expert**: `@if` conditional logic "split across files based on mode, not domain concept"

**Resolution**: Partial merge. Remove empty `view/list.rs` (no content to preserve). Merge `list_render.rs` → `list.rs` (both files form a single pipeline). Keep `debug.rs` separate (Tauri IPC vs postMessage is a genuine conceptual difference). Merged list.rs would be ~541 lines — within file size guidelines.

### Priorities Established

**Critical (Address Immediately — 0-2 weeks)**:
1. Fix `__omni_effect` dependency leak in runtime — correctness bug on hot path, 4-line change
2. Document multi-instance component state limitation — prevents users from debugging silent wrong behavior
3. Cap `__omni_runtimeErrors` array — prevents memory exhaustion in editor sessions

**Important (Address Soon — 2-8 weeks)**:
1. Add cross-target output equivalence tests to CI — prevents silent feature regression
2. Wrap event handler callbacks in try/catch — closes largest runtime error visibility gap
3. Fix module-level motion/timeline duplicate emission — eliminates N+1 output bloat
4. Extract arithmetic helper in stmt.rs — highest fix/benefit ratio in code duplication
5. Remove empty `view/list.rs` and merge `list_render.rs` → `list.rs`
6. Cache `__omni_el()` results — reduces DOM query overhead for multi-effect signals

**Nice-to-Have (2-6 months)**:
1. Generator struct decomposition (sub-struct extraction)
2. Shared ViewTree visitor trait across backends
3. Compilation mode as typed enum (CodegenMode)
4. Runtime performance instrumentation (opt-in)
5. Cross-target full test matrix (deferred from ADR-021)
6. Non-@for per-instance state isolation

---

## Consolidated Findings

### Strengths

1. **Language-First design with correct layer placement**: Signals, design blocks, components as language keywords — consistently mapped through pipeline. Theme switching correctly placed at CSS cascade layer (ADR-016), not signal/mutation layer.

2. **Deferred JS swap mechanism**: Elegantly solves the universal reactive compiler problem (DOM must exist before event binding). Single-pass, no two-phase traversal. Combined with pre-compiled @for bindings and event delegation, the runtime generation is well-optimized.

3. **Comprehensive test infrastructure**: 800+ tests with insta snapshots for motion triggers, cross-target compilation tests in CI, targeted unit and regression suites. The safety net is strong enough to support aggressive refactoring.

4. **Security-conscious architecture**: Consistent output sanitization (HTML escaping, CSS value blocking), defense-in-depth for debug window eval chain (sandboxed iframe + origin validation + production CSP), path traversal protection, input size limits at every boundary.

5. **Well-structured module decomposition**: 27 files with clear responsibilities, tree-structured dependencies (no circular imports), consistent naming conventions. The recent extract-refactors (element → view/element+events+style) demonstrate healthy codebase evolution.

6. **Observability substrate**: Multi-channel error reporting, signal introspection via `__omni_subs` Map, structured source maps and element trees, editor bridge nidMap — all provide rich data for debugging and future AI integration.

7. **ADR documentation culture**: 24 architectural decision records with blast radius, reversibility, and sequencing analysis. Creates institutional knowledge that survives team changes.

### Areas for Improvement

1. **Multi-instance component state**: Non-@for components share global signals. HIGH priority — correctness bug, though limited to multi-instance usage patterns.

2. **Cross-target parity enforcement**: No automated output equivalence testing. HIGH priority — 4 silent feature drops already found; new features will create more.

3. **Runtime memory management**: `__omni_effect` dependency leak + `__omni_runtimeErrors` unbounded growth. MEDIUM priority — S-effort fixes in single file.

4. **Generator struct complexity**: 46 fields × 3 implicit mode flags = state machine spanning 15+ files. MEDIUM priority — gradual sub-struct extraction recommended.

5. **Code duplication hotspots**: Mutation operators (6-fold), motion triggers (named/inline), effect cleanup (editor/non-editor). MEDIUM priority — S-to-M effort extractions.

### Technical Debt

**High Priority**:
- **`__omni_effect` subscriber leak**: Pattern just fixed in `__omni_computed` but not propagated to the far-more-commonly-emitted `__omni_effect`. **Effort**: S. **Timeline**: Immediate.
- **Module-level motion/timeline duplicates**: N+1 emission for N components. **Effort**: S. **Timeline**: Immediate.

**Medium Priority**:
- **Motion trigger dispatch duplication**: ~150 lines of near-identical code in two match blocks. **Effort**: M. **Timeline**: This sprint.
- **`generate_component` monolith**: 410 lines, 8 responsibilities. **Effort**: M. **Timeline**: Next sprint.
- **Arithmetic mutation 6-fold repetition**: 120 lines → 20 lines with helper extraction. **Effort**: S. **Timeline**: This sprint.

**Low Priority**:
- **Inline JS template maintainability**: ~150+ untyped `writeln!` calls across 7 files. **Effort**: L. **Timeline**: Next quarter.
- **7-pass IR scanning**: Working set eviction for large programs. **Effort**: M. **Timeline**: When WASM compile time becomes a bottleneck.

### Risks

**Technical Risks**:
- **Multi-instance component silent failure** (Likelihood: Medium, Impact: High): Users writing natural multi-instance patterns get wrong output with no error. Mitigation: Document limitation, plan per-instance isolation.
- **Cross-target feature regression** (Likelihood: High, Impact: Medium): Each new IR feature silently drops in 3-5 backends. Mitigation: Automated output equivalence tests.
- **Memory leak in long editor sessions** (Likelihood: Medium, Impact: Medium): Combined effect of `__omni_effect` leak and unbounded error array. Mitigation: Both are S-effort runtime.rs fixes.
- **Generator struct growth bottleneck** (Likelihood: Low, Impact: Medium): 46 fields at threshold; each new feature adds more. Mitigation: Gradual sub-struct extraction.

**Operational Risks**:
- **Hot-replace handler inconsistency**: `__omni_forHandlers` recreated on each compile without clearing old entries. **Mitigation**: Reset handler map on recompilation.
- **Deferred JS ordering fragility**: 6-step execution order implicit in code, undocumented. **Mitigation**: Module-level doc comment.

**Business Risks**:
- **Competitive positioning**: Cross-target compilation is a key differentiator. Silent feature drops across backends erode this advantage. **Mitigation**: CI-level parity enforcement.

---

## Recommendations

### Immediate (0-2 weeks)

| # | Action | Effort | Owner |
|---|--------|--------|-------|
| 1 | **Fix `__omni_effect` dependency leak** — Wrap `fn` in cleanup-before-exec wrapper matching `__omni_computed` pattern. `runtime.rs:84-89`, 4 lines. | S | Compiler team |
| 2 | **Cap `__omni_runtimeErrors` at 100 entries** — Add `if (length >= 100) shift()` before push. `runtime.rs:96`, 3 lines. | S | Compiler team |
| 3 | **Document multi-instance component limitation** — Add to language reference: non-@for multi-instance components share state. | S | Docs team |
| 4 | **Wrap event handlers in try/catch** — Add `try { handler(e); } catch(err) { __omni_reportError("event:click", err); }` around all addEventListener callbacks. `view/events.rs`, `component_html.rs`. | S | Compiler team |
| 5 | **Add cross-target output equivalence assertions** — Structural invariant checks for the 4 ADR-021 parity features in `cross_target.rs`. | S | Compiler team |

### Short-term (2-8 weeks)

| # | Action | Effort | Owner |
|---|--------|--------|-------|
| 6 | **Remove empty `view/list.rs`** — Delete file and `mod list;` declaration. | S | Compiler team |
| 7 | **Merge `list_render.rs` into `list.rs`** — Single file for the @for two-phase pipeline (541 lines total). | S | Compiler team |
| 8 | **Fix motion/timeline duplicate emission** — Remove from `generate_component()` or gate behind `editor_mode`. `store.rs:271-288`. | S | Compiler team |
| 9 | **Extract arithmetic helper in `stmt.rs`** — `emit_mutation_arithmetic` helper, eliminates 100 lines of 6-fold duplication. | S | Compiler team |
| 10 | **Unify effect cleanup target buffer** — Assign `comp_mount_js`/`mount_js` to local variable. `store.rs:388-436`. | S | Compiler team |
| 11 | **Cache `__omni_el()` results** — Add closure-level nid→element cache in runtime; clear on @for re-render. `runtime.rs:109-111`. | S | Compiler team |
| 12 | **Consolidate motion trigger dispatch** — Build `anim_call` before single match; eliminate ~150 lines of named/inline duplication. | M | Compiler team |
| 13 | **Add name collision diagnostic** — Warn on store/component prefix conflicts during semantic analysis. | S | Compiler team |

### Long-term (2-6 months)

| # | Action | Effort | Owner |
|---|--------|--------|-------|
| 14 | **Generator sub-struct extraction** — `GeneratorOutput` (5 fields), `ForLoopState` (5 fields), `CodegenMode` enum replacing 3 booleans. | M | Compiler team |
| 15 | **Shared ViewTree visitor trait** — Prevent future variants from silently missing backends. | M-L | Compiler team |
| 16 | **Cross-target full test matrix** — Compile all golden fixtures against all 9 backends with output-level assertions. | L | QA/CI team |
| 17 | **Runtime performance instrumentation** — Opt-in `performance.now()` wrapping in effects, gated behind flag. | M | Compiler team |
| 18 | **Non-@for per-instance state isolation** — Extend `@for` pattern to all component inlining. | XL | Compiler team |
| 19 | **JS template macro/codegen DSL** — Replace ~150 ad-hoc `writeln!` calls with typed, shared generation functions. | L | Compiler team |

---

## Success Metrics

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| Runtime memory leak (effect deps) | Unbounded growth | Bounded (cleared per execution) | Immediate |
| Event handler error visibility | 0% caught by monitoring | 100% caught by `__omni_reportError` | Immediate |
| Cross-target feature parity enforcement | Manual review only | CI-level structural assertions | 4 weeks |
| Code duplication (motion triggers) | ~150 lines duplicated | Single dispatch block | 4 weeks |
| Generator struct field count | 46 fields | ~35 fields (after sub-struct extraction) | 3 months |
| `generate_component` line count | 410 lines | ~300 lines | 3 months |

---

## Follow-up

**Next Review**: After implementation of Immediate actions (estimated 2 weeks)

**Tracking**: Create GitHub issues for each Immediate and Short-term action. Use the architecture recalibration process for progress assessment.

**Recalibration**:
After implementing recommendations, conduct architecture recalibration:
```
"Start architecture recalibration for OmniView codegen"
```

**Accountability**:
- Compiler team owns all compiler/runtime fixes
- Docs team owns documentation updates
- QA/CI team owns cross-target test infrastructure
- Weekly progress check, documented in `.architecture/reviews/component-omniview-2026-07-29-status.md`

---

## Related Documentation

**Architectural Decision Records**:
- [ADR-016: Design Theme System](../decisions/adrs/ADR-016-design-theme-system.md) — CSS cascade over JS signal cascade for theme switching
- [ADR-018: Event System Architecture](../decisions/adrs/ADR-018-omniview-event-system-architecture.md) — Event handler parameter threading, emit lowering, onSubmit preventDefault
- [ADR-021: Cross-Target Feature Parity](../decisions/adrs/ADR-021-cross-target-feature-parity.md) — 4 silent feature drops across 3 backends, deferred test matrix
- [ADR-022: Canonical Easing Table](../decisions/adrs/ADR-022-canonical-easing-table.md) — Single-source easing values
- [ADR-023: Shared View-Tree Traversal](../decisions/adrs/ADR-023-shared-view-tree-traversal.md) — 8 duplicate walkers → 1 shared function
- [ADR-024: React Motion useEffect Lifecycle](../decisions/adrs/ADR-024-react-motion-useeffect-lifecycle.md) — React motion lifecycle fix

**Previous Reviews**:
- [OmniView Architecture Assessment](../../memory/omniview-architecture-assessment.md) — 2026-07-26, A-grade rating, Language-First design validated
- [OmniView Event System Review](../../memory/omniview-event-system-review-2026-07-28.md) — B- maturity, 6 findings, ADR-018 created
- [OmniView Multi-Perspective Review](../../memory/omniview-multi-perspective-review-2026-07-28.md) — 4-expert parallel review, 29 findings

**Referenced Documents**:
- [OmniView Compiler Internals](../../memory/omniview-compiler-internals.md) — Full module-by-module compiler architecture
- [OmniView Codegen Refactoring](../../memory/omniview-codegen-refactoring.md) — element_to_html 839→110 lines, motion trigger dedup
- [WASM FFI Serialization Contract](../../memory/wasm-ffi-serialization-contract.md) — Cross-boundary serialization rules

---

## Appendix

### Review Methodology

This review was conducted using the AI Software Architect framework (v1.6.0) with 8 team members:

- **Systems Architect**: Overall system coherence, component interaction, architectural principles
- **Domain Expert**: Domain model integrity, ubiquitous language, semantic accuracy
- **Security Specialist**: Threat modeling, security patterns, data protection, CSP/capability review
- **Maintainability Expert**: Code organization, complexity analysis, duplication, technical debt
- **Performance Specialist**: Runtime memory/CPU, reactivity performance, compile-time allocation
- **Implementation Strategist**: Change sequencing, blast radius, reversibility, rollout readiness
- **AI Engineer**: Observability, instrumentation, evaluation framework, AI integration readiness
- **Pragmatic Enforcer**: YAGNI principles, complexity/necessity analysis, simplification advocacy

Each member reviewed independently (subagents with scoped tools), then findings were cross-cut for themes, conflicts resolved, and recommendations prioritized.

**Pragmatic Mode**: Balanced
- Complexity ratio target: <1.5
- All recommendations evaluated through YAGNI lens
- Security-critical and data-integrity items exempt from pragmatic challenge

### Glossary

- **OmniView**: The primary/JS codegen backend of OmniLang — generates HTML/CSS/JS with inline signal runtime
- **Deferred JS**: Buffer swap mechanism ensuring DOM insertion before event/reactive binding
- **Component inlining**: Compile-time expansion of child component HTML into parent, with prefixed signal names for isolation
- **@for**: List rendering construct with per-instance state isolation via DOM-scoped `__omniState`
- **design block**: Language-level design system declaration (tokens + classes + theme variants)
- **nid**: Node identifier — `data-nid` attribute on every generated HTML element, used for runtime element lookup
- **css_only mode**: Compilation mode skipping all JS emission; produces CSS + element metadata for editor canvas preview
- **IIFE**: Immediately Invoked Function Expression — editor mode wraps each component in `(function() { ... })()` for hot-replace

---

**Review Complete**
