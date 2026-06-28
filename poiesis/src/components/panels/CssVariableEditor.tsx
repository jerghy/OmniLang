// ui/panels/CssVariableEditor.tsx
// List of CSS variable rows with fude value editors.

import type { CssVariable, StyleValueSegment } from "../../core/types/style";
import { StyleValueEditor } from "./StyleValueEditor";

interface SignalOption {
  name: string;
  label: string;
}

interface Props {
  variables: CssVariable[];
  onAdd: () => void;
  onRemove: (id: string) => void;
  onChange: (id: string, patch: Partial<CssVariable>) => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];
}

export function CssVariableEditor({
  variables,
  onAdd,
  onRemove,
  onChange,
  availableSignals,
  availableCssVars,
}: Props) {
  return (
    <div style={{
      border: "1px solid #e5e7eb", borderRadius: 6, padding: 8,
      background: "#fafafa",
    }}>
      {variables.map((v) => (
        <div key={v.id} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
          <input
            type="text"
            value={v.name}
            onChange={(e) => onChange(v.id, { name: e.target.value })}
            placeholder="--variable-name"
            style={{
              border: "1px solid #d1d5db", borderRadius: 4, padding: "4px 8px",
              fontSize: 12, width: 160, flex: "none",
            }}
          />
          <span style={{ fontSize: 12, color: "#6B7280", flex: "none" }}>:</span>
          <StyleValueEditor
            segments={v.segments}
            onChange={(segments: StyleValueSegment[]) => onChange(v.id, { segments })}
            availableSignals={availableSignals}
            availableCssVars={availableCssVars}
          />
          <button
            onClick={() => onRemove(v.id)}
            style={{
              background: "none", border: "none", color: "#DC2626",
              cursor: "pointer", fontSize: 14, padding: "0 4px", flex: "none",
            }}
          >
            ×
          </button>
        </div>
      ))}
      <button
        onClick={onAdd}
        style={{
          background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
          color: "#4F46E5", cursor: "pointer", fontSize: 11,
          padding: "4px 8px", width: "100%",
        }}
      >
        + 添加变量
      </button>
    </div>
  );
}
