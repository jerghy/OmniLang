import { Handle, Position } from "@xyflow/react";
import { useState } from "react";

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

  const handleEditConfirm = (br: BranchData) => {
    data.onEditCondition?.(br.id, editValue);
    setEditingBranch(null);
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
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "var(--ol-color-warning)" }}
      />

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
              <input
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                onBlur={() => handleEditConfirm(br)}
                onKeyDown={(e) => { if (e.key === "Enter") handleEditConfirm(br); }}
                style={{
                  width: "100%",
                  fontSize: 12,
                  fontFamily: "monospace",
                  border: "1px solid var(--ol-color-warning)",
                  borderRadius: 3,
                  padding: "1px 4px",
                }}
                placeholder="条件表达式"
                aria-label="编辑条件表达式"
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

          <Handle
            type="source"
            position={Position.Right}
            id={br.id}
            style={{ background: "var(--ol-color-warning)", position: "relative", right: -6, transform: "none" }}
          />
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
