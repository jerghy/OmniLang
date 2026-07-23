// poiesis/src/components/canvas/code/card-editor/cards/IfBlockCard.tsx
import React from "react";
import type { IfBlockOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: IfBlockOp;
  isFocused: boolean;
  onUpdate: (op: IfBlockOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const IfBlockCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => {
  const br = op.branches[0] ?? { condition: "", body: [] };
  return (
    <div className={`${styles.card} ${styles.cardIf}`} onClick={onFocus}>
      <span className={styles.kw}>if</span>
      <input className={styles.slot} value={br.condition}
        onChange={e => onUpdate({ ...op, branches: [{ ...br, condition: e.target.value }] })}
        placeholder="condition" />
      <span className={styles.kw}>{"{"}</span>
    </div>
  );
};
