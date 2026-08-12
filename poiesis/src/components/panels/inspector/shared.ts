// components/panels/inspector/shared.ts
// 语境面板（T6）与右侧 ContextInspector（T7 删除前）共享的拾取器数据源：
// BASIC_TAGS（基础标签集合）、useSignalItems（信号/计算/存储条目）、useHandlerItems（处理器条目）。

import { useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { SignalPickerItem } from "./SignalSlashPicker";

export { useHandlerItems } from "./useHandlerItems";

/** HTML-like tags that get PropsSection + StylesSection */
export const BASIC_TAGS = new Set([
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
export function useSignalItems(): SignalPickerItem[] {
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
    // storeReferences 方向 = store → [components]，读方向 = entries 过滤（与 flow-scope-hook 同源）
    const referencedStores = Object.entries(storeReferences)
      .filter(([, comps]) => comps.includes(compName))
      .map(([storeName]) => storeName);
    for (const storeName of referencedStores) {
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
