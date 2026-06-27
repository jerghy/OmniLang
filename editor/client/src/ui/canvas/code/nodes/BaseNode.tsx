// editor/client/src/canvas/nodes/BaseNode.tsx

import { Handle, Position, type NodeProps } from "@xyflow/react";
import { getNodeConfig } from "../../shared/nodeRegistry";
import { TYPE_COLORS } from "../../shared/socketTypes";
import type { PortData } from "../../../../core/types";

export interface BaseNodeData {
  elementId: string;
  tag: string;
  label: string;
  typeAnnotation?: string;
  outputValue?: unknown;
  ports: PortData[];
  isFolded?: boolean;
}

/**
 * Shared node chrome. Renders icon + label + typed data handles with visible labels.
 * Pure data-flow: all ports are data-type, no flow handles.
 * Port colors are determined by data type (from TYPE_COLORS).
 */
export function BaseNode({ id, data, selected }: NodeProps<BaseNodeData>) {
  const nodeData = data;
  const config = getNodeConfig(nodeData.tag);
  const valueStr = nodeData.outputValue !== undefined
    ? ` → ${JSON.stringify(nodeData.outputValue)}`
    : "";
  const typeAnn = nodeData.typeAnnotation || "";

  const dataIn = nodeData.ports.filter((p) => p.direction === "in");
  const dataOut = nodeData.ports.filter((p) => p.direction === "out");

  return (
    <div
      style={{
        background: config.color.bg,
        border: `${selected ? 3 : 1.5}px solid ${selected ? "#6366F1" : config.color.border}`,
        borderRadius: 8,
        padding: "8px 12px",
        minWidth: 120,
        maxWidth: 260,
        fontFamily: "system-ui, sans-serif",
        fontSize: 12,
        boxShadow: selected
          ? "0 0 0 2px rgba(99,102,241,0.3), 0 4px 12px rgba(0,0,0,0.15)"
          : "0 1px 4px rgba(0,0,0,0.06)",
        position: "relative",
        transition: "box-shadow 0.15s, border-color 0.15s",
      }}
    >
      {/* Data input handles (left side) */}
      {dataIn.map((p, i) => (
        <DataHandle
          key={p.name}
          port={p}
          type="target"
          position={Position.Left}
          index={i}
          total={dataIn.length}
        />
      ))}

      {/* Node body */}
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 14 }}>{config.icon}</span>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 600, color: "#1F2937", fontSize: 11, lineHeight: 1.3 }}>
            {nodeData.label}
          </div>
          {(typeAnn || valueStr) && (
            <div style={{ fontSize: 9, color: "#6B7280", marginTop: 1 }}>
              {typeAnn && <span>: {typeAnn}</span>}
              <span>{valueStr}</span>
            </div>
          )}
        </div>
      </div>

      {/* Data output handles (right side) */}
      {dataOut.map((p, i) => (
        <DataHandle
          key={p.name}
          port={p}
          type="source"
          position={Position.Right}
          index={i}
          total={dataOut.length}
        />
      ))}
    </div>
  );
}

// ── Data Handle with visible port label ──

function DataHandle({ port, type, position, index, total }: {
  port: PortData; type: "source" | "target"; position: Position; index: number; total: number;
}) {
  const yOffset = total <= 1 ? 0 : (index - (total - 1) / 2) * 22;
  const typeColor = TYPE_COLORS[port.type] || TYPE_COLORS.any;
  const isLeft = position === Position.Left;

  return (
    <div
      style={{
        position: "absolute",
        [isLeft ? "left" : "right"]: -4,
        top: "50%",
        transform: `translateY(${yOffset - 6}px)`,
        display: "flex",
        alignItems: "center",
        gap: 4,
        flexDirection: isLeft ? "row" : "row-reverse",
      }}
    >
      {/* Port label — always visible */}
      <span
        style={{
          fontSize: 9,
          color: "#6B7280",
          fontFamily: "system-ui, sans-serif",
          whiteSpace: "nowrap",
          userSelect: "none",
          pointerEvents: "none",
        }}
      >
        {port.label || port.name}
      </span>
      {/* Handle dot */}
      <Handle
        id={port.name}
        type={type}
        position={position}
        style={{
          width: 10,
          height: 10,
          background: typeColor,
          border: "2px solid #fff",
          borderRadius: "50%",
          cursor: "crosshair",
          position: "relative",
          transform: "none",
        }}
        title={`${port.label || port.name}: ${port.type}`}
      />
    </div>
  );
}
