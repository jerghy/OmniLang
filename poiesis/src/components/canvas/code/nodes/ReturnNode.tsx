import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    returnExpr?: string;
    onDoubleClick?: () => void;
  };
}

export function ReturnNode({ data }: Props) {
  const expr = data.returnExpr || "";

  return (
    <div
      className="flow-node"
      data-node-type="return"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#FFF1F2",
        border: "2px solid #F43F5E",
        borderRadius: "4px 4px 12px 12px",
        padding: "8px 14px",
        minWidth: 120,
        textAlign: "center",
        fontSize: 12,
        fontWeight: 600,
        color: "#BE123C",
        cursor: "pointer",
      }}
    >
      <div>return{expr ? ` ${expr}` : ""}</div>
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#F43F5E" }}
      />
    </div>
  );
}
