// editor/client/src/canvas/edges/FlowEdge.tsx

import { getSmoothStepPath, type EdgeProps } from "@xyflow/react";

/**
 * Flow edge — white/gray thick line with triangle arrowhead.
 * Represents execution order (sequence flow) between nodes.
 */
export function FlowEdge({
  id,
  sourceX, sourceY,
  targetX, targetY,
  sourcePosition, targetPosition,
  selected,
}: EdgeProps) {
  const [edgePath] = getSmoothStepPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
    borderRadius: 8,
  });

  return (
    <>
      {/* Shadow/glow for selected state */}
      {selected && (
        <path
          d={edgePath}
          fill="none"
          stroke="#A5B4FC"
          strokeWidth={6}
          strokeLinecap="round"
          opacity={0.4}
        />
      )}
      <path
        d={edgePath}
        fill="none"
        stroke={selected ? "#6366F1" : "#9CA3AF"}
        strokeWidth={selected ? 3 : 2.5}
        strokeLinecap="round"
      />
      {/* Triangle arrowhead marker */}
      <marker
        id={`flow-arrow-${id}`}
        viewBox="0 0 10 8"
        refX="9" refY="4"
        markerWidth="8" markerHeight="6"
        orient="auto-start-reverse"
      >
        <path d="M 0 0 L 10 4 L 0 8 z" fill={selected ? "#6366F1" : "#9CA3AF"} />
      </marker>
      <path
        d={edgePath}
        fill="none"
        stroke="transparent"
        strokeWidth={1}
        markerEnd={`url(#flow-arrow-${id})`}
      />
    </>
  );
}
