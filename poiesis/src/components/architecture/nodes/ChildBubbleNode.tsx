import { Handle, Position } from "@xyflow/react";
import type { NodeProps } from "@xyflow/react";
import type { BubbleLayer } from "../../../core/types/architecture";
import React from "react";

export interface ChildBubbleNodeData {
  pid: string;
  type: "ui" | "store" | "code";
  label: string;
  depth: number;
  children: string[];
  expanded: boolean;
  isLeaf: boolean;
  parentId: string | null;
  layer?: BubbleLayer;
  childCount: number;
  [key: string]: unknown;
}

const LAYER_COLORS: Record<string, { border: string; bg: string }> = {
  interface: { border: "#3b82f6", bg: "#eff6ff" },
  composition: { border: "#10b981", bg: "#ecfdf5" },
  "store-deps": { border: "#ef4444", bg: "#fef2f2" },
  "code-deps": { border: "#8b5cf6", bg: "#f5f3ff" },
  internal: { border: "#6b7280", bg: "#f9fafb" },
};

const TYPE_COLORS: Record<string, { border: string; bg: string }> = {
  ui: { border: "#3b82f6", bg: "#eff6ff" },
  store: { border: "#f59e0b", bg: "#fffbeb" },
  code: { border: "#10b981", bg: "#ecfdf5" },
};

const ChildBubbleNode = React.memo(function ChildBubbleNode({ data }: NodeProps) {
  const d = data as unknown as ChildBubbleNodeData;
  const layerStyle = d.layer
    ? LAYER_COLORS[d.layer]
    : TYPE_COLORS[d.type] || TYPE_COLORS.ui;

  return (
    <div
      style={{
        padding: "8px 16px",
        borderRadius: 12,
        background: layerStyle.bg,
        border: `2px solid ${layerStyle.border}`,
        display: "flex",
        alignItems: "center",
        gap: 8,
        cursor: "pointer",
        fontSize: 13,
        fontWeight: 500,
        color: "#374151",
        transition: "box-shadow 0.2s ease, transform 0.2s ease",
        boxShadow: "0 1px 4px rgba(0,0,0,0.08)",
        position: "relative",
        minWidth: 100,
        maxWidth: 200,
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: layerStyle.border, width: 6, height: 6 }}
      />
      {d.isLeaf && (
        <span
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: layerStyle.border,
            flexShrink: 0,
          }}
        />
      )}
      {!d.isLeaf && (
        <span
          style={{
            width: 0,
            height: 0,
            borderLeft: "4px solid transparent",
            borderRight: "4px solid transparent",
            borderTop: `5px solid ${layerStyle.border}`,
            flexShrink: 0,
          }}
        />
      )}
      <span
        style={{
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          flex: 1,
        }}
      >
        {d.label}
      </span>
      {d.childCount > 0 && (
        <span
          style={{
            background: layerStyle.border,
            color: "#fff",
            fontSize: 10,
            fontWeight: 700,
            borderRadius: "50%",
            width: 18,
            height: 18,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          {d.childCount}
        </span>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: layerStyle.border, width: 6, height: 6 }}
      />
    </div>
  );
});
export { ChildBubbleNode };
