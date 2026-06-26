/** 属性面板。根据选中元素和统一元数据动态渲染编辑器。
 *  编辑自定义组件定义时显示 ComponentMetaEditor。*/

import React, { useState, useCallback, useRef, useEffect } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { useCodeGraphStore } from "../codegraph/store";
import type { EventBinding, ParamBinding, FnParam } from "../codegraph/types";
import { SetProp } from "../commands/SetProp";
import { uid } from "../utils/id";
import { compilerCache } from "../omni-compiler-cache";
import { COMPONENT_PROTOCOL } from "./ComponentPalette";
import {
  getUnifiedComponentMeta,
  useComponentRegistry,
  type PropSchema,
  type EventSchema,
} from "../store/componentRegistry";
import { StyleEntryEditor } from "./StyleEntryEditor";
import { AddStyleEntry } from "../commands/AddStyleEntry";
import { RemoveStyleEntry } from "../commands/RemoveStyleEntry";
import { UpdateStyleEntry } from "../commands/UpdateStyleEntry";
import { ReorderStyleEntry } from "../commands/ReorderStyleEntry";
import type { StyleEntry } from "../constants/style-properties";
import { getComponentEvents, UNIVERSAL_EVENTS } from "../data/componentProtocol";

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

/** Immutable reorder of an array: move element at `from` index to `to` index. */
function reorderArray<T>(arr: readonly T[], from: number, to: number): T[] {
  const result = [...arr];
  const [removed] = result.splice(from, 1);
  result.splice(to, 0, removed);
  return result;
}

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
  const [expandedStyleId, setExpandedStyleId] = useState<string | null>(null);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dropIndex, setDropIndex] = useState<number | null>(null);

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

  const [showEventDialog, setShowEventDialog] = useState(false);
  const [editingEvent, setEditingEvent] = useState<string | null>(null);

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

  // ── Map HTML tags to OmniLang component names for metadata lookup ──
  const HTML_TAG_TO_COMPONENT: Record<string, string> = {
    // Typography
    'p': 'Text', 'span': 'Text',
    'h1': 'Heading', 'h2': 'Heading', 'h3': 'Heading',
    'h4': 'Heading', 'h5': 'Heading', 'h6': 'Heading',
    // Actions
    'button': 'Button',
    // Forms
    'input': 'Input', 'textarea': 'TextArea', 'select': 'Select', 'form': 'Form',
    // Media
    'img': 'Image',
    // Navigation
    'a': 'Link',
    // Data
    'ul': 'List', 'li': 'ListItem',
    // Layout (ambiguous — default to Container for best-effort metadata)
    'div': 'Container', 'section': 'Container', 'header': 'Container',
    'footer': 'Container', 'main': 'Container',
  };
  const resolvedTag = HTML_TAG_TO_COMPONENT[el.tag] || el.tag;

  // Resolve metadata: custom component instances use their definition's metadata
  const unifiedMeta = getUnifiedComponentMeta();
  let meta: ComponentMeta | undefined = unifiedMeta[resolvedTag];

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

  // ── Merge universal events + component-specific events ──
  // OmniView 采用开放事件模型：任何组件都能绑定任何 onXxx 事件。
  // 通用事件是所有组件都支持的，组件特殊事件则提供更精准的描述和 ID。
  const allEvents: Record<string, EventMeta> = {
    ...getComponentEvents(resolvedTag),
    ...meta.events,
  };

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

      {/* Style List */}
      <div style={{ ...sectionHeaderStyle, display: "flex", alignItems: "center" }}>
        样式
        <button
          onClick={() => {
            const entry: StyleEntry = {
              id: `s_${Date.now()}`,
              type: "inline",
              name: "",
              properties: [],
            };
            undoExec(new AddStyleEntry(el.id, entry), ctx);
            setExpandedStyleId(entry.id);
          }}
          style={{
            background: "none", border: "1px solid #d1d5db", borderRadius: 4,
            color: "#4F46E5", cursor: "pointer", fontSize: 11,
            padding: "2px 8px", marginLeft: "auto",
          }}
        >
          +添加
        </button>
      </div>

      {/* Render each style entry */}
      {(el.styles || []).map((entry: StyleEntry, index: number) => {
        const isExpanded = expandedStyleId === entry.id;
        const displayName = entry.type === "inline"
          ? `inline (${entry.properties.map(p => p.name).join(", ") || "empty"})`
          : entry.type === "variable"
            ? entry.name || "--unnamed"
            : `.${entry.name}`;

        const handleDropAt = (toIndex: number) => {
          if (dragIndex !== null && dragIndex !== toIndex) {
            const entries = el.styles || [];
            const oldIds = entries.map((e: StyleEntry) => e.id);
            const newIds = reorderArray(oldIds, dragIndex, toIndex);
            undoExec(new ReorderStyleEntry(el.id, oldIds, newIds), ctx);
          }
          setDragIndex(null);
          setDropIndex(null);
        };

        return (
          <div key={entry.id}>
            {/* Collapsed row — draggable unless expanded */}
            <div
              draggable={!isExpanded}
              onDragStart={(e) => {
                setDragIndex(index);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragOver={(e) => {
                e.preventDefault();
                e.dataTransfer.dropEffect = "move";
                if (dragIndex !== index && dropIndex !== index) {
                  setDropIndex(index);
                }
              }}
              onDragLeave={() => {
                if (dropIndex === index) setDropIndex(null);
              }}
              onDrop={() => handleDropAt(index)}
              onDragEnd={() => {
                setDragIndex(null);
                setDropIndex(null);
              }}
              onClick={() => setExpandedStyleId(isExpanded ? null : entry.id)}
              style={{
                display: "flex", alignItems: "center",
                cursor: dragIndex === index ? "grabbing" : "grab",
                padding: "4px 0", color: "#374151", fontSize: 12,
                userSelect: "none",
                opacity: dragIndex === index ? 0.4 : 1,
                borderTop: dropIndex === index ? "2px solid #4F46E5" : "2px solid transparent",
                transition: "opacity 0.15s, border-color 0.15s",
              }}
            >
              <span style={{ marginRight: 6 }}>{isExpanded ? "▼" : "▶"}</span>
              <span style={{ flex: 1 }}>{displayName}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  undoExec(new RemoveStyleEntry(el.id, entry, index), ctx);
                  if (isExpanded) setExpandedStyleId(null);
                }}
                style={{
                  background: "none", border: "none", color: "#DC2626",
                  cursor: "pointer", fontSize: 14, padding: "0 4px",
                }}
              >
                ×
              </button>
            </div>

            {/* Expanded editor */}
            {isExpanded && (
              <StyleEntryEditor
                entry={entry}
                elementId={el.id}
                onChange={(patch) => {
                  const oldEntry = useEditorStore
                    .getState()
                    .getStyles(el.id)
                    .find((s) => s.id === entry.id);
                  if (!oldEntry) return;
                  const newEntry = { ...oldEntry, ...patch };
                  undoExec(
                    new UpdateStyleEntry(el.id, entry.id, oldEntry, newEntry),
                    ctx,
                  );
                }}
                onClose={() => setExpandedStyleId(null)}
              />
            )}
          </div>
        );
      })}

      {/* Bottom drop zone — dropping below the last entry */}
      {(el.styles?.length ?? 0) > 0 && (
        <div
          onDragOver={(e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
            const len = el.styles!.length;
            if (dropIndex !== len) setDropIndex(len);
          }}
          onDragLeave={() => {
            const len = el.styles!.length;
            if (dropIndex === len) setDropIndex(null);
          }}
          onDrop={() => {
            const len = el.styles!.length;
            if (dragIndex !== null && dragIndex !== len) {
              const entries = el.styles || [];
              const oldIds = entries.map((e: StyleEntry) => e.id);
              const newIds = reorderArray(oldIds, dragIndex, len);
              undoExec(new ReorderStyleEntry(el.id, oldIds, newIds), ctx);
            }
            setDragIndex(null);
            setDropIndex(null);
          }}
          style={{
            height: 4,
            background: dropIndex === el.styles!.length ? "#4F46E5" : "transparent",
            transition: "background 0.15s",
          }}
        />
      )}

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
            debouncedRecompile();
          }}
        />
      </label>
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

// ═══════════════════════════════════════════════
// Event binding dialog (modal)
// ═══════════════════════════════════════════════

function EventBindingDialog({ events, specialEventNames, initialEvent, selectedEl, onBind, onClose }: {
  events: [string, EventMeta][];
  specialEventNames: Set<string>;
  initialEvent: string | null;
  selectedEl: import("../store/editor").ElementData;
  onBind: (eventName: string, binding: EventBinding | null) => void;
  onClose: () => void;
}) {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const [selectedEventName, setSelectedEventName] = useState<string>(initialEvent || events[0]?.[0] || "");
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
    setTimeout(() => {
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
