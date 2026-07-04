import { Handle, Position } from "@xyflow/react";
import type { NodeProps } from "@xyflow/react";

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
  ui: { bg: "#eff6ff", border: "#3b82f6", text: "#1e40af" },
  store: { bg: "#fffbeb", border: "#f59e0b", text: "#92400e" },
  code: { bg: "#ecfdf5", border: "#10b981", text: "#065f46" },
};

const ICONS: Record<string, string> = {
  ui: "🎨",
  store: "📄",
  code: "⚙️",
};

export function BubbleNode({ data }: NodeProps) {
  const d = data as unknown as BubbleNodeData;
  const colors = COLORS[d.type] || COLORS.ui;
  const icon = ICONS[d.type] || "🔵";
  const size = 120;

  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: "50%",
        background: colors.bg,
        border: `3px solid ${colors.border}`,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        cursor: "pointer",
        transition: "transform 0.3s ease, box-shadow 0.3s ease",
        transform: d.expanded ? "scale(1.1)" : "scale(1)",
        boxShadow: d.expanded
          ? `0 0 20px ${colors.border}40`
          : "0 2px 8px rgba(0,0,0,0.1)",
        position: "relative",
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
          style={{
            position: "absolute",
            top: -4,
            right: -4,
            background: colors.border,
            color: "#fff",
            fontSize: 10,
            fontWeight: 700,
            borderRadius: "50%",
            width: 22,
            height: 22,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
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
}
