// poiesis/src/components/canvas/code/card-editor/InputRow.tsx
import React, { useRef, useEffect } from "react";
import styles from "./CodeCardEditor.module.css";

interface Props {
  text: string;
  indent: number;
  isFocused: boolean;
  onChange: (text: string) => void;
  onFocus: () => void;
  onTriggerAutocomplete: (token: string) => void;
  onEnter: () => void;
  onBackspaceEmpty: () => void;
}

export const InputRow: React.FC<Props> = ({
  text,
  indent,
  isFocused,
  onChange,
  onFocus,
  onTriggerAutocomplete,
  onEnter,
  onBackspaceEmpty,
}) => {
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isFocused && ref.current) ref.current.focus();
  }, [isFocused]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    onChange(value);
    onTriggerAutocomplete(value);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onEnter();
    } else if (e.key === "Backspace" && text === "") {
      onBackspaceEmpty();
    }
  };

  return (
    <div className={styles.inputRow} style={{ paddingLeft: indent * 20 + 8 }}>
      <input
        ref={ref}
        className={styles.inputField}
        value={text}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onFocus={onFocus}
        placeholder="Type to add..."
        spellCheck={false}
        autoComplete="off"
      />
    </div>
  );
};
