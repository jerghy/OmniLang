// editor/client/src/canvas/nodes/BaseNode.tsx

import { Handle, Position, type NodeProps } from "@xyflow/react";
import { getNodeConfig } from "../nodeRegistry";
import type { PortData } from "../../store/editor";

export interface BaseNodeData {
  elementId: string;
  tag: string;
  label: string;
  typeAnnotation?: string;
  outputValue?: unknown;
  ports: PortData[];
  isFolded?: boolean;
  onDoubleClick?: (elementId: string) => void;
}

/**
 * Shared node chrome. Renders icon + label + typed handles.
 * Flow ports (portType="flow") get diamond Handle style.
 * Data ports (portType="data" or undefined) get circle Handle style.
 */
export function BaseNode({ id, data, selected }: NodeProps) {
  const nodeData = data as unknown as BaseNodeData;
  const config = getNodeConfig(nodeData.tag);
  const valueStr = nodeData.outputValue !== undefined
    ? ` → ${JSON.stringify(nodeData.outputValue)}`
    : "";
  const typeAnn = nodeData.typeAnnotation || "";

  // Split ports by direction and type
  const flowIn = nodeData.ports.filter(p => p.direction === "in" && p.portType === "flow");
  const flowOut = nodeData.ports.filter(p => p.direction === "out" && p.portType === "flow");
  const dataIn = nodeData.ports.filter(p => p.direction === "in" && p.portType !== "flow");
  const dataOut = nodeData.ports.filter(p => p.direction === "out" && p.portType !== "flow");

  const handleDblClick = () => {
    nodeData.onDoubleClick?.(nodeData.elementId);
  };

  return (
    <div
      onDoubleClick={handleDblClick}
      style={{
        background: config.color.bg,
        border: `${selected ? 3 : 2}px solid ${config.color.border}`,
        borderRadius: 10,
        padding: "10px 16px",
        minWidth: 140,
        maxWidth: 260,
        fontFamily: "system-ui, sans-serif",
        fontSize: 12,
        boxShadow: selected
          ? "0 0 0 2px rgba(99,102,241,0.3), 0 4px 12px rgba(0,0,0,0.15)"
          : "0 2px 6px rgba(0,0,0,0.08)",
        position: "relative",
        transition: "box-shadow 0.15s",
      }}
    >
      {/* Flow input handles (left side, diamond shape) */}
      {flowIn.map((p, i) => (
        <FlowHandle
          key={p.name}
          port={p}
          type="target"
          position={Position.Left}
          index={i}
          total={flowIn.length}
        />
      ))}

      {/* Data input handles (left side, circle) */}
      {dataIn.map((p, i) => (
        <DataHandle
          key={p.name}
          port={p}
          type="target"
          position={Position.Left}
          index={i + flowIn.length}
          total={flowIn.length + dataIn.length}
        />
      ))}

      {/* Node body */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ fontSize: 16 }}>{config.icon}</span>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 600, color: "#1F2937", fontSize: 12, lineHeight: 1.3 }}>
            {nodeData.label}
          </div>
          {(typeAnn || valueStr) && (
            <div style={{ fontSize: 9, color: "#6B7280", marginTop: 2 }}>
              {typeAnn && <span>: {typeAnn}</span>}
              <span>{valueStr}</span>
            </div>
          )}
        </div>
      </div>

      {/* Flow output handles (right side, diamond shape) */}
      {flowOut.map((p, i) => (
        <FlowHandle
          key={p.name}
          port={p}
          type="source"
          position={Position.Right}
          index={i}
          total={flowOut.length}
        />
      ))}

      {/* Data output handles (right side, circle) */}
      {dataOut.map((p, i) => (
        <DataHandle
          key={p.name}
          port={p}
          type="source"
          position={Position.Right}
          index={i + flowOut.length}
          total={flowOut.length + dataOut.length}
        />
      ))}
    </div>
  );
}

// ── Handle sub-components ──

function FlowHandle({ port, type, position, index, total }: {
  port: PortData; type: "source" | "target"; position: Position; index: number; total: number;
}) {
  const yOffset = total <= 1 ? 0 : (index - (total - 1) / 2) * 24;
  return (
    <Handle
      id={port.name}
      type={type}
      position={position}
      style={{
        width: 10,
        height: 10,
        background: "#9CA3AF",
        border: "2px solid #fff",
        borderRadius: 2,
        transform: `translateY(${yOffset}px) rotate(45deg)`,
        cursor: "crosshair",
      }}
      title={`${port.label || port.name} (flow${port.type !== "void" ? `: ${port.type}` : ""})`}
    />
  );
}

function DataHandle({ port, type, position, index, total }: {
  port: PortData; type: "source" | "target"; position: Position; index: number; total: number;
}) {
  const yOffset = total <= 1 ? 0 : (index - (total - 1) / 2) * 24;
  const typeColor = getTypeColor(port.type);
  return (
    <Handle
      id={port.name}
      type={type}
      position={position}
      style={{
        width: 12,
        height: 12,
        background: typeColor,
        border: "2px solid #fff",
        borderRadius: "50%",
        transform: `translateY(${yOffset}px)`,
        cursor: "crosshair",
      }}
      title={`${port.label || port.name} (${port.type})`}
    />
  );
}

function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    any: "#6B7280", i32: "#2563EB", f64: "#059669",
    bool: "#D97706", String: "#DB2777", void: "#9CA3AF",
    MouseEvent: "#DC2626",
  };
  return colors[type] || colors.any;
}
