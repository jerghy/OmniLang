import { useState, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import { TokenEditor } from "./TokenEditor";
import { ClassEditor } from "./ClassEditor";
import { VariantEditor } from "./VariantEditor";

type DesignView = "tokens" | "classes" | "variants";

export function DesignTabContent() {
  const [view, setView] = useState<DesignView>("tokens");
  const [showCode, setShowCode] = useState(false);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);

  const designSource = useMemo(() => emitDesignSource(activeDesign), [activeDesign]);

  if (showCode) {
    return (
      <div style={{ padding: 16 }}>
        <button
          onClick={() => setShowCode(false)}
          style={{ marginBottom: 8, padding: "2px 8px", border: "1px solid #d4d4d8", borderRadius: 4, background: "#fff", cursor: "pointer", fontSize: 12 }}
        >
          ← 返回编辑器
        </button>
        <pre style={{ fontFamily: "monospace", fontSize: 12, color: "#1e293b", background: "#f8fafc", padding: 12, borderRadius: 6, overflow: "auto", whiteSpace: "pre-wrap" }}>
          {designSource}
        </pre>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid #e2e8f0", padding: "0 12px", alignItems: "center" }}>
        {(["tokens", "classes", "variants"] as const).map((v) => (
          <button key={v} onClick={() => setView(v)}
            style={{
              padding: "8px 16px", background: "none", border: "none",
              borderBottom: view === v ? "2px solid #6366f1" : "2px solid transparent",
              color: view === v ? "#6366f1" : "#64748b", fontSize: 13, fontWeight: view === v ? 600 : 400, cursor: "pointer",
            }}
          >
            {v === "tokens" ? "Tokens" : v === "classes" ? "Classes" : "Variants"}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        <button onClick={() => setShowCode(true)}
          style={{ padding: "2px 10px", border: "1px solid #e2e8f0", borderRadius: 4, background: "#f8fafc", cursor: "pointer", fontSize: 11, color: "#64748b" }}
        >
          Code ▸
        </button>
      </div>
      {/* View content */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {view === "tokens" && <TokenEditor />}
        {view === "classes" && <ClassEditor />}
        {view === "variants" && <VariantEditor />}
      </div>
    </div>
  );
}
