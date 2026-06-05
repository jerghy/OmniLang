# OmniView IR Prototype Validation Report

> **Project:** OmniLang v0.3 -- OmniView
> **Status:** FINAL
> **Date:** 2026-06-05
> **IR Version:** `0.3`
> **Purpose:** Validate Intent IR against 4 "canary cases" -- the hardest cross-framework mapping problems -- before freezing the IR design and proceeding to Phase 0 implementation.

---

## Case A: ListView.builder -- Dynamic List Rendering

### OmniView Source

```omni
@for item in items {
    <ListItem key={item.id}>{item.name}</ListItem>
}
@empty {
    <Text>No items found</Text>
}
```

### Intent IR Structure

```python
@dataclass
class ListRenderingIntent:
    """List rendering intent -- all @for in Flutter defaults to ListView.builder"""
    iterable: ExprIR                # IdentifierExpr("items")
    item_binding: BindingIR         # BindingIR(name="item", type=TypeIR(...))
    body: ViewTree                  # ElementIR(tag="ListItem", ...)
    key_expr: ExprIR                # FieldAccessExpr(IdentifierExpr("item"), "id")
    empty_body: Optional[ViewTree]  # ViewTree with ElementIR(tag="Text", ...)
    # NO strategy field -- per design:
    #   - React/Vue/Dioxus: default .map() / v-for / .iter().map()
    #   - Flutter: always ListView.builder (safe default, no performance trap)
    #   - Static list optimization: use <StaticList> element explicitly
```

### Target Framework Expansion

**React (L2 Idiomatic):**

```tsx
{items.length === 0 ? (
    <span>No items found</span>
) : (
    items.map(item => (
        <ListItem key={item.id}>{item.name}</ListItem>
    ))
)}
```

- `@for` expands to `.map()` with `key` on the root element
- `@empty` expands to a ternary length check before the map
- `key={item.id}` maps directly to React's reconciliation key

**Vue 3 (L2 Idiomatic):**

```html
<template v-if="items.length === 0">
    <span>No items found</span>
</template>
<template v-else>
    <ListItem v-for="item in items" :key="item.id">
        {{ item.name }}
    </ListItem>
</template>
```

- `@for` expands to `v-for` directive
- `@empty` expands to `v-if`/`v-else` block on a `<template>` wrapper
- Uses `<template>` for fragment-level control flow (Vue idiom)

**Dioxus (L2 Idiomatic):**

```rust
if items.is_empty() {
    rsx! { span { "No items found" } }
} else {
    rsx! {
        items.iter().map(|item| rsx! {
            ListItem {
                key: "{item.id}",
                item.name.clone()
            }
        })
    }
}
```

- Uses `if/else` for the empty branch (not ternary -- Rust idiom)
- `.iter().map()` with `|item|` closure -- standard Rust iterator pattern
- `key` as interpolated string `"{item.id}"` -- Dioxus key convention
- `.clone()` on `item.name` required by Rust ownership model

**Flutter (L2 Idiomatic, using ListView.builder):**

```dart
items.isEmpty
    ? Text('No items found')
    : ListView.builder(
          itemCount: items.length,
          itemBuilder: (context, index) {
              final item = items[index];
              return ListTile(
                  key: ValueKey(item.id),
                  title: Text(item.name),
              );
          },
      )
```

- `ListView.builder` for virtualized dynamic list -- the idiomatic Flutter choice
- `itemCount` + `itemBuilder` pattern (not `Column(children: ...)`)
- `ValueKey(item.id)` for widget identity
- Empty branch rendered via ternary

### Key Question Analysis

> **Does the IR need a `strategy` field (Dynamic vs Static), or is the default behavior (Flutter always ListView.builder) sufficient?**

**Design decision: NO strategy field needed.**

1. `ListView.builder` imposes negligible overhead for small lists -- the "optimization" to `Column` is premature.
2. The real risk is the opposite: a developer writes `@for` expecting virtualized behavior and gets a `Column` that renders 10,000 items, freezing the UI.
3. Defaulting to `ListView.builder` is **safe by default** -- it never causes a runtime performance trap.
4. Explicit optimization path exists: `<StaticList>` element for developers who need `Column` behavior.
5. A `strategy` field would add complexity to ALL codegens just to solve a Flutter-specific optimization that has a clean escape hatch.

### Conclusion: GO

All four targets produce idiomatic L2 code from a single IR structure. The no-strategy decision is sound.

| Framework | Idiomatic Level | Notes |
|-----------|----------------|-------|
| React | L2 | `.map()` + ternary empty guard |
| Vue 3 | L2 | `v-for` + `v-if`/`v-else` on `<template>` |
| Dioxus | L2 | `.iter().map()` + `if/else` in `rsx!` |
| Flutter | L2 | `ListView.builder` + ternary |

---

## Case B: TextEditingController Lifecycle -- Bidirectional Binding

### OmniView Source

```omni
<Input bind:value={email} placeholder="Enter email" />
```

### Intent IR Structure

```python
@dataclass
class BindIntent:
    """Bidirectional data binding intent"""
    source: SignalRefIR             # SignalRefIR(owner=None, name="email")
    element_type: BindableElement   # BindableElement.TEXT_INPUT
    direction: BindDirection        # BindDirection.BIDIRECTIONAL

class BindableElement(Enum):
    TEXT_INPUT = "text_input"
    TEXT_AREA = "text_area"
    CHECKBOX = "checkbox"

class BindDirection(Enum):
    BIDIRECTIONAL = "bidirectional"
```

### Target Framework Expansion

**React (L2 Idiomatic):**

```tsx
<input
    value={email}
    onChange={e => setEmail(e.target.value)}
    placeholder="Enter email"
/>
```

- Two lines, no lifecycle management required
- `value={email}` -- controlled component pattern

**Vue 3 (L2 Idiomatic):**

```html
<input v-model="email" placeholder="Enter email" />
```

- Single `v-model` directive -- the most concise of all four targets
- Vue's reactivity system handles the bidirectional sync internally

**Dioxus (L2 Idiomatic):**

```rust
input {
    value: "{email}",
    oninput: move |e| email.set(e.value()),
    placeholder: "Enter email",
}
```

- `value` bound via string interpolation `"{email}"`
- `move` closure captures `email` signal by value

**Flutter (L2 Idiomatic) -- 5-Step Lifecycle:**

```dart
class _EmailWidgetState extends State<EmailWidget> {
    late TextEditingController _emailController;

    @override
    void initState() {
        super.initState();
        // Step 1: Create controller with initial value
        _emailController = TextEditingController(text: widget.email);
        // Step 2: Add listener for signal <- controller direction
        _emailController.addListener(() {
            widget.onEmailChanged(_emailController.text);
        });
    }

    @override
    void didUpdateWidget(EmailWidget oldWidget) {
        super.didUpdateWidget(oldWidget);
        // Step 3: Sync controller when external signal changes
        if (widget.email != oldWidget.email) {
            _emailController.text = widget.email;
        }
    }

    @override
    void dispose() {
        // Step 4: Release controller resources
        _emailController.dispose();
        super.dispose();
    }

    @override
    Widget build(BuildContext context) {
        return TextField(
            controller: _emailController,
            decoration: InputDecoration(
                hintText: 'Enter email',
            ),
        );
    }
}
```

The 5-step lifecycle:
1. `initState`: Create `TextEditingController` with initial value
2. `initState` (continued): Add listener that writes to signal on change
3. `didUpdateWidget`: Sync controller text when external signal changes
4. `dispose`: Call `controller.dispose()` to release listeners
5. `build`: Wire `controller` to `TextField` widget

### Key Question Analysis

> **Can a single `BindIntent` drive both the simple React/Vue patterns AND Flutter's complex 5-step lifecycle? Or does Flutter need additional IR fields?**

**The current `BindIntent` drives all four targets because it encodes WHAT (source signal, element type, direction), not HOW.**

Three reasons NO additional IR fields are needed:

1. **The `BindIntent` is at the right abstraction level.** It says "bind this signal to this input element bidirectionally" -- the same way a developer thinks about it. How each framework implements that is a codegen concern, not an IR concern.

2. **Codegen has full context.** Flutter's codegen sees `ElementIR(tag="Input")` + `BindIntent(source=SignalRefIR(email))`. From these, it can deterministically produce the 5-step lifecycle without additional IR hints. `SignalRefIR(owner=None, name="email")` tells Flutter codegen this signal is owned by the component -- therefore generate a `StatefulWidget` with a prop named `email` and callback `onEmailChanged`. If `owner="store"`, the pattern changes.

3. **Pattern matching handles the complexity.** The Flutter codegen's pattern matcher recognizes `BindIntent + ElementIR(tag="Input")` and selects the `TextEditingController` idiom. This is the same mechanism that selects `ListView.builder` for `ListRenderingIntent`.

**Design risk to document:** The Flutter `StatefulWidget` generation from `BindIntent` introduces structural codegen complexity that the simpler targets do not face. This complexity is contained within the Flutter codegen module and does not leak into the IR definition.

### Conclusion: GO

A single `BindIntent` is sufficient to drive all four targets. The Flutter 5-step lifecycle is a codegen implementation challenge, not an IR design problem.

| Framework | Idiomatic Level | Notes |
|-----------|----------------|-------|
| React | L2 | `value` + `onChange` controlled pattern |
| Vue 3 | L2 | `v-model` directive (most concise) |
| Dioxus | L2 | `value` + `oninput` with `move` closure |
| Flutter | L2 | StatefulWidget + TextEditingController + full lifecycle |

> **Implementation Note for Phase 4:** The Flutter codegen must implement a `BindIntentToTextEditingController` pattern matcher that generates the complete `StatefulWidget` skeleton. Recommend prototyping in Phase 2a (Flutter L1) using `onChanged` + `setState` fallback, then upgrading in Phase 4.

---

## Case C: Flex Row/Column Mapping -- Layout Intent

### OmniView Source

```omni
<Flex direction="row" gap="16px" align="center">
    <Text>Item 1</Text>
    <Text>Item 2</Text>
</Flex>
```

### Intent IR Structure

```python
@dataclass
class LayoutIntent:
    """Semantic layout property intent"""
    target_element: str                  # ElementIR identifier
    properties: dict[str, ExprIR]       # {
                                        #     "direction": LiteralExpr("row"),
                                        #     "gap": LiteralExpr("16px"),
                                        #     "align": LiteralExpr("center"),
                                        # }

# Parent element:
# ElementIR(
#     tag="Flex",
#     children=[ElementIR(tag="Text"), ElementIR(tag="Text")],
#     inline_style=LayoutIntent(properties={...}),
# )
```

### Target Framework Expansion

**React (L2 Idiomatic) -- inline style:**

```tsx
<div style={{
    display: 'flex',
    flexDirection: 'row',
    gap: '16px',
    alignItems: 'center',
}}>
    <span>Item 1</span>
    <span>Item 2</span>
</div>
```

- `Flex` maps to `<div>` with inline flexbox CSS
- `direction="row"` maps to `flexDirection: 'row'`
- `align="center"` maps to `alignItems: 'center'`

**Vue 3 (L2 Idiomatic) -- inline style:**

```html
<div
    :style="{
        display: 'flex',
        flexDirection: 'row',
        gap: '16px',
        alignItems: 'center',
    }"
>
    <span>Item 1</span>
    <span>Item 2</span>
</div>
```

- Same CSS flexbox approach as React
- Vue uses `:style` directive

**Dioxus (L2 Idiomatic):**

```rust
div {
    style: "display: flex; flex-direction: row; gap: 16px; align-items: center;",
    span { "Item 1" }
    span { "Item 2" }
}
```

- Inline style string -- Dioxus CSS convention
- All properties concatenated into a single semicolon-separated string

**Flutter (L2 Idiomatic):**

```dart
Row(
    spacing: 16,  // Flutter 3.27+
    crossAxisAlignment: CrossAxisAlignment.center,
    children: [
        Text('Item 1'),
        Text('Item 2'),
    ],
)
```

- `direction="row"` determines the top-level widget: `Row()` (vs `Column()` for `direction="column"`)
- `gap: 16px` maps to `spacing: 16` (Flutter 3.27+) or `SizedBox` between children
- `align="center"` maps to `CrossAxisAlignment.center` -- fundamentally different from CSS `align-items` in implementation but semantically identical
- The `Flex` element itself is consumed by Flutter codegen -- it does NOT render as a `Flex` widget but as `Row`/`Column` directly

### Key Question Analysis

> **Can `LayoutIntent`'s flat properties map to both CSS key-value pairs AND Flutter constructor parameters? What about cross-axis alignment (`align` -> `align-items` in CSS vs `CrossAxisAlignment` in Flutter)?**

**The flat properties approach works because of a semantic mapping layer in each codegen.**

| Property | CSS (React/Vue/Dioxus) | Flutter Widget Param | Mapping Difficulty |
|----------|----------------------|---------------------|-------------------|
| `direction: "row"` | `flex-direction: row` | Widget type: `Row()` | Low -- widget type selection |
| `direction: "column"` | `flex-direction: column` | Widget type: `Column()` | Low -- widget type selection |
| `gap: "16px"` | `gap: 16px` | `spacing: 16` or SizedBox between | Medium -- value type diff |
| `align: "center"` | `align-items: center` | `crossAxisAlignment: CrossAxisAlignment.center` | Medium -- same concept, different enum |
| `justify: "space-between"` | `justify-content: space-between` | `mainAxisAlignment: MainAxisAlignment.spaceBetween` | Medium -- naming convention diff |
| `wrap: true` | `flex-wrap: wrap` | `Wrap()` widget instead of Row/Column | High -- structural change |

**Three key insights:**

1. **Widget type selection is the most critical mapping.** `direction: "row"` doesn't just set a CSS property -- for Flutter, it changes the root widget from `Column` to `Row`. The IR's flat properties encode this correctly -- the Flutter codegen pattern-matches on `direction` to select the widget type.

2. **Different value representations require per-codegen normalization.** CSS `gap: 16px` is a string with a unit. Flutter `spacing: 16` is a `double`. The codegen strips the `px` suffix. This is a codegen concern, not an IR concern.

3. **Cross-axis alignment maps correctly at the semantic level.** `align: "center"` means "center children along the cross axis." In CSS this is `align-items: center`; in Flutter this is `CrossAxisAlignment.center`. The Flutter codegen maintains a lookup table: `{"center": CrossAxisAlignment.center, "start": CrossAxisAlignment.start, ...}`.

**Edge case: `wrap: true`** -- Flutter must use the `Wrap` widget instead of `Row`/`Column`. The `LayoutIntent` properties dict can express this (`{"direction": "row", "wrap": True}`), but the Flutter codegen must recognize this combination and switch widget types. This is feasible but adds pattern matcher complexity. No IR changes needed.

### Conclusion: GO

`LayoutIntent` with flat properties is validated for all four targets. The semantic mapping layer in each codegen correctly translates abstract layout properties into framework-specific implementations. Flutter widget type selection (Row vs Column) based on `direction` is a codegen concern.

| Framework | Idiomatic Level | Notes |
|-----------|----------------|-------|
| React | L2 | Inline flexbox CSS or CSS Modules |
| Vue 3 | L2 | Same CSS approach as React |
| Dioxus | L2 | Inline style string concatenation |
| Flutter | L2 | `Row()`/`Column()` widget with constructor params |

> **Known edge case for Phase 2a/4:** `wrap: true` + `direction: "row"` requires Flutter to output `Wrap(direction: Axis.horizontal, ...)`. Add a test case for this in Phase 2a.

---

## Case D: If Empty Else Branch -- Conditional Rendering Null Representation

### OmniView Source

```omni
@if is_visible {
    <Content />
}
```
(No `@else` branch)

### Intent IR Structure

```python
@dataclass
class ConditionalIntent:
    """Conditional rendering intent"""
    branches: list[ConditionalBranch]   # [ConditionalBranch(cond=IdentifierExpr("is_visible"), body=Content)]
    fallback: Optional[ViewTree]        # None -- no @else present

@dataclass
class ConditionalBranch:
    condition: ExprIR   # IdentifierExpr("is_visible")
    body: ViewTree      # ElementIR(tag="Content")
```

### Target Framework Expansion

**React (L2 Idiomatic):**

```tsx
{is_visible ? <Content /> : null}
```

- Ternary expression with explicit `null` for the false branch
- React JSX renders `null` as nothing (no DOM node)

**Vue 3 (L2 Idiomatic):**

```html
<Content v-if="is_visible" />
```

- Single `v-if` directive -- when false, Vue removes the DOM node
- No explicit "else" representation needed -- Vue's template compiler handles it internally

**Dioxus (L2 Idiomatic):**

```rust
if is_visible {
    rsx! { Content {} }
} else {
    VNode::empty()
}
```

- `if/else` block in `rsx!` macro
- Explicit `VNode::empty()` for the false branch -- Dioxus requires a `VNode` in both branches

**Flutter (L2 Idiomatic):**

```dart
is_visible ? Content() : SizedBox.shrink()
```

- Ternary with `SizedBox.shrink()` for the false branch
- Zero-size widget that renders nothing -- Flutter's idiom for "no widget"

### Key Question Analysis

> **Each framework has a different "null" representation. Should the IR encode this knowledge, or should each codegen handle `None -> framework-null` independently?**

**The four null representations:**

| Framework | Null Representation | Technical Mechanism |
|-----------|-------------------|---------------------|
| React | `null` | JSX renders `null` as no DOM node |
| Vue 3 | (implicit) | `v-if` removes DOM node when false |
| Dioxus | `VNode::empty()` | Empty virtual node |
| Flutter | `SizedBox.shrink()` | Zero-size widget |

**Design decision: Each codegen handles independently. No IR encoding needed.**

Four reasons:

1. **The null representation is a framework idiom, not a semantic choice.** React's `null` is a JavaScript value. Vue's `v-if` has no explicit null -- it is baked into the directive semantics. Dioxus needs a typed `VNode`. Flutter's `SizedBox.shrink()` participates in the widget tree as zero-size. These are intrinsic framework properties, not semantic variations.

2. **Adding a `null_representation` field to `ConditionalIntent` would be a speculative abstraction.** It would add complexity to the IR and ALL codegens just to encode something each codegen already knows deterministically. This is a clear YAGNI violation.

3. **Codegen already knows its target.** When Flutter codegen encounters `ConditionalIntent(fallback=None)`, it knows to use `SizedBox.shrink()`. There is no ambiguity -- the null representation is a deterministic function of the target framework.

4. **Future frameworks would need their own null representation anyway.** Adding `Dioxus = "VNode::empty()"` today does not help when SwiftUI becomes a target (which uses `EmptyView()`). The null representation is inherently per-framework.

**Edge case with `@else` present:**

```omni
@if is_visible { <Content /> } @else { <Fallback /> }
```

When `fallback` is NOT `None`, no framework needs a "null" representation -- both branches produce real content.

| Framework | Expansion |
|-----------|-----------|
| React | `{is_visible ? <Content /> : <Fallback />}` |
| Vue | `<Content v-if="is_visible" /><Fallback v-else />` |
| Dioxus | `if/else { rsx!(...) }` |
| Flutter | `is_visible ? Content() : Fallback()` |

### Conclusion: GO

Each codegen should independently handle `None -> framework-null`. Encoding the null representation in `ConditionalIntent` would violate YAGNI and add speculative complexity.

| Framework | Idiomatic Level | Null Representation |
|-----------|----------------|--------------------|
| React | L2 | `null` |
| Vue 3 | L2 | Implicit (`v-if` handles it) |
| Dioxus | L2 | `VNode::empty()` |
| Flutter | L2 | `SizedBox.shrink()` (or `const SizedBox.shrink()`) |

> **Implementation Note for Phase 0:** React codegen for `ConditionalIntent(branches=[(cond, body)], fallback=None)` must produce `{cond ? (<body/>) : null}` without any wrapper (no fragment, no extra div).

---

## Overall Assessment

### Summary of Findings

| Canary Case | IR Type | Result | Key Insight |
|------------|---------|--------|-------------|
| **A.** ListView.builder | `ListRenderingIntent` | **GO** | No `strategy` field needed. Flutter always uses `ListView.builder`; `<StaticList>` for optimization. |
| **B.** TextEditingController Lifecycle | `BindIntent` | **GO** | Flutter's 5-step lifecycle is a codegen concern, not an IR concern. Codegen has full context from `SignalRefIR` + `ElementIR`. |
| **C.** Flex Row/Column Mapping | `LayoutIntent` | **GO** | Flat properties map correctly. Flutter codegen handles widget type selection (Row vs Column) based on `direction`. Semantic mapping layer handles cross-axis alignment. |
| **D.** If Empty Else Branch | `ConditionalIntent` | **GO** | Null representation is a per-framework idiom. Codegens handle independently. Encoding in IR would violate YAGNI. |

### Final Verdict

## ✅ GO

**The Intent IR approach is validated and ready for Phase 0 implementation.**

All 4 canary cases produce idiomatic L2 code across all four target frameworks without requiring IR structure changes. The core design principles are confirmed:

1. **IR stores intent, not implementation.** `ListRenderingIntent` stores "render a list" -- not ".map()" or "v-for" or "ListView.builder". Each codegen expands the intent into the framework-idiomatic implementation. This pattern is validated across all 4 cases.

2. **Codegen complexity is contained within each target's module.** Flutter's complex `TextEditingController` lifecycle and `Row`/`Column` widget selection are Flutter codegen concerns. They do not leak into the IR definition. This confirms the separation of concerns in the 3-stage pipeline (pattern matching -> idiomatic selection -> code expansion).

3. **Per-framework idioms are handled by codegen, not IR.** The null representation question (Case D), the `strategy` field question (Case A), and the cross-axis alignment mapping (Case C) all resolve to the same pattern: the IR says WHAT, the codegen decides HOW. This boundary is correctly placed.

### Risks and Recommendations

Despite the GO verdict, the following risks should be tracked:

| Risk | Severity | Recommendation |
|------|----------|---------------|
| Flutter codegen for `BindIntent` (TextEditingController lifecycle) is the single most complex pattern in the system. If poorly implemented, it could generate non-functional widgets. | **HIGH** | Prototype the `StatefulWidget` generation in Phase 2a (Flutter L1) using simplified `onChanged`+`setState`. Reserve `TextEditingController` for Phase 4 L2 with thorough unit testing. |
| `LayoutIntent` for `wrap:true` requires Flutter codegen to switch from Row/Column to `Wrap` widget -- a structural change, not just a parameter change. | **MEDIUM** | Document this edge case explicitly in the Flutter codegen module. Add a test case in Phase 2a that exercises `wrap: true`. |
| The `empty_body` field in `ListRenderingIntent` adds a conditional wrapper around the list. For large lists with `@empty`, React and Flutter both evaluate the length check every render -- no issue. But Dioxus codegen must ensure the `if/else` does not cause unnecessary re-renders. | **LOW** | Dioxus codegen should use `is_empty()` check (not `len() > 0` comparison) for idiomatic Rust. |
| 3 of 4 canary cases (A, B, C) require the Flutter codegen to make **structural widget decisions** (ListView.builder vs Column, StatefulWidget vs stateless, Row vs Column vs Wrap). This increases Flutter codegen complexity relative to the other targets. | **MEDIUM** | Budget Flutter codegen at 2x the effort of React/Vue codegen. The Phase 2a L1 implementation should confirm this estimate. |

### Phase 0 Starting Conditions

The IR design is frozen at `IRVersion 0.3`. The following types from the 4 canary cases are confirmed for Phase 0:

- `ListRenderingIntent` -- as defined in §10.1a, WITHOUT `strategy` field
- `BindIntent` -- as defined in §10.1a, with `source`/`element_type`/`direction` fields
- `LayoutIntent` -- as defined in §10.1b, with flat `properties` dict
- `ConditionalIntent` -- as defined in §10.1a, with `branches`/`fallback` fields

These types may be refined during Phase 0.2 (IR design + React codegen cross-validation) but their core structure is validated.

---

*Report generated 2026-06-05 for OmniLang v0.3 OmniView project. 4/4 canary cases passed.*
