import { useMemo } from "react";
import type { NodeProps } from "@xyflow/react";
import { usePoiesisStore } from "../../../../store";
import { useCanvasStore } from "../../../canvas/canvas-store";
import { OmniViewElementTree } from "../../../design/OmniViewElementTree";
import type { OmniViewOutput } from "../../../design/useWasmCompile";
import styles from "./DesignPreviewNode.module.css";

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
      className={styles.node}
      style={{ width: previewW, height: previewH }}
    >
      {/* Title bar */}
      <div className={styles.titleBar}>
        预览 (双击退出)
      </div>

      {/* Preview content — scaled down, pointer-events disabled so ReactFlow handles drag */}
      <div className={styles.previewContent}>
        <div
          className={styles.scaleWrapper}
          style={{
            transform: `scale(${scale})`,
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
            <div className={styles.emptyState}>
              编译中...
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
