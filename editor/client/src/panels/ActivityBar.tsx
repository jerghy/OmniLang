/** Activity Bar — VS Code-style icon column for panel switching.
 *  Design §4.2: click icon → toggle panel. Only one panel open at a time. */

import { useState, type ReactNode } from "react";

export type PanelId = "files" | "state" | "functions" | null;

interface PanelDef {
  id: NonNullable<PanelId>;
  icon: string;
  label: string;
  shortcut: string;
}

const PANELS: PanelDef[] = [
  { id: "files",     icon: "📁", label: "文件浏览器", shortcut: "Ctrl+1" },
  { id: "state",     icon: "⚡", label: "状态管理",   shortcut: "Ctrl+2" },
  { id: "functions", icon: "📦", label: "函数库",     shortcut: "Ctrl+3" },
];

interface ActivityBarProps {
  activePanel: PanelId;
  onToggle: (id: NonNullable<PanelId>) => void;
}

export function ActivityBar({ activePanel, onToggle }: ActivityBarProps) {
  const [hovered, setHovered] = useState<string | null>(null);

  return (
    <div style={{
      width: 48, minWidth: 48, flexShrink: 0,
      background: "#F3F4F6", borderRight: "1px solid #E5E7EB",
      display: "flex", flexDirection: "column", alignItems: "center",
      paddingTop: 8, gap: 2,
    }}>
      {PANELS.map(panel => {
        const isActive = activePanel === panel.id;
        return (
          <div
            key={panel.id}
            onClick={() => onToggle(panel.id)}
            onMouseEnter={() => setHovered(panel.id)}
            onMouseLeave={() => setHovered(null)}
            title={`${panel.label} (${panel.shortcut})`}
            style={{
              width: 40, height: 40, borderRadius: 8,
              display: "flex", alignItems: "center", justifyContent: "center",
              cursor: "pointer", fontSize: 18,
              background: isActive ? "#E5E7EB" : "transparent",
              borderLeft: isActive ? "3px solid #4F46E5" : "3px solid transparent",
              position: "relative",
              transition: "background 0.1s",
            }}
          >
            {panel.icon}
            {/* Tooltip on hover */}
            {hovered === panel.id && !isActive && (
              <div style={{
                position: "absolute", left: 48, top: "50%", transform: "translateY(-50%)",
                background: "#1F2937", color: "#fff", fontSize: 11,
                padding: "4px 10px", borderRadius: 4, whiteSpace: "nowrap",
                zIndex: 100, pointerEvents: "none",
              }}>
                {panel.label}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ── Panel container — slides in/out ─────────────

interface PanelContainerProps {
  visible: boolean;
  children: ReactNode;
}

export function PanelContainer({ visible, children }: PanelContainerProps) {
  const width = visible ? 240 : 0;
  return (
    <div style={{
      width, minWidth: width, maxWidth: width,
      overflow: visible ? "auto" : "hidden",
      borderRight: visible ? "1px solid #E5E7EB" : "none",
      background: "#F9FAFB",
      transition: "width 0.15s ease, min-width 0.15s ease",
    }}>
      <div style={{ width: 240, minWidth: 240 }}>
        {children}
      </div>
    </div>
  );
}

export { PANELS };
