// poiesis/src/components/canvas/code/fields/NodeField.tsx
// 节点内联编辑共享控件族：统一"草稿 → Enter/blur 提交 → Esc 取消"语义。
import { useEffect, useRef, useState } from "react";
import styles from "./NodeField.module.css";

export interface FieldOption {
  value: string;
  label: string;
}

interface TextProps {
  value: string;
  onCommit: (v: string) => void;
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
  invalid?: boolean;
}

export function NodeTextField({ value, onCommit, placeholder, ariaLabel, className, invalid }: TextProps) {
  const [draft, setDraft] = useState(value);
  const focusedRef = useRef(false);
  const dirtyRef = useRef(false);
  // 外部 value 变化（提交成功/重渲染）时同步草稿，但聚焦输入中不打断
  useEffect(() => {
    if (!focusedRef.current) setDraft(value);
  }, [value]);

  const commit = () => {
    if (dirtyRef.current && draft !== value) onCommit(draft);
    dirtyRef.current = false;
  };
  const cancel = () => {
    focusedRef.current = false;
    dirtyRef.current = false;
    setDraft(value);
  };

  return (
    <input
      data-field-kind="text"
      className={`nodrag ${styles.field} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
      value={draft}
      placeholder={placeholder}
      aria-label={ariaLabel}
      aria-invalid={invalid || undefined}
      onFocus={() => { focusedRef.current = true; }}
      onChange={(e) => { setDraft(e.target.value); dirtyRef.current = true; }}
      onBlur={() => {
        const wasDirty = dirtyRef.current;
        commit();
        focusedRef.current = false;
        if (!wasDirty) setDraft(value);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); commit(); }
        if (e.key === "Escape") { e.preventDefault(); cancel(); }
      }}
    />
  );
}

interface SelectProps {
  value: string;
  options: FieldOption[];
  onCommit: (v: string) => void;
  ariaLabel?: string;
  allowCustom?: boolean;
}

export function NodeSelect({ value, options, onCommit, ariaLabel, allowCustom }: SelectProps) {
  const opts = [...options];
  if (allowCustom && value && !options.some((o) => o.value === value)) {
    opts.push({ value, label: value });
  }
  return (
    <select
      data-field-kind="select"
      className={`nodrag ${styles.field}`}
      value={value}
      aria-label={ariaLabel}
      onChange={(e) => onCommit(e.target.value)}
    >
      {opts.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );
}

interface ChipProps {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  ariaLabel?: string;
}

export function NodeChipToggle({ checked, onChange, label, ariaLabel }: ChipProps) {
  return (
    <button
      type="button"
      data-field-kind="chip"
      className={`nodrag ${styles.chip} ${checked ? styles.chipOn : ""}`}
      aria-label={ariaLabel ?? label}
      aria-pressed={checked}
      onClick={() => onChange(!checked)}
    >
      {label}
    </button>
  );
}

interface ArgsProps {
  values: string[];
  onChange: (values: string[]) => void;
  placeholder?: string;
}

export function NodeArgsRows({ values, onChange, placeholder }: ArgsProps) {
  // 尾随空行折叠为一个可编辑槽位（不堆叠）；中间空行原样保留
  const trimmed = [...values];
  while (trimmed.length > 0 && trimmed[trimmed.length - 1] === "") trimmed.pop();
  const hasTrailing = trimmed.length !== values.length;
  const visible = hasTrailing ? [...trimmed, ""] : trimmed;

  const update = (i: number, v: string) => {
    const next = [...values];
    next[i] = v;
    onChange(next);
  };
  const remove = (i: number) => {
    onChange(values.filter((_, idx) => idx !== i));
  };
  const add = () => {
    if (!hasTrailing) onChange([...values, ""]);
  };

  return (
    <div data-field-kind="args" className={styles.args}>
      {visible.map((v, i) => (
        <div key={i} className={styles.argsRow}>
          <NodeTextField value={v} onCommit={(nv) => update(i, nv)} placeholder={placeholder} ariaLabel={`参数 ${i + 1}`} />
          <button
            type="button"
            className={`nodrag ${styles.argsRemove}`}
            aria-label="×"
            onClick={() => remove(i)}
          >×</button>
        </div>
      ))}
      {!hasTrailing && (
        <button type="button" className={`nodrag ${styles.argsAdd}`} onClick={add}>+ 添加参数</button>
      )}
    </div>
  );
}
