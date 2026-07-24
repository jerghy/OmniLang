# Task 6 Report: KeyboardPlugin Rewrite

## STATUS: COMPLETE

## File Modified

| File | Description |
|------|-------------|
| `poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts` | Rewrote Delete/Backspace/Enter/Tab handlers for v2 cursor three-state model |

## Summary

Rewrote `KeyboardPlugin.ts` to align with the v2 cursor model where CardNode is an `ElementNode` with `contenteditable="false"`. The browser now natively handles card selection, arrow-key navigation, and card deletion. The plugin only intercepts edge cases the browser cannot handle.

### Key Changes

- **Delete handler**: Uses `detectRightEdge()` to only intercept when cursor is at container or body-line right-edge. Lets browser handle card deletion natively.
- **Backspace handler**: New handler that swallows Backspace at container start (no-op). All other cases fall through to native browser.
- **Enter handler**: Uses `findAncestorBodyLine()` for Enter-in-body-line. Also handles Enter after container right-edge.
- **Tab handler**: Uses `findCardAtCursor()` helper to detect operator cards adjacent to cursor (not just directly on card), enabling Tab cycling from adjacent text nodes.
- **Helpers**: Three new functions encapsulating cursor-position detection logic.

### TypeScript
- 0 new errors in KeyboardPlugin.ts (pre-existing 657 errors unchanged).

### Test Results
- **6/6 tests passed** (same as baseline):
  - Delete on inline card removes it
  - Delete on container right edge removes entire container
  - Enter at end of body line creates new line
  - backslash in let container creates name input (placeholder)
  - backslash at top level is a no-op (placeholder)
  - Tab in set container cycles operator

### Type Fixes Applied vs. Brief
Three minor type fixes beyond the exact brief code:
1. Added `$isCardNode(cardNode)` guard before card-specific methods on `findCardAtCursor` result
2. Added explicit `LexicalNode | null` type annotation to `parent` in `detectRightEdge` while loop
3. Added `$isElementNode(parent)` guard before `getLastChild()` (parent is `LexicalNode`, `getLastChild` on `ElementNode`)

### Commit
```
2ee8a87 refactor(codecard-v2): rewrite KeyboardPlugin with right-edge detection and cursor navigation
```
