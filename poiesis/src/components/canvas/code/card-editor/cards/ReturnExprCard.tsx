// poiesis/src/components/canvas/code/card-editor/cards/ReturnExprCard.tsx
import React from "react";
import type { ReturnExprOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: ReturnExprOp;
  isFocused: boolean;
  onUpdate: (op: ReturnExprOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const ReturnExprCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => (
  <div className={`${styles.card} ${styles.cardReturn}`} onClick={onFocus}>
    <span className={styles.kw}>return</span>
    <input className={styles.slot} value={op.value ?? ""}
      onChange={e => onUpdate({ ...op, value: e.target.value })}
      placeholder="expr" />
  </div>
);
