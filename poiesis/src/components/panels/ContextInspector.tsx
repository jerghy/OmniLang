// components/panels/ContextInspector.tsx
// Context-sensitive right panel showing properties, styles, or events
// based on the selected element type.

import { usePoiesisStore } from "../../store";
import { PropsSection } from "./inspector/PropsSection";
import { EventsSection } from "./inspector/EventsSection";
import { StylesSection } from "./inspector/StylesSection";

/** HTML-like tags that get PropsSection + StylesSection */
const BASIC_TAGS = new Set([
  "div",
  "span",
  "button",
  "input",
  "p",
  "h1",
  "h2",
  "h3",
  "img",
  "a",
  "section",
  "article",
  "header",
  "footer",
  "nav",
  "main",
  "ul",
  "ol",
  "li",
  "label",
  "textarea",
  "select",
]);

export function ContextInspector() {
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const elements = usePoiesisStore((s) => s.elements);

  if (selectedIds.length === 0) {
    return (
      <div
        className="context-inspector"
        style={{
          padding: 16,
          color: "#94a3b8",
          fontSize: 13,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
        }}
      >
        选择元素以编辑属性
      </div>
    );
  }

  const el = elements.find((e) => e.id === selectedIds[0]);
  if (!el) return null;

  const isBasic = BASIC_TAGS.has(el.tag);

  return (
    <div
      className="context-inspector"
      data-section="props"
      style={{
        padding: 12,
        overflow: "auto",
        height: "100%",
        boxSizing: "border-box",
      }}
    >
      <h3
        style={{
          fontSize: 13,
          fontWeight: 600,
          margin: "0 0 8px",
          color: "#374151",
        }}
      >
        {el.tag}
        {el.componentRef ? ` (${el.componentRef})` : ""}
      </h3>

      <PropsSection element={el} />

      {isBasic && <StylesSection element={el} />}

      {!isBasic && <EventsSection element={el} />}
    </div>
  );
}
