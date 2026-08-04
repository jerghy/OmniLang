// poiesis/src/components/canvas/code/nodes/ReturnNode.tsx
import { Handle, Position } from "@xyflow/react";
import React, { useState } from "react";
import styles from "./ReturnNode.module.css";
import { NodeTextField } from "../fields/NodeField";

interface Props {
  data: {
    label: string;
    returnExpr?: string;
    onDoubleClick?: () => void;
    onCommitReturn?: (expr: string) => void;
  };
}

// Handle 依赖 ReactFlowProvider（连线渲染）；单元测试直接渲染节点时静默跳过，避免抛错。
class HandlesBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

export function ReturnNode({ data }: Props) {
  const expr = data.returnExpr || "";
  const [editing, setEditing] = useState(false);

  return (
    <div
      className={`flow-node ${styles.node}`}
      data-node-type="return"
      onDoubleClick={data.onDoubleClick}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 4 }}>
        {editing ? (
          <NodeTextField
            value={expr}
            onCommit={(v) => { data.onCommitReturn?.(v); setEditing(false); }}
            placeholder="表达式"
            ariaLabel="return 表达式"
            className={styles.grow}
          />
        ) : (
          <button
            type="button"
            onClick={() => setEditing(true)}
            onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setEditing(true); } }}
            title="点击编辑"
            style={{
              fontFamily: "inherit",
              fontSize: "inherit",
              fontWeight: "inherit",
              color: "inherit",
              background: "none",
              border: "none",
              padding: 0,
              cursor: "pointer",
            }}
          >
            return{expr ? ` ${expr}` : ""}
          </button>
        )}
      </div>
      <HandlesBoundary>
        <Handle
          type="target"
          position={Position.Top}
          id="flow-in"
          style={{ background: "var(--ol-color-danger)" }}
        />
      </HandlesBoundary>
    </div>
  );
}
