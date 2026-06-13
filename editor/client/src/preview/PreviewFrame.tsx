import { useEditorStore } from "../store/editor";

export function PreviewFrame() {
  const compileErrors = useEditorStore((s) => s.compileErrors);

  if (compileErrors.length > 0) {
    return (
      <div style={{ padding: 16, height: "100%", overflow: "auto" }}>
        <div style={{ fontSize: 13, fontWeight: 600, color: "#DC2626", marginBottom: 8 }}>
          ❌ Compilation Errors
        </div>
        {compileErrors.map((e, i) => (
          <div
            key={i}
            style={{
              padding: "6px 12px", marginBottom: 4, borderRadius: 4,
              background: "#FEF2F2", border: "1px solid #FECACA",
              fontSize: 12, fontFamily: "monospace",
            }}
          >
            <span style={{ color: "#991B1B", fontWeight: 600 }}>[{e.code}]</span>{" "}
            <span style={{ color: "#7F1D1D" }}>L{e.line}: {e.message}</span>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div style={{
      display: "flex", alignItems: "center", justifyContent: "center",
      height: "100%", color: "#9ca3af", fontSize: 13,
    }}>
      <div style={{ textAlign: "center" }}>
        <div style={{ fontSize: 32, marginBottom: 8 }}>📄</div>
        <div>编译以预览输出</div>
        <div style={{ fontSize: 11, marginTop: 4, color: "#d1d5db" }}>Ctrl+Enter</div>
      </div>
    </div>
  );
}
