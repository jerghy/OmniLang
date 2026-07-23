// poiesis/src/components/canvas/code/cards/ReturnExprCard.tsx
import React from "react";
import type { ReturnExprOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const ReturnExprCard: React.FC<CardComponentProps<ReturnExprOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card return-expr ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #f59e0b" : "1px solid #fcd34d",
      background: "#fffbeb", fontFamily: "monospace", fontSize: 13,
    }}>
    <span className="card-keyword" style={{ color: "#d97706", fontWeight: 600 }}>return</span>
    <InlineSlot value={op.value ?? ""} placeholder="expr"
      onChange={(v) => onUpdate({ ...op, value: v || undefined })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
  </div>
);

registerCard("return", ReturnExprCard);
export default ReturnExprCard;
