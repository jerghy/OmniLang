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

The `@motion` keyword is a top-level or component-scoped definition, consistent with `@component`, `@store`, etc.

### 2.2 Two Mutually Exclusive Modes

A `@motion` block is **either** a keyframe animation **or** a timeline. Mixing the two is a compile error.

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

Compile error if:
- `start` used in keyframe mode
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

All easing parameters use `key: value` colon syntax.

| Easing | Syntax | Web Mapping | Flutter Mapping |
|--------|--------|-------------|-----------------|
| Linear | `ease: linear` | `linear` | `Curves.linear` |
| Ease In | `ease: easeIn` | `ease-in` | `Curves.easeIn` |
| Ease Out | `ease: easeOut` | `ease-out` | `Curves.easeOut` |
| Ease InOut | `ease: easeInOut` | `ease-in-out` | `Curves.easeInOut` |
| Cubic Bezier | `ease: cubic(x1: 0.16, y1: 1, x2: 0.3, y2: 1)` | `cubic-bezier()` | `Cubic()` |
| Spring | `ease: spring(tension: 200, friction: 15)` | JS spring simulation | `SpringSimulation` |
| Spring + velocity | `ease: spring(tension: 200, friction: 15, velocity: 2)` | JS spring simulation | `SpringSimulation` |
| Back | `ease: back(intensity: 1.7)` | Approximated cubic-bezier | `BackCurve` |
| Elastic | `ease: elastic(amplitude: 1, period: 0.5)` | JS elastic simulation | `ElasticCurve` |
| Bounce | `ease: bounce` | JS bounce simulation | `BounceCurve` |
| Steps | `ease: steps(count: 5, direction: end)` | `steps()` | Custom stepped curve |

Compile error: `ease` and `spring` cannot both be specified.

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
repeatDelay: 0ms           // pause between iterations
repeatRefresh: true | false  // re-evaluate from values each repeat
```

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

---

## 8. Stagger

### 8.1 Stagger Function

`stagger(motion, timing, direction?)` — creates a staggered version of any motion.

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

### 8.3 Stagger Direction (`from`)

| Value | Behavior |
|-------|----------|
| `start` | Item[0] first → Item[N-1] last |
| `center` | Item[N/2] first → ripples outward |
| `end` | Item[N-1] first → Item[0] last |
| `edges` | Item[0] and Item[N-1] first → ripples inward |

### 8.4 Stagger + `@for` + Key

When stagger is applied to `@for item in items key={item.id}`:
- Items with **new keys** (previously unseen) → play enter animation
- Items whose **keys disappeared** → play leave animation
- Items with **unchanged keys** → no animation (stay in place)
- Compile error if `@for` has stagger but no `key={}`

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
| `@unhover={motion}` | Mouse/touch leaves element | (Explicit leave animation, overrides default reverse) |
| `@press={motion}` | Element is pressed/tapped | Reverses on release |
| `@focus={motion}` | Element receives focus | Reverses on blur |
| `@animate={motion}` | Continuous play | — |
| `@in-view={motion}` | Element enters viewport | — |
| `@scroll={motion}` | Scroll position drives animation | — |
| `@drag-horizontal={motion}` | Horizontal drag gesture | — |
| `@drag-vertical={motion}` | Vertical drag gesture | — |

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

### 9.5 Multi-State Animation (`@states`)

```omni
<Panel @states={{
  idle:    to { scale: 1; opacity: 1 } over: 150ms,
  hover:   to { scale: 1.05; opacity: 1 } over: 150ms,
  pressed: to { scale: 0.95; opacity: 0.8 } over: 100ms
}}>
```

The active state is determined by element interaction state (idle/hover/pressed). This is syntactic sugar for `@hover` + `@press` + `@unhover` combinations.

### 9.6 `@scroll` (Scroll-Scrubbed Animation)

```omni
<div @scroll={fadeIn}
     @scroll-scrub={0.5}                          // smoothing delay (seconds)
     @scroll-start="top 80%"                      // trigger range start
     @scroll-end="bottom 20%">                    // trigger range end
```

The scroll position within the trigger range maps 0→1 to animation progress.

### 9.7 `@drag` (Gesture-Driven Animation)

```omni
<Card @drag-horizontal={swipeCard}
      @drag-scrub={true}                          // drag position scrubs animation
      @drag-momentum={friction: 0.95}             // inertia on release
      @drag-threshold={30%}                       // snap threshold
      @drag-velocity-max={2000px}>                // velocity cap
```

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

## 10. `@if` + Animation: Double-Branch Coexistence

When `@if condition` flips, both the old branch and new branch coexist in DOM during the animation transition:

1. Old branch starts its `@leave` animation (stays in DOM)
2. New branch starts its `@enter` animation (inserted into DOM immediately)
3. Old branch is removed from DOM after its `@leave` animation completes
4. New branch animation runs independently

```omni
@if isOpen {
  <Modal @enter={fadeIn} @leave={fadeOut} />
}
// isOpen: true → false:
//   fadeOut plays (old Modal stays), new content inserted immediately
//   Modal removed from DOM after fadeOut completes
```

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
- `$velocity` — current velocity (from gesture)
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

| Error | Type |
|-------|------|
| Non-interpolatable property (e.g., `display`, `position`) | ❌ Error |
| Type mismatch (`opacity: 0→"hello"`) | ❌ Error |
| Keyword value in animation (`width: auto→300px`) | ❌ Error |
| Missing `to` after standalone `from` | ❌ Error |
| `set` outside of timeline | ❌ Error |
| Mixed keyframe + timeline mode | ❌ Error |
| Undefined motion reference | ❌ Error |
| Circular reference (`A then B then A`) | ❌ Error |
| `@for` with stagger/enter/leave but no `key={}` | ❌ Error |
| `driven-by` + `loop` | ❌ Error |
| `ease` + `spring` simultaneously | ❌ Error |
| `fill: both` + `loop: infinite` | ❌ Error |
| Platform-unsupported property (e.g., `boxShadow` on Flutter) | ⚠️ Warning |
| `over: 0ms` | ⚠️ Warning |
| >10 animations on single element | ⚠️ Warning |
| `loop: infinite` without reachable `kill()` | ⚠️ Warning |

---

## 21. Easing → Platform Mapping

| OmniView Easing | Web (WAAPI/CSS) | Flutter |
|------------------|------------------|---------|
| `linear` | `linear` | `Curves.linear` |
| `easeIn` | `ease-in` | `Curves.easeIn` |
| `easeOut` | `ease-out` | `Curves.easeOut` |
| `easeInOut` | `ease-in-out` | `Curves.easeInOut` |
| `cubic(x1,y1,x2,y2)` | `cubic-bezier(x1,y1,x2,y2)` | `Cubic(x1,y1,x2,y2)` |
| `spring(t,f)` | JS spring simulation (rAF loop) | `SpringSimulation(t,f)` |
| `spring(t,f,v)` | JS spring simulation with velocity | `SpringSimulation(t,f)` + velocity |
| `back(intensity)` | Approximated `cubic-bezier()` | `BackCurve(intensity)` |
| `elastic(a,p)` | JS elastic simulation | `ElasticCurve(a,p)` |
| `bounce` | JS bounce simulation | `BounceCurve` |
| `steps(count,dir)` | `steps(count,dir)` | Custom stepped `Curve` |

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
motion_body     := keyframe_body | timeline_body
keyframe_body   := (keyframe_stop)+ timing_spec?
keyframe_stop   := ("from" | NUMBER "%") property_block ("ease:" easing)?
                 | "to" property_block ("ease:" easing)?
timeline_body   := timeline_item (connector timeline_item)*
connector       := "then" | "with"    // then = sequential, with = parallel
timeline_item   := "set" property_block
                 | direction property_block timing_spec ("start:" position)?
                 | IDENT ("start:" position)?                         // motion reference
                 | "stagger" "(" motion_expr "," stagger_opts ")"
                 | "label" STRING ("onEnter:" expr_block)?
                 | callback_spec
timing_spec     := "over:" duration ("ease:" easing)? ("delay:" duration)? ("colorSpace:" IDENT)?
direction       := "to" | "from" "to"
position        := STRING  // ">" | "<" | "-=Ns" | "<+Ns" | "+=Ns" | NUMBER "s"
stagger_opts    := stagger_timing "," stagger_dir
stagger_timing  := "each:" duration | "amount:" duration
stagger_dir     := "from:" ("start" | "center" | "end" | "edges")
easing          := "linear" | "easeIn" | "easeOut" | "easeInOut"
                 | "cubic" "(" cubic_params ")"
                 | "spring" "(" spring_params ")"
                 | "back" "(" "intensity:" NUMBER ")"
                 | "elastic" "(" "amplitude:" NUMBER "," "period:" NUMBER ")"
                 | "bounce"
                 | "steps" "(" "count:" NUMBER "," "direction:" IDENT ")"
property_block  := "{" property (";" property)* ";"? "}"
property        := IDENT ":" value ("→" value)?
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
- Animation control API (`play`, `pause`, `seek`, etc.)
- Compile-time validation (all errors)
- Web codegen (WAAPI + JS runtime)
- Flutter codegen (AnimationController + Tween)

### Phase 2: Advanced
- `@in-view` trigger
- `@scroll` scrub animation
- `@drag-horizontal` / `@drag-vertical` + momentum
- `@driven-by` signal binding
- `@states` multi-state mapping
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
