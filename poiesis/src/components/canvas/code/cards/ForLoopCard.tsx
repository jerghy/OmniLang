// poiesis/src/components/canvas/code/cards/ForLoopCard.tsx
import React from "react";
import type { ForLoopOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { CardList } from "./CardList";
import { registerCard } from "./card-registry";

const ForLoopCard: React.FC<CardComponentProps<ForLoopOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const [selectedPid, setSelectedPid] = React.useState<string | null>(null);

  return (
    <div className={`code-card for-loop ${isSelected ? "selected" : ""}`}
      style={{
        border: isSelected ? "2px solid #10b981" : "1px solid #6ee7b7",
        borderRadius: 6, background: "#ecfdf5",
        fontFamily: "monospace", fontSize: 13,
      }}>
      <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px" }}>
        <span style={{ color: "#059669", fontWeight: 600 }}>for</span>
        <InlineSlot value={op.itemName} placeholder="item"
          onChange={(v) => onUpdate({ ...op, itemName: v })}
          onCommit={() => {}} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        <span style={{ color: "#64748b" }}>in</span>
        <InlineSlot value={op.collection} placeholder="collection"
          onChange={(v) => onUpdate({ ...op, collection: v })}
          onCommit={() => {}} onDeleteEmpty={onDelete}
          onNavigateLeft={() => {}} onNavigateRight={() => {}} />
        <span>{`{`}</span>
      </div>
      <div style={{ paddingLeft: 16 }}>
        <CardList ops={op.body} selectedPid={selectedPid} onSelect={setSelectedPid}
          onUpdate={(idx, updated) => {
            const newBody = [...op.body];
            newBody[idx] = updated;
            onUpdate({ ...op, body: newBody });
          }}
          onDelete={(idx) => onUpdate({ ...op, body: op.body.filter((_, i) => i !== idx) })}
          onInsertAfter={(idx) => {
            const newBody = [...op.body];
            newBody.splice(idx + 1, 0, { kind: "raw", pid: `op-${Date.now()}`, source: "" } as any);
            onUpdate({ ...op, body: newBody });
          }}
        />
      </div>
      <div style={{ padding: "0 8px 4px" }}>{`}`}</div>
    </div>
  );
};

registerCard("for_in", ForLoopCard);
export default ForLoopCard;
