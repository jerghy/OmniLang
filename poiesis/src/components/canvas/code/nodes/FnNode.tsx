import { Handle, Position } from "@xyflow/react";
import styles from "./FnNode.module.css";

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
      className={`flow-node ${styles.node}`}
      data-node-type="fn"
      onDoubleClick={data.onDoubleClick}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 4 }}>
        <span style={{ fontSize: 14 }}>ƒ</span>
        <span>{name}</span>
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
