// poiesis/src/components/canvas/code/nodes/TryNode.tsx
// try 块节点：对标 ConditionNode 分支行模式——每分支一行（行首 label + catchVar 行内
// 公式框 + source handle id=br.id），底部 flow-out source handle 接块后继。
// 语义约束：try 必需；catch ≤ 1；finally ≤ 1；至少 catch/finally 其一
// （`+ catch`/`+ finally` 按钮按 hasCatch/hasFinally 禁用）。

import { Handle, Position } from "@xyflow/react";
import React, { useMemo, useState } from "react";
import { NodeFormulaField } from "../fields/NodeFormulaField";
import { TokenPreview } from "../fields/TokenPreview";
import { buildFormulaSource, type FlowScope } from "../../../../services/codegen/flow-scope";
import "./node-base.css";

interface TryBranchData {
  id: string;
  type: "try" | "catch" | "finally";
  catchVar?: string;
}

interface Props {
  data: {
    label: string;
    branches?: TryBranchData[];
    scope?: FlowScope;
    letVars?: Array<{ name: string }>;
    onUpdateBranches?: (branches: TryBranchData[]) => void;
    onEditCatchVar?: (branchId: string, catchVar: string) => void;
  };
}

const TYPE_CN: Record<string, string> = { try: "尝试", catch: "捕获", finally: "收尾" };

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

export function TryNode({ data }: Props) {
  const { scope, letVars } = data;
  const branches: TryBranchData[] = data.branches || [{ id: "br0", type: "try" }];
  const [editingBranch, setEditingBranch] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const formulaSource = useMemo(() => buildFormulaSource(scope, letVars ?? []), [scope, letVars]);

  const handleAddBranch = (type: "catch" | "finally") => {
    const nextId =
      branches.reduce(
        (max, b) => Math.max(max, parseInt((b.id ?? "br0").replace(/^br/, ""), 10) || 0),
        0,
      ) + 1;
    data.onUpdateBranches?.([...branches, { id: `br${nextId}`, type }]);
  };

  const handleEditStart = (br: TryBranchData) => {
    setEditingBranch(br.id);
    setEditValue(br.catchVar ?? "");
  };

  const hasCatch = branches.some((b) => b.type === "catch");
  const hasFinally = branches.some((b) => b.type === "finally");

  return (
    <div
      className="flow-node codeflow-node"
      data-node-type="try"
      style={{
        background: "var(--ol-color-danger-subtle)",
        border: "2px solid var(--ol-color-danger)",
        borderRadius: 8,
        padding: "6px 0",
        minWidth: 200,
        fontSize: 12,
      }}
    >
      <HandlesBoundary>
        <Handle
          className="codeflow-handle"
          type="target"
          position={Position.Top}
          id="flow-in"
          style={{ background: "var(--ol-color-danger)" }}
        />
      </HandlesBoundary>

      <div style={{ padding: "0 10px", fontWeight: 600, marginBottom: 4, color: "var(--ol-color-danger-active)" }}>
        ⚠ {data.label}
      </div>

      {branches.map((br, i) => (
        <div
          key={br.id}
          data-testid="try-branch-row"
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "3px 10px",
            borderTop: i > 0 ? "1px solid var(--ol-color-danger-muted)" : "none",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1, minWidth: 0 }}>
            <span
              style={{
                fontSize: 12,
                fontWeight: 600,
                minWidth: 36,
                color:
                  br.type === "try"
                    ? "var(--ol-color-success-active)"
                    : br.type === "catch"
                      ? "var(--ol-color-danger)"
                      : "var(--ol-color-warning)",
              }}
            >
              {TYPE_CN[br.type]}
            </span>
            {br.type === "catch" &&
              (editingBranch === br.id ? (
                <NodeFormulaField
                  value={editValue}
                  candidates={formulaSource}
                  onCommit={(v) => {
                    data.onEditCatchVar?.(br.id, v);
                    setEditingBranch(null);
                  }}
                  onCancel={() => setEditingBranch(null)}
                  placeholder="catch 变量"
                  ariaLabel="编辑 catch 变量"
                />
              ) : (
                <button
                  type="button"
                  tabIndex={0}
                  onClick={() => handleEditStart(br)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      handleEditStart(br);
                    }
                  }}
                  style={{
                    fontFamily: "monospace",
                    fontSize: 12,
                    color: "var(--ol-gray-800)",
                    cursor: "pointer",
                    flex: 1,
                    background: "none",
                    border: "none",
                    textAlign: "left",
                    padding: 0,
                    minWidth: 0,
                  }}
                >
                  {br.catchVar ? (
                    <TokenPreview value={br.catchVar} />
                  ) : (
                    <span style={{ color: "var(--ol-text-muted)" }}>...</span>
                  )}
                </button>
              ))}
          </div>

          <HandlesBoundary>
            <Handle
              className="codeflow-handle"
              type="source"
              position={Position.Right}
              id={br.id}
              style={{ background: "var(--ol-color-danger)", position: "relative", right: -6, transform: "none" }}
            />
          </HandlesBoundary>
        </div>
      ))}

      <div style={{ display: "flex", gap: 4, padding: "4px 10px 0", borderTop: "1px solid var(--ol-color-danger-muted)" }}>
        <button
          type="button"
          data-testid="add-catch"
          disabled={hasCatch}
          onClick={() => handleAddBranch("catch")}
          style={{
            fontSize: 12,
            padding: "2px 6px",
            border: "1px solid var(--ol-color-danger)",
            borderRadius: 3,
            background: "var(--ol-surface)",
            color: "var(--ol-color-danger)",
            cursor: hasCatch ? "not-allowed" : "pointer",
            opacity: hasCatch ? 0.5 : 1,
          }}
        >
          + catch
        </button>
        <button
          type="button"
          data-testid="add-finally"
          disabled={hasFinally}
          onClick={() => handleAddBranch("finally")}
          style={{
            fontSize: 12,
            padding: "2px 6px",
            border: "1px solid var(--ol-color-danger)",
            borderRadius: 3,
            background: "var(--ol-surface)",
            color: "var(--ol-color-danger)",
            cursor: hasFinally ? "not-allowed" : "pointer",
            opacity: hasFinally ? 0.5 : 1,
          }}
        >
          + finally
        </button>
      </div>

      <HandlesBoundary>
        <Handle
          className="codeflow-handle"
          type="source"
          position={Position.Bottom}
          id="flow-out"
          style={{ background: "var(--ol-text-muted)" }}
        />
      </HandlesBoundary>
    </div>
  );
}
