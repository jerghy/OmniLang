// poiesis/src/components/canvas/code/card-editor/cards/DeclareVarCard.tsx
import React, { useRef, useEffect } from "react";
import type { DeclareVarOp } from "../../../../../core/types/code-op";
import styles from "../CodeCardEditor.module.css";

interface Props {
  op: DeclareVarOp;
  isFocused: boolean;
  onUpdate: (op: DeclareVarOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const DeclareVarCard: React.FC<Props> = ({ op, isFocused, onUpdate, onFocus, onInsertBelow, onDelete }) => {
  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isFocused && !op.name) nameRef.current?.focus();
  }, [isFocused, op.name]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Tab") {
      e.preventDefault();
      // focus next input in card
      const t = e.currentTarget as HTMLElement;
      const inputs = t.parentElement?.querySelectorAll("input");
      if (inputs) {
        const idx = Array.from(inputs).indexOf(e.currentTarget as HTMLInputElement);
        const next = inputs[idx + 1] as HTMLInputElement | undefined;
        next?.focus();
      }
    }
    if (e.key === "Backspace" && !op.name && !op.varType && !op.initValue) {
      onDelete();
    }
  };

  return (
    <div className={`${styles.card} ${styles.cardDeclare}`} onClick={onFocus}>
      <span className={styles.kw}>let</span>
      {op.mutable && <span className={styles.kw}>mut</span>}
      <input ref={nameRef} className={styles.slot} value={op.name}
        onChange={e => onUpdate({ ...op, name: e.target.value })}
        onKeyDown={handleKeyDown} onFocus={onFocus} placeholder="name" />
      {op.varType !== undefined ? (
        <><span>:</span><input className={styles.slot} value={op.varType}
          onChange={e => onUpdate({ ...op, varType: e.target.value })}
          onKeyDown={handleKeyDown} placeholder="Type" /></>
      ) : (
        <button className={styles.addSlot} onClick={() => onUpdate({ ...op, varType: "" })}>+:type</button>
      )}
      {op.initValue !== undefined ? (
        <><span>=</span><input className={styles.slot} value={op.initValue}
          onChange={e => onUpdate({ ...op, initValue: e.target.value })}
          onKeyDown={handleKeyDown} placeholder="value" /></>
      ) : (
        <button className={styles.addSlot} onClick={() => onUpdate({ ...op, initValue: "" })}>+=value</button>
      )}
    </div>
  );
};
