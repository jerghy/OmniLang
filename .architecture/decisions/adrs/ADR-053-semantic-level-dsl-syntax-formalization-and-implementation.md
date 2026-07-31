# ADR-053: Semantic-Level DSL Syntax Formalization & Implementation Architecture

## Status

Accepted

## Context

ADR-052 established OmniLang as a semantic-level DSL with three-tier compilation (primitives → IR → codegen) and a 24-item feature floor. However, the semantic-level design document (`docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md`) describes semantic intent-to-platform mappings in prose — it lacks precise, unambiguous grammar rules at the level of rigor needed for parser implementation. Meanwhile, the existing grammar documents (`14-grammar-lang.md`, `15-grammar-view.md`) reflect the pre-ADR-052 OmniView-first world.

The 2026-07-31 brainstorming session identified that formalizing the syntax is a prerequisite for implementation — the codegen cannot "understand intent" if the parser itself produces generic AST nodes without semantic recognition. Key gaps included:

1. **No grammar document** encoding the semantic-level BNF rules — parser implementors had no precise reference
2. **Ref operations parsed as generic MethodCall** — compiler didn't know `ref.focus()` meant "request focus" vs. "call method named focus"
3. **Animation keyframe properties** accepted arbitrary CSS names — no platform-independent semantic property set
4. **Design/variant/class usage forms** documented in prose but not in BNF
5. **Slot model** from inline view declarations needed restructuring for cross-target mapping

Additionally, the implementation plan needed to resolve: breaking change scope, phase ordering (parser changes must precede IR changes; design gate removal must coincide with codegen existence), and which existing features to retain vs. delete.

## Decision Drivers

1. **Parser must produce semantically-aware AST** — The parser should recognize semantic constructs (ref bindings, animation properties, slot type constraints) at parse time, producing specialized AST nodes that downstream passes can operate on without guesswork.
2. **BNF must be the single source of truth for syntax** — Prose descriptions in design docs are ambiguous; formal BNF rules eliminate implementation drift between parser authors and codegen authors.
3. **Breaking changes are acceptable where no usage exists** — Features like `ref.style.*`, `paused_override`, and `@paused` attribute have zero `.omni` usage outside docs. They can be removed without migration cost.
4. **Breaking changes with existing usage require explicit migration** — `computed` in components (10+ files) and animation syntax (8+ files) have heavy usage and need documented migration paths.
5. **Design gate must open simultaneously with codegen existence** — Removing the gate before codegen exists causes panics; the gate removal must be the SAME commit that adds the design codegen for that target.
6. **OmniView debug runtime stays consistent** — All three P0 targets (React, Flutter, OmniView) update together to maintain cross-target behavior consistency.

## Decision

**Create a formal syntax specification document** (`docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`) as the BNF authority for the semantic-level DSL, separate from the design document. The design document specifies *what* semantic intent maps to *which* platform code; the syntax spec specifies *how* that intent is expressed in precise BNF rules.

**Seven sub-decisions** embedded in the spec and implementation plan:

### D1: Separate syntax spec from design doc

| Document | Role | Audience |
|----------|------|----------|
| `semantic-level-dsl-design.md` | Semantic intent → platform code strategy | Codegen implementors |
| `semantic-level-dsl-syntax-spec.md` | Precise BNF grammar rules | Parser implementors + language users |

Each references the other. No duplication — the design doc references the syntax spec for grammar; the syntax spec references the design doc for platform mapping tables.

### D2: Mixed ref model — declarative bindings + semantic methods

| Frequency | Form | Syntax | Examples |
|-----------|------|--------|---------|
| High | Declarative signal binding | Element attributes | `focus={sig}`, `scroll-y={sig}`, `value={sig}` |
| Low | Semantic method (compiler-recognized) | `ref.method()` | `ref.select()`, `ref.measure()`, `ref.setSelection(0,5)` |

Rationale: High-frequency bidirectional operations (focus/blur, scroll position) map naturally to signal-driven reactive bindings. Low-frequency one-shot operations (select all text, measure element, capture pointer) fit better as imperative calls.

The parser recognizes semantic binding attribute names (`focus`, `scroll-x`, `scroll-y`, `value`) and produces `ElementProp::RefBinding` instead of generic `ElementProp::Static`. The semantic pass `validate_ref_ops` rewrites legal ref methods to `ExprIR::RefOpExpr` and emits compile errors for DOM-specific APIs (`ref.style.*`, `ref.innerHTML`, `ref.querySelector`, etc.) with migration hints.

**Deleted DOM APIs**: `ref.style.*`, `ref.innerHTML`, `ref.addEventListener`, `ref.querySelector`, `ref.classList`, `ref.parentElement`, `ref.textContent`, `ref.offsetWidth`/`offsetHeight`, `ref.getBoundingClientRect`. All had zero `.omni` usage outside `ElementRefDemo.omni` (which is migrated per the plan).

### D3: Animation property whitelist — 16 semantic properties

Keyframe properties are restricted to a platform-independent semantic set:

```
opacity, scale, scaleX, scaleY,
translateX, translateY, translateZ,
rotate, rotateX, rotateY, rotateZ,
skewX, skewY,
backgroundColor, color,
width, height
```

The parser rejects non-whitelist properties at parse time with a clear error message. CSS custom properties (`--*`) are allowed as passthrough for CSS-based targets only.

Rationale: Arbitrary CSS properties (blur, brightness, borderWidth, fontSize, etc.) have no cross-platform equivalents. A whitelist ensures that keyframe definitions are valid on all targets. Properties without Flutter equivalents (width/height, rotateX/Y/Z, skewX/Y, translateZ) emit target-specific diagnostics (warning for perf-impacting, error for unsupported).

### D4: Animation triggers — native platform refs, no DOM scanning

The React codegen replaces `document.querySelectorAll('[data-motion-*]')` + `MutationObserver` on `document.body` (the OmniView pattern) with direct React refs + `useEffect`/event handlers:

| Trigger | React Implementation |
|---------|---------------------|
| `@enter` | `useEffect(() => { ref.current?.animate(kf, opts) }, [])` |
| `@hover` | `onMouseEnter`/`onMouseLeave` + `element.animate()` |
| `@press` | `onPointerDown`/`onPointerUp` + `element.animate()` |
| `@focus` | `onFocus` + `element.animate()` |
| `@in-view` | Per-ref `IntersectionObserver` with `useEffect` cleanup |
| `@scroll` | Per-ref scroll event listener with progress scrubbing |
| `@drag-*` | Per-ref pointer event listeners with distance→progress mapping |

This eliminates per-component-instance MutationObservers on `document.body` (N instances = N× observers with `subtree: true` on the entire DOM).

### D5: Slot model — component body declarations

Slots move from inline view declarations to component body:

```omni
// OLD: inline in view
<div>
  @slot header { <DefaultHeader /> }
  @slot default
  {header}
  {children}
</div>

// NEW: component body + view
component Modal {
  @slot header { <DefaultHeader /> }
  @slot default

  <div>
    {header}
    {children}
  </div>
}
```

Consumer side changes from `slot="name"` attribute to named props:
```omni
// OLD: <Modal><div slot="header">Title</div>content</Modal>
// NEW: <Modal header={<div>Title</div>}>content</Modal>
```

Rationale: This maps cleanly to all three targets — React `{props.header ?? fallback}`, Flutter `widget.header ?? FallbackWidget()`, OmniView `childrenMap['header'] || fallbackHTML`.

### D6: Retain computed in components

Despite initially being considered for deletion in early spec drafts (mapped to "use `getter` or `fn` instead"), `computed` in components is retained. It has a clear semantic meaning ("derived reactive value with auto-tracked dependencies") that maps cleanly:

| Target | Mapping |
|--------|---------|
| React | `useMemo(() => expr, [deps])` |
| Flutter | Getter on State class |
| OmniView | Computed signal |

It is used in 10+ components across `learn-omniview` and deleting it would be a high-cost migration with no architectural benefit.

### D7: Delete truly unused features

| Feature | Usage | Action |
|---------|-------|--------|
| `paused_override` | Zero `.omni` usage (14 doc/code refs only) | Delete from IR, parser, codegen |
| `PlaybackIR.paused` | Zero `.omni` usage | Delete |
| `@paused={expr}` element attr | 1 usage in `KeyframeAnimationDemo.omni` | Delete; demo migrated to conditional rendering |
| React Hooks store mode | CLI flag exists; Zustand is the future | Delete; Zustand v5 only |
| `@leave` trigger | Already deleted (ADR-032) | No action |
| Inline store in component | 1 usage in `InlineStoreDemo.omni` | Deprecation warning, not deletion |

### Architectural Components Affected

- `crates/omnilang-core/src/lexer/keywords.rs` — trigger token renames
- `crates/omnilang-core/src/parser/` — new syntax forms (semantic bindings, motion body, slot constraint, @if mode, @for [scroll])
- `crates/omnilang-core/src/parser/ast/` — AST type changes (RefBinding, SlotDirective fields, motion types)
- `crates/omnilang-core/src/ir/types.rs` — RefOpIR enum, ViewTree::Slot struct, ElementIR.semantic_bindings
- `crates/omnilang-core/src/ir/types_motion.rs` — TriggerKind::Focus, deleted paused/paused_override, scroll_hint
- `crates/omnilang-core/src/ir/lower/` — Slot fallback preservation, inline motion lowering, semantic binding lowering
- `crates/omnilang-core/src/semantic/` — validate_ref_ops pass, semantic binding validation
- `crates/omnilang-core/src/codegen/shared/expr.rs` — OptionalChain/BlockExpr/Match/RefOpExpr formatting
- `omnilang/src/codegen/mod.rs` — Design gate removal (P0) + warning→error (non-P0)
- `omnilang/src/codegen/shared/design.rs` — Extracted CSS emitter
- `omnilang/src/codegen/react/` — Zustand v5, slots, refs, animation native refs, design CSS
- `omnilang/src/codegen/flutter/` — Riverpod 3.0, GlobalKey refs, TweenSequence, design ThemeData
- `omnilang/src/codegen/omniview/` — Semantic binding JS, slot fallback, @focus trigger, paused removal
- `test/projects/learn-omniview/` — 15+ file migration (animation, refs, slots, @if mode)
- `.architecture/deferrals.md` — Post-implementation sync

## Consequences

### Positive

- **Single source of truth for syntax**: BNF spec eliminates ambiguity between parser and codegen implementors. New language contributors reference one document for grammar rules.
- **Compiler understands intent at parse time**: Semantic bindings and ref methods are recognized by the parser (not guessed by codegen). Illegal operations rejected at compile time, not runtime.
- **React animation uses platform-native patterns**: No DOM scanning, no MutationObservers. Generated code passes `eslint-plugin-react-hooks` exhaustive-deps rule.
- **Flutter generated Dart compiles**: All 6 CRITICAL bugs fixed. `dart analyze` validation gate prevents regression.
- **Design blocks produce real output on all P0 targets**: No more zero-file gate. React gets CSS custom properties; Flutter gets ThemeData/ThemeExtension.
- **Backward compatibility preserved where usage exists**: `computed` retained, `variant:` call form retained, `class:` map form retained.
- **Zero silent failures**: Non-P0 design blocks → hard error (was warning + zero files).

### Negative

- **Breaking changes to animation syntax in 8+ demo files**: Motion body syntax (`over:`/`easing:` with colon), keyframe property whitelist, trigger attribute renames. Migration is mechanical but verbose.
- **Slot model restructure in 4 component files**: Moving slot declarations from view to component body changes component structure.
- **Implementation is large**: 35 tasks across 10 phases, ~6-8 person-weeks per ADR-052 estimate.
- **Design gate removal must be precisely sequenced**: Gate opens in the same commit as codegen for that target. Ordering error → panics or zero-file regression.

### Neutral

- **Two-document model for language specification**: Design doc + syntax spec. Cross-references prevent drift. Maintenance burden is modest (~200 lines of cross-reference links).
- **Naming convention changes**: `@inView`→`@in-view`, `@dragHorizontal`→`@drag-horizontal` (kebab-case for multi-word directives). Consistent with HTML/CSS conventions but different from current codebase.

## Implementation Strategy

### Blast Radius

**Impact Scope**: All three P0 codegen backends + IR types + semantic validation + parser + lexer + demo project + golden fixtures + CI configuration.

**Affected Components**: See "Architectural Components Affected" above. 30+ files created or significantly modified.

**Risk Mitigation**:
- Parameterized test matrix expanded to 5 fixture categories (design, slots, refs, animation, store) with cross-target compilation assertions
- CI gates: `tsc --noEmit` (React output) + `dart analyze` (Flutter output) prevent regression of uncompileable generated code
- `cargo test -p omnilang` (existing 550+ tests) must stay green throughout
- `cargo clippy` zero-warnings maintained

### Reversibility

**Reversibility Level**: Medium. All changes are in compiler code and demo project. No database migrations, no API contracts, no user-facing service changes. Individual phases can be reverted via `git revert`. However, cross-module IR type changes (`ViewTree::Slot`, `RefOpIR`, `ExprIR` variants) create wide match-arm update blast radius — reverting one requires reverting all.

**Rollback Feasibility**: Per-phase revert is viable. Full rollback requires reverting ~10 commits (one per phase).

**Commitments Made**: Zustand v5, Riverpod 3.0 (library version lock-in). Deletion of Hooks mode, paused_override, @paused attribute (permanent — no backward compat shim).

**Options Preserved**: `@native` escape hatch for platform-specific code (not implemented in this phase, but syntax reserved). Future cross-target features follow the same semantic-layer approach.

### Sequencing & Timing

**Prerequisites**:
- [x] ADR-052 (Semantic-Level DSL Cross-Target Codegen) — Accepted
- [x] 5-expert cross-target feature gap review (2026-07-31)
- [x] Semantic-level DSL design document
- [x] Semantic-level DSL syntax specification
- [x] Implementation plan (35 tasks, 10 phases)

**Implementation Order**:
```
Phase 0:  Lexer/Parser  (4 tasks) — new syntax forms parse
Phase 1:  IR Foundation (3 tasks) — RefOpIR, Slot fix, motion changes
Phase 2:  Semantic      (3 tasks) — intent recognition passes
Phase 3:  Shared Codegen (2 tasks) — design CSS extract, expr fixes
Phase 4:  React Codegen  (5 tasks) — Zustand v5, slots, refs, animation, design
Phase 5:  Flutter Codegen (5 tasks) — Riverpod 3.0, refs, slots, TweenSequence, design  [∥ Phase 4]
Phase 6:  OmniView       (1 task) — debug runtime update
Phase 7:  Non-P0 Targets (1 task) — IR fixes + design block error
Phase 8:  CI + Tests     (2 tasks) — matrix + tsc/dart gates
Phase 9:  Demo Migration (5 tasks) — 15+ files
Phase 10: Governance     (1 task) — ADR-020, ADR-021, deferrals
```

**Critical path**: 17 sequential tasks from Phase 0 through Phase 10. Phases 4 & 5 are parallelizable (~40% wall-clock reduction).

## Alternatives Considered

### Alternative A: No separate syntax spec — embed BNF in design doc

Keep all grammar rules within the design document as inline BNF blocks.

**Pros**: Single document to maintain. No cross-reference drift risk.

**Cons**: Design doc is already 1,200+ lines of prose + tables. Adding ~500 lines of BNF would make it unwieldy. Different audiences (codegen implementors vs. parser implementors) need different entry points. Design doc describes strategy; syntax spec prescribes rules. **Rejected**: Separation of concerns justifies two documents.

### Alternative B: Pure declarative bindings for ALL ref operations

No `ref.select()` method calls — everything is a signal-driven binding. `ref.select()` becomes a `selectText={sig}` binding on the element.

**Pros**: Unified mental model. No imperative escape hatches. Simpler codegen (one pattern).

**Cons**: One-shot operations (select all, set selection range, measure element) are awkward as signal bindings. `measure()` needs a return value — not naturally expressed as a binding. `capturePointer`/`releasePointer` with pointer IDs are inherently imperative. **Rejected**: Mixed model (D2) provides the right granularity.

### Alternative C: Keep all CSS properties in keyframes — diagnostic at codegen

Accept any CSS property in keyframes, emit diagnostics at codegen time for properties without cross-platform equivalents.

**Pros**: No parser changes needed. Existing demo files work unchanged.

**Cons**: Parser can't help the user — error messages come late (codegen phase) with less context. Invalid properties survive parsing, accumulate in AST, and confuse downstream passes. Authoring experience suffers (write animation, compile, get "property not supported" — fix, repeat). **Rejected**: Parser-level whitelist gives immediate feedback.

### Alternative D: Keep paused_override and @paused with deprecation warning

Instead of deleting, emit deprecation warnings for a transition period.

**Pros**: Migration path for the one usage in `KeyframeAnimationDemo.omni`.

**Cons**: Maintaining dead code paths across all three codegen backends. `paused_override` has zero `.omni` usage (14 doc/code refs only). The one `@paused={isPaused}` usage is trivially replaced with conditional rendering. **Rejected**: Delete unused code; don't maintain deprecation shims for features nobody uses.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: This ADR documents decisions already made during the brainstorming→spec→plan process. It does not introduce new complexity — it formalizes the architecture of work that has already been designed and planned. The seven sub-decisions (D1-D7) collectively reduce complexity by: (a) deleting unused features, (b) establishing a single BNF authority, (c) unifying ref/animation models across targets.

**Necessity Assessment**: 9/10
- **Current need**: ADR-052 established semantic-level DSL as the architecture. This ADR documents HOW to implement it. Without these decisions, the implementation plan has no architectural foundation.
- **Future need**: Every future language feature addition will reference the syntax spec as the BNF authority.
- **Cost of waiting**: Implementation proceeds without documented decisions → architectural drift → rework. The 35-task plan would need to re-invent these decisions per-task.
- **Evidence of need**: The initial implementation plan (v1) had 12 identified gaps (missing parser tasks, wrong sequencing, missing OmniView updates). This ADR's decisions directly address those gaps.

**Complexity Assessment**: 3/10
- **Added complexity**: Two new documents (syntax spec + implementation plan). Both replace/consolidate information already scattered across design doc, grammar docs, and review findings. Net reduction in scattered information.
- **Maintenance burden**: Syntax spec needs updates when language syntax changes. Design doc references syntax spec for grammar, so one update propagates.
- **Learning curve**: New contributors need to understand the two-document model (design doc for strategy, syntax spec for grammar). This is documented in both files' headers.
- **Dependencies introduced**: None. No new libraries, frameworks, or tools.

**Alternative Analysis**: Alternative A (one doc) was considered and rejected because the audiences and content types are genuinely different. The two-document model is not over-engineering — it's appropriate separation of concerns for a compiler project.

**Simpler Alternative Proposal**: None. These decisions represent the minimal architecture needed to implement ADR-052. Deferring any of them would leave implementation gaps.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 3/10
- **Ratio**: 0.33 ✅ (Target: <1.5)

**Overall Assessment**: These decisions do not add complexity — they organize existing information and delete unused features. The two-document model (spec + plan) is appropriate engineering for a multi-target compiler project. D7 (deleting truly unused features) actively reduces codebase size and maintenance burden.

## Validation

**Acceptance Criteria:**
- [ ] Syntax spec committed to `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`
- [ ] Implementation plan committed to `docs/superpowers/plans/2026-07-31-semantic-level-dsl-implementation.md`
- [ ] All 35 implementation tasks have no deferred work, no TODOs, no placeholders
- [ ] Breaking change analysis complete — all `.omni` usages cataloged with migration paths
- [ ] Design gate removal sequenced within codegen tasks (not before)
- [ ] OmniView debug runtime included in update scope
- [ ] Non-P0 targets updated for IR changes + design block error
- [ ] CI gates (tsc + dart analyze) specified in plan
- [ ] `cargo test -p omnilang` remains green (550+ tests) throughout implementation
- [ ] ADR-020 and ADR-021 updated post-implementation

## References

- [ADR-052 — Semantic-Level DSL Cross-Target Codegen](ADR-052-semantic-level-dsl-cross-target-codegen.md)
- [Semantic-Level DSL Syntax Specification](../../docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md)
- [Semantic-Level DSL Implementation Plan](../../docs/superpowers/plans/2026-07-31-semantic-level-dsl-implementation.md)
- [Semantic-Level DSL Design Document](../../docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md)
- [Cross-Target Feature Gap Review](../../.architecture/reviews/cross-target-feature-gap-review-2026-07-31.md)
- [ADR-020 — Flutter State Management Riverpod](ADR-020-flutter-state-management-riverpod.md)
- [ADR-021 — Cross-Target Feature Parity](ADR-021-cross-target-feature-parity.md)
- [ADR-028 — OmniView Debug Runtime Design Tradeoffs](ADR-028-omniview-debug-runtime-design-tradeoffs.md)
- [ADR-032 — Animation Design Scoping](ADR-032-omniview-animation-design-scoping.md)
- [ADR-036 — Element Reference System](ADR-036-element-reference-system.md)
- [OmniLang Grammar — Core Language](../../docs/omniview/14-grammar-lang.md)
- [OmniLang Grammar — OmniView UI](../../docs/omniview/15-grammar-view.md)
