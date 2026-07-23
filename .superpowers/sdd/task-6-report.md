# Task 6 Report: OmniView Codegen — Timeline + Remaining Files

## Status: COMPLETE

## Summary
Rewrote 6 OmniView codegen files to emit calls to the @omni/animation library (Omni.animate, Omni.timeline, Omni.flipBefore/After) instead of the old AnimationPlayer/setTimeout/kill/resume/seek patterns.

## Files Modified

| File | Changes |
|------|---------|
| `omnilang/src/codegen/omniview/timeline.rs` | Rewrote `emit_timeline_js()`: setTimeout chains replaced with `steps.push({el, keyframes, options, at})` + `Omni.timeline(steps)`. Kill function uses `anims.forEach(a => a.cancel())`. All timeline cursor math preserved. |
| `omnilang/src/codegen/omniview/html.rs` | @driven-by: `player.seek(progress)` -> `anim.currentTime = progress * dur`. @paused: `player.resume()` -> `anim.play()`. Stale comment updated. |
| `omnilang/src/codegen/omniview/component_html.rs` | Comment only (code already used `__omni_motion_{name}` factory). |
| `omnilang/src/codegen/omniview/editor.rs` | Wait-mode animation: `leavePlayer.completed/playing` -> native `playState`/`.finished` Promise. `enterPlayer.config?.paused` -> `enterPlayer.playState !== "paused"`. |
| `omnilang/src/codegen/omniview/store.rs` | `.kill()` -> `.cancel()` in @enter replay. Stale comment updated. |
| `omnilang/src/codegen/omniview/list_render.rs` | Added `Omni.flipBefore(parent)` and `Omni.flipAfter(parent)` for list re-render FLIP animations. |

## Verification

- `cargo check`: PASS (same 12 pre-existing warnings as before)
- Full project build (`omnic build test/projects/learn-omniview --target omniview`): PASS
- Old patterns in output (`AnimationPlayer`, `.seek(`, `.resume(`, `.kill(`): 0 occurrences
- New patterns in output: `Omni.timeline` (6), `Omni.flip*` (2), `Omni.animate` (81), `__omni_motion_` (185)

## Commit

```
61f24ed refactor(omnilang): rewrite remaining OmniView codegen files for @omni/animation
```

## Concerns

- `__omni_getAnimation` in list_render.rs (used for @leave on @for items) references a runtime helper that must be defined in the @omni/animation runtime library for key-based leave animations to work.
- Path/motion data in timeline steps like `"path": "M0,0 C..."` is emitted inline within `options: {duration, easing, "path": "..."}`. The Omni.timeline runtime must handle path-based keyframe options.
- FLIP animations require `Omni.flipBefore/After` to be defined in the runtime library.
