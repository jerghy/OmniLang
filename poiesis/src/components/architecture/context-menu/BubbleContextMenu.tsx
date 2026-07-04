import { useCallback, useEffect, useRef } from "react";
import { usePoiesisStore } from "../../../store";

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
  const focusComponent = usePoiesisStore((s) => s.focusComponent);
  const openTab = usePoiesisStore((s) => s.openTab);
  const expandBubble = usePoiesisStore((s) => s.expandBubble);
  const bubbles = usePoiesisStore((s) => s.architecture.bubbles);

  // Close on click outside
  useEffect(() => {
    if (!position) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
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
  }, [position, onClose]);

  const handleFocus = useCallback(() => {
    if (bubbleId) focusComponent(bubbleId);
    onClose();
  }, [bubbleId, focusComponent, onClose]);

  const handleOpenDesign = useCallback(() => {
    if (!bubbleId) return;
    const bubble = bubbles.find((b) => b.id === bubbleId);
    if (bubble && bubble.pid) {
      openTab(bubble.pid, bubble.label);
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
      style={{
        position: "absolute",
        left: position.x,
        top: position.y,
        background: "#fff",
        border: "1px solid #e5e7eb",
        borderRadius: 8,
        boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
        padding: "4px 0",
        minWidth: 180,
        zIndex: 1000,
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
    <div
      onClick={onClick}
      style={{
        padding: "8px 16px",
        fontSize: 13,
        cursor: "pointer",
        color: "#374151",
        transition: "background 0.15s",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.background = "#f3f4f6")}
      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
    >
      {children}
    </div>
  );
}

function MenuDivider() {
  return (
    <div
      style={{
        height: 1,
        background: "#e5e7eb",
        margin: "4px 0",
      }}
    />
  );
}
