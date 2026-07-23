// poiesis/src/components/canvas/code/card-editor/cards/ContinueStmtCard.tsx
import React from "react";
import type { ContinueStmtOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: ContinueStmtOp;
  isFocused: boolean;
  onUpdate: (op: ContinueStmtOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const ContinueStmtCard: React.FC<Props> = ({ onFocus }) => (
  <div className={`${styles.card} ${styles.cardContinue}`} onClick={onFocus}>
    <span className={styles.kw}>continue</span>
  </div>
);
