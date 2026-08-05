// poiesis/src/components/canvas/code/nodes/ReturnNode.tsx
import { Handle, Position } from "@xyflow/react";
import React, { useMemo, useState } from "react";
import styles from "./ReturnNode.module.css";
import { NodeFormulaField } from "../fields/NodeFormulaField";
import { TokenPreview } from "../fields/TokenPreview";
import { buildFormulaSource, type FlowScope } from "../../../../services/codegen/flow-scope";

interface Props {
  data: {
    label: string;
    returnExpr?: string;
    // FlowCanvas 按 wire 拓扑注入所属 fn 的 scope/let 变量（公式框候选数据源）
    scope?: FlowScope;
    letVars?: Array<{ name: string }>;
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
  // 公式框候选源：与 SyntaxNode/ConditionNode 共用 buildFormulaSource（无 scope 退化纯文本）
  const formulaSource = useMemo(
    () => buildFormulaSource(data.scope, data.letVars ?? []),
    [data.scope, data.letVars],
  );

  return (
    <div
      className={`flow-node ${styles.node}`}
      data-node-type="return"
      onDoubleClick={data.onDoubleClick}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 4 }}>
        {editing ? (
          <NodeFormulaField
            value={expr}
            candidates={formulaSource}
            onCommit={(v) => { data.onCommitReturn?.(v); setEditing(false); }}
            onCancel={() => setEditing(false)}
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
            return{expr ? <> <TokenPreview value={expr} /></> : ""}
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
