import { Handle, Position } from "@xyflow/react";
import styles from "./ReturnNode.module.css";

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
      className={`flow-node ${styles.node}`}
      data-node-type="return"
      onDoubleClick={data.onDoubleClick}
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
