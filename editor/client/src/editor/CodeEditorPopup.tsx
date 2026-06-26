/** Monaco editor modal overlay — triggered by double-clicking a code block in wire canvas.
 *  Edits the block's own source code, not the whole file. */

import { useState, useCallback, useEffect, useRef } from "react";
import { MonacoEditor, type MonacoEditorHandle } from "./MonacoEditor";
import { useEditorStore } from "../store/editor";
import type { ElementData } from "../core/types";
import { SnippetPanel } from "../panels/SnippetPanel";
import { TYPE_COLORS } from "../canvas/socketTypes";

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
  const setBlockValue = useEditorStore(s => s.setBlockValue);
  const setElementPorts = useEditorStore(s => s.setElementPorts);

  useEffect(() => {
    if (visible) {
      setDraft(source);
      setModified(false);
    }
  }, [visible, source]);

  const handleChange = useCallback((value: string) => {
    setDraft(value);
    setModified(value !== source);
  }, [source]);

  const handleSave = useCallback(() => {
    onSave(draft);
    setModified(false);
    // After saving code, sync runtime value + port types
    if (blockId) {
      syncBlockFromSource(blockId, draft, setBlockValue, setElementPorts);
    }
  }, [draft, onSave, blockId, setBlockValue, setElementPorts]);

  const handleClose = useCallback(() => {
    if (modified) {
      if (window.confirm("有未保存的更改，是否保存？")) {
        onSave(draft);
        if (blockId) syncBlockFromSource(blockId, draft, setBlockValue, setElementPorts);
      }
    }
    onClose();
    setModified(false);
  }, [modified, draft, onSave, onClose, blockId, setBlockValue, setElementPorts]);

  // Close on Escape
  useEffect(() => {
    if (!visible) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
      if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); handleSave(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [visible, handleClose, handleSave]);

  const editorRef = useRef<MonacoEditorHandle>(null);

  if (!visible) return null;

  return (
    <div
      style={{
        position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
        background: "rgba(0,0,0,0.5)", zIndex: 1000,
        display: "flex", alignItems: "center", justifyContent: "center",
      }}
      onClick={(e) => { if (e.target === e.currentTarget) handleClose(); }}
    >
      <div style={{
        width: "90vw", height: "85vh",
        background: "#fff", borderRadius: 12,
        boxShadow: "0 8px 40px rgba(0,0,0,0.2)",
        display: "flex", flexDirection: "column",
        overflow: "hidden",
      }}>
        {/* Header */}
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "10px 16px", borderBottom: "1px solid #e5e7eb",
          background: "#f9fafb", flexShrink: 0,
        }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: 14 }}>💻</span>
              <span style={{ fontSize: 13, fontWeight: 600, color: "#1F2937" }}>{title}</span>
              {blockId && <span style={{ fontSize: 10, color: "#9CA3AF", fontFamily: "monospace" }}>{blockId.slice(0, 16)}...</span>}
            </div>
            <PortsBadge el={blockElement} />
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            {modified && <span style={{ fontSize: 11, color: "#D97706", alignSelf: "center" }}>已修改</span>}
            <button onClick={handleSave} style={{
              padding: "4px 16px", fontSize: 12, fontWeight: 600,
              background: modified ? "#4F46E5" : "#E5E7EB",
              color: modified ? "#fff" : "#6B7280",
              border: "none", borderRadius: 6, cursor: "pointer",
            }}>
              保存 (Ctrl+S)
            </button>
            <button onClick={handleClose} style={{
              padding: "4px 12px", fontSize: 12,
              background: "transparent", border: "1px solid #d1d5db",
              borderRadius: 6, cursor: "pointer", color: "#6B7280",
            }}>
              ✕
            </button>
          </div>
        </div>

        {/* Body: Monaco editor (left) + Snippet panel (right) */}
        <div style={{ flex: 1, display: "flex", overflow: "hidden", minHeight: 0 }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <MonacoEditor value={draft} onChange={handleChange} />
          </div>
          <div style={{ width: 220, minWidth: 180, flexShrink: 0, borderLeft: "1px solid #E5E7EB", background: "#FAFBFC", overflow: "auto" }}>
            <SnippetPanel
              source={draft}
              cursorOffset={0}
              onInsert={(body) => setDraft(prev => prev + "\n" + body)}
            />
          </div>
        </div>

        {/* Footer */}
        <div style={{
          padding: "6px 16px", borderTop: "1px solid #e5e7eb",
          fontSize: 11, color: "#9CA3AF", display: "flex", gap: 16,
          flexShrink: 0,
        }}>
          <span>Ctrl+S 保存</span>
          <span>Esc 关闭</span>
          <span style={{ flex: 1 }} />
          <span>💡 代码块的输入端口接收上游数据，输出端口传递计算结果</span>
        </div>
      </div>
    </div>
  );
}

// ── Block evaluation + port sync ──────────────────

type PortDef = { name: string; direction: "in" | "out"; type: string; label?: string };

function syncBlockFromSource(
  blockId: string,
  source: string,
  setBlockValue: (id: string, value: unknown) => void,
  setElementPorts: (id: string, ports: PortDef[] | undefined) => void,
) {
  try {
    // ── signal: name : TYPE = VALUE ──
    const signalMatch = source.match(/signal\s+(\w+)\s*:\s*(\w+)\s*=\s*([^;\n]+)/);
    // ── computed: name : TYPE { ... return EXPR } ──
    const computedMatch = source.match(/computed\s+(\w+)\s*:\s*(\w+)\s*\{[^}]*return\s+([^;\n}]+)/s);
    // ── action: name(event) { } ──
    const actionMatch = source.match(/action\s+(\w+)\s*\(/);

    if (signalMatch) {
      const parsedType = signalMatch[2].trim();
      const val = signalMatch[3].trim();
      const parsed = tryParseValue(val);
      setBlockValue(blockId, parsed);
      // Sync ports with actual type from source
      setElementPorts(blockId, [
        { name: "value", direction: "out", type: parsedType, label: `${signalMatch[1]}:${parsedType}` },
        { name: "set", direction: "in", type: parsedType, label: `set ${signalMatch[1]}` },
      ]);
    } else if (computedMatch) {
      const parsedType = computedMatch[2].trim();
      setBlockValue(blockId, `expr: ${computedMatch[3].trim()}`);
      setElementPorts(blockId, [
        { name: "value", direction: "out", type: parsedType, label: `${computedMatch[1]}:${parsedType}` },
        { name: "deps", direction: "in", type: "any", label: "依赖输入" },
      ]);
    } else if (actionMatch) {
      setBlockValue(blockId, `fn: ${actionMatch[1].trim()}(event)`);
      // Keep existing ports for action (input depends on wiring events)
    }
    // store blocks keep their original ports
  } catch {
    // Best-effort sync
  }
}

function tryParseValue(raw: string): unknown {
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (raw === "null") return null;
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  if (/^["'].*["']$/.test(raw)) return raw.slice(1, -1);
  return raw;
}
