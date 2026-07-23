// poiesis/src/components/canvas/code/cards/IfBlockCard.tsx
import React from "react";
import type { IfBlockOp, RawCodeOp } from "../../../../core/types/code-op";
import type { CardComponentProps } from "./card-types";
import { InlineSlot } from "./InlineSlot";
import { CardList } from "./CardList";
import { registerCard } from "./card-registry";
import { uid } from "../../../../core/utils/id";

const IfBlockCard: React.FC<CardComponentProps<IfBlockOp>> = ({
  op, isSelected, onUpdate, onDelete, onInsertAfter,
}) => {
  const [selectedPid, setSelectedPid] = React.useState<string | null>(null);

  const updateBranch = (bi: number, field: "condition" | "body", value: any) => {
    const newBranches = op.branches.map((br, i) =>
      i === bi ? { ...br, [field]: value } : br
    );
    onUpdate({ ...op, branches: newBranches });
  };

  return (
    <div className={`code-card if-block ${isSelected ? "selected" : ""}`}
      style={{
        border: isSelected ? "2px solid #8b5cf6" : "1px solid #c4b5fd",
        borderRadius: 6, background: "#faf5ff",
        fontFamily: "monospace", fontSize: 13,
      }}>
      {op.branches.map((br, bi) => {
        const kw = bi === 0 ? "if" : br.condition ? "else if" : "else";
        return (
          <div key={bi} style={{ padding: "4px 8px" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ color: "#7c3aed", fontWeight: 600 }}>{kw}</span>
              {br.condition && (
                <InlineSlot value={br.condition} placeholder="condition"
                  onChange={(v) => updateBranch(bi, "condition", v)}
                  onCommit={() => {}}
                  onDeleteEmpty={() => updateBranch(bi, "condition", "")}
                  onNavigateLeft={() => {}} onNavigateRight={() => {}} />
              )}
              <span>{`{`}</span>
            </div>
            <div style={{ paddingLeft: 16 }}>
              <CardList
                ops={br.body}
                selectedPid={selectedPid}
                onSelect={setSelectedPid}
                onUpdate={(idx, updated) => {
                  const newBody = [...br.body];
                  newBody[idx] = updated;
                  updateBranch(bi, "body", newBody);
                }}
                onDelete={(idx) => {
                  const newBody = br.body.filter((_, i) => i !== idx);
                  updateBranch(bi, "body", newBody);
                }}
                onInsertAfter={(idx) => {
                  const newBody = [...br.body];
                  const rawOp: RawCodeOp = { kind: "raw", pid: uid("op"), source: "" };
                  newBody.splice(idx + 1, 0, rawOp);
                  updateBranch(bi, "body", newBody);
                }}
              />
            </div>
            <div>{`}`}</div>
          </div>
        );
      })}
    </div>
  );
};

registerCard("if_block", IfBlockCard);
export default IfBlockCard;
