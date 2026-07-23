// poiesis/src/components/canvas/code/cards/CallFnCard.tsx
import React from "react";
import type { CallFnOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const CallFnCard: React.FC<CardComponentProps<CallFnOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => (
  <div className={`code-card call-fn ${isSelected ? "selected" : ""}`}
    style={{
      display: "flex", alignItems: "center", gap: 4,
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #6366f1" : "1px solid #cbd5e1",
      background: "#fff", fontFamily: "monospace", fontSize: 13,
    }}>
    <InlineSlot value={op.fnName} placeholder="fn"
      onChange={(v) => onUpdate({ ...op, fnName: v })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
    <span>(</span>
    <InlineSlot value={op.args.join(", ")} placeholder="args"
      onChange={(v) => onUpdate({ ...op, args: v ? v.split(",").map(a => a.trim()) : [] })}
      onCommit={onInsertAfter} onDeleteEmpty={onDelete}
      onNavigateLeft={() => {}} onNavigateRight={() => {}} />
    <span>)</span>
  </div>
);

registerCard("call_fn", CallFnCard);
export default CallFnCard;
