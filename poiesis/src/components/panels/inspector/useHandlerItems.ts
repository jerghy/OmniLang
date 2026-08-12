// poiesis/src/components/panels/inspector/useHandlerItems.ts
// Collects available functions and store actions for the active component
// to populate the event handler picker.

import { useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { FnDef } from "../../../core/types/project-json";
import type { StoreActionDef } from "../../../core/types/store";
import type { CodeblockElementData } from "../../../core/types/element";
import { mergeComponentFns } from "../../../services/codegen/flow-scope";

export interface HandlerItem {
  pid: string;
  name: string;
  displayName: string;
  params: Array<{ name: string; type: string }>;
  source: "component" | "store";
  sourceName: string;
  category: "fn" | "action";
}

export function useHandlerItems(): HandlerItem[] {
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);
  const elements = usePoiesisStore((s) => s.elements);

  return useMemo(() => {
    const activeTab = activeTabId
      ? openTabs.find((t) => t.id === activeTabId)
      : undefined;
    const compName = activeTab?.componentName;
    if (!compName) return [];

    const items: HandlerItem[] = [];
    const compDef = componentDefs[compName];

    // Component functions — 组件 tab 以画布 fn 元素为权威（CodeFlow 新建/改名/删除的 fn
    // 只体现在 elements，componentDefs.fns 是加载时快照），与快照合并去重；
    // 其余 tab（store/design）保持原快照行为
    const codeElements = elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock");
    const fns = activeTab?.tabType === "component"
      ? mergeComponentFns(compDef?.fns, codeElements)
      : (compDef?.fns ?? []);
    for (const fn of fns) {
      items.push(formatFnItem(fn, compName, "component"));
    }

    // Store actions from referenced stores
    // storeReferences 方向 = store → [components]，读方向 = entries 过滤（与 flow-scope-hook 同源）
    const referencedStores = Object.entries(storeReferences)
      .filter(([, comps]) => comps.includes(compName))
      .map(([storeName]) => storeName);
    for (const storeName of referencedStores) {
      const storeDef = storeDefs[storeName];
      if (storeDef?.actions) {
        for (const action of storeDef.actions) {
          items.push(formatActionItem(action, storeName));
        }
      }
    }

    return items;
  }, [activeTabId, openTabs, componentDefs, storeDefs, storeReferences, elements]);
}

function formatFnItem(fn: FnDef, compName: string, source: "component" | "store"): HandlerItem {
  const paramsStr = fn.params.map((p) => `${p.name}: ${p.type}`).join(", ");
  return {
    pid: fn.pid,
    name: fn.name,
    displayName: `${fn.name}(${paramsStr})`,
    params: fn.params,
    source,
    sourceName: compName,
    category: "fn",
  };
}

function formatActionItem(action: StoreActionDef, storeName: string): HandlerItem {
  const paramsStr = action.params.map((p) => `${p.name}: ${p.type}`).join(", ");
  return {
    pid: action.pid,
    name: action.name,
    displayName: `${storeName}.${action.name}(${paramsStr})`,
    params: action.params,
    source: "store",
    sourceName: storeName,
    category: "action",
  };
}
