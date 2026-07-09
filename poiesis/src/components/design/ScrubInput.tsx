// components/design/ScrubInput.tsx
// Drag-scrub numeric input with Poiesis design panel styling.

import React, { useState, useRef, useCallback, useEffect } from "react";

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

export const ScrubInput = React.memo(function ScrubInput({
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
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const committedRef = useRef(false);
  const clampedValueRef = useRef(value);
  const rafRef = useRef<number | null>(null);
  const latestValueRef = useRef(value);
  const scrubRef = useRef<{
    startX: number;
    startValue: number;
    moved: boolean;
  } | null>(null);

  const clamped = clamp(value, min, max);
  clampedValueRef.current = clamped;

  useEffect(() => {
    if (!editing) {
      setEditText(String(clamped));
    }
  }, [clamped, editing]);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

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
          // Batch updates via rAF — only commit once per frame
          latestValueRef.current = nv;
          if (rafRef.current === null) {
            rafRef.current = requestAnimationFrame(() => {
              rafRef.current = null;
              currentOnChange(latestValueRef.current);
            });
          }
        }
      };

      const onUp = () => {
        // Cancel pending rAF
        if (rafRef.current !== null) {
          cancelAnimationFrame(rafRef.current);
          rafRef.current = null;
        }

        const s = scrubRef.current;
        scrubRef.current = null;
        setScrubbing(false);

        if (s && !s.moved) {
          setEditing(true);
          setEditText(String(clamped));
        } else if (s && s.moved) {
          // Commit final value
          currentOnChange(latestValueRef.current);
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

  // Compute border color based on state
  const borderColor = focus
    ? "var(--dp-accent, #6366f1)"
    : hover
      ? "var(--dp-border-active, #6366f1)"
      : "var(--dp-border, #e8e5f2)";

  const rootStyle: React.CSSProperties = {
    display: "inline-flex",
    alignItems: "center",
    fontFamily: "var(--dp-font-mono, monospace)",
    fontSize: 12,
    border: `1px solid ${borderColor}`,
    borderRadius: "var(--dp-radius-sm, 6px)",
    padding: "0 5px",
    cursor: scrubbing ? "ew-resize" : disabled ? "default" : "ew-resize",
    userSelect: "none",
    minWidth: 48,
    width: 64,
    height: 24,
    boxSizing: "border-box",
    background: disabled ? "#f5f5f5" : "var(--dp-surface, #fff)",
    color: disabled ? "#999" : "var(--dp-text, #1e1b4b)",
    transition: "border-color 150ms ease, box-shadow 150ms ease",
    boxShadow: focus ? "0 0 0 2px rgba(99,102,241,0.15)" : "none",
    outline: "none",
    ...style,
  };

  const valueStyle: React.CSSProperties = {
    flex: 1,
    textAlign: "right",
  };

  const inputStyle: React.CSSProperties = {
    flex: 1,
    minWidth: 0,
    border: "none",
    outline: "none",
    padding: 0,
    fontFamily: "inherit",
    fontSize: "inherit",
    textAlign: "right",
    background: "transparent",
    color: "var(--dp-accent, #6366f1)",
  };

  const unitStyle: React.CSSProperties = {
    marginLeft: 2,
    fontSize: 9,
    color: "var(--dp-text-muted, #a09db8)",
    lineHeight: 1,
    fontWeight: 500,
  };

  return (
    <div
      style={rootStyle}
      tabIndex={disabled ? -1 : 0}
      onMouseDown={handleMouseDown}
      onKeyDown={handleContainerKeyDown}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onFocus={() => setFocus(true)}
      onBlur={() => setFocus(false)}
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
});
