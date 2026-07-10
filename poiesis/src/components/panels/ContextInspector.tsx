// components/panels/ContextInspector.tsx
// Context-sensitive right panel showing properties, styles, or events
// based on the selected element type.

import { useSelectedElementIds, useElements } from "../../store/hooks";
import { PropsSection } from "./inspector/PropsSection";
import { EventsSection } from "./inspector/EventsSection";
import { InlineStylePanel } from "./inspector/InlineStylePanel";
import { ClassesSection } from "./inspector/ClassesSection";
import { VariantBindingSection } from "./inspector/VariantBindingSection";
import { usePoiesisStore } from "../../store";

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
  const selectedIds = useSelectedElementIds();
  const elements = useElements();
  const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const setStorePanelVisible = usePoiesisStore((s) => s.setStorePanelVisible);

  const el = selectedIds.length > 0 ? elements.find((e) => e.id === selectedIds[0]) : undefined;

  return (
    <div className="context-inspector" style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Title bar */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "6px 12px", borderBottom: "1px solid #e2e8f0",
        minHeight: 32,
      }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: "#64748b" }}>
          {el ? ("tag" in el ? `属性 · <${el.tag}>` : "属性") : "属性面板"}
        </span>
        <button
          onClick={() => setStorePanelVisible(!storePanelVisible)}
          title={storePanelVisible ? "关闭 Store 面板" : "打开 Store 面板"}
          style={{
            display: "flex", alignItems: "center", gap: 4,
            padding: "2px 10px", fontSize: 11, fontWeight: 600,
            border: storePanelVisible ? "1px solid #6366f1" : "1px solid #e2e8f0",
            borderRadius: 4,
            background: storePanelVisible ? "#eef2ff" : "transparent",
            cursor: "pointer",
            color: storePanelVisible ? "#6366f1" : "#64748b",
            fontFamily: "inherit",
          }}
        >
          📦 Store
        </button>
      </div>

      {/* Body — existing logic */}
      {!el ? (
        <div style={{ padding: 16, color: "#94a3b8", fontSize: 13, textAlign: "center", flex: 1 }}>
          选择元素以编辑属性
        </div>
      ) : null}

      {el && (() => {
        const isBasic = "tag" in el && BASIC_TAGS.has(el.tag);
        const tag = "tag" in el ? el.tag : "unknown";
        const componentRef = "componentRef" in el ? el.componentRef : undefined;

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
              {tag}
              {componentRef ? ` (${componentRef})` : ""}
            </h3>

            <PropsSection element={el} />

            {isBasic && <ClassesSection element={el} />}
            {isBasic && <VariantBindingSection element={el} />}

            {isBasic && <InlineStylePanel element={el} />}

            {!isBasic && <EventsSection element={el} />}
          </div>
        );
      })()}

    </div>
  );
}
