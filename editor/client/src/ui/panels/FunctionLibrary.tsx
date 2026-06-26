/** Function library panel — manages external code blocks (fn definitions).
 *  Design §1.10: fn definitions live here, not on canvas.
 *  Double-click → open CodeEditorPopup. Drag to canvas → creates 📞 call node. */

import { useState } from "react";
import { useEditorStore } from "../../store/editor";
import type { ElementData } from "../../core/types";
import type { CommandContext } from "../../core/commands/types";
import { MonacoEditor } from "../editor/MonacoEditor";
import { uid } from "../../core/utils";

// ── Types ──────────────────────────────────────

export interface FnEntry {
  id: string;
  name: string;
  params: string;
  returnType: string;
  source: string;
}

// ── Props ──────────────────────────────────────

interface FunctionLibraryProps {
  ctx: CommandContext;
  onFnDoubleClick?: (fnId: string, fnData: FnEntry) => void;
}

// ── Component ──────────────────────────────────

export function FunctionLibrary({ ctx, onFnDoubleClick }: FunctionLibraryProps) {
  const elements = useEditorStore((s) => s.elements);
  const [showCreate, setShowCreate] = useState(false);
  const [editingFn, setEditingFn] = useState<FnEntry | null>(null);

  // ── Derive fn entries from elements tagged "fn" ──
  const fnEntries: FnEntry[] = [];
  for (const el of elements) {
    if (el.nodeType === "codeblock" && el.tag === "fn") {
      const source = el.source || "";
      const parsed = parseFnSource(source);
      fnEntries.push({
        id: el.id,
        name: (el.props.name as string) || parsed.name || "未命名",
        params: parsed.params,
        returnType: parsed.returnType,
        source,
      });
    }
  }

  // ── Drag to canvas → create 📞 call node ──────

  const handleDragStart = (e: React.DragEvent, entry: FnEntry) => {
    e.dataTransfer.setData("application/omni-fn", JSON.stringify({
      fnId: entry.id,
      fnName: entry.name,
      params: entry.params,
      returnType: entry.returnType,
    }));
    e.dataTransfer.effectAllowed = "copy";
  };

  // ── Delete fn ─────────────────────────────────

  const handleDelete = (entry: FnEntry) => {
    const store = useEditorStore.getState();
    const callers = store.elements.filter(e =>
      e.nodeType === "codeblock" && e.tag === "fn-call" && e.props.fnId === entry.id);
    if (callers.length > 0) {
      alert(`"${entry.name}" 被 ${callers.length} 个调用节点引用，删除后这些节点将失效。`);
    }
    store.removeElement(entry.id);
  };

  const handleDoubleClick = (entry: FnEntry) => {
    if (onFnDoubleClick) {
      onFnDoubleClick(entry.id, entry);
    } else {
      // Fallback: open inline editor
      setEditingFn(entry);
    }
  };

  // ── Save edited fn ────────────────────────────

  const handleSaveEdit = () => {
    if (!editingFn) return;
    const store = useEditorStore.getState();
    store.setElementSource(editingFn.id, editingFn.source);
    // Update name from parsed source
    const parsed = parseFnSource(editingFn.source);
    if (parsed.name) {
      store.setProp(editingFn.id, "name", parsed.name);
    }
    setEditingFn(null);
  };

  return (
    <div style={{ padding: "8px 0", fontSize: 12, userSelect: "none" }}>
      {/* Header */}
      <div style={{ padding: "0 8px 6px", fontSize: 10, fontWeight: 600, color: "#9CA3AF", textTransform: "uppercase" }}>
        📦 函数库
      </div>

      {/* Fn list */}
      {fnEntries.length === 0 && (
        <div style={{ padding: "6px 8px", fontSize: 10, color: "#D1D5DB", fontStyle: "italic" }}>
          暂无函数，点击下方按钮创建
        </div>
      )}
      {fnEntries.map(entry => (
        <div
          key={entry.id}
          draggable
          onDragStart={(e) => handleDragStart(e, entry)}
          onDoubleClick={() => handleDoubleClick(entry)}
          title={`双击编辑源码\n拖入画布创建调用节点\n函数 ${entry.name}(${entry.params}) → ${entry.returnType}`}
          style={{
            padding: "5px 8px", cursor: "grab", display: "flex", alignItems: "center", gap: 6,
            fontSize: 11, borderRadius: 0, margin: "0 4px",
            transition: "background 0.1s",
          }}
          onMouseEnter={e => { e.currentTarget.style.background = "#F3F4F6"; }}
          onMouseLeave={e => { e.currentTarget.style.background = "transparent"; }}
        >
          <span style={{ fontSize: 13 }}>📦</span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontWeight: 500, color: "#374151" }}>
              {entry.name}
            </div>
            <div style={{ fontSize: 9, color: "#9CA3AF" }}>
              ({entry.params}) → {entry.returnType}
            </div>
          </div>
          <button
            onClick={(e) => { e.stopPropagation(); handleDelete(entry); }}
            title="删除函数"
            style={{
              background: "transparent", border: "none", cursor: "pointer",
              color: "#D1D5DB", fontSize: 10, padding: "2px 4px",
            }}
            onMouseEnter={e => (e.currentTarget.style.color = "#EF4444")}
            onMouseLeave={e => (e.currentTarget.style.color = "#D1D5DB")}
          >
            ✕
          </button>
        </div>
      ))}

      {/* Create button */}
      <div style={{ padding: "4px 8px" }}>
        <button
          onClick={() => setShowCreate(true)}
          style={{
            padding: "2px 8px", fontSize: 10, fontWeight: 600,
            background: "#F3F4F6", border: "1px solid #E5E7EB",
            borderRadius: 4, cursor: "pointer", color: "#6B7280",
          }}
        >
          ➕ 添加函数
        </button>
      </div>

      {/* Create modal — uses Monaco editor */}
      {showCreate && (
        <CreateFnModal
          ctx={ctx}
          onClose={() => setShowCreate(false)}
        />
      )}

      {/* Inline edit modal — uses Monaco editor */}
      {editingFn && (
        <EditFnModal
          entry={editingFn}
          onChange={(source) => setEditingFn(prev => prev ? { ...prev, source } : null)}
          onSave={handleSaveEdit}
          onClose={() => setEditingFn(null)}
        />
      )}
    </div>
  );
}

// ── Create Fn Modal (Monaco editor) ─────────────

function CreateFnModal({ ctx, onClose }: {
  ctx: CommandContext;
  onClose: () => void;
}) {
  const [fnName, setFnName] = useState("newFn");
  const [params, setParams] = useState("event");
  const [returnType, setReturnType] = useState("void");
  const [body, setBody] = useState("  // 在此编写函数逻辑\n  return;");

  const source = `// 外部函数：可被画布代码块调用
fn ${fnName}(${params}) → ${returnType} {
${body}
}`;

  const handleCreate = () => {
    const id = uid("cb-fn");
    const el: ElementData = {
      id, nodeType: "codeblock", tag: "fn",
      props: { name: fnName },
      children: [], parentId: null,
      ports: [],
      isFolded: false,
      source,
    };
    ctx.addElement(el);
    onClose();
  };

  return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      background: "rgba(0,0,0,0.4)", zIndex: 1000,
      display: "flex", alignItems: "center", justifyContent: "center",
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        background: "#fff", borderRadius: 12, boxShadow: "0 8px 40px rgba(0,0,0,0.2)",
        width: 700, maxWidth: "95vw", height: "85vh", padding: 0,
        display: "flex", flexDirection: "column",
      }}>
        {/* Header */}
        <div style={{
          padding: "12px 16px", borderBottom: "1px solid #F3F4F6",
          display: "flex", alignItems: "center", gap: 8, flexShrink: 0,
        }}>
          <span style={{ fontSize: 18 }}>📦</span>
          <span style={{ fontSize: 14, fontWeight: 600, color: "#1F2937" }}>新建函数</span>
          <button onClick={onClose} style={{
            marginLeft: "auto", background: "transparent", border: "none",
            cursor: "pointer", color: "#9CA3AF", fontSize: 16,
          }}>✕</button>
        </div>

        {/* Header fields */}
        <div style={{ padding: "10px 16px", borderBottom: "1px solid #F3F4F6", flexShrink: 0,
          display: "flex", gap: 8, alignItems: "flex-end" }}>
          <div style={{ flex: 1 }}>
            <div style={labelStyle}>函数名</div>
            <input value={fnName} onChange={e => setFnName(e.target.value)}
              style={inputStyle} autoFocus />
          </div>
          <div style={{ width: 120 }}>
            <div style={labelStyle}>返回类型</div>
            <select value={returnType} onChange={e => setReturnType(e.target.value)}
              style={{ ...inputStyle, width: "100%" }}>
              {["void","i32","f64","bool","String","any"].map(t => <option key={t}>{t}</option>)}
            </select>
          </div>
        </div>
        <div style={{ padding: "6px 16px", borderBottom: "1px solid #F3F4F6", flexShrink: 0 }}>
          <div style={labelStyle}>参数</div>
          <input value={params} onChange={e => setParams(e.target.value)}
            style={inputStyle} placeholder="参数1: 类型, 参数2: 类型" />
        </div>

        {/* Monaco editor body */}
        <div style={{ flex: 1, minHeight: 200 }}>
          <MonacoEditor value={source} onChange={v => {
            // Parse the value back to extract body
            const match = v.match(/fn\s+\w+\s*\([^)]*\)\s*→\s*\w+\s*\{([^]*)\}/);
            if (match) {
              setBody(match[1]);
            } else {
              setBody(v);
            }
          }} />
        </div>

        {/* Footer */}
        <div style={{
          padding: "10px 16px", borderTop: "1px solid #F3F4F6", flexShrink: 0,
          display: "flex", gap: 8, justifyContent: "flex-end",
        }}>
          <button onClick={onClose} style={secondaryBtnStyle}>取消</button>
          <button onClick={handleCreate} style={primaryBtnStyle}>创建</button>
        </div>
      </div>
    </div>
  );
}

// ── Edit Fn Modal (Monaco editor, inline fallback) ──

function EditFnModal({ entry, onChange, onSave, onClose }: {
  entry: FnEntry;
  onChange: (source: string) => void;
  onSave: () => void;
  onClose: () => void;
}) {
  return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      background: "rgba(0,0,0,0.4)", zIndex: 1000,
      display: "flex", alignItems: "center", justifyContent: "center",
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        background: "#fff", borderRadius: 12, boxShadow: "0 8px 40px rgba(0,0,0,0.2)",
        width: 700, maxWidth: "95vw", height: "80vh", padding: 0,
        display: "flex", flexDirection: "column",
      }}>
        {/* Header */}
        <div style={{
          padding: "10px 16px", borderBottom: "1px solid #E5E7EB",
          display: "flex", alignItems: "center", gap: 8, flexShrink: 0,
          background: "#F9FAFB",
        }}>
          <span style={{ fontSize: 16 }}>📦</span>
          <span style={{ fontSize: 13, fontWeight: 600, color: "#1F2937" }}>{entry.name}</span>
          <span style={{ fontSize: 10, color: "#9CA3AF" }}>
            ({entry.params}) → {entry.returnType}
          </span>
          <span style={{ flex: 1 }} />
          <button onClick={onSave} style={{
            padding: "4px 14px", fontSize: 11, fontWeight: 600,
            background: "#4F46E5", color: "#fff", border: "none", borderRadius: 6, cursor: "pointer",
          }}>
            保存 (Ctrl+S)
          </button>
          <button onClick={onClose} style={{
            background: "transparent", border: "none", cursor: "pointer",
            color: "#9CA3AF", fontSize: 16, padding: "2px 6px",
          }}>✕</button>
        </div>
        {/* Monaco editor */}
        <div style={{ flex: 1, minHeight: 0 }}>
          <MonacoEditor value={entry.source} onChange={onChange} />
        </div>
      </div>
    </div>
  );
}

// ── Helpers ─────────────────────────────────────

export function parseFnSource(source: string): { name: string; params: string; returnType: string; body: string } {
  const match = source.match(/fn\s+(\w+)\s*\(([^)]*)\)\s*→\s*(\w+)\s*\{([^]*)\}/);
  if (match) {
    return {
      name: match[1].trim(),
      params: match[2].trim(),
      returnType: match[3].trim(),
      body: match[4].trim(),
    };
  }
  return { name: "", params: "", returnType: "void", body: "" };
}

export function parseFnSignature(source: string): { name: string; params: Array<{ name: string; type: string }>; returnType: string } {
  const match = source.match(/fn\s+(\w+)\s*\(([^)]*)\)\s*→\s*(\w+)/);
  if (!match) return { name: "", params: [], returnType: "void" };
  const params: Array<{ name: string; type: string }> = [];
  if (match[2].trim()) {
    for (const part of match[2].split(",")) {
      const [pName, pType] = part.split(":").map(s => s.trim());
      if (pName && pType) params.push({ name: pName, type: pType });
    }
  }
  return { name: match[1].trim(), params, returnType: match[3].trim() };
}

// ── Styles ──────────────────────────────────────

const inputStyle: React.CSSProperties = {
  padding: "5px 10px", fontSize: 12, border: "1px solid #D1D5DB",
  borderRadius: 6, outline: "none", width: "100%", boxSizing: "border-box",
};

const labelStyle: React.CSSProperties = {
  fontSize: 10, fontWeight: 600, color: "#6B7280", marginBottom: 3,
};

const primaryBtnStyle: React.CSSProperties = {
  padding: "6px 20px", fontSize: 12, fontWeight: 600,
  background: "#4F46E5", color: "#fff", border: "none", borderRadius: 6, cursor: "pointer",
};

const secondaryBtnStyle: React.CSSProperties = {
  padding: "6px 16px", fontSize: 12,
  background: "#fff", border: "1px solid #D1D5DB", borderRadius: 6, cursor: "pointer", color: "#6B7280",
};
