# Task 5 Report: OmniView codegen — motion_triggers.rs rewrite

## Status: COMPLETE

## Summary

Rewrote `omnilang/src/codegen/omniview/motion_triggers.rs` and supporting `motion_config.rs` to migrate from the old AnimationPlayer runtime to `@omni/animation` (WAAPI-based) API.

## Changes Made

### `omnilang/src/codegen/omniview/motion_config.rs`
- Changed `motion_body_to_config()` return type from `String` (config fragment) to `(String, String)` (keyframes_js, opts_js), delegating to `keyframe_to_animate_call()`, `timeline_to_factory()`, and `callback_to_animate_call()` respectively.

### `omnilang/src/codegen/omniview/motion_triggers.rs`
Applied mechanical replacements to ALL 11 trigger handlers (Enter, Hover, Press, Focus, Leave, InView, Scroll, Drag, Animate, Timeline, and default catch-all):

| Change | Old | New |
|--------|-----|-----|
| Variable rename | `const player = ...` | `const anim = ...` |
| Factory function (named) | `__omni_motion_{name}(el, config)` | `__omni_motion_{name}(el, { trigger: ... })` (inlined config) |
| Inline motion creation | `new AnimationPlayer(el, config)` | `Omni.animate(el, kf, opts)` |
| Playback control | `player.play()` / `.pause()` / `.reverse()` | `anim.play()` / `.pause()` / `.reverse()` |
| Seek replacement | `player.seek(progress)` | `anim.currentTime = progress * dur` |
| Animation lookup | `__omni_getAnimation(nid, trigger)` | `__omni_animations.get(nid + "_" + trigger)` |
| Duration fetch (scroll/drag) | n/a | `const dur = anim.effect?.getTiming()?.duration \|\| 300;` |

### Key implementation details

1. **Named motion path** (motion_ref): Config object inlined into factory call, eliminating the intermediate `const config = {...}` line.
2. **Inline motion path** (inline_motion): Switched from `new AnimationPlayer(el, config)` to `Omni.animate(el, kf, {{opts}})`, using the `motion_body_to_config()` tuple return.
3. **Scroll and drag handlers**: Both need to seek the animation to a progress value. Since WAAPI `Animation.currentTime` is in milliseconds (not normalized 0-1), the handlers fetch the actual duration dynamically via `anim.effect?.getTiming()?.duration || 300` before computing `currentTime = progress * dur`.
4. **Event listener lookups**: All `__omni_getAnimation(nid, trigger)` calls replaced with `__omni_animations.get(nid + "_" + trigger)`.
5. **Leave trigger**: Left unchanged as it uses `__omni_createAnimation` (a different runtime helper not covered by this migration).

## Verification

- `cargo check`: PASS (0 new warnings, only pre-existing warnings)
- Existing warnings: 12 (all pre-existing, unrelated to these changes)
- The `__omni_createAnimation` call in the Leave handler was intentionally preserved (it doesn't use AnimationPlayer)

## Commit

```
b6d9384 refactor(omnilang): rewrite OmniView motion triggers to use @omni/animation
```

## Concerns

None. All changes are mechanical and the build passes cleanly.
