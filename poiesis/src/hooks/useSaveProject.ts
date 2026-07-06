// hooks/useSaveProject.ts
// Hook that provides the save-project logic: generates .omni files and writes them to disk.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../store";
import { api } from "../commands";
import { generateOmniFiles } from "../services/codegen/pipeline";
import type { CodeTab } from "../core/types/codegraph";

export function useSaveProject() {
  const markClean = usePoiesisStore((s) => s.markClean);
  const [saving, setSaving] = useState(false);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const store = usePoiesisStore.getState();
      const activeTabId = store.designPage.activeTabId;
      const activeTab = activeTabId
        ? store.designPage.openTabs.find((t) => t.id === activeTabId)
        : null;

      // ── 1. Build codeTabs from current elements+wires ──
      const codeTabs: CodeTab[] = [
        {
          id: "main",
          type: "logic",
          label: "Main",
          icon: "📄",
          graph: { elements: store.elements, wires: store.wires },
          variables: [],
        },
      ];

      // ── 2. Generate .omni source via codegen pipeline ──
      const componentName = activeTab?.componentName ?? "main";
      const genResult = generateOmniFiles({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName,
        rootOrder: store.rootOrder,
        forSave: true,
        filePath: activeTab?.filePath,
      });

      if (genResult.value) {
        // ── 3. Write each generated .omni file to disk ──
        for (const [filePath, source] of Object.entries(genResult.value)) {
          await api.project.writeFile(filePath, source);
        }
      } else if (genResult.errors.some((e) => e.severity === "fatal")) {
        console.error("[save] codegen errors:", genResult.errors);
        return;
      }

      // ── 4. Save project.json (editor metadata) ──
      const result = await api.annotation.saveProject();
      if (result.ok) {
        markClean();
      }

      // ── 5. Update custom component def with designed elements ──
      if (componentName !== "main") {
        const currentDef = store.componentDefs[componentName] || {};
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (store.setComponentDef as any)(componentName, {
          ...currentDef,
          elements: store.elements,
          wires: store.wires,
          rootOrder: store.rootOrder,
        });
      }
    } catch (e) {
      console.error("[save] failed:", e);
    } finally {
      setSaving(false);
    }
  }, [markClean]);

  return { saving, handleSave };
}
