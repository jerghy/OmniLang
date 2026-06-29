# OmniView Event Data Transmission — Design Spec v2

> 2026-06-29 | Status: Approved (post-review v2)

## Problem

The OmniView compiler currently emits event handler code that always passes
`e.detail` (CustomEvent.detail) to OmniLang handler functions. Native DOM
events (scroll, wheel, keydown, …) store their payload on the native event
object itself — not in `detail`. As a result, event handlers always receive
`undefined`.

Store action handlers receive no event data at all (`fn()` with no arguments).

The compiler must extract event data from native platform events and pass it to
`.omni` handlers in a cross-target, compile-time-optimized way.

## Design Principles

1. **IR is the single source of truth** — event data schema defined once in IR
2. **Compile-time resolution** — `e.field` references resolved & validated at lowering
3. **Dead-field elimination** — only fields actually used by the handler are emitted
4. **Framework-idiomatic codegen** — each target generates code that reads like hand-written code for that framework
5. **Null-safe extraction** — all generated field extractions include fallback values

## IR Design: Shared Fields

Modifier keys (`alt`, `ctrl`, `shift`, `meta`) are **not** KeyEvent-specific.
In native DOM, `MouseEvent`, `WheelEvent`, `PointerEvent`, `KeyboardEvent`,
`DragEvent`, and `TouchEvent` all carry modifier state.  Likewise `button`
(0=left, 1=middle, 2=right) is a property of every pointer-like event.

The IR schema reflects this: any EventDataKind whose native source carries
modifiers includes them as first-class fields — same name, same type, same
semantics across all categories.  There is no "shared mixin" indirection;
each category lists its full field set explicitly.

## Event Categories (20)

| # | EventDataKind | Events | Fields |
|---|--------------|--------|--------|
| 1 | PointerEvent | onClick, onPress, onTap, onDoubleClick, onLongPress, onMouseMove, onHover, onMouseEnter, onMouseLeave, onPointerDown, onPointerUp, onPointerMove, onPointerCancel | `x: float`, `y: float`, `button: int`, `pressure: float`, `tiltX: float`, `tiltY: float`, `pointerType: string`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 2 | TouchEvent | onTouchStart, onTouchMove, onTouchEnd, onTouchCancel | `touches: [{x: float, y: float, id: int}]`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 3 | WheelEvent | onWheel | `x: float`, `y: float`, `deltaX: float`, `deltaY: float`, `deltaZ: float`, `deltaMode: int`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 4 | KeyEvent | onKeyDown, onKeyUp | `key: string`, `code: string`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 5 | ScrollEvent | onScroll | `x: float`, `y: float` |
| 6 | InputEvent | onChange, onInput | `value: string`, `checked: bool` |
| 7 | ToggleEvent | onToggle | `checked: bool` |
| 8 | SubmitEvent | onSubmit | Web: `values: {[fieldName]: string}` / Flutter: `value: string` |
| 9 | FocusEvent | onFocus, onBlur | `value: string` |
| 10 | ClipboardEvent | onCopy, onCut, onPaste | `text: string` |
| 11 | DragEvent | onDragStart, onDrag, onDragEnd | `x: float`, `y: float`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 12 | DropEvent | onDrop | `x: float`, `y: float`, `files: string[]`, `text: string`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 13 | ContextMenuEvent | onContextMenu | `x: float`, `y: float`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 14 | SelectEvent | onSelect | `start: int`, `end: int`, `text: string` |
| 15 | ResizeEvent | onResize | `width: float`, `height: float` |
| 16 | MediaEvent | onPlay, onPause, onEnded | `currentTime: float`, `duration: float` |
| 17 | MediaLoadEvent | onLoad | `naturalWidth: float`, `naturalHeight: float` |
| 18 | MediaErrorEvent | onError | `message: string` |
| 19 | AnimationEvent | onAnimationStart, onAnimationEnd, onAnimationIteration | `animationName: string`, `elapsedTime: float` |
| 20 | TransitionEvent | onTransitionEnd | `propertyName: string`, `elapsedTime: float` |

All event data objects implicitly carry `.preventDefault()` and
`.stopPropagation()` methods.  `onSubmit` always auto-calls `preventDefault`
(backward-compatible with current behavior).

## Implicit `e` Variable

In `.omni` event handler bodies (both inline and fn_ref), `e` is the implicit
event data variable — scoped per-handler, typed per-event-category.

```omni
// Inline handler — e is implicitly available
@onScroll={scrollY = e.y}
@onKeyDown={if e.key == "Enter" { submit() }}
@onContextMenu={e.preventDefault(); menuX = e.x}

// FnRef handler — first parameter receives event data
@onScroll={handleScroll}
fn handleScroll(e) { ... }  // e: ScrollData

// Store action — no event data (emit lint warning if developer
// writes e.field in an action target, since it won't receive data)
@onPress={Counter.inc}
```

### `e` Semantics in Nested Scopes

When a handler body contains `@for` or `@if` directives, the `e` variable
remains scoped to the event handler, not the inner directive.  If an inner
scope declares a variable named `e`, it shadows the event data — the
compiler emits a warning.

### `e.preventDefault()` Is Optional for onSubmit

`onSubmit` always calls `preventDefault()` in generated code regardless of
whether the handler explicitly writes it.  This preserves backward
compatibility with existing `@onSubmit={handler}` code that relied on the
compiler's previously-hardcoded behavior.  For all other event types,
`preventDefault()` is emitted only when the handler explicitly calls
`e.preventDefault()`.

## Architecture

```
.omni source
    │
    ▼  Parser (no changes — FieldAccess & MethodCall already parse)
    │
    ▼  Lowering Pass 1 (inline handlers)
    │   • event_name → EventDataKind lookup
    │   • walk handler body → collect e.field references → used_fields
    │   • walk handler body → detect e.preventDefault() / e.stopPropagation()
    │   • validate: field exists in EventDataKind schema
    │   • validate: platform supports this event type (warning if not)
    │
    ▼  Lowering Pass 2 (fn_ref handlers — backfill)
    │   • Re-visit fn_ref handlers after component function bodies lowered
    │   • Walk referenced function's body → collect e.field → used_fields
    │   • If function body unavailable (external / not yet lowered):
    │     mark used_fields = None (codegen emits all fields)
    │
    ▼  IR
    │   EventHandlerIR {
    │     event_data: Option<EventDataKind>,
    │     used_fields: Option<Vec<String>>,   // None = all fields
    │     needs_prevent_default: bool,
    │     needs_stop_propagation: bool,
    │   }
    │
    ▼  Codegen (per target)
        • Read IR event_data schema → emit only used_fields (or all if None)
        • Special binding targets: ResizeEvent → window
        • @for delegation: dynamic event-type subscription + JS kind map
        • All extractions are null-safe with fallback values
```

## IR Schema

### EventDataKind

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventDataKind {
    PointerEvent,
    TouchEvent,
    WheelEvent,
    KeyEvent,
    ScrollEvent,
    InputEvent,
    ToggleEvent,
    SubmitEvent,
    FocusEvent,
    ClipboardEvent,
    DragEvent,
    DropEvent,
    ContextMenuEvent,
    SelectEvent,
    ResizeEvent,
    MediaEvent,
    MediaLoadEvent,
    MediaErrorEvent,
    AnimationEvent,
    TransitionEvent,
}
```

### EventFieldDef

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFieldDef {
    pub name: String,        // "x", "key", "deltaY", ...
    pub field_type: String,  // "float" | "string" | "bool" | "int"
                             // | "{x,y,id}[]" | "string[]"
}

/// Full schema: kind -> field definitions.
pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef> { ... }

/// Map event name string → EventDataKind.
pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind> { ... }

/// Map EventDataKind to a string label for the @for JS kind lookup.
pub fn event_data_kind_label(kind: &EventDataKind) -> &'static str { ... }
```

### Updated EventHandlerIR

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandlerIR {
    pub kind: String,                    // "fn_ref" | "inline_fn" | "store_action"
    pub body: Option<BlockIR>,
    pub r#ref: Option<String>,
    pub params: Vec<String>,
    // ── New fields ──
    #[serde(default)]
    pub event_data: Option<EventDataKind>,
    /// None = "all fields" (conservative for unresolved fn_ref).
    /// Some(vec) = only these fields are used; empty vec = no fields used (pass `{}`).
    #[serde(default)]
    pub used_fields: Option<Vec<String>>,
    #[serde(default)]
    pub needs_prevent_default: bool,      // explicit e.preventDefault()
    #[serde(default)]
    pub needs_stop_propagation: bool,     // explicit e.stopPropagation()
}
```

## Lowering Pass (AST → IR)

### Pass 1 — Inline Handlers (with full AST body)

In `semantic/mod.rs`, during the lowering of `ElementProp::Event`:

1. Call `map_event_data_kind(&event_name)` → `Option<EventDataKind>`
2. If `Some(kind)`: call `event_data_schema(&kind)` for field validation
3. If `event_name == "onSubmit"`: set `needs_prevent_default = true` (always)
4. Walk handler body AST:
   - `Expr::FieldAccess { object: VarRef("e"), field }`:
     - Check `field` exists in schema; if not → **compile error**: "`e.{field}` is not a field of {Kind}"
     - Record `field` in `used_fields`
   - `Expr::MethodCall { object: VarRef("e"), method: "preventDefault", args: [] }`:
     - Set `needs_prevent_default = true`
   - `Expr::MethodCall { object: VarRef("e"), method: "stopPropagation", args: [] }`:
     - Set `needs_stop_propagation = true`
   - If `Expr::FieldAccess` where base is NOT `e` but base contains nested `e.field`:
     - Record the field access normally (codegen handles indirection)
5. For **inline_fn**: `used_fields = Some(collected_fields)`
6. For **fn_ref**: `used_fields = None` (deferred to Pass 2)
7. For **store_action**: `event_data = None`, `used_fields = None`
   If the action path contains a field access on `e` → **compile error**: "store actions cannot access event data; use an inline handler or fn_ref instead"

### Pass 2 — FnRef Backfill

After all functions in the component are lowered:

1. For each fn_ref handler with `used_fields == None`:
   - Resolve `r#ref` name → component function (via `global_renames`)
   - If found: walk function body IR for `FieldAccessExpr { object: VarRefExpr("e"), field }` → collect `used_fields`
   - If not found (external function): leave `None`
2. Set `used_fields = Some(collected)` or leave `None`

## Codegen Changes

### Web (omniview.rs) — Field Extraction Table with Null-Safety

All generated extractions use optional chaining + nullish coalescing:

| EventDataKind | field | extraction | fallback |
|--------------|-------|-----------|----------|
| PointerEvent | x | `_e.clientX` | `0` |
| PointerEvent | y | `_e.clientY` | `0` |
| PointerEvent | button | `_e.button` | `0` |
| PointerEvent | pressure | `_e.pressure` | `0` |
| PointerEvent | tiltX | `_e.tiltX` | `0` |
| PointerEvent | tiltY | `_e.tiltY` | `0` |
| PointerEvent | pointerType | `_e.pointerType` | `""` |
| PointerEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| TouchEvent | touches | merged `touches` + `changedTouches`, deduped by `identifier` | `[]` |
| TouchEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| WheelEvent | x | `_e.clientX` | `0` |
| WheelEvent | y | `_e.clientY` | `0` |
| WheelEvent | delta* | `_e.deltaX/Y/Z` | `0` |
| WheelEvent | deltaMode | `_e.deltaMode` | `0` |
| WheelEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| KeyEvent | key | `_e.key` | `""` |
| KeyEvent | code | `_e.code` | `""` |
| KeyEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| ScrollEvent | x | `_e.target?.scrollLeft` | `0` |
| ScrollEvent | y | `_e.target?.scrollTop` | `0` |
| InputEvent | value | `_e.target?.value` | `""` |
| InputEvent | checked | `_e.target?.checked` | `false` |
| ToggleEvent | checked | `_e.target?.checked` | `false` |
| SubmitEvent | values | `Object.fromEntries(new FormData(_e.target))` | `{}` |
| FocusEvent | value | `_e.target?.value` | `""` |
| ClipboardEvent | text | try-catch wrapped `_e.clipboardData?.getData('text')` | `""` |
| DragEvent | x | `_e.clientX` | `0` |
| DragEvent | y | `_e.clientY` | `0` |
| DragEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| DropEvent | x | `_e.clientX` | `0` |
| DropEvent | y | `_e.clientY` | `0` |
| DropEvent | files | `[...(_e.dataTransfer?.files ?? [])].map(f => f.name)` | `[]` |
| DropEvent | text | try-catch `_e.dataTransfer?.getData('text')` | `""` |
| DropEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| ContextMenuEvent | x | `_e.clientX` | `0` |
| ContextMenuEvent | y | `_e.clientY` | `0` |
| ContextMenuEvent | alt/ctrl/shift/meta | `_e.altKey` etc. | `false` |
| SelectEvent | start | `_e.target?.selectionStart` | `0` |
| SelectEvent | end | `_e.target?.selectionEnd` | `0` |
| SelectEvent | text | computed from `substring(start, end)` on target value | `""` |
| ResizeEvent | width | `window.innerWidth` | `0` |
| ResizeEvent | height | `window.innerHeight` | `0` |
| MediaEvent | currentTime | `_e.target?.currentTime` | `0` |
| MediaEvent | duration | `_e.target?.duration` | `0` |
| MediaLoadEvent | naturalWidth | `_e.target?.naturalWidth` | `0` |
| MediaLoadEvent | naturalHeight | `_e.target?.naturalHeight` | `0` |
| MediaErrorEvent | message | `_e.target?.error?.message` | `"unknown"` |
| AnimationEvent | animationName | `_e.animationName` | `""` |
| AnimationEvent | elapsedTime | `_e.elapsedTime` | `0` |
| TransitionEvent | propertyName | `_e.propertyName` | `""` |
| TransitionEvent | elapsedTime | `_e.elapsedTime` | `0` |

### TouchEvent — Touches Merge

```js
// Merges touches + changedTouches, deduplicated by identifier.
// changedTouches is essential for touchend (touches is empty on touchend).
function _mergeTouches(e) {
  const seen = new Map();
  for (const t of e.touches)
    seen.set(t.identifier, { x: t.clientX, y: t.clientY, id: t.identifier });
  for (const t of e.changedTouches)
    if (!seen.has(t.identifier))
      seen.set(t.identifier, { x: t.clientX, y: t.clientY, id: t.identifier });
  return [...seen.values()];
}
```

The `_mergeTouches` helper is emitted once per compilation unit when TouchEvent appears anywhere in the IR.

### ResizeEvent — Special Binding Target

`onResize` binds to `window`, not individual elements:

```js
// @onResize={handleResize} → used_fields: ["width"]
window.addEventListener("resize", (_e) => {
  handleResize({ width: window.innerWidth });
});
```

### SubmitEvent — Web vs Flutter

**Web:** SubmitEvent maps to `<form>` submit.  Field `values` is generated from
`FormData`.  There is no `value` field on Web — it is `values`:

```js
el.addEventListener("submit", (_e) => {
  _e.preventDefault();  // always present for onSubmit
  handler({ values: Object.fromEntries(new FormData(_e.target)) });
});
```

**Flutter:** SubmitEvent maps to `TextField.onSubmitted(String value)`:

```dart
TextField(
  onSubmitted: (value) => handler(SubmitData(values: { '': value })),
)
```

### onMouseMove — Add to map_event

`onMouseMove` is added to `map_event` in omniview.rs:

```rust
"onMouseMove" => "mousemove".into(),
```

### Codegen — Direct Binding (non-@for)

```js
// @onScroll={handleScroll} used_fields: Some(["y"])
el.addEventListener("scroll", (_e) => {
  handler({ y: _e.target?.scrollTop ?? 0 });
});

// @onKeyDown={handleKey} used_fields: Some(["key", "code"])
el.addEventListener("keydown", (_e) => {
  handler({ key: _e.key, code: _e.code });
});

// @onScroll={handleScroll} used_fields: None (fn_ref, unresolved)
el.addEventListener("scroll", (_e) => {
  handler({ x: _e.target?.scrollLeft ?? 0, y: _e.target?.scrollTop ?? 0 });
});

// @onClick={Counter.inc} store_action — no event data
el.addEventListener("click", () => { Counter_inc(); });
```

### React (react.rs)

```jsx
// @onScroll={handleScroll} used_fields: Some(["y"])
<div onScroll={(e) => handleScroll({ y: e.target.scrollLeft })} />

// @onKeyDown={handleKey} used_fields: Some(["key", "code"])
<input onKeyDown={(e) => handleKey({ key: e.key, code: e.code })} />
```

### Flutter (flutter.rs)

```dart
// @onScroll={handleScroll}
NotificationListener<ScrollNotification>(
  onNotification: (notification) {
    handleScroll(ScrollData(
      x: notification.metrics.pixels,
      y: notification.metrics.pixels,
    ));
    return false;
  },
  child: ...
)

// @onKeyDown={handleKey}
RawKeyboardListener(
  onKey: (event) {
    handleKey(KeyData(
      key: event.logicalKey.keyLabel,
      code: event.logicalKey.debugName,
    ));
  },
  child: ...
)
```

### Other Targets

Same pattern (dioxus, vue, javascript, python, dart, rust): each target
maintains its own adapter mapping `(EventDataKind, field_name)` → native
extraction code string with null-safe fallback.

## @for Event Delegation (Rewritten)

### Problem

Current @for delegation subscribes to only 7 hardcoded event types (lines
1762–1768): `click, submit, input, change, focus, blur, keydown`.  With
the new event categories, `scroll, wheel, mousemove, keyup, touchstart,
touchmove, touchend, paste, copy, cut, dragstart, drag, dragend, drop,
contextmenu, animationstart, animationend, transitionend` must also work.

Additionally, the current delegation passes `e.detail` to handlers — same
bug as non-@for bindings.

### Solution — Dynamic Subscription + JS Kind Map

Instead of adding `data-event-kind` HTML attributes, the compiler generates a
JS map from DOM event name → event data kind.  No DOM attribute changes needed.

```js
// Generated once per @for container, covering ALL event types used within:
const _eventKinds = {
  scroll:  "ScrollEvent",
  click:   "PointerEvent",
  keydown: "KeyEvent",
  input:   "InputEvent",
  // ... only entries for events actually used in this @for
};

// Dynamic subscription — compiler emits one addEventListener per used event:
["scroll", "click", "keydown", "input"].forEach(function(evt) {
  parent.addEventListener(evt, _forHandle);
});

// The _forHandle dispatcher:
function _forHandle(e) {
  // Find closest child with data-event-<type> attribute
  const t = e.target.closest("[data-event-" + e.type + "]");
  if (!t) return;
  const attr = t.getAttribute("data-event-" + e.type);
  if (!attr) return;
  if (e.type === "submit") e.preventDefault();

  const kind = _eventKinds[e.type] || "";

  if (attr.startsWith("fn:")) {
    const fn = __omni_forHandlers[attr.slice(3)];
    if (!fn) return;
    // ... restore per-instance state (unchanged) ...
    fn(_extractEvent(kind, e));  // ← replaces fn(e.detail)
    // ... sync state back (unchanged) ...
  } else if (attr.startsWith("store:")) {
    const fn = __omni_forHandlers[attr.slice(6)];
    if (typeof fn === "function") fn();  // unchanged — store gets no data
  }
}

// Extraction dispatch — compiled at compile time, only for types present:
function _extractEvent(kind, e) {
  switch (kind) {
    case "ScrollEvent":
      return { x: e.target?.scrollLeft ?? 0, y: e.target?.scrollTop ?? 0 };
    case "PointerEvent":
      return { x: e.clientX, y: e.clientY, button: e.button,
               alt: e.altKey, ctrl: e.ctrlKey, shift: e.shiftKey, meta: e.metaKey };
    case "KeyEvent":
      return { key: e.key, code: e.code,
               alt: e.altKey, ctrl: e.ctrlKey, shift: e.shiftKey, meta: e.metaKey };
    case "WheelEvent":
      return { deltaX: e.deltaX, deltaY: e.deltaY, deltaZ: e.deltaZ, deltaMode: e.deltaMode,
               alt: e.altKey, ctrl: e.ctrlKey, shift: e.shiftKey, meta: e.metaKey };
    case "InputEvent":
      return { value: e.target?.value ?? "", checked: e.target?.checked ?? false };
    default:
      return {};
  }
}
```

**Why not `data-event-kind` attribute?** A JS lookup map avoids adding
attributes to every DOM element in @for output.  The extraction function is
generated per-@for-controller and includes only the event kinds that actually
appear.  TouchEvent's `_mergeTouches` helper is emitted globally once when
TouchEvent appears anywhere.

### @for used_fields Optimization

In @for context, used_fields are the **union** of all item-level handlers
for the same event type.  If item A uses `@onScroll={e.y}` and item B uses
`@onScroll={scrollPos = e.x + e.y}`, the union is `["x", "y"]`.

## preventDefault / stopPropagation

| Event | preventDefault | Source |
|-------|---------------|--------|
| onSubmit | **always** | backward compat + explicit |
| onContextMenu | explicit `e.preventDefault()` | handler body |
| onDrop | explicit `e.preventDefault()` | handler body |
| onWheel | explicit `e.preventDefault()` | handler body |
| onKeyDown | explicit `e.preventDefault()` | handler body |
| all others | explicit only | handler body |

In generated code:

```js
el.addEventListener("contextmenu", (_e) => {
  _e.preventDefault();               // emitted because handler wrote e.preventDefault()
  handler({ x: _e.clientX, y: _e.clientY });
});
```

```js
el.addEventListener("submit", (_e) => {
  _e.preventDefault();               // always emitted for onSubmit
  handler({ values: Object.fromEntries(new FormData(_e.target)) });
});
```

## Platform Validation

During lowering, each event name is validated against the target platform:

- **onAnimationStart / onAnimationEnd / onAnimationIteration / onTransitionEnd**:
  Web-only CSS animation events.  Emit **warning** (not error) for Flutter target:
  "event `{name}` has no native Flutter equivalent; consider using
  AnimationController.addStatusListener"

- **onMouseMove / onMouseEnter / onMouseLeave**: mouse-only events.  Emit
  **note** for mobile-first targets: "event `{name}` will not fire on
  touch-only devices"

### Event → Platform Support Matrix

| Event | Web | React | Flutter | Dioxus | Vue |
|-------|-----|-------|---------|--------|-----|
| onClick/onPress/onTap | ✓ | ✓ | ✓ | ✓ | ✓ |
| onMouseMove/Enter/Leave | ✓ | ✓ | ⚠ hover | ✓ | ✓ |
| onTouchStart/Move/End | ✓ | ✓ | ⚠ GestureDetector | ✓ | ✓ |
| onWheel | ✓ | ✓ | ⚠ Listener | ✓ | ✓ |
| onKeyDown/Up | ✓ | ✓ | ✓ | ✓ | ✓ |
| onScroll | ✓ | ✓ | ✓ | ✓ | ✓ |
| onChange/onInput | ✓ | ✓ | ✓ | ✓ | ✓ |
| onSubmit | ✓ | ✓ | ✓ | ✓ | ✓ |
| onAnimation* | ✓ | ✓ | ✗ | ✓ | ✓ |
| onTransitionEnd | ✓ | ✓ | ✗ | ✓ | ✓ |

✓ = native support, ⚠ = has equivalent but different API, ✗ = no equivalent

Warning is emitted only for ✗ targets.

## Files to Modify

| File | Changes |
|------|---------|
| `omnilang/src/ir/mod.rs` | Add `EventDataKind`, `EventFieldDef`, schema + mapping functions; update `EventHandlerIR` (used_fields: Option<Vec<String>>) |
| `omnilang/src/semantic/mod.rs` | Pass 1: inline handler e.field collection + validation. Pass 2: fn_ref backfill. onSubmit auto-preventDefault. Platform validation warnings. |
| `omnilang/src/semantic/lints.rs` | New lints: "store action cannot access event data", "event `{name}` has no equivalent on target `{target}`", "variable `e` shadows event data in nested scope" |
| `omnilang/src/codegen/omniview.rs` | Replace all `e.detail` with `_extractEvent(kind, _e)`; add `map_event_data_kind()`; add `onMouseMove` to `map_event`; @for dynamic subscription; ResizeEvent → window; TouchEvent touches merge; null-safe extraction; SubmitEvent.values |
| `omnilang/src/codegen/react.rs` | Event data extraction per React SyntheticEvent; SubmitEvent.values |
| `omnilang/src/codegen/flutter.rs` | Event data extraction; SubmitEvent.value; platform-specific warnings |
| `omnilang/src/codegen/dioxus.rs` | Same as Web pattern |
| `omnilang/src/codegen/vue.rs` | Same as Web pattern |
| `omnilang/src/codegen/javascript.rs` | Same as Web pattern |
| `test/projects/learn-omniview/` | Update .omni files to exercise event data across multiple event types |
| `build/omni-motion-runtime.js` | No changes needed (build artifact) |

## Backward Compatibility

1. **Handlers without `e.field`**: continue to work; they simply ignore the
   data argument (signature compatible: from receiving `undefined` to receiving
   an object — both are callable with one argument).

2. **onSubmit auto-preventDefault**: preserved.  All existing `@onSubmit`
   handlers continue to have `preventDefault()` auto-emitted.

3. **Store actions**: behavior unchanged — receive no event data.

4. **`e.detail`**: previously always `undefined` for native events.  After
   this change, `e` refers to the normalized data object, which does not have
   a `detail` property.  Since `undefined.detail` threw anyway, and
   `eventDataObject.detail` is `undefined` (no harm), this is not a real
   break.

## Testing Strategy

1. **Unit tests** — `map_event_data_kind()` for all event names
2. **Unit tests** — `event_data_schema()` field count & types for all 20 kinds
3. **Unit tests** — `_mergeTouches` (touchend + touchmove, dedup)
4. **Integration tests** — Pass 1: inline handler produces correct `used_fields`
5. **Integration tests** — Pass 2: fn_ref backfill produces correct `used_fields`
6. **Integration tests** — invalid `e.field` → compile error
7. **Integration tests** — store action accessing `e` → compile error
8. **Codegen snapshot tests** — Web: scroll handler emits `{ y: _e.target?.scrollTop ?? 0 }`
9. **Codegen snapshot tests** — Web: @for delegation emits dynamic event subscription
10. **Codegen snapshot tests** — Web: ResizeEvent binds to `window`
11. **Codegen snapshot tests** — Web: TouchEvent emits `_mergeTouches` helper
12. **Codegen snapshot tests** — React, Flutter: field extraction per framework
13. **Lint tests** — platform compatibility warnings
14. **E2E tests** — browser: scroll/key/wheel/touch data reaches handler
15. **E2E tests** — browser: @for scroll delegate fires with correct data
