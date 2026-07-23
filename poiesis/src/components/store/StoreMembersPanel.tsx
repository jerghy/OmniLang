// components/store/StoreMembersPanel.tsx
// Left panel for the Store editor: Signals, Getters, Actions sections.
// Uses Poiesis --ol-* design tokens for visual consistency.
// Supports storeOverride + CRUD callbacks for inline store drawer mode.

import { useState, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { CodeEditorModal } from "../canvas/code/CodeEditorModal";
import type { StoreDefinition, StoreSignalDef, StoreGetterDef, StoreActionDef } from "../../core/types/store";

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
    outline: "2px solid transparent",
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
    outline: "2px solid transparent",
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
  /** Override store lookup (for inline stores in drawer mode).
   *  When provided, used instead of s.storeDefs[storeName]. */
  storeOverride?: StoreDefinition;
  // CRUD callbacks (required when storeOverride is set; used instead of Zustand actions)
  onAddSignal?: (signal: StoreSignalDef) => void;
  onUpdateSignal?: (pid: string, updates: Partial<StoreSignalDef>) => void;
  onRemoveSignal?: (pid: string) => void;
  onAddGetter?: (getter: StoreGetterDef) => void;
  onUpdateGetter?: (pid: string, updates: Partial<StoreGetterDef>) => void;
  onRemoveGetter?: (pid: string) => void;
  onAddAction?: (action: StoreActionDef) => void;
  onRemoveAction?: (pid: string) => void;
}

export function StoreMembersPanel({
  storeName,
  selectedActionPid,
  onSelectAction,
  storeOverride,
  onAddSignal,
  onUpdateSignal,
  onRemoveSignal,
  onAddGetter,
  onUpdateGetter,
  onRemoveGetter,
  onAddAction,
  onRemoveAction,
}: Props) {
  // Store data: use override when provided, otherwise read from global storeDefs
  const globalStore = usePoiesisStore((s) => s.storeDefs[storeName]);
  const store = storeOverride ?? globalStore;

  // Read all store defs and convert to array (memoized to avoid reference changes)
  const storeDefsObj = usePoiesisStore((s) => s.storeDefs);
  const allStoreDefs = useMemo(
    () => Object.values(storeDefsObj),
    [storeDefsObj],
  );

  // Zustand CRUD actions (global store; used when storeOverride is not set)
  const zAddStoreSignal = usePoiesisStore((s) => s.addStoreSignal);
  const zUpdateStoreSignal = usePoiesisStore((s) => s.updateStoreSignal);
  const zRemoveStoreSignal = usePoiesisStore((s) => s.removeStoreSignal);
  const zAddStoreGetter = usePoiesisStore((s) => s.addStoreGetter);
  const zUpdateStoreGetter = usePoiesisStore((s) => s.updateStoreGetter);
  const zRemoveStoreGetter = usePoiesisStore((s) => s.removeStoreGetter);
  const zAddStoreAction = usePoiesisStore((s) => s.addStoreAction);
  const zRemoveStoreAction = usePoiesisStore((s) => s.removeStoreAction);

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

  // ── CRUD routing: use callback props when storeOverride is set ──

  const handleAddSignal = () => {
    const sig: StoreSignalDef = {
      pid: `${store.pid}_n${nextN()}`,
      name: "newSignal",
      type: "String",
      defaultValue: "",
    };
    if (storeOverride && onAddSignal) {
      onAddSignal(sig);
    } else {
      zAddStoreSignal(storeName, sig);
    }
  };

  const handleAddGetter = () => {
    const getter: StoreGetterDef = {
      pid: `${store.pid}_n${nextN()}`,
      name: "newGetter",
      returnType: "String",
      expression: "",
    };
    if (storeOverride && onAddGetter) {
      onAddGetter(getter);
    } else {
      zAddStoreGetter(storeName, getter);
    }
  };

  const handleAddAction = () => {
    const actionPid = `${store.pid}_n${nextN()}`;
    const fnPid = `${actionPid}_c1`;
    const action: StoreActionDef = {
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
    };
    if (storeOverride && onAddAction) {
      onAddAction(action);
    } else {
      zAddStoreAction(storeName, action);
    }
    onSelectAction(actionPid);
  };

  // ── Inline CRUD helpers used in JSX ──

  const updateSignal = (pid: string, updates: Partial<StoreSignalDef>) => {
    if (storeOverride && onUpdateSignal) onUpdateSignal(pid, updates);
    else zUpdateStoreSignal(storeName, pid, updates);
  };

  const removeSignal = (pid: string) => {
    if (storeOverride && onRemoveSignal) onRemoveSignal(pid);
    else zRemoveStoreSignal(storeName, pid);
  };

  const updateGetter = (pid: string, updates: Partial<StoreGetterDef>) => {
    if (storeOverride && onUpdateGetter) onUpdateGetter(pid, updates);
    else zUpdateStoreGetter(storeName, pid, updates);
  };

  const removeGetter = (pid: string) => {
    if (storeOverride && onRemoveGetter) onRemoveGetter(pid);
    else zRemoveStoreGetter(storeName, pid);
  };

  const removeAction = (pid: string) => {
    if (storeOverride && onRemoveAction) onRemoveAction(pid);
    else zRemoveStoreAction(storeName, pid);
  };

  const editingGetter = editingGetterPid
    ? store.getters.find((g) => g.pid === editingGetterPid)
    : null;

  return (
    <div style={S.panel}>
      {/* ── Signals ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("signals")} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle("signals"); } }}>
          <span>Signals</span>
          <button type="button" style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddSignal(); }}>+</button>
        </div>
        {!collapsed.signals && store.signals.map((sig) => (
          <div key={sig.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input aria-label="Signal name" style={{ ...S.input, width: 72 }} value={sig.name}
                onChange={(e) => updateSignal(sig.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select aria-label="Signal type" style={{ ...S.select, width: 128 }} value={sig.type}
                onChange={(e) => updateSignal(sig.pid, { type: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(sig.type) && <option value={sig.type}>{sig.type}</option>}
              </select>
              <button type="button" style={S.removeBtn} onClick={() => removeSignal(sig.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <input aria-label="Signal default value" style={S.input} value={sig.defaultValue !== undefined ? String(sig.defaultValue) : ""}
                placeholder="default value"
                onChange={(e) => updateSignal(sig.pid, { defaultValue: e.target.value })} />
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
        <div style={S.sectionHeader} onClick={() => toggle("getters")} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle("getters"); } }}>
          <span>Getters</span>
          <button type="button" style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddGetter(); }}>+</button>
        </div>
        {!collapsed.getters && store.getters.map((g) => (
          <div key={g.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input aria-label="Getter name" style={{ ...S.input, width: 72 }} value={g.name}
                onChange={(e) => updateGetter(g.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select aria-label="Getter return type" style={{ ...S.select, width: 128 }} value={g.returnType}
                onChange={(e) => updateGetter(g.pid, { returnType: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(g.returnType) && <option value={g.returnType}>{g.returnType}</option>}
              </select>
              <button type="button" style={S.removeBtn} onClick={() => removeGetter(g.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <button type="button" tabIndex={0} style={S.getterExpr} onClick={() => setEditingGetterPid(g.pid)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setEditingGetterPid(g.pid); } }} title={g.expression || "Click to edit"}>
                {g.expression ? g.expression.slice(0, 40) + (g.expression.length > 40 ? "…" : "") : "click to edit"}
              </button>
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
        <div style={S.sectionHeader} onClick={() => toggle("actions")} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle("actions"); } }}>
          <span>Actions</span>
          <button type="button" style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddAction(); }}>+</button>
        </div>
        {!collapsed.actions && store.actions.map((a) => {
          const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
          const isSelected = selectedActionPid === a.pid;
          return (
            <div key={a.pid} style={S.actionItem(isSelected)} onClick={() => onSelectAction(a.pid)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelectAction(a.pid); } }}>
              <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", fontFamily: "var(--ol-font-family-mono)" }}>
                {a.name}({paramsStr}){a.returnType ? `: ${a.returnType}` : ""}{a.isAsync ? " async" : ""}
              </span>
              <button type="button"
                style={S.removeBtn}
                onClick={(e) => { e.stopPropagation(); removeAction(a.pid); }}
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
        {store.imports.map((imp) => (
          <div key={imp} style={{ wordBreak: "break-all" }}>{imp}</div>
        ))}
      </div>

      {/* ── Getter expression editor modal ── */}
      {editingGetter && (
        <CodeEditorModal
          elementId={editingGetter.pid}
          initialSource={editingGetter.expression}
          initialDescription=""
          storeDefs={allStoreDefs}
          onSave={(source: string) => {
            updateGetter(editingGetter.pid, { expression: source });
            setEditingGetterPid(null);
          }}
          onClose={() => setEditingGetterPid(null)}
        />
      )}
    </div>
  );
}
