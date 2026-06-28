/** 组件画布头部条 — 编辑自定义组件时显示在 LayoutView 上方。 */

import { useEditorStore } from "../../../store/editor";
import { useComponentRegistry } from "../../../store/componentRegistry";

interface ComponentCanvasHeaderProps {
  onBack: () => void;
}

export function ComponentCanvasHeader({ onBack }: ComponentCanvasHeaderProps) {
  const activeDocId = useEditorStore((s) => s.activeDocId);
  const definitions = useComponentRegistry((s) => s.definitions);

  if (!activeDocId?.startsWith("comp:")) return null;

  const compName = activeDocId.replace("comp:", "");
  const def = definitions[compName];
  if (!def) return null;

  return (
    <div style={{
      padding: "8px 16px",
      background: "linear-gradient(135deg, #EEF2FF 0%, #F5F3FF 100%)",
      borderBottom: "2px solid #4F46E5",
      display: "flex",
      alignItems: "center",
      gap: 12,
      fontSize: 13,
      flexShrink: 0,
      zIndex: 10,
    }}>
      <button
        onClick={onBack}
        style={{
          padding: "4px 12px",
          border: "1px solid #C7D2FE",
          borderRadius: 4,
          background: "#fff",
          cursor: "pointer",
          fontSize: 12,
          fontWeight: 600,
          color: "#4F46E5",
        }}
      >
        ← 返回主画布
      </button>
      <span style={{ fontWeight: 600, color: "#1F2937" }}>
        组件编辑器: <span style={{ color: "#4F46E5" }}>{compName}</span>
      </span>
      <span style={{ color: "#6B7280", fontSize: 11 }}>
        {def.elements.length} 个元素
        {def.wires.length > 0 && ` / ${def.wires.length} 条连线`}
      </span>
      <span style={{ flex: 1 }} />
      <span style={{ fontSize: 10, color: "#9CA3AF" }}>
        {def.storage === "file" ? "📄 文件存储" : "⚙️ 配置存储"}
      </span>
    </div>
  );
}
