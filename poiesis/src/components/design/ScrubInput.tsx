// components/design/ScrubInput.tsx
// Drag-scrub numeric input: click+drag to increment/decrement, click to enter edit mode.

import { useState, useRef, useCallback, useEffect } from "react";

interface ScrubInputProps {
  value: number;
  unit?: string;
  min?: number;
  max?: number;
  step?: number;
  fineStep?: number;
  onChange: (value: number) => void;
  onChangeEnd?: (value: number) => void;
  disabled?: boolean;
  style?: React.CSSProperties;
}

function clamp(n: number, lo?: number, hi?: number): number {
  let r = n;
  if (lo !== undefined) r = Math.max(lo, r);
  if (hi !== undefined) r = Math.min(hi, r);
  return r;
}

export function ScrubInput({
  value,
  unit,
  min,
  max,
  step = 1,
  fineStep = 10,
  onChange,
  onChangeEnd,
  disabled = false,
  style,
}: ScrubInputProps) {
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(String(value));
  const [scrubbing, setScrubbing] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const committedRef = useRef(false);
  const clampedValueRef = useRef(value);
  const scrubRef = useRef<{
    startX: number;
    startValue: number;
    moved: boolean;
  } | null>(null);

  const clamped = clamp(value, min, max);
  clampedValueRef.current = clamped;

  // Keep editText in sync when value changes externally and not editing
  useEffect(() => {
    if (!editing) {
      setEditText(String(clamped));
    }
  }, [clamped, editing]);

  // Auto-focus input when entering edit mode
  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  // Reset commit guard when entering edit mode
  useEffect(() => {
    if (editing) {
      committedRef.current = false;
    }
  }, [editing]);

  const commitEdit = useCallback(() => {
    if (committedRef.current) return;
    committedRef.current = true;
    const parsed = parseFloat(editText);
    if (!isNaN(parsed)) {
      const c = clamp(parsed, min, max);
      onChange(c);
      onChangeEnd?.(c);
    }
    setEditing(false);
  }, [editText, min, max, onChange, onChangeEnd]);

  const cancelEdit = useCallback(() => {
    setEditing(false);
  }, []);

  // Keyboard handler for the container (when NOT in edit mode)
  const handleContainerKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (disabled || editing) return;
      if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        const delta = e.shiftKey ? fineStep : step;
        const dir = e.key === "ArrowUp" ? 1 : -1;
        const nv = clamp(clamped + dir * delta, min, max);
        onChange(nv);
        onChangeEnd?.(nv);
      }
    },
    [disabled, editing, clamped, step, fineStep, min, max, onChange, onChangeEnd]
  );

  // Keyboard handler for the input element (when in edit mode)
  const handleInputKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        commitEdit();
      } else if (e.key === "Escape") {
        cancelEdit();
      } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        const delta = e.shiftKey ? fineStep : step;
        const dir = e.key === "ArrowUp" ? 1 : -1;
        const nv = clamp(clamped + dir * delta, min, max);
        onChange(nv);
        setEditText(String(nv));
        onChangeEnd?.(nv);
      }
    },
    [clamped, step, fineStep, min, max, onChange, onChangeEnd, commitEdit, cancelEdit]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (disabled) return;
      e.preventDefault();

      scrubRef.current = {
        startX: e.clientX,
        startValue: clamped,
        moved: false,
      };

      const currentStep = step;
      const currentFineStep = fineStep;
      const currentMin = min;
      const currentMax = max;
      const currentOnChange = onChange;
      const currentOnChangeEnd = onChangeEnd;

      const onMove = (me: MouseEvent) => {
        const s = scrubRef.current;
        if (!s) return;
        const dx = me.clientX - s.startX;
        if (Math.abs(dx) > 2) {
          s.moved = true;
          setScrubbing(true);
        }
        if (s.moved) {
          const eff = me.shiftKey ? currentFineStep : currentStep;
          const nv = clamp(
            s.startValue + Math.round(dx / 4) * eff,
            currentMin,
            currentMax
          );
          currentOnChange(nv);
        }
      };

      const onUp = () => {
        const s = scrubRef.current;
        scrubRef.current = null;
        setScrubbing(false);

        if (s && !s.moved) {
          // Click without drag -> enter edit mode
          setEditing(true);
          setEditText(String(clamped));
        } else if (s && s.moved) {
          // Drag complete -> notify parent
          currentOnChangeEnd?.(clampedValueRef.current);
        }

        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [disabled, clamped, step, fineStep, min, max, onChange, onChangeEnd]
  );

  // ---- Styles ----

  const rootStyle: React.CSSProperties = {
    display: "inline-flex",
    alignItems: "center",
    fontFamily: "'Consolas', 'Monaco', 'Courier New', monospace",
    fontSize: 11,
    border: "1px solid #d0d0d0",
    borderRadius: 3,
    padding: "0 4px",
    cursor: scrubbing ? "ew-resize" : disabled ? "default" : "ew-resize",
    userSelect: "none",
    minWidth: 48,
    height: 22,
    boxSizing: "border-box",
    background: disabled ? "#f5f5f5" : "#fff",
    color: disabled ? "#999" : "inherit",
    ...style,
  };

  const valueStyle: React.CSSProperties = {
    flex: 1,
    textAlign: "right",
  };

  const inputStyle: React.CSSProperties = {
    width: "100%",
    border: "none",
    outline: "none",
    padding: 0,
    fontFamily: "inherit",
    fontSize: "inherit",
    textAlign: "right",
    background: "transparent",
    color: "inherit",
  };

  const unitStyle: React.CSSProperties = {
    marginLeft: 2,
    fontSize: 10,
    color: "#888",
    lineHeight: 1,
  };

  return (
    <div
      style={rootStyle}
      tabIndex={disabled ? -1 : 0}
      onMouseDown={handleMouseDown}
      onKeyDown={handleContainerKeyDown}
    >
      {editing ? (
        <input
          ref={inputRef}
          style={inputStyle}
          type="text"
          inputMode="decimal"
          autoComplete="off"
          value={editText}
          onChange={(e) => setEditText(e.target.value)}
          onKeyDown={handleInputKeyDown}
          onBlur={commitEdit}
        />
      ) : (
        <span style={valueStyle}>{clamped}</span>
      )}
      {unit && <span style={unitStyle}>{unit}</span>}
    </div>
  );
}
