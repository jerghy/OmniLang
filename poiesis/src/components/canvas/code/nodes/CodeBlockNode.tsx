import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    source?: string;
    onDoubleClick?: () => void;
  };
}

export function CodeBlockNode({ data }: Props) {
  const preview = data.source
    ? data.source.length > 50
      ? data.source.slice(0, 50) + "..."
      : data.source
    : data.label || "// 代码块";

  return (
    <div
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#F8FAFC",
        border: "1.5px solid #94A3B8",
        borderRadius: 6,
        padding: "10px 14px",
        minWidth: 180,
        maxWidth: 280,
        fontSize: 11,
        fontFamily: "monospace",
        cursor: "pointer",
        whiteSpace: "pre-wrap",
        wordBreak: "break-all",
        color: "#334155",
        lineHeight: 1.4,
      }}
    >
      <code>{preview}</code>
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
      <Handle
        type="target"
        position={Position.Left}
        id="data-in"
        style={{ background: "#CBD5E1" }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="data-out"
        style={{ background: "#CBD5E1" }}
      />
    </div>
  );
}
