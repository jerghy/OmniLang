// poiesis/src/components/canvas/code/card-editor/cards/WhileLoopCard.tsx
import React from "react";
import type { WhileLoopOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: WhileLoopOp;
  isFocused: boolean;
  onUpdate: (op: WhileLoopOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const WhileLoopCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => {
  return (
    <div className={`${styles.card} ${styles.cardWhile}`} onClick={onFocus}>
      <span className={styles.kw}>while</span>
      <input className={styles.slot} value={op.condition}
        onChange={e => onUpdate({ ...op, condition: e.target.value })}
        placeholder="condition" />
      <span className={styles.kw}>{"{"}</span>
    </div>
  );
};
