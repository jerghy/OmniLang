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
import styles from "./EventsSection.module.css";

interface EventsSectionProps {
  element: ElementData;
  handlerItems: HandlerItem[];
}

/** Event groups for optgroup rendering in the event dropdown */
// 语义化（Task 4）：旧 DOM 拼写 onClick/onMouseEnter/onMouseLeave 是编译期
// 硬错误（E0006），可见选项收敛为语义名 onPress/onEnter/onLeave + 指针族。
// 与 componentProtocol.ts UNIVERSAL_EVENTS / Rust event_names.rs 同步。
const EVENT_GROUPS: Record<string, string[]> = {
  "点击": ["onPress", "onDoubleClick", "onContextMenu"],
  "输入": ["onChange", "onInput"],
  "焦点": ["onFocus", "onBlur"],
  "键盘": ["onKeyDown", "onKeyUp"],
  "表单": ["onSubmit"],
  "滚动": ["onScroll"],
  "鼠标": ["onEnter", "onLeave", "onMouseDown", "onMouseUp", "onMouseMove"],
  "指针": ["onPointerDown", "onPointerMove", "onPointerUp"],
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
    const result: Array<{ eventName: string; handler: string; meta: EventMeta | undefined }> = [];
    for (const [key, val] of Object.entries(elementProps)) {
      if (key.startsWith("on") && typeof val === "string") {
        result.push({
          eventName: key,
          handler: String(val),
          meta: protocolEvents[key] as EventMeta | undefined,
        });
      }
    }
    return result;
  }, [elementProps, protocolEvents]);

  const boundEventNames = useMemo(
    () => new Set(boundEvents.map((b) => b.eventName)),
    [boundEvents]
  );

  // Event options for the "add" dropdown (exclude already bound)
  const availableEvents = useMemo(() => {
    const result: Array<{ name: string } & EventMeta> = [];
    for (const [name, meta] of Object.entries(protocolEvents)) {
      if (!boundEventNames.has(name)) {
        result.push({ name, ...meta });
      }
    }
    return result;
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
      <div className={styles.sectionHeader}>
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
            className={styles.addEventSelect}
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
        <div style={{ fontSize: 12, color: "var(--ol-text-muted)" }}>
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
              className={styles.eventNameSelect}
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
              className={styles.handlerBtn}
              style={{
                color: ev.handler ? "var(--ol-gray-800)" : "var(--ol-border-hover)",
              }}
            >
              {ev.handler || "点击编辑..."}
            </button>

            {/* Remove */}
            <button type="button"
              onClick={() => handleRemoveEvent(ev.eventName)}
              title="移除事件"
              className={styles.removeBtn}
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
