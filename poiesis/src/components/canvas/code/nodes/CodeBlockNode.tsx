import { Handle, Position } from "@xyflow/react";
import React from "react";

interface Props {
  data: {
    label: string;
    source?: string;
    description?: string;
    onDoubleClick?: () => void;
  };
}

const TAG_LABELS: Record<string, string> = {
  "code-block": "代码块",
};

const CodeBlockNode = React.memo(function CodeBlockNode({ data }: Props) {
  const chineseLabel = TAG_LABELS[data.label] || data.label;
  const preview = data.source
    ? data.source.length > 50
      ? data.source.slice(0, 50) + "..."
      : data.source
    : chineseLabel;

  return (
    <div
      className="flow-node"
      data-node-type="code-block"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#F8FAFC",
        border: "1.5px solid #94A3B8",
        borderRadius: 6,
        padding: "10px 14px",
        minWidth: 180,
        maxWidth: 280,
        fontSize: 12,
        fontFamily: "monospace",
        cursor: "pointer",
        whiteSpace: "pre-wrap",
        wordBreak: "break-all",
        color: "#334155",
        lineHeight: 1.4,
      }}
    >
      <code>{preview}</code>
      {data.description && (
        <div
          style={{
            marginTop: 6,
            paddingTop: 6,
            borderTop: "1px solid #E2E8F0",
            fontSize: 12,
            color: "#64748B",
            fontStyle: "italic",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {data.description}
        </div>
      )}
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#94A3B8" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#94A3B8" }}
      />
    </div>
  );
});
export { CodeBlockNode };
