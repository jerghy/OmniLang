// components/design/CodeFlowCanvas.tsx
// Container for the code node flow canvas (Phase 5 placeholder).

export function CodeFlowCanvas() {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "#1e293b",
        color: "#94a3b8",
        fontSize: 14,
        flexDirection: "column",
        gap: 8,
      }}
    >
      <div style={{ fontSize: 24, opacity: 0.5 }}>◉</div>
      <div>代码节点流</div>
      <div style={{ fontSize: 12, color: "#64748b" }}>
        双击代码块节点编辑
      </div>
    </div>
  );
}
