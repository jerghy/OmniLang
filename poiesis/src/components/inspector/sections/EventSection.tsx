/** Event section — event bindings UI with EventBindingDialog. */

import React, { useState, useRef, useEffect } from "react";
import type { ElementData } from "../../../core/types";
import type { EventBinding, ParamBinding, FnParam } from "../../../services/codegraph/types";
import { useCodeGraphStore } from "../../../services/codegraph/store";
import { uid } from "../../../core/utils";
import { SetProp } from "../../../core/commands/SetProp";
import { useUndoStore } from "../../../store/undo";
import type { CommandContext } from "../../../core/commands/types";

// Types

interface EventMeta {
  id?: string;
  description: string;
  payload: string | null;
}

interface ComponentMeta {
  component: string;
  category: string;
  props: Record<string, unknown>;
  events: Record<string, EventMeta>;
  children: boolean;
  isCustom?: boolean;
}

interface EventSectionProps {
  element: ElementData;
  meta: ComponentMeta;
  allEvents: Record<string, EventMeta>;
  ctx: CommandContext;
  onRecompile: () => void;
  onNavigateToCode?: (wireId: string) => void;
}

export function EventSection({ element: el, meta, allEvents, ctx, onRecompile }: EventSectionProps) {
  const [showEventDialog, setShowEventDialog] = useState(false);
  const [editingEvent, setEditingEvent] = useState<string | null>(null);

  const handleChange = (prop: string, value: unknown) => {
    useUndoStore.getState().execute(new SetProp(el.id, prop, value), ctx);
    onRecompile();
  };

  return (
    <>
      {/* Events — collapsible list, only bound events shown */}
      <div style={sectionHeaderStyle}>事件 <span style={{ fontWeight: 400, color: "#9CA3AF", fontSize: 9 }}>（所有组件支持通用事件）</span></div>

      {/* Legend: show universal events hint */}
      {Object.entries(allEvents).filter(([name]) => el.props[name]).length === 0 && (
        <div style={{ fontSize: 10, color: "#9CA3AF", marginBottom: 4, padding: "0 4px" }}>
          所有组件支持通用事件（点击、输入、焦点、键盘等）
        </div>
      )}

      {/* Bound events as expandable items */}
      {Object.entries(allEvents).filter(([name]) => el.props[name]).map(([name]) => {
        const binding = el.props[name] as EventBinding;
        const isSpecial = !!meta.events[name];
        return (
          <BoundEventItem
            key={name}
            name={name}
            binding={binding}
            description={allEvents[name]?.description || ""}
            isSpecial={isSpecial}
            onRemove={() => handleChange(name, null)}
            onUpdate={(updatedBinding) => handleChange(name, updatedBinding)}
          />
        );
      })}

      {/* Add event button */}
      <button
        onClick={() => { setEditingEvent(null); setShowEventDialog(true); }}
        style={{
          width: "100%", padding: "5px 8px", fontSize: 11, fontWeight: 600,
          border: "1px dashed #C7D2FE", borderRadius: 5,
          background: "#EEF2FF", color: "#4F46E5", cursor: "pointer",
          marginTop: Object.entries(allEvents).some(([n]) => el.props[n]) ? 4 : 0,
        }}
      >
        + 添加事件
      </button>

      {/* Event binding dialog */}
      {showEventDialog && (
        <EventBindingDialog
          events={Object.entries(allEvents)}
          specialEventNames={new Set(Object.keys(meta.events))}
          initialEvent={editingEvent}
          selectedEl={el}
          onBind={(eventName, binding) => {
            handleChange(eventName, binding);
            setShowEventDialog(false);
            setEditingEvent(null);
          }}
          onClose={() => {
            setShowEventDialog(false);
            setEditingEvent(null);
          }}
        />
      )}
    </>
  );
}

// ═══════════════════════════════════════════════
// Event binding dialog (modal)
// ═══════════════════════════════════════════════

function EventBindingDialog({ events, specialEventNames, initialEvent, selectedEl, onBind, onClose }: {
  events: [string, EventMeta][];
  specialEventNames: Set<string>;
  initialEvent: string | null;
  selectedEl: import("../../../store/editor").ElementData;
  onBind: (eventName: string, binding: EventBinding | null) => void;
  onClose: () => void;
}) {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [selectedEventName, setSelectedEventName] = useState<string>(initialEvent || events[0]?.[0] || "");

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const [selectedFn, setSelectedFn] = useState("");
  const [selectedTab, setSelectedTab] = useState("");
  const [paramBindings, setParamBindings] = useState<Record<string, ParamBinding>>({});

  const eventConfig = events.find(([n]) => n === selectedEventName)?.[1];
  const existingBinding = selectedEl.props[selectedEventName] as EventBinding | null;

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

  const currentFn = eventFnOptions.find((f) => f.fnName === selectedFn && f.tabId === selectedTab);

  const handleSelectFn = (fnName: string, tabId: string) => {
    setSelectedFn(fnName);
    setSelectedTab(tabId);
    const fn = eventFnOptions.find((f) => f.fnName === fnName && f.tabId === tabId);
    if (fn) {
      const bindings: Record<string, ParamBinding> = {};
      for (const param of fn.fnParams) {
        if (param.type === (eventConfig?.payload || "any") && Object.keys(bindings).length === 0) {
          bindings[param.name] = { sourceType: "event", value: `event.${param.name}`, expectedType: param.type };
        } else {
          bindings[param.name] = { sourceType: "literal", value: "", expectedType: param.type };
        }
      }
      setParamBindings(bindings);
    }
  };

  const handleSave = () => {
    if (!selectedFn || !selectedTab) return;
    onBind(selectedEventName, {
      eventType: selectedEventName,
      tabId: selectedTab,
      fnName: selectedFn,
      paramBindings,
    });
  };

  const handleRemove = () => {
    onBind(selectedEventName, null);
  };

  const handleCreateFunction = () => {
    // Generate function name from event (e.g. onPress → onPress, onChange → onChange)
    const fnName = selectedEventName.charAt(0).toUpperCase() + selectedEventName.slice(1);
    // If event name starts with "on", remove it for the handler name: onPress → handlePress
    const handlerName = fnName.startsWith("On") ? `handle${fnName.slice(2)}` : `handle${fnName}`;

    // Build params from event payload
    const fnParams: FnParam[] = eventConfig?.payload
      ? [{ name: "event", type: eventConfig.payload }]
      : [];

    // Create a new event code tab
    const cgStore = useCodeGraphStore.getState();
    const newFnId = uid("fn");
    const returnNodeId = uid("ret");
    const tabLabel = `事件: ${selectedEventName}`;
    const tabId = cgStore.addTab("event", tabLabel, {
      icon: "⚡",
      initialGraph: {
        elements: [
          {
            id: newFnId,
            nodeType: "codeblock",
            tag: "customFn",
            props: {},
            children: [],
            parentId: null,
            config: {
              fnName: handlerName,
              fnParams,
              fnReturnType: "void",
              description: `Handle ${selectedEventName} event`,
            },
            position: { x: 100, y: 100 },
          },
          {
            id: returnNodeId,
            nodeType: "codeblock",
            tag: "return",
            props: {},
            children: [],
            parentId: null,
            config: {},
            position: { x: 400, y: 100 },
          },
        ],
        wires: [{
          id: uid("wire"),
          fromId: newFnId,
          fromPort: "return",
          toId: returnNodeId,
          toPort: "value",
        }],
      },
    });

    // Auto-generate param bindings from the function signature
    const newParamBindings: Record<string, ParamBinding> = {};
    for (const param of fnParams) {
      newParamBindings[param.name] = {
        sourceType: "event",
        value: `event.${param.name}`,
        expectedType: param.type,
      };
    }

    // Save the event binding
    onBind(selectedEventName, {
      eventType: selectedEventName,
      tabId,
      fnName: handlerName,
      paramBindings: newParamBindings,
    });

    // Switch to code mode and select the new function node
    window.dispatchEvent(new CustomEvent("omni-mode-change", {
      detail: { mode: "code" },
    }));

    // On next tick, select the newly created node in the code graph
    timerRef.current = setTimeout(() => {
      useCodeGraphStore.getState().selectElement(newFnId);
    }, 50);
  };

  const getTypeError = (param: FnParam, binding: ParamBinding): string | null => {
    if (binding.sourceType === "event" && param.type !== (eventConfig?.payload || "any") && eventConfig?.payload) {
      if (param.type !== "any" && eventConfig.payload !== "any") {
        return `类型不匹配: ${param.type} ≠ ${eventConfig.payload}`;
      }
    }
    return null;
  };

  return (
    <>
      {/* Overlay */}
      <div onClick={onClose} style={{
        position: "fixed", inset: 0, zIndex: 9998, background: "rgba(0,0,0,0.3)",
      }} />
      {/* Dialog */}
      <div style={{
        position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)",
        zIndex: 9999, background: "#fff", borderRadius: 12,
        boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
        width: 520, maxWidth: "90vw", maxHeight: "80vh",
        display: "flex", flexDirection: "column",
      }}>
        {/* Header */}
        <div style={{
          padding: "12px 16px", borderBottom: "1px solid #E5E7EB",
          display: "flex", justifyContent: "space-between", alignItems: "center",
        }}>
          <span style={{ fontSize: 14, fontWeight: 600 }}>~ 事件绑定</span>
          <button onClick={onClose} style={{
            border: "none", background: "transparent", cursor: "pointer",
            fontSize: 16, color: "#9CA3AF", padding: 0, lineHeight: 1,
          }}>✕</button>
        </div>

        {/* Body */}
        <div style={{ flex: 1, display: "flex", overflow: "hidden", minHeight: 240 }}>
          {/* Left: Event list with grouping */}
          <div style={{
            width: 200, flexShrink: 0, borderRight: "1px solid #E5E7EB",
            overflow: "auto", padding: 8,
          }}>
            {/* Special events section */}
            {events.filter(([name]) => specialEventNames.has(name)).length > 0 && (
              <>
                <div style={{ fontSize: 9, color: "#7C3AED", fontWeight: 700, marginBottom: 4, padding: "0 4px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
                  ⭐ 特殊事件
                </div>
                {events.filter(([name]) => specialEventNames.has(name)).map(([name, config]) => {
                  const hasBinding = !!selectedEl.props[name];
                  return (
                    <div
                      key={name}
                      onClick={() => {
                        setSelectedEventName(name);
                        setSelectedFn(existingBinding?.fnName || "");
                        setSelectedTab(existingBinding?.tabId || "");
                        setParamBindings(existingBinding?.paramBindings || {});
                      }}
                      style={{
                        padding: "5px 8px", borderRadius: 5, cursor: "pointer", fontSize: 11,
                        background: selectedEventName === name ? "#F5F3FF" : "transparent",
                        color: selectedEventName === name ? "#6D28D9" : "#374151",
                        fontWeight: selectedEventName === name ? 600 : 400,
                        marginBottom: 2, border: `1px solid ${selectedEventName === name ? "#DDD6FE" : "transparent"}`,
                        display: "flex", alignItems: "center", gap: 4,
                      }}
                    >
                      <span>{hasBinding ? "🔗" : "●"}</span>
                      <span style={{ flex: 1 }}>{name}</span>
                      {config.payload && (
                        <span style={{ fontSize: 9, color: "#9CA3AF" }}>{config.payload}</span>
                      )}
                    </div>
                  );
                })}
                <div style={{ height: 8 }} />
              </>
            )}

            {/* Universal events section */}
            <div style={{ fontSize: 9, color: "#6B7280", fontWeight: 700, marginBottom: 4, padding: "0 4px", textTransform: "uppercase", letterSpacing: "0.5px" }}>
              🌐 通用事件
            </div>
            {events.filter(([name]) => !specialEventNames.has(name)).map(([name, config]) => {
              const hasBinding = !!selectedEl.props[name];
              return (
                <div
                  key={name}
                  onClick={() => {
                    setSelectedEventName(name);
                    setSelectedFn(existingBinding?.fnName || "");
                    setSelectedTab(existingBinding?.tabId || "");
                    setParamBindings(existingBinding?.paramBindings || {});
                  }}
                  style={{
                    padding: "4px 8px", borderRadius: 4, cursor: "pointer", fontSize: 10,
                    background: selectedEventName === name ? "#EEF2FF" : "transparent",
                    color: selectedEventName === name ? "#4F46E5" : "#6B7280",
                    fontWeight: selectedEventName === name ? 600 : 400,
                    marginBottom: 1, border: `1px solid ${selectedEventName === name ? "#C7D2FE" : "transparent"}`,
                    display: "flex", alignItems: "center", gap: 4,
                  }}
                >
                  <span style={{ fontSize: 9 }}>{hasBinding ? "🔗" : "○"}</span>
                  <span style={{ flex: 1 }}>{name}</span>
                  {config.payload && (
                    <span style={{ fontSize: 8, color: "#9CA3AF" }}>{config.payload}</span>
                  )}
                </div>
              );
            })}
          </div>

          {/* Right: Function selector + param binding */}
          <div style={{ flex: 1, padding: 12, overflow: "auto", display: "flex", flexDirection: "column", gap: 12 }}>
            {/* Event info */}
            <div>
              <div style={{ fontSize: 10, color: "#9CA3AF", marginBottom: 2 }}>选中事件</div>
              <div style={{ fontSize: 13, fontWeight: 600, color: "#374151" }}>
                {selectedEventName}
                {eventConfig?.description && (
                  <span style={{ fontWeight: 400, color: "#9CA3AF", marginLeft: 6, fontSize: 11 }}>
                    ({eventConfig.description})
                  </span>
                )}
              </div>
            </div>

            {/* Function selector */}
            <div>
              <div style={{ fontSize: 10, color: "#6B7280", fontWeight: 600, marginBottom: 4 }}>
                选择函数
              </div>
              {eventFnOptions.length === 0 ? (
                <div style={{ fontSize: 11, color: "#D97706", padding: "6px 8px", background: "#FFFBEB", borderRadius: 4, border: "1px solid #FDE68A", marginBottom: 6 }}>
                  ⚠ 暂无可用函数
                </div>
              ) : (
                <select
                  value={selectedFn ? `${selectedTab}:${selectedFn}` : ""}
                  onChange={(e) => {
                    const [tabId, fnName] = e.target.value.split(":");
                    handleSelectFn(fnName, tabId);
                  }}
                  style={{
                    padding: "5px 8px", fontSize: 11, border: "1px solid #D1D5DB",
                    borderRadius: 5, width: "100%", boxSizing: "border-box",
                    background: "#fff", marginBottom: 6,
                  }}
                >
                  <option value="">-- 选择自定义函数 --</option>
                  {eventFnOptions.map((f) => (
                    <option key={`${f.tabId}:${f.fnName}`} value={`${f.tabId}:${f.fnName}`}>
                      [{f.tabLabel}] {f.fnName}({f.fnParams.map(p => `${p.name}: ${p.type}`).join(", ")}) → {f.fnReturnType}
                    </option>
                  ))}
                </select>
              )}
              <button
                onClick={handleCreateFunction}
                style={{
                  width: "100%", padding: "5px 8px", fontSize: 10, fontWeight: 600,
                  border: "1px dashed #7C3AED", borderRadius: 5,
                  background: "#F5F3FF", color: "#7C3AED", cursor: "pointer",
                }}
              >
                + 创建函数
              </button>
            </div>

            {/* Param binding */}
            {currentFn && (
              <div>
                <div style={{ fontSize: 10, color: "#6B7280", fontWeight: 600, marginBottom: 4 }}>
                  参数绑定
                </div>
                {currentFn.fnParams.length === 0 ? (
                  <div style={{ fontSize: 11, color: "#9CA3AF" }}>
                    该函数无参数
                  </div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {currentFn.fnParams.map((param) => {
                      const binding = paramBindings[param.name];
                      const typeError = binding ? getTypeError(param, binding) : null;
                      return (
                        <div key={param.name} style={{
                          padding: "6px 8px", background: "#F9FAFB", borderRadius: 5,
                          border: "1px solid #E5E7EB",
                        }}>
                          <div style={{ fontSize: 10, color: "#374151", marginBottom: 3, fontFamily: "monospace" }}>
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
                              style={{
                                padding: "3px 6px", fontSize: 10, border: "1px solid #D1D5DB",
                                borderRadius: 4, width: 75, background: "#fff",
                              }}
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
                                  [param.name]: {
                                    ...binding,
                                    sourceType: binding?.sourceType || "literal",
                                    value: e.target.value,
                                    expectedType: param.type,
                                  },
                                });
                              }}
                              placeholder={
                                binding?.sourceType === "self" ? "self.signalName" :
                                binding?.sourceType === "event" ? "event.value" :
                                binding?.sourceType === "global" ? "app.xxx" :
                                "固定值"
                              }
                              style={{
                                padding: "3px 6px", fontSize: 10, border: "1px solid #D1D5DB",
                                borderRadius: 4, flex: 1, boxSizing: "border-box",
                              }}
                            />
                          </div>
                          {typeError && (
                            <div style={{ fontSize: 9, color: "#DC2626", marginTop: 2 }}>🚫 {typeError}</div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}

            {/* Existing binding info */}
            {existingBinding && (
              <div style={{
                fontSize: 11, color: "#059669", padding: "6px 8px",
                background: "#F0FDF4", borderRadius: 5, border: "1px solid #BBF7D0",
              }}>
                ✅ 当前已绑定: <strong>{existingBinding.fnName}</strong>
                <button
                  onClick={handleRemove}
                  style={{
                    marginLeft: 8, fontSize: 10, padding: "1px 8px",
                    border: "1px solid #FCA5A5", borderRadius: 3,
                    background: "#FEF2F2", color: "#DC2626", cursor: "pointer",
                  }}
                >
                  解除绑定
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div style={{
          padding: "8px 16px", borderTop: "1px solid #E5E7EB",
          display: "flex", justifyContent: "flex-end", gap: 8,
        }}>
          <button onClick={onClose} style={{
            padding: "6px 14px", fontSize: 11, border: "1px solid #D1D5DB",
            borderRadius: 6, background: "#fff", cursor: "pointer", color: "#374151",
          }}>
            取消
          </button>
          <button
            onClick={handleSave}
            disabled={!selectedFn || !selectedTab}
            style={{
              padding: "6px 14px", fontSize: 11, fontWeight: 600,
              border: "none", borderRadius: 6,
              background: selectedFn && selectedTab ? "#4F46E5" : "#D1D5DB",
              color: "#fff", cursor: selectedFn && selectedTab ? "pointer" : "not-allowed",
            }}
          >
            保存事件绑定
          </button>
        </div>
      </div>
    </>
  );
}

// ═══════════════════════════════════════════════
// BoundEventItem — collapsible bound event with param bindings
// ═══════════════════════════════════════════════

function BoundEventItem({ name, binding, description, isSpecial, onRemove, onUpdate }: {
  name: string;
  binding: EventBinding;
  description: string;
  isSpecial?: boolean;
  onRemove: () => void;
  onUpdate: (binding: EventBinding) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div style={{ marginBottom: 3, borderRadius: 5, border: "1px solid #BBF7D0", background: "#F0FDF4", overflow: "hidden" }}>
      {/* Header — click to expand/collapse */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "6px 8px", cursor: "pointer", userSelect: "none",
          fontSize: 11,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 8, transition: "transform 0.15s", transform: expanded ? "rotate(0deg)" : "rotate(-90deg)" }}>
            ▼
          </span>
          <span style={{ fontWeight: 600, color: "#374151" }}>{name}</span>
          {isSpecial && (
            <span style={{ fontSize: 8, padding: "0 4px", borderRadius: 3, background: "#EDE9FE", color: "#6D28D9", fontWeight: 600 }}>
              特有
            </span>
          )}
          {description && (
            <span style={{ fontSize: 9, color: "#9CA3AF" }}>({description})</span>
          )}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 10, color: "#059669", maxWidth: 120, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            → {binding.fnName}
          </span>
          <button
            onClick={(e) => { e.stopPropagation(); onRemove(); }}
            title="移除事件绑定"
            style={{
              border: "none", background: "transparent", cursor: "pointer",
              color: "#DC2626", fontSize: 10, padding: "0 2px", lineHeight: 1,
            }}
          >
            ✕
          </button>
        </div>
      </div>

      {/* Expanded: param bindings */}
      {expanded && (
        <div style={{ padding: "6px 8px 8px 22px", borderTop: "1px solid #D1FAE5", fontSize: 11 }}>
          <div style={{ fontSize: 10, color: "#6B7280", marginBottom: 4, fontWeight: 600 }}>参数绑定</div>
          {Object.keys(binding.paramBindings).length === 0 ? (
            <div style={{ color: "#9CA3AF", fontSize: 10 }}>无参数</div>
          ) : (
            Object.entries(binding.paramBindings).map(([paramName, pb]) => (
              <div key={paramName} style={{ marginBottom: 4 }}>
                <div style={{ fontSize: 10, color: "#374151", marginBottom: 1, fontFamily: "monospace" }}>
                  {paramName}
                </div>
                <div style={{ display: "flex", gap: 4 }}>
                  <select
                    value={pb.sourceType}
                    onChange={(e) => {
                      const sourceType = e.target.value as ParamBinding["sourceType"];
                      onUpdate({
                        ...binding,
                        paramBindings: {
                          ...binding.paramBindings,
                          [paramName]: { ...pb, sourceType, value: "", expectedType: pb.expectedType },
                        },
                      });
                    }}
                    style={{ padding: "3px 6px", fontSize: 10, border: "1px solid #D1D5DB", borderRadius: 4, width: 70, background: "#fff" }}
                  >
                    <option value="event">📩 事件</option>
                    <option value="self">🔤 self</option>
                    <option value="literal">🔢 固定值</option>
                    <option value="global">🌐 全局</option>
                  </select>
                  <input
                    value={pb.value || ""}
                    onChange={(e) => {
                      onUpdate({
                        ...binding,
                        paramBindings: {
                          ...binding.paramBindings,
                          [paramName]: { ...pb, value: e.target.value },
                        },
                      });
                    }}
                    placeholder={pb.sourceType === "self" ? "self.signalName" : pb.sourceType === "event" ? "event.value" : pb.sourceType === "global" ? "app.xxx" : "固定值"}
                    style={{ padding: "3px 6px", fontSize: 10, border: "1px solid #D1D5DB", borderRadius: 4, flex: 1 }}
                  />
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const sectionHeaderStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" };
