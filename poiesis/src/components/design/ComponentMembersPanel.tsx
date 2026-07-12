// components/design/ComponentMembersPanel.tsx
// Left panel for editing component props, signals, and computed declarations in code flow mode.
// Uses Poiesis --ol-* design tokens for visual consistency.

import { useState, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { CodeEditorModal } from "../canvas/code/CodeEditorModal";
import { uid } from "../../core/utils/id";
import type { ComponentDef, DefEntry, ComputedEntry } from "../../core/types/project-json";

// ── OmniLang common types for dropdown ──
const OMNI_TYPES = [
  "String", "i32", "i64", "f64", "f32",
  "bool", "Array<String>", "Array<i32>", "Array<f64>",
  "Map<String, String>",
];

// ── Styles using --ol-* tokens to match Poiesis visual language ──

const S = {
  panel: {
    height: "100%",
    overflow: "auto",
    background: "var(--ol-gray-50)",
  } as React.CSSProperties,

  sectionHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "8px 12px",
    cursor: "pointer",
    userSelect: "none",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  addBtn: {
    padding: "1px 7px",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    background: "var(--ol-surface)",
    color: "var(--ol-text-secondary)",
    cursor: "pointer",
    lineHeight: "18px",
  } as React.CSSProperties,

  itemRow: {
    padding: "6px 12px",
    fontSize: "var(--ol-font-size-12)",
    borderBottom: "1px solid var(--ol-border-subtle)",
    background: "var(--ol-surface)",
  } as React.CSSProperties,

  itemHeader: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1)",
    marginBottom: 3,
  } as React.CSSProperties,

  input: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "none",
    width: "100%",
  } as React.CSSProperties,

  select: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 4px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "none",
    cursor: "pointer",
  } as React.CSSProperties,

  typeSep: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
  } as React.CSSProperties,

  valueRow: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1)",
  } as React.CSSProperties,

  removeBtn: {
    border: "none",
    background: "transparent",
    color: "var(--ol-text-muted)",
    cursor: "pointer",
    fontSize: "var(--ol-font-size-13)",
    padding: "0 2px",
    lineHeight: 1,
  } as React.CSSProperties,

  exprClickable: {
    flex: 1,
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
    color: "var(--ol-text-link)",
    cursor: "pointer",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    background: "var(--ol-color-primary-subtle)",
    padding: "2px 6px",
    borderRadius: "var(--ol-radius-sm)",
  } as React.CSSProperties,
};

interface Props {
  componentName: string;
}

export function ComponentMembersPanel({ componentName }: Props) {
  const componentDef = usePoiesisStore((s) => s.componentDefs[componentName]);
  const setComponentDef = usePoiesisStore((s) => s.setComponentDef);

  const [editingComputedPid, setEditingComputedPid] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  // Use a default ComponentDef when none exists yet (e.g. main.omni has no explicit entry).
  // The default is also used as the base for the initial setComponentDef call on first add.
  const def: ComponentDef = componentDef ?? {
    name: componentName,
    filePath: `src/${componentName}.omni`,
    signals: [],
    props: [],
    computeds: [],
    events: [],
    fns: [],
  };

  const toggle = (key: string) => setCollapsed((c) => ({ ...c, [key]: !c[key] }));

  const updateDef = (updates: Partial<ComponentDef>) => {
    setComponentDef(componentName, { ...def, ...updates });
  };

  // ── Props CRUD ──

  const handleAddProp = () => {
    const entry: DefEntry = { pid: uid("p"), name: "newProp", type: "String", defaultValue: undefined };
    updateDef({ props: [...def.props, entry] });
  };

  const handleUpdateProp = (pid: string, updates: Partial<DefEntry>) => {
    updateDef({
      props: def.props.map((p) => (p.pid === pid ? { ...p, ...updates } : p)),
    });
  };

  const handleRemoveProp = (pid: string) => {
    updateDef({
      props: def.props.filter((p) => p.pid !== pid),
    });
  };

  // ── Signals CRUD ──

  const handleAddSignal = () => {
    const entry: DefEntry = { pid: uid("sgn"), name: "newSignal", type: "String", defaultValue: "" };
    updateDef({ signals: [...def.signals, entry] });
  };

  const handleUpdateSignal = (pid: string, updates: Partial<DefEntry>) => {
    updateDef({
      signals: def.signals.map((s) => (s.pid === pid ? { ...s, ...updates } : s)),
    });
  };

  const handleRemoveSignal = (pid: string) => {
    updateDef({
      signals: def.signals.filter((s) => s.pid !== pid),
    });
  };

  // ── Computed CRUD ──

  const handleAddComputed = () => {
    const entry: ComputedEntry = { pid: uid("c"), name: "newComputed", returnType: "String", expression: "" };
    updateDef({ computeds: [...def.computeds, entry] });
  };

  const handleUpdateComputed = (pid: string, updates: Partial<ComputedEntry>) => {
    updateDef({
      computeds: def.computeds.map((c) => (c.pid === pid ? { ...c, ...updates } : c)),
    });
  };

  const handleRemoveComputed = (pid: string) => {
    updateDef({
      computeds: def.computeds.filter((c) => c.pid !== pid),
    });
  };

  const editingComputed = editingComputedPid
    ? def.computeds.find((c) => c.pid === editingComputedPid)
    : null;

  // ── All store defs for CodeEditorModal type injection ──
  const storeDefsObj = usePoiesisStore((s) => s.storeDefs);
  const allStoreDefs = useMemo(() => Object.values(storeDefsObj), [storeDefsObj]);

  return (
    <div style={S.panel}>
      {/* ── Props ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("props")}>
          <span>Props</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddProp(); }}>+</button>
        </div>
        {!collapsed.props && def.props.map((prop) => (
          <div key={prop.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input
                style={{ ...S.input, width: 72 }}
                value={prop.name}
                onChange={(e) => handleUpdateProp(prop.pid, { name: e.target.value })}
              />
              <span style={S.typeSep}>:</span>
              <select
                style={{ ...S.select, width: 128 }}
                value={prop.type}
                onChange={(e) => handleUpdateProp(prop.pid, { type: e.target.value })}
              >
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(prop.type) && <option value={prop.type}>{prop.type}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => handleRemoveProp(prop.pid)}>&times;</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <input
                style={S.input}
                value={prop.defaultValue !== undefined ? String(prop.defaultValue) : ""}
                placeholder="default (empty = required)"
                onChange={(e) => handleUpdateProp(prop.pid, { defaultValue: e.target.value || undefined })}
              />
            </div>
          </div>
        ))}
        {!collapsed.props && def.props.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No props yet
          </div>
        )}
      </div>

      {/* ── Signals ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("signals")}>
          <span>Signals</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddSignal(); }}>+</button>
        </div>
        {!collapsed.signals && def.signals.map((sig) => (
          <div key={sig.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input
                style={{ ...S.input, width: 72 }}
                value={sig.name}
                onChange={(e) => handleUpdateSignal(sig.pid, { name: e.target.value })}
              />
              <span style={S.typeSep}>:</span>
              <select
                style={{ ...S.select, width: 128 }}
                value={sig.type}
                onChange={(e) => handleUpdateSignal(sig.pid, { type: e.target.value })}
              >
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(sig.type) && <option value={sig.type}>{sig.type}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => handleRemoveSignal(sig.pid)}>&times;</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <input
                style={S.input}
                value={sig.defaultValue !== undefined ? String(sig.defaultValue) : ""}
                placeholder="default value"
                onChange={(e) => handleUpdateSignal(sig.pid, { defaultValue: e.target.value })}
              />
            </div>
          </div>
        ))}
        {!collapsed.signals && def.signals.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No signals yet
          </div>
        )}
      </div>

      {/* ── Computed ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("computeds")}>
          <span>Computed</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddComputed(); }}>+</button>
        </div>
        {!collapsed.computeds && def.computeds.map((c) => (
          <div key={c.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input
                style={{ ...S.input, width: 72 }}
                value={c.name}
                onChange={(e) => handleUpdateComputed(c.pid, { name: e.target.value })}
              />
              <span style={S.typeSep}>:</span>
              <select
                style={{ ...S.select, width: 128 }}
                value={c.returnType}
                onChange={(e) => handleUpdateComputed(c.pid, { returnType: e.target.value })}
              >
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(c.returnType) && <option value={c.returnType}>{c.returnType}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => handleRemoveComputed(c.pid)}>&times;</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <span
                style={S.exprClickable}
                onClick={() => setEditingComputedPid(c.pid)}
                title={c.expression || "Click to edit"}
              >
                {c.expression
                  ? c.expression.slice(0, 40) + (c.expression.length > 40 ? "…" : "")
                  : "click to edit"}
              </span>
            </div>
          </div>
        ))}
        {!collapsed.computeds && def.computeds.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No computeds yet
          </div>
        )}
      </div>

      {/* ── Computed expression editor modal ── */}
      {editingComputed && (
        <CodeEditorModal
          elementId={editingComputed.pid}
          initialSource={editingComputed.expression}
          initialDescription=""
          storeDefs={allStoreDefs}
          onSave={(source: string) => {
            handleUpdateComputed(editingComputed.pid, { expression: source });
            setEditingComputedPid(null);
          }}
          onClose={() => setEditingComputedPid(null)}
        />
      )}
    </div>
  );
}
