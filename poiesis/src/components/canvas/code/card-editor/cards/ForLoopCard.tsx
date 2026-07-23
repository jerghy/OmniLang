// poiesis/src/components/canvas/code/card-editor/cards/ForLoopCard.tsx
import React from "react";
import type { ForLoopOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: ForLoopOp;
  isFocused: boolean;
  onUpdate: (op: ForLoopOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const ForLoopCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => {
  return (
    <div className={`${styles.card} ${styles.cardFor}`} onClick={onFocus}>
      <span className={styles.kw}>for</span>
      <input className={styles.slot} value={op.itemName}
        onChange={e => onUpdate({ ...op, itemName: e.target.value })} placeholder="item" />
      <span className={styles.kw}>in</span>
      <input className={styles.slot} value={op.collection}
        onChange={e => onUpdate({ ...op, collection: e.target.value })} placeholder="collection" />
      <span className={styles.kw}>{"{"}</span>
    </div>
  );
};
