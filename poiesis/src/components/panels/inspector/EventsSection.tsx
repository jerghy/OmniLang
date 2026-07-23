// components/panels/inspector/EventsSection.tsx
// Event binding panel with compact rows. Each row: event dropdown + handler display + remove.
// Clicking the handler area opens HandlerModal for visual editing.

import { useState, useCallback, useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";
import type { EventMeta } from "../../../data/componentProtocol";
import { getComponentEvents } from "../../../data/componentProtocol";
import type { HandlerItem } from "./useHandlerItems";
import { HandlerModal } from "./HandlerModal";

interface EventsSectionProps {
  element: ElementData;
  handlerItems: HandlerItem[];
}

/** Event groups for optgroup rendering in the event dropdown */
const EVENT_GROUPS: Record<string, string[]> = {
  "点击": ["onPress", "onClick", "onDoubleClick", "onContextMenu"],
  "输入": ["onChange", "onInput"],
  "焦点": ["onFocus", "onBlur"],
  "键盘": ["onKeyDown", "onKeyUp"],
  "表单": ["onSubmit"],
  "滚动": ["onScroll"],
  "鼠标": ["onMouseEnter", "onMouseLeave", "onMouseDown", "onMouseUp", "onMouseMove"],
  "拖拽": ["onDragStart", "onDragOver", "onDragEnd", "onDrop"],
  "触摸": ["onTouchStart", "onTouchEnd", "onTouchMove"],
  "滚轮": ["onWheel"],
  "媒体": ["onLoad", "onError", "onPlay", "onPause", "onEnded"],
  "动画": ["onAnimationStart", "onAnimationEnd", "onAnimationIteration", "onTransitionEnd"],
  "切换": ["onToggle"],
  "剪贴板": ["onCopy", "onCut", "onPaste"],
};

export function EventsSection({ element, handlerItems }: EventsSectionProps) {
  const setProp = usePoiesisStore((s) => s.setProp);

  // Which event is being edited in the modal (null = modal closed)
  const [editingEvent, setEditingEvent] = useState<string | null>(null);

  // Get available events from component protocol
  const tag = "tag" in element ? element.tag : "div";
  const protocolEvents = getComponentEvents(tag);

  // Build the list of bound events from element props
  const elementProps: Record<string, unknown> | null = "props" in element ? element.props : null;

  const boundEvents = useMemo(() => {
    if (!elementProps) return [];
    return Object.entries(elementProps)
      .filter(
        ([key, val]) =>
          key.startsWith("on") &&
          typeof val === "string"
      )
      .map(([key, val]) => ({
        eventName: key,
        handler: String(val),
        meta: protocolEvents[key] as EventMeta | undefined,
      }));
  }, [elementProps, protocolEvents]);

  const boundEventNames = useMemo(
    () => new Set(boundEvents.map((b) => b.eventName)),
    [boundEvents]
  );

  // Event options for the "add" dropdown (exclude already bound)
  const availableEvents = useMemo(() => {
    return Object.entries(protocolEvents)
      .filter(([name]) => !boundEventNames.has(name))
      .map(([name, meta]) => ({ name, ...meta }));
  }, [protocolEvents, boundEventNames]);

  // Group available events for optgroup rendering
  const groupedAvailable = useMemo(() => {
    const result: Array<{ group: string; events: typeof availableEvents }> = [];
    for (const [group, eventNames] of Object.entries(EVENT_GROUPS)) {
      const groupEvents = availableEvents.filter((ev) =>
        eventNames.includes(ev.name)
      );
      if (groupEvents.length > 0) {
        result.push({ group, events: groupEvents });
      }
    }
    return result;
  }, [availableEvents]);

  // ── Handlers ──

  const handleAddEvent = useCallback(
    (eventName: string) => {
      setProp(element.id, eventName, "");
    },
    [element.id, setProp]
  );

  const handleRemoveEvent = useCallback(
    (eventName: string) => {
      setProp(element.id, eventName, undefined);
    },
    [element.id, setProp]
  );

  const handleOpenModal = useCallback((eventName: string) => {
    setEditingEvent(eventName);
  }, []);

  const handleCloseModal = useCallback(() => {
    setEditingEvent(null);
  }, []);

  const handleConfirmHandler = useCallback(
    (handlerString: string) => {
      if (editingEvent) {
        setProp(element.id, editingEvent, handlerString);
      }
      setEditingEvent(null);
    },
    [element.id, editingEvent, setProp]
  );

  const editingHandler = editingEvent
    ? boundEvents.find((b) => b.eventName === editingEvent)?.handler ?? ""
    : "";

  return (
    <div style={{ marginBottom: 12 }}>
      {/* Section header */}
      <div
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: "#6b7280",
          textTransform: "uppercase",
          marginBottom: 6,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span>事件</span>
        {availableEvents.length > 0 && (
          <select
            aria-label="添加事件"
            value=""
            onChange={(e) => {
              if (e.target.value) {
                handleAddEvent(e.target.value);
                e.target.value = "";
              }
            }}
            style={{
              fontSize: 12,
              border: "1px solid #e0e7ff",
              borderRadius: 4,
              background: "#eef2ff",
              color: "#4f46e5",
              cursor: "pointer",
              padding: "2px 6px",
              fontFamily: "inherit",
              maxWidth: 120,
            }}
          >
            <option value="">+ 添加事件</option>
            {groupedAvailable.map((g) => (
              <optgroup key={g.group} label={g.group}>
                {g.events.map((ev) => (
                  <option key={ev.name} value={ev.name}>
                    {ev.name}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        )}
      </div>

      {/* Empty state */}
      {boundEvents.length === 0 && (
        <div style={{ fontSize: 12, color: "#94a3b8" }}>
          暂无事件绑定
        </div>
      )}

      {/* Event rows */}
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {boundEvents.map((ev) => (
          <div
            key={ev.eventName}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            {/* Event name — also a dropdown to change event type */}
            <select
              aria-label="Event name"
              value={ev.eventName}
              onChange={(e) => {
                const newName = e.target.value;
                if (newName !== ev.eventName) {
                  // Move handler to new event name
                  const handler = ev.handler;
                  setProp(element.id, ev.eventName, "");
                  setProp(element.id, newName, handler);
                }
              }}
              style={{
                fontSize: 12,
                padding: "3px 4px",
                border: "1px solid #e2e8f0",
                borderRadius: 4,
                background: "#fff",
                color: "#374151",
                fontFamily: "var(--dp-font-mono, monospace)",
                minWidth: 90,
                cursor: "pointer",
              }}
            >
              {Object.entries(EVENT_GROUPS).map(([group, names]) => (
                <optgroup key={group} label={group}>
                  {names.map((name) => {
                    const exists =
                      name !== ev.eventName && boundEventNames.has(name);
                    return (
                      <option
                        key={name}
                        value={name}
                        disabled={exists}
                      >
                        {name}
                      </option>
                    );
                  })}
                </optgroup>
              ))}
            </select>

            {/* Handler display — clickable to open modal */}
            <button type="button"
              onClick={() => handleOpenModal(ev.eventName)}
              title={ev.meta ? `${ev.meta.description} — ${ev.handler}` : ev.handler}
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                padding: "3px 8px",
                fontSize: 12,
                fontFamily: "var(--dp-font-mono, monospace)",
                border: "1px solid #e2e8f0",
                borderRadius: 4,
                background: "#fff",
                cursor: "pointer",
                textAlign: "left",
                color: ev.handler ? "#1e293b" : "#cbd5e1",
                minHeight: 24,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {ev.handler || "点击编辑..."}
            </button>

            {/* Remove */}
            <button type="button"
              onClick={() => handleRemoveEvent(ev.eventName)}
              title="移除事件"
              style={{
                border: "none",
                background: "none",
                cursor: "pointer",
                color: "#ef4444",
                fontSize: 14,
                padding: "2px 4px",
                lineHeight: 1,
                fontFamily: "inherit",
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Handler modal */}
      {editingEvent && (
        <HandlerModal
          eventName={editingEvent}
          initialHandler={editingHandler}
          handlerItems={handlerItems}
          onConfirm={handleConfirmHandler}
          onClose={handleCloseModal}
        />
      )}
    </div>
  );
}
