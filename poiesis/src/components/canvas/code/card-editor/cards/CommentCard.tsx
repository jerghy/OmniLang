// poiesis/src/components/canvas/code/card-editor/cards/CommentCard.tsx
import React from "react";
import type { CommentOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: CommentOp;
  isFocused: boolean;
  onUpdate: (op: CommentOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const CommentCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => (
  <div className={`${styles.card} ${styles.cardComment}`} onClick={onFocus}>
    <span className={styles.kw}>//</span>
    <input className={styles.slot} value={op.text}
      onChange={e => onUpdate({ ...op, text: e.target.value })}
      placeholder="comment" />
  </div>
);
