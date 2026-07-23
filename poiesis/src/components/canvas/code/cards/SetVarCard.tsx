// poiesis/src/components/canvas/code/cards/SetVarCard.tsx
import React from "react";
import type { SetVarOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const SetVarCard: React.FC<CardComponentProps<SetVarOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card set-var ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #6366f1" : "1px solid #cbd5e1",
      background: "#fff", fontFamily: "monospace", fontSize: 13,
    }}>
    <InlineSlot value={op.name} placeholder="variable"
      onChange={(v) => onUpdate({ ...op, name: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
    <span className="card-operator" style={{ color: "#2563eb", fontWeight: 600 }}>{op.operator}</span>
    <InlineSlot value={op.value} placeholder="expr"
      onChange={(v) => onUpdate({ ...op, value: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
  </div>
);

registerCard("set_var", SetVarCard);
export default SetVarCard;
