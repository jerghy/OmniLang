// ═══════════════════════════════════════════════
// Shared theme styles for shell components
// Uses CSS custom properties (--ol-*) for theming
// ═══════════════════════════════════════════════

import React from "react";

export const S = {
  app: { display: "flex", height: "100vh", fontFamily: "var(--ol-font-family-ui)" } as React.CSSProperties,
  main: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column", position: "relative", zIndex: 1 } as React.CSSProperties,
  toolbar: { height: 36, minHeight: 36, borderBottom: "1px solid var(--ol-border)", padding: "0 12px", display: "flex", alignItems: "center", gap: 8, fontSize: 12, position: "relative", zIndex: 5 } as React.CSSProperties,
  codeArea: { flex: 1, overflow: "hidden", display: "flex" } as React.CSSProperties,
  statusBar: { height: 24, minHeight: 24, borderTop: "1px solid var(--ol-border)", padding: "0 12px", display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 11, background: "var(--ol-gray-50)" } as React.CSSProperties,
  inspector: { width: 280, minWidth: 280, flexShrink: 0, borderLeft: "1px solid var(--ol-border)", display: "flex", flexDirection: "column", background: "var(--ol-gray-50)" } as React.CSSProperties,
  modeBtn: (active: boolean) => ({ padding: "3px 14px", fontSize: 12, border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600, background: active ? "var(--ol-color-primary-hover)" : "var(--ol-gray-100)", color: active ? "var(--ol-text-inverse)" : "var(--ol-text)" } as React.CSSProperties),
};
