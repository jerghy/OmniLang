// components/design/cards/LockIcon.tsx
// Shared lock/unlock toggle for property cards.

import React from "react";

const LOCK_BTN: React.CSSProperties = {
  border: "none", background: "none", cursor: "pointer",
  fontSize: 12, padding: "0 2px", flexShrink: 0,
};

/** Style for a locked (🔒) property input in base mode — greyed out. */
export const LOCKED_INPUT_STYLE: React.CSSProperties = {
  opacity: 0.45,
  pointerEvents: "none",
};

export interface LockIconProps {
  propName: string;
  inheritedProps?: string[];
  onToggleInherited?: (name: string) => void;
  readOnly?: boolean;
}

/** Returns true when the property is locked in base (not inherited, can be overridden). */
export function isPropLocked(propName: string, inheritedProps?: string[]): boolean {
  if (!inheritedProps) return false;
  return !inheritedProps.includes(propName);
}

export const LockIcon = React.memo(function LockIcon({
  propName, inheritedProps, onToggleInherited, readOnly,
}: LockIconProps) {
  const isUnlocked = inheritedProps?.includes(propName);
  if (!readOnly && onToggleInherited) {
    return (
      <button type="button"
        style={{ ...LOCK_BTN, color: isUnlocked ? "var(--dp-accent, #6366f1)" : "var(--dp-text-muted, #a09db8)" }}
        onClick={() => onToggleInherited(propName)}
        title={isUnlocked ? "已解锁（继承到分支）— 点击封锁" : "已封锁（分支可覆盖）— 点击解锁"}
      >{isUnlocked ? "🔓" : "🔒"}</button>
    );
  }
  return null;
});
