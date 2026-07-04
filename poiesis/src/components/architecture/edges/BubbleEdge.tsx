import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
} from "@xyflow/react";
import type { EdgeProps } from "@xyflow/react";
import type { BubbleEdge as BubbleEdgeType } from "../../../core/types/architecture";

const EDGE_COLORS: Record<string, string> = {
  composition: "#9ca3af",
  "store-read": "#3b82f6",
  "store-write": "#ef4444",
  "code-call": "#8b5cf6",
  event: "#f59e0b",
};

export function BubbleEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const d = data as unknown as BubbleEdgeType | undefined;
  const edgeColor = d?.edgeType
    ? EDGE_COLORS[d.edgeType] || "#9ca3af"
    : "#9ca3af";

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        style={{
          stroke: edgeColor,
          strokeWidth: d?.label ? 2 : 1.5,
          strokeDasharray: d?.edgeType === "composition" ? "5 5" : undefined,
        }}
      />
      {d?.label && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              background: "#fff",
              border: `1px solid ${edgeColor}`,
              borderRadius: 4,
              padding: "2px 8px",
              fontSize: 11,
              fontWeight: 500,
              color: edgeColor,
              pointerEvents: "none",
              whiteSpace: "nowrap",
            }}
          >
            {d.label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
