/** 属性面板。根据选中元素和统一元数据动态渲染编辑器。
 *  编辑自定义组件定义时显示 ComponentMetaEditor。*/

import React, { useState, useCallback, useRef, useEffect } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { useCodeGraphStore } from "../codegraph/store";
import type { EventBinding, ParamBinding, FnParam } from "../codegraph/types";
import { SetProp } from "../commands/SetProp";
import { compilerCache } from "../omni-compiler-cache";
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
// Helpers
// ═══════════════════════════════════════════════

/** Walk up the element tree to find which component owns an element.
 *  Returns 'App' for top-level elements that are not inside a custom component. */
function findOwningComponent(elementId: string, elements: ElementData[]): string {
  let current = elements.find((e) => e.id === elementId);
  if (!current) return 'App';

  // Walk up to root
  while (current?.parentId) {
    const parent = elements.find((e) => e.id === current!.parentId);
    if (!parent) break;
    current = parent;
  }

  return current?.componentRef || 'App';
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

  // ── Debounced recompile on property change ──
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debouncedRecompile = useCallback(() => {
    if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    debounceTimerRef.current = setTimeout(() => {
      const store = useEditorStore.getState();
      const cg = useCodeGraphStore.getState();
      const codeTabs = cg.toSerializable().tabs;

      // Determine which component owns the currently selected element
      const compName = selectedIds[0]
        ? findOwningComponent(selectedIds[0], store.elements)
        : 'App';

      const output = compilerCache.compileProject({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName: compName,
        rootOrder: store.rootOrder,
      }, true);

      if (output?.srcdoc) {
        // Dispatch custom event so LayoutView can update the iframe
        window.dispatchEvent(
          new CustomEvent('omni-srcdoc-update', { detail: { srcdoc: output.srcdoc } }),
        );
      }
    }, 300);
  }, [selectedIds]);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    };
  }, []);

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
    debouncedRecompile();
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

      {/* Events — new binding UI */}
      {Object.keys(meta.events).length > 0 && (
        <>
          <div style={sectionHeaderStyle}>事件</div>
          {Object.entries(meta.events).map(([name, config]) => (
            <div key={name} style={{ marginBottom: 4 }}>
              <EventBindingRow
                eventName={name}
                eventConfig={config}
                selectedEl={el}
                onBind={(binding) => {
                  undoExec(new SetProp(el.id, name, binding), ctx);
                }}
              />
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
// Event binding row — NEW (replaces old "navigate to code" button)
// ═══════════════════════════════════════════════

function EventBindingRow({ eventName, eventConfig, selectedEl, onBind }: {
  eventName: string;
  eventConfig: EventMeta;
  selectedEl: import("../store/editor").ElementData;
  onBind: (binding: EventBinding | null) => void;
}) {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const existingBinding = (selectedEl.props[eventName] as EventBinding | null) || null;

  // Get all custom functions from event-code tabs
  const eventFnOptions: Array<{ tabId: string; tabLabel: string; fnName: string; fnParams: FnParam[]; fnReturnType: string }> = [];
  for (const tab of tabs) {
    if (tab.type !== "event") continue;
    for (const el of tab.graph.elements) {
      if (el.tag === "customFn" && el.config?.fnName) {
        eventFnOptions.push({
          tabId: tab.id,
          tabLabel: tab.label,
          fnName: el.config.fnName as string,
          fnParams: (el.config.fnParams as FnParam[]) || [],
          fnReturnType: (el.config.fnReturnType as string) || "void",
        });
      }
    }
  }

  const [isExpanded, setIsExpanded] = useState(!!existingBinding);
  const [selectedFn, setSelectedFn] = useState(existingBinding?.fnName || "");
  const [selectedTab, setSelectedTab] = useState(existingBinding?.tabId || "");
  const [paramBindings, setParamBindings] = useState<Record<string, ParamBinding>>(existingBinding?.paramBindings || {});

  const currentFn = eventFnOptions.find((f) => f.fnName === selectedFn && f.tabId === selectedTab);

  const handleSelectFn = (fnName: string, tabId: string) => {
    setSelectedFn(fnName);
    setSelectedTab(tabId);
    // Auto-generate default param bindings from function signature
    const fn = eventFnOptions.find((f) => f.fnName === fnName && f.tabId === tabId);
    if (fn) {
      const bindings: Record<string, ParamBinding> = {};
      for (const param of fn.fnParams) {
        // Default: try to auto-match with event payload if types match
        if (param.type === (eventConfig.payload || "any") && Object.keys(bindings).length === 0) {
          bindings[param.name] = { sourceType: "event", value: `event.${param.name}`, expectedType: param.type };
        } else {
          bindings[param.name] = { sourceType: "literal", value: "", expectedType: param.type };
        }
      }
      setParamBindings(bindings);
    }
  };

  const handleSaveBinding = () => {
    if (!selectedFn || !selectedTab) return;
    const binding: EventBinding = {
      eventType: eventName,
      tabId: selectedTab,
      fnName: selectedFn,
      paramBindings,
    };
    onBind(binding);
    setIsExpanded(true);
  };

  const handleRemoveBinding = () => {
    setSelectedFn("");
    setSelectedTab("");
    setParamBindings({});
    onBind(null);
    setIsExpanded(false);
  };

  // Type mismatch check
  const getTypeError = (param: FnParam, binding: ParamBinding): string | null => {
    if (binding.sourceType === "event" && param.type !== (eventConfig.payload || "any") && eventConfig.payload) {
      if (param.type !== "any" && eventConfig.payload !== "any") {
        return `类型不匹配: 参数类型 ${param.type} ≠ 事件数据类型 ${eventConfig.payload}`;
      }
    }
    return null;
  };

  return (
    <div style={{
      border: "1px solid #E5E7EB", borderRadius: 6, padding: 6,
      background: existingBinding ? "#F0FDF4" : "#FAFAFA",
      marginBottom: 4,
    }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <span style={{ fontSize: 11, fontWeight: 600, color: "#374151" }}>
          {eventName}
          {eventConfig.description && (
            <span style={{ fontWeight: 400, color: "#9CA3AF", marginLeft: 4 }}>
              ({eventConfig.description}{eventConfig.payload ? ` → ${eventConfig.payload}` : ""})
            </span>
          )}
        </span>
        {existingBinding ? (
          <button onClick={handleRemoveBinding} style={removeBtnStyle}>
            ✕
          </button>
        ) : (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            style={{ ...addBtnStyle, marginTop: 0 }}
          >
            {isExpanded ? "取消" : "+ 绑定函数"}
          </button>
        )}
      </div>

      {existingBinding && (
        <div style={{ fontSize: 10, color: "#059669", marginTop: 2 }}>
          → {existingBinding.fnName}({Object.values(existingBinding.paramBindings).map((b) => {
            if (b.sourceType === "event") return `📩 ${b.value.split(".")[1] || b.value}`;
            if (b.sourceType === "self") return `🔤 ${b.value}`;
            if (b.sourceType === "literal") return `🔢 ${b.value || "..."}`;
            if (b.sourceType === "global") return `🌐 ${b.value}`;
            return b.value;
          }).join(", ")})
        </div>
      )}

      {isExpanded && !existingBinding && (
        <div style={{ marginTop: 6 }}>
          {/* Function selector */}
          <div style={{ fontSize: 10, color: "#6B7280", marginBottom: 2 }}>选择函数</div>
          {eventFnOptions.length === 0 ? (
            <div style={{ fontSize: 10, color: "#D97706", padding: "4px 0" }}>
              ⚠ 没有可用的事件函数。请在代码模式中创建"事件代码"标签页，并添加自定义函数节点。
            </div>
          ) : (
            <select
              value={selectedFn ? `${selectedTab}:${selectedFn}` : ""}
              onChange={(e) => {
                const [tabId, fnName] = e.target.value.split(":");
                handleSelectFn(fnName, tabId);
              }}
              style={{ ...inputStyle, fontSize: 10, marginBottom: 6 }}
            >
              <option value="">-- 选择自定义函数 --</option>
              {eventFnOptions.map((f) => (
                <option key={`${f.tabId}:${f.fnName}`} value={`${f.tabId}:${f.fnName}`}>
                  [{f.tabLabel}] {f.fnName}
                </option>
              ))}
            </select>
          )}

          {/* Parameter binding table */}
          {currentFn && currentFn.fnParams.length > 0 && (
            <>
              <div style={{ fontSize: 10, color: "#6B7280", marginBottom: 2, marginTop: 4 }}>
                参数绑定
              </div>
              {currentFn.fnParams.map((param) => {
                const binding = paramBindings[param.name];
                const typeError = binding ? getTypeError(param, binding) : null;
                return (
                  <div key={param.name} style={{ marginBottom: 4 }}>
                    <div style={{ fontSize: 10, color: "#374151", marginBottom: 1 }}>
                      {param.name}: <span style={{ color: "#6B7280" }}>{param.type}</span>
                    </div>
                    <div style={{ display: "flex", gap: 4 }}>
                      <select
                        value={binding?.sourceType || "literal"}
                        onChange={(e) => {
                          const sourceType = e.target.value as ParamBinding["sourceType"];
                          setParamBindings({
                            ...paramBindings,
                            [param.name]: { sourceType, value: "", expectedType: param.type },
                          });
                        }}
                        style={{ ...inputStyle, fontSize: 10, width: 70 }}
                      >
                        <option value="event">📩 事件</option>
                        <option value="self">🔤 self</option>
                        <option value="literal">🔢 固定值</option>
                        <option value="global">🌐 全局</option>
                      </select>
                      <input
                        value={binding?.value || ""}
                        onChange={(e) => {
                          setParamBindings({
                            ...paramBindings,
                            [param.name]: { ...binding, sourceType: binding?.sourceType || "literal", value: e.target.value, expectedType: param.type },
                          });
                        }}
                        placeholder={
                          binding?.sourceType === "self" ? "self.signalName" :
                          binding?.sourceType === "event" ? "event.value" :
                          binding?.sourceType === "global" ? "app.xxx" :
                          "固定值"
                        }
                        style={{ ...inputStyle, fontSize: 10, flex: 1 }}
                      />
                    </div>
                    {typeError && (
                      <div style={{ fontSize: 9, color: "#DC2626", marginTop: 1 }}>🚫 {typeError}</div>
                    )}
                  </div>
                );
              })}
            </>
          )}

          {/* Save button */}
          {selectedFn && (
            <button
              onClick={handleSaveBinding}
              disabled={!selectedFn}
              style={{
                width: "100%", padding: "3px 8px", marginTop: 6,
                fontSize: 10, fontWeight: 600,
                background: "#059669", color: "#fff",
                border: "none", borderRadius: 4, cursor: "pointer",
              }}
            >
              保存事件绑定
            </button>
          )}
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const labelStyle: React.CSSProperties = { display: "flex", flexDirection: "column", marginTop: 6 };
const labelTextStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", marginBottom: 2 };
const inputStyle: React.CSSProperties = { padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4, fontSize: 12, width: "100%", boxSizing: "border-box" };
const sectionHeaderStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" };
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
