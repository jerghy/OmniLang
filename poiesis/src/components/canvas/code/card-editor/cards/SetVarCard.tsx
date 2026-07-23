// poiesis/src/components/canvas/code/card-editor/cards/SetVarCard.tsx
import React from "react";
import type { SetVarOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: SetVarOp;
  isFocused: boolean;
  onUpdate: (op: SetVarOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const SetVarCard: React.FC<Props> = ({ op, onUpdate, onFocus }) => {
  return (
    <div className={`${styles.card} ${styles.cardSet}`} onClick={onFocus}>
      <span className={styles.kw}>set</span>
      <input className={styles.slot} value={op.name}
        onChange={e => onUpdate({ ...op, name: e.target.value })} placeholder="var" />
      <select className={styles.slot} value={op.operator}
        onChange={e => onUpdate({ ...op, operator: e.target.value as any })}>
        {["=", "+=", "-=", "*=", "/=", "%="].map(op => <option key={op} value={op}>{op}</option>)}
      </select>
      <input className={styles.slot} value={op.value}
        onChange={e => onUpdate({ ...op, value: e.target.value })} placeholder="expr" />
    </div>
  );
};
