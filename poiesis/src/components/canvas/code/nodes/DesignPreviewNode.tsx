import { useMemo } from "react";
import type { NodeProps } from "@xyflow/react";
import { usePoiesisStore } from "../../../../store";
import { useCanvasStore } from "../../../canvas/canvas-store";
import { OmniViewElementTree } from "../../../design/OmniViewElementTree";
import type { OmniViewOutput } from "../../../design/useWasmCompile";

interface DesignPreviewNodeData {
  output: OmniViewOutput | null;
}

/** Max bounding box for the preview node inside the flow canvas. */
const MAX_W = 240;
const MAX_H = 360;

export function DesignPreviewNode({
  data,
}: NodeProps & { data: DesignPreviewNodeData }) {
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const devicePreset = useCanvasStore((s) => s.devicePreset);

  // Derive preview dimensions from device preset, preserving aspect ratio
  const { previewW, previewH, scale } = useMemo(() => {
    const dw = devicePreset.width || 393;
    const dh = devicePreset.height || 852;
    const s = Math.min(MAX_W / dw, MAX_H / dh);
    return { previewW: Math.round(dw * s), previewH: Math.round(dh * s), scale: s };
  }, [devicePreset.width, devicePreset.height]);

  return (
    <div
      onDoubleClick={() => toggleCodeFlow(null)}
      style={{
        background: "#fff",
        border: "1.5px solid #6366f1",
        borderRadius: 8,
        boxShadow: "0 2px 12px rgba(99, 102, 241, 0.15)",
        width: previewW,
        height: previewH,
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
            transform: `scale(${scale})`,
            transformOrigin: "top left",
            width: devicePreset.width || 393,
            height: devicePreset.height || 852,
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
