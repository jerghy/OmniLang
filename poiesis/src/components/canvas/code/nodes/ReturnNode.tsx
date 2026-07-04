import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    returnType?: string;
  };
}

export function ReturnNode({ data }: Props) {
  return (
    <div
      className="flow-node"
      data-node-type="return"
      style={{
        background: "#ECFDF5",
        border: "2px solid #10B981",
        borderRadius: "4px 4px 12px 12px",
        padding: "8px 20px",
        minWidth: 120,
        textAlign: "center",
        fontSize: 13,
        fontWeight: 600,
        color: "#065F46",
      }}
    >
      <div>{'↩'} return</div>
      {data.returnType && (
        <div style={{ fontSize: 11, color: "#10B981", marginTop: 2 }}>
          {data.returnType}
        </div>
      )}
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#10B981" }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="data-in"
        style={{ background: "#6EE7B7" }}
        title="return value"
      />
    </div>
  );
}
