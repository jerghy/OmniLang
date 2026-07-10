// components/store/StoreMembersPanel.tsx
// Left panel for the Store editor: Signals, Getters, Actions sections.
// Uses Poiesis --ol-* design tokens for visual consistency.

import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { CodeEditorModal } from "../canvas/code/CodeEditorModal";

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

  actionItem: (selected: boolean): React.CSSProperties => ({
    display: "flex",
    alignItems: "center",
    padding: "5px 12px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    cursor: "pointer",
    borderBottom: "1px solid var(--ol-border-subtle)",
    background: selected ? "var(--ol-surface-active)" : "var(--ol-surface)",
    borderLeft: selected ? "3px solid var(--ol-color-primary)" : "3px solid transparent",
    fontWeight: selected ? "var(--ol-font-weight-500)" : "var(--ol-font-weight-400)",
    color: selected ? "var(--ol-color-primary)" : "var(--ol-text)",
  }),

  getterExpr: {
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

  importsArea: {
    borderTop: "1px solid var(--ol-border)",
    padding: "8px 12px",
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
    color: "var(--ol-text-muted)",
    lineHeight: 1.6,
  } as React.CSSProperties,
};

interface Props {
  storeName: string;
  selectedActionPid: string | null;
  onSelectAction: (pid: string) => void;
}

export function StoreMembersPanel({ storeName, selectedActionPid, onSelectAction }: Props) {
  const store = usePoiesisStore((s) => s.storeDefs[storeName]);
  const addStoreSignal = usePoiesisStore((s) => s.addStoreSignal);
  const updateStoreSignal = usePoiesisStore((s) => s.updateStoreSignal);
  const removeStoreSignal = usePoiesisStore((s) => s.removeStoreSignal);
  const addStoreGetter = usePoiesisStore((s) => s.addStoreGetter);
  const updateStoreGetter = usePoiesisStore((s) => s.updateStoreGetter);
  const removeStoreGetter = usePoiesisStore((s) => s.removeStoreGetter);
  const addStoreAction = usePoiesisStore((s) => s.addStoreAction);
  const removeStoreAction = usePoiesisStore((s) => s.removeStoreAction);

  const [editingGetterPid, setEditingGetterPid] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  if (!store) return null;

  const toggle = (key: string) => setCollapsed((c) => ({ ...c, [key]: !c[key] }));

  /** Find the next available member index (max of existing PID suffixes + 1). */
  const nextN = () => {
    const nums = [
      ...store.signals.map((s) => parseInt(s.pid.split("_n").pop() || "0")),
      ...store.getters.map((g) => parseInt(g.pid.split("_n").pop() || "0")),
      ...store.actions.map((a) => parseInt(a.pid.split("_n").pop() || "0")),
    ].filter((n) => !isNaN(n));
    return (nums.length > 0 ? Math.max(...nums) : 0) + 1;
  };

  const handleAddSignal = () => {
    addStoreSignal(storeName, {
      pid: `${store.pid}_n${nextN()}`,
      name: "newSignal",
      type: "String",
      defaultValue: "",
    });
  };

  const handleAddGetter = () => {
    addStoreGetter(storeName, {
      pid: `${store.pid}_n${nextN()}`,
      name: "newGetter",
      returnType: "String",
      expression: "",
    });
  };

  const handleAddAction = () => {
    const actionPid = `${store.pid}_n${nextN()}`;
    const fnPid = `${actionPid}_c1`;
    addStoreAction(storeName, {
      pid: actionPid,
      name: "newAction",
      params: [],
      returnType: undefined,
      isAsync: false,
      elements: [{
        id: fnPid,
        nodeType: "codeblock",
        tag: "fn",
        parentId: null,
        props: { name: "newAction", params: "" },
        source: "",
      }],
      wires: [],
    });
    onSelectAction(actionPid);
  };

  const editingGetter = editingGetterPid
    ? store.getters.find((g) => g.pid === editingGetterPid)
    : null;

  return (
    <div style={S.panel}>
      {/* ── Signals ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("signals")}>
          <span>Signals</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddSignal(); }}>+</button>
        </div>
        {!collapsed.signals && store.signals.map((sig) => (
          <div key={sig.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input style={{ ...S.input, width: 72 }} value={sig.name}
                onChange={(e) => updateStoreSignal(storeName, sig.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select style={{ ...S.select, width: 128 }} value={sig.type}
                onChange={(e) => updateStoreSignal(storeName, sig.pid, { type: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(sig.type) && <option value={sig.type}>{sig.type}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => removeStoreSignal(storeName, sig.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <input style={S.input} value={sig.defaultValue !== undefined ? String(sig.defaultValue) : ""}
                placeholder="default value"
                onChange={(e) => updateStoreSignal(storeName, sig.pid, { defaultValue: e.target.value })} />
            </div>
          </div>
        ))}
        {!collapsed.signals && store.signals.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No signals yet
          </div>
        )}
      </div>

      {/* ── Getters ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("getters")}>
          <span>Getters</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddGetter(); }}>+</button>
        </div>
        {!collapsed.getters && store.getters.map((g) => (
          <div key={g.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input style={{ ...S.input, width: 72 }} value={g.name}
                onChange={(e) => updateStoreGetter(storeName, g.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select style={{ ...S.select, width: 128 }} value={g.returnType}
                onChange={(e) => updateStoreGetter(storeName, g.pid, { returnType: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(g.returnType) && <option value={g.returnType}>{g.returnType}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => removeStoreGetter(storeName, g.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <span style={S.getterExpr} onClick={() => setEditingGetterPid(g.pid)} title={g.expression || "Click to edit"}>
                {g.expression ? g.expression.slice(0, 40) + (g.expression.length > 40 ? "…" : "") : "click to edit"}
              </span>
            </div>
          </div>
        ))}
        {!collapsed.getters && store.getters.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No getters yet
          </div>
        )}
      </div>

      {/* ── Actions ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("actions")}>
          <span>Actions</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddAction(); }}>+</button>
        </div>
        {!collapsed.actions && store.actions.map((a) => {
          const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
          const isSelected = selectedActionPid === a.pid;
          return (
            <div key={a.pid} style={S.actionItem(isSelected)} onClick={() => onSelectAction(a.pid)}>
              <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", fontFamily: "var(--ol-font-family-mono)" }}>
                {a.name}({paramsStr}){a.returnType ? `: ${a.returnType}` : ""}{a.isAsync ? " async" : ""}
              </span>
              <button
                style={S.removeBtn}
                onClick={(e) => { e.stopPropagation(); removeStoreAction(storeName, a.pid); }}
              >×</button>
            </div>
          );
        })}
        {!collapsed.actions && store.actions.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No actions yet
          </div>
        )}
      </div>

      {/* ── Imports ── */}
      <div style={S.importsArea}>
        {store.imports.map((imp, i) => (
          <div key={i} style={{ wordBreak: "break-all" }}>{imp}</div>
        ))}
      </div>

      {/* ── Getter expression editor modal ── */}
      {editingGetter && (
        <CodeEditorModal
          elementId={editingGetter.pid}
          initialSource={editingGetter.expression}
          initialDescription=""
          onSave={(source: string) => {
            updateStoreGetter(storeName, editingGetter.pid, { expression: source });
            setEditingGetterPid(null);
          }}
          onClose={() => setEditingGetterPid(null)}
        />
      )}
    </div>
  );
}
