// poiesis/src/components/canvas/code/card-editor/cards/AwaitExprCard.tsx
import React from "react";
import type { AwaitExprOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: AwaitExprOp;
  isFocused: boolean;
  onUpdate: (op: AwaitExprOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const AwaitExprCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => (
  <div className={`${styles.card} ${styles.cardAwait}`} onClick={onFocus}>
    <span className={styles.kw}>await</span>
    <input className={styles.slot} value={op.value}
      onChange={e => onUpdate({ ...op, value: e.target.value })}
      placeholder="expr" />
  </div>
);
