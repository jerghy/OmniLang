import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    fnName: string;
    params?: string;
  };
}

export function FunctionHeaderNode({ data }: Props) {
  const displayName = data.fnName || data.label;
  return (
    <div
      className="flow-node"
      data-node-type="function-header"
      style={{
        background: "#EEF2FF",
        border: "2px solid #6366F1",
        borderRadius: "12px 12px 4px 4px",
        padding: "8px 16px",
        minWidth: 160,
        textAlign: "center",
        fontSize: 13,
        fontWeight: 600,
      }}
    >
      <div>
        <span style={{ fontSize: 14, marginRight: 4 }}>ƒ</span>
        {displayName}
      </div>
      {data.params && (
        <div style={{ fontSize: 11, color: "#6366F1", marginTop: 2 }}>
          ({data.params})
        </div>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#6366F1" }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="data-out"
        style={{ background: "#818CF8" }}
      />
    </div>
  );
}
