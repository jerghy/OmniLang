import { useCallback, useEffect, useRef } from "react";
import { usePoiesisStore } from "../../../store";
import styles from "./BubbleContextMenu.module.css";

interface MenuPosition {
  x: number;
  y: number;
}

interface BubbleContextMenuProps {
  /** Set to null to hide, or {x,y} to show at that position */
  position: MenuPosition | null;
  /** The id of the bubble the menu was opened for */
  bubbleId: string | null;
  onClose: () => void;
}

export function BubbleContextMenu({
  position,
  bubbleId,
  onClose,
}: BubbleContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  useEffect(() => { onCloseRef.current = onClose; });
  const focusComponent = usePoiesisStore((s) => s.focusComponent);
  const openTab = usePoiesisStore((s) => s.openTab);
  const expandBubble = usePoiesisStore((s) => s.expandBubble);
  const bubbles = usePoiesisStore((s) => s.architecture.bubbles);

  // Close on click outside
  useEffect(() => {
    if (!position) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onCloseRef.current();
      }
    };
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCloseRef.current();
    };
    // Delay adding listener to avoid the same click that opened the menu
    const timer = setTimeout(() => {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("keydown", handleEsc);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEsc);
    };
  }, [position]);

  const handleFocus = useCallback(() => {
    if (bubbleId) focusComponent(bubbleId);
    onClose();
  }, [bubbleId, focusComponent, onClose]);

  const handleOpenDesign = useCallback(() => {
    if (!bubbleId) return;
    const bubble = bubbles.find((b) => b.id === bubbleId);
    if (bubble && bubble.pid) {
      const pidIndex = usePoiesisStore.getState().pidIndex;
      const entry = pidIndex[bubble.pid];
      const filePath = entry?.sourceFile ?? bubble.pid;
      openTab(filePath, bubble.label);
    }
    onClose();
  }, [bubbleId, bubbles, openTab, onClose]);

  const handleExpandAll = useCallback(() => {
    if (!bubbleId) return;
    const expandDescendants = (nodeId: string) => {
      expandBubble(nodeId);
      const node = bubbles.find((b) => b.id === nodeId);
      node?.children.forEach(expandDescendants);
    };
    expandDescendants(bubbleId);
    onClose();
  }, [bubbleId, bubbles, expandBubble, onClose]);

  if (!position || !bubbleId) return null;

  return (
    <div
      ref={menuRef}
      className={styles.menuPanel}
      style={{
        left: position.x,
        top: position.y,
      }}
    >
      <MenuItem onClick={handleFocus}>以该组件为中心</MenuItem>
      <MenuItem onClick={handleOpenDesign}>打开设计页面</MenuItem>
      <MenuDivider />
      <MenuItem onClick={handleExpandAll}>展开全部</MenuItem>
    </div>
  );
}

function MenuItem({
  onClick,
  children,
}: {
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick(); } }}
      className={styles.menuItem}
    >
      {children}
    </button>
  );
}

function MenuDivider() {
  return (
    <div
      style={{
        height: 1,
        background: "var(--ol-border)",
        margin: "4px 0",
      }}
    />
  );
}
