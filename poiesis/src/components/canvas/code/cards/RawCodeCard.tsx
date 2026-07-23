// poiesis/src/components/canvas/code/cards/RawCodeCard.tsx
import React from "react";
import type { RawCodeOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { registerCard } from "./card-registry";

const RawCodeCard: React.FC<CardComponentProps<RawCodeOp>> = ({
  op, isSelected, onDelete,
}) => (
  <div className={`code-card raw-code ${isSelected ? "selected" : ""}`}
    style={{
      padding: "4px 8px", borderRadius: 6,
      border: isSelected ? "2px solid #ef4444" : "1px solid #fca5a5",
      background: "#fef2f2", fontFamily: "monospace", fontSize: 12,
      color: "#991b1b", whiteSpace: "pre-wrap",
    }}>
    {op.source || "(empty)"}
  </div>
);

registerCard("raw", RawCodeCard);
export default RawCodeCard;
