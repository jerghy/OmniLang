// poiesis/src/components/design/omni-view-utils.ts
// Shared utilities for OmniViewElement and OmniViewRenderer.

/** Module-level: last click position (clientX, clientY), captured by click handlers.
 *  Read by OmniViewRenderer to center the code-flow shrink animation on the click point. */
export const _lastClickPos = { x: 0, y: 0 };
