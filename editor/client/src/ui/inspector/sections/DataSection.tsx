/** Data section — component props editing by type. */

import React from "react";
import type { ElementData } from "../../../core/types";
import { SetProp } from "../../../core/commands/SetProp";
import { useUndoStore } from "../../../store/undo";
import type { CommandContext } from "../../../core/commands/types";

// Types

interface PropMeta {
  type: string;
  default?: unknown;
  values?: string[];
  editor: string;
}

interface ComponentMeta {
  component: string;
  category: string;
  props: Record<string, PropMeta>;
  events: Record<string, EventMeta>;
  children: boolean;
  isCustom?: boolean;
}

interface EventMeta {
  id?: string;
  description: string;
  payload: string | null;
}

interface DataSectionProps {
  element: ElementData;
  meta: ComponentMeta;
  ctx: CommandContext;
  onRecompile: () => void;
}

export function DataSection({ element: el, meta, ctx, onRecompile }: DataSectionProps) {
  if (Object.keys(meta.props).length === 0) return null;

  return (
    <>
      <div style={sectionHeaderStyle}>属性</div>
      {Object.entries(meta.props).map(([name, config]) => (
        <PropEditor
          key={name}
          name={name}
          config={config}
          value={el.props[name] ?? config.default}
          onChange={(v) => {
            useUndoStore.getState().execute(new SetProp(el.id, name, v), ctx);
            onRecompile();
          }}
        />
      ))}
    </>
  );
}

// ═══════════════════════════════════════════════
// PropEditor — renders individual property editors
// ═══════════════════════════════════════════════

interface PropEditorProps {
  name: string;
  config: PropMeta;
  value: unknown;
  onChange: (v: unknown) => void;
}

function PropEditor({ name, config, value, onChange }: PropEditorProps) {
  switch (config.editor) {
    case "text":
      return (
        <label style={labelStyle}>
          <span style={labelTextStyle}>{name}</span>
          <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={inputStyle} />
        </label>
      );
    case "select":
      return (
        <label style={labelStyle}>
          <span style={labelTextStyle}>{name}</span>
          <select value={String(value ?? config.default)} onChange={(e) => onChange(e.target.value)} style={{ ...inputStyle, background: "#fff" }}>
            {(config.values || []).map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
      );
    case "checkbox":
      return (
        <label style={{ ...labelStyle, flexDirection: "row", alignItems: "center", gap: 8 }}>
          <input type="checkbox" checked={Boolean(value ?? false)} onChange={(e) => onChange(e.target.checked)} />
          <span style={{ fontSize: 12 }}>{name}</span>
        </label>
      );
    default:
      return null;
  }
}

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const labelStyle: React.CSSProperties = { display: "flex", flexDirection: "column", marginTop: 6 };
const labelTextStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", marginBottom: 2 };
const inputStyle: React.CSSProperties = { padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4, fontSize: 12, width: "100%", boxSizing: "border-box" };
const sectionHeaderStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" };
