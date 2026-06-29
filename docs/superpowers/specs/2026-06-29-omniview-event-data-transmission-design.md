# OmniView Event Data Transmission — Design Spec

> 2026-06-29 | Status: Approved

## Problem

The OmniView compiler currently emits event handler code that always passes `e.detail` (CustomEvent.detail) to OmniLang handler functions. Native DOM events (scroll, wheel, keydown, …) store their payload on the native event object itself — not in `detail`. As a result, event handlers always receive `undefined`.

Store action handlers receive no event data at all (`fn()` with no arguments).

The compiler must extract event data from native platform events and pass it to `.omni` handlers in a cross-target, compile-time-optimized way.

## Design Principles

1. **IR is the single source of truth** — event data schema defined once in IR
2. **Compile-time resolution** — `e.field` references resolved & validated at lowering
3. **Dead-field elimination** — only fields actually used by the handler are emitted
4. **Framework-idiomatic codegen** — each target generates code that reads like hand-written code for that framework

## Event Categories (20)

| # | EventDataKind | Events | Fields |
|---|--------------|--------|--------|
| 1 | PointerEvent | onClick, onPress, onTap, onDoubleClick, onLongPress, onMouseMove, onHover, onMouseEnter, onMouseLeave, onPointerDown, onPointerUp, onPointerMove, onPointerCancel | `x: float`, `y: float`, `pressure: float`, `tiltX: float`, `tiltY: float`, `pointerType: string` |
| 2 | TouchEvent | onTouchStart, onTouchMove, onTouchEnd, onTouchCancel | `touches: [{x: float, y: float, id: int}]` |
| 3 | WheelEvent | onWheel | `x: float`, `y: float`, `deltaX: float`, `deltaY: float`, `deltaZ: float`, `deltaMode: int` |
| 4 | KeyEvent | onKeyDown, onKeyUp | `key: string`, `code: string`, `alt: bool`, `ctrl: bool`, `shift: bool`, `meta: bool` |
| 5 | ScrollEvent | onScroll | `x: float`, `y: float` |
| 6 | InputEvent | onChange, onInput | `value: string`, `checked: bool` |
| 7 | ToggleEvent | onToggle | `checked: bool` |
| 8 | SubmitEvent | onSubmit | `value: string` |
| 9 | FocusEvent | onFocus, onBlur | `value: string` |
| 10 | ClipboardEvent | onCopy, onCut, onPaste | `text: string` |
| 11 | DragEvent | onDragStart, onDrag, onDragEnd | `x: float`, `y: float` |
| 12 | DropEvent | onDrop | `x: float`, `y: float`, `files: string[]`, `text: string` |
| 13 | ContextMenuEvent | onContextMenu | `x: float`, `y: float` |
| 14 | SelectEvent | onSelect | `start: int`, `end: int`, `text: string` |
| 15 | ResizeEvent | onResize | `width: float`, `height: float` |
| 16 | MediaEvent | onPlay, onPause, onEnded | `currentTime: float`, `duration: float` |
| 17 | MediaLoadEvent | onLoad | `naturalWidth: float`, `naturalHeight: float` |
| 18 | MediaErrorEvent | onError | `message: string` |
| 19 | AnimationEvent | onAnimationStart, onAnimationEnd, onAnimationIteration | `animationName: string`, `elapsedTime: float` |
| 20 | TransitionEvent | onTransitionEnd | `propertyName: string`, `elapsedTime: float` |

All event data objects implicitly carry `.preventDefault()` and `.stopPropagation()` methods.

## Implicit `e` Variable

In `.omni` event handler bodies (both inline and fn_ref), `e` is the implicit event data variable — scoped per-handler, typed per-event-category.

```omni
// Inline handler — e is implicitly available
@onScroll={scrollY = e.y}
@onKeyDown={if e.key == "Enter" { submit() }}
@onContextMenu={e.preventDefault(); menuX = e.x}

// FnRef handler — first parameter receives event data
@onScroll={handleScroll}
fn handleScroll(e) { ... }  // e: ScrollData

// Store action — event data N/A (no e)
@onPress={Counter.inc}
```

## Architecture

```
.omni source
    │
    ▼  Parser (no changes)
    │
    ▼  Lowering (AST → IR)
    │   • event_name → EventDataKind lookup
    │   • walk handler body → collect e.field references → used_fields
    │   • walk handler body → detect e.preventDefault() / e.stopPropagation()
    │   • validate: field exists in EventDataKind schema
    │   • validate: preventDefault allowed on this event type
    │
    ▼  IR
    │   EventHandlerIR {
    │     event_data: Option<EventDataKind>,
    │     used_fields: Vec<String>,
    │     needs_prevent_default: bool,
    │     needs_stop_propagation: bool,
    │   }
    │
    ▼  Codegen (per target)
        • Read IR event_data schema → emit field extraction per used_fields
        • Each target generates idiomatic code for its framework
```

## IR Schema

### EventDataKind

```rust
/// Event payload categories. Each variant has a fixed set of fields.
/// The field definitions are centralized in `event_data_schema()`.
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

/// Full schema: kind -> field definitions
/// Used by both lowering (validation) and codegen (extraction)
pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef> {
    match kind {
        EventDataKind::PointerEvent => vec![
            field("x", "float"),
            field("y", "float"),
            field("pressure", "float"),
            field("tiltX", "float"),
            field("tiltY", "float"),
            field("pointerType", "string"),
        ],
        // ... etc for all 20 variants
    }
}

/// Map event name string → EventDataKind
/// Used by lowering to determine which schema applies to each handler
pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind> {
    match event_name {
        "onClick" | "onPress" | "onTap" | "onDoubleClick" | "onLongPress"
        | "onMouseMove" | "onHover" | "onMouseEnter" | "onMouseLeave"
        | "onPointerDown" | "onPointerUp" | "onPointerMove" | "onPointerCancel"
            => Some(EventDataKind::PointerEvent),
        "onTouchStart" | "onTouchMove" | "onTouchEnd" | "onTouchCancel"
            => Some(EventDataKind::TouchEvent),
        "onWheel" => Some(EventDataKind::WheelEvent),
        "onKeyDown" | "onKeyUp" => Some(EventDataKind::KeyEvent),
        "onScroll" => Some(EventDataKind::ScrollEvent),
        "onChange" | "onInput" => Some(EventDataKind::InputEvent),
        "onToggle" => Some(EventDataKind::ToggleEvent),
        "onSubmit" => Some(EventDataKind::SubmitEvent),
        "onFocus" | "onBlur" => Some(EventDataKind::FocusEvent),
        "onCopy" | "onCut" | "onPaste" => Some(EventDataKind::ClipboardEvent),
        "onDragStart" | "onDrag" | "onDragEnd" => Some(EventDataKind::DragEvent),
        "onDrop" => Some(EventDataKind::DropEvent),
        "onContextMenu" => Some(EventDataKind::ContextMenuEvent),
        "onSelect" => Some(EventDataKind::SelectEvent),
        "onResize" => Some(EventDataKind::ResizeEvent),
        "onPlay" | "onPause" | "onEnded" => Some(EventDataKind::MediaEvent),
        "onLoad" => Some(EventDataKind::MediaLoadEvent),
        "onError" => Some(EventDataKind::MediaErrorEvent),
        "onAnimationStart" | "onAnimationEnd" | "onAnimationIteration"
            => Some(EventDataKind::AnimationEvent),
        "onTransitionEnd" => Some(EventDataKind::TransitionEvent),
        _ => None,
    }
}
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
    #[serde(default)]
    pub used_fields: Vec<String>,
    #[serde(default)]
    pub needs_prevent_default: bool,
    #[serde(default)]
    pub needs_stop_propagation: bool,
}
```

## Lowering Pass (AST → IR)

In `semantic/mod.rs`, during the lowering of `ElementProp::Event`:

1. Call `map_event_data_kind(&event_name)` to get `Option<EventDataKind>`
2. If `Some(kind)`: get `event_data_schema(&kind)` for validation
3. Walk handler body AST:
   - For each `Expr::MemberAccess(base, field)` where `base` is `e`:
     - Check `field` exists in schema; if not → compile error: "`e.xxx` is not a field of KindEvent"
     - Record `field` in `used_fields`
   - For `Stmt::Expr(Expr::MethodCall("e", "preventDefault", []))`:
     - Set `needs_prevent_default = true`
   - For `Stmt::Expr(Expr::MethodCall("e", "stopPropagation", []))`:
     - Set `needs_stop_propagation = true`
4. For FnRef handlers: analyze the referenced function's first parameter type
   to infer which fields are accessed (or conservatively use all)
5. Write `event_data`, `used_fields`, `needs_prevent_default`, `needs_stop_propagation`
   into `EventHandlerIR`

## Codegen Changes

### Web (omniview.rs)

**Before (current):**
```js
el.addEventListener("scroll", (e) => { handler(e.detail); });
```

**After:**
```js
// @onScroll={handleScroll} → used_fields: ["y"]
el.addEventListener("scroll", (_e) => {
  handler({ y: _e.target.scrollTop });
});
```

**Field extraction table (Web target):**

| EventDataKind | field | extraction |
|--------------|-------|-----------|
| PointerEvent | x | `_e.clientX` |
| PointerEvent | y | `_e.clientY` |
| PointerEvent | pressure | `_e.pressure` |
| PointerEvent | tiltX | `_e.tiltX` |
| PointerEvent | tiltY | `_e.tiltY` |
| PointerEvent | pointerType | `_e.pointerType` |
| TouchEvent | touches | `[..._e.touches].map(t => ({x:t.clientX, y:t.clientY, id:t.identifier}))` |
| WheelEvent | x | `_e.clientX` |
| WheelEvent | y | `_e.clientY` |
| WheelEvent | deltaX | `_e.deltaX` |
| WheelEvent | deltaY | `_e.deltaY` |
| WheelEvent | deltaZ | `_e.deltaZ` |
| WheelEvent | deltaMode | `_e.deltaMode` |
| KeyEvent | key | `_e.key` |
| KeyEvent | code | `_e.code` |
| KeyEvent | alt | `_e.altKey` |
| KeyEvent | ctrl | `_e.ctrlKey` |
| KeyEvent | shift | `_e.shiftKey` |
| KeyEvent | meta | `_e.metaKey` |
| ScrollEvent | x | `_e.target.scrollLeft` |
| ScrollEvent | y | `_e.target.scrollTop` |
| InputEvent | value | `_e.target.value` |
| InputEvent | checked | `_e.target.checked` |
| ToggleEvent | checked | `_e.target.checked` |
| SubmitEvent | value | `_e.target.elements[...].value` or `new FormData(_e.target).get(...)` |
| FocusEvent | value | `_e.target.value` |
| ClipboardEvent | text | `_e.clipboardData.getData('text')` |
| DragEvent | x | `_e.clientX` |
| DragEvent | y | `_e.clientY` |
| DropEvent | x | `_e.clientX` |
| DropEvent | y | `_e.clientY` |
| DropEvent | files | `[..._e.dataTransfer.files].map(f => f.name)` |
| DropEvent | text | `_e.dataTransfer.getData('text')` |
| ContextMenuEvent | x | `_e.clientX` |
| ContextMenuEvent | y | `_e.clientY` |
| SelectEvent | start | `_e.target.selectionStart` |
| SelectEvent | end | `_e.target.selectionEnd` |
| SelectEvent | text | `_e.target.value.substring(_e.target.selectionStart, _e.target.selectionEnd)` |
| ResizeEvent | width | `window.innerWidth` |
| ResizeEvent | height | `window.innerHeight` |
| MediaEvent | currentTime | `_e.target.currentTime` |
| MediaEvent | duration | `_e.target.duration` |
| MediaLoadEvent | naturalWidth | `_e.target.naturalWidth` |
| MediaLoadEvent | naturalHeight | `_e.target.naturalHeight` |
| MediaErrorEvent | message | `_e.target.error?.message \|\| 'unknown'` |
| AnimationEvent | animationName | `_e.animationName` |
| AnimationEvent | elapsedTime | `_e.elapsedTime` |
| TransitionEvent | propertyName | `_e.propertyName` |
| TransitionEvent | elapsedTime | `_e.elapsedTime` |

### React (react.rs)

**After:**
```jsx
// @onScroll={handleScroll} → used_fields: ["y"]
<div onScroll={(e) => handleScroll({ y: e.target.scrollLeft })} />

// @onKeyDown={handleKey} → used_fields: ["key", "code"]
<input onKeyDown={(e) => handleKey({ key: e.key, code: e.code })} />
```

React uses its own SyntheticEvent — field extraction adapts accordingly (mostly identical to Web, differences: clipboard uses `e.clipboardData`, touch uses React's event wrapper).

### Flutter (flutter.rs)

**After:**
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

### Other targets (dioxus.rs, vue.rs, javascript.rs, python.rs, dart.rs, rust.rs)

Same pattern: each target maintains its own `EventDataAdapter` that maps
`(EventDataKind, field_name)` → native extraction code string.

## @for Event Delegation

When events are inside `@for` template bodies, the current mechanism using
`data-event-*` attributes must be extended:

```html
<!-- Before (current): -->
<div data-event-scroll="fn:DataTable_handleScroll">...</div>

<!-- After: -->
<div data-event-scroll="fn:DataTable_handleScroll" data-event-kind="ScrollEvent">
  ...
</div>
```

The delegated listener reads `data-event-kind` to determine which extraction
to apply before calling the handler. The extraction is done via a compile-time
generated lookup:

```js
const _extractors = {
  ScrollEvent(e) {
    return { x: e.target.scrollLeft, y: e.target.scrollTop };
  },
  KeyEvent(e) {
    return { key: e.key, code: e.code, alt: e.altKey, ctrl: e.ctrlKey, shift: e.shiftKey, meta: e.metaKey };
  },
  // ... only kinds that actually appear in @for templates
};

parent.addEventListener("scroll", (e) => {
  const t = e.target.closest("[data-event-scroll]");
  if (!t) return;
  const fnName = t.getAttribute("data-event-scroll").slice(3);
  const fn = __omni_forHandlers[fnName];
  if (!fn) return;
  const kind = t.dataset.eventKind || "";
  const extract = _extractors[kind];
  fn(extract ? extract(e) : {});
});
```

In the @for context, `used_fields` optimization is applied conservatively:
since multiple items may have different field access patterns, the union
of all used fields across all @for items is emitted.

## preventDefault / stopPropagation

In `.omni` source:
```omni
@onContextMenu={e.preventDefault(); menuX = e.x}
@onSubmit={e.preventDefault(); submit()}    // alread handled by current pd flag
```

In generated code, these become direct calls on the raw event:
```js
el.addEventListener("contextmenu", (_e) => {
  _e.preventDefault();
  handler({ x: _e.clientX, y: _e.clientY });
});
```

The lowering pass detects `e.preventDefault()` / `e.stopPropagation()` as
method calls on the implicit `e` variable and sets the corresponding IR flags.

## Files to Modify

| File | Changes |
|------|---------|
| `omnilang/src/ir/mod.rs` | Add `EventDataKind`, `EventFieldDef`, schema functions; update `EventHandlerIR` |
| `omnilang/src/semantic/mod.rs` | Add lowering logic: collect e.field → used_fields, validate |
| `omnilang/src/semantic/lints.rs` | Add lint: "unused event field" warning |
| `omnilang/src/codegen/omniview.rs` | Replace all 5 `e.detail` call sites with field extraction from IR; add emit_event_extract(); add @for event-kind attribution |
| `omnilang/src/codegen/react.rs` | Replace event handler emission with IR-based field extraction |
| `omnilang/src/codegen/flutter.rs` | Replace event handler emission with IR-based field extraction |
| `omnilang/src/codegen/dioxus.rs` | Same as React/Web pattern |
| `omnilang/src/codegen/vue.rs` | Same as Web pattern |
| `omnilang/src/codegen/javascript.rs` | Same as Web pattern |
| `test/projects/learn-omniview/` | Update test .omni files to exercise event data (e.g., `@onScroll={scrollY = e.y}`) |
| `build/omni-motion-runtime.js` | No changes needed (build artifact, regenerated) |

## Backward Compatibility

Existing `.omni` code that doesn't reference `e.field` continues to work
unchanged — handlers that don't use event data simply ignore the argument.

Code that currently accesses `e.detail` in inline handlers will need to be
updated — but since `e.detail` has always been `undefined` for native events,
no working code depends on it.

## Testing Strategy

1. **Unit tests** — `map_event_data_kind()` returns correct kind for all event names
2. **Unit tests** — `event_data_schema()` returns correct fields for all 20 kinds
3. **Integration tests** — Lowering produces correct used_fields for various handler patterns
4. **Codegen snapshot tests** — verify emitted JS/Dart/JSX for each target
5. **Lint tests** — invalid field access produces compile error
6. **E2E tests** — browser tests verifying scroll/key/wheel data reaches handler
