/** Monaco editor modal overlay — triggered by double-clicking a code block in wire canvas.
 *  Edits the block's own source code, not the whole file. */

import { useState, useCallback, useEffect } from "react";
import { MonacoEditor } from "./MonacoEditor";
import { usePoiesisStore } from "../../store";
import type { ElementData } from "../../core/types";
// NOTE(poiesis-v2): SnippetPanel was removed during Phase 0 restructuring.
import { TYPE_COLORS } from "../canvas/shared/socketTypes";

interface CodeEditorPopupProps {
  visible: boolean;
  blockId: string | null;
  title: string;
  source: string;
  blockElement?: ElementData | null;
  onClose: () => void;
  onSave: (newSource: string) => void;
}

/** Display the ports of a block as a small info bar */
function PortsBadge({ el }: { el?: ElementData | null }) {
  if (!el?.ports?.length) return null;
  const inputs = el.ports.filter(p => p.direction === "in");
  const outputs = el.ports.filter(p => p.direction === "out");
  return (
    <div style={{ display: "flex", gap: 12, fontSize: 10, color: "#6B7280" }}>
      {inputs.length > 0 && (
        <span>
          输入: {inputs.map(p => (
            <span key={p.name} style={{ marginLeft: 4, padding: "1px 5px", borderRadius: 3, background: (TYPE_COLORS[p.type] || "#6B7280") + "18", color: TYPE_COLORS[p.type] || "#6B7280", fontFamily: "monospace" }}>
              {p.label || p.name}:{p.type}
            </span>
          ))}
        </span>
      )}
      {outputs.length > 0 && (
        <span>
          输出: {outputs.map(p => (
            <span key={p.name} style={{ marginLeft: 4, padding: "1px 5px", borderRadius: 3, background: (TYPE_COLORS[p.type] || "#6B7280") + "18", color: TYPE_COLORS[p.type] || "#6B7280", fontFamily: "monospace" }}>
              {p.label || p.name}:{p.type}
            </span>
          ))}
        </span>
      )}
    </div>
  );
}

export function CodeEditorPopup({ visible, blockId, title, source, blockElement, onClose, onSave }: CodeEditorPopupProps) {
  const [draft, setDraft] = useState(source);
  const [modified, setModified] = useState(false);
  // NOTE: setBlockValue and setElementPorts are not on PoiesisState yet — migration needed
  const setBlockValue = (usePoiesisStore as any).getState().setBlockValue;
  const setElementPorts = (usePoiesisStore as any).getState().setElementPorts;

  useEffect(() => {
    if (visible) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- sync draft state when opening with new source
      setDraft(source);
      setModified(false);
    }
  }, [visible, source]);

  const handleChange = useCallback((value: string) => {
    setDraft(value);
    setModified(value !== source);
  }, [source]);

  const handleSave = useCallback(() => {
    if (blockId && modified) {
      setBlockValue(blockId, draft);
      if (blockElement?.ports) {
        setElementPorts(blockId, blockElement.ports);
      }
      onSave(draft);
    }
  }, [blockId, draft, modified, blockElement, setBlockValue, setElementPorts, onSave]);

  if (!visible || !blockId) return null;

  return (
    <div style={{
      position: "fixed", inset: 0, zIndex: 9999,
      display: "flex", alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.4)",
    }}>
      <div style={{
        width: "80%", maxWidth: 900, height: "70%",
        background: "#fff", borderRadius: 12,
        display: "flex", flexDirection: "column",
        boxShadow: "0 16px 48px rgba(0,0,0,0.25)",
        overflow: "hidden",
      }}>
        {/* Header */}
        <div style={{
          padding: "10px 16px", borderBottom: "1px solid #E5E7EB",
          display: "flex", alignItems: "center", justifyContent: "space-between",
          flexShrink: 0,
        }}>
          <div style={{ fontWeight: 600, fontSize: 13 }}>{title}</div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {modified && <span style={{ fontSize: 11, color: "#F59E0B" }}>未保存</span>}
            <button onClick={handleSave} style={{
              padding: "4px 12px", fontSize: 11, fontWeight: 600,
              background: "#4F46E5", color: "#fff", border: "none", borderRadius: 6,
              cursor: "pointer",
            }}>保存</button>
            <button onClick={onClose} style={{
              padding: "4px 10px", fontSize: 11,
              background: "#F3F4F6", color: "#374151", border: "1px solid #D1D5DB", borderRadius: 6,
              cursor: "pointer",
            }}>取消 (Esc)</button>
          </div>
        </div>

        {/* Ports info */}
        {blockElement && <div style={{ padding: "6px 16px", borderBottom: "1px solid #F3F4F6", background: "#F9FAFB", flexShrink: 0 }}>
          <PortsBadge el={blockElement} />
        </div>}

        {/* Editor + snippet panel */}
        <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <MonacoEditor value={draft} onChange={handleChange} />
          </div>
          {/* NOTE(poiesis-v2): SnippetPanel was removed during Phase 0 restructuring
          <div style={{ width: 220, minWidth: 180, flexShrink: 0, borderLeft: "1px solid #E5E7EB", background: "#FAFBFC", overflow: "auto" }}>
            Snippet panel placeholder
          </div>
          */}
        </div>

        {/* Footer */}
        <div style={{
          padding: "6px 16px", borderTop: "1px solid #e5e7eb",
          fontSize: 11, color: "#9CA3AF", display: "flex", gap: 16,
          flexShrink: 0,
        }}>
          <span>Ctrl+S 保存</span>
          <span>Esc 关闭</span>
        </div>
      </div>
    </div>
  );
}
