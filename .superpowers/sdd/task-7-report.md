# Task 7 Report: React Codegen — keyframes.rs + mod.rs + view.rs fix

## Status: COMPLETE

## Commits
- `f23a8b4` — `refactor(omnilang): rewrite React motion codegen for @omni/animation; fix hardcoded @hover/@press/@focus`

## Files Modified

1. **`omnilang/src/codegen/react/motion/keyframes.rs`** — Rewrote `format_motion_to_waapi()` to `format_motion_to_animate()` which outputs `{kf:[...],opts:{...}}` format. Added `format_keyframe_stops()`, `format_animate_options()`, and `format_easing_to_js()` helpers. The new keyframe format uses `{offset:0,properties:{opacity:0}}` instead of flat WAAPI keyframes.

2. **`omnilang/src/codegen/react/motion/mod.rs`** — Updated `emit_motion_effects_tsx()` to:
   - Emit `const{kf,opts}=_M[n]` and `animate(el,kf,opts)` instead of WAAPI `el.animate()`
   - Renamed `collect_enter_motion_refs` to `collect_all_motion_refs` to collect from all trigger types
   - Removed the old warnings/comments mechanism (no longer needed)
   - Added useEffect handlers for @hover (`data-motion-hover-name`), @press (`data-motion-press-name`), @focus (`data-motion-focus-name`) that attach event listeners and call `animate()`
   - There are now 4 useEffect sections: @enter, @hover, @press, @focus

3. **`omnilang/src/codegen/react/view.rs`** — Fixed the hardcoded @hover/@press/@focus bug:
   - Replaced inline `el.animate([{opacity:0},{opacity:1}],{duration:300,...})` with data attributes
   - Uses scoped name attributes (`data-motion-hover-name`, `data-motion-press-name`, `data-motion-focus-name`) instead of generic `data-motion-name` to avoid attribute duplication when multiple triggers are on the same element
   - Added handling for `TriggerKind::Unhover` (was previously ignored)

4. **`omnilang/src/codegen/react/component.rs`** — Added `import { animate } from "@omni/animation"` before component function body, gated on `has_motion`.

## Build Results

- `cargo check`: PASS (0 new warnings, 12 pre-existing)
- `cargo test --lib`: PASS (444 tests)
- Generated TSX verification: PASS
  - MotionTest.omni output confirmed: uses `import { animate }`, `{kf, opts}` format, `animate(el,kf,opts)`, scoped data attributes per trigger type
  - MotionDemo.omni output confirmed: same pattern with real motion definitions
  - No duplicate `data-motion-name` attributes (each trigger has its own scoped name attr)
  - No hardcoded inline animations

## Key Changes

| Before | After |
|--------|-------|
| `el.animate([{opacity:0},{opacity:1}],{duration:300})` | `animate(el, {kf:[{offset:0,properties:{opacity:0}},...], opts:{...}})` |
| `@hover` emitted hardcoded inline `el.animate()` | `data-motion-hover="true"` + `data-motion-hover-name="motionName"` |
| `_M[n]` was `[keyframes[], options{}, setupFn?]` tuple | `_M[n]` is `{kf:[...], opts:{...}}` object |
| Import inside function body | `import { animate }` at top of file |

## Concerns

- The `format_motion_to_animate()` function no longer preserves path/offsetPath setup for motion paths (the old `format_motion_to_waapi()` had a `(el) => {...}` setup function in the tuple). This is a gap but was explicitly dropped in the brief since the omni-animation library handles paths differently.
- `@unhover` trigger handling was added to view.rs but is not yet wired into the useEffect (no handler for `data-motion-unhover`). This is an existing gap in the IR — `Unhover` exists as a TriggerKind variant but no compilable Omni syntax produces it separately from `@hover`.
