# ADR-036: Element Reference System for High-Frequency DOM Manipulation

**Date**: 2026-07-29
**Author**: Architecture review + implementation

---

## Status

Superseded by [ADR-052](ADR-052-semantic-level-dsl-cross-target-codegen.md) / [ADR-053](ADR-053-semantic-level-dsl-syntax-formalization-and-implementation.md)（2026-08-01，见 [ADR-054](ADR-054-omniview-semantic-dsl-era-design-philosophy.md)）

> 旧 ref 机制（`ref.style.*`/`getBoundingClientRect`/`offsetWidth` 等 DOM 形状 API）被 RefOpIR 11 语义 ops + RefBinding（focus/scroll-x/y/value）+ `style:{...}` 绑定取代；非法 API 现在产生带迁移提示的编译错误。原文保留作历史参考。

## Context

OmniView needs to support high-frequency element operations — drag, mousemove-driven updates, resize handles, scroll-linked effects — where per-frame DOM updates at 60fps are required. The existing signal reactivity system is inappropriate for this: each `signal.set()` triggers synchronous subscriber notification (ADR-028 D3), and in the React codegen, every state change triggers full reconciliation via `useState`.

The architectural review of OmniView's high-frequency element manipulation (`.architecture/reviews/component-omniview-high-frequency-element-manipulation.md`) found that:

1. **OmniView debug runtime** already has `__omni_el(nid)` — a cached DOM element lookup — used pervasively for direct manipulation (`el.textContent = ...`, `el.classList.toggle(...)`) within reactive effects, bypassing HTML re-rendering.
2. **React production codegen** had zero equivalent mechanism — no `useRef`, no element caching, no direct DOM access path. All state flowed through `useState` → full reconciliation.
3. **No language-level mechanism** existed for developers to express "I need a stable reference to this DOM element."
4. **The animation system** (`@drag-horizontal`/`@drag-vertical` triggers) was hardcoded to WAAPI animation scrubbing — it couldn't be used for general element movement, drag-to-reorder, or custom interactions.

## Decision Drivers

- **Performance**: 60fps drag/mousemove operations cannot go through signal → reconciliation for every frame
- **Expressiveness**: Developers need a way to directly manipulate elements at high frequency, updating signal/state only at the end
- **Cross-backend consistency**: The primitive must map cleanly to React (`useRef`), Flutter (`GlobalKey`), and OmniView (`__omni_el`)
- **Safety**: The API must not expose dangerous capabilities (`innerHTML`, `addEventListener`, `querySelector`)
- **Simplicity**: Follow the well-established React pattern: direct DOM during the high-frequency phase, state update at the end

## Decision

**Add an Element Reference System to OmniLang at the language, IR, and codegen levels.**

### Language Syntax

```omni
component Draggable {
  signal x: Float = 0
  signal y: Float = 0
  ref box: Element              ← NEW: element reference declaration

  fn onDrag(e) {
    box.style.transform = "translate({e.clientX}px, {e.clientY}px)"  ← direct DOM
  }
  fn endDrag(e) {
    x = e.clientX               ← signal update only at the end
    y = e.clientY
  }

  <div @ref={box}               ← NEW: bind ref to element
       @onPointerMove={onDrag}
       @onPointerUp={endDrag}
       style:transform="translate({x}px, {y}px)">
    Drag me
  </div>
}
```

### IR Design

```rust
// ElementIR gains:
pub ref_binding: Option<String>,  // ref name from @ref={name}

// ComponentIR already had:
pub refs: Vec<RefDeclIR>,         // from `ref name: Element` declarations
```

### Codegen Per Backend

| Backend | Mechanism | Example Output |
|---------|-----------|----------------|
| **OmniView** | `__omni_el(nid)` IIFE wrapper with lazy getters | `const box = (function() { const _el = () => __omni_el("el-div-001"); return { get style() { ... }, focus() { ... }, ... }; })();` |
| **React** | `useRef<HTMLDivElement>(null)` + `ref={}` JSX + `nameRef.current` expression translation | `const boxRef = useRef<HTMLDivElement>(null);` → `<div ref={boxRef}>` → `boxRef.current.style.transform = ...` |
| **Flutter** | `GlobalKey` (Phase 2) | `final boxKey = GlobalKey();` |

### Ref API Surface (cross-backend safe subset)

| API | Purpose |
|-----|---------|
| `.style` | CSS property access for direct manipulation |
| `.getBoundingClientRect()` | Position/size queries |
| `.capturePointer(id)` / `.releasePointer(id)` | Pointer capture for drag |
| `.focus()` / `.blur()` | Focus management |
| `.scrollTo(x, y)` / `.scrollBy(dx, dy)` | Scroll control |
| `.offsetWidth` / `.offsetHeight` | Read-only dimension access |

**Not exposed**: `innerHTML`, `addEventListener`, `querySelector`, `parentElement` — these would break the cross-backend abstraction and create XSS/event-leak risks.

## Consequences

### Positive

1. **High-frequency operations are now a first-class language concept** — developers don't need to understand codegen internals to write performant drag/resize/scroll interactions
2. **Cross-backend portability** — the same `.omni` source compiles to efficient code on all targets
3. **Zero production overhead for non-ref components** — `ref_binding` defaults to `None`, all existing code paths unchanged
4. **Parser infrastructure was already ~80% complete** — `ref name: Element` and `ref: name` syntax already parsed correctly; only IR lowering and codegen were missing
5. **Safety by design** — the restricted API surface prevents common footguns (XSS via ref, event listener leaks, DOM traversal)

### Negative

1. **React codegen must now emit `useRef`** — adds a new hook path that didn't exist before (6 lines of hook selection + ~40 caller API updates for `format_expr_ir_tsx`)
2. **`@for` + `@ref` semantics unresolved** — what does `@ref={box}` mean inside a `@for` loop? Deferred to Phase 2
3. **Custom component ref forwarding deferred** — `@ref` on `<MyComponent>` is not yet supported (requires React `forwardRef` or equivalent)
4. **Expression translation threading incomplete** — `ref_names` parameter added to `format_expr_ir_tsx` but not yet threaded through `gen_view_node_tsx` or `generate_statement_tsx_with_mode` (all callers pass `&FxHashSet::default()`)
5. **Type parameter hardcoded** — `useRef<HTMLDivElement>(null)` uses `HTMLDivElement` for all elements; should be parameterized when IR gains element-type tracking

## Implementation

**Phase 1** (this ADR — 11 commits, `0f430b04..3233b795`):

| Task | Layer | What |
|------|-------|------|
| IR | `ElementIR.ref_binding` field + lowering fix | 32 files |
| Semantic | `validate_refs.rs` — bind/declare/duplicate checks | 334 lines, 7 tests |
| OmniView | `__omni_el` IIFE wrapper objects | 3 files |
| React | `useRef` import → declaration → JSX `ref={}` → expression translation | 7 files |
| Tests | 6 integration tests (3 OmniView + 3 React) | 2 files |

**Phase 2** (deferred):

- Thread `ref_names` through `gen_view_node_tsx` for inline handler expression translation
- Thread `ref_names` through `generate_statement_tsx_with_mode` for function body expression translation
- Flutter `GlobalKey` codegen
- `@for` + `@ref` semantics
- Custom component ref forwarding (`forwardRef`)

## Alternatives Considered

### A. High-Frequency Signal Optimization

Mark certain signals as `@highFreq` — the compiler automatically generates direct DOM writes instead of reconciliation for these signals.

**Rejected**: More declarative but less flexible. Can only update properties that are already bound to signals. Cannot support arbitrary DOM operations like `capturePointer` or `getBoundingClientRect`. The fundamental problem is that signals are a *state* abstraction, but drag/mousemove are *imperative* interactions.

### B. `@dom` Escape Hatch Block

Add a `@dom { ... }` block syntax where developers write direct DOM manipulation code that the compiler recognizes and bypasses the signal system.

**Rejected**: Ties the language to DOM concepts, making Flutter codegen awkward. Also creates a two-tier language where some code is "real OmniLang" and some is "escape hatch JS." The `ref` approach is cleaner: the ref is a typed object with a defined API surface, not raw code.

### C. Extend `@drag-*` Triggers to Support Element Movement

Generalize the existing `@drag-horizontal`/`@drag-vertical` animation triggers to support direct element position manipulation.

**Rejected**: Couples drag to animation scrubbing. Does not generalize to other high-frequency scenarios (resize handles, scroll-linked position updates, custom gesture recognizers). The `ref` approach is more general.

### D. Do Nothing (Status Quo)

Rely on the existing `onMouseMove` event + signal system for all element manipulation. Accept the performance cost.

**Rejected**: Would make OmniView unusable for any application requiring drag, resize, or smooth scroll-linked interactions. The debug runtime already has the `__omni_el()` pattern for internal use — the gap is that developers can't access it from `.omni` source.

## Validation

- All 837+ workspace tests pass (zero regressions)
- Clippy clean, rustfmt clean
- 6 new integration tests covering basic ref generation, multiple refs, `@for` compatibility, React `useRef` import/declaration/JSX attribute, and negative case (no `useRef` when no refs)
- Full pipeline verified: `.omni` source → parse → IR → codegen → correct JS/TSX output

## Pragmatic Enforcer Analysis

| Assessment | Score | Notes |
|------------|-------|-------|
| **Necessity** | 8/10 | Real consumer: drag/mousemove-driven interactions are required for any interactive UI framework. Cost of waiting: OmniView cannot support basic UX patterns (drag, resize). |
| **Complexity** | 5/10 | Adds one IR field, one AST variant (already parsed), ~200 lines of codegen across two backends. Phase 1 is appropriately scoped — ref forwarding, Flutter, and expression threading are deferred. |
| **Ratio** | 0.63 | Well below the 1.5 threshold. The feature is necessary and the implementation is appropriately minimal. |

### Recommendation

**Approve.** The Phase 1 implementation is minimal and focused: one IR field, one semantic validation module, two codegen backends. Deferred items (expression threading, Flutter, `@for` semantics, component ref forwarding) are explicitly scoped to Phase 2. The parser was already 80% complete before this ADR — the implementation connected existing infrastructure rather than building from scratch.

## References

- Architecture review: `.architecture/reviews/component-omniview-high-frequency-element-manipulation.md`
- Design document: `docs/design/element-ref-system.md`
- Implementation plan: `docs/superpowers/plans/2026-07-29-element-ref-system.md`
- Related ADRs: ADR-028 (OmniView debug runtime trade-offs), ADR-034 (ComponentScope inlining isolation), ADR-021 (cross-target feature parity)
