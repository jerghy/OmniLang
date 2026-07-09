// hooks/useLiveCodePreview.ts
// Generates live .omni source from the current editor state for the code drawer.
// Runs the codegen pipeline on every relevant state change (debounced 300ms).

import { useMemo } from "react";
import { usePoiesisStore } from "../store";
import { generateOmniFiles } from "../services/codegen/pipeline";
import type { CodeTab } from "../core/types/codegraph";

/**
 * Returns the live-generated .omni source for the active component tab.
 * Re-generates whenever elements, wires, rootOrder, activeDesign, or
 * componentDefs change — so class binding, inline style, and design
 * data changes all reflect immediately in the code drawer.
 */
export function useLiveCodePreview(): string | null {
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const rootOrder = usePoiesisStore((s) => s.rootOrder);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);

  return useMemo(() => {
    if (elements.length === 0) return null;

    const activeTab = activeTabId
      ? openTabs.find((t) => t.id === activeTabId)
      : null;
    if (!activeTab || activeTab.tabType !== "component") return null;

    const codeTabs: CodeTab[] = [
      {
        id: "main",
        type: "logic",
        label: "Main",
        icon: "📄",
        graph: { elements, wires },
        variables: [],
      },
    ];

    try {
      const genResult = generateOmniFiles({
        designElements: elements,
        designWires: wires,
        codeTabs,
        componentName: activeTab.componentName ?? "main",
        rootOrder,
        forSave: false,
        filePath: activeTab.filePath,
        designName: activeDesign?.name && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length) ? activeDesign.name : undefined,
      });

      if (genResult.value) {
        const targetPath = activeTab.filePath ?? "src/main.omni";
        return genResult.value[targetPath] ?? Object.values(genResult.value)[0] ?? null;
      }
      return null;
    } catch {
      return null;
    }
  }, [elements, wires, rootOrder, activeDesign, componentDefs, activeTabId, openTabs]);
}
