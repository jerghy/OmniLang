// components/panels/inspector/EventsSection.tsx
// Event binding form. Shows available events for the component type.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";

interface EventsSectionProps {
  element: ElementData;
}

/** Standard DOM events applicable to most elements */
const STANDARD_EVENTS = [
  "onClick",
  "onDoubleClick",
  "onChange",
  "onSubmit",
  "onFocus",
  "onBlur",
  "onMouseEnter",
  "onMouseLeave",
  "onKeyDown",
  "onKeyUp",
  "onInput",
  "onDragStart",
  "onDrop",
];

export function EventsSection({ element }: EventsSectionProps) {
  const setProp = usePoiesisStore((s) => s.setProp);

  const [expanded, setExpanded] = useState(false);

  // Find bound events from props
  const boundEvents = "props" in element
    ? Object.entries(element.props)
        .filter(
          ([key, val]) =>
            key.startsWith("on") && typeof val === "string" && val.trim().length > 0
        )
        .map(([key, val]) => ({ name: key, handler: String(val) }))
    : [];

  const availableEvents = STANDARD_EVENTS.filter(
    (ev) => !boundEvents.some((b) => b.name === ev)
  );

  const handleBindEvent = useCallback(
    (eventName: string) => {
      const handler = `handle${eventName.replace("on", "")}`;
      setProp(element.id, eventName, handler);
    },
    [element.id, setProp]
  );

  const handleRemoveEvent = useCallback(
    (eventName: string) => {
      setProp(element.id, eventName, "");
    },
    [element.id, setProp]
  );

  const handleHandlerChange = useCallback(
    (eventName: string, handler: string) => {
      setProp(element.id, eventName, handler);
    },
    [element.id, setProp]
  );

  return (
    <div style={{ marginBottom: 12 }}>
      <div
        style={{
          fontSize: 11,
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
          <button
            onClick={() => setExpanded(!expanded)}
            style={{
              fontSize: 10,
              border: "none",
              background: "none",
              cursor: "pointer",
              color: "#6366F1",
              padding: 0,
            }}
          >
            {expanded ? "收起" : "+ 添加事件"}
          </button>
        )}
      </div>

      {/* Bound events */}
      {boundEvents.length === 0 && !expanded && (
        <div style={{ fontSize: 11, color: "#94a3b8" }}>
          暂无事件绑定
        </div>
      )}

      {boundEvents.map((ev) => (
        <div
          key={ev.name}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            marginBottom: 4,
          }}
        >
          <span
            style={{
              fontSize: 11,
              color: "#374151",
              minWidth: 70,
              fontWeight: 500,
            }}
          >
            {ev.name}
          </span>
          <input
            type="text"
            value={ev.handler}
            onChange={(e) => handleHandlerChange(ev.name, e.target.value)}
            style={{
              flex: 1,
              padding: "2px 6px",
              fontSize: 11,
              border: "1px solid #e2e8f0",
              borderRadius: 4,
              outline: "none",
              fontFamily: "monospace",
            }}
            onFocus={(e) => {
              e.target.style.borderColor = "#6366F1";
            }}
            onBlur={(e) => {
              e.target.style.borderColor = "#e2e8f0";
            }}
          />
          <button
            onClick={() => handleRemoveEvent(ev.name)}
            title="移除事件"
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: "#ef4444",
              fontSize: 12,
              padding: "2px 4px",
            }}
          >
            ×
          </button>
        </div>
      ))}

      {/* Available events to add */}
      {expanded && availableEvents.length > 0 && (
        <div
          style={{
            marginTop: 4,
            display: "flex",
            flexWrap: "wrap",
            gap: 4,
          }}
        >
          {availableEvents.map((ev) => (
            <button
              key={ev}
              onClick={() => {
                handleBindEvent(ev);
              }}
              style={{
                padding: "2px 8px",
                fontSize: 10,
                border: "1px solid #e0e7ff",
                borderRadius: 4,
                background: "#eef2ff",
                color: "#4f46e5",
                cursor: "pointer",
              }}
            >
              {ev}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
