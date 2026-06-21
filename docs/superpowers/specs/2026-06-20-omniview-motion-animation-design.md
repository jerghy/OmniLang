# OmniView Motion Animation System — Design Spec

**Date:** 2026-06-20
**Status:** Design Complete
**Author:** OmniLang Team

---

## 1. Overview

OmniView Motion is a cross-platform animation syntax that compiles to **Web** (WAAPI + JS runtime) and **Flutter** (AnimationController + Tween). It takes design inspiration from GSAP's tween-centric flexibility while being a language-native feature with strict compile-time validation.

### Design Principles

1. **Tween-driven, not keyframe-driven** — Most animations are "go from A to B". Multi-stop keyframes are the exception, not the rule.
2. **Everything is a composable motion** — A single `@motion` primitive covers transitions, keyframes, springs, and staggers.
3. **Parser-friendly** — Every token has an unambiguous first character determining the parse path (LL(1)-compatible).
4. **Zero runtime errors** — All type mismatches, invalid properties, parameter conflicts, and circular references are caught at compile time.
5. **Platform-abstract** — Syntax maps cleanly to both WAAPI/CSS and Flutter AnimationController.

---

## 2. Motion Definition

### 2.1 Basic Syntax

```
@motion NAME {
  <body>
}
```

The `@motion` keyword can be defined at:

| Scope | Allowed | Example |
|-------|---------|---------|
| File top-level | ✅ | `@motion globalFade { ... }` |
| Inside `component` | ✅ | Component-scoped, shadows file-level names |
| Inside `store` | ❌ | Compile error |
| Inside another `@motion` | ❌ | No nesting; use composition via reference |
| Imported from another file | ✅ | `import { fadeIn } from "./animations.omni"` |

Name resolution: current scope → parent scope → file-level → imports.

### 2.2 Two Mutually Exclusive Modes

A `@motion` block is **either** a keyframe animation **or** a timeline **or** a pure-callback motion. Mixing features from different modes is a compile error.

**Mode determination at parse time:**

| First token(s) in body | Mode | Description |
|------------------------|------|-------------|
| `from`, `to`, `N%` | Keyframe | Must have NO timeline features |
| `set`, IDENT reference, `stagger(`, `label` | Timeline | Must have NO percentage stops |
| `onStart`, `onUpdate`, `onComplete`, `onRepeat` | Callback | Only callbacks + `over`; no visual tweens |

If the body contains `then`, `with`, or `start:`, it is forced to timeline mode.

**The `over:` position is the primary mode discriminator at parse time:**

| `over:` Position | Mode | Example |
|------------------|------|---------|
| Immediately after `}` (same line as property block) | **Timeline item** | `to { opacity: 1 } over: 300ms` |
| At block level (after ALL stops, before `}`) | **Keyframe** | `from {...} to {...} over: 300ms` |

Parser rule: after parsing a property block `{...}`, if the next token is `over:`, this is a timeline item. If the next token is another stop (`to`/`from`/`N%`) or `then`/`with`/playback props, the parser accumulates all stops and expects block-level `over:` later.

The same rule applies to `delay:` — item-level `delay:` means timeline mode.

#### Keyframe Mode (shared `over`)

Uses `from`/`to` and/or percentage stops. One shared `over` duration. Each stop may have an independent `ease`.

```omni
@motion bounce {
  0%   { y: 0 }      ease: easeOut
  40%  { y: -30px }  ease: easeIn
  60%  { y: -15px }  ease: easeOut
  100% { y: 0 }
  over: 600ms
}

@motion fadeIn {
  from { opacity: 0; y: 20px }
  to   { opacity: 1; y: 0 }
  over: 300ms
  ease: easeOut           // global ease (when per-stop ease omitted)
}
```

- `from` = syntactic sugar for `0%`
- `to` = syntactic sugar for `100%`
- `from` **MUST** be paired with `to` (standalone `from` is a compile error)
- `to` alone is allowed: animate from element's current value to target
- Percentages and `from`/`to` can be mixed

#### Timeline Mode (independent `over` per item)

Each item has its own `over` duration and optional `start` position. No percentage keyframes.

```omni
@motion entrance {
  to { opacity: 1 }            over: 300ms ease: easeOut
  then to { y: 0 }             over: 200ms                    // after previous
  then to { scale: 1.05 }      over: 150ms start: "-=0.1s"   // overlap 0.1s
  with to { x: 20 }            over: 200ms                    // parallel with previous
  then to { scale: 1 }         over: 200ms start: "<+0.3s"    // 0.3s after previous started
}
```

### 2.3 Direction Primitives

| Primitive | Meaning | Compile Error If |
|-----------|---------|------------------|
| `to { props }` | Animate TO target values FROM current state | — |
| `from { props } to { props }` | Explicit both ends | — |
| `from { props }` (alone) | — | ❌ Must pair with `to` |
| `set { props }` | Instant assignment (no animation) | ❌ Only allowed in timeline |

#### Callback-Only Mode

A motion with no visual tweens — drives logic purely via callbacks.

```omni
@motion countUp {
  onUpdate: { displayValue = floor($progress * 100) }
  over: 1000ms
}
```

Callbacks are the only allowed body items; `to`/`from`/`set`/`stagger` are compile errors in this mode.

### 2.4 `set` — Instant Assignment

`set` can only appear inside a timeline. It cannot be a standalone motion.

```omni
@motion flyIn {
  set { opacity: 0; y: 50px; scale: 0.9 }   // instant reset
  then to { opacity: 1; y: 0; scale: 1 } over: 400ms ease: easeOut
}
```

---

## 3. Timeline Position System (GSAP-inspired)

Each item in a timeline has an optional `start` parameter. Default is `">"` (after previous).

| `start` Value | Meaning | GSAP Equivalent |
|---------------|---------|-----------------|
| `">"` (default) | Start after previous item ends | default |
| `"<"` | Start at same time as previous | `"<"` |
| `"-=Ns"` | Start N seconds before previous ends (overlap) | `"-=N"` |
| `"<+Ns"` | Start N seconds after previous started | `"<N"` |
| `"+=Ns"` | Start N seconds after previous ends | `"+=N"` |
| `N.Ns` (number) | Absolute time from timeline start | Absolute position |

**`with` grouping rule:** When items are connected by `with`, they form a group. A subsequent `then` starts after the *entire group* completes (i.e., after the longest item in the group ends).

```omni
to { x: 100 } over: 1s
with to { y: 100 } over: 0.5s
then to { scale: 2 } over: 0.3s    // starts at 1s (max(1s, 0.5s)), not 0.5s
```

**Implicit connector rule:** In timeline mode, items separated by a newline without an explicit `then`/`with` connector default to `then` (sequential, `start: ">"`). `then` is an optional readability keyword; omitting it produces identical behavior.

Compile error if:
- `start` used in keyframe or callback mode
- `start` references a non-existent item
- Circular reference via `start: "<"` chains

---

## 4. Timing & Easing

### 4.1 Duration & Delay

```omni
over: 300ms        // milliseconds (required)
delay: 100ms       // milliseconds (optional, default: 0ms)
```

Values are snapshotted at animation start. Signal references are evaluated once at start time, not tracked during animation.

### 4.2 Easing Catalog

All easing functions use GSAP-style naming: `{Family}{Direction}` with camelCase. Direction defaults to `Out` when omitted. Parameterized curves use colon syntax for arguments.

#### 4.2.1 Standard Presets (no parameters)

| Easing | Web Mapping | Flutter Mapping |
|--------|-------------|-----------------|
| `ease: linear` | `linear` | `Curves.linear` |
| `ease: easeIn` | `ease-in` | `Curves.easeIn` |
| `ease: easeOut` | `ease-out` | `Curves.easeOut` |
| `ease: easeInOut` | `ease-in-out` | `Curves.easeInOut` |

`easeIn`/`easeOut`/`easeInOut` are CSS-compatible cubic-bezier presets.

#### 4.2.2 Power Curves (intensity 1–4, no parameters)

| Easing | Meaning | Web | Flutter |
|--------|---------|-----|---------|
| `power1In`, `power1Out`, `power1InOut` | Quadratic (t²) — subtle | cubic-bezier | `Curves.easeInQuad` etc. |
| `power2In`, `power2Out`, `power2InOut` | Cubic (t³) — moderate | cubic-bezier | `Curves.easeInCubic` etc. |
| `power3In`, `power3Out`, `power3InOut` | Quartic (t⁴) — strong | cubic-bezier | `Curves.easeInQuart` etc. |
| `power4In`, `power4Out`, `power4InOut` | Quintic (t⁵) — extreme | cubic-bezier | `Curves.easeInQuint` etc. |

#### 4.2.3 Specialized Curves (Phase 1)

| Family | Direction Variants | Parameters | Web | Flutter |
|--------|-------------------|------------|-----|---------|
| `backOut` | `backIn`, `backOut`, `backInOut` | `intensity:` (default: 1.70158) | Approx. cubic-bezier | `BackCurve` |
| `bounceOut` | `bounceIn`, `bounceOut`, `bounceInOut` | none | JS bounce simulation | `BounceCurve` |
| `elasticOut` | `elasticIn`, `elasticOut`, `elasticInOut` | `amplitude:` (default: 1), `period:` (default: 0.5) | JS elastic simulation | `ElasticCurve` |
| `spring` | (no direction — physics-based) | `tension:`, `friction:`, `velocity:` (optional) | JS spring simulation | `SpringSimulation` |
| `cubic` | (custom bezier) | `x1:`, `y1:`, `x2:`, `y2:` | `cubic-bezier()` | `Cubic()` |
| `steps` | (stepped) | `count:`, `direction:` (`start`\|`end`) | `steps()` | Custom stepped Curve |

#### 4.2.4 Future Curves (Phase 2)

| Family | Direction Variants | Parameters |
|--------|-------------------|------------|
| `sineOut` | `sineIn`, `sineOut`, `sineInOut` | none |
| `circOut` | `circIn`, `circOut`, `circInOut` | none |
| `expoOut` | `expoIn`, `expoOut`, `expoInOut` | none |

#### 4.2.5 Backward-Compatible Aliases

| Old Syntax | Maps To | Notes |
|-----------|---------|-------|
| `ease: back(intensity: 1.7)` | `backOut(intensity: 1.7)` | Default direction = Out |
| `ease: elastic(amplitude: 1, period: 0.5)` | `elasticOut(...)` | Default direction = Out |
| `ease: bounce` | `bounceOut` | Default direction = Out |

Compile error if: `ease` and `spring` both specified; unknown easing name.

### 4.3 Color Interpolation Mode

```omni
colorSpace: rgb    // default
colorSpace: hsl
colorSpace: oklch
```

- **Web**: When `colorSpace` is `hsl` or `oklch`, the runtime uses JS-per-frame color interpolation (WAAPI only supports RGB natively).
- **Flutter**: Uses `HSLColor.lerp` or `ColorTween`.

### 4.4 timeScale

```omni
@motion fastPulse {
  to { scale: 1.1 } over: 300ms
  timeScale: 2.0        // 2× speed
}
```

Global timeScale via CSS variable `--omni-motion-speed` (Web) or theme (Flutter).

---

## 5. Playback Control

```omni
loop: 3 | infinite         // iteration count
direction: normal | reverse | alternate | alternateReverse
fill: none | forwards | backwards | both
delay: 0ms
paused: true | false       // start in paused state (default: false)
repeatDelay: 0ms           // pause between iterations
repeatRefresh: true | false  // re-evaluate from values each repeat
```

When `paused: true`, the animation is fully initialized (including applying `from` state via `fill`) but does not auto-play. Use `ref.trigger.play()` to start. Per-element override: `<div @enter={fadeIn} @paused={false}>` overrides the motion's `paused` value.

### Parameter Conflict Rules (Compile Errors)

| Conflict | Error |
|----------|-------|
| `driven-by` used + `loop` | ❌ Mutually exclusive |
| `driven-by` used + `direction` | Ignored (not an error) |
| `fill: both` + `loop: infinite` | ❌ fill has no meaning with infinite loop |
| `direction: alternate` + `loop: 1` | ⚠️ Warning: 1 iteration with alternate = play forward then reverse |

---

## 6. Animatable Properties

### 6.1 Property Type System

Every animatable property has a strict type. Type mismatches are compile errors.

| Type | Properties | Example |
|------|-----------|---------|
| **float** | `opacity`, `scale`, `scaleX`, `scaleY`, `scaleZ` | `0→1`, `1→1.5` |
| **length** | `x`, `y`, `translateZ`, `width`, `height`, `borderRadius`, `margin`, `padding` | `0→100px`, `0%→50%` |
| **angle** | `rotate`, `skewX`, `skewY` | `0deg→360deg` |
| **color** | `color`, `backgroundColor`, `borderColor`, `fill`, `stroke` | `#ff0000→#00ff00` |
| **SVG** | `strokeWidth`, `strokeDashoffset`, `r`, `cx`, `cy`, `d` | `1→5`, `100%→0` |
| **filter** | `blur`, `brightness`, `contrast`, `saturate` | `0px→10px`, `1→1.5` |
| **shadow** | `boxShadow` | `"0 0 0..."→"0 4px 12px..."` |
| **clip** | `clipPath` | `"circle(0%)"→"circle(100%)"` |
| **3D** | `perspective`, `rotateX`, `rotateY`, `rotateZ`, `translateZ`, `scaleZ` | `1000px`, `0deg→180deg` |

### 6.2 Compile-Time Type Errors

```omni
// ❌ display: none→block           → display is not interpolatable
// ❌ opacity: 0→"hello"            → type mismatch: float vs string
// ❌ width: auto→300px             → auto is a keyword, not a length
// ⚠️  boxShadow: ...→... (Flutter) → platform warning: Flutter does not support boxShadow
```

**Additional format validation rules:**

| Property | Rule | Error Type |
|----------|------|------------|
| `boxShadow` | Both sides must use identical value structure (all values with/without `rgba`, same spread format) | ❌ Error |
| `clipPath` | Both sides of `→` must use the same CSS shape function (e.g., both `circle()`, both `inset()`) | ❌ Error |
| `d` (SVG path) | Both path strings must have identical command structure (same number and types of segments) | ⚠️ Warning (runtime fallback: skip this property if interpolation fails) |
| `$velocity` | Only valid in callbacks of `@drag-horizontal`/`@drag-vertical` triggered motions | ❌ Error in other contexts |

### 6.3 Transform Composition (Web)

`x`, `y`, `scale`, `rotate` etc. are compiled into a single CSS `transform` property. The compiler ensures `from` and `to` have identical transform component sets, auto-filling missing components with defaults:

- Missing `x`/`y` → `0px`
- Missing `scale` → `1`
- Missing `rotate` → `0deg`

### 6.4 Relative Values

```omni
to { scale: +=0.05 }     // add to current value
to { x: -=20px }         // subtract from current value
to { scale: *=1.1 }      // multiply current value
```

---

## 7. Composition: `then` / `with`

Motions can be composed by referencing named motions or inline tweens.

```omni
@motion cardEnter {
  fadeIn                                             // reference named motion
  then to { scale: 1.05 } over: 150ms               // inline tween
  then stagger(listItem, each: 60ms)                 // stagger reference
}
```

- `then`: sequential (same as default `start: ">"`)
- `with`: parallel (same as `start: "<"`)
- Motion inheritance via `extends` is NOT supported — use composition via reference instead.

### Composition Rules

1. Timeline items are parsed top-to-bottom
2. Default position is `">"` (after previous)
3. `start` position overrides default
4. A motion reference used in composition cannot reference itself (circular reference → compile error)

### 7.2 `@timeline` — Cross-Element Orchestration

`@timeline` orchestrates animations across **multiple elements** within the same component tree. Each line references a child element by its `id` attribute and the motion to apply. The syntax follows the same `then`/`with`/`start` position system as `@motion` timeline mode.

```omni
@timeline pageLoad {
  header    fadeIn             over: 300ms ease: easeOut
  then cards   listEnter      start: "-=0.1s"
  then footer  fadeIn
}
```

**How it works:**
- Each line: `<id> <motion>` — matches a child element with that `id` attribute
- The compiler looks up `id="header"`, `id="cards"`, `id="footer"` in the component's view tree
- At runtime: all matched elements play their assigned motion in the specified order
- `then`/`with`/`start:` work identically to `@motion` timeline mode

**Cross-platform mapping of `id`:**

| Platform | `id="header"` compiles to | Runtime lookup |
|----------|--------------------------|----------------|
| Web | `id="header"` HTML attribute | `document.getElementById("header")` |
| Flutter | `Key("header")` on widget constructor | `GlobalKey` / widget tree search |

**Compile-time validation:**
- Every `id` referenced in `@timeline` must exist as an `id` attribute on a child element → compile error if missing
- The referenced element must have the corresponding motion trigger (e.g., `@enter={fadeIn}`) → compile error if missing
- An `id` can only appear once in a `@timeline` → compile error on duplicate
- Circular dependency between `@timeline` blocks → compile error

**Usage on element:**

```omni
<div @timeline={pageLoad}>
  <Header id="header"  @enter={fadeIn} />
  <CardList id="cards" @enter={listEnter} />
  <Footer id="footer"  @enter={fadeIn} />
</div>
```

The `@timeline` attribute on the parent element tells the runtime to play the timeline when the parent mounts (equivalent to `@enter` for the orchestrated group). The `@timeline` triggers the child animations in the specified order — the children's individual `@enter` triggers are suppressed and instead controlled by the timeline.

**Differences from `@motion` timeline:**
- `@motion` timeline operates on a single element's properties over time
- `@timeline` orchestrates multiple elements, each running its own pre-defined `@motion`
- `@timeline` items reference elements by `id`, not properties

**@timeline with other triggers:**

```omni
// Scroll-driven timeline
<div @scroll={pageLoad} @scroll-scrub={0.5} @scroll-start="top 80%" @scroll-end="bottom 20%">
  <Section1 id="s1" @enter={fadeIn} />
  <Section2 id="s2" @enter={slideUp} />
</div>
```

---

## 8. Stagger

### 8.1 Stagger Function

`stagger(motion, timing, direction?)` — creates a staggered version of any motion. Valid in two contexts: as a timeline item inside `@motion`, and as an inline expression on element triggers (`@enter={stagger(...)}`). Nested stagger is allowed: `stagger(stagger(fadeIn, each: 30ms), each: 100ms)` — the outer stagger wraps the inner stagger's timing.

```omni
stagger(
  from { opacity: 0; y: 20 } to { opacity: 1; y: 0 } over: 400ms,
  each: 60ms,          // interval mode: delay between each item
  from: center          // stagger direction
)

stagger(
  fadeIn,
  amount: 600ms,        // distribute mode: spread all items evenly over 600ms
  from: edges
)
```

### 8.2 Stagger Timing Modes

| Mode | Parameter | Meaning |
|------|-----------|---------|
| **Interval** | `each: Nms` | Each item starts Nms after the previous one starts |
| **Distribute** | `amount: Nms` | All items uniformly distributed within Nms |

Total stagger duration (interval mode): `(count - 1) × each + motion.duration`

When `stagger()` appears as a timeline item, subsequent `then` items start after the *last* staggered item completes (at total stagger duration from the stagger's own start time).

### 8.3 Stagger Direction (`from`)

| Value | Behavior |
|-------|----------|
| `start` | Item[0] first → Item[N-1] last |
| `center` | For odd N: item[floor(N/2)] starts first, ripples outward. For even N: items[floor((N-1)/2)] and items[ceil((N-1)/2)] both start at t=0 |
| `end` | Item[N-1] first → Item[0] last |
| `edges` | Item[0] and Item[N-1] both start at t=0, ripples inward |

### 8.4 Stagger + `@for` + Key

When stagger is applied to `@for item in items key={item.id}`:
- Items with **new keys** (previously unseen) → play enter animation
- Items whose **keys disappeared** → play leave animation
- Items with **unchanged keys** → no animation (stay in place)
- Compile error if `@for` has stagger but no `key={}`

### 8.5 Parent-Child Orchestration

Three optional parameters on `@enter`/`@leave` triggers control the timing order between a parent element and its children's animations.

#### `when` — Play Order

```omni
<ul @enter={
  to { opacity: 1 } over: 300ms
  when: beforeChildren     // parent FIRST, then children
}>
```

| Value | Behavior |
|-------|----------|
| `withChildren` (default) | Parent and children play simultaneously |
| `beforeChildren` | Parent animation completes fully, THEN all child `@enter` animations begin |
| `afterChildren` | All child `@enter` animations complete fully, THEN parent begins |

Compile error if `when` appears on `@motion` definitions or on triggers other than `@enter`/`@leave`.

#### `delayChildren` — Extra Delay Before Children

```omni
<div @enter={
  to { opacity: 1 } over: 300ms
  when: beforeChildren
  delayChildren: 100ms    // 100ms AFTER parent finishes, children start
}>
```

With default `when: withChildren`: children start `delayChildren` after the parent's animation begins (not after it ends). Stacks with `when`: if `when: beforeChildren` + `delayChildren: 100ms`, children start 100ms after parent finishes.

#### `staggerChildren` — Auto-Stagger Shorthand

Convenience for staggering all direct children without explicit `stagger()` on each child:

```omni
<ul @enter={
  to { opacity: 1; y: 0 } over: 300ms
  staggerChildren: 60ms        // stagger each child by 60ms
  staggerFrom: center           // optional direction (default: start)
}>
  @for item in items key={item.id} {
    <li @enter={fadeIn} />     // children DON'T need explicit stagger()
  }
</ul>
```

Equivalent to wrapping each child's `@enter` in `stagger(motion, each: 60ms)`.

**Interaction with child's own `stagger()`**: The parent's `staggerChildren` wraps the child's stagger — children start with the parent's stagger interval, and each child's own stagger plays independently within its start window. This enables nested stagger (e.g., sections stagger in, items within each section also stagger).

**Compile-time:** `when`/`delayChildren`/`staggerChildren` only valid on `@enter`/`@leave` triggers (not on `@motion` definitions, `@hover`, `@press`, etc.). `staggerChildren` warns if element has no children.

---

## 9. Element Attachment

### 9.1 Trigger Syntax

All animation triggers use `@trigger={expr}` — consistent with existing `@onClick={handler}` syntax. DSL code is NOT inside strings; the compiler sees the full AST.

### 9.2 Trigger Catalog

| Trigger | When | Reverse Behavior |
|---------|------|------------------|
| `@enter={motion}` | Element mounts into DOM | — |
| `@leave={motion}` | Element unmounts from DOM | — |
| `@hover={motion}` | Mouse/touch enters element | Automatically reverses from current progress |
| `@unhover={motion}` | Mouse/touch leaves element | (Explicit leave, overrides default reverse) |
| `@press={motion}` | Element is pressed/tapped | Reverses on release |
| `@focus={motion}` | Element receives focus | Reverses on blur |
| `@animate={motion}` | Continuous play | — |
| `@in-view={motion}` | Element enters viewport | — |
| `@scroll={motion}` | Scroll position drives animation | — |
| `@drag-horizontal={motion}` | Horizontal drag gesture | — |
| `@drag-vertical={motion}` | Vertical drag gesture | — |
| `@paused={bool}` | Per-element pause override | Overrides motion's `paused` value |
| `states: { ... }` | Multi-state animation map (colon prop) | — |
| `on: {signal}` | Signal driving `states:` active state (colon prop) | — |
| `@if-mode: "wait"` | Component-level default for @if transition mode | — |
| `@timeline={motion}` | Parent-level cross-element orchestration (matches children by `id`) | — |

### 9.3 Hover Reverse Modes

```omni
// Default: smooth reverse — reverse from current progress position
<button @hover={scale(1.05)}>

// snap: immediately jump to end state, then play leave animation
<button @hover={scaleUp} @hover-reverse={snap}>

// Custom leave animation
<button @hover={scaleUp} @unhover={scaleDown}>
```

### 9.4 `@in-view` Options

```omni
<div @in-view={fadeIn}>                          // play on enter viewport
<div @in-view={fadeIn} @in-view-once={true}>     // play only first time
<div @in-view={fadeIn} @in-view-repeat={true}>   // replay every time
<div @in-view={fadeIn} @in-view-threshold={0.5}> // fire when 50% visible
```

Web: `IntersectionObserver`. Flutter: `VisibilityDetector`.

### 9.5 State-Driven Animation (`states:` + `on:`)

Two colon-props for defining named animation states and binding a driving signal. Follows the existing `style:`, `class:`, `ref:` colon-prop pattern.

#### Interaction States (reserved names)

Reserved state names `idle`, `hover`, `pressed` auto-bind to mouse/touch events — no `on:` needed:

```omni
<Panel states: {
  idle:    to { scale: 1;   opacity: 1 }   over: 150ms,
  hover:   to { scale: 1.05; opacity: 1 }   over: 150ms,
  pressed: to { scale: 0.95; opacity: 0.8 } over: 100ms
}>
```

Compiler auto-generates mouseenter/mouseleave/mousedown/mouseup bindings for the reserved names.

#### Custom Signal-Driven States

Arbitrary state names driven by a signal value via `on:`:

```omni
<div states: {
  loading: { opacity: 0.7; scale: 0.95 } over: 200ms ease: easeOut,
  success: { opacity: 1; scale: 1 }      over: 300ms ease: easeOut,
  error:   { opacity: 1; color: #ff0000 } over: 300ms
} on: {status}>
```

When `status.value` changes to `"success"`, the runtime captures current computed style values and transitions to the target state's properties using that state's `over`/`ease`.

**Compile-time validation:**
- State names must be unique within the block
- The block is **either** all reserved names (`idle`/`hover`/`pressed`) **or** all custom names + `on:` signal binding. Mixing reserved and custom names in the same `states:` block is a compile error, regardless of whether `on:` is present
- Custom state names require `on:` — otherwise compile error
- Reserved names with `on:` present → compile error (`on:` only valid with custom names)
- `on:` value must be a signal expression
- Signal value that doesn't match any state name → element stays at current values (no animation)
- Optional `default` state catches unmatched signal values

**Transition behavior:** When transitioning from state A to state B, properties in A but not B animate back to their element default. Properties only in B animate from current value to B's target. Properties in both animate from A's value to B's value.

### 9.6 `@scroll` (Scroll-Scrubbed Animation)

```omni
<div @scroll={fadeIn}
     @scroll-scrub={0.5}                          // smoothing delay (seconds)
     @scroll-start="top 80%"                      // trigger range start
     @scroll-end="bottom 20%">                    // trigger range end
```

The scroll position within the trigger range maps 0→1 to animation progress.

`@scroll-start` and `@scroll-end` values follow the format `"{elementEdge} {viewportPosition}"`:

| Element Edge | Viewport Position | Example | Meaning |
|-------------|-------------------|---------|---------|
| `top` | `N%` | `"top 80%"` | Element top reaches 80% from viewport top |
| `center` | `N%` | `"center 50%"` | Element center reaches viewport center |
| `bottom` | `N%` | `"bottom 20%"` | Element bottom reaches 20% from viewport top |
| `top` | `top` / `center` / `bottom` | `"top bottom"` | Element top reaches viewport bottom |

Phase 1 supports percentages and keywords only (no px values).

### 9.7 `@drag` (Gesture-Driven Animation)

```omni
<Card @drag-horizontal={swipeCard}
      @drag-scrub={true}                          // drag position scrubs animation
      @drag-momentum={friction: 0.95}             // inertia on release
      @drag-threshold={30%}                       // snap threshold
      @drag-velocity-max={2000px}                 // velocity cap
      @drag-constraints="left:0 right:300"        // drag bounds
      @drag-elastic={0.2}>                        // resistance past bounds
```

#### Drag Constraints

String-valued attribute using CSS-like `key:value` pairs:

| Format | Meaning | Valid Axes |
|--------|---------|------------|
| `"left:N right:N"` | Horizontal bounds in px | `@drag-horizontal` only |
| `"top:N bottom:N"` | Vertical bounds in px | `@drag-vertical` only |
| `"parent"` | Use parent element's bounding rect | Both |
| `"ref:NAME"` | Use named ref element's bounding rect | Both |

Compile error if:
- `left`/`right` used with `@drag-vertical` (axis mismatch)
- `top`/`bottom` used with `@drag-horizontal` (axis mismatch)
- `parent` combined with explicit edges
- `ref:NAME` where NAME doesn't match any `ref:` on a sibling element

#### Drag Elasticity

```omni
@drag-elastic={0.2}    // 0.0 = strict boundary, 1.0 = no resistance
```

| Value | Behavior |
|-------|----------|
| `0.0` (default) | Drag stops exactly at constraint. No overshoot. |
| `0.1`–`0.9` | Resistance increases past bound. Spring-back on release. |
| `1.0` | No resistance — infinite drag past bounds. |

**Evaluation order:** constraints → threshold → momentum. Elastic rebound uses spring simulation with `tension = 200 / (1 - elasticity + 0.01)`.

### 9.8 `@driven-by` (External Progress Control)

```omni
<div @animate={fadeIn} @driven-by={scrollProgress}>
```

- Animation progress = `clamp(expr, 0, 1)` every frame
- Animation never plays autonomously
- `loop` and `direction` are ignored in driven-by mode
- Multiple `@animate` can share one `@driven-by`

### 9.9 Inline Tween (Shorthand)

```omni
// Full expression inside braces — compiler sees full AST
<div @enter={to { opacity: 1; y: 0 } over: 300ms ease: easeOut}>
<div @enter={
  set { opacity: 0; x: -100px }
  then to { opacity: 1; x: 0 } over: 400ms
}>
<div @enter={stagger(to { opacity: 1 } over: 300ms, each: 60ms, from: center)}>
```

---

## 10. `@if` + Animation: Transition Modes

### 10.1 `mode` Attribute

The `mode` attribute on `@if` controls how branch-switching interacts with `@enter`/`@leave` animations:

```omni
@if isOpen mode="sync" {
  <Modal @enter={fadeIn} @leave={fadeOut} />
}
```

| Mode | Behavior | Framer Motion Equivalent |
|------|----------|--------------------------|
| `"sync"` (default) | Old branch plays `@leave`, new branch inserted immediately with `@enter`. Both coexist in DOM during transition. | `<AnimatePresence>` default |
| `"wait"` | Old branch plays `@leave` to completion → removed from DOM → new branch inserted → plays `@enter`. Sequential. | `mode="wait"` |
| `"crossfade"` (Phase 2) | Both branches in DOM simultaneously with opacity crossfade. | Custom |

### 10.2 Component-Level Default

```omni
@component App {
  @if-mode: "wait"     // applies to all @if blocks in this component
}
```

Per-`@if` `mode=` overrides the component-level `@if-mode`.

### 10.3 `sync` Mode (Default — Double-Branch Coexistence)

When `@if condition` flips in `sync` mode:

1. Old branch starts its `@leave` animation (stays in DOM)
2. New branch starts its `@enter` animation (inserted into DOM immediately)
3. Old branch is removed from DOM after its `@leave` animation completes
4. New branch animation runs independently

### 10.4 `wait` Mode (Sequential)

When `@if condition` flips in `wait` mode:

1. Old branch starts its `@leave` animation (stays in DOM)
2. Wait for `@leave` to complete
3. Remove old branch from DOM
4. Insert new branch into DOM
5. New branch starts its `@enter` animation

### 10.5 Edge Cases

- **`@if` without `@else`**: `wait` still useful — prevents instant disappearance before `@leave` can play.
- **Multiple `@else if` branches**: `mode` applies uniformly across all branch transitions. Only the currently active branch plays `@leave`.
- **Branch without `@enter`/`@leave`**: That branch appears/disappears instantly regardless of mode.
- **`mode` set but no animations on any branch**: Compiler warning — mode has no effect.
- **Dynamic `mode`**: Phase 1 only accepts string literals (`"sync"` / `"wait"`). Unknown string is a compile error.

---

## 11. `@for` + Key + Animation

### 11.1 Key Syntax

```omni
@for item in items key={item.id} {
  <Item @enter={slideIn} @leave={slideOut} />
}
```

Compile error if `@for` uses `@enter`/`@leave`/`stagger` but has no `key={}`.

### 11.2 Identity Tracking

- **New key** (not seen before) → element enters with `@enter` animation
- **Removed key** (no longer in data) → element exits with `@leave` animation, removed after completion
- **Unchanged key** → element stays, NO animation replayed
- **Key order changed** → (Phase 2: FLIP animation)

---

## 12. Shared Element Transition (`@hero`)

### 12.1 Scope

Phase 1: `@hero` only within the same page/component tree. Cross-page hero transitions are deferred.

### 12.2 Syntax

```omni
// Two elements with the same hero tag
<img src={thumb} @hero="product-image" />
<img src={full} @hero="product-image" />

// Custom hero transition
@hero product-image {
  over: 400ms
  ease: spring(tension: 200, friction: 15)
}

<img @hero="product-image" @hero-transition={product-image}>
```

When both elements exist in DOM and one is removed while the other is added, the compiler generates FLIP animation code (Web: position recording + WAAPI, Flutter: `Hero` widget).

---

## 13. Animation Control API

### 13.1 Reference via Trigger Name

```omni
<div ref:card @enter={fadeIn} @leave={fadeOut} @hover={glow}>

action demo {
  card.enter.play()           // Play from start
  card.enter.pause()          // Pause at current position
  card.enter.resume()         // Resume from paused position
  card.enter.reverse()        // Reverse direction
  card.enter.seek(0.5)        // Seek to 50% progress
  card.enter.kill()           // Terminate and clean up

  await card.enter.complete   // Await completion (Promise/Future)

  // Read-only state
  card.enter.progress         // 0→1 (float)
  card.enter.playing          // bool
  card.enter.completed        // bool
  card.enter.iteration        // current loop count (int)
}
```

### 13.2 Timeline Labels

```omni
@motion complexReveal {
  from { opacity: 0; y: 40px } over: 300ms
  label "fadedIn"

  then to { scale: 1.05 } over: 150ms
  label "popped"

  then to { scale: 1 } over: 200ms
  label "done"
}

action replayPop {
  card.complexReveal.seek("popped")    // Jump to labeled position
  card.complexReveal.play()            // Play from there
}
```

Labels can have callbacks:

```omni
label "visible" onEnter: { startTracking() }
```

---

## 14. Callbacks

```omni
@motion fadeIn {
  to { opacity: 1 } over: 300ms

  onStart:    { isAnimating = true }
  onUpdate:   { animProgress = $progress }    // $progress: 0→1
  onComplete: { isAnimating = false; count += 1 }
  onRepeat:   { repeatCount += 1 }
  onInterrupt:{ /* animation was killed */ }
}

// Also supports function references
@motion fadeIn {
  to { opacity: 1 } over: 300ms
  onStart: startAnimHandler
  onComplete: finishAnimHandler
}
```

Magic variables available in callbacks:
- `$progress` — current animation progress (0→1)
- `$velocity` — velocity at the moment of gesture release, in **pixels per second** (px/s). Only valid in `@drag-horizontal`/`@drag-vertical` triggered motion callbacks. Typically used as `spring(..., velocity: $velocity)` to carry gesture momentum into the animation. Compile error if referenced in non-gesture contexts
- `$iteration` — current loop count (for `onRepeat`)

---

## 15. Path Animation

```omni
@motion followPath {
  to { offset: 1 } over: 2s ease: easeInOut
  path: "M 0,0 C 50,-50 100,50 200,0"       // SVG path string
}

@motion curveMove {
  to { offset: 1; rotate: 360deg } over: 2s
  path: cubic(x1: 0, y1: 0, x2: 100, y2: -50, x3: 200, y3: 0)
  pathRotate: true                            // auto-orient along path direction
}
```

Web: `offset-path` + `offset-distance`. Flutter: `Path` + `PathMetric.getTangentForOffset`.

---

## 16. Interrupt Strategy

```omni
@motion fadeIn {
  to { opacity: 1 } over: 300ms
  interrupt: stop        // stop: freeze at current value (default)
                         // complete: jump to end immediately
                         // queue: wait for previous animation to finish
                         // replace: new animation kills old immediately
}
```

`@hover` defaults to `interrupt: replace` (fast interaction). All other triggers default to `interrupt: stop`.

---

## 17. Animation Cleanup

| Scenario | Behavior |
|----------|----------|
| Element unmounts mid-animation | `@leave` animation plays to completion before DOM removal; non-leave animations are killed with `onInterrupt` |
| Animation completes normally | Auto-removed from global registry, no memory leak |
| `loop: infinite` animation | Must be explicitly `kill()`-ed; compiler warns if no `kill()` path exists |
| Component with running animations unmounted | All component-scoped animations are killed when component unmounts |

---

## 18. Accessibility: `respectReducedMotion`

```omni
@motion fadeIn {
  to { opacity: 1 } over: 300ms
  respectReducedMotion: true       // default: true
}
```

When `true` and user prefers reduced motion:
- **Web**: Detected via `matchMedia('(prefers-reduced-motion: reduce)')`. All durations set to `0ms`.
- **Flutter**: Detected via `MediaQuery.of(context).disableAnimations`.

When `respectReducedMotion: false`, the animation always plays with its defined duration.

---

## 19. Compiler-Inserted Optimizations

- **`will-change`**: Automatically added for animations targeting `transform`/`opacity`. Removed 500ms after animation ends.
- **Transform composition**: `x`/`y`/`scale`/`rotate` composed into single CSS `transform` property for compositor-thread animation on Web.
- **Transform component auto-fill**: Missing transform components in `from`/`to` auto-filled with defaults for consistent WAAPI interpolation.

---

## 20. Compile-Time Validation (Complete List)

### 20.1 Errors (block compilation)

| Error |
|-------|
| Non-interpolatable property (e.g., `display`, `position`) |
| Type mismatch (`opacity: 0→"hello"`) |
| Keyword value in animation (`width: auto→300px`) |
| Missing `to` after standalone `from` |
| `set` outside of timeline |
| Mixed keyframe + timeline + callback features |
| Undefined motion reference |
| Circular reference (`A then B then A`) |
| `@for` with stagger/enter/leave but no `key={}` |
| `driven-by` + `loop` |
| `ease` + `spring` simultaneously |
| `fill: both` + `loop: infinite` |
| `clipPath` with different shape functions on each side of `→` |
| `boxShadow` with different value structures on each side of `→` |
| `$velocity` in non-gesture callback context |
| `@motion` inside `store` or inside another `@motion` |
| `stagger()` inside keyframe mode |
| Unknown easing name |
| `when` / `delayChildren` / `staggerChildren` on non-`@enter`/`@leave` trigger or on `@motion` definition |
| `states:` with custom names but no `on:` signal binding |
| `states:` mixing reserved names (`idle`/`hover`/`pressed`) with `on:` |
| `@if mode` with unknown mode string |
| `@drag-constraints` axis mismatch (e.g., `left`/`right` with `@drag-vertical`) |
| `@drag-constraints` mixing `parent` with explicit edge values |
| `@drag-constraints` with `ref:NAME` where NAME is not a sibling ref |
| `@drag-elastic` value outside 0.0..1.0 |
| `@timeline` referencing an `id` that doesn't exist in the view tree |
| `@timeline` referencing an element without the required motion trigger |
| `@timeline` with duplicate `id` references |
| Circular `@timeline` dependency |

### 20.2 Warnings (do not block compilation)

| Warning |
|---------|
| Platform-unsupported property (e.g., `boxShadow` on Flutter, `elevation` on Web) |
| `over: 0ms` (use `set` instead) |
| >10 animations on single element (performance risk) |
| `loop: infinite` without reachable `kill()` |
| `direction: alternate` + `loop: 1` (plays forward then reverse — is this intended?) |
| `d` attribute interpolation — path structure mismatch (runtime may skip) |
| `opacity` value outside 0..1 range |
| `over` > 3600s (suspiciously long) |
| `over` < 0ms (negative duration), `delay` < 0ms, `loop: 0` — treated as errors, see 20.1 |

### 20.3 Parameter Value Ranges

| Parameter | Valid Range | Out of Range |
|-----------|------------|--------------|
| `over` | 0ms .. 3600s | < 0 → ❌ Error; > 3600s → ⚠️ Warning |
| `delay` | 0ms .. 3600s | < 0 → ❌ Error |
| `loop` | 1 .. 10000 or `infinite` | 0 or negative → ❌ Error |
| `repeatDelay` | 0ms .. 3600s | < 0 → ❌ Error |
| `timeScale` | 0.0 .. 100.0 | ≤ 0 → ❌ Error; > 100 → ⚠️ Warning |
| `opacity` | 0.0 .. 1.0 | Outside → ⚠️ Warning |
| `friction` (spring) | 0 .. 1000 | Outside → ❌ Error |
| `tension` (spring) | 0 .. 1000 | Outside → ❌ Error |

---

## 21. Easing → Platform Mapping

| OmniView Easing | Web (WAAPI/CSS) | Flutter |
|------------------|------------------|---------|
| `linear` | `linear` | `Curves.linear` |
| `easeIn` / `easeOut` / `easeInOut` | `ease-in` / `ease-out` / `ease-in-out` | `Curves.easeIn` / `Curves.easeOut` / `Curves.easeInOut` |
| `power1In` / `power1Out` / `power1InOut` | `cubic-bezier(...)` (quad) | `Curves.easeInQuad` etc. |
| `power2In` / `power2Out` / `power2InOut` | `cubic-bezier(...)` (cubic) | `Curves.easeInCubic` etc. |
| `power3In` / `power3Out` / `power3InOut` | `cubic-bezier(...)` (quart) | `Curves.easeInQuart` etc. |
| `power4In` / `power4Out` / `power4InOut` | `cubic-bezier(...)` (quint) | `Curves.easeInQuint` etc. |
| `backIn` / `backOut` / `backInOut(intensity:)` | Approx. `cubic-bezier()` | `BackCurve` |
| `bounceIn` / `bounceOut` / `bounceInOut` | JS bounce simulation | `BounceCurve` |
| `elasticIn` / `elasticOut` / `elasticInOut(amplitude:,period:)` | JS elastic simulation | `ElasticCurve` |
| `spring(tension:,friction:,velocity?:)` | JS spring simulation (rAF) | `SpringSimulation` |
| `cubic(x1:,y1:,x2:,y2:)` | `cubic-bezier()` | `Cubic()` |
| `steps(count:,direction:)` | `steps()` | Custom stepped `Curve` |
| `sineIn` / `sineOut` / `sineInOut` (Phase 2) | `cubic-bezier(...)` | `Curves.easeInSine` etc. |
| `circIn` / `circOut` / `circInOut` (Phase 2) | `cubic-bezier(...)` | `Curves.easeInCirc` etc. |
| `expoIn` / `expoOut` / `expoInOut` (Phase 2) | `cubic-bezier(...)` | `Curves.easeInExpo` etc. |

---

## 22. Property → Platform Mapping

| OmniView Property | Web CSS | Flutter |
|-------------------|---------|---------|
| `opacity` | `opacity` | `Opacity` widget or `Color.withOpacity` |
| `x`, `y` | `transform: translate()` | `Transform.translate` |
| `scale` | `transform: scale()` | `Transform.scale` |
| `rotate` | `transform: rotate()` | `Transform.rotate` |
| `rotateX/Y/Z` | `transform: rotateX/Y/Z()` | `Transform` (Matrix4) |
| `translateZ` | `transform: translateZ()` | `Transform` (Matrix4) |
| `perspective` | `perspective` | `Transform` (Matrix4) |
| `width`, `height` | `width`, `height` | `SizedBox` or `Container` |
| `color`, `backgroundColor` | `color`, `background-color` | `Color` in `Container` or `Text` |
| `borderColor` | `border-color` | `Border` in `Container` |
| `borderRadius` | `border-radius` | `BorderRadius` in `Container` |
| `margin`, `padding` | `margin`, `padding` | `Padding` or `Container` margin |
| `blur` | `filter: blur()` | `BackdropFilter` |
| `brightness` | `filter: brightness()` | `ColorFilter` |
| `boxShadow` | `box-shadow` | ⚠️ Not supported in Flutter |
| `clipPath` | `clip-path` | `ClipPath` |
| `strokeDashoffset` | `stroke-dashoffset` | `CustomPaint` |
| `fill` | `fill` | `CustomPaint` |
| `stroke` | `stroke` | `CustomPaint` |
| `d` (path data) | `d` | `CustomPaint` (Path) |

---

## 23. Grammar Sketch

```
motion_def      := "@motion" IDENT "{" motion_body "}"

motion_body     := keyframe_body | timeline_body | callback_body

keyframe_body   := (keyframe_stop)+ timing_spec? playback_spec*
keyframe_stop   := ("from" | NUMBER "%") property_block ("ease:" easing)?
                 | "to" property_block ("ease:" easing)?
// Mode detection: "from" | "to" | N"%" as first token → keyframe mode
// Mode detection: presence of "then" | "with" | "start:" → forced to timeline mode

timeline_body   := timeline_item (connector timeline_item)* playback_spec*
connector       := "then" | "with"    // then = sequential, with = parallel
timeline_item   := "set" property_block
                 | direction property_block timing_spec ("start:" position)?
                 | IDENT ("start:" position)?                         // motion reference
                 | "stagger" "(" motion_expr "," stagger_opts ")"
                 | "label" STRING ("onEnter:" expr_block)?
// "then" after a "with" group: starts after longest item in group ends

callback_body   := callback_spec+ "over:" duration playback_spec*
// No visual tweens; only callbacks + timing

timing_spec     := "over:" duration ("ease:" easing)? ("delay:" duration)? ("colorSpace:" IDENT)?
direction       := "to" | "from" "to"
position        := STRING  // ">" | "<" | "-=Ns" | "<+Ns" | "+=Ns" | NUMBER "s"
stagger_opts    := stagger_timing "," stagger_dir
stagger_timing  := "each:" duration | "amount:" duration
stagger_dir     := "from:" ("start" | "center" | "end" | "edges")
easing          := "linear" | "easeIn" | "easeOut" | "easeInOut"
                 | "power1In" | "power1Out" | "power1InOut"
                 | "power2In" | "power2Out" | "power2InOut"
                 | "power3In" | "power3Out" | "power3InOut"
                 | "power4In" | "power4Out" | "power4InOut"
                 | "backIn" | "backOut" | "backInOut" ("(" "intensity:" NUMBER ")")?
                 | "bounceIn" | "bounceOut" | "bounceInOut"
                 | "elasticIn" | "elasticOut" | "elasticInOut" ("(" elastic_params ")")?
                 | "cubic" "(" cubic_params ")"
                 | "spring" "(" spring_params ")"
                 | "steps" "(" "count:" NUMBER "," "direction:" IDENT ")"
                 // Phase 2: sineIn/sineOut/sineInOut, circIn/circOut/circInOut, expoIn/expoOut/expoInOut

cubic_params    := "x1:" NUMBER "," "y1:" NUMBER "," "x2:" NUMBER "," "y2:" NUMBER
spring_params   := "tension:" NUMBER ("," "friction:" NUMBER)? ("," "velocity:" NUMBER)?
elastic_params  := ("amplitude:" NUMBER)? ("," "period:" NUMBER)?

parent_child    := "when:" ("beforeChildren" | "afterChildren" | "withChildren")
                 | "delayChildren:" duration
                 | "staggerChildren:" duration
                 | "staggerFrom:" ("start" | "center" | "end" | "edges")
property_block  := "{" property (";" property)* ";"? "}"
property        := IDENT ":" value ("→" value)?
playback_spec   := "loop:" (NUMBER | "infinite")
                 | "direction:" ("normal" | "reverse" | "alternate" | "alternateReverse")
                 | "fill:" ("none" | "forwards" | "backwards" | "both")
                 | "delay:" duration
                 | "paused:" ("true" | "false")
                 | "repeatDelay:" duration
                 | "repeatRefresh:" ("true" | "false")
                 | "timeScale:" NUMBER
                 | "interrupt:" ("stop" | "complete" | "queue" | "replace")
                 | "respectReducedMotion:" ("true" | "false")

// Element-level props (colon syntax)
colon_props     := "states:" "{" state_entry ("," state_entry)* "}"
                 | "on:" "{" expr "}"
state_entry     := IDENT ":" property_block ("over:" duration)? ("ease:" easing)?

// @timeline — cross-element orchestration
timeline_def    := "@timeline" IDENT "{" timeline_body "}"
timeline_ref    := IDENT IDENT timing_spec? ("start:" position)?   // <id> <motion_ref> over:... start:...

// @if with mode
if_directive    := "@if" expr ("mode=" STRING)? "{" view_nodes "}"
                 ("@else" "@if" expr "{" view_nodes "}")*
                 ("@else" "{" view_nodes "}")?
callback_spec   := ("onStart:" | "onUpdate:" | "onComplete:" | "onRepeat:" | "onInterrupt:") (IDENT | expr_block)
```

---

## 24. Implementation Phasing

### Phase 1: Core Animation
- `@motion` definition (keyframe + timeline modes)
- `to` / `from to` / `set` primitives
- All easing functions including spring
- `then` / `with` / `start` position system
- `@enter` / `@leave` / `@hover` / `@press` triggers
- `stagger()` function
- `@if` double-branch coexistence
- `@for key={}` syntax + identity tracking
- Callbacks (`onStart`, `onUpdate`, `onComplete`, `onRepeat`, `onInterrupt`)
- `prefers-reduced-motion` support
- Basic `@hero` (same page)
- `@timeline` cross-element orchestration
- Animation control API (`play`, `pause`, `seek`, etc.)
- Compile-time validation (all errors)
- Web codegen (WAAPI + JS runtime)
- Flutter codegen (AnimationController + Tween)

### Phase 2: Advanced
- `@in-view` trigger
- `@scroll` scrub animation
- `@drag-horizontal` / `@drag-vertical` + momentum
- `@driven-by` signal binding
- `states:` multi-state mapping
- Path animation
- Timeline labels
- `repeatDelay` / `repeatRefresh`
- FLIP layout animation for `@for` list reordering

### Phase 3: Polish
- Cross-component / cross-page `@hero`
- Animation DevTools integration
- Global timeScale control
- `--omni-motion-speed` CSS variable
- Performance warnings (frame budget monitoring)
- HTML/CSS `will-change` auto-management

---

## 25. Open Questions (Deferred)

- Cross-component `@hero` tracking mechanism
- `matchMedia`-style responsive animation parameters
- SVG shape morphing (`d` attribute with mismatched segment counts)
- Lottie/Rive animation file import
- `TrainHoppingAnimation`-style controller handoff
- Animation recording/replay for testing
