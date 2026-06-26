/** Unified event creation panel — triggered when wiring an event port.
 *  Merges: create code block / call function / bind handler / create state.
 *  Design §3.2: all options in one popup. */

import { useState } from "react";
import { useEditorStore } from "../store/editor";
import type { ElementData } from "../core/types";
import type { EventMeta } from "./ComponentPalette";

export interface EventSource {
  elementId: string;
  componentTag: string;
  eventName: string;
  eventMeta: EventMeta;
  /** Canvas position to anchor the panel near */
  x: number;
  y: number;
}

interface EventCreationPanelProps {
  source: EventSource;
  /** Existing code block IDs on canvas (for "bind existing" dropdown) */
  existingCodeBlocks: Array<{ id: string; name: string }>;
  /** Existing store actions (for "bind Store action" dropdown) */
  existingStoreActions: Array<{ storeId: string; storeName: string; actionName: string }>;
  onConfirm: (action: CreationAction) => void;
  onClose: () => void;
}

export interface CreationAction {
  type: "new-code-block" | "call-function" | "bind-code-block" | "bind-store-action";
  /** For new-code-block: name of the action to create */
  blockName?: string;
  /** For bind-code-block: ID of existing block */
  bindBlockId?: string;
  /** For call-function: function name */
  functionName?: string;
  /** For bind-store-action */
  storeId?: string;
  actionName?: string;
  /** Whether to also create a signal for the event payload */
  createSignal?: boolean;
  signalName?: string;
}

export function EventCreationPanel({ source, existingCodeBlocks, existingStoreActions, onConfirm, onClose }: EventCreationPanelProps) {
  const hasPayload = source.eventMeta.payload !== null;
  const payloadType = source.eventMeta.payload || "无";
  const isSimplePayload = hasPayload && ["String", "i32", "f64", "bool"].includes(payloadType);

  const [mode, setMode] = useState<CreationAction["type"]>("new-code-block");
  const [blockName, setBlockName] = useState(`handle${capitalize(source.eventName)}`);
  const [bindBlockId, setBindBlockId] = useState(existingCodeBlocks[0]?.id || "");
  const [createSignal, setCreateSignal] = useState(isSimplePayload);
  const [signalName, setSignalName] = useState(`${source.componentTag.toLowerCase()}Value`);

  const handleConfirm = () => {
    const action: CreationAction = { type: mode };
    if (mode === "new-code-block") {
      action.blockName = blockName;
      if (hasPayload && createSignal) {
        action.createSignal = true;
        action.signalName = signalName;
      }
    } else if (mode === "bind-code-block") {
      action.bindBlockId = bindBlockId;
    }
    onConfirm(action);
  };

  return (
    <div style={{
      position: "absolute", left: source.x, top: source.y, zIndex: 1000,
      background: "#fff", borderRadius: 10, boxShadow: "0 8px 32px rgba(0,0,0,0.18)", border: "1px solid #E5E7EB",
      minWidth: 320, maxWidth: 380, padding: 0, fontSize: 12,
    }}>
      {/* Header */}
      <div style={{
        padding: "10px 14px", borderBottom: "1px solid #F3F4F6",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14 }}>🔌</span>
        <div>
          <div style={{ fontWeight: 600, color: "#1F2937", fontSize: 13 }}>
            {source.componentTag}.{source.eventName}
          </div>
          <div style={{ fontSize: 10, color: "#9CA3AF" }}>
            事件数据: {payloadType}
          </div>
        </div>
        <button onClick={onClose} style={{
          marginLeft: "auto", background: "transparent", border: "none",
          cursor: "pointer", color: "#9CA3AF", fontSize: 14, padding: "2px 6px",
        }}>✕</button>
      </div>

      {/* Body */}
      <div style={{ padding: "8px 0" }}>
        {/* Section: event handling */}
        <div style={{ padding: "0 14px 4px", fontSize: 10, fontWeight: 600, color: "#9CA3AF", textTransform: "uppercase" }}>
          事件处理
        </div>

        <RadioOption
          selected={mode === "new-code-block"}
          onClick={() => setMode("new-code-block")}
          icon="💻" label="创建新代码块"
          desc={`自动生成 action ${blockName}(event: ${payloadType}) { }`}
        >
          {mode === "new-code-block" && (
            <div style={{ marginTop: 6 }}>
              <Label>名称</Label>
              <input value={blockName} onChange={e => setBlockName(e.target.value)}
                style={inputStyle} />
            </div>
          )}
        </RadioOption>

        <RadioOption
          selected={mode === "call-function"}
          onClick={() => setMode("call-function")}
          icon="📞" label="调用已有函数"
          desc="从函数库选择已有函数，传入事件参数"
          disabled
        />

        <RadioOption
          selected={mode === "bind-code-block"}
          onClick={() => setMode("bind-code-block")}
          icon="💻" label="绑定已有代码块"
          desc="将事件连线到画布上已有的代码块"
          disabled={existingCodeBlocks.length === 0}
        >
          {mode === "bind-code-block" && existingCodeBlocks.length > 0 && (
            <div style={{ marginTop: 6 }}>
              <Label>选择代码块</Label>
              <select value={bindBlockId} onChange={e => setBindBlockId(e.target.value)}
                style={{ ...inputStyle, width: "100%" }}>
                {existingCodeBlocks.map(b => (
                  <option key={b.id} value={b.id}>{b.name}</option>
                ))}
              </select>
            </div>
          )}
        </RadioOption>

        <RadioOption
          selected={mode === "bind-store-action"}
          onClick={() => setMode("bind-store-action")}
          icon="🗄️" label="绑定 Store 操作"
          desc="事件直接触发 Store 的操作"
          disabled={existingStoreActions.length === 0}
        />

        {/* Section: state creation (only when payload exists) */}
        {hasPayload && (
          <>
            <div style={{ marginTop: 8, borderTop: "1px solid #F3F4F6" }} />
            <div style={{ padding: "8px 14px 4px", fontSize: 10, fontWeight: 600, color: "#9CA3AF", textTransform: "uppercase" }}>
              可选：同时创建状态
            </div>
            <div style={{ padding: "0 14px" }}>
              <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", fontSize: 11, color: "#6B7280" }}>
                <input type="checkbox" checked={createSignal} onChange={e => setCreateSignal(e.target.checked)}
                  style={{ accentColor: "#4F46E5" }} />
                <span>同时创建状态存储 {source.eventName} 的值</span>
              </label>
              {createSignal && (
                <div style={{ margin: "6px 0 2px 22px" }}>
                  <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <Label>名称</Label>
                    <input value={signalName} onChange={e => setSignalName(e.target.value)}
                      style={{ ...inputStyle, width: 140 }} />
                    <span style={{ fontSize: 10, color: "#9CA3AF" }}>: {payloadType}</span>
                  </div>
                  <div style={{ fontSize: 9, color: "#9CA3AF", marginTop: 3, marginLeft: 2 }}>
                    连线: {source.componentTag}.{source.eventName} → 💻{blockName} → ⚡{signalName}.设置
                  </div>
                </div>
              )}
            </div>
          </>
        )}
      </div>

      {/* Footer */}
      <div style={{
        padding: "8px 14px", borderTop: "1px solid #F3F4F6",
        display: "flex", gap: 8, justifyContent: "flex-end",
      }}>
        <button onClick={onClose} style={secondaryBtnStyle}>取消</button>
        <button onClick={handleConfirm} style={primaryBtnStyle}>确认</button>
      </div>
    </div>
  );
}

// ── Sub-components ──────────────────────────────

function RadioOption({
  selected, onClick, icon, label, desc, disabled, children,
}: {
  selected: boolean; onClick: () => void; icon: string; label: string; desc: string;
  disabled?: boolean; children?: React.ReactNode;
}) {
  return (
    <div style={{ opacity: disabled ? 0.4 : 1 }}>
      <div
        onClick={disabled ? undefined : onClick}
        style={{
          padding: "7px 14px", cursor: disabled ? "not-allowed" : "pointer",
          display: "flex", alignItems: "flex-start", gap: 10,
          background: selected ? "#EEF2FF" : "transparent",
          borderLeft: selected ? "3px solid #4F46E5" : "3px solid transparent",
          transition: "background 0.1s",
        }}
      >
        <span style={{ fontSize: 17, flexShrink: 0, marginTop: 1 }}>{icon}</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 600, color: "#374151", fontSize: 12 }}>{label}</div>
          <div style={{ fontSize: 10, color: "#9CA3AF" }}>{desc}</div>
          {children}
        </div>
        <span style={{
          width: 16, height: 16, borderRadius: "50%", flexShrink: 0,
          border: selected ? "4px solid #4F46E5" : "2px solid #D1D5DB",
          background: "#fff", marginTop: 2,
        }} />
      </div>
    </div>
  );
}

function Label({ children }: { children: string }) {
  return <span style={{ fontSize: 10, color: "#6B7280", marginRight: 4 }}>{children}:</span>;
}

// ── Styles ──────────────────────────────────────

const inputStyle: React.CSSProperties = {
  padding: "3px 8px", fontSize: 11, border: "1px solid #D1D5DB",
  borderRadius: 4, outline: "none", width: 160,
};

const primaryBtnStyle: React.CSSProperties = {
  padding: "5px 18px", fontSize: 12, fontWeight: 600,
  background: "#4F46E5", color: "#fff", border: "none", borderRadius: 6, cursor: "pointer",
};

const secondaryBtnStyle: React.CSSProperties = {
  padding: "5px 14px", fontSize: 12,
  background: "#fff", border: "1px solid #D1D5DB", borderRadius: 6, cursor: "pointer", color: "#6B7280",
};

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
