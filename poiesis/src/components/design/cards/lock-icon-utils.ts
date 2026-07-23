// poiesis/src/components/design/cards/lock-icon-utils.ts
// Shared lock/unlock utilities for property cards.

import type React from "react";

/** Style for a locked (🔒) property input in base mode — greyed out. */
export const LOCKED_INPUT_STYLE: React.CSSProperties = {
  opacity: 0.45,
  pointerEvents: "none",
};

/** Returns true when the property is locked in base (not inherited, can be overridden). */
export function isPropLocked(propName: string, inheritedProps?: string[]): boolean {
  if (!inheritedProps) return false;
  return !inheritedProps.includes(propName);
}
