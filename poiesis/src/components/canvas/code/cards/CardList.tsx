// poiesis/src/components/canvas/code/cards/CardList.tsx
import React from "react";
import type { CardListProps } from "./card-types";
import { getCardComponent } from "./card-registry";

export const CardList: React.FC<CardListProps> = ({
  ops, selectedPid, onSelect, onUpdate, onDelete, onInsertAfter,
}) => {
  if (ops.length === 0) {
    return (
      <div className="card-list-empty" style={{ padding: "8px 0", color: "#94a3b8", fontSize: 12, fontStyle: "italic" }}>
        Empty block — type to add operations
      </div>
    );
  }

  return (
    <div className="card-list" style={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {ops.map((op, index) => {
        const CardComponent = getCardComponent(op.kind);
        if (!CardComponent) {
          return (
            <div key={op.pid} style={{ color: "#94a3b8", fontSize: 12 }}>
              Unknown op: {op.kind}
            </div>
          );
        }
        return (
          <CardComponent
            key={op.pid}
            op={op}
            isSelected={op.pid === selectedPid}
            onUpdate={(updated) => onUpdate(index, updated)}
            onDelete={() => onDelete(index)}
            onInsertAfter={() => onInsertAfter(index)}
          />
        );
      })}
    </div>
  );
};
