// poiesis/src/components/canvas/code/card-editor/cards/RawCodeCard.tsx
import React from "react";
import type { RawCodeOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: RawCodeOp;
  isFocused: boolean;
  onUpdate: (op: RawCodeOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const RawCodeCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => (
  <div className={`${styles.card} ${styles.cardRaw}`} onClick={onFocus}>
    <span className={styles.kw}>raw</span>
    <input className={styles.slot} value={op.source}
      onChange={e => onUpdate({ ...op, source: e.target.value })}
      placeholder="free code" style={{ width: "100%" }} />
  </div>
);
