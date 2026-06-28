// ═══════════════════════════════════════════════
// Shared theme styles for shell components
// ═══════════════════════════════════════════════

import React from "react";

export const S = {
  app: { display: "flex", height: "100vh", fontFamily: "system-ui, sans-serif" } as React.CSSProperties,
  main: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column", position: "relative", zIndex: 1 } as React.CSSProperties,
  toolbar: { height: 36, minHeight: 36, borderBottom: "1px solid #e5e7eb", padding: "0 12px", display: "flex", alignItems: "center", gap: 8, fontSize: 12, position: "relative", zIndex: 5 } as React.CSSProperties,
  codeArea: { flex: 1, overflow: "hidden", display: "flex" } as React.CSSProperties,
  statusBar: { height: 24, minHeight: 24, borderTop: "1px solid #e5e7eb", padding: "0 12px", display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 11, background: "#f9fafb" } as React.CSSProperties,
  inspector: { width: 280, minWidth: 280, flexShrink: 0, borderLeft: "1px solid #e5e7eb", display: "flex", flexDirection: "column", background: "#f9fafb" } as React.CSSProperties,
  modeBtn: (active: boolean) => ({ padding: "3px 14px", fontSize: 12, border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600, background: active ? "#4F46E5" : "#F3F4F6", color: active ? "#fff" : "#374151" } as React.CSSProperties),
};

export const btnStyle: React.CSSProperties = { padding: "4px 12px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 4, background: "#fff", cursor: "pointer" };

export const iconBtn: React.CSSProperties = { padding: "2px 6px", fontSize: 13, border: "1px solid transparent", borderRadius: 4, background: "transparent", cursor: "pointer", lineHeight: 1 };
