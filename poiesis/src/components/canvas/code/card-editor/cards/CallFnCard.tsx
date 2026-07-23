// poiesis/src/components/canvas/code/card-editor/cards/CallFnCard.tsx
import React from "react";
import type { CallFnOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: CallFnOp;
  isFocused: boolean;
  onUpdate: (op: CallFnOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const CallFnCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => (
  <div className={`${styles.card} ${styles.cardCall}`} onClick={onFocus}>
    <input className={styles.slot} value={op.fnName}
      onChange={e => onUpdate({ ...op, fnName: e.target.value })} placeholder="fn" />
    <span>(</span>
    <input className={styles.slot} value={op.args.join(", ")}
      onChange={e => onUpdate({ ...op, args: e.target.value.split(",").map(a => a.trim()).filter(Boolean) })}
      placeholder="args" />
    <span>)</span>
  </div>
);
