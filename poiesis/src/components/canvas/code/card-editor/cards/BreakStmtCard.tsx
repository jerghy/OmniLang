// poiesis/src/components/canvas/code/card-editor/cards/BreakStmtCard.tsx
import React from "react";
import type { BreakStmtOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: BreakStmtOp;
  isFocused: boolean;
  onUpdate: (op: BreakStmtOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const BreakStmtCard: React.FC<Props> = ({ onFocus }) => (
  <div className={`${styles.card} ${styles.cardBreak}`} onClick={onFocus}>
    <span className={styles.kw}>break</span>
  </div>
);
