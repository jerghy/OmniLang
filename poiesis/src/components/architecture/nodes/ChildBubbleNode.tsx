import { Handle, Position } from "@xyflow/react";
import type { NodeProps } from "@xyflow/react";
import type { BubbleLayer } from "../../../core/types/architecture";
import React from "react";
import nodeStyles from "./ChildBubbleNode.module.css";

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
  interface: { border: "var(--ol-color-info)", bg: "var(--ol-color-info-subtle)" },
  composition: { border: "var(--ol-color-success)", bg: "var(--ol-color-success-subtle)" },
  "store-deps": { border: "var(--ol-color-danger)", bg: "var(--ol-color-danger-subtle)" },
  "code-deps": { border: "var(--ol-color-primary)", bg: "var(--ol-color-primary-subtle)" },
  internal: { border: "var(--ol-text-secondary)", bg: "var(--ol-gray-50)" },
};

const TYPE_COLORS: Record<string, { border: string; bg: string }> = {
  ui: { border: "var(--ol-color-info)", bg: "var(--ol-color-info-subtle)" },
  store: { border: "var(--ol-color-warning)", bg: "var(--ol-color-warning-subtle)" },
  code: { border: "var(--ol-color-success)", bg: "var(--ol-color-success-subtle)" },
};

const ChildBubbleNode = React.memo(function ChildBubbleNode({ data }: NodeProps) {
  const d = data as unknown as ChildBubbleNodeData;
  const layerStyle = d.layer
    ? LAYER_COLORS[d.layer]
    : TYPE_COLORS[d.type] || TYPE_COLORS.ui;

  return (
    <div
      className={nodeStyles.node}
      style={{
        background: layerStyle.bg,
        border: `2px solid ${layerStyle.border}`,
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
          className={nodeStyles.badge}
          style={{
            background: layerStyle.border,
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
