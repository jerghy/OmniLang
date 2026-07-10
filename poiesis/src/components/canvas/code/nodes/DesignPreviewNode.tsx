import type { NodeProps } from "@xyflow/react";
import { usePoiesisStore } from "../../../../store";
import { OmniViewElementTree } from "../../../design/OmniViewElementTree";
import type { OmniViewOutput } from "../../../design/useWasmCompile";

interface DesignPreviewNodeData {
  output: OmniViewOutput | null;
}

export function DesignPreviewNode({
  data,
}: NodeProps & { data: DesignPreviewNodeData }) {
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);

  return (
    <div
      onDoubleClick={() => toggleCodeFlow(null)}
      style={{
        background: "#fff",
        border: "1.5px solid #6366f1",
        borderRadius: 8,
        boxShadow: "0 2px 12px rgba(99, 102, 241, 0.15)",
        width: 240,
        height: 360,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        cursor: "grab",
      }}
    >
      {/* Title bar */}
      <div
        style={{
          padding: "4px 8px",
          background: "#EEF2FF",
          borderBottom: "1px solid #C7D2FE",
          fontSize: 10,
          fontWeight: 600,
          color: "#4338CA",
          display: "flex",
          alignItems: "center",
          gap: 4,
          cursor: "grab",
          flexShrink: 0,
        }}
      >
        预览 (双击退出)
      </div>

      {/* Preview content — scaled down, pointer-events disabled so ReactFlow handles drag */}
      <div
        style={{
          flex: 1,
          overflow: "hidden",
          pointerEvents: "none",
        }}
      >
        <div
          style={{
            transform: "scale(0.2)",
            transformOrigin: "top left",
            width: 1200,
            height: 1800,
          }}
        >
          {data.output ? (
            <OmniViewElementTree
              output={data.output}
              selectedNid={null}
              onSelectNid={() => {}}
              onDoubleClickNid={() => {}}
            />
          ) : (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                height: "100%",
                color: "#94a3b8",
                fontSize: 13,
              }}
            >
              编译中...
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
