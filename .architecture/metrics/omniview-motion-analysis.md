# OmniView Motion/Timeline/@for Code Path Analysis

## 1. Module Overview

### `motion.rs` (150 lines)
Top-level dispatcher for motion/animation JS codegen. Contains three functions:
- `emit_motion_js` -- entry point for named `@motion` blocks (defined at module or component level). Branches on `MotionBodyIR` variants (Keyframe, Timeline, CallbackOnly) and emits a `__omni_motion_{name}` factory function that calls `Omni.animate()`.
- `format_transform_value` -- stateless utility that maps shorthand transform keys to CSS transform function strings (e.g., `"x"` + `"100"` -> `"translateX(100px)"`).
- `emit_states_js` -- generates multi-state animation JS for an element. Two modes: reserved names (`idle`/`hover`/`pressed`) auto-bind mouse events; custom names rely on a signal-driven `__omni_effect()`.

Key IR types consumed: `MotionIntent`, `MotionBodyIR`, `KeyframeIR`, `TimelineIR`, `CallbackBodyIR`, `StatesIR`, `StateTweenIR`.

### `motion_config.rs` (412 lines)
Serialization layer that converts IR animation types to JavaScript config object fragments. Contains all the pure-ish "IR to JS string" conversion logic:
- `motion_body_to_config` -- dispatcher for inline motion definitions (returns `(keyframes_js, opts_js)` pair for `Omni.animate()`).
- `keyframe_to_config` / `keyframe_to_animate_call` -- keyframe serialization (from/to stops, `over`, ease, path, playback config, callbacks).
- `timeline_body_to_config` / `timeline_to_factory` -- timeline/config serialization (stub: returns placeholder keyframes with playback config).
- `callback_to_config` / `callback_to_animate_call` -- callback-only config serialization.
- `easing_to_js` -- converts `EasingIR` to JS expression (6 variants: Named, CubicBezier, Spring, Back, Elastic, Steps).
- `playback_to_config` -- converts `PlaybackIR` to a list of JS config property strings (14 conditional fields).
- `is_transform_prop` -- stateless property-name check for CSS transform properties.

### `motion_triggers.rs` (594 lines)
The largest and most complex module. Generates JS for element-level motion triggers (`@enter`, `@hover`, `@press`, `@in-view`, `@scroll`, `@drag-x`, `@drag-y`, etc.). Structure:
- Three private helper functions for complex trigger types, each shared between named-motion-ref and inline-motion paths via an `is_inline` boolean:
  - `emit_trigger_inview_js` -- `IntersectionObserver`-based trigger with configurable threshold, disconnect-once/repeat logic.
  - `emit_trigger_scroll_js` -- scroll-linked animation via `Omni.scrollProgress()` with lerp-based scrub support. Notable: different JS output for `is_inline` vs named (placement of `anim.currentTime` assignment).
  - `emit_trigger_drag_js` -- pointer-event-based drag with velocity tracking, constraints parsing (parent/ref:/numeric), elastic overshoot, and threshold-based commit/reverse.
- `emit_motion_triggers_for_element` -- main dispatcher that iterates all `TriggerIntent`s on an element and emits appropriate JS per trigger kind. Two main branches: named motion ref (11 trigger kinds) and inline motion (with similar structure). Uses `self.js` for deferred code, `self.mount_js` for mount-lifecycle code, and `self.enter_trigger_nids` for signal-change replay registration.

### `timeline.rs` (329 lines)
Generates timeline orchestration functions (`__omni_motion_{name}`) that coordinate multi-element animations. Key details:
- `emit_timeline_js` -- single entry point. Parses 6 timeline item kinds:
  - `Direction` (To/FromTo): the core animation item. Complex delay calculation with cursor-position tracking and a "pair base" pattern (two Direction items share a base cursor). Start modes: `with-prev`, `overlap(N)`, `after-prev(N)`, `with-offset(N)`, `absolute(N)`.
  - `Set`: immediate property application (zero-duration keyframe with `fill: "forwards"`).
  - `MotionRef`: delegates to a named `@motion` for all children.
  - `Stagger`: applies a named motion to children with per-index delay.
  - `Label`: names a cursor position for reference; optionally fires a callback at that position.
  - Supports 3 connectors: `None` (resets cursor), `Then` (sequential), `With` (parallel).
  - Emits label map (`window.__omni_timeline_labels`), wrapper function with `kill()`/`cancel()`, and calls itself at mount time.

### `list.rs` (156 lines)
State initialization for `@for` list rendering. Contains:
- `emit_list_rendering_html` -- entry point that chains `emit_list_state_init` + `emit_list_dom`.
- `emit_list_state_init` -- generates three `<template>` elements (item template, container marker, optional empty state) and starts the reactive `__omni_effect()` JS. Sets up `data`/`tpl`/`ctr`/`parent` JS variables, key-based identity tracking (`_forKeys` Map), and empty-state fallback.
  - State managed: `for_text_bindings`, `for_class_bindings`, `for_attr_bindings`, `for_rebind_fn`, `in_for_template`.

### `list_render.rs` (368 lines)
DOM generation and event delegation for `@for`. Contains:
- `emit_list_dom` -- single entry point. Generates the iterative cloning JS:
  - FLIP recording (`Omni.flipBefore`/`Omni.flipAfter`) for key-animated items.
  - Cloning with `<fragment>` unwrapping.
  - Pre-compiled text/class/attribute binding application (no `new Function`).
  - Per-instance component state initialization (`data-for-state-names` / `__omniState`).
  - Key-based identity tracking: reuse existing DOM nodes for matching keys, remove nodes for missing keys with `@leave` animation support.
  - Delegated event listener setup with `_extractEvent` switch function and `_eventKinds` lookup map.
  - Per-instance state save/restore during event handling (signal push/pop pattern).
  - `__omni_rebind_{comp}_{item}` function emission for state-change text re-evaluation.

### Supporting files
- `mod.rs` (622 lines): Defines `OmniViewGenerator` struct (46 fields), `OmniViewOutput`, entry point `generate()`, `project_uses_motion()` guard, `view_tree_has_motion()` recursive scan. Also defines `AttrPart` enum (Static/Dynamic) for template attribute parsing.
- `types_motion.rs` (412 lines): Defines all motion/animation IR types (see Section 2).
- `runtime.rs` (~150 lines): `emit_animation_runtime()` emits the `@omni/animation` runtime reference (WAAPI-based, loaded from external script).
- `utils.rs` (280 lines): `collect_for_events()` recursive tree walker used by list_render; `parse_attr_template()` used for attribute binding parsing.
- `types.rs` (ElementIR): `ElementIR` carries `motion`, `scroll`, `drag`, `hero` optional Box<> fields with accessor methods.

---

## 2. Key Functions & Contracts

### motion.rs

| Function | Signature | Input | Output | Side Effects |
|---|---|---|---|---|
| `emit_motion_js` | `(&mut self, motion: &MotionIntent)` | Named motion definition IR | JS factory function: `function __omni_motion_{name}(el, overrides) { ... }` | Appends to `self.js` |
| `format_transform_value` | `(key: &str, value_js: &str) -> String` | Transform shorthand name + JS expression | CSS transform function string | None (pure) |
| `emit_states_js` | `(&mut self, nid: &str, states_ir: &StatesIR)` | Element NID + state definitions | JS lookup table `var __omni_states_{nid}` + event binding code | Appends to `self.js` and `self.mount_js` |

### motion_config.rs

| Function | Signature | Input | Output | Side Effects |
|---|---|---|---|---|
| `motion_body_to_config` | `(&self, body: &MotionBodyIR) -> (String, String)` | Motion body IR | (keyframes_js, opts_js) pair | None (read-only) |
| `keyframe_to_config` | `(&self, kf: &KeyframeIR) -> String` | Keyframe IR | JS config fragment as string | None (read-only) |
| `timeline_body_to_config` | `(&self, tl: &TimelineIR) -> String` | Timeline IR | JS config fragment with `"type":"timeline"` | None (read-only) |
| `callback_to_config` | `(&self, cb: &CallbackBodyIR) -> String` | Callback body IR | JS config fragment | None (read-only) |
| `easing_to_js` | `(&self, easing: &EasingIR) -> String` | Easing IR | JS string or object expression | None (read-only) |
| `keyframe_to_animate_call` | `(&self, kf: &KeyframeIR) -> (String, String)` | Keyframe IR | (keyframes_array_js, options_json) | None (read-only) |
| `timeline_to_factory` | `(&self, _name: &str, tl: &TimelineIR) -> (String, String)` | Timeline IR | (placeholder_kf, opts) | None (read-only) |
| `callback_to_animate_call` | `(&self, cb: &CallbackBodyIR) -> (String, String)` | Callback body IR | (empty_kf, callback_options) | None (read-only) |
| `playback_to_config` | `(&self, playback: &PlaybackIR) -> Vec<String>` | Playback IR | List of JS property strings | None (read-only) |
| `is_transform_prop` | `(key: &str) -> bool` | Property name string | Boolean | None (pure) |

### motion_triggers.rs

| Function | Signature | Input | Output | Side Effects |
|---|---|---|---|---|
| `emit_trigger_inview_js` | `(&mut self, nid, trigger_str, anim_call, el, is_inline)` | Element data + animation call | `IntersectionObserver` JS with configurable threshold/disconnect | Appends to `self.js` |
| `emit_trigger_scroll_js` | `(&mut self, nid, trigger_str, anim_call, el, is_inline)` | Element data + animation call | Scroll-linked animation JS with scrub lerp | Appends to `self.js` |
| `emit_trigger_drag_js` | `(&mut self, nid, trigger_str, anim_call, el, is_horizontal, is_inline)` | Element data + animation call + axis | Pointer-event drag JS with constraints/elastic/inertia | Appends to `self.js` |
| `emit_motion_triggers_for_element` | `(&mut self, nid: &str, el: &ElementIR)` | Element NID + element IR | Trigger JS for all trigger kinds on the element | Appends to `self.js`, `self.mount_js`, `self.enter_trigger_nids` |

### timeline.rs

| Function | Signature | Input | Output | Side Effects |
|---|---|---|---|---|
| `emit_timeline_js` | `(&mut self, timeline: &TimelineIntent)` | Timeline intent IR | Timeline factory function + mount call | Appends to `self.js` and `self.mount_js`; may push warnings to `self.warnings` |

### list.rs

| Function | Signature | Input | Output | Side Effects |
|---|---|---|---|---|
| `emit_list_rendering_html` | `(&mut self, list: &ListRenderingIntent, indent: usize) -> String` | List rendering IR + indent | HTML string for template elements | Calls `emit_list_state_init` + `emit_list_dom` |
| `emit_list_state_init` | `(&mut self, list: &ListRenderingIntent, indent: usize) -> String` | List rendering IR + indent | HTML string for templates | Appends to `self.js`; mutates `for_text_bindings`, `for_class_bindings`, `for_attr_bindings`, `for_rebind_fn`, `in_for_template` |

### list_render.rs

| Function | Signature | Input | Output | Side Effects |
|---|---|---|---|---|
| `emit_list_dom` | `(&mut self, list: &ListRenderingIntent)` | List rendering IR | JS iteration/cloning/binding code | Appends to `self.js` |

---

## 3. Edge Cases & Error Conditions

### motion_config.rs
1. **Keyframe `over` duration is a non-numeric literal** (e.g., `ExprIR::LiteralExpr` with a string like `"auto"`): `keyframe_to_config` silently skips it (line 55-59 drops the non-f64 literal). The `over` field is omitted from config, falling back to the runtime default.
2. **Keyframe `over` is a non-Literal expression**: falls through to `expr_to_js` -- will produce whatever JS expression was provided, no validation.
3. **Keyframe has no `from` stop (position 0.0)**: `keyframe_to_config` emits only `"to"` config. Works but produces a one-sided animation (animates from current state).
4. **Keyframe has no `to` stop (position 1.0)**: `keyframe_to_config` emits only `"from"` config. Unusual but won't crash.
5. **Keyframe with no stops at all**: `keyframe_to_animate_call` produces an empty keyframes array `[]` -- likely a runtime error from WAAPI.
6. **Path animation with `path_rotate` missing**: `keyframe_to_config` has duplicate path/path_rotate emission (lines 72-77 and 84-91) -- first block emits path_rotate even when `false`, second block only when `true`. The `path` config is emitted twice (minor inefficiency, but functionally redundant not harmful).
7. **Easing with missing optional fields**: `Back.direction` defaults to `"out"`, `Spring.velocity` defaults to `0.0`, `Elastic.direction` defaults to `"out"` -- defensive but may mask logical errors.
8. **Callback name matching is exhaustive**: `CallbackKindIR` has 5 variants, each matched explicitly with no wildcard -- adding a new variant would produce a compile error (desirable).

### motion_triggers.rs
9. **Trigger with no `motion_ref` and empty `inline_motion`**: `emit_motion_triggers_for_element` iterates triggers; if both are absent, `kf_override` is empty string and `motion_ref` is `None` -- the entire trigger is silently skipped (no JS emitted). This is the desired behavior for triggers parsed from invalid/missing motion references.
10. **`in_view_threshold` is non-numeric expression**: passes through to `expr_to_js` as-is, may produce invalid JS at runtime.
11. **`in_view_repeat` evaluates to non-boolean**: comparison `self.expr_to_js(expr) == "true"` may give false negatives/positives for unusual expressions.
12. **Scroll `scrub` value is non-numeric**: `expr_to_js` output used directly in JS ternary comparison (`scrubDelay > 0`). If the expression is a string/object, this may produce unexpected truthiness.
13. **Drag `constraints` is `"parent"` but element has no `parentElement`**: inline `parseConstraint` returns `{min: -Infinity, max: Infinity}` -- no constraint applied, fallback works.
14. **Drag `constraints` starts with `"ref:"` but ref element not found**: same Infinity fallback.
15. **Drag `constraints` empty string**: `parseConstraint` returns `{min: -Infinity, max: Infinity}` -- unconstrained drag.
16. **Drag `constraints` has malformed format (not `"min:val max:val"`)**: `parseFloat(val) || 0` may silently produce 0 for NaN values.
17. **Drag element not found (`__omni_el` returns null)**: all trigger helper functions early-return `if (!el) return;` -- safe.

### timeline.rs
18. **Timeline item with negative or zero `over` duration**: Warning emitted ("Timeline 'over' duration must be positive"), defaults to 300ms.
19. **Timeline item with non-numeric `over` literal**: Warning emitted ("Timeline 'over' duration not a valid number"), defaults to 300ms.
20. **Timeline item with non-literal `over`**: No validation -- `over_ms` is set to 300.0 (falls to else branch in match). Could silently override a dynamic expression.
21. **Stagger `each` delay negative or zero**: Warning emitted, defaults to 50ms.
22. **Stagger `each` non-numeric literal**: Warning emitted, defaults to 50ms.
23. **Timeline start mode parsing**: String-based parsing with `starts_with()` / manual substring extraction. Malformed strings like `"overlap"` (no parens) fall through to the `ref_cursor` default. Unrecognized start modes silently use `ref_cursor`.
24. **Timeline `MotionRef` item**: Emits `children.forEach(...)` which applies the motion to ALL children. If the motion name doesn't exist, it's a runtime error.
25. **`parse_ms` closure**: If `trimmed.parse::<f64>()` fails, emits warning and returns `0.0`. The closure captures `&self` for the warning push.
26. **Connector `None` resets cursor to 0.0**: This means a `None`-connected item starts a new independent timeline branch. If the item has no explicit delay, it starts at 0 regardless of preceding items.
27. **Timeline with zero items**: `emit_timeline_js` produces a factory with empty steps array. `Omni.timeline([])` behavior at runtime is undefined.
28. **`DirectionIR::FromTo` with odd number of properties**: The flip-flop `is_to` toggle (line 46-52) would put the last property in the wrong bucket (from instead of to). Properties must come in pairs.

### list.rs
29. **No `list.body`**: `<template data-nid="{nid}-tpl">` contains `<!-- empty item -->`. The JS still clones and inserts it at runtime, producing empty nodes.
30. **Empty data array with no `empty_body`**: Early return via `if (!data || data.length === 0) return;` -- no DOM changes.
31. **Empty data array with `empty_body`**: Clones empty template and inserts before container marker. Does NOT clear previous items first (the clear loop runs before the data check). **Likely bug**: If data transitions from non-empty to empty, previous items remain in DOM because the clear loop runs, but it only clears items between tpl and ctr. However, if items were already cleared on the non-empty pass, this might be fine. Need careful verification -- the empty state clone is inserted AFTER the clear loop, but the `return` prevents the data.forEach from adding more items. The previously rendered items would have been cleared by the while loop on line 106.
32. **`key_expr` throws during evaluation**: Caught by try/catch, returns `null`. Items with null keys are treated as new clones each render.
33. **Template root is `<fragment>`**: Clone creates a `DocumentFragment` with all children moved individually (line 47). The `data-for-item` attribute is set on `firstElementChild` only -- multi-root fragments lose the index marker on non-first children.
34. **Template root is not an Element (e.g., text node)**: `clone.querySelector` calls fail silently. Pre-compiled bindings don't apply.

### list_render.rs
35. **Keyed render with no key match**: On first render with key, all items are new. On subsequent renders, oldKeys with no match trigger leave animations or immediate removal.
36. **Leave animation player not found**: Falls through to immediate `el.remove()`.
37. **Leave animation `.play()` returns non-thenable**: The `.then()` on line 188 could throw if the return value isn't a Promise. (Likely from `Omni.animate()` which probably returns a WAAPI Animation whose `play()` returns undefined -- `undefined.then()` crashes.)
38. **Event delegation source element (`data-event-{type}`) not found**: No crash -- `closest` returns null, handler returns early.
39. **`_forKeys` Map holds stale DOM references**: If an item is removed externally or its DOM node is recycled, the Map entry becomes stale. No cleanup mechanism other than the key-based removal pass.
40. **Per-instance signal save/restore during event handling**: The restore step (`_sig.set(value)`) mutates the global signal. If event handlers trigger effects that read the signal, they see the per-instance value. After the handler, signals are synced back to element state via `peek()`. **Race condition risk**: If multiple event handlers fire concurrently (e.g., rapid pointer events on different instances), the signal push/pop pattern may interleave.
41. **Empty `for_event_types`**: Event delegation setup is skipped entirely -- minor optimization but means events defined in the @for body are unbound.

### Cross-cutting
42. **`expr_to_js` errors**: Called extensively throughout all modules. `expr_to_js` lives in `expr.rs` (~ hundreds of lines) and can produce arbitrary strings. Invalid expressions produce invalid JS.
43. **JS string concatenation vs injection**: All modules use `format!()` with raw string interpolation into JS. There is no JS-level escaping for generated string literals (e.g., state names, CSS class names). A state name containing `"` would break the JS.

---

## 4. Testable Units

### Pure / Read-Only Functions (can test with constructed IR, no generator mutation)

| Function | File:Line | Priority | Why Worth Testing |
|---|---|---|---|
| `format_transform_value` | motion.rs:31 | **High** | Pure function, 6 shorthand-to-CSS mappings. Critical for correct transform animation output. Edge: unknown keys pass through as raw. |
| `is_transform_prop` | motion_config.rs:399 | **High** | Pure function, 9 recognized property names. Determines whether a property goes into `transform:` or as a standalone CSS property. |
| `easing_to_js` | motion_config.rs:174 | **High** | 6 EasingIR variants, each with optional fields. Output consumed by `Omni.resolveEasing()`. Coverage: Named, CubicBezier, Spring, Back, Elastic, Steps. Edge: missing direction on Back/Elastic, missing velocity on Spring. |
| `keyframe_to_animate_call` | motion_config.rs:200 | **High** | Core conversion: KeyframeIR -> (keyframes_array, options_object). 5 playback fields mapped. Edge: empty stops, missing from/to stops, path rotation, all callback kinds, non-literal over. |
| `playback_to_config` | motion_config.rs:304 | **High** | 14 conditional fields from PlaybackIR. Each field (loop_count, direction, fill, delay, etc.) can be independently present/absent. Edge: every combination of fields. |
| `keyframe_to_config` | motion_config.rs:18 | **Med** | Similar to `keyframe_to_animate_call` but produces config-object fragment (different format). Used by inline motion paths. |
| `callback_to_animate_call` | motion_config.rs:276 | **Med** | CallbackBodyIR -> (keyframes, opts). Handles 5 callback kinds. Edge: empty callbacks, non-literal over. |
| `callback_to_config` | motion_config.rs:132 | **Med** | Alternative serialization for callback bodies. |
| `timeline_body_to_config` | motion_config.rs:117 | **Low** | Stub: always emits `"type":"timeline"` + playback config. Minimal logic. |
| `timeline_to_factory` | motion_config.rs:263 | **Low** | Always returns placeholder keyframes + playback options. Trivial. |
| `motion_body_to_config` | motion_config.rs:9 | **Med** | Dispatcher that delegates to 3 variants. Easy to test each variant in isolation via its dedicated function; this is a thin wrapper. |

### State-Mutating Functions (need `OmniViewGenerator` instance via `test_new()`)

| Function | File:Line | Priority | Why Worth Testing |
|---|---|---|---|
| `emit_motion_js` | motion.rs:7 | **High** | Entry point for all named motion emission. Verifies correct delegation to keyframe/timeline/callback branches. Output must be valid `__omni_motion_{name}(el, overrides)` factory. |
| `emit_states_js` | motion.rs:45 | **High** | Two modes (reserved + signal-driven). Emits state lookup table + event bindings. Edge: empty states map, missing signal for custom states, no reserved names mixed with custom. |
| `emit_motion_triggers_for_element` | motion_triggers.rs:322 | **High** | Largest function in the motion path. 11 trigger kinds x 2 modes (named ref / inline) = 22+ paths. Each path emits different JS patterns. Must test: each trigger kind, each mode, `_` wildcard fallback, empty triggers, triggers with only ref + no inline, triggers with only inline + no ref. |
| `emit_trigger_inview_js` | motion_triggers.rs:7 | **High** | Shared by named and inline @in-view. Edge: threshold 0/negative/expression, disconnect=true/false, is_inline flag difference. |
| `emit_trigger_scroll_js` | motion_triggers.rs:81 | **High** | Scroll scrub animation with lerp. Edge: scrub=0 (immediate), scrub>0 (lerp), missing start/end defaults, is_inline vs named branch (different anim.currentTime placement). |
| `emit_trigger_drag_js` | motion_triggers.rs:185 | **High** | Complex pointer event logic with constraints parsing, elastic overshoot, velocity tracking, threshold commit/reverse. Edge: unconstrained drag, parent constraint with/without parent, ref: constraint with/without ref element, elastic=0/1, threshold=0, velocity_max=0. |
| `emit_timeline_js` | timeline.rs:7 | **High** | Timeline orchestration with 6 item kinds. Edge: empty timeline, single item, Direction items with each start mode, FromTo with odd/even property counts, Stagger with zero/non-literal each, Label with/without callback, connector=None (cursor reset), all playback settings. |
| `emit_list_state_init` | list.rs:28 | **High** | Template HTML generation + JS effect header. Edge: no body, no empty_body, key_expr present/absent, empty data with/without empty_body, nested @for (in_for_template flag). |
| `emit_list_dom` | list_render.rs:16 | **High** | Core @for iteration JS. Edge: key-based vs non-keyed, pre-compiled text/class/attr bindings present/absent, for_state_names present/absent, event delegation setup, FLIP animations, leave animation integration. |
| `project_uses_motion` | mod.rs:355 | **Med** | Guard function for 46KB runtime embedding. Recursively scans all modules/components/view trees for motion/scroll/drag/hero. Edge: motion only in nested conditional branches, component-only motions, module-level-only motions. |
| `view_tree_has_motion` | mod.rs:376 | **Med** | Recursive tree walker. Edge: deep nesting, motion on conditional branches, motion in @for body, empty view trees. |

### Utility Functions Used by the Above

| Function | File:Line | Priority | Why Worth Testing |
|---|---|---|---|
| `collect_for_events` | utils.rs:49 | **Med** | Recursive event type collection for @for delegation. Edge: nested @for, event on conditional branches, duplicate event types, events on component roots. |
| `parse_attr_template` | utils.rs:125 | **Low** | Template attribute parsing (`{expr}` placeholders). Edge: unclosed `{`, consecutive placeholders, leading/trailing static text, empty input. |
| `map_event` | utils.rs:29 | **Low** | Omni event name to DOM event name mapping. 13 explicit mappings + fallback. |

---

## 5. Module Dependencies

### Call Graph (simplified)

```
mod.rs :: generate()
  ├── project_uses_motion(ir)          [standalone fn, no generator needed]
  │    └── view_tree_has_motion(tree)  [recursive]
  ├── emit_motion_js(motion)           [motion.rs]
  │    ├── keyframe_to_animate_call(kf)  [motion_config.rs]
  │    │    └── expr_to_js / easing_to_js / playback_to_config
  │    ├── timeline_to_factory(name, tl) [motion_config.rs]
  │    │    └── expr_to_js / playback_to_config
  │    └── callback_to_animate_call(cb)  [motion_config.rs]
  │         └── expr_to_js / playback_to_config
  ├── emit_timeline_js(timeline)       [timeline.rs]
  │    ├── expr_to_js / easing_to_js   [expr.rs / motion_config.rs]
  │    └── self.warnings               [state mutation]
  └── generate_component()             [component_html.rs]
       └── view_to_html / emit_element... etc.
            └── emit_motion_triggers_for_element(nid, el)  [motion_triggers.rs]
                 ├── motion_body_to_config(inline)          [motion_config.rs]
                 ├── emit_trigger_inview_js(...)            [motion_triggers.rs]
                 ├── emit_trigger_scroll_js(...)            [motion_triggers.rs]
                 ├── emit_trigger_drag_js(...)              [motion_triggers.rs]
                 ├── self.enter_trigger_nids                [state mutation]
                 ├── self.mount_js                          [state mutation]
                 └── self.js                                [state mutation]

view_to_html / emit_element... (view/element.rs)
  └── emit_list_rendering_html(list, indent)   [list.rs]
       ├── emit_list_state_init(list, indent)  [list.rs]
       │    ├── self.for_text_bindings / for_class_bindings / for_attr_bindings
       │    ├── self.for_rebind_fn / in_for_template
       │    ├── self.next_nid / expr_to_js / view_to_html
       │    └── self.js                        [state mutation]
       └── emit_list_dom(list)                 [list_render.rs]
            ├── collect_for_events(tree)       [utils.rs]
            ├── emit_event_object(kind)        [crate::codegen::emit_event_object]
            ├── self.for_text_bindings / for_class_bindings / for_attr_bindings
            ├── self.for_rebind_fn
            └── self.js                        [state mutation]
```

### Shared Dependencies

All modules depend on:
- `OmniViewGenerator` struct (defined in `mod.rs`, 46 fields)
- `expr_to_js` (defined in `expr.rs`) -- converts `ExprIR` to JS expression strings
- `easing_to_js` (defined in `motion_config.rs`) -- shared by timeline, motion_triggers, and motion modules
- `playback_to_config` (defined in `motion_config.rs`) -- used by keyframe, timeline, callback codegen paths
- IR types from `types_motion.rs`, `types.rs`

### Data Flow Between Modules

1. **Named motion emission**: `mod.rs:emit_motion_js()` -> dispatches to `motion_config.rs` for body serialization -> writes factory function to `self.js`.
2. **Timeline emission**: `mod.rs:emit_timeline_js()` -> independent emission of timeline factory to `self.js` and mounting call to `self.mount_js`.
3. **Trigger emission**: `view/element.rs` calls `emit_motion_triggers_for_element()` -> reads `MotionBehavior` from `ElementIR` -> for inline motions, calls `motion_config.rs:motion_body_to_config()` -> writes to `self.js`, `self.mount_js`, `self.enter_trigger_nids`.
4. **@for rendering**: `view/element.rs` calls `emit_list_rendering_html()` -> `emit_list_state_init()` generates templates and starts JS effect header -> `emit_list_dom()` completes the effect body with iteration/cloning/binding JS.
5. **Output stream separation**: `self.js` gets deferred execution code (functions, effects, class/attr bindings). `self.mount_js` gets mount-lifecycle code (trigger plugins, animation creation). These are merged at `finalize_js()` in `runtime.rs`.

### State Coupling (Side Effects on OmniViewGenerator)

| State Field | Written By | Read By |
|---|---|---|
| `self.js` | All 6 modules | `mod.rs::finalize_js()`, wrapped as output |
| `self.mount_js` | motion_triggers.rs, timeline.rs, mod.rs | `mod.rs::finalize_js()`, appended after deferred JS |
| `self.warnings` | timeline.rs (parse_ms closure) | `OmniViewOutput::warnings` output field |
| `self.enter_trigger_nids` | motion_triggers.rs | `mod.rs::generate_component()` for replay binding |
| `self.for_text_bindings` | list.rs (via view_to_html during in_for_template) | list_render.rs |
| `self.for_class_bindings` | list.rs | list_render.rs |
| `self.for_attr_bindings` | list.rs | list_render.rs |
| `self.for_rebind_fn` | list.rs | list_render.rs |
| `self.in_for_template` | list.rs | view/element.rs (conditional rendering mode) |

---

## 6. Test Strategy Recommendations

### Order of Writing Tests

**Phase 1: Pure function tests (fast, no generator setup needed)**

1. `is_transform_prop` -- 1 test file, ~10 assertions, can verify all 9 recognized + unrecognized keys
2. `format_transform_value` -- 1 test file, ~12 assertions, cover 6 shorthand keys + unknown fallback + edge values (negative, decimal, zero)
3. `easing_to_js` -- 1 test file, ~15 assertions, cover all 6 variants with optional fields (missing direction, missing velocity, steps edge cases)

**Estimated: 3 files, ~40 assertions, can be written in parallel**

**Phase 2: Read-only conversion tests (need constructed IR)**

4. `playback_to_config` -- 1 test file, ~20 assertions, each field independently tested + combined cases (empty PlaybackIR, all fields populated)
5. `keyframe_to_animate_call` -- 1 test file, ~25 assertions, cover: empty stops, single stop, from+to stops, path+path_rotate, all playback fields, non-literal over, all callback kinds
6. `callback_to_animate_call` -- 1 test file, ~10 assertions, cover empty/multiple callbacks, non-literal over
7. `motion_body_to_config` -- delegates; light testing of dispatcher logic only

**Estimated: 3 files, ~55 assertions**

**Phase 3: State-mutating function tests (need `OmniViewGenerator::test_new()`)**

8. `emit_motion_js` -- 1 test file, ~15 assertions. Verify `self.js` output for Keyframe/Timeline/CallbackOnly paths. Snapshot factory function shape.
9. `emit_states_js` -- 1 test file, ~12 assertions. Reserved names mode vs signal-driven mode, empty states, state target lookup table format.
10. `emit_motion_triggers_for_element` -- **the largest test target**. Recommend splitting into sub-tests:
    - Named-ref triggers: Enter, Hover, Press, Focus, Leave, InView, Scroll, DragX, DragY, Animate, Timeline (11 sub-tests)
    - Inline triggers: Enter, Hover, InView, Scroll, DragX, DragY (6 sub-tests)
    - Edge: empty triggers vec, both ref + inline (inline takes priority?), unhandled trigger kinds via `_` wildcard
    - **Estimated: ~30 assertions across ~15 test cases**

11. `emit_trigger_inview_js` -- 1 test file, ~8 assertions. Named vs inline, threshold variations, disconnect-once vs repeat.
12. `emit_trigger_scroll_js` -- 1 test file, ~8 assertions. Named vs inline (different anim.currentTime placement), scrub=0 vs >0, missing start/end defaults.
13. `emit_trigger_drag_js` -- 1 test file, ~12 assertions. Horizontal vs vertical, constraints=parent/ref:/numeric/empty, elastic=0/0.5/1, threshold=0/0.5, velocity_max=0/2000.
14. `emit_timeline_js` -- 1 test file, ~20 assertions. All 6 item kinds, Direction start modes (with-prev, overlap, after-prev, with-offset, absolute, default), FromTo with even/odd properties, Stagger with zero/non-literal each, Label with/without callback, connector=None reset, warning accumulation on invalid over/each, motion name collisions.

**Estimated: 7+ files, ~100 assertions**

**Phase 4: @for rendering tests**

15. `emit_list_state_init` -- 1 test file, ~15 assertions. Template HTML format, JS effect header content, key map initialization, empty state handling, state binding clear/reset.
16. `emit_list_dom` -- 1 test file, ~20 assertions. Keyed vs non-keyed iteration, text/class/attr pre-compiled bindings, per-instance state setup, FLIP before/after presence, leave animation integration, event delegation setup, rebind function emission.
17. Integration test: full `emit_list_rendering_html` round-trip combining state_init + dom.

**Estimated: 2+ files, ~35 assertions**

**Phase 5: Guard/utility tests**

18. `project_uses_motion` -- 1 test file, ~8 assertions. Motions only in module vs component, timeline-only, hero-only, nested @if/for bodies.
19. `view_tree_has_motion` -- 1 test file, ~10 assertions. Recursive edge cases.
20. `collect_for_events` -- 1 test file, ~6 assertions. Duplicate events, nested @for, events in conditional fallback.

**Estimated: 3 files, ~24 assertions**

### Test Fixtures Needed

1. **`OmniViewGenerator::test_new()`** -- already exists (mod.rs:515). Returns a generator with all fields defaulted.
2. **IR constructors** -- Constructing IR types inline is verbose. Recommend helper functions/struct-builders:
   - `fn make_keyframe(over: f64, ...) -> KeyframeIR` with sensible defaults
   - `fn make_stop(pos: f64, props: &[(&str, f64)]) -> KeyframeStopIR`
   - `fn make_trigger(kind: TriggerKind, motion_ref: &str) -> TriggerIntent`
   - `fn make_timeline_item(kind: TimelineItemKindIR) -> TimelineItemIR`
3. **Snapshot approach** -- For state-mutating functions, assert on the output string contains specific JS patterns rather than exact full-string matches. E.g., `assert!(gen.js.contains("__omni_motion_myMotion"))`.
4. **Warning assertions** -- For timeline tests, check `gen.warnings` vector after calling `emit_timeline_js` with invalid inputs.

### Total Estimated Test Count

| Phase | Files | Assertions |
|---|---|---|
| Phase 1: Pure functions | 3 | ~40 |
| Phase 2: Read-only conversions | 3 | ~55 |
| Phase 3: State-mutating motion | 7+ | ~100 |
| Phase 4: @for rendering | 2+ | ~35 |
| Phase 5: Guards/utilities | 3 | ~24 |
| **Total** | **~18 files** | **~254 assertions** |

### Risk Areas Requiring Special Attention

1. **Timeline delay calculation logic** (timeline.rs:22-135): The pair-base cursor pattern, string-parsed start modes, and multi-branch delay computation are the most complex algorithm in the codebase. Should have the highest test density.
2. **Drag constraint parsing** (motion_triggers.rs:237-263): Inline JS `parseConstraint` is generated as raw JS string but the logic branches on constraint format (parent/ref:/numeric). Hard to test in Rust without running JS -- consider snapshot testing the generated JS string for each constraint format.
3. **Key-based identity tracking in @for** (list_render.rs:54-67, 174-201): Multiple Map operations (oldKeys/newKeys, _forKeys.get/set) with leave animation orchestration. Any bug here causes wrong DOM state (duplicate or missing nodes).
4. **Per-instance state push/pop in @for events** (list_render.rs:257-317): The signal save/restore pattern must maintain ordering invariants. Serialization bugs cause state pollution between @for instances.
5. **JS string escaping**: All motion/trigger/timeline code uses `format!()` with no JS string escaping. If motion names, state names, or element IDs contain special characters, the generated JS is invalid.
