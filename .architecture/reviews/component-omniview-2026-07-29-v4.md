# OmniView Component Architecture Review — v4 (2026-07-29)

**Review type:** Comprehensive 10-perspective architecture review
**Target:** OmniView component system (IR → codegen pipeline, all backends)
**Reviewers:** Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, TypeScript & React Specialist, Compiler & Language Architect, Rust Expert, Pragmatic Enforcer
**Date:** 2026-07-29

---

## Executive Summary

OmniView's component architecture is **Adequate** — fundamentally sound with Language-First design validated by industry trends, but carrying accumulated technical debt that creates real correctness and performance risks.

**Overall assessment:** The core IR abstraction (`ComponentIR`/`ViewTree`) unifies 9 codegen backends correctly. The compile-time inlining strategy for OmniView is architecturally elegant. However, three structural issues demand attention: (1) the `InliningState` save/restore pattern has caused bugs twice and remains fragile; (2) cross-backend semantic gaps (slots, lifecycles, CSS→Flutter) produce silently incorrect output in non-OmniView targets; (3) dead IR fields and unexecuted ADR decisions add noise and waste compilation cycles.

**Top 3 immediate concerns:**
1. Slot projection silently fails in React/Flutter — children are never rendered (Compiler Architect HIGH-1)
2. Flutter CSS produces syntactically invalid Dart — `EdgeInsets.all("10px")` won't compile (Compiler Architect HIGH-2)
3. Dynamic motion parameters serialized as strings in React — `parseFloat("myVar")` → `NaN` (TS/React Specialist C1)

---

## Cross-Cutting Themes

### Theme 1: InliningState Field Coverage Fragility (4 specialists)

**Flagged by:** Systems Architect (C1), Maintainability Expert (C2), Implementation Strategist (C1), Rust Expert (O2)

The `InliningState` struct (`mod.rs:559-598`) saves/restores 8 of 46 `OmniViewGenerator` fields. When a new field is added that needs per-component isolation, nothing forces the developer to add it to `InliningState`. ADR-025 D1 and ADR-026 D3 document real bugs from this pattern. The fix needs to move from "documented convention" to "compiler-enforced invariant."

**Consensus recommendation:** Extract per-component fields into a `ComponentScope` struct that is atomically saved/restored during inlining. The compiler then enforces that all per-component state lives in this struct. Effort: L, but eliminates a proven bug class.

### Theme 2: `push_str(&format!(...))` Inefficiency (4 specialists)

**Flagged by:** Performance Specialist (M3), Maintainability Expert (C1), Rust Expert (C3), Implementation Strategist (C4)

78 instances across 10 files create temporary `String` allocations that are immediately consumed. The `write!`/`writeln!` macros write directly into the target buffer. This is a mechanical fix — zero behavioral change, only efficiency improvement.

**Consensus recommendation:** Mechanical replacement of all 78 instances. Use `write!(self.js, ...)`/`writeln!(self.js, ...)`. Effort: S.

### Theme 3: Cross-Backend Semantic Gaps (3+ specialists)

**Flagged by:** Compiler Architect (HIGH-1, HIGH-2), Domain Expert (HIGH-1, HIGH-2), Systems Architect (C2)

The shared IR abstraction masks fundamentally different execution models: OmniView's compile-time inlining vs React/Flutter's runtime composition. Slots, lifecycles, and CSS→Widget mapping all diverge. The `ElementIR` type conflates HTML elements with component instances — the distinction is only rediscovered at codegen time via `component_map` lookups.

**Consensus recommendation:** At minimum, add codegen warnings when slot-using components target React/Flutter. Long-term: add `ViewTree::Component` variant and `ComponentRuntimeModel` enum to `ComponentIR`.

### Theme 4: Dead/Speculative Fields on ComponentIR (3 specialists)

**Flagged by:** Pragmatic Enforcer (C1, C2, C3), Domain Expert (C5), Compiler Architect (LOW-1)

- `generic_params: Vec<String>` — zero consumers, no parser grammar, pure speculation
- `if_mode_default: Option<String>` — vestigial from removed animation features (ADR-032)
- TypeScript declarations generated on every codegen call despite ADR-031 D6 explicitly deciding to skip

**Consensus recommendation:** Remove `generic_params` and `if_mode_default`. Implement ADR-031 D6 (gate TypeScript gen). Combined effort: S-M. Net reduction: ~2 fields + 298 lines of active code.

### Theme 5: Event Field Extraction Duplication (2 specialists)

**Flagged by:** Implementation Strategist (C2), Maintainability Expert (C8)

Four `field_extract_*` functions across JS/React/Flutter/Dioxus backends manually mirror `event_data_schema()`. Adding a new event field requires updating 5 locations, and the compiler does not catch omissions.

**Consensus recommendation:** Generate field_extract functions from `event_data_schema()` or introduce a shared `FieldExtractor` trait. Effort: M-L.

---

## Trade-offs and Disagreements

### Trade-off 1: `innerHTML` DOM insertion
**Security Specialist** (MEDIUM-3) recommends migrating to DOM API (`createElement`/`setAttribute`/`textContent`) to eliminate the entire `innerHTML` XSS surface. **Performance Specialist** notes that `innerHTML` is the fastest bulk-DOM-insertion method in browsers (single parse, single reflow). **Systems Architect** notes the deferred JS swap pattern depends on bulk DOM insertion being complete before event binding.

→ **Resolution:** Keep `innerHTML` with strengthened escaping. The property-tested template-literal escaping is the correct defense. Add a secondary HTML-sanitization pass if supply-chain risk is a concern. Do not migrate to DOM API — the performance cost is not justified.

### Trade-off 2: `React.memo` wrapping for generated components
**TS/React Specialist** (C6) notes that generated React components are not wrapped in `React.memo`, causing unnecessary re-renders. **Pragmatic Enforcer** counters that auto-wrapping every component could break `children`/context semantics and adds complexity without proven need.

→ **Resolution:** Do not auto-wrap. Add a language-level `@memo` directive if users request it. YAGNI applies.

### Trade-off 3: OmniView's compile-time inlining vs React's runtime composition
**Domain Expert** argues the IR should distinguish between the two models explicitly (`ComponentRuntimeModel` enum). **Systems Architect** notes this adds complexity to the IR for a distinction that only matters at codegen time. **Implementation Strategist** notes the blast radius of adding a new IR field touches all 9 backends.

→ **Resolution:** Add `ComponentRuntimeModel` but only as informational metadata (not used for dispatch). Backends that need it can check it. This preserves the shared IR while documenting the semantic gap.

---

## Prioritized Recommendations

### Immediate (this session — S effort each)

| # | Action | Severity | Effort | Specialist |
|---|--------|----------|--------|------------|
| I1 | Fix React dynamic motion parameter serialization — use JSX expressions `{expr}` for non-literal values in `data-inview-threshold`, `data-scroll-start/end/scrub`, `data-drag-*` | HIGH | M (30min) | TS/React |
| I2 | Gate TypeScript declaration generation per ADR-031 D6 — comment out lines 393-395 in `mod.rs`, set `types = String::new()` | HIGH | S | Pragmatic |
| I3 | Add `sanitize_css_value()` to design-block CSS emission (`design.rs:126, 158, 171`) | MEDIUM | S | Security |
| I4 | Add codegen warning when slot-using component is compiled for React/Flutter | HIGH | S | Compiler |
| I5 | Replace `panic!` with graceful degradation in `typescript.rs:145` (`TypeKind` wildcard arm) | MEDIUM | S | Rust |

### Short-term (this sprint — M effort)

| # | Action | Severity | Effort | Specialist |
|---|--------|----------|--------|------------|
| S1 | Extract `InliningState`-covered fields into `ComponentScope` struct for compiler-enforced isolation | HIGH | L | Systems/Maintainability/Impl |
| S2 | Fix Flutter CSS: parse numeric values from CSS strings, expand property mapping beyond 4 properties | HIGH | L | Compiler |
| S3 | Add compile-time `#[cfg(debug_assertions)]` guard for InliningState field coverage | MEDIUM | M | Maintainability |
| S4 | Store element references in `Map<nid, Element>` during @for clone phase — replace `querySelector` with O(1) map lookup | HIGH | M | Performance |
| S5 | Remove `generic_params` and `if_mode_default` from ComponentIR/StoreIR pipeline | HIGH | M | Pragmatic |
| S6 | Replace all 78 `push_str(&format!(...))` with `write!`/`writeln!` | MEDIUM | S | Performance/Rust/Maintainability |
| S7 | Add `EventDeclIR` struct to replace `event_names`/`event_param_types` parallel arrays | MEDIUM | S | Domain |
| S8 | Add fast path in `try_inline_component` to borrow view tree without clone when `slot_children` is empty | HIGH | S | Performance |
| S9 | Change `__omni_animations` from flat Map to nid-keyed nested Map for O(1) per-element lookup | HIGH | S | Performance |

### Long-term (next sprint — L effort)

| # | Action | Effort |
|---|--------|--------|
| L1 | Add `ViewTree::Component` variant for first-class component instance representation in IR | L |
| L2 | Generate `field_extract_*` from `event_data_schema()` via shared trait or codegen | M |
| L3 | Implement required-prop validation in semantic analysis pass | M |
| L4 | Split `OmniViewGenerator` monolith into composed sub-structs (`JsBuffer`, `HtmlBuffer`, `ScopeState`, `DesignContext`) | L |
| L5 | Add `TypeKind::Union` and `TypeKind::Fn` variants to preserve structural type information through lowering | M |
| L6 | Migrate OmniView backend to `CodeWriter` for output strategy unification with other 8 backends | L |
| L7 | Implement runtime slot emulation for React: translate `Slot(None)` → `{children}`, `Slot(Some("name"))` → `{props["name"]}` | M |

---

## Individual Member Reviews

### 1. Systems Architect

**Perspective:** Evaluates how components work together as a cohesive system. OmniView's core strength is the shared IR abstraction, but the divergence of composition models (compile-time inlining vs runtime imports) creates systemic fragility.

**Key observations:**
- Two fundamentally different component systems share one IR — intentional but risky
- `OmniViewGenerator` is a 46-field monolithic state bag
- ViewTree traversal logic remains duplicated despite ADR-023 consolidation
- Signal-driven runtime sanitization creates a correct but complex two-layer security model
- The deferred JS swap pattern is lightweight but has been a repeated bug source

**Strengths:**
- Shared IR provides single source of truth (HIGH value)
- Compile-time inlining eliminates runtime component overhead (HIGH value)
- InliningState pattern is well-structured despite being manual (MEDIUM value)
- ADR-023 shared ViewTree traversal reduced duplication by 119 lines (MEDIUM value)
- Phase assertion guards provide compile-time safety for JIT ordering (HIGH value)

**Concerns:**
- **HIGH:** InliningState save/restore covers 8/46 fields — proven bug source (ADR-025 D1, ADR-026 D3)
- **MEDIUM:** Cross-backend composition model divergence creates implicit feature gaps
- **MEDIUM:** React codegen has independent recursive ViewTree walkers duplicating ADR-023 pattern
- **LOW:** Component name resolution by tag lookup has no namespace isolation
- **MEDIUM:** Deferred JS four-buffer architecture remains complex despite phase guards

**Recommendations:** Immediate: add `view_tree_collect` generic collector; add known-HTML-element guard. Short-term: formalize cross-backend feature tracking; document buffer protocol in type system. Long-term: refactor OmniViewGenerator into scoped concern groups.

---

### 2. Domain Expert

**Perspective:** Evaluates how well the architecture represents the problem domain. The IR conflates key domain concepts (Element vs Component), and cross-backend lifecycle semantics diverge without IR-level documentation.

**Key observations:**
- `ElementIR` merges "HTML element" and "component instance" into one type
- `ConstructionKind` enum has drifted — contains unused variants, missing `Component`
- `event_names`/`event_param_types` parallel arrays are fragile
- Cross-backend lifecycle semantics differ but IR assumes uniformity
- Infrastructure fields (`source_file`) exist in domain types

**Strengths:**
- Signal/Computed/Effect/Lifecycle separation is rigorous
- Event data modeling is comprehensive (21 kinds with typed field schemas)
- ViewTree covers all view structure concepts
- Design system has independent IR
- Module-level organization reflects filesystem structure

**Concerns:**
- **HIGH:** ElementIR conflates HTML elements and component instances — no `ViewTree::Component` variant
- **HIGH:** Cross-backend component lifecycle semantics inconsistent and not represented in IR
- **MEDIUM:** `event_names`/`event_param_types` as parallel arrays
- **MEDIUM:** `ConstructionKind` contains unused variants (`Normal`, `Static`), missing `Component`
- **LOW:** Infrastructure data (`source_file`) in domain types

**Recommendations:** Immediate: add `EventDeclIR` struct; clean up `ConstructionKind`. Short-term: add `ViewTree::Component` variant; add `ComponentRuntimeModel` enum. Long-term: component validation in IR; shared component detection logic.

---

### 3. Security Specialist

**Perspective:** Examines injection vectors, data flow security, trust boundaries. The three-layer defense-in-depth is structurally sound, but design-block CSS and @for URL attributes have sanitization gaps.

**Key observations:**
- Trust model is binary — no distinction between first-party and third-party components
- `innerHTML` is the single point of DOM insertion failure
- @for templates create a parallel codegen path with weaker sanitization
- Design-block CSS values bypass compile-time sanitization
- CSP provides strong outer layer but `img-src` allows `https:` (CSS data exfiltration possible)

**Strengths:**
- Defense-in-depth is structurally sound (3 independent layers)
- Sanitizer design well-considered (`String()` coercion, CR/LF/TAB stripping, CSS escape normalization)
- Cross-origin communication correctly locked down
- Read-only prop flow prevents child-to-parent signal mutation
- Error reporting does not leak sensitive data

**Concerns:**
- **MEDIUM:** URL-valued dynamic attributes in @for items bypass runtime sanitization (currently non-functional, but a fix could create regression)
- **MEDIUM:** Design-block CSS class values are not compile-time sanitized (inconsistent with inline styles)
- **MEDIUM:** `innerHTML` as sole DOM insertion mechanism — fragile single point of failure
- **LOW:** CSS token values not validated/escaped
- **LOW:** `__omni_runtimeErrors` exposed on global `window` scope
- **LOW:** Expression codegen produces arbitrary JS without output validation

**Recommendations:** Immediate: add `sanitize_css_value` to design-block CSS; add regression test for @for URL sanitization. Short-term: add compile-time identifier validation in `expr_to_js`; remove errors from global scope. Long-term: migrate from `innerHTML` to DOM API; implement component-level trust boundaries.

---

### 4. Maintainability Expert

**Perspective:** Evaluates code organization, complexity, technical debt, evolutionary design. Code is well-organized but carries systematic inefficiencies and manual sync requirements.

**Key observations:**
- Explicit state machine with 46 fields but asymmetric save/restore coverage
- Cross-backend event extraction duplicated with no shared schema
- Two string-building idioms co-exist inconsistently (`push_str(&format!(...))` vs `write!`)
- Design-grade slot/view tree traversal duplicated across 5+ backends
- Generated JS embedded in Rust strings at scale (`emit_list_dom`: 365 lines)

**Strengths:**
- JS execution order documented and enforced (6-phase with asserts)
- Zero unwrap/panic/todo in production code
- InliningState extracted after saving real bugs twice
- Well-separated module boundaries by codegen concern
- Debug assertions catch state-machine misuse at dev time

**Concerns:**
- **MEDIUM:** Inconsistent string building (78 `push_str(&format!(...))` vs 82 `write!`) — style guide gap
- **MEDIUM:** InliningState covers 7/46 fields — future field additions at risk
- **MEDIUM:** `Result<String, String>` bare string errors throughout
- **MEDIUM:** `emit_list_dom` generates 365 lines of JS inside Rust push_str calls
- **LOW:** Triple attribute list duplication in `emit_element_props`
- **LOW:** Magic number 300 as fallback animation duration
- **LOW:** 11x repeated `motion_behavior().and_then()` calls
- **MEDIUM:** Cross-backend `field_extract_*` functions lack shared schema

**Recommendations:** Immediate: consolidate `push_str(&format!(...))` to `write!`; extract attribute lists; define duration constant. Short-term: typed `DesignError` enum; compile-time guard for InliningState; snapshot tests for `emit_list_dom`. Long-term: `FieldExtractor` trait; structured JS builder; consolidate Fragment traversal.

---

### 5. Performance Specialist

**Perspective:** Evaluates compile-time and runtime performance. The Svelte-style compiler-driven approach is correct, but several O(N×M) patterns in generated JS and unnecessary allocations in the compiler degrade performance.

**Key observations:**
- Compile-time string construction dominates the codegen hot path (78 `push_str(&format!(...))`)
- Runtime reactive system has no batched update model
- Component inlining is O(1) on Arc but view-tree cloning is O(n)
- @for bindings bypass element caching entirely (raw `querySelector` per item per binding)
- @enter replay re-runs all animations on any signal change within a component

**Strengths:**
- Arc-based component reference sharing avoids deep clones
- Deferred JS swap is O(1) and correctly separates DOM from bindings
- Shared scroll handler consolidates O(N) listeners into one
- Motion runtime conditionally loaded only when used (46KB gated)
- `FxHashMap`/`FxHashSet` for all lookup tables

**Concerns — HIGH:**
- **H1:** @for bindings use uncached DOM queries — O(I×B) per re-render. 200 items × 5 bindings = 1000 `querySelector` calls. Fix: store element references in `Map<nid, Element>` during clone phase.
- **H2:** `inject_slot_children` deep-clones entire child view tree even without slots. Fix: add fast path when `slot_children` is empty.
- **H3:** `__omni_animations.forEach` on paused override iterates ALL page animations for one nid. Fix: key animations by nid for O(1) lookup.

**Concerns — MEDIUM:** computed eager evaluation; effect cleanup chaining closure nesting; `push_str(&format!(...))` allocation pressure; @enter replay reads every component signal; drag handlers call `getBoundingClientRect()` on every pointer interaction.

**Recommendations:** Immediate: fast path for no-slot view clone; nid-keyed animation lookup. Short-term: @for element reference caching; `push_str` → `write!` migration; nested animation Map. Long-term: signal-level @enter dependency tracking; lazy computed evaluation; effect batching.

---

### 6. Implementation Strategist

**Perspective:** Evaluates change sequencing, blast radius, reversibility. The `OmniViewGenerator` monolith creates a high per-feature coordination cost, and the event extraction system has a known structural drift problem.

**Key observations:**
- `OmniViewGenerator` is the single bottleneck for all OmniView codegen changes (4 touch points per new field)
- Event data extraction has documented structural drift (5 locations must be manually synced)
- ViewTree variant additions have wide but compiler-enforced blast radius (~123 match arms across 16 files)
- React backend avoids InliningState problem through fundamentally different architectural approach
- Test revalidation surface is large but test quality is high (174 test functions)

**Strengths:**
- Compiler-enforced exhaustive matching prevents silent ViewTree variant misses
- InliningState exists and is documented with ADR cross-references
- Shared `view_tree_any()` (ADR-023) reduced 8 functions to 1 + 4 wrappers
- `field_extract_react` and `field_extract_flutter` use same structural pattern as shared `field_extract_js`
- `css_only` mode allows style-only recompilation without JS emission

**Concerns:**
- **HIGH:** InliningState save/restore not auto-synced with generator fields — proven recurrence (2 ADRs)
- **HIGH:** Event field extraction duplicated across 4 backends with no sync mechanism (5 coordinated touch points)
- **MEDIUM:** OmniViewGenerator monolith creates high per-feature coordination cost
- **LOW:** 78 `push_str(&format!(...))` double-allocation patterns
- **LOW:** Magic number 300 used as default animation duration

**Recommendations:** Immediate: add INVARIANT comment adjacent to InliningState; define duration constant. Short-term: replace `push_str(&format!(...))` with `write!`; generate field_extract from schema. Long-term: split OmniViewGenerator into sub-structs; replace manual `test_new()` with Default derive.

---

### 7. TypeScript & React Specialist

**Perspective:** Reviews generated React code quality and TypeScript type safety. The codegen is architecturally strong with precise dependency tracking, but has a real HIGH-severity bug in dynamic motion parameter serialization.

**Key observations:**
- Impressively precise dependency tracking for `useMemo`/`useCallback` deps
- React idioms well-handled (immutable spreads, Fragment, key prop, useEffect cleanup, aria-label, ESLint noise suppression)
- TypeScript type safety has gap: event param types default to `any`
- Dynamic motion parameter values serialize as string literals instead of JSX expressions (real bug)
- Zustand integration is architecturally sound

**Strengths:**
- Complete React pattern coverage (useState/useMemo/useCallback/useEffect)
- Dead code elimination via precise reachability analysis
- Intelligent setter name collision resolution
- CSS Modules integration with `styles['name']` syntax
- Thorough event data extraction (17 event kinds mapped to correct SyntheticEvent paths)

**Concerns:**
- **HIGH (C1):** Dynamic motion parameters serialized as string attributes instead of JSX expressions. `data-inview-threshold="myVar"` → `parseFloat("myVar")` = `NaN`. Affects `data-inview-threshold`, `data-scroll-start/end/scrub`, `data-drag-constraints/elastic/threshold/velocity-max`. Fix: distinguish literal vs expression values, emit `{expr}` for dynamic.
- **MEDIUM (C2):** Event handler parameter types default to `any` — should be `unknown`. Fix: change fallback in `imports.rs:141-144` and `typescript.rs:61-63, 142`.
- **MEDIUM (C3):** `JSX.Element` return type deprecated in React 19+. Fix: use `React.ReactNode`.
- **MEDIUM (C4):** Stale closure risk in transitive store action dependencies (theoretical, low probability).
- **LOW (C5):** Unused `_i` index parameter in list rendering adds noise.
- **LOW (C6):** No `React.memo` wrapping for pure components.

**Recommendations:** Immediate: fix dynamic motion parameter serialization (M); change `any` → `unknown` (S). Short-term: update `.d.ts` to `React.ReactNode` (S); add snapshot tests for dynamic motion values (M). Long-term: `@memo` directive for React.memo wrapping (L).

---

### 8. Compiler & Language Architect

**Perspective:** Evaluates language semantics, IR design, lowering correctness, cross-target consistency. Two HIGH-severity bugs found where valid Omni source produces incorrect output in non-OmniView targets.

**Key observations:**
- Component inlining vs runtime composition creates the dominant semantic gap
- Type annotations are systematically lossy in lowering (unions → strings, functions → `"function"`)
- Slots are a compile-time projection feature with no runtime equivalent
- `<input>` auto-binding couples lowering to fragile English-specific heuristic
- Flutter CSS-to-Widget mapping is lossy and syntactically incorrect

**Strengths:**
- Event system lowering is well-architected (two-phase: scan-then-collect)
- `FormatStmt`/`FormatExpr` traits achieve genuine deduplication (9 backends share ~80% logic)
- Component inlining state management is explicit (InliningState save/restore)
- Class map bindings correctly handled at IR level
- `ConstructionKind` distinguishes synthetic wrappers from user elements

**Concerns:**
- **HIGH-1:** Slot/children projection silently breaks in React (emits nothing) and Flutter (`SizedBox.shrink()`). Children passed by parent are never rendered.
- **HIGH-2:** Flutter CSS property mapping produces syntactically invalid Dart. `EdgeInsets.all("10px")` will not compile. Only 4 CSS properties mapped.
- **MEDIUM-1:** Union type representation is structurally lossy (type_args discarded, nullability lost).
- **MEDIUM-2:** Function type annotations erased to bare `"function"` string.
- **MEDIUM-3:** No compile-time validation of required props at component boundaries.
- **LOW-1:** `generic_params` always empty vec — no parser support.
- **LOW-2:** `InliningState` not RAII-guarded against panics during inlining.
- **LOW-3:** `<input>` auto-binding heuristic is English-specific and fragile.

**Recommendations:** Immediate: fix Flutter CSS numeric parsing (S); add slot gap warnings (S). Short-term: required prop validation pass (M); union/function type IR fixes (M); Flutter style expansion (L). Long-term: runtime slot emulation for React (M); component generics parser support (L); RAII guard for InliningState (S).

---

### 9. Rust Expert

**Perspective:** Evaluates ownership patterns, error handling, type safety, idiomatic Rust. Code is production-grade with zero unwrap in output paths, but diverges from shared `CodeWriter` infrastructure.

**Key observations:**
- Architecture split: OmniView bypasses `CodeWriter` used by all other 8 backends
- Inlining save/restore is integrity-check pattern, not architectural isolation
- `#[non_exhaustive]` on enums forces wildcard arms but `panic!` in those arms is risky
- `#[serde(untagged)]` on `ViewTree` makes `Clone` expensive and opaque
- Generator struct is monolithic but responsibility separation is reasonable

**Strengths:**
- Zero production `unwrap()` in output generation paths
- `std::mem::swap` for JS execution order is zero-cost and correct
- Compile-time dangerous URL scheme detection provides defense-in-depth
- `#[non_exhaustive]` on TypeKind and EventDataKind for forward compatibility
- InliningState field-by-purpose grouping with ADR cross-references

**Concerns:**
- **MEDIUM:** `panic!` in `typescript.rs:145` (TypeKind wildcard arm) — not guarded by `#[non_exhaustive]`
- **MEDIUM:** `unreachable!()` in `component.rs:77` — coupled to non-adjacent filter logic
- **LOW:** `push_str(&format!(...))` anti-pattern 78× in codegen hot path
- **LOW:** `ComponentIR` deep-cloned twice due to Arc wrapping after clone
- **LOW:** `strip_slot_prop` unconditionally clones entire ViewTree
- **LOW:** Inconsistent `writeln!().unwrap()` vs `let _ = writeln!()` (typescript.rs vs rest)

**Recommendations:** Immediate: replace panic with graceful degradation (S); unify writeln pattern (S). Short-term: `Cow<ViewTree>` for strip_slot_prop (S); refactor inject_slot_children to mutate in-place (M). Long-term: migrate OmniView to CodeWriter (L); store Arc<ComponentIR> in IRProgram (L); proc-macro for InliningState boilerplate (M).

---

### 10. Pragmatic Enforcer (YAGNI Guardian)

**Perspective:** Evaluates necessity vs complexity of every abstraction. The codebase is healthy in aggregate, but several speculative fields and unexecuted ADR decisions survive from before the cleanup wave.

**Key observations:**
- Codebase is healthy in aggregate — 32 ADRs, justified patterns (InliningState, view_tree_any, Fragment)
- Two ComponentIR fields are purely speculative (zero consumers)
- ADR-031 D6 decision was not executed — TypeScript gen still runs on every call
- EventHandlerIR variant duplication (FnRef/InlineFn share 5 fields)
- Editor concerns leak into codegen core

**Strengths:**
- `project_uses_motion()` guard — 46KB runtime conditionally embedded
- InliningState save/restore — justified complexity (fixed real bugs)
- `view_tree_any` shared utility — 41 lines, 6 callers, justified
- `#[serde(default)]` on optional fields prevents serialization breakage
- Motion runtime 46KB guarded

**Necessity/Complexity/Ratio Analysis:**

| Feature | Necessity | Complexity | Ratio | Verdict |
|---------|-----------|------------|-------|---------|
| `generic_params` | 0/10 | 1/10 | Speculative, no parser grammar | **Remove** |
| `if_mode_default` | 0/10 | 2/10 | Vestigial from removed animation features | **Remove** |
| TypeScript gen always-on | 1/10 | 1/10 | ADR decided to skip, not implemented | **Gate per ADR-031 D6** |
| EventHandlerIR duplication | 6/10 | 3/10 | Real variants, redundant modeling | **Extract base struct** |
| Editor mode in codegen | 5/10 | 4/10 | Leaky abstraction, works but messy | **Separate config** |

**Concerns:**
- **HIGH (C1):** `generic_params: Vec<String>` — zero consumers, pure speculative generality. Remove.
- **HIGH (C2):** `if_mode_default: Option<String>` — zero codegen consumers, vestigial. Remove.
- **HIGH (C3):** TypeScript declarations generated on every codegen call — zero production consumers, ADR-031 D6 not implemented. Gate.
- **MEDIUM (C4):** `FnRef` and `InlineFn` share 5 duplicated fields. Extract `EventHandlerBase`.
- **MEDIUM (C5):** Editor/debug mode as first-class fields on codegen struct. Separate config.
- **LOW (C6):** `event_param_types` as parallel array — fragile but lowering enforces sync.

**Recommendations:** Immediate: remove `generic_params` (S); gate TypeScript gen (S). Short-term: remove `if_mode_default` (M); extract `EventHandlerBase` (M). Long-term: separate editor config from core codegen (L).

---

## Risks If Unaddressed

| Risk | Probability | Impact | Concerns |
|------|-------------|--------|----------|
| Slot-using components silently fail on React/Flutter | High (any slot use) | High (missing content) | CA-H1 |
| Flutter output won't compile for styled components | High (any style use) | High (build break) | CA-H2 |
| Dynamic motion params silently break in React | Medium (signal-driven params) | Medium (broken animation) | TR-H1 |
| New generator field causes inlining state corruption | Medium (field additions ongoing) | High (incorrect JS) | SA-C1, MT-C2, IS-C1 |
| @for performance degrades at scale | Medium (large lists) | Medium (frame drops) | PF-H1 |
| CSS injection via design-block in 3rd-party components | Low (requires supply-chain) | High (data exfiltration) | SE-M2 |
| Event extraction drift across backends | Medium (adding event kind) | Medium (incorrect data) | IS-C2, MT-C8 |
| innerHTML escaping regression | Very Low (property-tested) | Critical (XSS) | SE-M3 |

---

## Success Metrics and Follow-Up

### Quality Gates
- [ ] All HIGH concern fixes merged (target: 10/12)
- [ ] Cross-target parity test passes for slot projection (React + Flutter)
- [ ] `cargo test` — all 830+ tests passing, zero regressions
- [ ] `cargo clippy` — zero new warnings
- [ ] Snapshot tests updated for mechanically-changed output (push_str→write!, TypeScript gate)

### Follow-Up ADRs Needed
- ADR for `ComponentScope` extraction (InliningState fix)
- ADR for dead field removal (`generic_params`, `if_mode_default`) 
- ADR for TypeScript declaration gating (implementing ADR-031 D6)
- Update ADR-027 to document design-block CSS sanitization gap fix

### Next Review
Schedule component architecture re-review after 2 sprints to verify:
- InliningState structural fix effectiveness
- Cross-backend slot parity
- Flutter CSS mapping completeness
- Dead field cleanup

---

*Generated by 10-perspective architecture review. Per-specialist reviews preserved verbatim in the sections above. Cross-cutting analysis and prioritization by the orchestrator.*
