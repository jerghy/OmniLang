// components/design/PreviewElementHeader.tsx
// Header for the ClassPreview panel with element type selector dropdown.

interface PreviewElementOption {
  value: string;
  label: string;
}

interface PreviewElementHeaderProps {
  elementIdx: number;
  options: readonly PreviewElementOption[];
  onChange: (idx: number) => void;
}

import styles from "./PreviewElementHeader.module.css";

export function PreviewElementHeader({
  elementIdx,
  options,
  onChange,
}: PreviewElementHeaderProps) {
  return (
    <div className={styles.header}>
      Preview
      <select
        aria-label="Preview element"
        value={elementIdx}
        onChange={(e) => onChange(Number(e.target.value))}
        className={styles.elementSelect}
      >
        {options.map((e, i) => (
          <option key={e.value} value={i}>{e.label}</option>
        ))}
      </select>
    </div>
  );
}
