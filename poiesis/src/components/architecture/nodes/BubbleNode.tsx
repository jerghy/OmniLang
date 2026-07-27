import { Handle, Position } from "@xyflow/react";
import type { NodeProps } from "@xyflow/react";
import React from "react";
import nodeStyles from "./BubbleNode.module.css";

export interface BubbleNodeData {
  pid: string;
  type: "ui" | "store" | "code";
  label: string;
  depth: number;
  children: string[];
  expanded: boolean;
  isLeaf: boolean;
  parentId: string | null;
  childCount: number;
  [key: string]: unknown;
}

const COLORS: Record<string, { bg: string; border: string; text: string }> = {
  ui: { bg: "var(--ol-color-info-subtle)", border: "var(--ol-color-info)", text: "var(--ol-color-info-active)" },
  store: { bg: "var(--ol-color-warning-subtle)", border: "var(--ol-color-warning)", text: "var(--ol-color-warning-active)" },
  code: { bg: "var(--ol-color-success-subtle)", border: "var(--ol-color-success)", text: "var(--ol-color-success-active)" },
};

const ICONS: Record<string, string> = {
  ui: "🎨",
  store: "📄",
  code: "⚙️",
};

const BubbleNode = React.memo(function BubbleNode({ data }: NodeProps) {
  const d = data as unknown as BubbleNodeData;
  const colors = COLORS[d.type] || COLORS.ui;
  const icon = ICONS[d.type] || "🔵";
  return (
    <div
      className={nodeStyles.node}
      style={{
        background: colors.bg,
        border: `3px solid ${colors.border}`,
        transform: d.expanded ? "scale(1.1)" : "scale(1)",
        boxShadow: d.expanded
          ? `0 0 20px ${colors.border}40`
          : "0 2px 8px rgba(0,0,0,0.1)",
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: colors.border, width: 8, height: 8 }}
      />
      <span style={{ fontSize: 28, lineHeight: 1 }}>{icon}</span>
      <span
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: colors.text,
          marginTop: 4,
          textAlign: "center",
          lineHeight: 1.2,
        }}
      >
        {d.label}
      </span>
      {d.childCount > 0 && (
        <span
          className={nodeStyles.badge}
          style={{
            background: colors.border,
          }}
        >
          {d.childCount}
        </span>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: colors.border, width: 8, height: 8 }}
      />
    </div>
  );
});
export { BubbleNode };
