import { Handle, Position } from "@xyflow/react";
import React, { useMemo, useState } from "react";
import { NodeFormulaField } from "../fields/NodeFormulaField";
import { TokenPreview } from "../fields/TokenPreview";
import { buildFormulaSource, type FlowScope } from "../../../../services/codegen/flow-scope";
import "./node-base.css";

interface BranchData {
  id: string;
  type: "if" | "elif" | "else";
  condition: string;
}

interface Props {
  data: {
    label: string;
    branches?: BranchData[];
    // FlowCanvas 按 wire 拓扑注入所属 fn 的 scope/let 变量（公式框候选数据源）
    scope?: FlowScope;
    letVars?: Array<{ name: string }>;
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
  const { scope, letVars } = data;
  const branches: BranchData[] = data.branches || [{ id: "br0", type: "if", condition: "" }];
  const [editingBranch, setEditingBranch] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  // 公式框候选源：scope 全量候选扁平化 + store 信号分组（与 SyntaxNode/ReturnNode 共用 buildFormulaSource；
  // 无 scope 退化纯文本——else 分支行保持不可编辑，仍走既有 button 语义）
  const formulaSource = useMemo(() => buildFormulaSource(scope, letVars ?? []), [scope, letVars]);

  const handleAddBranch = (type: "elif" | "else") => {
    // 新分支 id 取现有 id 的 max+1：追加序=数组长度会与既有 id 碰撞
    //（历史源/手写编辑使数组乱序时，length 序 id 可能重复 → 连线 handle 冲突）。
    const nextId =
      branches.reduce(
        (max, b) => Math.max(max, parseInt((b.id ?? "br0").replace(/^br/, ""), 10) || 0),
        0,
      ) + 1;
    const newBranches = [...branches, {
      id: `br${nextId}`,
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
      className="flow-node codeflow-node"
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
          className="codeflow-handle"
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
              <NodeFormulaField
                value={editValue}
                candidates={formulaSource}
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
                {br.type === "else" ? "" : (br.condition ? <TokenPreview value={br.condition} /> : "...")}
              </button>
            )}
          </div>

          <HandlesBoundary>
            <Handle
              className="codeflow-handle"
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

      {/* 条件块后置续接（flow-out）：emit 侧 condition 分支发射后经 flow-out 续接后继段——
          解析侧已重建 cond.flow-out → 后置段 wire（中链 condition round-trip，E2E 缺陷 2），
          画布必须渲染对应 handle，否则既有 flow-out wire 成不可见边（xyflow error008）。 */}
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
