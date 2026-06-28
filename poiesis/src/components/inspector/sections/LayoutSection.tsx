/** Layout section — instance info, element ID editor. */

import { useState } from "react";
import type { ElementData } from "../../../core/types";
import { useEditorStore } from "../../../store/editor";

interface LayoutSectionProps {
  element: ElementData;
  onRecompile: () => void;
}

export function LayoutSection({ element: el, onRecompile }: LayoutSectionProps) {
  const elements = useEditorStore((s) => s.elements);

  return (
    <>
      {/* Instance info for custom components */}
      {el.componentRef && (
        <div style={{ marginTop: 8, padding: "4px 8px", background: "#F5F3FF", borderRadius: 4, fontSize: 10, color: "#6D28D9" }}>
          📦 实例化自: <strong>{el.componentRef}</strong>
        </div>
      )}

      {/* ID — 可编辑 */}
      <label style={{ marginTop: 16, paddingTop: 8, borderTop: "1px solid #e5e7eb", display: "flex", flexDirection: "column", gap: 2 }}>
        <span style={{ fontSize: 10, color: "#6b7280", fontWeight: 600 }}>ID</span>
        <IdEditor
          currentId={el.id}
          allIds={new Set(elements.map((x) => x.id))}
          onRename={(newId) => {
            useEditorStore.getState().setElementId(el.id, newId);
            onRecompile();
          }}
        />
      </label>
    </>
  );
}

// ═══════════════════════════════════════════════
// ID Editor — inline editable ID with duplicate check
// ═══════════════════════════════════════════════

function IdEditor({ currentId, allIds, onRename }: {
  currentId: string;
  allIds: Set<string>;
  onRename: (newId: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(currentId);
  const [error, setError] = useState("");

  const handleSave = () => {
    const trimmed = draft.trim();
    if (!trimmed) { setError("ID 不能为空"); return; }
    if (trimmed === currentId) { setEditing(false); setError(""); return; }
    if (allIds.has(trimmed)) { setError(`ID "${trimmed}" 已存在`); return; }
    onRename(trimmed);
    setEditing(false);
    setError("");
  };

  const handleCancel = () => {
    setDraft(currentId);
    setEditing(false);
    setError("");
  };

  if (!editing) {
    return (
      <div
        onClick={() => { setDraft(currentId); setEditing(true); }}
        style={{
          padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4,
          fontSize: 12, cursor: "pointer", display: "flex", alignItems: "center",
          gap: 6, color: "#374151", fontFamily: "monospace",
        }}
        title="点击编辑 ID"
      >
        <span style={{ flex: 1 }}>{currentId}</span>
        <span style={{ fontSize: 10, color: "#9ca3af" }}>✏️</span>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <div style={{ display: "flex", gap: 4 }}>
        <input
          type="text"
          value={draft}
          onChange={(e) => { setDraft(e.target.value); setError(""); }}
          onKeyDown={(e) => { if (e.key === "Enter") handleSave(); if (e.key === "Escape") handleCancel(); }}
          style={{
            flex: 1, padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4,
            fontSize: 12, fontFamily: "monospace", outline: "none",
            borderColor: error ? "#DC2626" : "#d1d5db",
          }}
          placeholder="唯一标识"
          autoFocus
        />
        <button onClick={handleSave} style={{
          padding: "3px 8px", fontSize: 10, border: "none", borderRadius: 4,
          background: "#4F46E5", color: "#fff", cursor: "pointer",
        }}>✓</button>
        <button onClick={handleCancel} style={{
          padding: "3px 8px", fontSize: 10, border: "1px solid #d1d5db", borderRadius: 4,
          background: "#fff", cursor: "pointer", color: "#6B7280",
        }}>✕</button>
      </div>
      {error && <div style={{ fontSize: 10, color: "#DC2626" }}>❌ {error}</div>}
    </div>
  );
}
