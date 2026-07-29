# Architecture Review: OmniView Frontend Framework

**Date**: 2026-07-29
**Review Type**: Component
**Reviewers**: Compiler Architect, TypeScript & React Specialist, Security Specialist, Rust Expert, Maintainability Expert, Performance Specialist, Domain Expert, Pragmatic Enforcer, Systems Architect, Implementation Strategist
**Post-Review ADR**: [ADR-028: OmniView as Debug/Development Runtime — Intentional Design Trade-offs](../decisions/adrs/ADR-028-omniview-debug-runtime-design-tradeoffs.md)

## Executive Summary

OmniView is a frontend framework embedded in the OmniLang programming language that treats UI concepts (components, signals, stores, design systems, animation) as language-first keywords compiled to self-contained JS/HTML/CSS output. The architecture is **sound at its core**: a clean compiler pipeline (AST → IR → codegen), a well-designed signal runtime (~200 lines), and a principled multi-layer security model.

**Critical context**: As documented in ADR-012 and ADR-028, OmniView codegen is designed as a **debug/development runtime** — not a production deployment target. Production deployment goes through React or Flutter backends. This role explains 14 design decisions that initially appeared to be deficiencies: no signal batching, no minification, no tree-shaking, inline runtime, string-based codegen, and editor metadata on output are all **intentional trade-offs** optimizing for human readability, self-contained output, synchronous determinism, and editor integration. These are acknowledged architectural decisions, not bugs.

With this context, the review identifies **2 CRITICAL bugs** (BlockStmtIR silently dropped, `__omni_activeEffect` leak), **8 HIGH-severity concerns** (God Object, CSP gaps, teardown incomplete, @enter replay over-subscription, VariantIR name clash, and others), and systemic maintainability challenges centered on the 46-field `OmniViewGenerator`.

**Overall Assessment**: Adequate — functionally strong but requires architectural refactoring to sustain the current development pace.

**Key Findings**:
- **BlockStmtIR silently discarded**: Valid `.omni` source with bare blocks `{ ... }` compiles to broken OmniView output while working correctly on React/Flutter targets — a cross-target inconsistency bug.
- **`__omni_activeEffect` leak**: A subscriber exception in the signal runtime leaves global state corrupted, causing silent UI freezes with no error trace.
- **God Object at 46 fields**: `OmniViewGenerator` has grown from ~30 to ~46 fields across recent sprints without structural refactoring. At current velocity, it reaches 70+ fields within 3 months.
- **No signal batching**: Every `signal.set()` triggers immediate synchronous cascading updates. Complex interactions (form submissions, store synchronizations) will produce visible jank.
- **No browser-level JS runtime tests**: All runtime functions (signals, sanitizers, DOM cache, error reporting) are verified only via Rust string-content checks. A broken JS runtime bug would pass CI undetected.
- **Known test failure unaddressed**: `test_resolve_styles_with_design_class` has been failing across multiple review cycles. CI signal is degraded.

**Critical Actions**:
- Fix BlockStmtIR handling in `emit_statement` (5 lines, immediate)
- Add try/catch/finally to signal subscriber notification (3 lines, immediate)
- Add `form-action 'self'; object-src 'none'; frame-src 'none'` to CSP (1 line, immediate)

---

## System Overview

**What is OmniView**: A frontend framework embedded in the OmniLang programming language. It treats UI concepts as "Language-First Citizens" — `component`, `signal`, `store`, `design`, `@motion`/`@timeline` are keywords in the `.omni` DSL compiled to self-contained JS/HTML/CSS output.

**Architecture**: The compiler pipeline follows `AST → IR (lowering) → Codegen`. The IR is shared across all compilation targets (OmniView, React, Flutter). The OmniView codegen backend lives in `omnilang/src/codegen/omniview/` (~11,000 lines across 19 source files + 3,800 lines of tests).

**Key Technologies**:
- Compiler: Rust (omnilang crate)
- Runtime: ~200-line inline JS signal system (no external dependencies)
- Output: Self-contained HTML document with inline JS/CSS
- Editor integration: Poiesis (Tauri desktop app) via WASM bridge + postMessage
- Security: Three-layer defense (compile-time checks, runtime sanitizers, CSP)

**Codebase Scale**:
- 19 source files + 3 test files in `omniview/` codegen
- Largest files: `tests_regression/motion.rs` (1,259 lines), `view/element.rs` (790 lines), `tests.rs` (1,485 lines)
- ~800 unit tests, 13 insta snapshots
- 15-chapter grammar reference documentation
- 6 active ADRs (ADR-018, ADR-025, ADR-026, ADR-027 most recent)

---

## Individual Member Reviews

### Compiler & Language Architect — Dr. Lin Wei

**Perspective**: Evaluating the IR design, cross-target consistency, and pipeline-stage responsibility boundaries.

#### Key Observations
- Clean pipeline separation with well-typed IR and serde serialization
- String-based JS codegen vs structured CodeWriter used by Flutter/Dart
- Component inlining at codegen time using name-mangling with complex `name_renames` lifecycle
- Dual rendering modes (css_only vs full) reusing the same pipeline
- Cross-target IR sharing with no shared codegen trait beyond the top-level `Codegen` interface

#### Strengths
1. **Strong event data schema**: `EventDataKind` with typed schemas and field-level `used_fields` tracking, correctly shared across all codegen backends via `pub use omnilang_core::codegen::event`
2. **Clean css_only reuse**: Same codegen pipeline with JS disabled, enforced by `debug_assert!` guards
3. **Motion runtime gating**: `project_uses_motion()` scans IR before embedding 46KB animation runtime
4. **Source map threading**: Properly threaded from IR through codegen to output
5. **Reactive binding guard for two-way bindings**: Guard counter pattern prevents infinite loops in `bind:value`/`bind:checked`

#### Concerns
1. **[HIGH] BlockStmtIR silently dropped in OmniView emit_statement**
   - **Location**: `omnilang/src/codegen/omniview/stmt.rs:133-136`
   - **Issue**: `StatementIR::BlockStmtIR` is unhandled and falls through to wildcard arm emitting `/* unhandled stmt */`. Inner statements of bare blocks `{ ... }` are silently lost. React/Flutter codegens handle this correctly.
   - **Fix**: Add `StatementIR::BlockStmtIR` arm that recurses into `body.statements`. Also fix `stmt_to_js_inline` at line 141. Effort: XS (15 lines).

2. **[MEDIUM] Fragile state save/restore in component inlining**
   - **Location**: `omnilang/src/codegen/omniview/component_html.rs:22-37`
   - **Issue**: 9 fields manually saved/restored with no compiler enforcement. Adding a new field to OmniViewGenerator that needs per-component isolation requires remembering this location.
   - **Fix**: Extract into `InliningState` helper struct with `save()`/`restore()` methods. Comment acknowledges historical bugs.

3. **[MEDIUM] StoreAction event handler lacks preventDefault/stopPropagation and used_fields**
   - **Location**: `crates/omnilang-core/src/ir/types.rs:450-454`
   - **Issue**: `EventHandlerIR::StoreAction` cannot express `e.preventDefault()` or `e.stopPropagation()`. Users must work around with inline function handlers.

4. **[MEDIUM] Dual signal access pattern**
   - **Location**: `expr.rs:21-29` (reads via `.value`) vs `stmt.rs:193-198` (writes via `__omni_subs.get().set()`)
   - **Issue**: Two different paths into the reactive runtime, creating coupling risk if runtime implementation changes.

#### Recommendations
1. **Add BlockStmtIR handling** (Immediate, XS) — Fix the missing arm in `emit_statement` and `stmt_to_js_inline`
2. **Extract InliningState save/restore helper** (Short-term, S) — Structured pattern to prevent field drift
3. **Add used_fields to StoreAction** (Short-term, M) — Extend IR and update lowering
4. **Add unit tests for BlockStmtIR** (Short-term, S) — Ensure coverage in OmniView codegen test suite

---

### TypeScript & React Specialist — Emma Rodriguez

**Perspective**: Evaluating the generated JS runtime quality, component model against established reactive frameworks, and developer experience for frontend engineers.

#### Key Observations
- Language-First component model is genuinely novel — enforces semantics at the compiler level
- DOM strategy is "static HTML + reactive patching" via `data-nid` markers — avoids virtual DOM overhead
- ~200-line inline signal runtime is functionally complete but has edge-case brittleness
- Component scope isolation via name prefixing, not closures
- Runtime error handling is dual-path but incomplete — core reactive primitives lack try/catch wrappers

#### Strengths
1. **Excellent security sanitization (defense-in-depth)**: Compile-time AND runtime URL/CSS sanitization with dual-layer consistency
2. **Two-way binding guard pattern**: Correctly implements guard-flag pattern for `bind:value`/`bind:checked`, matching Svelte's approach
3. **Component factory pattern enables editor hot-replace**: Clean protocol with signal value snapshot/restore using `(0, eval)` indirect eval
4. **Motion/animation system is thorough**: Covers all major WAAPI integrations with 24+ easing functions
5. **Component-prefixed naming prevents signal collision**: Multi-instance isolation via name prefixing

#### Concerns
1. **[HIGH] `__omni_activeEffect` leak on subscriber exception**
   - **Location**: `omnilang/src/codegen/omniview/runtime.rs:39-45`
   - **Issue**: If `e.run()` throws, `__omni_activeEffect` is never restored, causing silent state corruption. Remaining subscribers are skipped.
   - **Fix**: Wrap in try/catch/finally to restore `__omni_activeEffect`. Report error via `__omni_reportError`.

2. **[MEDIUM] No batch updates cause redundant cascades**
   - **Location**: `runtime.rs:36-46`
   - **Issue**: Every `signal.value = v` triggers immediate synchronous notification. Multiple signal updates in sequence trigger multiple full cascades.
   - **Fix**: Implement microtask-based batching with `__omni_batch()` wrapper and deferred effect queue.

3. **[MEDIUM] No signal disposal on component unmount**
   - **Location**: `store.rs:454-482` (teardown only removes DOM)
   - **Issue**: `__omni_subs` Map accumulates entries never removed. Old signal objects retain subscriber sets.
   - **Fix**: Extend teardown to clean up `__omni_subs`, `__omni_animations`, `__omni_forHandlers`, `__omni_el._cache`.

4. **[MEDIUM] `__omni_computed` dual-update path is fragile**
   - **Location**: `runtime.rs:56-82`
   - **Issue**: Computed has two distinct code paths for updating value — through `signal.set()` (which notifies subscribers) and through local `run()` (which doesn't). Cross-contamination risk for computed-to-computed dependencies.
   - **Fix**: Merge into single evaluation path; remove `signal.set(fn())` from `effect.run`.

#### Recommendations
1. **Fix `__omni_activeEffect` leak** (Immediate, S) — Add try/catch/finally in signal setter
2. **Clear `__omni_el._cache` on teardown** (Immediate, S) — One line in editor.rs
3. **Implement signal batching** (Short-term, M) — Microtask-based deferred effect queue
4. **Add signal cleanup on teardown** (Short-term, M) — Extend unmount protocol
5. **Refactor computed evaluation** (Short-term, S) — Single evaluation path

---

### Security Specialist

**Perspective**: Evaluating the security architecture for a compiler that generates frontend code — where vulnerabilities manifest in all generated output and the trust boundary is the compiler input itself.

#### Key Observations
- Three-layer defense in place (compile-time, runtime sanitizer, CSP) — correct architectural approach
- `data:` URI scheme fully blocked at runtime, closing previously identified gap
- `String()` coercion in sanitizers eliminates `toString()`-based bypass
- Dynamic URL attributes in `@for` templates silently dropped — functional gap, not vulnerability
- CSP missing three important directives: `form-action`, `object-src`, `frame-src`

#### Strengths
1. **Defense-in-depth correctly layered**: Three complementary layers with no single point of failure
2. **`String()` coercion discipline**: Applied consistently in both sanitizer functions
3. **Compile-time `is_dangerous_url` mirrors runtime logic**: Consistent treatment of `javascript:`, `data:`, `vbscript:` across layers
4. **`postMessage` origin hygiene**: Origin validation in both editor and debug bridges
5. **ES5 compatibility in runtime JS**: All sanitization uses ES5 patterns, avoiding subtle scoping issues
6. **Property-based test coverage**: InnerHTML escaping verified exhaustively
7. **All text content uses `textContent`**: Never `innerHTML` for reactive bindings

#### Concerns
1. **[MEDIUM] CSP missing `form-action 'self'` and `object-src 'none'`**
   - **Location**: `omnilang/src/codegen/omniview/utils.rs:248`
   - **Issue**: Without `form-action`, HTML injection could exfiltrate data via `<form action="https://evil.com">`. Without `object-src 'none'`, `<object>`/`<embed>` allowed from same origin.
   - **Fix**: Add `form-action 'self'; object-src 'none'; frame-src 'none'` to CSP string. Effort: S.

2. **[MEDIUM] `wrap_debug_html` has no CSP**
   - **Location**: `omnilang/src/codegen/omniview/utils.rs:191-224`
   - **Issue**: Debug HTML wrapper emits inline JS with zero CSP protection. If used in production-adjacent contexts, there's no XSS protection.
   - **Fix**: Add CSP to debug wrapper or add prominent docstring warning. Effort: S.

3. **[MEDIUM] CSS URL escape sequence bypass in `__omni_sanitizeCss`**
   - **Location**: `runtime.rs:148-172`, documented in ADR-027
   - **Issue**: CSS escape sequences like `\6aavascript:alert(1)` bypass the substring check. Browser normalizes `\6a` to `j` at parse time.
   - **Fix**: Add CSS escape sequence normalization before checking for dangerous keywords. Effort: M.

4. **[LOW] `__omni_el` selector injection via unescaped nid**
   - **Location**: `runtime.rs:178`
   - **Issue**: nid interpolated into CSS attribute selector without escaping quotes.
   - **Fix**: Escape `"` in nid before querySelector. Defense-in-depth. Effort: S.

5. **[LOW] Editor bridge uses `(0, eval)` for component replacement**
   - **Location**: `editor.rs:102`
   - **Issue**: Any XSS in parent editor can execute arbitrary code in preview iframe via postMessage.
   - **Fix**: Add JSON.parse validation on factoryCode parameter. Effort: M.

#### Recommendations
1. **Add missing CSP directives** (Immediate, S) — `form-action`, `object-src`, `frame-src`
2. **Add CSP to debug wrapper** (Immediate, S) — Security hardening
3. **Add CSS escape normalization** (Short-term, M) — Close sanitizer bypass
4. **Escape nid in querySelector** (Short-term, S) — Defense-in-depth
5. **Reconcile Rust/JS CSS sanitizer inconsistency** (Long-term, S) — Documented in ADR-027

---

### Rust Expert — Katya Volkov

**Perspective**: Evaluating Rust code quality, ownership patterns, allocation discipline, and performance-sensitive code patterns in the compiler codegen.

#### Key Observations
- Zero `unsafe` blocks in 6,500 lines of production code
- Zero `unwrap`/`expect` in production code (only in test helpers)
- All allocation on the write path (accumulating into String buffers) — natural for codegen
- Extensive use of `std::mem::take` and `std::mem::swap` for context save/restore
- Mixed string-building idioms: `push_str(&format!(...))` (103 sites) vs `write!(buf, ...)` (74 sites)

#### Strengths
1. **Sound ownership and borrowing**: Single `&mut self` generator pattern avoids interior mutability. Component inlining's save/restore pattern correctly mirrors save order in restore.
2. **Hash selection is appropriate**: `FxHashMap`/`FxHashSet` from `rustc_hash` — correct for non-DoS-sensitive internal maps
3. **Effective `#[track_caller]` for debug assertions**: `assert_can_emit_js()` and `assert_not_css_only()` use `debug_assert!` — zero cost in release
4. **Well-structured module decomposition**: 19 files organized by concern with clear re-export boundaries
5. **Comprehensive lossless error model**: Warnings collected as `Vec<String>` surfaced through `OmniViewOutput.warnings`

#### Concerns
1. **[MEDIUM] `push_str(&format!(...))` pervasive on hot codegen paths**
   - **Location**: 103 occurrences across 11 files
   - **Issue**: `format!()` allocates intermediate String, then `push_str()` copies bytes. `write!(target, ...)` writes directly. On hot inner loops, measurable allocation pressure.
   - **Fix**: Replace with `write!(self.js, ...).unwrap()`. Enable `clippy::format_push_string` lint. Effort: M.

2. **[MEDIUM] `StaggerDirection::Random` mapped to `"edges"` (probable logic error)**
   - **Location**: `motion_config.rs:251`
   - **Issue**: Silently replaces `staggerFrom: random` with `"edges"` behavior. No warning emitted.
   - **Fix**: Change to `"random"` or emit warning. Effort: XS.

3. **[MEDIUM] OmniViewGenerator struct cohesion (50+ fields)**
   - **Location**: `mod.rs:441-543`
   - **Issue**: Implicit coupling between unrelated concerns, difficult testing isolation, cognitive load for new contributors.
   - **Fix**: Split into focused sub-structs: `SignalTracker`, `ForLoopState`, `ElementTreeBuilder`. Effort: L.

4. **[MEDIUM] View tree clone in component inlining defeats borrowing**
   - **Location**: `component_html.rs:126`
   - **Issue**: Clones entire child view tree to break borrow on `self.component_map`. Multiplicative cost for nested components.
   - **Fix**: Pre-extract `&ViewTree` reference before taking `&mut self`. Effort: S.

#### Recommendations
1. **Fix `StaggerDirection::Random`** (Immediate, XS) — One line
2. **Convert `push_str(&format!(...))` to `write!()`** (Short-term, M) — 103 sites
3. **Introduce `js!()`/`html!()`/`css!()` write macros** (Short-term, S) — Eliminate `let _ = writeln!` ceremony
4. **Avoid view-tree clone** (Short-term, S) — Pre-extract reference pattern
5. **Decompose OmniViewGenerator** (Long-term, L) — Split into focused sub-structs

---

### Maintainability Expert

**Perspective**: Evaluating code organization, technical debt, and the long-term maintainability of the codegen module.

#### Key Observations
- 46-field God Object is the central architectural concern
- String-template JS generation provides zero syntax checking for generated code
- Heavy duplication in `motion_triggers.rs` (604 lines, 10 trigger kinds × 2 branches)
- Duplicated expression handling in `expr.rs` (FieldAccessExpr vs OptionalChainExpr)
- Fragile state management in component inlining (7 fields saved/restored manually)

#### Strengths
1. **Strong module decomposition by concern**: Clean split into `view/`, `motion/`, `store`, `design`, `list`, `expr`, `stmt`, `runtime`, `editor`, `debug`, `utils`
2. **Explicit JS execution order guarantee**: 6-phase execution order documented with debug assertion guards
3. **Comprehensive test suite**: 94 `#[test]` annotations, 13 insta snapshots, 3,849 lines of test code (35% of total)
4. **Good use of `std::mem::swap`/`std::mem::take`**: Architecturally sound buffer management patterns
5. **Inline signal runtime compact and readable**: ~200 lines, single-file dependency

#### Concerns
1. **[HIGH] OmniViewGenerator is a 46-field God Object**
   - **Location**: `mod.rs:441-543`
   - **Issue**: 65+ methods across 15+ files operate on the same struct. No encapsulation between phases. Adding features requires adding more fields.
   - **Fix**: Extract phase-specific state into contextual structs (`CodegenOutput`, `ForLoopState`, `ViewEmitContext`). Pass as `&mut` parameters. Effort: L.

2. **[HIGH] Massive duplication in motion_triggers.rs**
   - **Location**: `motion_triggers.rs:321-601`
   - **Issue**: ~280 lines of near-identical code between named-motion and inline-motion branches for each trigger kind.
   - **Fix**: Extract trigger-kind helpers with factory-call parameter. Effort: M.

3. **[MEDIUM] Duplicated FieldAccessExpr and OptionalChainExpr handling**
   - **Location**: `expr.rs:52-101`
   - **Issue**: 48 lines of duplicated logic differing only in `.value` vs `?.value` and `.` vs `?.` join operator.
   - **Fix**: Extract `resolve_store_field_access` helper. Effort: S.

4. **[MEDIUM] Triplicated class_map_bindings effect emission**
   - **Location**: `view/element.rs:575-587, 626-637, 771-777`
   - **Issue**: Same `__omni_effect` calls for class toggling duplicated in void-tag, normal-tag, and @for-template branches.
   - **Fix**: Extract `emit_class_map_effects` helper. Effort: S.

5. **[MEDIUM] Magic strings for HTML attributes**
   - **Location**: `view/element.rs:122-132`
   - **Issue**: 60 HTML attribute names as inline string literals. Boolean list, URL list, and standard list duplicated in meaning.
   - **Fix**: Define named const slices. Effort: S.

6. **[LOW] Dead field `current_nid_path`**
   - **Location**: `mod.rs:447`
   - **Issue**: Field initialized but never written to — appears vestigial from earlier design.
   - **Fix**: Remove field and its initialization. Effort: XS.

#### Recommendations
1. **Remove dead field `current_nid_path`** (Immediate, XS) — 3 lines
2. **Extract `emit_class_map_effects` helper** (Immediate, S) — 15 minutes
3. **Extract `resolve_store_field_access` helper** (Immediate, S) — 15 minutes
4. **Extract trigger-kind helpers** (Short-term, M) — Eliminate ~280 lines of duplication
5. **Define named const slices for HTML attributes** (Short-term, S) — 30 minutes
6. **Begin God Object decomposition** (Long-term, L) — Phased: ForLoopState → CodegenOutput → ViewEmitContext

---

### Performance Specialist

**Perspective**: Evaluating compile-time and runtime performance, allocation patterns, and output efficiency.

#### Key Observations
- Codegen is I/O-bound by String allocation — O(N) with output size
- Runtime signal system deliberately simple — no batching, no topological ordering
- Component inlining is the dominant compile-time cost — clones entire child IR view tree
- @enter replay mechanism couples ALL components' reactivity to ALL signals
- Generated JS has no post-processing — no minification, code splitting, or tree-shaking

#### Strengths
1. **O(1) signal name resolution via FxHashSet/FxHashMap**: Fast hashed membership check using `rustc_hash`
2. **Deferred JS pattern**: DOM created in one shot via innerHTML, then bound reactively — correct approach
3. **Conditional 46KB motion runtime inclusion**: `project_uses_motion()` gates expensive runtime
4. **@for pre-compiled bindings**: Avoid `new Function()` — security and performance win
5. **`__omni_el` caching**: Prevents repeated querySelector with `isConnected` invalidation

#### Concerns
1. **[HIGH] @enter replay subscribes to every signal in the program**
   - **Location**: `store.rs:417-452`
   - **Issue**: When root component has @enter animations, a single unrelated signal change causes N signal reads + M animation create/cancel cycles. At 50 signals + 10 animations, this is O(50+10) per frame.
   - **Fix**: Restrict replay dependencies to only signals the animation's component actually references. Effort: M.

2. **[HIGH] Signal `set()` has no batching**
   - **Location**: `runtime.rs:36-46`
   - **Issue**: Cascading DOM writes within single synchronous chain. With computed chains of depth 4+, effects run multiple times per logical update.
   - **Fix**: Synchronous worklist with dedup (Set-based), ensuring each effect runs at most once per cycle. Effort: S-M.

3. **[HIGH] Unbatched DOM insertion in @for loops**
   - **Location**: `list.rs:333` — `parent.insertBefore(clone, ctr)` per iteration
   - **Issue**: N sequential layout recalculations for N items. For 5,000 items, that's 5,000 layout passes.
   - **Fix**: Use DocumentFragment to batch all insertions into one layout pass. Effort: S.

4. **[MEDIUM] `try_inline_component` clones entire child view trees**
   - **Location**: `component_html.rs:126`
   - **Issue**: Multiplicative cloning for nested components: O(e_comp × depth).
   - **Fix**: Pre-compute and cache inlineable HTML/JS per component. Effort: M.

5. **[MEDIUM] No output minification**
   - **Location**: All emit_* functions
   - **Issue**: Generated JS/CSS 3-5× larger than necessary. 200KB output where 60KB would suffice.
   - **Fix**: Optional minification pass via WASM-based minifier. Effort: M.

6. **[MEDIUM] `escape_html` allocates 5 intermediate strings per call**
   - **Location**: `utils.rs:177-183`
   - **Issue**: Five chained `.replace()` calls, each allocating new String. 25,000 calls × 5 allocations = 125,000 temporary allocations per compilation.
   - **Fix**: Single-pass scanner/writer. Effort: S.

#### Recommendations
1. **DocumentFragment for @for DOM insertion** (Immediate, S) — Eliminate layout thrashing
2. **Single-pass `escape_html`** (Immediate, S) — 125K fewer allocations per build
3. **Restrict @enter replay dependencies** (Short-term, M) — IR analysis to trace signal dependencies
4. **Signal batching worklist** (Short-term, S-M) — Dedup effects, prevent double-execution
5. **Animation entry cleanup** (Short-term, S) — Delete stale entries on unmount
6. **Output minification** (Long-term, M) — WASM-based post-processing

---

### Domain Expert

**Perspective**: Evaluating whether the "Language-First Citizens" concept models the frontend UI domain coherently, and whether domain boundaries are respected.

#### Key Observations
- Five well-bounded citizens, two with naming friction (`computed` vs `getter`, `VariantIR` name clash)
- Style is the least-mature citizen — weaker IR model than design system classes
- Theme concept is fragmented — two separate mechanisms, one unimplemented
- Props are signals in disguise — documented as immutable but emitted as mutable
- Animation domain embedded in ElementIR — couples orthogonal concerns at type level

#### Strengths
1. **Clean bounded context separation**: Each citizen has well-defined output contract (design→CSS only, store→JS only, component→JS+HTML+CSS, motion→JS only)
2. **Intentional keyword distinction**: `computed` in components vs `getter` in stores — domain-appropriate naming
3. **ViewTree enum models UI template domain faithfully**: Six variants correspond exactly to what developers write
4. **Design token type system**: 13 `TokenTypeIR` variants with domain-specific validation
5. **Event domain well-encapsulated**: 21 `EventDataKind` variants with typed field schemas

#### Concerns
1. **[HIGH] `VariantIR` name clash between two different domain concepts**
   - **Location**: `crates/omnilang-core/src/ir/logic.rs:50` vs `crates/omnilang-core/src/ir/design.rs:91`
   - **Issue**: `logic::VariantIR` (enum variant in sum types) vs `design::VariantIR` (design system variant with dimensions). Same name, completely different semantics. `ir/mod.rs:36-37` has explicit comment acknowledging the conflict.
   - **Fix**: Rename `design::VariantIR` to `DesignVariantIR`. Effort: M (6-8 files, rename only).

2. **[HIGH] Component `style {}` block has weaker domain model than design system classes**
   - **Location**: `ir/types.rs:684-690` (no pseudo-states) vs `ir/design.rs:68-86` (has `pseudo_states: Vec<PseudoStateIR>`)
   - **Issue**: `.button:hover` works accidentally as string concatenation. Design system classes model pseudo-states properly. Tooling cannot validate component pseudo-classes. Arbitrary capability gap.
   - **Fix**: Add `pseudo_states: Vec<PseudoStateIR>` to `StyleRuleIR`. Effort: L.

3. **[MEDIUM] `theme` keyword vs `@themes` directive — two theme mechanisms, one unimplemented**
   - **Location**: `docs/omniview/10-design-system.md:205-251`
   - **Issue**: Standalone `theme {}` block parses but generates zero output. `@themes` works. Developers encounter dead-end API surface.
   - **Fix**: Implement codegen for `theme {}` or deprecate keyword. Effort: S-M.

4. **[MEDIUM] Props emitted as signals with no immutability enforcement**
   - **Location**: `store.rs:223-234` vs `docs/omniview/05-components.md:28-44`
   - **Issue**: Doc says "cannot be modified inside component" but emitted JS offers no enforcement.
   - **Fix**: Add compile-time check rejecting direct assignment to prop names. Effort: S.

#### Recommendations
1. **Rename `design::VariantIR` to `DesignVariantIR`** (Immediate, M) — Eliminate HIGH name clash
2. **Add prop immutability compile check** (Immediate, S) — Close doc-implementation gap
3. **Resolve theme fragmentation** (Short-term, S-M) — Implement or deprecate `theme {}`
4. **Add pseudo-states to component StyleRuleIR** (Long-term, L) — Domain completeness
5. **Expand StyleRuleIR for at-rules** (Long-term, L) — @media, @keyframes support

---

### Pragmatic Enforcer — YAGNI Guardian & Simplicity Advocate

**Perspective**: Evaluating whether the framework's complexity is justified by current requirements, identifying over-engineering, and pushing for the simplest viable approach.

#### Key Observations
- OmniView is a "build-your-own-everything" framework — language, compiler, runtime, editor integration, multi-target codegen, documentation system
- The framework competes against mature ecosystems (React, Vue, Svelte, Solid) with near-zero switching cost
- Learn-omniview has 66+ source files demonstrating all features — but this is a self-contained demo with no external production deployments visible
- ~11,000 lines of codegen code for the OmniView backend alone (not counting the rest of the compiler pipeline)
- The multi-backend architecture (OmniView + React + Flutter + 6 paused backends) amplifies maintenance cost for every feature

#### Necessity/Complexity/Ratio Analysis

| Feature | Necessity (1-10) | Complexity (1-10) | Ratio | Verdict |
|---------|------------------|-------------------|-------|---------|
| Signal runtime (~200 lines) | 10 | 2 | 0.2 | **Essential** — core reactive primitive |
| Component system | 10 | 4 | 0.4 | **Essential** — UI composition |
| Store system | 8 | 5 | 0.63 | **Needed** — state management, but simpler alternatives exist |
| Design system (tokens, classes, variants, themes) | 6 | 7 | 1.17 | **Questionable** — powerful but over-engineered for current scale |
| Motion/animation system (604-line triggers file) | 5 | 8 | 1.6 | **Complex** — exceeds threshold; many triggers unused |
| CSS-only mode for editor | 5 | 6 | 1.2 | **Editor-specific** — only needed for Poiesis |
| Source maps | 7 | 3 | 0.43 | **Valuable** — debugging requirement |
| Multi-target (React, Flutter, 6 paused) | 4 | 9 | 2.25 | **Over-threshold** — triples maintenance burden |
| 15-chapter documentation | 8 | 4 | 0.5 | **Needed** — but grammar specs could be deferred |

#### Concerns
1. **[HIGH] Multi-target architecture amplifies maintenance cost**
   - **Issue**: 3 active + 6 paused backends. Every IR change, parser change, or semantic model change must be reflected across all backends. The cross-target parity tests only check string presence, not semantic equivalence.
   - **Recommendation**: Freeze paused backends formally (no maintenance, accept they'll rot). Focus on OmniView as the primary target. React/Flutter as "best-effort community targets."

2. **[HIGH] Motion/animation system ratio exceeds threshold (1.6)**
   - **Issue**: 604 lines of trigger code, 292 lines of config, 329 lines of timeline, 146 lines of motion wrapper = ~1,400 lines for animation. Most apps don't use WAAPI animations at this level.
   - **Recommendation**: Reduce to 3 core triggers (enter, hover, scroll). Defer the rest (drag, press, focus, in-view, timeline) until there's proven demand.

3. **[MEDIUM] Design system variant system may be over-engineered**
   - **Issue**: 13 TokenTypeIR variants, multi-dimensional variant resolution, pseudo-state modeling — this is more sophisticated than most CSS-in-JS libraries.
   - **Recommendation**: Audit which variant features learn-omniview actually uses. Defer unused token types.

4. **[MEDIUM] 15-chapter docs with 2 grammar specs**
   - **Issue**: Grammar specs (14-grammar-lang.md, 15-grammar-view.md) are for tool developers. If OmniView has no external tooling ecosystem, these are premature.
   - **Recommendation**: Keep language reference docs (01-13). Downgrade grammar specs to "draft" status.

#### Strengths
1. **Inline runtime is appropriately simple**: ~200 lines with no dependencies — exactly right
2. **Security hardening is needs-driven**: Recent sanitization work addresses real threats
3. **Feature-gating via Cargo features**: Paused backends don't affect build times
4. **Reactive guard pattern for two-way binding**: Correctly prevents infinite loops without over-engineering
5. **CSS-only mode reuses pipeline**: Avoids duplicate code for editor preview

#### Recommendations
1. **Freeze paused backends** (Immediate, XS) — Policy decision, no code change
2. **Defer motion complexity** (Short-term, M) — Reduce to 3 core triggers
3. **Audit variant usage** (Short-term, S) — Data-driven deferral of unused features
4. **Downgrade grammar specs** (Short-term, XS) — Mark as draft

---

### Systems Architect

**Perspective**: Evaluating how OmniView fits within the broader OmniLang ecosystem, multi-backend architecture soundness, and integration patterns.

#### Key Observations
- OmniView is the "special" backend — bypasses standard `codegen::compile()` dispatch via feature-gated if/else
- Cross-backend tests check structure not semantics — verifying output contains string markers, not equivalent logic
- WASM architecture sound but cache has gaps — main `compile_multi_to_omniview` doesn't use incremental cache
- `UiCodegen` trait promises shared implementation but doesn't deliver — comment says "code not moved into trait methods"
- Editor integration has two independent compilation pipelines (WASM + Tauri IPC)

#### Strengths
1. **Clear layered module architecture**: `omnilang-core` → `omnilang` → `omnilang-wasm` with explicit interface boundaries
2. **CSS-only mode for editor preview**: Reuses full codegen pipeline guaranteeing style consistency
3. **Incremental cache architecture**: Content-hash-based invalidation with dependency graph propagation
4. **Feature-gated backend support**: Paused backends don't affect CI or binary size
5. **Dual editor bridge architecture**: postMessage (browser) + Tauri IPC (desktop) as orthogonal dimensions
6. **Self-contained output format**: No npm, no bundler, no external runtime — ideal for iframe embedding

#### Concerns
1. **[HIGH] OmniView bypasses standard codegen dispatch**
   - **Location**: `omnilang/src/lib.rs:401-418`
   - **Issue**: Feature-gated if/else that skips `codegen::compile()`. Changes to the standard path must be manually replicated. `validate_stdlib_imports` and source map extension don't happen on the bypass path.
   - **Fix**: Extract shared logic or make OmniView output compatible with `CompileOutput`. Effort: L.

2. **[HIGH] Cross-backend semantic equivalence tests inadequate**
   - **Location**: `omnilang/tests/cross_target.rs:225-302`
   - **Issue**: String presence checks (`has(&output, "slot")`) don't verify semantic equivalence. Known differences (ADR-021: Flutter uses `BoxDecoration()` placeholder) remain undetected.
   - **Fix**: Add feature-specific semantic checks verifying equivalent logic across backends. Effort: M.

3. **[MEDIUM] UiCodegen trait is delegation not shared implementation**
   - **Location**: `crates/omnilang-core/src/codegen/shared/ui_codegen.rs:17-18`
   - **Issue**: Comment explicitly states "code not moved into trait methods." Each backend independently implements `generate_component_code`.
   - **Fix**: Move shared logic into trait default implementations. Effort: L.

4. **[MEDIUM] WASM compile_multi_to_omniview not using incremental cache**
   - **Location**: `omnilang-wasm/src/lib.rs:432-473`
   - **Issue**: Full re-parse/re-analyze on every call despite existing `ANALYSIS_CACHE` infrastructure.
   - **Fix**: Add cache parameter and logic matching `analyse_multi_cached`. Effort: S.

5. **[MEDIUM] Rust vs JS CSS sanitizer inconsistency**
   - **Location**: `crates/omnilang-core/src/codegen/sanitize.rs:18` vs `runtime.rs`
   - **Issue**: Rust blocks all `url(`; JS allows with protocol check. Same value, different results.
   - **Fix**: Align Rust sanitizer with JS runtime logic. Effort: S.

#### Recommendations
1. **Connect WASM incremental cache** (Immediate, S) — Use existing `ANALYSIS_CACHE`
2. **Align Rust/JS CSS sanitizers** (Immediate, S) — Single behavior for `url()`
3. **Add semantic cross-backend tests** (Short-term, M) — Beyond string presence
4. **Extract shared runtime to optional separate file** (Short-term, M) — For multi-page apps
5. **Remove bypass path in compile_multi_internal_post** (Long-term, L) — Route through standard dispatch
6. **Migrate UiCodegen to shared defaults** (Long-term, L) — Reduce new-backend cost

---

### Implementation Strategist

**Perspective**: Evaluating change cadence, blast radius, reversibility, and production readiness.

#### Key Observations
- Project is in active post-review remediation — 3 ADRs in 3 days (ADR-025, 026, 027)
- 30 recent commits show healthy cadence: security hardening, refactoring, bug fixes
- 40+ memory files indicate complex accumulated knowledge
- Tests are the safety net — 800+ tests running clean
- Learn-omniview demo project is comprehensive but not a production deployment

#### Strengths
1. **Rapid iteration with quality gates**: 30 commits without breaking tests demonstrates good CI/CD discipline
2. **ADR-driven decision making**: Architectural decisions documented before implementation
3. **Refactoring courage**: Recent extract-method refactoring (emit_element_css_only_capture, emit_event_listener, finalize_js inline) shows willingness to improve
4. **Security-first culture**: Sanitization work prioritized and completed systematically
5. **Test coverage as safety net**: 800+ tests enable confident refactoring

#### Concerns
1. **[HIGH] No production deployment evidence**
   - **Issue**: Learn-omniview is the only known deployed application. No real users, no production traffic, no performance data from real workloads.
   - **Impact**: All performance concerns (batching, @enter replay, @for DOM insertion) are theoretical until validated with real data.
   - **Fix**: Ship one real application (even internal) before further framework investment. Use data to prioritize.

2. **[MEDIUM] Rapid ADR creation may outpace implementation**
   - **Issue**: ADR-025 (remediation plan), ADR-026 (v2 fixes), ADR-027 (security sanitization) created in rapid succession. Not all decisions may have been fully implemented.
   - **Fix**: Audit ADR implementation status. Create tracking issue per ADR.

3. **[MEDIUM] Element tree and nid_map fields add coupling to Poiesis**
   - **Issue**: `OmniViewOutput` carries 11 editor-specific fields. Changes to Poiesis editor require changes to OmniView codegen output format.
   - **Fix**: Extract editor metadata into optional `EditorMetadata` struct separate from standard output.

4. **[LOW] No CI for WASM compilation path**
   - **Issue**: WASM-specific compilation path (`compile_multi_to_omniview`) may diverge from native path without CI detection.
   - **Fix**: Add WASM-target CI job compiling learn-omniview through WASM path.

#### Recommendations
1. **Ship a real application** (Immediate, M) — Internal tool or demo with real users
2. **Audit ADR implementation status** (Immediate, S) — Track what's done vs deferred
3. **Extract editor metadata from OmniViewOutput** (Short-term, M) — Reduce coupling
4. **Add WASM CI job** (Short-term, S) — Prevent compilation path divergence
5. **Establish performance benchmarks** (Short-term, M) — Lighthouse scores for learn-omniview

---

### Implementation Strategist

**Perspective**: Evaluating change cadence, blast radius, reversibility, test adequacy, and production readiness.

#### Key Observations
- Healthy review rhythm — 3 ADRs in 3 days, 30+ bugs found and fixed before reaching production
- Deferred items accumulating: multi-instance component state (XL), motion trigger merge (M), CSS sanitizer inconsistency (M), UiCodegen trait integration (M)
- 46-field `OmniViewGenerator` is the highest blast-radius change point — state save/restore bugs are the root cause of ADR-025 D1 and ADR-026 D3
- Known test failure (`test_resolve_styles_with_design_class`) unaddressed across 3 review cycles — degrades CI signal
- Zero browser-level JS runtime tests — signals, sanitizers, event handling, DOM cache verified only via Rust string checks

#### Strengths
1. **Granular test regression guard**: 94 snapshot files + 241 unit tests prevent unintended JS output changes
2. **Reversible fix patterns**: All ADR-025/026 fixes are S-M effort, single-file, `git revert` reversible, no language syntax changes
3. **Defensive execution order documentation**: 6-phase JS pipeline documented with `debug_assert!` guards
4. **Systematic cross-target test infrastructure**: 20+ golden files compile against all 3 P0 backends
5. **Maintainability improvements**: Recent commits reduced technical debt — 2 files deleted, ~60 lines net reduction, event listener emission unified

#### Concerns
1. **[HIGH] JS runtime behavior not browser-tested**
   - **Location**: `runtime.rs:15-203`, `tests_regression/sanitizers.rs`, `tests_regression/motion.rs`
   - **Issue**: All runtime functions verified only via Rust string-content checks. A broken `===` or misspelled variable in the generated JS would pass all 800+ tests. The security-critical sanitizers are untested in actual JS engines.
   - **Fix**: Add Node.js/Deno script that loads generated JS and asserts runtime function correctness. Use `boa_engine` crate or shell to `node -e`. CI check for any runtime.rs changes.
   - **Effort**: M (half day for Node.js script + CI integration)

2. **[HIGH] Known test failure unaddressed across 3 review cycles**
   - **Location**: `omnilang/tests/resolve_styles_test.rs:52-83` — `test_resolve_styles_with_design_class`
   - **Issue**: Test asserts no warnings when using `class: Card` (defined in design block). Warning is incorrectly produced. Recognized in ADR-026 as pre-existing but never fixed. Degrades CI signal — "red is normal" learned helplessness.
   - **Fix**: Fix `design_class_names` lookup in `emit_element_props` to correctly index design-block classes in the HashSet built at `mod.rs:504-506`.
   - **Effort**: S (1-2 hours)

3. **[MEDIUM] Cross-target test coverage inadequate for 6 backends**
   - **Location**: `omnilang/tests/cross_target.rs:226-302`
   - **Issue**: Only OmniView, React, Flutter tested. Vue, Dioxus, Dart, Python, JavaScript, Rust have zero semantic verification. ADR-021 confirms silent feature dropping in untested backends.
   - **Fix**: Extend P0_BACKENDS to include Vue and Dioxus. Add structural output checks for Python/JS/Rust/Dart.
   - **Effort**: M (half day)

4. **[MEDIUM] CI warning noise masks real issues**
   - **Issue**: ~30 `ts-rs` warnings + 4 library test warnings create high noise floor. Developers scan past warnings.
   - **Fix**: Fix `let mut gen` warning in `motion.rs:118`. Address `ts-rs` annotation gaps.
   - **Effort**: S (30 minutes)

#### Recommendations
1. **Fix known test failure** (Immediate, S) — Restore CI signal integrity
2. **Add browser-level JS runtime tests** (Short-term, M) — Close largest test blind spot
3. **Extend cross-target tests to all backends** (Short-term, M) — Prevent silent feature loss
4. **Fix CI warnings** (Short-term, S) — Reduce noise floor
5. **Decompose OmniViewGenerator** (Long-term, L) — Root cause of highest blast-radius changes

---

## Acknowledged Design Trade-offs (ADR-028)

Post-review analysis revealed that **14 findings initially classified as concerns are intentional design trade-offs** documented in [ADR-028](../decisions/adrs/ADR-028-omniview-debug-runtime-design-tradeoffs.md). OmniView codegen is a **debug/development runtime** — not a production deployment target. The following decisions optimize for human readability, self-contained output, synchronous determinism, and editor integration:

| # | Trade-off | Why Intentional | ADR-028 Ref |
|---|-----------|----------------|-------------|
| 1 | No signal batching | Synchronous = predictable = debuggable | D3 |
| 2 | No output minification | Readable output for debugging; comment markers identify code origins | D4 |
| 3 | Inline runtime (~200 lines) | Self-contained iframe-ready document; fits in one screen of code | D1 |
| 4 | No tree-shaking / code splitting | Single-file output design; all code inspectable | D5 |
| 5 | `wrap_debug_html` without CSP | Debug-only wrapper; production path includes CSP | D6 |
| 6 | String-based codegen (not CodeWriter) | ADR-012: imperative pattern matches "one output, one document" | D2 |
| 7 | CSS-only mode for editor canvas | Reuses codegen pipeline for style consistency editor↔export | D7 |
| 8 | `(0, eval)` hot-replace | Standard indirect eval pattern; only in editor_mode | D8 |
| 9 | `debug_mode` / `editor_mode` flags | Orthogonal dimensions for different editor contexts | D9 |
| 10 | Editor metadata on OmniViewOutput | WASM/FFI boundary needs structured data; avoids HTML re-parse | D10 |
| 11 | 6-phase JS execution order | Buffer-swap pattern enforces DOM-before-bindings invariant | D11 |
| 12 | `debug_assert!` phase guards | Zero-cost in release; catches phase violations in dev | D12 |
| 13 | Source maps | Debugging requirement; editor selection tracking | D13 |
| 14 | Overall debug-tool role | ADR-012: "for development debugging, not production deployment" | D14 |

These trade-offs should NOT be re-flagged as bugs or deficiencies in future reviews. Some have documented remediation paths if OmniView ever becomes a production target (signal batching D3, minification D4, CSS sanitizer alignment).

---

## Cross-Cutting Themes (Remaining Concerns After ADR-028 Filter)

The following themes emerged independently from **3 or more** specialists and are NOT covered by ADR-028's intentional trade-offs — they represent actual issues requiring action:

### Theme 1: God Object Crisis (4 specialists: Rust, Maintainability, Compiler Architect, Systems Architect)

`OmniViewGenerator` has grown from ~30 to ~46 fields across recent sprints. Every method across 15+ files operates on the same struct with no phase-state encapsulation. All four specialists independently identified this as the primary architectural risk. The Maintainability Expert noted the struct will reach 70+ fields within 3 months at current velocity. The Rust Expert noted the 51-field constructor in `test_new()` is a maintenance smell. The Compiler Architect identified the fragile save/restore pattern as a direct consequence. The Systems Architect noted the struct's coupling to editor-specific fields.

**Consensus**: Decompose into focused sub-structs. Phase 1: `ForLoopState` (6 fields). Phase 2: `CodegenOutput`. Phase 3: `ViewEmitContext`.

### Theme 2: Signal Batching Absence (3 specialists: TS/React, Performance, Pragmatic Enforcer) — ⚠️ Acknowledged per ADR-028 D3

Every `signal.set()` triggers immediate synchronous cascading updates. With computed chains of depth 4+, effects execute multiple times per logical update. The TS/React Specialist identified this as diverging from established reactive frameworks. The Performance Specialist identified cascading DOM writes. The Pragmatic Enforcer noted this is a "build the right thing later" scenario.

**ADR-028 D3**: Synchronous = predictable = debuggable. The ~200-line runtime budget deliberately excludes batch infrastructure. This is a debug-runtime trade-off, not a bug. If OmniView becomes a production target, batching is the first optimization to add.

**Consensus**: Documented as intentional. Synchronous dedup worklist as optional future enhancement. No immediate action required.

### Theme 3: Component Teardown Incomplete (3 specialists: TS/React, Performance, Maintainability)

Component teardown only removes DOM. `__omni_subs`, `__omni_animations`, `__omni_forHandlers`, and `__omni_el._cache` are never cleaned up. Over long editor sessions with many hot-replaces, memory growth is measurable. The TS/React Specialist identified the signal disposal gap. The Performance Specialist identified animation entry accumulation. The Maintainability Expert identified the stale cache issue.

**Consensus**: Extend teardown to clean up all four global registries. Immediate fix: clear `__omni_el._cache` on teardown (1 line). Short-term: add signal/animations/forHandlers cleanup.

### Theme 4: String Allocation Overhead (3 specialists: Rust, Performance, Maintainability) — ⚠️ Acknowledged per ADR-028 D2

103 `push_str(&format!(...))` sites across 11 files generate unnecessary intermediate allocations. The `escape_html` function allocates 5 intermediate strings per call. The Rust Expert recommended `write!()` replacement. The Performance Specialist calculated ~125,000 temporary allocations per compilation.

**ADR-028 D2**: String-based codegen (not structured CodeWriter) is an intentional choice documented in ADR-012 — the imperative pattern matches OmniView's "generate entire app in one shot" requirement. The `push_str(&format!(...))` vs `write!()` inconsistency is a style issue, not a design flaw.

**Consensus**: Mechanical `write!()` conversion is a code-quality improvement (not architectural). `escape_html` single-pass optimization is a quick win. Neither is urgent.

### Theme 5: Fragile Component Inlining State (3 specialists: Maintainability, Rust, Compiler Architect)

`try_inline_component` manually saves/restores 9 fields with no compiler enforcement. Adding a new per-component field requires remembering this location. The inline comment `// review v2 finding` confirms historical bugs. All three specialists recommended extracting into a helper struct.

**Consensus**: Extract `InliningState` struct with `save()`/`restore()` methods. Short-term fix.

---

## Trade-offs and Disagreements

### Trade-off 1: Multi-Backend Architecture vs Maintenance Cost

- **Systems Architect**: Multi-target is the core value proposition — "write once, run anywhere." The shared IR is architecturally sound. Paused backends preserved for future activation.
- **Pragmatic Enforcer**: 3 active + 6 paused backends triples maintenance burden. Cross-target parity tests only check string presence. Ratio of 2.25 exceeds acceptable threshold.
- **Resolution**: The team agrees to **freeze paused backends** (no maintenance, accept degradation) and **focus investment on OmniView as primary target**. React/Flutter maintained as best-effort. This preserves the architecture while reducing maintenance burden.

### Trade-off 2: CSS url() Sanitization Strictness

- **Security Specialist**: Runtime sanitizer correctly allows `url(https:...)` with protocol check. Compile-time sanitizer should align. Both should handle CSS escape sequences.
- **Performance Specialist**: CSS escape normalization adds runtime cost to every style binding. For developer-controlled signal values, the risk is theoretical.
- **Resolution**: Align compile-time and runtime behavior first (immediate). CSS escape normalization deferred to short-term (M effort, low urgency). ADR-027 risk acceptance ("signals are developer-controlled") remains valid for now.

### Trade-off 3: Language Purity vs Implementation Pragmatism

- **Domain Expert**: `VariantIR` name clash, prop mutability gap, component style vs design system capability gap — these are domain model leaks that erode developer trust.
- **Compiler Architect**: The IR is a pragmatic intermediate representation, not a user-facing API. String-based JS generation is standard for codegen. The gaps are real but prioritizing runtime correctness over model purity is correct.
- **Resolution**: Fix the HIGH-severity issues (VariantIR rename, prop immutability check). Defer domain model expansion (pseudo-states, at-rules) until there's user demand. The IR serves codegen, not documentation.

---

## Consolidated Findings

### Strengths

1. **Clean compiler pipeline**: AST → IR → Codegen separation with well-typed IR shared across targets. The pipeline is testable, debuggable, and extensible.

2. **Inline signal runtime is appropriately simple**: ~200 lines, no dependencies, ES5-compatible. The reactive triad (signal, computed, effect) is complete and well-tested.

3. **Three-layer security defense**: Compile-time static checks, runtime sanitizer functions, Content-Security-Policy. Each layer covers a different boundary with no single point of failure.

4. **Comprehensive test suite**: 800+ unit tests, 13 insta snapshots, property-based tests for escaping. 35% test-to-code ratio enables confident refactoring.

5. **CSS-only mode for editor reuse**: Same codegen pipeline with JS disabled guarantees style consistency between editor preview and exported output.

6. **Deferred JS execution model**: DOM created in one shot via innerHTML, then reactively bound. Correctly avoids incremental DOM construction overhead.

7. **Component hot-replace protocol**: Signal value snapshot/restore with `(0, eval)` indirect eval for editor live preview. Clean separation between old and new component instances.

### Areas for Improvement

1. **God Object decomposition**: `OmniViewGenerator` at 46+ fields needs phased decomposition into focused sub-structs. Priority: HIGH. Impact: Reduces bug surface, improves testability, enables parallel development.

2. **Signal batching**: No batching causes redundant cascading updates. Priority: HIGH. Impact: Visible jank in animation-heavy UIs, wasted DOM writes.

3. **Component teardown completeness**: Only DOM is removed on unmount. Signal registries, animation entries, and caches accumulate. Priority: HIGH. Impact: Memory growth in long-running editor sessions.

4. **BlockStmtIR bug**: Valid `.omni` with bare blocks produces broken OmniView output. Priority: CRITICAL. Impact: Silent incorrect output, cross-target inconsistency.

5. **`__omni_activeEffect` leak**: Subscriber exception corrupts global reactive state. Priority: CRITICAL. Impact: Silent UI freezes with no error trace.

6. **Motion trigger code duplication**: ~280 lines of near-identical code between named/inline branches. Priority: MEDIUM. Impact: Bug fix divergence, maintenance friction.

7. **Cross-backend test quality**: String presence checks don't verify semantic equivalence. Priority: MEDIUM. Impact: Cross-target bugs undetected by CI.

### Technical Debt

**High Priority**:
- **God Object (OmniViewGenerator)**: 46 fields, 65+ methods, 15+ files. Resolution: Phased decomposition. Effort: L (2-3 days). Timeline: Start within 2 weeks.
- **push_str(&format!(...)) pattern**: 103 sites generating intermediate allocations. Resolution: Mechanical write!() conversion. Effort: M (2-3 hours). Timeline: Within 1 week.

**Medium Priority**:
- **Motion trigger duplication**: ~280 lines duplicated. Resolution: Extract trigger-kind helpers. Effort: M. Timeline: Next sprint.
- **Fragile inlining state save/restore**: 9 fields manual save/restore. Resolution: Extract InliningState struct. Effort: S. Timeline: Next sprint.
- **Missing WASM cache**: compile_multi_to_omniview doesn't use ANALYSE_CACHE. Resolution: Wire up existing cache. Effort: S. Timeline: Next sprint.

**Low Priority**:
- **Dead field current_nid_path**: Never written. Resolution: Remove. Effort: XS. Timeline: Immediate.
- **Large test files**: tests.rs (1,485 lines), motion.rs tests (1,259 lines). Resolution: Split into modules. Effort: M. Timeline: When touching those files.
- **Paused backends as dead code**: 6 backends not compiled or tested. Resolution: Freeze formally. Effort: XS. Timeline: Policy decision.

### Risks

**Technical Risks**:
- **God Object accretion** (Likelihood: HIGH, Impact: MEDIUM): Each new feature adds fields to OmniViewGenerator. At 60+ fields, maintainability collapses. Mitigation: Begin phased decomposition now. Owner: Compiler team.

- **Cross-target silent divergence** (Likelihood: MEDIUM, Impact: HIGH): Changes to one backend not replicated to others. Current tests don't catch semantic differences. Mitigation: Add semantic cross-backend tests. Owner: Codegen team.

- **@enter replay performance cliff** (Likelihood: MEDIUM, Impact: MEDIUM): At 50+ signals, per-frame animation rebuild costs become measurable. Mitigation: Restrict replay dependencies. Owner: Motion/animation owner.

**Operational Risks**:
- **No production deployment** (Likelihood: HIGH, Impact: HIGH): Framework is well-tested but unproven in production. All performance concerns are theoretical. Mitigation: Ship one real application. Owner: Project lead.

- **ADR implementation tracking gap** (Likelihood: MEDIUM, Impact: LOW): Rapid ADR creation may outpace implementation. Mitigation: Audit ADR status, create tracking issues. Owner: Tech lead.

**Business Risks**:
- **Competing against mature ecosystems** (Likelihood: HIGH, Impact: HIGH): React, Vue, Svelte, Solid have massive ecosystems. OmniView competes on language integration but lacks ecosystem. Mitigation: Focus on use cases where language-first approach provides unique value (Poiesis editor, embedded widgets). Owner: Product/strategy.

---

## Recommendations

### Immediate (0-2 weeks)

1. **Fix BlockStmtIR handling** (CRITICAL)
   - **Why**: Valid `.omni` source produces broken OmniView output. Fix is 15 lines.
   - **How**: Add `StatementIR::BlockStmtIR` arm to `emit_statement` and `stmt_to_js_inline` in `stmt.rs`
   - **Success Criteria**: Bare blocks in function bodies produce correct JS output. Test coverage added.
   - **Estimated Effort**: XS (30 minutes)

2. **Fix `__omni_activeEffect` leak** (CRITICAL)
   - **Why**: Subscriber exception corrupts global reactive state silently.
   - **How**: Add try/catch/finally around `e.run()` in signal setter at `runtime.rs:39-45`
   - **Success Criteria**: Throwing subscriber doesn't block remaining subscribers or corrupt activeEffect.
   - **Estimated Effort**: XS (15 minutes)

3. **Add missing CSP directives** (HIGH)
   - **Why**: `form-action`, `object-src`, `frame-src` gaps weaken defense-in-depth.
   - **How**: Add `form-action 'self'; object-src 'none'; frame-src 'none'` to CSP string at `utils.rs:248`
   - **Success Criteria**: CSP header includes all three directives. Iframe behavior unchanged.
   - **Estimated Effort**: XS (5 minutes)

4. **Clear `__omni_el._cache` on teardown** (HIGH)
   - **Why**: Stale cache entries accumulate over editor sessions.
   - **How**: Add `__omni_el._cache = {}` in teardown path at `editor.rs:108`
   - **Success Criteria**: Hot-replaced components don't retain cache entries for removed DOM.
   - **Estimated Effort**: XS (5 minutes)

5. **Fix `StaggerDirection::Random` → `"edges"` bug** (HIGH)
   - **Why**: Silently produces wrong animation behavior.
   - **How**: Change to `"random"` or emit warning at `motion_config.rs:251`
   - **Success Criteria**: staggerFrom: random produces random stagger behavior or documented warning.
   - **Estimated Effort**: XS (2 minutes)

6. **Fix known test failure `test_resolve_styles_with_design_class`** (HIGH)
   - **Why**: Test has been failing across 3 review cycles. Degrades CI signal — "red is normal" masks real regressions.
   - **How**: Fix `design_class_names` lookup in `emit_element_props` at `view/element.rs`. The `design_class_names` HashSet is built at `mod.rs:504-506` but the warning check may not be using it correctly.
   - **Success Criteria**: Test passes, zero warnings for design-defined classes.
   - **Estimated Effort**: S (1-2 hours)

7. **Remove dead field `current_nid_path`** (LOW)
   - **Why**: Never written, initialized only. Cleanup.
   - **How**: Remove field and initialization in `mod.rs`
   - **Success Criteria**: Compiles clean, all tests pass.
   - **Estimated Effort**: XS (2 minutes)

### Short-term (2-8 weeks)

1. **Implement signal batching with dedup worklist**
   - **Why**: Cascading DOM writes cause visual jank in animation-heavy UIs.
   - **How**: Add `__omni_batchDepth` counter, `__omni_pendingEffects` Set, and deferred flush
   - **Success Criteria**: Multiple sequential signal updates trigger at most one effect execution per effect.
   - **Estimated Effort**: M (1-2 days)

2. **Add DocumentFragment batching for @for DOM insertion**
   - **Why**: N sequential layout recalculations for N-item lists.
   - **How**: Accumulate clones into DocumentFragment, insert once at `list.rs:333`
   - **Success Criteria**: Single layout pass regardless of list size.
   - **Estimated Effort**: S (1 hour)

3. **Convert `push_str(&format!(...))` to `write!()`**
   - **Why**: 103 sites generating unnecessary intermediate allocations.
   - **How**: Mechanical find-replace, enable `clippy::format_push_string` lint
   - **Success Criteria**: Zero `push_str(&format!(...))` sites. Clippy clean.
   - **Estimated Effort**: M (2-3 hours)

4. **Extract InliningState save/restore helper struct**
   - **Why**: Prevent field drift when adding new per-component state.
   - **How**: Create `InliningState` struct with `save()`/`restore()` methods at `component_html.rs`
   - **Success Criteria**: Adding a new per-component field to generator requires updating InliningState.
   - **Estimated Effort**: S (1-2 hours)

5. **Rename `design::VariantIR` to `DesignVariantIR`**
   - **Why**: Eliminates HIGH-severity name clash with `logic::VariantIR`.
   - **How**: Mechanical rename across 6-8 files
   - **Success Criteria**: No ambiguity between design variant and enum variant types.
   - **Estimated Effort**: M (1-2 hours, mechanical)

6. **Restrict @enter replay dependencies**
   - **Why**: Every signal change triggers all @enter animation rebuilds.
   - **How**: IR analysis to trace which signals each animation config actually references
   - **Success Criteria**: Unrelated signal changes don't trigger @enter replay.
   - **Estimated Effort**: M (3-4 hours)

7. **Extend component teardown to clean up registries**
   - **Why**: `__omni_subs`, `__omni_animations`, `__omni_forHandlers` accumulate over sessions.
   - **How**: Add cleanup iteration in unmount handler at `store.rs`
   - **Success Criteria**: Hot-replaced component registries are fully cleaned.
   - **Estimated Effort**: M (half day)

8. **Add prop immutability compile check**
   - **Why**: Doc says "immutable" but JS emits mutable signal.
   - **How**: Add semantic analysis pass rejecting direct prop assignment in component bodies
   - **Success Criteria**: Compile-time error when prop is assigned within its component.
   - **Estimated Effort**: S (1-2 hours)

9. **Audit ADR implementation status**
   - **Why**: 3 ADRs in 3 days — may have deferred items.
   - **How**: Cross-reference ADR decisions with git history, create tracking issues
   - **Success Criteria**: Every ADR decision has clear status: done, in-progress, or deferred.
   - **Estimated Effort**: S (1 hour)

10. **Extract motion trigger-kind helpers**
    - **Why**: ~280 lines of duplicated code between named/inline branches.
    - **How**: Create `emit_trigger_enter`, `emit_trigger_hover`, etc. with factory-call parameter
    - **Success Criteria**: Single implementation per trigger kind, shared by named and inline paths.
    - **Estimated Effort**: M (1-2 hours)

11. **Add browser-level JS runtime tests**
    - **Why**: Signals, sanitizers, DOM cache, error reporting verified only via Rust string-content checks. A broken `===` in generated JS would pass all 800+ tests.
    - **How**: Node.js/Deno script that loads generated JS output and asserts: `__omni_sanitizeUrl("javascript:alert(1)")` returns `"#blocked"`, signal set/get roundtrips, effect execution fires. Add CI check for any `runtime.rs` changes.
    - **Success Criteria**: All runtime functions have at least one behavioral assertion in a JS engine.
    - **Estimated Effort**: M (half day + CI integration)

### Long-term (2-6 months)

1. **Begin God Object decomposition (Phase 1: ForLoopState)**
   - **Why**: Primary architectural risk. Sustains development velocity.
   - **How**: Extract 6 @for template fields into `ForLoopState` struct, pass explicitly
   - **Success Criteria**: @for fields no longer on OmniViewGenerator. All tests pass.
   - **Estimated Effort**: M (half day for Phase 1)

2. **Add output minification pass**
   - **Why**: Generated JS/CSS 3-5× larger than necessary.
   - **How**: Optional WASM-based minifier in `build_file_output` path
   - **Success Criteria**: Minified output ≤ 40% of unminified size. Roundtrip: unminified → minified → parse yields same behavior.
   - **Estimated Effort**: M (2-3 days)

3. **Ship one production application**
   - **Why**: Validates framework with real data, uncovers real performance characteristics.
   - **How**: Internal tool or customer-facing widget using OmniView
   - **Success Criteria**: Running in production for ≥ 2 weeks with monitoring.
   - **Estimated Effort**: L (depends on app)

4. **Reconcile WASM and native compilation paths**
   - **Why**: Two independent compilation pipelines create drift risk.
   - **How**: Remove bypass in `compile_multi_internal_post`, route OmniView through standard dispatch
   - **Success Criteria**: Single code path for OmniView compilation in both WASM and native.
   - **Estimated Effort**: L (2-3 days)

5. **Migrate UiCodegen from delegation to shared defaults**
   - **Why**: Reduces cost of adding new UI backends from L to M.
   - **How**: Move shared component/store codegen logic into trait default implementations
   - **Success Criteria**: Adding a new backend requires implementing only diff points.
   - **Estimated Effort**: L (3-5 days)

---

## Success Metrics

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| OmniViewGenerator field count | 46 | ≤ 30 | 3 months |
| push_str(&format!(...)) sites | 103 | 0 | 2 weeks |
| Motion trigger duplication | ~280 lines | ≤ 50 lines | 4 weeks |
| Signal batching | None | Synchronous dedup | 4 weeks |
| CSP directive coverage | 6 directives | 9 directives | Immediate |
| Production deployments | 0 | 1+ | 3 months |
| Cross-backend semantic tests | String presence only | Feature-specific logic checks | 6 weeks |
| Test coverage (OmniView codegen) | ~800 tests | Maintain or grow | Ongoing |
| ADR implementation tracking | None | Per-ADR issues | 1 week |

---

## Follow-up

**Next Review**: After Phase 1 God Object decomposition (target: 2026-08-15)

**Tracking**: Create GitHub issues for every Immediate recommendation. Add Short-term items to sprint backlog. Tag with `architecture-review-2026-07-29` label.

**Recalibration**: After implementing critical and immediate actions, re-run a focused review:
```
"Start architecture recalibration for component-omniview"
```

**Accountability**:
- Compiler team: BlockStmtIR fix, God Object decomposition
- Runtime team: activeEffect fix, signal batching, teardown cleanup
- Security: CSP directives, CSS sanitizer alignment
- Domain: VariantIR rename, prop immutability check

---

## Related Documentation

**Architectural Decision Records**:
- [ADR-028: OmniView as Debug/Development Runtime — Intentional Design Trade-offs](../decisions/adrs/ADR-028-omniview-debug-runtime-design-tradeoffs.md) — **Created from this review.** Documents 14 intentional design decisions that were initially flagged as concerns.
- [ADR-018: OmniView Event System Architecture](../decisions/adrs/ADR-018-omniview-event-system-architecture.md)
- [ADR-025: Review Remediation 2026-07-29](../decisions/adrs/ADR-025-omniview-review-remediation-2026-07-29.md)
- [ADR-026: Review v2 Fixes](../decisions/adrs/ADR-026-omniview-review-v2-fixes.md)
- [ADR-027: Security Sanitization Architecture](../decisions/adrs/ADR-027-omniview-security-sanitization-architecture.md)
- [ADR-012: OmniView Codegen as Debug Tool](../decisions/adrs/ADR-012-omniview-codegen-debug-tool.md) — Established OmniView's debug-tool role

**Previous Reviews**: This is the first comprehensive OmniView architecture review in the `.architecture/reviews/` format. Previous reviews were documented in ADRs (ADR-025, ADR-026) and project memory files.

**Referenced Documents**:
- [OmniView Grammar Reference](../../../docs/omniview/README.md) — 15-chapter language and framework reference
- [OmniView Architecture Assessment](../../memory/omniview-architecture-assessment.md) — Prior assessment: A-grade, Language-First Citizens validated
- [OmniView Compiler Internals](../../memory/omniview-compiler-internals.md) — 21-module deep-dive of the compiler

---

## Appendix

### Review Methodology

This review was conducted using the AI Software Architect framework v1.6.0 with **10 team members** reviewing independently in parallel, followed by cross-cutting analysis and aggregation.

**Participating Members**:
- **Compiler & Language Architect** (Dr. Lin Wei): Language design, multi-target codegen, IR optimization
- **TypeScript & React Specialist** (Emma Rodriguez): React component architecture, TypeScript type systems, frontend state management
- **Security Specialist**: Threat modeling, security patterns, data protection
- **Rust Expert** (Katya Volkov): Memory safety, zero-cost abstractions, compiler design
- **Maintainability Expert**: Code quality, refactoring, technical debt
- **Performance Specialist**: Performance optimization, resource utilization, scalability
- **Domain Expert**: Domain-driven design, bounded contexts, ubiquitous language
- **Pragmatic Enforcer**: YAGNI principles, incremental design, complexity analysis
- **Systems Architect**: Distributed systems, service architecture, multi-backend integration
- **Implementation Strategist**: Change sequencing, blast radius analysis, production readiness

**Pragmatic Mode**: Balanced (max complexity ratio: 1.5, min necessity score: 7)
- Motion/animation system (ratio 1.6) exceeds threshold
- Multi-target architecture (ratio 2.25) significantly exceeds threshold
- Deferred motion complexity recommendation included
- Paused backend freeze recommended

### Finding Summary by Severity

| Severity | Count | Key Items |
|----------|-------|-----------|
| CRITICAL | 2 | BlockStmtIR bug, activeEffect leak |
| HIGH | 10 | God Object, signal batching, @enter replay, CSP gaps, teardown incomplete, VariantIR clash, motion duplication, inlining state, bypass path, no production deployment |
| MEDIUM | 20 | push_str(format!()), StaggerDirection bug, computed dual-path, storeAction limitations, UI codegen delegation, CSS sanitizer inconsistency, WASM cache gap, debug CSP absence, CSS escape bypass, cross-backend tests, escape_html allocations, etc. |
| LOW | 12 | Dead field, magic strings, large test files, selector injection, naming asymmetry, etc. |

### Glossary

| Term | Definition |
|------|------------|
| **Language-First Citizen** | A UI concept elevated to a language keyword with dedicated syntax, compiler support, and codegen rules |
| **IR** | Intermediate Representation — typed data structures between parser and codegen |
| **nid** | Node ID — unique identifier for each DOM element, used for reactive binding |
| **css_only** | Editor preview mode that generates CSS only, skipping JS emission |
| **WAAPI** | Web Animations API — browser-native animation system |
| **FxHashMap/FxHashSet** | `rustc_hash` crate's fast hashing maps/sets (non-cryptographic) |
| **CSP** | Content Security Policy — browser security header |
| **@enter replay** | Mechanism that re-creates entrance animations when any signal changes |
| **Poiesis** | Tauri-based visual editor for OmniLang projects |

---

**Review Complete**
