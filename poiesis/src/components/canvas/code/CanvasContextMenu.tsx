// components/canvas/code/CanvasContextMenu.tsx
// Right-click context menu for the flow canvas with collapsible sections and smart positioning.

import { MENU_SECTIONS, MENU_WIDTH, ITEM_HEIGHT } from "./flowConstants";
import styles from "./CanvasContextMenu.module.css";

export interface ContextMenuProps {
  x: number;
  y: number;
  menuRef: React.RefObject<HTMLDivElement | null>;
  collapsedSections: Set<number>;
  onToggleSection: (i: number) => void;
  onSelectNode: (tag: string) => void;
  onClose: () => void;
}

export function ContextMenu({
  x,
  y,
  menuRef,
  collapsedSections,
  onToggleSection,
  onSelectNode,
  onClose,
}: ContextMenuProps) {

  // Compute total visible items for height estimation
  let visibleCount = 0;
  for (let i = 0; i < MENU_SECTIONS.length; i++) {
    visibleCount++; // header
    if (!collapsedSections.has(i)) {
      visibleCount += MENU_SECTIONS[i].items.length;
    }
  }
  const estimatedHeight = visibleCount * ITEM_HEIGHT + MENU_SECTIONS.length * 4;
  const maxHeight = Math.min(estimatedHeight, 400);

  const vwW = window.innerWidth;
  const vwH = window.innerHeight;

  let left = x;
  if (x + MENU_WIDTH > vwW) {
    left = Math.max(0, x - MENU_WIDTH);
  }
  let top = y;
  if (y + maxHeight > vwH) {
    top = Math.max(0, y - maxHeight);
  }

  return (
    <div
      ref={menuRef}
      style={{
        position: "fixed",
        top,
        left,
        zIndex: 10000,
        background: "#fff",
        borderRadius: 8,
        boxShadow: "0 4px 20px rgba(0,0,0,0.15)",
        minWidth: MENU_WIDTH,
        maxWidth: MENU_WIDTH,
        maxHeight,
        overflowY: "auto",
        fontSize: 12,
        padding: "2px 0",
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {MENU_SECTIONS.map((section, i) => {
        const collapsed = collapsedSections.has(i);
        return (
          <div key={section.label}>
            {i > 0 && <div style={{ height: 1, background: "#e2e8f0", margin: "2px 0" }} />}
            <button type="button"
              onClick={() => onToggleSection(i)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                width: "100%",
                padding: "4px 10px",
                border: "none",
                background: "transparent",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: 600,
                color: "#64748b",
                textTransform: "uppercase" as const,
                letterSpacing: "0.5px",
                fontFamily: "inherit",
              }}
            >
              <span style={{ fontSize: 12, width: 10, textAlign: "center" as const }}>
                {collapsed ? "▶" : "▼"}
              </span>
              <span>{section.label}</span>
              <span style={{ marginLeft: "auto", fontSize: 12, color: "#94a3b8" }}>
                {collapsed ? section.items.length : ""}
              </span>
            </button>
            {!collapsed &&
              section.items.map((item) => {
                const isDisabled = item.tag === "";
                return (
                  <button type="button"
                    key={item.label}
                    disabled={isDisabled}
                    onClick={() => { if (!isDisabled) { onSelectNode(item.tag); onClose(); } }}
                    className={styles.menuBtn}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      width: "100%",
                      padding: "5px 12px 5px 26px",
                      border: "none",
                      background: "transparent",
                      cursor: isDisabled ? "default" : "pointer",
                      fontSize: 12,
                      color: isDisabled ? "#94a3b8" : "#1e293b",
                      fontStyle: isDisabled ? "italic" : "normal",
                      textAlign: "left" as const,
                      fontFamily: "inherit",
                    }}
                  >
                    <span>{item.label}</span>
                    {item.shortcut && (
                      <span style={{ color: isDisabled ? "#cbd5e1" : "#94a3b8", fontSize: 12, fontFamily: "monospace" }}>
                        {item.shortcut}
                      </span>
                    )}
                  </button>
                );
              })}
          </div>
        );
      })}
    </div>
  );
}
