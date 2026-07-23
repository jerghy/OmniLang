// poiesis/src/components/canvas/code/cards/AwaitExprCard.tsx
import React from "react";
import type { AwaitExprOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const AwaitExprCard: React.FC<CardComponentProps<AwaitExprOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card await-expr ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #f59e0b" : "1px solid #fcd34d",
      background: "#fffbeb", fontFamily: "monospace", fontSize: 13,
    }}>
    <span style={{ color: "#d97706", fontWeight: 600 }}>await</span>
    <InlineSlot value={op.value} placeholder="expr"
      onChange={(v) => onUpdate({ ...op, value: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
  </div>
);

registerCard("await", AwaitExprCard);
export default AwaitExprCard;
