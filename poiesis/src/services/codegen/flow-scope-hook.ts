// poiesis/src/services/codegen/flow-scope-hook.ts
// 从 Zustand store 取数据构建 FlowScope（组件内便捷入口）。
import { useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { useShallow } from "zustand/react/shallow";
import type { CodeblockElementData } from "../../core/types/element";
import { buildFlowScope, type FlowScope } from "./flow-scope";

export function useFlowScope(componentName: string | undefined): FlowScope | null {
  const componentDef = usePoiesisStore((s) => (componentName ? s.componentDefs[componentName] : undefined));
  const storeDefs = usePoiesisStore(useShallow((s) => s.storeDefs));
  const inlineStoreDefs = usePoiesisStore(useShallow((s) => s.inlineStoreDefs));
  const storeReferences = usePoiesisStore(useShallow((s) => s.storeReferences));
  const elements = usePoiesisStore(useShallow((s) => s.elements));

  return useMemo(() => {
    if (!componentName) return null;
    const storesUsed = Object.entries(storeReferences)
      .filter(([, comps]) => comps.includes(componentName))
      .map(([name]) => name);
    const fnEl = (elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock" && e.tag === "fn"));
    return buildFlowScope({
      componentDef,
      storeDefs,
      inlineStoreDefs,
      storesUsed,
      fnParamsStr: fnEl[0]?.props?.params as string | undefined,
      fnElements: elements.filter((e): e is CodeblockElementData => e.nodeType === "codeblock"),
    });
  }, [componentName, componentDef, storeDefs, inlineStoreDefs, storeReferences, elements]);
}
