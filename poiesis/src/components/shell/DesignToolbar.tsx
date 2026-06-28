// ═══════════════════════════════════════════════
// Design-mode-specific toolbar buttons
// ═══════════════════════════════════════════════

import React from "react";
import { iconBtn } from "./theme";

export interface DesignToolbarProps {
  breakpoint: "desktop" | "tablet" | "mobile";
  previewState: string;
  showTokens: boolean;
  wireMode: boolean;
  showHUD: boolean;
  showFocusOrder: boolean;
  onBreakpointChange: (bp: "desktop" | "tablet" | "mobile") => void;
  onPreviewStateChange: (state: string) => void;
  onToggleTokens: () => void;
  onToggleWireMode: () => void;
  onToggleHUD: () => void;
  onToggleFocusOrder: () => void;
  onOpenProject: () => void;
}

export function DesignToolbar({
  breakpoint,
  previewState,
  showTokens,
  wireMode,
  showHUD,
  showFocusOrder,
  onBreakpointChange,
  onPreviewStateChange,
  onToggleTokens,
  onToggleWireMode,
  onToggleHUD,
  onToggleFocusOrder,
  onOpenProject,
}: DesignToolbarProps) {
  return (
    <>
      {/* Open project */}
      <button onClick={onOpenProject} style={iconBtn} title="打开项目">📂</button>
      <span style={{ flex: 1 }} />
      {/* Viewport */}
      <span style={{ fontSize: 10, color: "#9CA3AF" }}>视口:</span>
      {(["desktop","tablet","mobile"] as const).map(bp => (
        <button key={bp} onClick={() => onBreakpointChange(bp)}
          style={{ padding: "2px 6px", fontSize: 10, border: "1px solid", borderRadius: 3, cursor: "pointer",
            background: breakpoint === bp ? "#4F46E5" : "#fff",
            color: breakpoint === bp ? "#fff" : "#6B7280",
            borderColor: breakpoint === bp ? "#4F46E5" : "#d1d5db" }}>
          {{desktop:"🖥",tablet:"📱",mobile:"📱"}[bp]}
        </button>
      ))}
      {/* State preview */}
      <select value={previewState} onChange={(e) => onPreviewStateChange(e.target.value)}
        style={{ padding: "2px 4px", fontSize: 10, border: "1px solid #d1d5db", borderRadius: 3, background: "#fff" }}
        title="Component state preview">
        <option value="normal">👁</option>
        <option value="hover">🖱</option>
        <option value="active">👆</option>
        <option value="focus">🎯</option>
        <option value="disabled">🚫</option>
        <option value="loading">⏳</option>
      </select>
      <span style={{ width: 8 }} />
      {/* Toggle buttons (compact icon-only) */}
      <button onClick={onToggleTokens}
        style={{...iconBtn, background: showTokens ? "#EEF2FF" : "transparent", borderColor: showTokens ? "#4F46E5" : "transparent" }}
        title="令牌面板">🎨</button>
      <button onClick={onToggleWireMode}
        style={{...iconBtn, background: wireMode ? "#EEF2FF" : "transparent", borderColor: wireMode ? "#4F46E5" : "transparent" }}
        title="连线模式">🔗</button>
      <button onClick={onToggleHUD}
        style={{...iconBtn, background: showHUD ? "#EEF2FF" : "transparent", borderColor: showHUD ? "#4F46E5" : "transparent" }}
        title="状态 HUD">🔍</button>
      <button onClick={onToggleFocusOrder}
        style={{...iconBtn, background: showFocusOrder ? "#EEF2FF" : "transparent", borderColor: showFocusOrder ? "#4F46E5" : "transparent" }}
        title="焦点顺序">↹</button>
      <span style={{ width: 8 }} />
    </>
  );
}
