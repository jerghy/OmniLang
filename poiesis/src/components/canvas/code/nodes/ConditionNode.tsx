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
        background: "#FFFBEB",
        border: "2px solid #F59E0B",
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
        style={{ background: "#F59E0B" }}
      />

      <div style={{ padding: "0 10px", fontWeight: 600, marginBottom: 4, color: "#92400E" }}>
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
            borderTop: i > 0 ? "1px solid #FDE68A" : "none",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
            <span style={{
              fontSize: 12,
              fontWeight: 600,
              color: br.type === "if" ? "#059669" : br.type === "elif" ? "#D97706" : "#DC2626",
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
                  border: "1px solid #F59E0B",
                  borderRadius: 3,
                  padding: "1px 4px",
                }}
                placeholder="条件表达式"
              />
            ) : (
              <span
                onClick={() => br.type !== "else" && handleEditStart(br)}
                style={{
                  fontFamily: "monospace",
                  fontSize: 12,
                  color: br.type === "else" ? "#9CA3AF" : "#1F2937",
                  cursor: br.type === "else" ? "default" : "pointer",
                  flex: 1,
                }}
              >
                {br.type === "else" ? "" : br.condition || "..."}
              </span>
            )}
          </div>

          <Handle
            type="source"
            position={Position.Right}
            id={br.id}
            style={{ background: "#F59E0B", position: "relative", right: -6, transform: "none" }}
          />
        </div>
      ))}

      {/* Add branch buttons */}
      <div style={{ display: "flex", gap: 4, padding: "4px 10px 0", borderTop: "1px solid #FDE68A" }}>
        {!hasElse && (
          <>
            <button type="button"
              onClick={() => handleAddBranch("elif")}
              style={{
                fontSize: 12, padding: "2px 6px", border: "1px solid #D97706",
                borderRadius: 3, background: "#FFF", color: "#D97706", cursor: "pointer",
              }}
            >
              + elif
            </button>
            <button type="button"
              onClick={() => handleAddBranch("else")}
              style={{
                fontSize: 12, padding: "2px 6px", border: "1px solid #DC2626",
                borderRadius: 3, background: "#FFF", color: "#DC2626", cursor: "pointer",
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
