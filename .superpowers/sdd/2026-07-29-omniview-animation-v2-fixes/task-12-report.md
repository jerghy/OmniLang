# Task 12 Report: Fix `__omni_scrollRemoveEntry` cleanup order in @for leave-animation

## Root Cause

In `omnilang/src/codegen/omniview/list.rs`, the generated JavaScript for the @for leave-animation path called `el.remove()` BEFORE `__omni_cleanupElement(nid)`. Inside `__omni_cleanupElement`, the call to `__omni_scrollRemoveEntry(nid)` uses `__omni_el(nid)` to find the element. Since the element was already removed from the DOM, `__omni_el(nid)` returned null, short-circuiting the scroll entry cleanup.

## Fix

Swapped the order in two codegen locations so `__omni_cleanupElement(leavingNid)` is called **before** `el.remove()`:

- **Line 353** (player animation path): `player.play().then(() => { __omni_cleanupElement(leavingNid); el.remove(); ... })`
- **Line 357** (no animation path): `__omni_cleanupElement(leavingNid); el.remove(); ...`

## Verification

- `cargo test --workspace` -- all 515+ tests pass
- `cargo clippy --workspace -- -D warnings` -- clean
- Committed as `3b8d9199` with message `fix(omniview): swap cleanup order in @for leave-animation path`

## File Changed

- `omnilang/src/codegen/omniview/list.rs` -- 2 insertions, 2 deletions
