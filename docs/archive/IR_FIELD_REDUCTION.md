# OmniView IR Pre-Embedded Field Reduction

> **Date:** 2026-06-05
> **Source:** Council review recommendation (2026-06-04)
> **Action:** Reduce 27 pre-embedded `Optional=None` fields → 8 essential fields

---

## Current State Analysis

**Key finding:** No IR code exists yet in the Python codebase (`omnilang/`). All 27 pre-embedded fields exist only in the design document (`OmniLang_v0.3_设计方案.md` §十 lines 1322-1352). The existing `omnilang/ast_nodes.py` contains **parser AST nodes** (e.g., `Literal`, `BinaryOp`, `FuncCall` using `@dataclass(frozen=True)`), not IR types.

This means the field reduction is implemented **at IR creation time** -- the new IR module starts with only 8 fields. No existing code needs to be modified.

---

## The 8 Retained Fields

These 8 fields each drive a **structural codegen branch** where output differs in kind, not in value:

| # | Field | IR Type | Codegen Branch |
|---|-------|---------|---------------|
| 1 | `construction_kind` | `ElementIR` | Flutter widget constructor selection (`Container` vs `Row` vs `ListView.builder`) |
| 2 | `output_mode` | `StyleRuleIR` | CSS-modules (React/Vue) vs inline-style (Dioxus) vs widget-properties (Flutter) |
| 3 | `update_semantics` | `SignalIR` | `setCount()` / `count.value =` (setter-call) vs `setState(() => ...)` (Flutter) |
| 4 | `accessor_style` | `SignalIR` | `count()` (Dioxus fn) vs `count.value` (Vue ref) vs `count` (React closure) |
| 5 | `is_copy` | `SignalIR` | Rust `Copy` types move directly vs non-Copy types need `.clone()` in Dioxus |
| 6 | `needs_clone` | `TypeIR` | `#[derive(Clone)]` for Dioxus struct types used as signal values |
| 7 | `needs_partial_eq` | `TypeIR` | `#[derive(PartialEq)]`, `React.memo`, `==` override -- key comparison across all frameworks |
| 8 | `execution_mode` | `BlockIR` | SSR safety boundary -- `if (typeof window !== 'undefined')` guards |

### Detailed Rationale for Each Retained Field

**1. `construction_kind` (ElementIR)**
Flutter requires fundamentally different widget constructors for different use cases. The same `<Container>` element might map to `Container()` for basic layout, `Padding()` for spacing-only, or `DecoratedBox()` for visual styling. This is not a parameter difference -- it's a type-level decision. All other frameworks render `<Container>` as a `<div>`.

**2. `output_mode` (StyleRuleIR)**
Three fundamentally different output channels: CSS file generation (React CSS Modules, Vue scoped), inline style string generation (Dioxus), and Widget property generation (Flutter). The codegen pipeline structure differs entirely depending on this field.

**3. `update_semantics` (SignalIR)**
The difference between "call a setter function" (React `setCount(5)`, Vue `count.value = 5`, Dioxus `count.set(5)`) and "wrap in setState callback" (Flutter `setState(() => _count = 5)`) changes the structure of the generated code, not just the function name.

**4. `accessor_style` (SignalIR)**
Reading a signal value uses different syntax across frameworks: React reads plain closure variable `count`, Vue reads `.value` property `count.value`, Dioxus calls a getter function `count()`. The accessor style determines how signal references are compiled in expressions.

**5-6. `is_copy` / `needs_clone` (SignalIR / TypeIR)**
Rust's ownership model requires explicit `.clone()` calls for non-Copy types when values are used in closures or rsx! blocks. This is a compile-time static analysis that affects generated code structure (insertion of `.clone()` or `.cloned()` calls). For TypeScript/Dart targets, these fields are ignored but retained for Dioxus codegen.

**7. `needs_partial_eq` (TypeIR)**
Key comparison across all frameworks: React needs `key` prop uniqueness, Vue uses `:key`, Dioxus uses `PartialEq` for diffing, Flutter uses `ValueKey` with `==`. This field drives `#[derive(PartialEq)]` generation and memo comparison logic.

**8. `execution_mode` (BlockIR)**
Marks code blocks as `"client-only"` (default) vs `"server-safe"` (future SSR). This is a safety boundary -- codegen can insert guard conditions for client-only code in SSR contexts. While the SSR target is P5+, the field is retained because it affects code block structure (not just a decoration).

---

## The 19 Fields to Remove

These fields become comments in the dataclass definition, not runtime fields:

### ElementIR fields (6 removed)

| Field | Phase for Re-addition | Comment Text |
|-------|----------------------|-------------|
| `key_expr` | P0 (not pre-embedded; structural) | `# reserved for P0 -- key_expr moved to ListRenderingIntent.key_expr (structural, not pre-embedded)` |
| `accessibility_label` | P1+ | `# reserved for P1+ -- accessibility_label: aria-label / Semantics.label` |
| `accessibility_role` | P1+ | `# reserved for P1+ -- accessibility_role: ARIA role / SemanticsRole` |
| `keep_alive` | P3 | `# reserved for P3 -- keep_alive: KeepAlive component cache flag` |
| `portal_target` | P3 | `# reserved for P3 -- portal_target: Portal/Teleport mount target selector` |
| `validation_rules` | P3 | `# reserved for P3 -- validation_rules: form field validation rule chain` |
| `virtual_list_config` | P3 | `# reserved for P3 -- virtual_list_config: virtual scroll configuration` |

### SignalIR field (1 removed)

| Field | Phase | Comment Text |
|-------|-------|-------------|
| `accessor_style` | P2 | Already in retained list above for Dioxus codegen |

### EventIR field (1 removed)

| Field | Phase | Comment Text |
|-------|-------|-------------|
| `payload_kind` | P4 | `# reserved for P4 -- payload_kind: "dom-event" vs "flutter-value-event"` |

### StyleRuleIR field (1 removed)

| Field | Phase | Comment Text |
|-------|-------|-------------|
| `animation_def` | P3 | `# reserved for P3 -- animation_def: AnimationIR (keyframes, duration, easing)` |

### ComponentIR fields (6 removed)

| Field | Phase | Comment Text |
|-------|-------|-------------|
| `i18n_resources` | P2 | `# reserved for P2 -- i18n_resources: translation key-value pairs` |
| `error_handler` | P2 | `# reserved for P2 -- error_handler: @error error boundary handler block` |
| `generic_params` | P2 | `# reserved for P2 -- generic_params: generic type parameter list for components` |
| `is_pure` | P2 | `# reserved for P2 -- is_pure: React.memo / const Widget optimization flag` |
| `composables` | P3 | `# reserved for P3 -- composables: list[ComposableIR] for logic reuse` |
| `async_loader` | P3 | `# reserved for P3 -- async_loader: @load async data loading block` |
| `is_lazy` | P3 | `# reserved for P3 -- is_lazy: React.lazy / async defineComponent flag` |
| `form_state` | P3 | `# reserved for P3 -- form_state: form validation state signal` |

### ThemeIR fields (2 removed)

| Field | Phase | Comment Text |
|-------|-------|-------------|
| `modes` | P3 | `# reserved for P3 -- modes: theme modes list (["light", "dark", ...])` |
| `breakpoints` | P3 | `# reserved for P3 -- breakpoints: responsive breakpoints (sm/md/lg/xl)` |

---

## Comment Format Specification

Each removed field becomes a comment in the IR dataclass definition:

```python
@dataclass
class ElementIR:
    """Abstract element node -- most numerous IR node type."""
    tag: str
    props: dict[str, ExprIR]
    bindings: dict[str, SignalRefIR]
    events: dict[str, EventHandlerIR]
    style_classes: Optional[StyleIntent] = None
    inline_style: Optional[LayoutIntent] = None
    children: list[ViewTree] = field(default_factory=list)
    test_id: Optional[str] = None
    
    # ── Phase 0 essential fields ──
    construction_kind: str = "tag-mapping"
    source_span: Optional[SourceSpan] = None
    
    # ── Reserved for future Phases ──
    # reserved for P1+ -- accessibility_label: ExprIR → aria-label / Semantics.label
    # reserved for P1+ -- accessibility_role: str → ARIA role / SemanticsRole
    # reserved for P3 -- keep_alive: bool → KeepAlive component cache (default False)
    # reserved for P3 -- portal_target: str → Portal/Teleport mount target selector
    # reserved for P3 -- validation_rules: list → form field validation rule chain
    # reserved for P3 -- virtual_list_config: VirtualListConfig → virtual scroll params
```

Key rules:
1. Comments use `# reserved for P<N> -- field_name: type → description`
2. The `→` arrow connects type info to the description
3. Comments go AFTER the active fields, in a clearly marked section
4. Do NOT create `Optional[X] = None` fields for reserved items

---

## Field Re-addition Governance

### The Rule

> **Fields are added as independent PRs at Phase start, with full JSON Schema update and at least 1 construction example. No pre-embedding for future Phases.**

### PR Requirements

Each field addition PR must:
1. Be opened at the START of the Phase that consumes the field (not before)
2. Include the field definition with correct type (not `Optional[Any]` placeholder)
3. Include codegen in at least one backend that reads and branches on the field
4. Update the IR JSON Schema
5. Include at least one `.omni` construction fixture containing the field
6. Include tests per affected backend
7. Be a single-responsibility PR (field + consumer, never field-only)

### Re-addition Schedule

| Phase | Fields to Add | Approximate PRs |
|-------|--------------|----------------|
| P0 | 0 (no pre-embedding needed) | 0 |
| P1 | 2 (`accessibility_label`, `accessibility_role`) | 2 |
| P2 | 4 (`i18n_resources`, `error_handler`, `generic_params`, `is_pure`) | 4 |
| P3 | 10 (`composables`, `async_loader`, `is_lazy`, `form_state`, `keep_alive`, `portal_target`, `validation_rules`, `virtual_list_config`, `modes`, `breakpoints`, `animation_def`) | ~10 |
| P4 | 1 (`payload_kind`) | 1 |
| **Total** | **19 fields** | **~17 PRs** |

Each PR is small, focused, and adds both the field and its consumer code. This is intentionally MORE PRs than a bulk add -- it enforces the discipline of adding only what's needed.

---

## Design Document Reconciliation

The field table in `OmniLang_v0.3_设计方案.md` §十 (lines 1322-1352) lists 27 fields. Some have discrepancies with the IR code block definitions in §10.1a-10.1c:

| Field | Table Owner | Code Block Status | Resolution |
|-------|------------|-------------------|------------|
| `key_expr` | ElementIR | In ListRenderingIntent code block | No discrepancy -- `key_expr` is on `ListRenderingIntent`, table lists it under ElementIR for documentation. Keep on ListRenderingIntent. |
| `validation_rules` | ElementIR | Not in ElementIR code block (10.1c) | Add when P3 starts |
| `virtual_list_config` | ElementIR | Not in ElementIR code block (10.1c) | Add when P3 starts |
| `animation_def` | StylePropertyIR (table) vs StyleRuleIR (code §10.1c) | Inconsistency | Standardize to `StyleRuleIR` |
| `payload_kind` | EventIR | EventIR has no Phase 0 code definition | Add when P4 starts |
| `modes`, `breakpoints` | ThemeIR | ThemeIR has no Phase 0 code definition | Add when P3 starts |
| ComponentIR fields (7) | ComponentIR | ComponentIR has no Phase 0 code definition | Add when respective Phase starts |

**Action:** These discrepancies should be noted in the design document's next revision but do not affect Phase 0 implementation.

---

## Summary

| Metric | Before | After |
|--------|--------|-------|
| Pre-embedded fields | 27 | 8 |
| Removed fields (→ comments) | — | 19 |
| Risk of YAGNI violation | High (27 Optional fields) | Low (only structural fields) |
| IR dataclass clarity | Cluttered | Clean |
| Addition cost | Free (already there) | ~17 focused PRs across P1-P4 |

**Bottom line:** The 8 retained fields are the minimum set needed for the IR to express structural codegen differences. Every other field can be added when its consuming Phase starts, with the full context of how it will be used.
