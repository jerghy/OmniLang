// poiesis/src/components/canvas/code/cards/DeclareVarCard.tsx
import React from "react";
import type { DeclareVarOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { registerCard } from "./card-registry";

const DeclareVarCard: React.FC<CardComponentProps<DeclareVarOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  return (
    <div className={`code-card declare-var ${isSelected ? "selected" : ""}`}
      style={{
        display: "flex", alignItems: "center", gap: 4,
        padding: "4px 8px", borderRadius: 6,
        border: isSelected ? "2px solid #6366f1" : "1px solid #cbd5e1",
        background: isSelected ? "#f8fafc" : "#fff",
        fontFamily: "monospace", fontSize: 13,
      }}>
      <span className="card-keyword" style={{ color: "#7c3aed", fontWeight: 600 }}>let</span>
      {op.mutable && <span className="card-keyword" style={{ color: "#7c3aed", fontWeight: 600 }}>mut</span>}
      <InlineSlot
        value={op.name} placeholder="name"
        onChange={(v) => onUpdate({ ...op, name: v })}
        onCommit={onInsertAfter}
        onDeleteEmpty={onDelete}
        onNavigateLeft={() => {}}
        onNavigateRight={() => {}}
        autoFocus={op.name === ""}
      />
      {op.varType !== undefined && (
        <>
          <span>:</span>
          <InlineSlot value={op.varType} placeholder="Type"
            onChange={(v) => onUpdate({ ...op, varType: v })}
            onCommit={onInsertAfter} onDeleteEmpty={() => onUpdate({ ...op, varType: undefined })}
            onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        </>
      )}
      {op.initValue !== undefined && (
        <>
          <span>=</span>
          <InlineSlot value={op.initValue} placeholder="value"
            onChange={(v) => onUpdate({ ...op, initValue: v })}
            onCommit={onInsertAfter} onDeleteEmpty={() => onUpdate({ ...op, initValue: undefined })}
            onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        </>
      )}
      <span className="card-pid" style={{ fontSize: 9, color: "#94a3b8", marginLeft: "auto" }}>
        {op.scope}
      </span>
    </div>
  );
};

registerCard("declare_var", DeclareVarCard);
export default DeclareVarCard;
