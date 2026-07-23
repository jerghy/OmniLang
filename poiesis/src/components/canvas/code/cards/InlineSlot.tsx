import React, { useState, useRef, useEffect } from "react";

interface Props {
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onCommit: () => void;       // Enter pressed
  onDeleteEmpty: () => void;  // Backspace on empty slot
  onNavigateLeft: () => void;
  onNavigateRight: () => void;
  autoFocus?: boolean;
}

export const InlineSlot: React.FC<Props> = ({
  value, placeholder, onChange, onCommit,
  onDeleteEmpty, onNavigateLeft, onNavigateRight, autoFocus,
}) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [isEmpty, setIsEmpty] = useState(value === "");

  useEffect(() => {
    if (autoFocus && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [autoFocus]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange(e.target.value);
    setIsEmpty(e.target.value === "");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onCommit();
    } else if (e.key === "Backspace" && isEmpty) {
      e.preventDefault();
      onDeleteEmpty();
    } else if (e.key === "ArrowLeft" && e.currentTarget.selectionStart === 0) {
      e.preventDefault();
      onNavigateLeft();
    } else if (e.key === "ArrowRight" && e.currentTarget.selectionEnd === (e.currentTarget as HTMLInputElement).value.length) {
      e.preventDefault();
      onNavigateRight();
    }
  };

  if (value && isEmpty === false) {
    // Tokenized display
    return (
      <span
        className="inline-slot-token"
        tabIndex={0}
        onFocus={() => setIsEmpty(false)}
        style={{
          display: "inline-block",
          background: "#e0e7ff",
          color: "#3730a3",
          borderRadius: 3,
          padding: "0 4px",
          fontFamily: "monospace",
          fontSize: 13,
          cursor: "text",
        }}
      >
        {value}
      </span>
    );
  }

  return (
    <input
      ref={inputRef}
      className="inline-slot-input"
      value={value}
      placeholder={placeholder}
      onChange={handleChange}
      onKeyDown={handleKeyDown}
      style={{
        display: "inline-block",
        width: Math.max(60, (value.length + 1) * 9),
        minWidth: 40,
        border: "1px solid #a5b4fc",
        borderRadius: 3,
        padding: "0 4px",
        fontFamily: "monospace",
        fontSize: 13,
        outline: "none",
        background: "#eef2ff",
      }}
    />
  );
};
