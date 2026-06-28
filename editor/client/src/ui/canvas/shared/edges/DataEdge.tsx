// editor/client/src/canvas/edges/DataEdge.tsx

import { getSmoothStepPath, type EdgeProps } from "@xyflow/react";

/** Color map for data types (matches socketTypes.ts TYPE_COLORS). */
const DATA_TYPE_COLORS: Record<string, string> = {
  any: "#6B7280",
  i32: "#2563EB",
  f64: "#059669",
  bool: "#D97706",
  String: "#DB2777",
  void: "#9CA3AF",
  MouseEvent: "#DC2626",
};

/**
 * Data edge — colored by data type with circle dot marker.
 * Represents value passing between node ports.
 */
export function DataEdge({
  id: _id,
  sourceX, sourceY,
  targetX, targetY,
  sourcePosition, targetPosition,
  data,
  selected,
}: EdgeProps) {
  const [edgePath] = getSmoothStepPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
    borderRadius: 8,
  });

  // data.type is set when creating the edge in FlowCanvas
  const dataType = typeof data === "object" && data !== null ? String((data as Record<string, unknown>).type ?? "any") : "any";
  const color = DATA_TYPE_COLORS[dataType] || DATA_TYPE_COLORS.any;

  return (
    <>
      {selected && (
        <path
          d={edgePath}
          fill="none"
          stroke={color}
          strokeWidth={5}
          strokeLinecap="round"
          opacity={0.2}
        />
      )}
      <path
        d={edgePath}
        fill="none"
        stroke={color}
        strokeWidth={selected ? 2.5 : 2}
        strokeLinecap="round"
      />
      {/* Circle dot marker at target end */}
      <circle
        cx={targetX}
        cy={targetY}
        r={selected ? 5 : 4}
        fill={color}
        stroke="#fff"
        strokeWidth={1.5}
      />
    </>
  );
}
