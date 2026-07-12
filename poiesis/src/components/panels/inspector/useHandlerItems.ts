// poiesis/src/components/panels/inspector/useHandlerItems.ts
// Collects available functions and store actions for the active component
// to populate the event handler picker.

import { useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { FnDef } from "../../../core/types/project-json";
import type { StoreActionDef } from "../../../core/types/store";

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

  return useMemo(() => {
    const activeTab = activeTabId
      ? openTabs.find((t) => t.id === activeTabId)
      : undefined;
    const compName = activeTab?.componentName;
    if (!compName) return [];

    const items: HandlerItem[] = [];
    const compDef = componentDefs[compName];

    // Component functions
    if (compDef?.fns) {
      for (const fn of compDef.fns) {
        items.push(formatFnItem(fn, compName, "component"));
      }
    }

    // Store actions from referenced stores
    const refs = storeReferences[compName] ?? [];
    for (const storeName of refs) {
      const storeDef = storeDefs[storeName];
      if (storeDef?.actions) {
        for (const action of storeDef.actions) {
          items.push(formatActionItem(action, storeName));
        }
      }
    }

    return items;
  }, [activeTabId, openTabs, componentDefs, storeDefs, storeReferences]);
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
