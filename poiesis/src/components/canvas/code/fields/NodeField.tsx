// poiesis/src/components/canvas/code/fields/NodeField.tsx
// 节点内联编辑共享控件族：统一"草稿 → Enter/blur 提交 → Esc 取消"语义。
import { useEffect, useRef, useState } from "react";
// NodeFormulaField 的运行时依赖（expression-trigger/NodeCandidateMenu/css）不 import NodeField.tsx，
// 单向边 NodeField → NodeFormulaField，无循环
import { NodeFormulaField, type FormulaCandidateSource } from "./NodeFormulaField";
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
  onCancel?: () => void;
  list?: string;
  onBlur?: () => void;
}

export function NodeTextField({ value, onCommit, placeholder, ariaLabel, className, invalid, onCancel, list, onBlur }: TextProps) {
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
    onCancel?.();
  };

  return (
    <input
      data-field-kind="text"
      className={`nodrag ${styles.field} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
      value={draft}
      placeholder={placeholder}
      aria-label={ariaLabel}
      aria-invalid={invalid || undefined}
      list={list}
      onFocus={() => { focusedRef.current = true; }}
      onChange={(e) => { setDraft(e.target.value); dirtyRef.current = true; }}
      onBlur={() => {
        const wasDirty = dirtyRef.current;
        commit();
        focusedRef.current = false;
        if (!wasDirty) setDraft(value);
        onBlur?.();
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
  className?: string;
  invalid?: boolean;
}

export function NodeSelect({ value, options, onCommit, ariaLabel, allowCustom, className, invalid }: SelectProps) {
  const opts = [...options];
  if (allowCustom && value && !options.some((o) => o.value === value)) {
    opts.push({ value, label: value });
  }
  return (
    <select
      data-field-kind="select"
      className={`nodrag ${styles.field} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
      value={value}
      aria-label={ariaLabel}
      aria-invalid={invalid || undefined}
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
  /** 公式候选源：传此值每行渲染 NodeFormulaField（触发式补全）；缺省退化 NodeTextField（既有行为） */
  candidates?: FormulaCandidateSource;
}

export function NodeArgsRows({ values, onChange, placeholder, candidates }: ArgsProps) {
  // 尾随空行折叠为一个可编辑槽位（防御：父组件可能传入）；"+" 按钮走本地 adding 态
  const trimmed = [...values];
  while (trimmed.length > 0 && trimmed[trimmed.length - 1] === "") trimmed.pop();
  const hasTrailing = trimmed.length !== values.length;
  const visible = hasTrailing ? [...trimmed, ""] : trimmed;
  const [adding, setAdding] = useState(false);

  const update = (i: number, v: string) => {
    const next = [...values];
    next[i] = v;
    onChange(next);
  };
  const remove = (i: number) => {
    onChange(values.filter((_, idx) => idx !== i));
  };
  const add = () => {
    if (!hasTrailing && !adding) setAdding(true);
  };
  const commitAdding = (v: string) => {
    setAdding(false);
    if (v.trim() !== "") onChange([...values, v]);
  };

  // 行字段：传 candidates 时用公式框（触发式补全，args 可为表达式），缺省退化普通文本（既有行为）
  const rowField = (v: string, opts: { onCommit: (nv: string) => void; ariaLabel: string; onCancel?: () => void; onBlur?: () => void }) =>
    candidates
      ? <NodeFormulaField value={v} candidates={candidates} placeholder={placeholder} {...opts} />
      : <NodeTextField value={v} placeholder={placeholder} {...opts} />;

  return (
    <div data-field-kind="args" className={styles.args}>
      {visible.map((v, i) => (
        <div key={i} className={styles.argsRow}>
          {rowField(v, { onCommit: (nv) => update(i, nv), ariaLabel: `参数 ${i + 1}` })}
          <button
            type="button"
            className={`nodrag ${styles.argsRemove}`}
            aria-label="×"
            onClick={() => remove(i)}
          >×</button>
        </div>
      ))}
      {adding && (
        <div className={styles.argsRow} data-field-kind="args-adding">
          {rowField("", { onCommit: commitAdding, onCancel: () => setAdding(false), onBlur: () => setAdding(false), ariaLabel: "新参数" })}
          <button
            type="button"
            className={`nodrag ${styles.argsRemove}`}
            aria-label="×"
            onClick={() => setAdding(false)}
          >×</button>
        </div>
      )}
      {!hasTrailing && !adding && (
        <button type="button" className={`nodrag ${styles.argsAdd}`} onClick={add}>+ 添加参数</button>
      )}
    </div>
  );
}
