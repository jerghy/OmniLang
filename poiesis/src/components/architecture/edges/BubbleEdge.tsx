import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
} from "@xyflow/react";
import type { EdgeProps } from "@xyflow/react";
import type { BubbleEdge as BubbleEdgeType } from "../../../core/types/architecture";
import edgeStyles from "./BubbleEdge.module.css";

const EDGE_COLORS: Record<string, string> = {
  composition: "var(--ol-text-muted)",
  "store-read": "var(--ol-color-info)",
  "store-write": "var(--ol-color-danger)",
  "code-call": "var(--ol-color-primary)",
  event: "var(--ol-color-warning)",
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
    ? EDGE_COLORS[d.edgeType] || "var(--ol-text-muted)"
    : "var(--ol-text-muted)";

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
            className={edgeStyles.edgeLabel}
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              border: `1px solid ${edgeColor}`,
              color: edgeColor,
            }}
          >
            {d.label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
