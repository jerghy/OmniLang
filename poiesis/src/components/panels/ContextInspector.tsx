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
import { ConditionalBlockEditor } from "./inspector/ConditionalBlockEditor";
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
  } else if (el && "tag" in el) {
    titleText = `属性 · <${el.tag}>`;
  }

  return (
    <div className="context-inspector" style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Title bar */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "6px 12px", borderBottom: "1px solid #e2e8f0",
        minHeight: 32,
      }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: "#64748b" }}>
          {titleText}
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

      {/* Body — conditional block editor when block itself is selected */}
      {el && el.nodeType === "conditional" ? (
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
        <div style={{ padding: 16, color: "#94a3b8", fontSize: 13, textAlign: "center", flex: 1 }}>
          选择元素以编辑属性
        </div>
      ) : null}

      {el && !isBranchSelected && el.nodeType !== "conditional" && (() => {

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
