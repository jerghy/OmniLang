import { Handle, Position } from "@xyflow/react";
import React, { useState } from "react";
import { NodeTextField } from "../fields/NodeField";

interface BranchData {
  id: string;
  type: "if" | "elif" | "else";
  condition: string;
}

interface Props {
  data: {
    label: string;
    branches?: BranchData[];
    onDoubleClick?: () => void;
    onUpdateBranches?: (branches: BranchData[]) => void;
    onEditCondition?: (branchId: string, condition: string) => void;
  };
}

const TYPE_CN: Record<string, string> = { if: "如果", elif: "否则如果", else: "否则" };

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

export function ConditionNode({ data }: Props) {
  const branches: BranchData[] = data.branches || [{ id: "br0", type: "if", condition: "" }];
  const [editingBranch, setEditingBranch] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const handleAddBranch = (type: "elif" | "else") => {
    const newBranches = [...branches, {
      id: `br${branches.length}`,
      type,
      condition: "",
    }];
    data.onUpdateBranches?.(newBranches);
  };

  const handleEditStart = (br: BranchData) => {
    setEditingBranch(br.id);
    setEditValue(br.condition);
  };

  const hasElse = branches.some((b) => b.type === "else");

  return (
    <div
      className="flow-node"
      data-node-type="condition"
      style={{
        background: "var(--ol-color-warning-subtle)",
        border: "2px solid var(--ol-color-warning)",
        borderRadius: 8,
        padding: "6px 0",
        minWidth: 200,
        fontSize: 12,
      }}
    >
      <HandlesBoundary>
        <Handle
          type="target"
          position={Position.Top}
          id="flow-in"
          style={{ background: "var(--ol-color-warning)" }}
        />
      </HandlesBoundary>

      <div style={{ padding: "0 10px", fontWeight: 600, marginBottom: 4, color: "var(--ol-color-warning-active)" }}>
        ◆ 判断
      </div>

      {branches.map((br, i) => (
        <div
          key={br.id}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "3px 10px",
            borderTop: i > 0 ? "1px solid var(--ol-color-warning-muted)" : "none",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
            <span style={{
              fontSize: 12,
              fontWeight: 600,
              color: br.type === "if" ? "var(--ol-color-success-active)" : br.type === "elif" ? "var(--ol-color-warning)" : "var(--ol-color-danger)",
              minWidth: 36,
            }}>
              {TYPE_CN[br.type]}
            </span>
            {editingBranch === br.id ? (
              <NodeTextField
                value={editValue}
                onCommit={(v) => { data.onEditCondition?.(br.id, v); setEditingBranch(null); }}
                onCancel={() => setEditingBranch(null)}
                placeholder="条件表达式"
                ariaLabel="编辑条件表达式"
              />
            ) : (
              <button
                type="button"
                tabIndex={0}
                onClick={() => br.type !== "else" && handleEditStart(br)}
                onKeyDown={(e) => { if ((e.key === 'Enter' || e.key === ' ') && br.type !== "else") { e.preventDefault(); handleEditStart(br); } }}
                style={{
                  fontFamily: "monospace",
                  fontSize: 12,
                  color: br.type === "else" ? "var(--ol-text-muted)" : "var(--ol-gray-800)",
                  cursor: br.type === "else" ? "default" : "pointer",
                  flex: 1,
                  background: "none",
                  border: "none",
                  textAlign: "left",
                  padding: 0,
                }}
                disabled={br.type === "else"}
              >
                {br.type === "else" ? "" : br.condition || "..."}
              </button>
            )}
          </div>

          <HandlesBoundary>
            <Handle
              type="source"
              position={Position.Right}
              id={br.id}
              style={{ background: "var(--ol-color-warning)", position: "relative", right: -6, transform: "none" }}
            />
          </HandlesBoundary>
        </div>
      ))}

      {/* Add branch buttons */}
      <div style={{ display: "flex", gap: 4, padding: "4px 10px 0", borderTop: "1px solid var(--ol-color-warning-muted)" }}>
        {!hasElse && (
          <>
            <button type="button"
              onClick={() => handleAddBranch("elif")}
              style={{
                fontSize: 12, padding: "2px 6px", border: "1px solid var(--ol-color-warning)",
                borderRadius: 3, background: "var(--ol-surface)", color: "#B45309", cursor: "pointer",
              }}
            >
              + elif
            </button>
            <button type="button"
              onClick={() => handleAddBranch("else")}
              style={{
                fontSize: 12, padding: "2px 6px", border: "1px solid var(--ol-color-danger)",
                borderRadius: 3, background: "var(--ol-surface)", color: "var(--ol-color-danger)", cursor: "pointer",
              }}
            >
              + else
            </button>
          </>
        )}
      </div>
    </div>
  );
}
