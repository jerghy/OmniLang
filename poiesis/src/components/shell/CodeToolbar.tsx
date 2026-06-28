// ═══════════════════════════════════════════════
// Code-mode-specific toolbar buttons
// ═══════════════════════════════════════════════

import React from "react";
import { btnStyle } from "./theme";

export interface CodeToolbarProps {
  target: string;
  showConnector: boolean;
  projectDir: string | null;
  onTargetChange: (target: string) => void;
  onToggleConnector: () => void;
  onAutoLayout: () => void;
}

export function CodeToolbar({
  target,
  showConnector,
  projectDir,
  onTargetChange,
  onToggleConnector,
  onAutoLayout,
}: CodeToolbarProps) {
  return (
    <>
      {/* Quick template dropdown */}
      <select
        value=""
        onChange={(e) => {
          const v = e.target.value;
          if (v) {
            const O = (window as unknown as Record<string, unknown>).__OMNI__ as { snippet: (v: string) => void } | undefined;
            if (O) O.snippet(v);
          }
          e.target.value = "";
        }}
        style={{...btnStyle, fontSize: 11, padding: "2px 6px"}}
        title="快速模板"
      >
        <option value="" disabled>📋 模板</option>
        <option value="login-form">登录表单</option>
        <option value="data-card">数据卡片</option>
        <option value="crud-table">CRUD 表格</option>
      </select>
      {/* Project dir indicator */}
      {projectDir && <span style={{ color: "#9ca3af", fontSize: 11 }}>{projectDir.split(/[/\\]/).slice(-2).join("/")}</span>}
      <span style={{ flex: 1 }} />
      {/* Target selector */}
      <span style={{ fontSize: 10, color: "#9ca3af" }}>目标:</span>
      <select value={target} onChange={e => onTargetChange(e.target.value)} style={{ padding: "2px 6px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4 }}>
        {["react","omniview","vue","dioxus","flutter","python","javascript","rust","dart"].map(t => <option key={t}>{t}</option>)}
      </select>
      {/* Connector toggle */}
      <button onClick={onToggleConnector}
        style={{...btnStyle, background: showConnector ? "#EEF2FF" : "#fff", borderColor: showConnector ? "#4F46E5" : "#d1d5db" }}>
        {showConnector ? "🔌 连接器" : "🔌 连接器"}
      </button>
      {/* Auto-layout trigger */}
      <button onClick={onAutoLayout}
        style={{...btnStyle, background: "#fff"}}>
        📐 自动布局
      </button>
    </>
  );
}
