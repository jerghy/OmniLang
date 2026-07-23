# Omni Animation Library — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract a standalone `@omni/animation` npm package (WAAPI-based) from the embedded `motion-runtime.js`, making both OmniView and React codegen share the same runtime.

**Architecture:** A ~450-line TypeScript library exposing Motion v2-style functions (`animate`, `timeline`, `buildKeyframes`, `resolveEasing`, `staggerDelays`, `flipBefore/After`, `heroBefore/After`). OmniView codegen outputs `<script src="omni-animation.js">` + calls to `Omni.animate(...)`. React codegen outputs `import { animate } from "@omni/animation"`. Both share the same IR (`KeyframeIR` / `EasingIR` / `PlaybackIR`) from the compiler. Flutter codegen is unchanged.

**Tech Stack:** TypeScript (library), Rust (codegen in `omnilang/src/codegen/`), WAAPI (runtime)

## Global Constraints

- Parser, Semantic Analysis, IR layers: **do not modify**
- Flutter codegen: **do not modify**
- Library must work as both a `<script>` global (`window.Omni`) and an ES module import
- All existing OmniView test `.omni` files must compile to functionally identical output
- React codegen regression tests (18 Playwright tests) must continue to pass

---

### Task 1: Library — Package scaffolding + types

**Files:**
- Create: `packages/omni-animation/package.json`
- Create: `packages/omni-animation/tsconfig.json`
- Create: `packages/omni-animation/src/types.ts`

**Interfaces:**
- Produces: `MotionOptions`, `EasingConfig`, `KeyframeConfig`, `KeyframeStop`, `TimelineStep` types

- [ ] **Step 1: Create package.json**

```json
{
  "name": "@omni/animation",
  "version": "0.1.0",
  "description": "WAAPI-based animation library for OmniLang compiled output",
  "main": "dist/omni-animation.js",
  "module": "dist/omni-animation.mjs",
  "types": "dist/types.d.ts",
  "files": ["dist"],
  "scripts": {
    "build": "tsc && node build-bundle.js",
    "check": "tsc --noEmit"
  },
  "devDependencies": {
    "typescript": "^5.5.0"
  },
  "license": "MIT"
}
```

- [ ] **Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "declaration": true,
    "declarationDir": "dist",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "esModuleInterop": true
  },
  "include": ["src"]
}
```

- [ ] **Step 3: Create types.ts**

```typescript
// ═══ Motion properties (Omni naming conventions) ═══
// These are the property names used in .omni @motion blocks.
// The library maps them to CSS properties.

/** Properties that become CSS transform functions. */
export type TransformProp = 'x' | 'y' | 'scale' | 'scaleX' | 'scaleY'
  | 'rotate' | 'rotateX' | 'rotateY' | 'rotateZ' | 'skewX' | 'skewY'

/** Properties that become CSS filter functions. */
export type FilterProp = 'blur' | 'brightness' | 'contrast' | 'saturate' | 'hueRotate'

/** All valid Omni animation property names. */
export type OmniProp = TransformProp | FilterProp
  | 'opacity' | 'width' | 'height' | 'backgroundColor' | 'color'
  | 'borderColor' | 'borderRadius' | 'margin' | 'padding' | 'boxShadow'
  | 'marginTop' | 'marginRight' | 'marginBottom' | 'marginLeft'
  | 'paddingTop' | 'paddingRight' | 'paddingBottom' | 'paddingLeft'
  | 'borderWidth' | 'top' | 'left' | 'right' | 'bottom'
  | 'fontSize' | 'letterSpacing' | 'offsetDistance'

/** Easing specification — string name or structured config. */
export type EasingConfig =
  | string  // "easeOut", "linear", "power2In"...
  | { type: 'cubicBezier'; x1: number; y1: number; x2: number; y2: number }
  | { type: 'spring'; tension: number; friction: number; velocity?: number }
  | { type: 'back'; intensity: number; direction?: 'in' | 'out' | 'inOut' }
  | { type: 'elastic'; amplitude: number; period: number; direction?: 'in' | 'out' | 'inOut' }
  | { type: 'steps'; count: number; direction: 'start' | 'end' }

/** A single keyframe stop in Omni terms. */
export interface KeyframeStop {
  offset: number  // 0..1
  properties: Partial<Record<OmniProp, number | string>>
  easing?: EasingConfig
}

/** A single keyframe in WAAPI terms (CSS properties only). */
export interface Keyframe {
  offset?: number
  [cssProperty: string]: string | number | undefined
}

/** Options passed to el.animate(). */
export interface MotionOptions {
  duration: number
  delay?: number
  easing?: string  // WAAPI-compatible easing string (output of resolveEasing)
  fill?: 'none' | 'forwards' | 'backwards' | 'both'
  direction?: 'normal' | 'reverse' | 'alternate' | 'alternate-reverse'
  iterations?: number   // use Infinity for loop
}

/** A single step in a timeline sequence. */
export interface TimelineStep {
  el: Element
  keyframes: KeyframeStop[]
  options: MotionOptions
  at: number  // absolute start time in milliseconds
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd packages/omni-animation && npx tsc --noEmit`
Expected: PASS (no errors)

- [ ] **Step 5: Commit**

```bash
git add packages/omni-animation/package.json packages/omni-animation/tsconfig.json packages/omni-animation/src/types.ts
git commit -m "feat(omni-animation): add package scaffolding and type definitions"
```

---

### Task 2: Library — Easing + Keyframes (pure functions)

**Files:**
- Create: `packages/omni-animation/src/easing.ts`
- Create: `packages/omni-animation/src/keyframes.ts`

**Interfaces:**
- Produces: `resolveEasing(easing: EasingConfig): string`
- Produces: `buildKeyframes(stops: KeyframeStop[]): Keyframe[]`, `buildProperties(props: Record<string, number | string>): Record<string, string>`
- Consumes: `EasingConfig`, `KeyframeStop`, `Keyframe` from types.ts (Task 1)

- [ ] **Step 1: Create easing.ts**

```typescript
import type { EasingConfig } from './types'

/** Named easing → WAAPI cubic-bezier / steps string. */
const NAMED_EASINGS: Record<string, string> = {
  linear: 'linear',
  ease: 'ease',
  easeIn: 'ease-in',
  easeOut: 'ease-out',
  easeInOut: 'ease-in-out',
  // power1 (quadratic)
  power1In: 'cubic-bezier(0.550,0.085,0.680,0.530)',
  power1Out: 'cubic-bezier(0.250,0.460,0.450,0.940)',
  power1InOut: 'cubic-bezier(0.455,0.030,0.515,0.955)',
  // power2 (cubic)
  power2In: 'cubic-bezier(0.550,0.055,0.675,0.190)',
  power2Out: 'cubic-bezier(0.215,0.610,0.355,1.000)',
  power2InOut: 'cubic-bezier(0.645,0.045,0.355,1.000)',
  // power3 (quartic)
  power3In: 'cubic-bezier(0.895,0.030,0.685,0.220)',
  power3Out: 'cubic-bezier(0.165,0.840,0.440,1.000)',
  power3InOut: 'cubic-bezier(0.770,0.000,0.175,1.000)',
  // power4 (quintic)
  power4In: 'cubic-bezier(0.755,0.050,0.855,0.060)',
  power4Out: 'cubic-bezier(0.230,1.000,0.320,1.000)',
  power4InOut: 'cubic-bezier(0.860,0.000,0.070,1.000)',
  // back
  backIn: 'cubic-bezier(0.600,-0.280,0.735,0.045)',
  backOut: 'cubic-bezier(0.175,0.885,0.320,1.275)',
  backInOut: 'cubic-bezier(0.680,-0.550,0.265,1.550)',
  // bounce (approximations)
  bounceIn: 'cubic-bezier(0.215,0.610,0.355,1.000)',
  bounceOut: 'cubic-bezier(0.215,0.610,0.355,1.000)',
  bounceInOut: 'cubic-bezier(0.645,0.045,0.355,1.000)',
  // elastic (approximations)
  elasticIn: 'cubic-bezier(0.600,0.040,0.980,0.335)',
  elasticOut: 'cubic-bezier(0.250,0.460,0.450,0.940)',
  elasticInOut: 'cubic-bezier(0.680,-0.550,0.265,1.550)',
  // spring → generic bouncy
  spring: 'cubic-bezier(0.250,0.460,0.450,0.940)',
  stepStart: 'steps(1,start)',
  stepEnd: 'steps(1,end)',
}

/**
 * Resolve an Omni EasingConfig to a WAAPI-compatible easing string.
 * All non-WAAPI-native easings (spring, back, elastic, bounce) are
 * approximated as cubic-bezier() since WAAPI does not support custom
 * easing functions.
 */
export function resolveEasing(easing: EasingConfig): string {
  if (typeof easing === 'string') {
    // Check for parameterized form: backIn(1.5), elasticOut(1,0.5), etc.
    return NAMED_EASINGS[easing] ?? resolveParameterized(easing) ?? 'ease-out'
  }

  switch (easing.type) {
    case 'cubicBezier':
      return `cubic-bezier(${easing.x1},${easing.y1},${easing.x2},${easing.y2})`

    case 'spring': {
      const t = easing.tension ?? 180
      const f = easing.friction ?? 12
      const ratio = t > 0 ? f / t : 1.0
      if (ratio < 0.3) {
        const overshoot = Math.min(0.3, 1.0 - ratio * 2.0)
        return `cubic-bezier(0.250,${(0.460 + overshoot).toFixed(3)},0.450,${(0.940 + overshoot).toFixed(3)})`
      }
      return 'cubic-bezier(0.250,0.460,0.450,0.940)'
    }

    case 'back': {
      const i = Math.min(1.5, Math.max(0.5, easing.intensity ?? 1.7))
      const d = easing.direction ?? 'out'
      if (d === 'in') return `cubic-bezier(0.600,${(-0.28 * i / 1.7).toFixed(3)},0.735,0.045)`
      if (d === 'inOut') return `cubic-bezier(0.680,${(-0.55 * i / 1.7).toFixed(3)},0.265,${Math.min(1.8, 1.55 * i / 1.7).toFixed(3)})`
      return `cubic-bezier(0.175,${(0.885 * i / 1.7).toFixed(3)},0.320,${Math.min(2.0, 1.275 * i / 1.7).toFixed(3)})`
    }

    case 'elastic': {
      const d = easing.direction ?? 'out'
      if (d === 'in') return 'cubic-bezier(0.600,0.040,0.980,0.335)'
      if (d === 'inOut') return 'cubic-bezier(0.680,-0.550,0.265,1.550)'
      return 'cubic-bezier(0.250,0.460,0.450,0.940)'
    }

    case 'steps':
      return `steps(${easing.count},${easing.direction})`

    default:
      return 'ease-out'
  }
}

/** Handle parameterized string easings like "backIn(1.5)" or "elasticOut(1,0.5)". */
function resolveParameterized(easing: string): string | null {
  const match = easing.match(/^(backIn|backOut|backInOut)\(([\d.]+)\)$/)
  if (match) {
    return resolveEasing({ type: 'back', intensity: parseFloat(match[2]), direction: match[1].replace('back', '').toLowerCase() as 'in' | 'out' | 'inOut' })
  }
  const elasticMatch = easing.match(/^(elasticIn|elasticOut|elasticInOut)\(([\d.]+),([\d.]+)\)$/)
  if (elasticMatch) {
    return resolveEasing({ type: 'elastic', amplitude: parseFloat(elasticMatch[2]), period: parseFloat(elasticMatch[3]), direction: elasticMatch[1].replace('elastic', '').toLowerCase() as 'in' | 'out' | 'inOut' })
  }
  const springMatch = easing.match(/^spring\(([\d.]+),([\d.]+)(?:,([\d.]+))?\)$/)
  if (springMatch) {
    return resolveEasing({ type: 'spring', tension: parseFloat(springMatch[1]), friction: parseFloat(springMatch[2]), velocity: springMatch[3] ? parseFloat(springMatch[3]) : undefined })
  }
  return null
}
```

- [ ] **Step 2: Create keyframes.ts**

```typescript
import type { Keyframe, KeyframeStop, OmniProp } from './types'

/** Properties that must be mapped to CSS transform functions. */
const TRANSFORM_MAP: Record<string, string> = {
  x: 'translateX', y: 'translateY',
  scale: 'scale', scaleX: 'scaleX', scaleY: 'scaleY',
  rotate: 'rotate', rotateX: 'rotateX', rotateY: 'rotateY', rotateZ: 'rotateZ',
  skewX: 'skewX', skewY: 'skewY',
}

const TRANSFORM_PROPS = new Set(Object.keys(TRANSFORM_MAP))

/** Properties that need a "px" suffix when the value is a number. */
const PX_PROPS = new Set([
  'width', 'height', 'margin', 'padding', 'borderRadius', 'borderWidth',
  'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
  'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
  'borderTopLeftRadius', 'borderTopRightRadius',
  'borderBottomLeftRadius', 'borderBottomRightRadius',
  'gap', 'rowGap', 'columnGap', 'maxWidth', 'minWidth', 'maxHeight', 'minHeight',
  'top', 'left', 'right', 'bottom', 'fontSize', 'letterSpacing',
])

/** Properties that map to CSS filter functions. */
const FILTER_MAP: Record<string, string> = {
  blur: 'blur', brightness: 'brightness',
  contrast: 'contrast', saturate: 'saturate', hueRotate: 'hue-rotate',
}

/**
 * Convert Omni animation property names + values to CSS property names + formatted values.
 * Transform properties (x, y, scale, rotate, etc.) are combined into one `transform` string.
 * Filter properties (blur, brightness, etc.) are combined into one `filter` string.
 */
export function buildProperties(
  props: Partial<Record<OmniProp, number | string>>
): Record<string, string> {
  const cssProps: Record<string, string> = {}
  const transforms: string[] = []
  const filters: string[] = []

  for (const [key, value] of Object.entries(props)) {
    if (TRANSFORM_PROPS.has(key)) {
      const cssName = TRANSFORM_MAP[key]
      const num = typeof value === 'number' ? value : parseFloat(value as string)
      if (key === 'x' || key === 'y') {
        transforms.push(`${cssName}(${num}px)`)
      } else if (key.startsWith('rotate') || key.startsWith('skew')) {
        transforms.push(`${cssName}(${num}deg)`)
      } else {
        transforms.push(`${cssName}(${num})`)
      }
    } else if (key in FILTER_MAP) {
      const cssName = FILTER_MAP[key]
      const num = typeof value === 'number' ? value : parseFloat(value as string)
      if (key === 'blur') filters.push(`${cssName}(${num}px)`)
      else filters.push(`${cssName}(${num})`)
    } else {
      // Standard CSS property
      let formatted = value
      if (typeof value === 'number' && PX_PROPS.has(key)) {
        formatted = `${value}px`
      }
      cssProps[key] = String(formatted)
    }
  }

  if (transforms.length) cssProps.transform = transforms.join(' ')
  if (filters.length) cssProps.filter = filters.join(' ')
  return cssProps
}

/**
 * Convert Omni KeyframeStop[] to WAAPI Keyframe[].
 * Handles offset → CSS property mapping, transform/filter merging.
 */
export function buildKeyframes(stops: KeyframeStop[]): Keyframe[] {
  const sorted = [...stops].sort((a, b) => a.offset - b.offset)

  return sorted.map(stop => {
    const kf: Keyframe = { offset: stop.offset }
    const cssProps = buildProperties(stop.properties)
    Object.assign(kf, cssProps)
    return kf
  })
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd packages/omni-animation && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add packages/omni-animation/src/easing.ts packages/omni-animation/src/keyframes.ts
git commit -m "feat(omni-animation): add easing resolver and keyframe builder"
```

---

### Task 3: Library — animate + timeline + extras

**Files:**
- Create: `packages/omni-animation/src/animate.ts`
- Create: `packages/omni-animation/src/timeline.ts`
- Create: `packages/omni-animation/src/stagger.ts`
- Create: `packages/omni-animation/src/scroll.ts`
- Create: `packages/omni-animation/src/flip.ts`
- Create: `packages/omni-animation/src/hero.ts`
- Create: `packages/omni-animation/src/index.ts`

**Interfaces:**
- Produces: `animate(el, stops, options): Animation`
- Produces: `timeline(steps): Animation[]`
- Produces: `staggerDelays(count, each, from): number[]`
- Produces: `scrollProgress(el, startCfg, endCfg): number`
- Produces: `flipBefore(selector): void`, `flipAfter(selector, duration?, easing?): void`
- Produces: `heroBefore(): void`, `heroAfter(): void`
- Consumes: all types from types.ts, `buildKeyframes` from keyframes.ts, `resolveEasing` from easing.ts

- [ ] **Step 1: Create animate.ts**

```typescript
import type { Animation } from './types'
import type { KeyframeStop, MotionOptions } from './types'
import { buildKeyframes } from './keyframes'
import { resolveEasing } from './easing'

/**
 * Drive a single animation via WAAPI.
 * Returns the native Animation object — caller controls pause/cancel/seek.
 */
export function animate(
  el: Element,
  stops: KeyframeStop[],
  options: MotionOptions
): globalThis.Animation {
  const keyframes = buildKeyframes(stops)

  const timing: KeyframeAnimationOptions = {
    duration: options.duration,
    delay: options.delay ?? 0,
    easing: options.easing ?? 'ease-out',
    fill: options.fill ?? 'both',
    direction: options.direction ?? 'normal',
    iterations: options.iterations ?? 1,
  }

  return el.animate(keyframes as PropertyIndexedKeyframes, timing)
}
```

- [ ] **Step 2: Create timeline.ts**

```typescript
import type { TimelineStep } from './types'
import { animate } from './animate'

/**
 * Orchestrate multi-element animations by absolute start time.
 * Returns Animation[] for unified control (pause all, cancel all).
 */
export function timeline(steps: TimelineStep[]): globalThis.Animation[] {
  const animations: globalThis.Animation[] = []

  for (const step of steps) {
    const opts = { ...step.options, delay: (step.options.delay ?? 0) + step.at }
    const anim = animate(step.el, step.keyframes, opts)
    animations.push(anim)
  }

  return animations
}
```

- [ ] **Step 3: Create stagger.ts**

```typescript
/**
 * Compute stagger delay array for N items.
 * Direction controls the order: 'start' (first→last), 'end' (last→first),
 * 'center' (middle→outward), 'edges' (edges→inward).
 */
export function staggerDelays(
  count: number,
  each: number,
  from: 'start' | 'center' | 'end' | 'edges' = 'start'
): number[] {
  const indices: number[] = []
  if (from === 'start') {
    for (let i = 0; i < count; i++) indices.push(i)
  } else if (from === 'end') {
    for (let i = count - 1; i >= 0; i--) indices.push(i)
  } else if (from === 'edges') {
    let l = 0, r = count - 1
    while (l <= r) { indices.push(l++); if (l <= r) indices.push(r--) }
  } else { // center
    const mid = Math.floor((count - 1) / 2)
    indices.push(mid)
    let l = mid - 1, r = mid + 1
    while (l >= 0 || r < count) {
      if (l >= 0) indices.push(l--)
      if (r < count) indices.push(r++)
    }
  }

  const delays = new Array(count).fill(0)
  indices.forEach((origIdx, pos) => { delays[origIdx] = pos * each })
  return delays
}
```

- [ ] **Step 4: Create scroll.ts**

```typescript
/**
 * Compute scroll-driven animation progress (0..1) based on "top 80%" / "center 50%"
 * style trigger configuration strings.
 */
export function scrollProgress(
  el: Element,
  startCfg: string,
  endCfg: string
): number {
  const vh = window.innerHeight
  const startPos = parseScrollTrigger(startCfg, el, vh)
  const endPos = parseScrollTrigger(endCfg, el, vh)
  const raw = (window.scrollY - startPos) / (endPos - startPos)
  return Math.max(0, Math.min(1, raw))
}

function parseScrollTrigger(cfg: string, el: Element, vh: number): number {
  const parts = cfg.trim().split(/\s+/)
  if (parts.length < 2) return 0
  const edge = parts[0]
  const offset = parts[1]

  const rect = el.getBoundingClientRect()
  let elPos: number
  if (edge === 'top') elPos = rect.top
  else if (edge === 'center') elPos = rect.top + rect.height / 2
  else if (edge === 'bottom') elPos = rect.bottom
  else elPos = rect.top

  const absEdge = window.scrollY + elPos
  if (offset.endsWith('%')) return absEdge - vh * (parseFloat(offset) / 100)
  if (offset === 'bottom') return absEdge - vh
  if (offset === 'top') return absEdge
  return absEdge - vh * 0.5
}
```

- [ ] **Step 5: Create flip.ts**

```typescript
// FLIP animation for @for list reordering
const flipState = new Map<string, { rect: DOMRect; element: Element }>()

export function flipBefore(containerSelector: string): void {
  const container = document.querySelector(containerSelector)
  if (!container) return
  container.querySelectorAll('[data-for-key]').forEach(el => {
    const key = el.getAttribute('data-for-key')
    if (key) flipState.set(key, { rect: el.getBoundingClientRect(), element: el })
  })
}

export function flipAfter(
  containerSelector: string,
  duration: number = 400,
  easing: string = 'ease-out'
): void {
  const container = document.querySelector(containerSelector)
  if (!container) return
  container.querySelectorAll('[data-for-key]').forEach(el => {
    const key = el.getAttribute('data-for-key')
    const prev = flipState.get(key)
    if (!prev) return
    const newRect = el.getBoundingClientRect()
    const dx = prev.rect.left - newRect.left
    const dy = prev.rect.top - newRect.top
    if (dx === 0 && dy === 0) return
    ;(el as HTMLElement).style.transform = `translate(${dx}px,${dy}px)`
    ;(el as HTMLElement).style.transition = 'none'
    void (el as HTMLElement).offsetHeight // force reflow
    ;(el as HTMLElement).style.transition = `transform ${duration}ms ${easing}`
    ;(el as HTMLElement).style.transform = 'translate(0,0)'
    el.addEventListener('transitionend', function handler() {
      ;(el as HTMLElement).style.transition = ''
      ;(el as HTMLElement).style.transform = ''
      el.removeEventListener('transitionend', handler)
    })
  })
  flipState.clear()
}
```

- [ ] **Step 6: Create hero.ts**

```typescript
// @hero shared element transition
const heroState = new Map<string, { rect: DOMRect; element: Element }>()

export function heroBefore(): void {
  document.querySelectorAll('[data-hero]').forEach(el => {
    const tag = el.getAttribute('data-hero')
    if (!tag) return
    const isVisible = (el as HTMLElement).offsetParent !== null
      || el.getBoundingClientRect().width > 0
    if (!isVisible) return
    if (!heroState.has(tag)) {
      heroState.set(tag, { rect: el.getBoundingClientRect(), element: el })
    }
  })
}

export function heroAfter(): void {
  heroState.forEach((prev, tag) => {
    let visible: Element | null = null
    document.querySelectorAll(`[data-hero="${tag}"]`).forEach(el => {
      if ((el as HTMLElement).offsetParent !== null
        || el.getBoundingClientRect().width > 0) {
        visible = el
      }
    })
    if (!visible || visible === prev.element) return

    const newRect = visible.getBoundingClientRect()
    const dx = prev.rect.left - newRect.left
    const dy = prev.rect.top - newRect.top
    const dw = prev.rect.width / (newRect.width || 1)
    const dh = prev.rect.height / (newRect.height || 1)
    if (dx === 0 && dy === 0 && dw === 1 && dh === 1) return

    const dur = 400
    const ease = 'ease-out'
    ;(visible as HTMLElement).style.transformOrigin = '0 0'
    ;(visible as HTMLElement).style.transition = 'none'
    ;(visible as HTMLElement).style.transform = `translate(${dx}px,${dy}px) scale(${dw},${dh})`
    void (visible as HTMLElement).offsetHeight

    const kfs = [
      { transform: `translate(${dx}px,${dy}px) scale(${dw},${dh})`, offset: 0 },
      { transform: 'translate(0,0) scale(1,1)', offset: 1 }
    ]
    const anim = (visible as HTMLElement).animate(kfs, {
      duration: dur, easing: ease, fill: 'both'
    })
    anim.onfinish = () => {
      ;(visible as HTMLElement).style.transition = ''
      ;(visible as HTMLElement).style.transform = ''
      ;(visible as HTMLElement).style.transformOrigin = ''
    }
  })
  heroState.clear()
}
```

- [ ] **Step 7: Create index.ts (barrel export + UMD global)**

```typescript
export { animate } from './animate'
export { timeline } from './timeline'
export { buildKeyframes, buildProperties } from './keyframes'
export { resolveEasing } from './easing'
export { staggerDelays } from './stagger'
export { scrollProgress } from './scroll'
export { flipBefore, flipAfter } from './flip'
export { heroBefore, heroAfter } from './hero'
export type * from './types'

// UMD global — attach to window.Omni for <script> tag usage (OmniView)
import * as Omni from './index'
if (typeof window !== 'undefined') {
  ;(window as any).Omni = Omni
}
```

Note: The `index.ts` barrel export and the UMD global attachment need to be in separate files to avoid circular imports. The UMD wrapper goes in a separate entry point.

Create `packages/omni-animation/src/global.ts`:

```typescript
import * as Omni from './index'

if (typeof window !== 'undefined') {
  (window as any).Omni = Omni
}
```

And update `index.ts` (remove the UMD part):

```typescript
export { animate } from './animate'
export { timeline } from './timeline'
export { buildKeyframes, buildProperties } from './keyframes'
export { resolveEasing } from './easing'
export { staggerDelays } from './stagger'
export { scrollProgress } from './scroll'
export { flipBefore, flipAfter } from './flip'
export { heroBefore, heroAfter } from './hero'
export type * from './types'
```

- [ ] **Step 8: Verify TypeScript compiles**

Run: `cd packages/omni-animation && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add packages/omni-animation/src/
git commit -m "feat(omni-animation): add animate, timeline, stagger, scroll, flip, hero"
```

---

### Task 4: OmniView codegen — motion_config.rs + motion.rs rewrite

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_config.rs`
- Modify: `omnilang/src/codegen/omniview/motion.rs`
- Modify: `omnilang/src/codegen/omniview/runtime.rs`

**Interfaces:**
- Consumes: `MotionBodyIR`, `KeyframeIR`, `EasingIR`, `PlaybackIR` from IR (unchanged)
- Produces: JS code that calls `Omni.animate(el, kf, opts)` instead of `new AnimationPlayer(el, config)`

**Key design decision for codegen output:**
The codegen still emits plain JS strings. The emitted JS calls into the `Omni` global (set by `omni-animation.js`). For each `@motion` definition, codegen emits a factory function. For triggers, codegen emits event binding code that calls the factory + stores the returned `Animation` in `__omni_animations`.

- [ ] **Step 1: Rewrite easing_to_js() in motion_config.rs**

The easing serialization changes from `"new SpringSolver(180, 12, 1, 0)"` (AnimationPlayer constructor) to a string or object literal that `Omni.resolveEasing()` accepts.

In `motion_config.rs`, replace the existing `easing_to_js()`:

```rust
/// Convert an EasingIR to a JS expression for Omni.resolveEasing().
/// Output format: a string like `"easeOut"` or an object like
/// `{type:"spring", tension:180, friction:12}` for structured easings.
pub(super) fn easing_to_js(&self, easing: &EasingIR) -> String {
    match easing {
        EasingIR::Named(name) => format!("\"{}\"", name),
        EasingIR::CubicBezier { x1, y1, x2, y2 } => {
            format!("{{type:\"cubicBezier\",x1:{},y1:{},x2:{},y2:{}}}", x1, y1, x2, y2)
        }
        EasingIR::Spring { tension, friction, velocity } => {
            let vel = velocity.unwrap_or(0.0);
            format!("{{type:\"spring\",tension:{},friction:{},velocity:{}}}", tension, friction, vel)
        }
        EasingIR::Back { intensity, direction } => {
            let dir = direction.as_deref().unwrap_or("out");
            format!("{{type:\"back\",intensity:{},direction:\"{}\"}}", intensity, dir)
        }
        EasingIR::Elastic { amplitude, period, direction } => {
            let dir = direction.as_deref().unwrap_or("out");
            format!("{{type:\"elastic\",amplitude:{},period:{},direction:\"{}\"}}", amplitude, period, dir)
        }
        EasingIR::Steps { count, direction } => {
            format!("{{type:\"steps\",count:{},direction:\"{}\"}}", count, direction)
        }
    }
}
```

- [ ] **Step 2: Rewrite keyframe_to_config() in motion_config.rs**

Change from producing `"from": {...}, "to": {...}, "over": 400, "easing": "easeOut"` (AnimationPlayer config string) to producing two separate values: a keyframes array and an options object.

Add a new method `keyframe_to_animate_call()`:

```rust
/// Emit JS code that creates keyframes + options and calls Omni.animate().
/// Returns (keyframes_js, options_js) — two JS expressions.
pub(super) fn keyframe_to_animate_call(&self, kf: &KeyframeIR) -> (String, String) {
    // Build keyframes array: [{offset:0,opacity:"0",transform:"translateY(20px)"}, ...]
    let mut kf_items: Vec<String> = Vec::new();
    for stop in &kf.stops {
        let mut props: Vec<String> = Vec::new();
        for (prop, value) in &stop.properties {
            let val_js = self.expr_to_js(value);
            props.push(format!("\"{}\":{}", prop, val_js));
        }
        let mut item = format!("{{offset:{},properties:{{{}}}}}", stop.position, props.join(","));
        if let Some(ref ease) = stop.ease {
            let ease_js = self.easing_to_js(ease);
            item.push_str(&format!(",easing:{}", ease_js));
        }
        kf_items.push(item);
    }
    let kf_js = format!("[{}]", kf_items.join(","));

    // Build options object
    let mut opts: Vec<String> = Vec::new();
    let dur_js = self.expr_to_js(&kf.over);
    opts.push(format!("duration:{}", dur_js));
    if let Some(ref ease) = kf.ease {
        let ease_js = self.easing_to_js(ease);
        // resolveEasing at runtime
        opts.push(format!("easing:Omni.resolveEasing({})", ease_js));
    }
    // Playback options (fill, direction, iterations, delay)
    let fill_str = match &kf.playback.fill {
        Some(FillValue::None) => "\"none\"",
        Some(FillValue::Forwards) => "\"forwards\"",
        Some(FillValue::Backwards) => "\"backwards\"",
        Some(FillValue::Both) => "\"both\"",
        None => "\"both\"",
    };
    opts.push(format!("fill:{}", fill_str));
    if let Some(ref dir) = kf.playback.direction {
        let d = match dir {
            DirectionValue::Normal => "normal",
            DirectionValue::Reverse => "reverse",
            DirectionValue::Alternate => "alternate",
            DirectionValue::AlternateReverse => "alternate-reverse",
        };
        opts.push(format!("direction:\"{}\"", d));
    }
    if let Some(ref loop_val) = kf.playback.loop_count {
        match loop_val {
            LoopValue::Infinite => opts.push("iterations:Infinity".into()),
            LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
        }
    }
    if let Some(ref delay) = kf.playback.delay {
        let d = self.expr_to_js(delay);
        opts.push(format!("delay:{}", d));
    }
    let opts_js = format!("{{{}}}", opts.join(","));

    (kf_js, opts_js)
}
```

- [ ] **Step 3: Rewrite emit_motion_js() in motion.rs**

Change from emitting `new AnimationPlayer(el, {config})` to calling `Omni.animate(el, keyframes, options)`.

```rust
/// Emit a JS function that creates a WAAPI animation for a named @motion block.
pub(super) fn emit_motion_js(&mut self, motion: &MotionIntent) {
    let (kf_js, opts_js) = match &motion.body {
        MotionBodyIR::Keyframe(kf) => self.keyframe_to_animate_call(kf),
        MotionBodyIR::Timeline(tl) => {
            // Timeline: emit a function that calls Omni.timeline()
            // (handled separately in timeline.rs for the actual body)
            // For the factory, emit a placeholder that returns a timeline control object
            self.timeline_to_factory(&motion.name, tl)
        }
        MotionBodyIR::CallbackOnly(cb) => {
            // Callback-only: still needs keyframes for the animate call
            self.callback_to_animate_call(cb)
        }
    };

    self.js.push_str(&format!(
        "function __omni_motion_{name}(el, overrides) {{\n\
         \x20 const opts = Object.assign({{{opts}}}, overrides || {{}});\n\
         \x20 const kf = {kf};\n\
         \x20 return Omni.animate(el, kf, opts);\n\
         }};\n",
        name = motion.name,
        opts = opts_js,
        kf = kf_js,
    ));
}
```

- [ ] **Step 4: Rewrite emit_animation_runtime() in runtime.rs**

Change from embedding `motion-runtime.js` via `include_str!()` to emitting a `<script>` tag reference.

```rust
/// Emit the animation library reference instead of embedding motion-runtime.js.
/// Two modes:
///   - standalone: embed the library source inline (for single-file output)
///   - default: emit <script src="./omni-animation.js"> reference
pub(super) fn emit_animation_runtime(&mut self) {
    self.js.push_str("/// ─── Omni Animation Runtime (WAAPI-based) ───\n");
    // Reference the standalone library file
    // Codegen option `inline_runtime` controls whether to embed or reference
    self.js.push_str(
        "// @omni/animation v0.1.0 — load from external script\n\
         // <script src=\"./omni-animation.js\"> should be included in the HTML wrapper\n\
         if (typeof Omni === 'undefined') console.warn('[OmniView] @omni/animation not loaded');\n\n"
    );
}
```

- [ ] **Step 5: Build and verify**

Run: `cd omnilang && cargo check 2>&1`
Expected: PASS (no errors in modified files)

- [ ] **Step 6: Compile a test .omni file to check output format**

Run: `./omnic compile test/projects/learn-omniview/src/pages/MotionDemo.omni --target omniview 2>&1 | head -80`
Expected: Generated JS contains `Omni.animate(` calls, not `new AnimationPlayer(`

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_config.rs omnilang/src/codegen/omniview/motion.rs omnilang/src/codegen/omniview/runtime.rs
git commit -m "refactor(omnilang): rewrite OmniView motion codegen to use @omni/animation library"
```

---

### Task 5: OmniView codegen — motion_triggers.rs rewrite

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs`

**Interfaces:**
- Consumes: `TriggerIntent`, `ElementIR` from IR (unchanged)
- Produces: JS event binding code that calls `__omni_motion_NAME(el, config)` (factory from Task 4) instead of `new AnimationPlayer(el, config)`

- [ ] **Step 1: Change all "new AnimationPlayer(el, config)" to use factory function**

The migration is mechanical: every block of the form
```javascript
const config = { trigger: "enter" };
const player = __omni_motion_fadeIn(el, config);
__omni_animations.set("nid_enter", player);
```

Becomes:
```javascript
const anim = __omni_motion_fadeIn(el, { trigger: "enter" });
__omni_animations.set("nid_enter", anim);
```

The key changes:
- `player` → `anim` (variable rename — it's now a native `Animation`, not an `AnimationPlayer`)
- `player.play()` → `anim.play()` (WAAPI Animation has `.play()`)
- `player.pause()` → `anim.pause()` (WAAPI Animation has `.pause()`)
- `player.reverse()` → `anim.reverse()` (WAAPI Animation has `.reverse()`)
- `player.seek(progress)` → `anim.currentTime = progress * duration` (WAAPI doesn't have `.seek()`)
- `player.kill()` → `anim.cancel()` (WAAPI name)
- `__omni_getAnimation(nid, trigger)` → `__omni_animations.get(nid + "_" + trigger)` (inline the helper, simpler)

For `seek()`, the codegen needs to know the duration. Emit:
```javascript
// Instead of: player.seek(currentProgress);
const duration = anim.effect?.getTiming()?.duration || 300;
anim.currentTime = currentProgress * duration;
```

Since the codegen already knows the duration from `kf.over`, we can emit it directly:
```javascript
anim.currentTime = currentProgress * {duration_ms};
```

Edit each trigger handler block in `emit_motion_triggers_for_element()` to:
1. Replace `new AnimationPlayer(el, config)` with `__omni_motion_{name}(el, config)`
2. Replace `const player =` with `const anim =`
3. Replace `.seek(...)` with `.currentTime = ... * {over_ms}`
4. Replace `.kill()` with `.cancel()`
5. Replace `__omni_getAnimation(...)` with `__omni_animations.get(...)`

Due to the file being ~690 lines with 40+ replacement sites, the implementation should:
1. First compile the existing code to make sure it builds
2. Make the replacements site-by-site
3. Re-compile after each group of changes

- [ ] **Step 2: Handle the "inline motion" (nameless) path**

Currently inline motions use `new AnimationPlayer(el, config)`. Change to `Omni.animate(el, kf, opts)`. At this point in the codegen, the inline motion body has already been serialized by `motion_body_to_config()`. Change `motion_body_to_config()` to produce `(kf_js, opts_js)` tuple instead of a single config string.

Update the inline motion handler sites to use:
```javascript
const anim = Omni.animate(el, [{offset:0,properties:{...}}, {offset:1,properties:{...}}], {duration:400,easing:Omni.resolveEasing(...)});
__omni_animations.set("nid_enter", anim);
```

- [ ] **Step 3: Build and verify**

Run: `cd omnilang && cargo check 2>&1`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_triggers.rs
git commit -m "refactor(omnilang): rewrite OmniView motion triggers to use @omni/animation"
```

---

### Task 6: OmniView codegen — timeline + remaining files

**Files:**
- Modify: `omnilang/src/codegen/omniview/timeline.rs`
- Modify: `omnilang/src/codegen/omniview/html.rs` (3 sites)
- Modify: `omnilang/src/codegen/omniview/component_html.rs` (1 site)
- Modify: `omnilang/src/codegen/omniview/editor.rs` (2 sites)
- Modify: `omnilang/src/codegen/omniview/store.rs` (3 sites)
- Modify: `omnilang/src/codegen/omniview/list_render.rs` (2 sites)

**Interfaces:**
- Consumes: Productions from Tasks 4-5
- Produces: Complete OmniView codegen that emits calls to @omni/animation library

- [ ] **Step 1: Rewrite timeline.rs — setTimeout chains → Omni.timeline()**

The current timeline.rs emits `setTimeout` chains calling `new AnimationPlayer(el, config)`. Change to emit `Omni.timeline(steps)`:

```rust
// In emit_timeline_js(), replace the setTimeout chain generation with:
self.js.push_str(&format!(
    "function __omni_motion_{name}(el, overrides) {{\n\
     \x20 if (!el) return {{ kill: function() {{}} }};\n\
     \x20 const children = Array.from(el.querySelectorAll('[data-nid]'));\n\
     \x20 const steps = [];\n\
     // ... emit steps.push({{ el: children[i], keyframes: [...], options: {{...}}, at: delay }}) for each timeline item ...\
     \x20 const anims = Omni.timeline(steps);\n\
     \x20 return {{\n\
     \x20\x20 kill: function() {{ anims.forEach(function(a) {{ a.cancel(); }}); }}\n\
     \x20 }};\n\
     }}\n",
    name = timeline.name,
));
```

The body of the function (building the `steps` array) is generated by iterating `timeline.items` and converting each `Direction`/`Set`/`MotionRef`/`Stagger` item into a `{el, keyframes, options, at}` JS object literal. The timeline cursor math (overlap, with-prev, etc.) stays the same — only the output changes from `setTimeout(...)` to `steps.push(...)`.

- [ ] **Step 2: Fix html.rs — inline animation handlers**

In `html.rs`, there are 3 sites where inline event handlers create `AnimationPlayer`:
- Line ~636: Motion reference in event handler
- Line ~840: @driven-by seeks all animations
- Line ~862: @paused override

Fix each:
- The event handler inline code (addEventListener that creates AnimationPlayer): change `new AnimationPlayer(el, config)` to `__omni_motion_{name}(el, {})`
- @driven-by: `player.seek(progress)` → `anim.currentTime = progress * (anim.effect?.getTiming()?.duration || 300)`
- @paused: `player.pause()` / `player.resume()` → `anim.pause()` / `anim.play()`

- [ ] **Step 3: Fix component_html.rs — same as html.rs for component rendering**

Same pattern — the inline event handler creates `AnimationPlayer`. Change to use `__omni_motion_{name}()`.

- [ ] **Step 4: Fix editor.rs — animation lookup**

In `editor.rs`, `__omni_animations.get(nid + "_leave")` returns a native `Animation` now. The `.kill()` call becomes `.cancel()`.

- [ ] **Step 5: Fix store.rs — registry + kill → cancel**

Three changes:
1. `__omni_animations.set()` values are now native `Animation` (no code change needed — the JS is still `__omni_animations.set(key, value)`)
2. `__omni_animations.get(key)?.kill()` → `__omni_animations.get(key)?.cancel()`
3. The @enter replay logic in emit_store now calls `__omni_motion_{name}(el, config)` (factory from Task 4) instead of creating AnimationPlayer directly

- [ ] **Step 6: Fix list_render.rs — flip functions**

Change `__omni_flip_before` / `__omni_flip_after` calls to `Omni.flipBefore` / `Omni.flipAfter`.

- [ ] **Step 7: Build and verify**

Run: `cd omnilang && cargo check 2>&1`
Expected: PASS

- [ ] **Step 8: Compile all test .omni files to verify output**

Run: `for f in test/projects/learn-omniview/src/pages/*Demo.omni test/projects/learn-omniview/src/components/*Demo.omni; do echo "=== $f ===" && ./omnic compile "$f" --target omniview 2>&1 | tail -5; done`
Expected: All compile successfully, no errors

- [ ] **Step 9: Commit**

```bash
git add omnilang/src/codegen/omniview/timeline.rs omnilang/src/codegen/omniview/html.rs omnilang/src/codegen/omniview/component_html.rs omnilang/src/codegen/omniview/editor.rs omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/list_render.rs
git commit -m "refactor(omnilang): rewrite remaining OmniView codegen files for @omni/animation"
```

---

### Task 7: React codegen — keyframes.rs + mod.rs + view.rs fix

**Files:**
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs`
- Modify: `omnilang/src/codegen/react/motion/mod.rs`
- Modify: `omnilang/src/codegen/react/view.rs`
- Modify: `omnilang/src/codegen/react/component.rs`

**Interfaces:**
- Consumes: `MotionBodyIR`, `KeyframeIR` from IR (unchanged)
- Produces: TSX that imports from `@omni/animation` and calls `animate()` instead of `el.animate()`

- [ ] **Step 1: Rewrite keyframes.rs — output JS config objects, not inline WAAPI strings**

Change `format_motion_to_waapi()` to output `{ keyframes: KeyframeStop[], options: {duration, easing, fill, ...} }` instead of `[keyframes[], options{}]`.

```rust
/// Format a MotionBodyIR into JS: `{kf:{...},opts:{...}}` for use with animate().
pub(super) fn format_motion_to_animate(motion: &MotionBodyIR) -> String {
    match motion {
        MotionBodyIR::Keyframe(kf) => {
            let kf_items = format_keyframe_stops(kf);
            let opts = format_animate_options(&kf.over, &kf.ease, &kf.playback);
            format!("{{kf:{kf},opts:{opts}}}", kf = kf_items, opts = opts)
        }
        MotionBodyIR::Timeline(_) | MotionBodyIR::CallbackOnly(_) => {
            // Provide sensible defaults for unsupported body types in React
            "{kf:[{offset:0,properties:{opacity:0}},{offset:1,properties:{opacity:1}}],opts:{duration:300,easing:\"ease-out\",fill:\"both\"}}".to_string()
        }
    }
}

/// Format keyframe stops into the KeyframeStop[] format (not WAAPI Keyframe[]).
fn format_keyframe_stops(kf: &KeyframeIR) -> String {
    let mut items: Vec<String> = Vec::new();
    let mut stops = kf.stops.clone();
    stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

    for stop in &stops {
        let mut prop_pairs: Vec<String> = Vec::new();
        for (prop, value) in &stop.properties {
            let val_str = expr::format_expr_ir_tsx(value);
            prop_pairs.push(format!("\"{}\":{}", prop, val_str));
        }
        items.push(format!("{{offset:{},properties:{{{}}}}}", stop.position, prop_pairs.join(",")));
    }
    format!("[{}]", items.join(","))
}

/// Format motion options for animate() call.
fn format_animate_options(over: &ExprIR, ease: &Option<EasingIR>, playback: &PlaybackIR) -> String {
    let mut opts: Vec<String> = Vec::new();
    let dur = expr::format_expr_ir_tsx(over);
    opts.push(format!("duration:{}", dur));
    if let Some(e) = ease {
        opts.push(format!("easing:{}", format_easing_to_js(e)));
    }
    // fill
    let fill = match &playback.fill {
        Some(FillValue::Forwards) | None => "\"forwards\"",
        Some(FillValue::None) => "\"none\"",
        Some(FillValue::Backwards) => "\"backwards\"",
        Some(FillValue::Both) => "\"both\"",
    };
    opts.push(format!("fill:{}", fill));
    // direction
    if let Some(dir) = &playback.direction {
        let d = match dir {
            DirectionValue::Normal => "normal",
            DirectionValue::Reverse => "reverse",
            DirectionValue::Alternate => "alternate",
            DirectionValue::AlternateReverse => "alternate-reverse",
        };
        opts.push(format!("direction:\"{}\"", d));
    }
    // iterations
    if let Some(loop_val) = &playback.loop_count {
        match loop_val {
            LoopValue::Infinite => opts.push("iterations:Infinity".into()),
            LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
        }
    }
    // delay
    if let Some(delay) = &playback.delay {
        opts.push(format!("delay:{}", expr::format_expr_ir_tsx(delay)));
    }
    format!("{{{}}}", opts.join(","))
}
```

- [ ] **Step 2: Rewrite mod.rs — _M format + import { animate }**

Update `emit_motion_effects_tsx()`:

```rust
// Before the _M constant:
w.emit("import { animate } from \"@omni/animation\";");
w.emit("");

// The _M constant now stores {kf, opts} objects:
w.emit("const _M={");
// ...
for name in &sorted {
    if let Some(body) = motion_bodies.get(name.as_str()) {
        let def_js = keyframes::format_motion_to_animate(body);
        w.emit_fmt(format_args!("{}:{},", name, def_js));
    }
}
// ...

// The useEffect now calls animate() instead of el.animate():
w.emit("const[kf,opts]=[_M[n].kf,_M[n].opts];");
w.emit("animate(el,kf,opts);");
```

- [ ] **Step 3: Fix view.rs — @hover/@press/@focus hardcoded animation Bug**

In `omnilang/src/codegen/react/view.rs`, lines 160-182 currently emit hardcoded inline `el.animate()` calls for @hover, @press, @focus. These don't use the actual motion definition.

Change to use data attributes (similar to @enter's `data-motion-name`) and a shared `_M_trigger` lookup.

For @hover:
```rust
// Instead of hardcoded:
// onMouseEnter={(e) => { e.currentTarget.animate([{opacity:0},{opacity:1}],{duration:300,easing:'ease-out'}) }}
//
// Emit:
TriggerKind::Hover => {
    attrs.push("data-motion-hover=\"true\"".into());
    if let Some(ref name) = trigger.motion_ref {
        attrs.push(format!("data-motion-name=\"{}\"", name));
    }
}
```

And add a corresponding `useEffect` in `emit_motion_effects_tsx()` (mod.rs) that handles @hover, @press, @focus the same way @enter is handled — querying `[data-motion-hover]` elements and calling `animate()` with the correct motion definition.

Similarly for @press (`data-motion-press`), @focus (`data-motion-focus`), @unhover (`data-motion-unhover`).

- [ ] **Step 4: Update component.rs — add import statement**

In `omnilang/src/codegen/react/component.rs`, add before the component function body:
```rust
w.emit("import { animate } from \"@omni/animation\";");
```

- [ ] **Step 5: Build and verify**

Run: `cd omnilang && cargo check 2>&1`
Expected: PASS

- [ ] **Step 6: Compile React test to verify output**

Run: `./omnic compile test/projects/learn-omniview/src/pages/MotionDemo.omni --target react 2>&1 | head -60`
Expected: Generated TSX contains `import { animate }` and `animate(el, kf, opts)` calls

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/react/
git commit -m "refactor(omnilang): rewrite React motion codegen for @omni/animation; fix hardcoded @hover/@press/@focus"
```

---

### Task 8: React codegen — transforms.rs (in-view, scroll, drag)

**Files:**
- Modify: `omnilang/src/codegen/react/motion/transforms.rs`

**Interfaces:**
- Consumes: `ViewTree` from IR (unchanged)
- Produces: useEffect hooks that call `animate()` instead of `el.animate()`

- [ ] **Step 1: Fix emit_inview_effects_tsx()**

Change `el.animate([{opacity:0},{opacity:1}],{duration:300,easing:'ease-out',fill:'both'})` to `animate(el,[{offset:0,properties:{opacity:0}},{offset:1,properties:{opacity:1}}],{duration:300,easing:'ease-out',fill:'both'})`.

- [ ] **Step 2: Fix emit_scroll_effects_tsx()**

The current scroll code seeks WAAPI animations directly. Change `a.currentTime = scrubbed * duration` to also work with `animate()`-created animations (which are identical — they're still WAAPI `Animation` objects, so `.currentTime` and `.getAnimations()` still work). No change needed in the seek logic itself.

But the initial animation creation (not shown in emit_scroll_effects_tsx — that's in mod.rs where _M is used) needs updating. The scroll effect just seeks already-created animations, so it's compatible as-is.

Verify: `el.getAnimations()` returns native `Animation` objects regardless of whether they were created by `el.animate()` directly or through `animate()`. Both use WAAPI internally. **No change needed.**

- [ ] **Step 3: Fix emit_drag_effects_tsx()**

Similarly, drag effects interact with animations via `.getAnimations()`, `.play()`, `.reverse()`. These are all WAAPI-native methods. **No change needed.**

- [ ] **Step 4: Build and verify**

Run: `cd omnilang && cargo check 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/motion/transforms.rs
git commit -m "refactor(omnilang): update React transform effects for @omni/animation"
```

---

### Task 9: Cleanup — remove motion-runtime.js, build artifact, integration test

**Files:**
- Delete: `omnilang/src/runtime/motion-runtime.js`
- Delete: `build/omni-motion-runtime.js`
- Create: `packages/omni-animation/build-bundle.js` (bundler script for standalone JS output)
- Modify: `build/` build scripts (if any reference motion-runtime.js)

- [ ] **Step 1: Create bundler script for standalone JS output**

Create `packages/omni-animation/build-bundle.js`:

```javascript
// Simple bundler: concatenate TypeScript-compiled modules into one IIFE
// that exposes window.Omni for <script> tag usage (OmniView).
const fs = require('fs')
const path = require('path')

// Read compiled JS files from dist/
const files = ['types.js', 'easing.js', 'keyframes.js', 'animate.js',
  'timeline.js', 'stagger.js', 'scroll.js', 'flip.js', 'hero.js']

let bundle = '(function(){\n"use strict";\n\n'
for (const f of files) {
  const content = fs.readFileSync(path.join('dist', f), 'utf-8')
  // Strip import/export statements (simple approach)
  const stripped = content
    .replace(/^import .+$/gm, '')
    .replace(/^export /gm, '')
  bundle += stripped + '\n'
}

// Attach to global
bundle += `
const Omni = { animate, timeline, buildKeyframes, buildProperties, resolveEasing, staggerDelays, scrollProgress, flipBefore, flipAfter, heroBefore, heroAfter };
window.Omni = Omni;
})();
`

fs.writeFileSync('dist/omni-animation.js', bundle)
fs.writeFileSync('dist/omni-animation.mjs', bundle) // for module usage
console.log('Bundled omni-animation.js')
```

- [ ] **Step 2: Delete motion-runtime.js**

```bash
rm omnilang/src/runtime/motion-runtime.js
rm build/omni-motion-runtime.js
```

- [ ] **Step 3: Verify runtime.rs compiles after deletion**

The `include_str!("../../../../build/omni-motion-runtime.js")` in runtime.rs was already replaced in Task 4 Step 4. If not, replace it now.

Run: `cd omnilang && cargo check 2>&1`
Expected: PASS (no reference to deleted file)

- [ ] **Step 4: Run full build to verify everything links**

Run: `cd omnilang && cargo build 2>&1`
Expected: PASS

- [ ] **Step 5: Run existing tests**

Run: `cd omnilang && cargo test 2>&1`
Expected: All existing tests pass (motion-related tests may need minor assertion updates if they check for `AnimationPlayer` in output strings)

- [ ] **Step 6: Compile all animation demo .omni files with both targets**

```bash
# OmniView target
for f in test/projects/learn-omniview/src/pages/*Demo.omni test/projects/learn-omniview/src/components/*Demo.omni; do
  echo "=== OmniView: $f ==="
  ./omnic compile "$f" --target omniview 2>&1 | tail -3
done

# React target
for f in test/projects/learn-omniview/src/pages/*Demo.omni test/projects/learn-omniview/src/components/*Demo.omni; do
  echo "=== React: $f ==="
  ./omnic compile "$f" --target react 2>&1 | tail -3
done
```

Expected: All compile without errors.

- [ ] **Step 7: Commit**

```bash
git rm omnilang/src/runtime/motion-runtime.js build/omni-motion-runtime.js
git add packages/omni-animation/build-bundle.js
git commit -m "chore: remove embedded motion-runtime.js; add bundler for @omni/animation"
```

---

### Post-Implementation Verification

After all tasks are complete:

1. **OmniView output sanity check**: Compile `MotionDemo.omni` to OmniView, verify output JS contains `Omni.animate(` not `AnimationPlayer`
2. **React output sanity check**: Compile `MotionDemo.omni` to React, verify output TSX contains `import { animate }` and `animate(el,`
3. **Run Playwright tests**: Execute the existing React Playwright test suite (18 tests) — all should pass
4. **Manual browser test**: Open compiled OmniView output in browser, verify animations play correctly
5. **Library standalone test**: Load `omni-animation.js` in a browser console, call `Omni.animate(document.body, [{offset:0,properties:{opacity:0}}, {offset:1,properties:{opacity:1}}], {duration:1000})` and verify it animates
