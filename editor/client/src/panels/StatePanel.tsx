/** State management panel — lists signals, computeds, and stores.
 *  Design §2.2: dual-view architecture — create/manage in panel, drag to canvas to wire.
 *  Form-based creation (no code editor for state creation — §2.3). */

import { useState } from "react";
import { useEditorStore } from "../store/editor";
import type { ElementData } from "../core/types";
import { useUndoStore } from "../store/undo";
import { AddElement } from "../commands/AddElement";
import type { CommandContext } from "../commands/types";

// ── Types ──────────────────────────────────────

type StateKind = "signal" | "computed";
type CreateMode = { kind: StateKind } | { kind: "store" } | null;

interface StateEntry {
  id: string;
  kind: "signal" | "computed" | "store";
  name: string;
  typeAnnotation: string;
  outputValue?: unknown;
  /** For stores: child signal names */
  children?: string[];
  /** IDs of elements on canvas that reference this state */
  canvasNodeIds: string[];
}

// ── Props ──────────────────────────────────────

interface StatePanelProps {
  ctx: CommandContext;
}

// ── Component ──────────────────────────────────

export function StatePanel({ ctx }: StatePanelProps) {
  const elements = useEditorStore((s) => s.elements);
  const [createMode, setCreateMode] = useState<CreateMode>(null);
  const [search, setSearch] = useState("");

  // ── Derive state entries from elements ────────

  const stateEntries: StateEntry[] = [];
  const storeMap = new Map<string, ElementData[]>();

  for (const el of elements) {
    if (el.nodeType !== "codeblock") continue;
    if (el.tag === "store") {
      const childSignals = elements.filter(e => e.parentId === el.id);
      storeMap.set(el.id, childSignals);
      stateEntries.push({
        id: el.id,
        kind: "store",
        name: (el.props.name as string) || el.tag,
        typeAnnotation: "",
        children: childSignals.map(c => (c.props.name as string) || "?"),
        canvasNodeIds: elements.filter(e => e.nodeType === "codeblock" && e.parentId === el.id).map(e => e.id),
      });
    } else if (!el.parentId) {
      // Only top-level state elements (not children of stores)
      stateEntries.push({
        id: el.id,
        kind: (el.tag === "signal" || el.tag === "computed") ? (el.tag as "signal" | "computed") : "signal",
        name: (el.props.name as string) || el.tag,
        typeAnnotation: (el.props.typeAnnotation as string) || (el.tag === "computed" ? "any" : "i32"),
        outputValue: el.outputValue,
        canvasNodeIds: [],
      });
    }
  }

  // Filter by search
  const filtered = search
    ? stateEntries.filter(e =>
        e.name.toLowerCase().includes(search.toLowerCase()) ||
        e.typeAnnotation.toLowerCase().includes(search.toLowerCase()) ||
        e.kind.includes(search.toLowerCase()))
    : stateEntries;

  // ── Drag to canvas ────────────────────────────

  const handleDragStart = (e: React.DragEvent, entry: StateEntry) => {
    e.dataTransfer.setData("application/omni-state", JSON.stringify({
      id: entry.id, kind: entry.kind, name: entry.name,
      typeAnnotation: entry.typeAnnotation,
    }));
    e.dataTransfer.effectAllowed = "copy";
  };

  return (
    <div style={{ padding: "8px 0", fontSize: 12, userSelect: "none" }}>
      {/* Header */}
      <div style={{ padding: "0 8px 6px", fontSize: 10, fontWeight: 600, color: "#9CA3AF", textTransform: "uppercase" }}>
        ⚡ 状态管理
      </div>

      {/* Search */}
      <div style={{ padding: "0 8px 6px" }}>
        <input
          placeholder="🔍 搜索状态..."
          value={search}
          onChange={e => setSearch(e.target.value)}
          style={{
            width: "100%", padding: "3px 8px", fontSize: 11,
            border: "1px solid #E5E7EB", borderRadius: 4,
            outline: "none", boxSizing: "border-box",
          }}
        />
      </div>

      {/* State list */}
      <div>
        {/* Component State section */}
        <SectionLabel>组件状态</SectionLabel>
        {filtered.filter(s => s.kind !== "store").length === 0 && (
          <EmptyHint>暂无状态，点击下方按钮创建</EmptyHint>
        )}
        {filtered.filter(s => s.kind !== "store").map(entry => (
          <StateItem key={entry.id} entry={entry} onDragStart={handleDragStart} />
        ))}

        {/* Store section */}
        <SectionLabel>Store</SectionLabel>
        {filtered.filter(s => s.kind === "store").length === 0 && (
          <EmptyHint>暂无 Store</EmptyHint>
        )}
        {filtered.filter(s => s.kind === "store").map(entry => (
          <StateItem key={entry.id} entry={entry} onDragStart={handleDragStart} />
        ))}
      </div>

      {/* Add buttons */}
      <div style={{ padding: "4px 8px", display: "flex", gap: 4, flexWrap: "wrap" }}>
        <AddBtn onClick={() => setCreateMode({ kind: "signal" })} label="➕ Signal" />
        <AddBtn onClick={() => setCreateMode({ kind: "computed" })} label="➕ Computed" />
        <AddBtn onClick={() => setCreateMode({ kind: "store" })} label="➕ Store" />
      </div>

      {/* Creation modal */}
      {createMode && (
        <CreateStateModal
          mode={createMode}
          ctx={ctx}
          onClose={() => setCreateMode(null)}
        />
      )}
    </div>
  );
}

// ── Sub-components ──────────────────────────────

function SectionLabel({ children }: { children: string }) {
  return (
    <div style={{
      padding: "4px 8px 2px", fontSize: 9, fontWeight: 600,
      color: "#9CA3AF", textTransform: "uppercase",
      borderTop: "1px solid #F3F4F6", marginTop: 2,
    }}>
      {children}
    </div>
  );
}

function EmptyHint({ children }: { children: string }) {
  return (
    <div style={{ padding: "6px 8px", fontSize: 10, color: "#D1D5DB", fontStyle: "italic" }}>
      {children}
    </div>
  );
}

function AddBtn({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "2px 8px", fontSize: 10, fontWeight: 600,
        background: "#F3F4F6", border: "1px solid #E5E7EB",
        borderRadius: 4, cursor: "pointer", color: "#6B7280",
      }}
    >
      {label}
    </button>
  );
}

function StateItem({ entry, onDragStart }: {
  entry: StateEntry;
  onDragStart: (e: React.DragEvent, entry: StateEntry) => void;
}) {
  const icons: Record<string, string> = { signal: "⚡", computed: "🔄", store: "🗄️" };
  const icon = icons[entry.kind] || "📦";
  const valueStr = entry.outputValue !== undefined ? ` = ${JSON.stringify(entry.outputValue)}` : "";

  return (
    <div
      draggable
      onDragStart={(e) => onDragStart(e, entry)}
      title={`拖入画布创建节点\n类型: ${entry.typeAnnotation || "—"}\n${entry.kind}`}
      style={{
        padding: "4px 8px", cursor: "grab", display: "flex", alignItems: "center", gap: 6,
        fontSize: 11, borderRadius: 0, margin: "0 4px",
        transition: "background 0.1s",
      }}
      onMouseEnter={e => { e.currentTarget.style.background = "#F3F4F6"; }}
      onMouseLeave={e => { e.currentTarget.style.background = "transparent"; }}
    >
      <span style={{ fontSize: 13 }}>{icon}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <span style={{ fontWeight: 500, color: "#374151" }}>
          {entry.name}
        </span>
        {entry.typeAnnotation && (
          <span style={{ fontSize: 9, color: "#9CA3AF", marginLeft: 4 }}>
            : {entry.typeAnnotation}
          </span>
        )}
        <span style={{ fontSize: 9, color: "#9CA3AF", marginLeft: 4 }}>
          {valueStr}
        </span>
      </div>
      {entry.children && entry.children.length > 0 && (
        <span style={{ fontSize: 9, color: "#9CA3AF" }}>
          ({entry.children.length})
        </span>
      )}
    </div>
  );
}

// ── Creation Modal ──────────────────────────────

const TYPE_OPTIONS = ["i32", "f64", "bool", "String", "any"];

function CreateStateModal({ mode, ctx, onClose }: {
  mode: NonNullable<CreateMode>;
  ctx: CommandContext;
  onClose: () => void;
}) {
  const [name, setName] = useState("");
  const [dataType, setDataType] = useState("i32");
  const [initialValue, setInitialValue] = useState("0");
  const [storeSource, setStoreSource] = useState(`store NewStore {
    // 在此定义 signals 和 actions
}`);
  const [storeName, setStoreName] = useState("NewStore");

  const isStore = mode.kind === "store";

  const handleCreate = () => {
    if (isStore) {
      createStore(storeName, storeSource, ctx);
    } else {
      createState(mode.kind, name, dataType, initialValue, ctx);
    }
    onClose();
  };

  const kindLabel = isStore ? "Store" : mode.kind === "signal" ? "Signal" : "Computed";

  return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      background: "rgba(0,0,0,0.4)", zIndex: 1000,
      display: "flex", alignItems: "center", justifyContent: "center",
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        background: "#fff", borderRadius: 12, boxShadow: "0 8px 40px rgba(0,0,0,0.2)",
        width: 420, maxWidth: "90vw", padding: 0,
      }}>
        {/* Header */}
        <div style={{
          padding: "12px 16px", borderBottom: "1px solid #F3F4F6",
          display: "flex", alignItems: "center", gap: 8,
        }}>
          <span style={{ fontSize: 18 }}>
            {isStore ? "🗄️" : mode.kind === "signal" ? "⚡" : "🔄"}
          </span>
          <span style={{ fontSize: 14, fontWeight: 600, color: "#1F2937" }}>
            新建 {kindLabel}
          </span>
          <button onClick={onClose} style={{
            marginLeft: "auto", background: "transparent", border: "none",
            cursor: "pointer", color: "#9CA3AF", fontSize: 16,
          }}>✕</button>
        </div>

        {/* Body */}
        <div style={{ padding: "14px 16px" }}>
          {isStore ? (
            <>
              <FormField label="Store 名称">
                <input value={storeName} onChange={e => setStoreName(e.target.value)}
                  style={inputStyle} autoFocus />
              </FormField>
              <FormField label="初始内容">
                <textarea
                  value={storeSource}
                  onChange={e => setStoreSource(e.target.value)}
                  style={{
                    ...inputStyle, width: "100%", height: 180,
                    fontFamily: "monospace", fontSize: 11,
                    resize: "vertical",
                  }}
                />
              </FormField>
            </>
          ) : (
            <>
              <FormField label="名称">
                <input value={name} onChange={e => setName(e.target.value)}
                  style={inputStyle} autoFocus placeholder={`new${kindLabel}`} />
              </FormField>
              <FormField label="数据类型">
                <select value={dataType} onChange={e => setDataType(e.target.value)}
                  style={{ ...inputStyle, width: "100%" }}>
                  {TYPE_OPTIONS.map(t => <option key={t} value={t}>{t}</option>)}
                </select>
              </FormField>
              <FormField label="初始值">
                <input value={initialValue} onChange={e => setInitialValue(e.target.value)}
                  style={inputStyle} placeholder="0" />
              </FormField>
            </>
          )}
        </div>

        {/* Footer */}
        <div style={{
          padding: "10px 16px", borderTop: "1px solid #F3F4F6",
          display: "flex", gap: 8, justifyContent: "flex-end",
        }}>
          <button onClick={onClose} style={secondaryBtnStyle}>取消</button>
          <button onClick={handleCreate} style={primaryBtnStyle}>创建</button>
        </div>
      </div>
    </div>
  );
}

function FormField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 10 }}>
      <div style={{ fontSize: 11, fontWeight: 600, color: "#6B7280", marginBottom: 4 }}>
        {label}
      </div>
      {children}
    </div>
  );
}

// ── Creation helpers ────────────────────────────

function createState(
  kind: StateKind,
  name: string,
  dataType: string,
  initialValue: string,
  ctx: CommandContext,
) {
  const safeName = name || `new${kind === "signal" ? "Signal" : "Computed"}`;
  const id = `cb-${kind}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const val = tryParseValue(initialValue);

  let source: string;
  if (kind === "signal") {
    source = `signal ${safeName}: ${dataType} = ${initialValue};

// 值由事件更新
set(newValue) { value = newValue; }`;
  } else {
    source = `computed ${safeName}: ${dataType} {
    // 从 deps 端口获取输入值，计算后输出
    // const input = deps.value;
    // return input * 2;
}`;
  }

  const el: ElementData = {
    id, nodeType: "codeblock", tag: kind,
    props: { name: safeName, typeAnnotation: dataType },
    children: [], parentId: null,
    ports: kind === "signal"
      ? [
          { name: "value", direction: "out", type: dataType, label: `${safeName}:${dataType}` },
          { name: "set", direction: "in", type: dataType, label: `设置 ${safeName}` },
        ]
      : [
          { name: "value", direction: "out", type: dataType, label: `${safeName}:${dataType}` },
          { name: "deps", direction: "in", type: "any", label: "依赖输入" },
        ],
    isFolded: false,
    source,
  };

  // Use Zustand directly to avoid undo for initial creation
  ctx.addElement(el);
  if (val !== undefined) {
    useEditorStore.getState().setBlockValue(id, val);
  }
}

function createStore(name: string, source: string, ctx: CommandContext) {
  const safeName = name || "NewStore";
  const id = `cb-store-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

  const el: ElementData = {
    id, nodeType: "codeblock", tag: "store",
    props: { name: safeName },
    children: [], parentId: null,
    ports: [
      { name: "items", direction: "out", type: "any", label: "所有条目" },
    ],
    isFolded: false,
    source,
  };

  ctx.addElement(el);
}

function tryParseValue(raw: string): unknown {
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (raw === "null") return null;
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  if (/^["'].*["']$/.test(raw)) return raw.slice(1, -1);
  return raw;
}

// ── Styles ──────────────────────────────────────

const inputStyle: React.CSSProperties = {
  padding: "5px 10px", fontSize: 12, border: "1px solid #D1D5DB",
  borderRadius: 6, outline: "none", width: "100%", boxSizing: "border-box",
};

const primaryBtnStyle: React.CSSProperties = {
  padding: "6px 20px", fontSize: 12, fontWeight: 600,
  background: "#4F46E5", color: "#fff", border: "none", borderRadius: 6, cursor: "pointer",
};

const secondaryBtnStyle: React.CSSProperties = {
  padding: "6px 16px", fontSize: 12,
  background: "#fff", border: "1px solid #D1D5DB", borderRadius: 6, cursor: "pointer", color: "#6B7280",
};
