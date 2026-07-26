# Task 5 Report: SelectionManager — Cursor Navigation System

## Status: COMPLETE

## Summary

Implemented `SelectionManager` — a game-controller-style cursor navigation system for the Block Editor. Builds a spatial grid model on top of Blockly, tracks cursor position, renders a visual selection box overlay, and fires `onChange` callbacks.

## Files Created

### `poiesis/src/components/canvas/code/blocks/SelectionManager.ts`

**Core type** — `SelectionTarget` union:
- `{ type: "block"; blockId: string }`
- `{ type: "slot"; blockId: string; inputName: string }`
- `{ type: "gap"; parentBlockId: string | null; position: number }`

**Grid model:**
- `refresh()` scans `workspace.getTopBlocks(true)` and builds a flat list: for N blocks, the grid has N block entries and N-1 gap entries between them
- Gap positions use N+0.5 numbering (0.5, 1.5, etc.)

**Navigation methods:**
| Method | Behavior |
|--------|----------|
| `moveLeft()` | Previous grid entry (clamped at start) |
| `moveRight()` | Next grid entry (clamped at end) |
| `moveUp()` | Previous block, skipping gap entries |
| `moveDown()` | Next block, skipping gap entries |
| `moveTo(target)` | Jump directly to a specific target |
| `enterSlot()` | Stub for Task 7 KeyboardHandler |
| `exitSlot()` | Stub for Task 7 KeyboardHandler |

**Visual rendering:**
- Absolutely-positioned `<div>` overlaid on the Blockly SVG parent
- Block: 2px solid terracotta (`#cc6b4a`) outline, offset 2px, rounded corners
- Slot: 2px dashed terracotta border, rounded corners
- Gap: horizontal terracotta line with CSS blink animation (`selectionManagerBlink`)
- Auto-creates blink keyframes via `<style>` element
- Gracefully falls back to a small indicator when coordinates are unavailable (jsdom)
- `dispose()` removes DOM elements and resets callback

### `poiesis/src/components/canvas/code/blocks/__tests__/SelectionManager.test.ts`

25 tests across 6 describe blocks:

| Group | Tests | What it covers |
|-------|-------|----------------|
| grid model | 4 | Initial target, empty workspace, 3-block grid structure (block-gap-block-gap-block) |
| moveLeft/moveRight | 6 | Single block stays, boundary clamping, gap navigation, 2-block cycle |
| moveUp/moveDown | 6 | Block-only navigation, gap skipping, multi-block skip |
| moveTo | 2 | Direct navigation to block by ID and gap by position |
| onChange | 4 | Callback fires after each move direction |
| refresh | 1 | Grid rebuilds when blocks change |
| dispose | 2 | Cleanup and double-dispose safety |

## Verification

- `npx vitest run src/components/canvas/code/blocks/` — **3 suites, 96 tests, all passed**
- Full project test suite: 26 suites, 245/246 passed (1 pre-existing failure in `editor-helpers.test.ts`, unrelated)

## Commit

```
c1e2db7 feat: add SelectionManager — game-controller cursor navigation for Block Editor
```

## API Surface for Downstream Consumers (Task 7 KeyboardHandler)

```typescript
const sm = new SelectionManager(workspace);
sm.getCurrentTarget();           // SelectionTarget | null
sm.moveLeft/Right/Up/Down();     // void
sm.moveTo(target);               // void
sm.enterSlot(inputName);         // void (stub)
sm.exitSlot();                   // void (stub)
sm.refresh();                    // void
sm.dispose();                    // void
sm.onChange = (target) => { ... };
```

The `enterSlot`/`exitSlot` methods are defined as API stubs. Implementation of slot sub-navigation is deferred to Task 7 (KeyboardHandler).

---

## Fix Report (Review Findings)

### Fix Commit

```
5805ac8 fix: resolve 4 review findings in SelectionManager
```

### Finding 1 (HIGH) — Memory leak: change listener not removed in dispose()

**Root cause:** The `workspace.addChangeListener()` return value was not stored,
so `dispose()` could not unsubscribe. The listener held a reference to the
SelectionManager instance, preventing GC.

**Fix:** Stored the return value as `this.changeListenerId`. In `dispose()`,
calls `this.workspace.removeChangeListener(this.changeListenerId)` before
cleaning up DOM elements and callback.

### Finding 2 (HIGH) — Coordinate system bug in getTargetBoundingRect

**Root cause:** `getTargetBoundingRect` for blocks mixed coordinate spaces:
`svgRect.left + xy.x` where `svg.getBoundingClientRect()` returns viewport
coordinates but `block.getRelativeToSurfaceXY()` returns workspace-surface
coordinates (which include scroll/window offset differences).

**Fix:** Use `block.getSvgRoot().getBoundingClientRect()` directly, which
returns the block's viewport-positioned rect in a single call, eliminating
the coordinate space mismatch entirely.

### Finding 3 (MEDIUM) — Non-null assertion and `as` casts

**Root cause:**
- `entry.blockId!` in `entryToTarget()` uses a non-null assertion on a
  nullable field. If a non-gap entry somehow had `blockId: null`, this
  would produce `{ type: "block", blockId: null }` at runtime.
- `targetsEqual()` used `as SelectionTarget & { type: "block" }` casts
  instead of discriminated union narrowing, bypassing the type checker.

**Fix:**
- Added explicit `if (entry.blockId === null)` guard with a defensive
  fallback treating it as a gap entry.
- Replaced all `as` casts with proper discriminated union narrowing:
  `if (a.type === "block" && b.type === "block")`, etc.

### Finding 4 (MEDIUM) — Empty catch blocks

**Root cause:** Two `catch { }` blocks silently swallowed exceptions,
making debugging difficult.

**Fix:** Changed both to `catch (error) { console.warn("SelectionManager.<method>:", error); }`:
- `getTargetBoundingRect` catch block
- `getSvgParent` catch block

### Verification

- 25/25 tests pass (unchanged from original)
- 96/96 block tests pass
