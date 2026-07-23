// poiesis/src/components/canvas/code/cards/SimpleStmtCard.tsx
import React from "react";
import type { BreakStmtOp, ContinueStmtOp, CommentOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

type SimpleOp = BreakStmtOp | ContinueStmtOp | CommentOp;

const SimpleStmtCard: React.FC<CardComponentProps<SimpleOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const label = op.kind === "break" ? "break" : op.kind === "continue" ? "continue" : "//";

  return (
    <div className={`code-card simple-stmt ${isSelected ? "selected" : ""}`}
      style={{
        display: "flex", alignItems: "center", gap: 4,
        padding: "4px 8px", borderRadius: 6,
        border: isSelected ? "2px solid #94a3b8" : "1px solid #e2e8f0",
        background: "#fff", fontFamily: "monospace", fontSize: 13,
      }}>
      <span style={{ color: "#64748b", fontWeight: 600 }}>{label}</span>
      {op.kind === "comment" && (
        <InlineSlot value={(op as CommentOp).text} placeholder="comment"
          onChange={(v) => onUpdate({ ...op, text: v } as any)}
          onCommit={onInsertAfter} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
      )}
    </div>
  );
};

registerCard("break", SimpleStmtCard);
registerCard("continue", SimpleStmtCard);
registerCard("comment", SimpleStmtCard);
export default SimpleStmtCard;
