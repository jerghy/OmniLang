import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    fnName?: string;
    params?: string;
    onDoubleClick?: () => void;
  };
}

export function FnNode({ data }: Props) {
  const name = data.fnName || data.label || "fn";
  const params = data.params || "";

  return (
    <div
      className="flow-node"
      data-node-type="fn"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#EEF2FF",
        border: "2px solid #6366F1",
        borderRadius: "12px 12px 4px 4px",
        padding: "10px 18px",
        minWidth: 160,
        textAlign: "center",
        fontSize: 13,
        fontWeight: 600,
        color: "#3730A3",
        cursor: "pointer",
      }}
    >
      <div>
        <span style={{ fontSize: 14, marginRight: 4 }}>ƒ</span>
        {name}
        {params && <span style={{ fontWeight: 400 }}>({params})</span>}
      </div>
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#6366F1" }}
      />
    </div>
  );
}
