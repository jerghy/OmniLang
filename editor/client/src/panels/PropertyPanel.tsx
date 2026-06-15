/** 属性面板。根据选中元素和统一元数据动态渲染编辑器。
 *  编辑自定义组件定义时显示 ComponentMetaEditor。*/

import React from "react";
import { useEditorStore } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { SetProp } from "../commands/SetProp";
import { COMPONENT_PROTOCOL } from "./ComponentPalette";
import {
  getUnifiedComponentMeta,
  useComponentRegistry,
  type PropSchema,
  type EventSchema,
} from "../store/componentRegistry";

// ═══════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════

interface PropMeta {
  type: string;
  default?: unknown;
  values?: string[];
  editor: string;
}

interface EventMeta {
  id?: string;
  description: string;
  payload: string | null;
}

interface ComponentMeta {
  component: string;
  category: string;
  props: Record<string, PropMeta>;
  events: Record<string, EventMeta>;
  styleProperties: string[];
  children: boolean;
  isCustom?: boolean;
}

// ═══════════════════════════════════════════════
// Component Meta Editor (when editing a definition)
// ═══════════════════════════════════════════════

function ComponentMetaEditor({ componentName }: { componentName: string }) {
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
        {def.styleProperties.map((sp) => (
          <span key={sp} style={{
            fontSize: 10, padding: "1px 6px", borderRadius: 3,
            background: "#EDE9FE", color: "#6D28D9",
            display: "flex", alignItems: "center", gap: 3,
          }}>
            {sp}
            <button
              onClick={() => updateMeta(componentName, {
                styleProperties: def.styleProperties.filter((s) => s !== sp),
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

// ═══════════════════════════════════════════════
// Main Component
// ═══════════════════════════════════════════════

import type { CommandContext } from "../commands/types";

interface PropertyPanelProps {
  ctx: CommandContext;
  onNavigateToCode?: (wireId: string) => void;
}

export function PropertyPanel({ ctx, onNavigateToCode }: PropertyPanelProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const activeDocId = useEditorStore((s) => s.activeDocId);
  const undoExec = useUndoStore((s) => s.execute);

  const isEditingComponent = activeDocId?.startsWith("comp:") ?? false;
  const editingCompName = isEditingComponent ? activeDocId!.replace("comp:", "") : null;

  const raw = elements.find((e) => e.id === selectedIds[0]);

  // When editing a component definition and nothing is selected, show component meta editor
  if (!raw && isEditingComponent && editingCompName) {
    return (
      <div style={{ padding: 12, overflow: "auto", flex: 1, fontSize: 13 }}>
        <ComponentMetaEditor componentName={editingCompName} />
        <div style={{ padding: 16, color: "#9ca3af", textAlign: "center", marginTop: 16 }}>
          <div style={{ fontSize: 24, marginBottom: 8 }}>🔍</div>
          <div>选择元素以编辑属性</div>
          <div style={{ fontSize: 10, marginTop: 4 }}>或在画布上拖入组件设计内部结构</div>
        </div>
      </div>
    );
  }

  if (!raw) {
    return (
      <div style={{ padding: 16, color: "#9ca3af", textAlign: "center" }}>
        <div style={{ fontSize: 24, marginBottom: 8 }}>🔍</div>
        <div>选择画布元素以编辑属性</div>
      </div>
    );
  }

  const el = { ...raw, props: raw.props || {} };

  // Resolve metadata: custom component instances use their definition's metadata
  const unifiedMeta = getUnifiedComponentMeta();
  let meta: ComponentMeta | undefined = unifiedMeta[el.tag];

  // If this is a custom component instance, use the definition's metadata
  if (!meta && el.componentRef) {
    const registry = useComponentRegistry.getState();
    const def = registry.definitions[el.componentRef];
    if (def) {
      meta = {
        component: def.name,
        category: def.category,
        props: def.props,
        events: def.events,
        styleProperties: def.styleProperties,
        children: def.children,
        isCustom: true,
      };
    }
  }

  if (!meta) {
    return (
      <div style={{ padding: 16 }}>
        <strong>{`<${el.tag}>`}</strong>
        {el.componentRef && <div style={{ color: "#DC2626", fontSize: 11, marginTop: 4 }}>
          ⚠ 组件 "{el.componentRef}" 未找到
        </div>}
        <div style={{ color: "#9ca3af", fontSize: 12, marginTop: 4 }}>无元数据</div>
      </div>
    );
  }

  const handleChange = (prop: string, value: unknown) => {
    undoExec(new SetProp(el.id, prop, value), ctx);
  };

  return (
    <div style={{ padding: 12, overflow: "auto", flex: 1, fontSize: 13 }}>
      {/* Component meta editor when editing definition */}
      {isEditingComponent && editingCompName && (
        <ComponentMetaEditor componentName={editingCompName} />
      )}

      <h3 style={{ margin: "0 0 8px", fontSize: 14 }}>
        {meta.isCustom && <span style={{ color: "#4F46E5", marginRight: 4 }}>★</span>}
        {`<${el.tag}>`}
        {el.nodeType !== "view" && (
          <span style={{ fontSize: 10, color: "#6b7280", marginLeft: 8 }}>{el.nodeType}</span>
        )}
        {meta.isCustom && (
          <span style={{ fontSize: 10, color: "#4F46E5", marginLeft: 6, fontWeight: 400 }}>自定义</span>
        )}
      </h3>

      {/* Props */}
      {Object.keys(meta.props).length > 0 && (
        <>
          <div style={sectionHeaderStyle}>属性</div>
          {Object.entries(meta.props).map(([name, config]) => (
            <PropEditor
              key={name}
              name={name}
              config={config}
              value={el.props[name] ?? config.default}
              onChange={(v) => handleChange(name, v)}
            />
          ))}
        </>
      )}

      {/* Events */}
      {Object.keys(meta.events).length > 0 && (
        <>
          <div style={sectionHeaderStyle}>事件</div>
          {Object.entries(meta.events).map(([name, config]) => (
            <div key={name} style={{ marginBottom: 4 }}>
              <EventEditor
                name={name}
                config={config}
                value={el.props[name]}
                onChange={(v) => handleChange(name, v)}
              />
              {onNavigateToCode && (
                <button
                  onClick={() => {
                    const store = useEditorStore.getState();
                    const wireId = `wire-${el.id}-${name}-${Date.now()}`;
                    const handlerId = `handler-${el.id}-${name}`;
                    const handlerName = `on${name.replace(/^on/, "").charAt(0).toUpperCase()}${name.replace(/^on/, "").slice(1)}`;
                    const payloadType = config.payload || "any";
                    const eventPortId = config.id || name;

                    const newCodeEl = {
                      id: handlerId,
                      nodeType: "codeblock" as const,
                      tag: "action",
                      props: { name: handlerName, parentStore: "", typeAnnotation: "" },
                      children: [],
                      parentId: null,
                      ports: [
                        { name: "call", direction: "in" as const, type: payloadType, portType: "flow" as const, label: config.payload ? `${name}(${config.payload})` : name },
                        { name: "result", direction: "out" as const, type: "any", portType: "data" as const, label: "返回结果" },
                      ],
                      isFolded: false,
                      source: `action ${handlerName}(event) {\n    // ${config.description || name}\n}`,
                      nodeCategory: "action" as const,
                    };
                    const newWire = {
                      id: wireId,
                      fromId: el.id,
                      fromPort: eventPortId,
                      toId: handlerId,
                      toPort: "call",
                    };

                    // Add to code canvas (not design canvas) so FlowCanvas renders them
                    store.codeElements = [...store.codeElements, newCodeEl];
                    store.codeWires = [...store.codeWires, newWire];
                    useEditorStore.setState({ codeElements: store.codeElements, codeWires: store.codeWires, isDirty: true });

                    undoExec(new SetProp(el.id, name, handlerName), ctx);
                    onNavigateToCode(wireId);
                  }}
                  style={wireBtnStyle}
                  title={`设置 ${name} 事件 → 创建 Action 代码块${config.payload ? ` (接收 ${config.payload} 类型参数)` : ""}`}
                >
                  🔗 连线到代码块 ({name})
                </button>
              )}
            </div>
          ))}
        </>
      )}

      {/* Style Properties */}
      {meta.styleProperties.length > 0 && (
        <>
          <div style={sectionHeaderStyle}>样式</div>
          {meta.styleProperties.map((prop) => (
            <StyleEditor key={prop} name={prop} value={el.props[prop]} onChange={(v) => handleChange(prop, v)} />
          ))}
        </>
      )}

      {/* Instance info for custom components */}
      {el.componentRef && (
        <div style={{ marginTop: 8, padding: "4px 8px", background: "#F5F3FF", borderRadius: 4, fontSize: 10, color: "#6D28D9" }}>
          📦 实例化自: <strong>{el.componentRef}</strong>
        </div>
      )}

      {/* ID info */}
      <div style={{ marginTop: 16, paddingTop: 8, borderTop: "1px solid #e5e7eb", fontSize: 10, color: "#9ca3af" }}>
        ID: {el.id.slice(0, 8)}...
        {meta.isCustom && <span style={{ marginLeft: 8 }}>| 🏷 {meta.category}</span>}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════
// Sub Editors
// ═══════════════════════════════════════════════

function PropEditor({ name, config, value, onChange }: {
  name: string; config: PropMeta; value: unknown; onChange: (v: unknown) => void;
}) {
  switch (config.editor) {
    case "text":
      return (
        <label style={labelStyle}>
          <span style={labelTextStyle}>{name}</span>
          <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={inputStyle} />
        </label>
      );
    case "select":
      return (
        <label style={labelStyle}>
          <span style={labelTextStyle}>{name}</span>
          <select value={String(value ?? config.default)} onChange={(e) => onChange(e.target.value)} style={{ ...inputStyle, background: "#fff" }}>
            {(config.values || []).map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
      );
    case "checkbox":
      return (
        <label style={{ ...labelStyle, flexDirection: "row", alignItems: "center", gap: 8 }}>
          <input type="checkbox" checked={Boolean(value ?? false)} onChange={(e) => onChange(e.target.checked)} />
          <span style={{ fontSize: 12 }}>{name}</span>
        </label>
      );
    default:
      return null;
  }
}

function EventEditor({ name, config, value, onChange }: {
  name: string; config: EventMeta; value: unknown; onChange: (v: unknown) => void;
}) {
  return (
    <label style={labelStyle}>
      <span style={labelTextStyle}>
        {name}
        {config.description && <span style={{ color: "#9ca3af", fontSize: 10 }}> ({config.description})</span>}
      </span>
      <select
        value={String(value ?? "")}
        onChange={(e) => onChange(e.target.value || null)}
        style={{ ...inputStyle, background: "#fff" }}
      >
        <option value="">无</option>
        <option value="__new_fn__">＋ 新建函数...</option>
        <option value="__inline__">＋ 内联函数...</option>
      </select>
    </label>
  );
}

function StyleEditor({ name, value, onChange }: {
  name: string; value: unknown; onChange: (v: unknown) => void;
}) {
  if (name.includes("color")) {
    return (
      <label style={labelStyle}>
        <span style={labelTextStyle}>{name}</span>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input type="color" value={String(value ?? "#000000")} onChange={(e) => onChange(e.target.value)} style={{ width: 28, height: 24, border: "none", cursor: "pointer" }} />
          <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={{ ...inputStyle, flex: 1 }} />
        </div>
      </label>
    );
  }
  return (
    <label style={labelStyle}>
      <span style={labelTextStyle}>{name}</span>
      <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={inputStyle} placeholder="e.g. 16px" />
    </label>
  );
}

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const labelStyle: React.CSSProperties = { display: "flex", flexDirection: "column", marginTop: 6 };
const labelTextStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", marginBottom: 2 };
const inputStyle: React.CSSProperties = { padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4, fontSize: 12, width: "100%", boxSizing: "border-box" };
const sectionHeaderStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" };
const wireBtnStyle: React.CSSProperties = {
  width: "100%", padding: "3px 8px", marginTop: 2,
  fontSize: 10, fontWeight: 600,
  background: "#EEF2FF", color: "#4F46E5",
  border: "1px solid #C7D2FE", borderRadius: 4,
  cursor: "pointer", textAlign: "center",
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
