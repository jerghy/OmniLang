// components/store/StoreMembersPanel.tsx
// Left panel for the Store editor: grouped lists of Signals, Getters, and Actions
// with inline editing, add/remove, and getter expression editing via CodeEditorModal.

import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { CodeEditorModal } from "../canvas/code/CodeEditorModal";

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

  if (!store) return <div style={{ padding: 16, color: "#999" }}>Store not found: {storeName}</div>;

  const toggle = (key: string) => setCollapsed((c) => ({ ...c, [key]: !c[key] }));

  const handleAddSignal = () => {
    const storePid = store.pid;
    const idx = store.signals.length + 1;
    addStoreSignal(storeName, {
      pid: `${storePid}_n${idx}`,
      name: "newSignal",
      type: "String",
      defaultValue: "",
    });
  };

  const handleAddGetter = () => {
    const storePid = store.pid;
    const idx = store.getters.length + store.signals.length + 1;
    addStoreGetter(storeName, {
      pid: `${storePid}_n${idx}`,
      name: "newGetter",
      returnType: "String",
      expression: "",
    });
  };

  const handleAddAction = () => {
    const storePid = store.pid;
    const idx = store.actions.length + store.signals.length + store.getters.length + 1;
    const actionPid = `${storePid}_n${idx}`;
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
  };

  const editingGetter = editingGetterPid
    ? store.getters.find((g) => g.pid === editingGetterPid)
    : null;

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 8, borderRight: "1px solid #e5e7eb" }}>
      {/* Signals */}
      <div style={{ marginBottom: 8 }}>
        <div
          style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }}
          onClick={() => toggle("signals")}
        >
          <strong>Signals</strong>
          <button onClick={(e) => { e.stopPropagation(); handleAddSignal(); }} style={{ padding: "2px 6px" }}>+</button>
        </div>
        {!collapsed.signals && store.signals.map((sig) => (
          <div key={sig.pid} style={{ padding: "4px 8px", background: "#f9fafb", borderRadius: 4, marginTop: 4, fontSize: 13 }}>
            <input
              style={{ width: "40%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={sig.name}
              onChange={(e) => updateStoreSignal(storeName, sig.pid, { name: e.target.value })}
            />
            <span> : </span>
            <input
              style={{ width: "25%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={sig.type}
              onChange={(e) => updateStoreSignal(storeName, sig.pid, { type: e.target.value })}
            />
            {sig.defaultValue !== undefined && (
              <>
                <span> = </span>
                <input
                  style={{ width: "20%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
                  value={String(sig.defaultValue ?? "")}
                  onChange={(e) => updateStoreSignal(storeName, sig.pid, { defaultValue: e.target.value })}
                />
              </>
            )}
            <button onClick={() => removeStoreSignal(storeName, sig.pid)} style={{ marginLeft: 4, color: "#ef4444", border: "none", background: "none", cursor: "pointer" }}>✕</button>
          </div>
        ))}
      </div>

      {/* Getters */}
      <div style={{ marginBottom: 8 }}>
        <div
          style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }}
          onClick={() => toggle("getters")}
        >
          <strong>Getters</strong>
          <button onClick={(e) => { e.stopPropagation(); handleAddGetter(); }} style={{ padding: "2px 6px" }}>+</button>
        </div>
        {!collapsed.getters && store.getters.map((g) => (
          <div key={g.pid} style={{ padding: "4px 8px", background: "#f9fafb", borderRadius: 4, marginTop: 4, fontSize: 13 }}>
            <input
              style={{ width: "35%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={g.name}
              onChange={(e) => updateStoreGetter(storeName, g.pid, { name: e.target.value })}
            />
            <span> : </span>
            <input
              style={{ width: "20%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={g.returnType}
              onChange={(e) => updateStoreGetter(storeName, g.pid, { returnType: e.target.value })}
            />
            <span
              style={{ display: "block", marginTop: 2, fontSize: 12, color: "#6366f1", cursor: "pointer" }}
              onClick={() => setEditingGetterPid(g.pid)}
            >
              {g.expression ? g.expression.slice(0, 40) + (g.expression.length > 40 ? "..." : "") : "[click to edit expression]"}
            </span>
            <button onClick={() => removeStoreGetter(storeName, g.pid)} style={{ marginLeft: 4, color: "#ef4444", border: "none", background: "none", cursor: "pointer" }}>✕</button>
          </div>
        ))}
      </div>

      {/* Actions */}
      <div style={{ marginBottom: 8 }}>
        <div
          style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }}
          onClick={() => toggle("actions")}
        >
          <strong>Actions</strong>
          <button onClick={(e) => { e.stopPropagation(); handleAddAction(); }} style={{ padding: "2px 6px" }}>+</button>
        </div>
        {!collapsed.actions && store.actions.map((a) => {
          const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
          const retStr = a.returnType ? `: ${a.returnType}` : "";
          const asyncStr = a.isAsync ? " async" : "";
          const isSelected = selectedActionPid === a.pid;
          return (
            <div
              key={a.pid}
              onClick={() => onSelectAction(a.pid)}
              style={{
                padding: "6px 8px",
                background: isSelected ? "#eef2ff" : "#f9fafb",
                borderRadius: 4,
                marginTop: 4,
                fontSize: 13,
                cursor: "pointer",
                border: isSelected ? "2px solid #6366f1" : "1px solid #e5e7eb",
                fontWeight: isSelected ? 600 : 400,
              }}
            >
              {isSelected && "● "}{a.name}({paramsStr}){retStr}{asyncStr}
              <button
                onClick={(e) => { e.stopPropagation(); removeStoreAction(storeName, a.pid); }}
                style={{ float: "right", color: "#ef4444", border: "none", background: "none", cursor: "pointer" }}
              >✕</button>
            </div>
          );
        })}
      </div>

      {/* Getter Monaco Modal */}
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
