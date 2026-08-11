// components/panels/ContextInspector.tsx
// Context-sensitive right panel showing properties, styles, or events
// based on the selected element type.

import { useSelectedElementIds, useElements } from "../../store/hooks";
import styles from "./ContextInspector.module.css";
import { PropsSection } from "./inspector/PropsSection";
import { EventsSection } from "./inspector/EventsSection";
import { BindingsSection } from "./inspector/BindingsSection";
import { useHandlerItems, BASIC_TAGS, useSignalItems } from "./inspector/shared";
import { InlineStylePanel } from "./inspector/InlineStylePanel";
import { ClassesSection } from "./inspector/ClassesSection";
import { VariantBindingSection } from "./inspector/VariantBindingSection";
import { BranchConditionEditor } from "./inspector/BranchConditionEditor";
import { ConditionalBlockEditor } from "./inspector/ConditionalBlockEditor";
import { ForBlockEditor } from "./inspector/ForBlockEditor";
import { usePoiesisStore } from "../../store";

export function ContextInspector() {
  const selectedIds = useSelectedElementIds();
  const elements = useElements();
  const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const setStorePanelVisible = usePoiesisStore((s) => s.setStorePanelVisible);
  const signalItems = useSignalItems();
  const handlerItems = useHandlerItems();

  const el = selectedIds.length > 0 ? elements.find((e) => e.id === selectedIds[0]) : undefined;

  // Branch selection: synthetic IDs like "comp.@if.0.br1"
  const branchMatch = selectedIds[0]?.match(/^(.+)\.br(\d+)$/);
  const isBranchSelected = !!branchMatch;

  // Get the conditional block element when a branch is selected
  const condBlock = isBranchSelected && branchMatch
    ? elements.find((e) => e.id === branchMatch[1] && e.nodeType === "conditional")
    : null;

  // Title text
  let titleText = "属性面板";
  if (isBranchSelected && condBlock && branchMatch) {
    const branch = (condBlock as import("../../core/types/element").ConditionalBlockData).branches[parseInt(branchMatch[2], 10)];
    if (branch) {
      titleText = branch.type === "else" ? "🔀 @else" : `🔀 ${branch.type === "if" ? "@if" : "@else if"} · ${branch.condition || "(空条件)"}`;
    }
  } else if (el && el.nodeType === "conditional") {
    titleText = "🔀 条件渲染";
  } else if (el && el.nodeType === "for") {
    titleText = "🔁 循环";
  } else if (el && "tag" in el) {
    titleText = `属性 · <${el.tag}>`;
  }

  return (
    <div className="context-inspector" style={{ display: "flex", flexDirection: "column", height: "100%", minWidth: 0 }}>
      {/* Title bar */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "6px 12px", borderBottom: "1px solid var(--ol-border)",
        minHeight: 32,
      }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: "var(--ol-text-secondary)" }}>
          {titleText}
        </span>
        <button type="button"
          onClick={() => setStorePanelVisible(!storePanelVisible)}
          title={storePanelVisible ? "关闭 Store 面板" : "打开 Store 面板"}
          className={styles.storeBtn}
          style={{
            border: storePanelVisible ? "1px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
            background: storePanelVisible ? "var(--ol-color-primary-subtle)" : "transparent",
            color: storePanelVisible ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
          }}
        >
          📦 Store
        </button>
      </div>

      {/* Body — for block editor（view @for，2026-08-06） */}
      {el && el.nodeType === "for" ? (
        <ForBlockEditor blockId={el.id} />
      ) : el && el.nodeType === "conditional" ? (
        <div className="context-inspector" data-section="conditional-editor" style={{
          padding: 12, overflow: "auto", height: "100%", boxSizing: "border-box",
        }}>
          <ConditionalBlockEditor blockId={el.id} />
        </div>
      ) : isBranchSelected && branchMatch ? (
        <div className="context-inspector" data-section="branch-editor" style={{
          padding: 12, overflow: "auto", height: "100%", boxSizing: "border-box",
        }}>
          <BranchConditionEditor blockId={branchMatch[1]} branchIndex={parseInt(branchMatch[2], 10)} />
        </div>
      ) : !el ? (
        <div style={{ padding: 16, color: "var(--ol-text-muted)", fontSize: 13, textAlign: "center", flex: 1 }}>
          选择元素以编辑属性
        </div>
      ) : null}

      {el && !isBranchSelected && el.nodeType !== "conditional" && el.nodeType !== "for" && (() => {

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
                color: "var(--ol-gray-700)",
              }}
            >
              {tag}
              {componentRef ? ` (${componentRef})` : ""}
            </h3>

            <PropsSection element={el} signalItems={signalItems} />

            {isBasic && <BindingsSection element={el} signalItems={signalItems} />}

            <EventsSection element={el} handlerItems={handlerItems} />

            {isBasic && <ClassesSection element={el} />}
            {isBasic && <VariantBindingSection element={el} />}

            {isBasic && <InlineStylePanel element={el} />}
          </div>
        );
      })()}

    </div>
  );
}
