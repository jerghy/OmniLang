// poiesis/src/components/canvas/code/cards/WhileLoopCard.tsx
import React from "react";
import type { WhileLoopOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { CardList } from "./CardList";
import { registerCard } from "./card-registry";

const WhileLoopCard: React.FC<CardComponentProps<WhileLoopOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const [selectedPid, setSelectedPid] = React.useState<string | null>(null);

  return (
    <div className={`code-card while-loop ${isSelected ? "selected" : ""}`}
      style={{
        border: isSelected ? "2px solid #06b6d4" : "1px solid #67e8f9",
        borderRadius: 6, background: "#ecfeff",
        fontFamily: "monospace", fontSize: 13,
      }}>
      <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px" }}>
        <span style={{ color: "#0891b2", fontWeight: 600 }}>while</span>
        <InlineSlot value={op.condition} placeholder="condition"
          onChange={(v) => onUpdate({ ...op, condition: v })}
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

registerCard("while", WhileLoopCard);
export default WhileLoopCard;
