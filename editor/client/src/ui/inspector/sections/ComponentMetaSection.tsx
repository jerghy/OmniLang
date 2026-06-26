/** Component Meta Editor — shown when editing a component definition (no element selected). */

import React from "react";
import {
  useComponentRegistry,
} from "../../../store/componentRegistry";

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const sectionHeaderStyle: React.CSSProperties = {
  fontSize: 11, color: "#6b7280", textTransform: "uppercase",
  margin: "12px 0 4px",
};

const removeBtnStyle: React.CSSProperties = {
  border: "none", background: "transparent", cursor: "pointer",
  color: "#DC2626", fontSize: 10, padding: "0 2px",
};

const addBtnStyle: React.CSSProperties = {
  fontSize: 10, padding: "2px 8px", marginTop: 4,
  border: "1px dashed #C7D2FE", borderRadius: 4,
  background: "#fff", cursor: "pointer", color: "#4F46E5",
  fontWeight: 600,
};

const inputStyle: React.CSSProperties = {
  padding: "3px 6px", border: "1px solid #d1d5db",
  borderRadius: 4, fontSize: 12, width: "100%",
  boxSizing: "border-box",
};

// ═══════════════════════════════════════════════
// Component
// ═══════════════════════════════════════════════

export function ComponentMetaSection({ componentName }: { componentName: string }) {
  const def = useComponentRegistry((s) => s.definitions[componentName]);
  const updateMeta = useComponentRegistry((s) => s.updateMeta);

  if (!def) return null;

  return (
    <div style={{
      marginBottom: 12, padding: 8,
      background: "linear-gradient(135deg, #F5F3FF 0%, #EEF2FF 100%)",
      borderRadius: 6, border: "1px solid #DDD6FE",
    }}>
      <div style={{ fontSize: 12, fontWeight: 600, color: "#4F46E5", marginBottom: 8 }}>
        📐 组件定义
      </div>

      {/* Props schema */}
      <div style={sectionHeaderStyle}>属性 (Props)</div>
      {Object.entries(def.props).length === 0 && (
        <div style={{ fontSize: 10, color: "#9CA3AF", marginBottom: 4 }}>尚未定义属性</div>
      )}
      {Object.entries(def.props).map(([pName, schema]) => (
        <div key={pName} style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 2, fontSize: 11 }}>
          <span style={{ fontFamily: "monospace", flex: 1, color: "#374151" }}>
            {pName}: <span style={{ color: "#6B7280" }}>{schema.type}</span>
          </span>
          <button
            onClick={() => {
              const { [pName]: _, ...rest } = def.props;
              updateMeta(componentName, { props: rest });
            }}
            style={removeBtnStyle}
            title="删除属性"
          >
            ✕
          </button>
        </div>
      ))}
      <button
        onClick={() => {
          const pName = prompt("属性名称 (camelCase):");
          if (pName && pName.trim()) {
            updateMeta(componentName, {
              props: {
                ...def.props,
                [pName.trim()]: { type: "string", editor: "text", default: "" },
              },
            });
          }
        }}
        style={addBtnStyle}
      >
        + 添加属性
      </button>

      {/* Events schema */}
      <div style={{ ...sectionHeaderStyle, marginTop: 8 }}>事件 (Events)</div>
      {Object.entries(def.events).length === 0 && (
        <div style={{ fontSize: 10, color: "#9CA3AF", marginBottom: 4 }}>尚未定义事件</div>
      )}
      {Object.entries(def.events).map(([evtName, schema]) => (
        <div key={evtName} style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 2, fontSize: 11 }}>
          <span style={{ flex: 1, color: "#374151" }}>
            {evtName}
            {schema.payload && <span style={{ color: "#9CA3AF", fontSize: 10 }}> ({schema.payload})</span>}
          </span>
          <button
            onClick={() => {
              const { [evtName]: _, ...rest } = def.events;
              updateMeta(componentName, { events: rest });
            }}
            style={removeBtnStyle}
            title="删除事件"
          >
            ✕
          </button>
        </div>
      ))}
      <button
        onClick={() => {
          const evtName = prompt("事件名称 (如 onPress):");
          if (evtName && evtName.trim()) {
            const payload = prompt("事件数据类型 (如 String, 留空无数据):");
            updateMeta(componentName, {
              events: {
                ...def.events,
                [evtName.trim()]: {
                  id: `evt-custom-${componentName.toLowerCase()}-${evtName.trim().toLowerCase()}`,
                  description: "",
                  payload: payload?.trim() || null,
                },
              },
            });
          }
        }}
        style={addBtnStyle}
      >
        + 添加事件
      </button>

      {/* Style properties */}
      <div style={{ ...sectionHeaderStyle, marginTop: 8 }}>样式属性</div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 2 }}>
        {def.styleProperties.map((sp: string) => (
          <span key={sp} style={{
            fontSize: 10, padding: "1px 6px", borderRadius: 3,
            background: "#EDE9FE", color: "#6D28D9",
            display: "flex", alignItems: "center", gap: 3,
          }}>
            {sp}
            <button
              onClick={() => updateMeta(componentName, {
                styleProperties: def.styleProperties.filter((s: string) => s !== sp),
              })}
              style={{ border: "none", background: "transparent", cursor: "pointer", color: "#9CA3AF", fontSize: 9, padding: 0 }}
            >
              ✕
            </button>
          </span>
        ))}
      </div>
      <button
        onClick={() => {
          const sp = prompt("样式属性名 (如 border_radius):");
          if (sp && sp.trim() && !def.styleProperties.includes(sp.trim())) {
            updateMeta(componentName, {
              styleProperties: [...def.styleProperties, sp.trim()],
            });
          }
        }}
        style={addBtnStyle}
      >
        + 添加样式
      </button>

      {/* Children toggle */}
      <label style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 8, fontSize: 12, cursor: "pointer" }}>
        <input
          type="checkbox"
          checked={def.children}
          onChange={(e) => updateMeta(componentName, { children: e.target.checked })}
        />
        允许子元素
      </label>

      {/* Storage selector */}
      <div style={{ ...sectionHeaderStyle, marginTop: 8 }}>存储位置</div>
      <select
        value={def.storage}
        onChange={(e) => updateMeta(componentName, { storage: e.target.value as "config" | "file" })}
        style={{ ...inputStyle, width: "100%", marginTop: 2 }}
      >
        <option value="config">⚙️ 编辑器配置 (.omni-components.json)</option>
        <option value="file">📄 源文件 (.omni)</option>
      </select>
    </div>
  );
}
