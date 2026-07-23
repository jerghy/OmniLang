// ═══════════════════════════════════════════════
// Shared Toolbar — present in both design and code modes
// ═══════════════════════════════════════════════

import React from "react";
import { DebugWindowButton } from "../../features/debug/DebugWindowButton";
import { S, btnStyle, iconBtn } from "./theme";

export interface ToolbarProps {
  mode: "design" | "code";
  isDirty: boolean;
  projectDir: string | null;
  runStatus: { running: boolean; url: string | null };
  runLoading: boolean;
  onModeChange: (mode: "design" | "code") => void;
  onUndo: () => void;
  onRedo: () => void;
  onSave: () => void;
  onRun: () => void;
  onStop: () => void;
  onCompile: () => void;
  compileLabel?: string;
  onOpenCssStyles?: () => void;
  children?: React.ReactNode;
}

export function Toolbar({
  mode,
  isDirty,
  runStatus,
  runLoading,
  onModeChange,
  onUndo,
  onRedo,
  onSave,
  onRun,
  onStop,
  onCompile,
  compileLabel = "编译",
  onOpenCssStyles = () => {},
  children,
}: ToolbarProps) {
  return (
    <div style={S.toolbar}>
      {/* Mode toggle */}
      <button type="button" onMouseDown={() => onModeChange("design")} onClick={() => onModeChange("design")} style={S.modeBtn(mode === "design")}>🎨 设计</button>
      <button type="button" onMouseDown={() => onModeChange("code")} onClick={() => onModeChange("code")} style={S.modeBtn(mode === "code")}>💻 代码</button>
      <span style={{ width: 8 }} />
      {/* Undo/Redo */}
      <button type="button" onClick={onUndo} style={iconBtn} title="撤销 (Ctrl+Z)">↩</button>
      <button type="button" onClick={onRedo} style={iconBtn} title="重做 (Ctrl+Shift+Z)">↪</button>
      {/* Save */}
      <button type="button" onClick={onSave} style={{...iconBtn, color: isDirty ? "#DC2626" : "#6B7280", fontWeight: isDirty ? 700 : 400 as React.CSSProperties['fontWeight']}} title="保存项目 (Ctrl+S)">💾</button>
      <span style={{ width: 12 }} />
      <button type="button" onClick={onOpenCssStyles} style={{...btnStyle, color: "#6366F1", fontWeight: 600, fontSize: 12}} title="CSS 样式设计">🎨 CSS 样式</button>
      <DebugWindowButton />
      {children}
      {/* Run/Stop/Compile */}
      {runStatus.running ? (
        <>
          <a href={runStatus.url || "#"} target="_blank" rel="noreferrer"
            style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", textDecoration: "none" }}>
            🔗 {runStatus.url?.replace("http://", "") || "运行"}
          </a>
          <button type="button" onClick={onStop}
            style={{...btnStyle, background: "#FEF2F2", borderColor: "#FCA5A5", color: "#DC2626" }}>
            ⏹ 停止
          </button>
        </>
      ) : (
        <button type="button" onClick={onRun} disabled={runLoading}
          style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", fontWeight: 600 }}>
          {runLoading ? "⏳" : "▶"} 运行
        </button>
      )}
      <button type="button" onClick={onCompile} style={btnStyle}>{compileLabel}</button>
    </div>
  );
}
