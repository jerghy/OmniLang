// components/panels/ContextInspector.tsx
// Context-sensitive right panel showing properties, styles, or events
// based on the selected element type.

import { useMemo } from "react";
import { useSelectedElementIds, useElements } from "../../store/hooks";
import { PropsSection } from "./inspector/PropsSection";
import { EventsSection } from "./inspector/EventsSection";
import { BindingsSection } from "./inspector/BindingsSection";
import { useHandlerItems } from "./inspector/useHandlerItems";
import { InlineStylePanel } from "./inspector/InlineStylePanel";
import { ClassesSection } from "./inspector/ClassesSection";
import { VariantBindingSection } from "./inspector/VariantBindingSection";
import { BranchConditionEditor } from "./inspector/BranchConditionEditor";
import { usePoiesisStore } from "../../store";
import type { SignalPickerItem } from "./inspector/SignalSlashPicker";

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

/** Compute available signal/computed/store items for the active component. */
function useSignalItems(): SignalPickerItem[] {
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);
  const inlineStoreDefs = usePoiesisStore((s) => s.inlineStoreDefs);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);

  return useMemo(() => {
    const activeTab = activeTabId
      ? openTabs.find((t) => t.id === activeTabId)
      : undefined;
    const compName = activeTab?.componentName;
    if (!compName) return [];

    const items: SignalPickerItem[] = [];
    const compDef = componentDefs[compName];

    // Component signals
    if (compDef?.signals) {
      for (const s of compDef.signals) {
        items.push({
          pid: s.pid,
          name: s.name,
          insertionText: s.name,
          type: s.type,
          category: "signals",
        });
      }
    }

    // Component computeds
    if (compDef?.computeds) {
      for (const c of compDef.computeds) {
        items.push({
          pid: c.pid,
          name: c.name,
          insertionText: c.name,
          type: c.returnType,
          category: "computeds",
        });
      }
    }

    // Store signals and getters referenced by this component
    const refs = storeReferences[compName] ?? [];
    for (const storeName of refs) {
      const storeDef = storeDefs[storeName];
      if (storeDef) {
        for (const sig of storeDef.signals) {
          items.push({
            pid: sig.pid,
            name: `${storeName}.${sig.name}`,
            insertionText: `${storeName}.${sig.name}`,
            type: sig.type,
            category: "store-signals",
          });
        }
        for (const getter of storeDef.getters) {
          items.push({
            pid: getter.pid,
            name: `${storeName}.${getter.name}`,
            insertionText: `${storeName}.${getter.name}`,
            type: getter.returnType,
            category: "store-getters",
          });
        }
      }
    }

    // Inline store signals and getters
    for (const [key, storeDef] of Object.entries(inlineStoreDefs)) {
      // Inline stores are keyed as "ComponentName.StoreName"
      if (!key.startsWith(compName + ".")) continue;

      // Inline store signals can be accessed without prefix
      for (const sig of storeDef.signals) {
        items.push({
          pid: sig.pid,
          name: sig.name,
          insertionText: sig.name,
          type: sig.type,
          category: "store-signals",
        });
      }
      // Inline store getters can be accessed without prefix
      for (const getter of storeDef.getters) {
        items.push({
          pid: getter.pid,
          name: getter.name,
          insertionText: getter.name,
          type: getter.returnType,
          category: "store-getters",
        });
      }
    }

    return items;
  }, [activeTabId, openTabs, componentDefs, storeDefs, inlineStoreDefs, storeReferences]);
}

export function ContextInspector() {
  const selectedIds = useSelectedElementIds();
  const elements = useElements();
  const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const setStorePanelVisible = usePoiesisStore((s) => s.setStorePanelVisible);
  const signalItems = useSignalItems();
  const handlerItems = useHandlerItems();

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

      {/* Body — branch condition editor if selected is a branch */}
      {el && selectedIds[0] && (() => {
        const branchMatch = selectedIds[0].match(/^(.+)\.br(\d+)$/);
        if (branchMatch) {
          const blockId = branchMatch[1];
          const branchIndex = parseInt(branchMatch[2], 10);
          return (
            <div className="context-inspector" data-section="branch-editor" style={{
              padding: 12, overflow: "auto", height: "100%", boxSizing: "border-box",
            }}>
              <BranchConditionEditor blockId={blockId} branchIndex={branchIndex} />
            </div>
          );
        }
        return null;
      })()}

      {/* Empty state */}
      {!el ? (
        <div style={{ padding: 16, color: "#94a3b8", fontSize: 13, textAlign: "center", flex: 1 }}>
          选择元素以编辑属性
        </div>
      ) : null}

      {el && (() => {
        // If branch editor is active, skip normal rendering
        const isBranchSelected = selectedIds[0]?.match(/^(.+)\.br(\d+)$/);
        if (isBranchSelected) return null;

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
