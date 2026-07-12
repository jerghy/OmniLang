// hooks/useSaveProject.ts
// Hook that provides the save-project logic: generates .omni files and writes them to disk.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../store";
import { api } from "../commands";
import { generateOmniFiles } from "../services/codegen/pipeline";
import { emitStoreDefinition } from "../services/codegen/source-emitter";
import { validateSources } from "../services/monaco/diagnostics";
import type { CodeTab } from "../core/types/codegraph";
import { createLogger } from "../lib/utils/logger";

const log = createLogger("save");

/** Format WASM diagnostics into a human-readable error string. */
function formatDiagnostics(
  errors: Array<{ severity: string; file?: string | null; line: number; col: number; message: string; code: string }>,
): string {
  const lines = errors.map((e) => {
    const loc = e.file ? `${e.file}:` : "";
    return `  ${loc}${e.line}:${e.col} — ${e.message} [${e.code}]`;
  });
  return `OmniLang analyse found ${errors.length} error(s):\n${lines.join("\n")}`;
}

export function useSaveProject() {
  const markClean = usePoiesisStore((s) => s.markClean);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const handleSave = useCallback(async () => {
    setSaveError(null);
    setSaving(true);
    try {
      const store = usePoiesisStore.getState();
      const activeTabId = store.designPage.activeTabId;
      const activeTab = activeTabId
        ? store.designPage.openTabs.find((t) => t.id === activeTabId)
        : null;

      // ── 0. Always save design.omni if design state is non-empty ──
      // Design state (activeDesign) is global, not per-tab, so we must persist
      // it regardless of which tab is currently active.
      const hasDesignContent =
        store.activeDesign?.tokens?.length ||
        store.activeDesign?.classes?.length ||
        store.activeDesign?.variants?.length;
      if (hasDesignContent) {
        const { emitDesignSource } = await import("../services/codegen/design-emitter");
        const source = emitDesignSource(store.activeDesign);
        const designFilePath = activeTab?.tabType === "design" && activeTab?.filePath
          ? activeTab.filePath
          : "src/design.omni";

        // Validate through WASM analyse before writing
        const designErrors = await validateSources({ [designFilePath]: source });
        const designCriticals = designErrors.filter((e) => e.severity === "error");
        if (designCriticals.length > 0) {
          setSaveError(formatDiagnostics(designCriticals));
          setSaving(false);
          return;
        }

        await api.project.writeFile(designFilePath, source);
      }

      // If we're on the design tab, design.omni save above is sufficient.
      // Skip component file generation (elements/wires are from a different tab).
      if (activeTab?.tabType === "design") {
        const result = await api.annotation.saveProject();
        if (result.ok) markClean();
        return;
      }

      // ── Store tab: emit StoreDefinition from storeDefs ──
      if (activeTab?.tabType === "store") {
        const storeDef = store.storeDefs[activeTab.componentName];
        let source: string;
        if (storeDef) {
          source = emitStoreDefinition(storeDef);
        } else {
          // New store with no members yet — write minimal stub
          source = `store ${activeTab.componentName} {\n}\n`;
        }

        // Validate through WASM analyse before writing
        const storeErrors = await validateSources({ [activeTab.filePath]: source });
        const storeCriticals = storeErrors.filter((e) => e.severity === "error");
        if (storeCriticals.length > 0) {
          setSaveError(formatDiagnostics(storeCriticals));
          setSaving(false);
          return;
        }

        await api.project.writeFile(activeTab.filePath, source);
        const result = await api.annotation.saveProject();
        if (result.ok) markClean();
        return;
      }

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

      // Collect inline store definitions associated with the current component
      const inlineStores = Object.entries(store.inlineStoreDefs)
        .filter(([key]) => key.startsWith(`${componentName}.`))
        .map(([, def]) => def);

      const genResult = generateOmniFiles({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName,
        rootOrder: store.rootOrder,
        // forSave removed — pipeline always emits clean code
        filePath: activeTab?.filePath,
        inlineStores, // NEW
        designName: store.activeDesign?.name && (store.activeDesign.tokens?.length || store.activeDesign.classes?.length || store.activeDesign.variants?.length) ? store.activeDesign.name : undefined,
      });

      if (genResult.value) {
        // ── 3. Validate generated sources through WASM analyse ──
        const analyseErrors = await validateSources(genResult.value);
        const criticalErrors = analyseErrors.filter((e) => e.severity === "error");
        if (criticalErrors.length > 0) {
          setSaveError(formatDiagnostics(criticalErrors));
          log.error("WASM analyse errors:", criticalErrors);
          setSaving(false);
          return;
        }

        // ── 4. Write each generated .omni file to disk ──
        for (const [filePath, source] of Object.entries(genResult.value)) {
          await api.project.writeFile(filePath, source);
        }
        // Update activeFileContent so the code drawer shows the latest content
        const targetPath = activeTab?.filePath ?? "src/main.omni";
        if (genResult.value[targetPath]) {
          usePoiesisStore.setState({ activeFileContent: genResult.value[targetPath] });
        }
      } else if (genResult.errors.some((e) => e.severity === "fatal")) {
        log.error("codegen errors:", genResult.errors);
        return;
      }

      // ── 5. Save project.json (editor metadata) ──
      const result = await api.annotation.saveProject();
      if (result.ok) {
        markClean();
      }

      // ── 5. Update custom component def with designed elements ──
      if (componentName !== "main") {
        const currentDef = store.componentDefs[componentName] || {};
        store.setComponentDef(componentName, {
          ...currentDef,
          name: componentName,
          filePath: `src/components/${componentName}.omni`,
          signals: [],
          props: [],
          events: [],
          fns: [],
          elements: store.elements,
          wires: store.wires,
          rootOrder: store.rootOrder,
        });
      }
    } catch (e) {
      log.error("failed:", e);
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  }, [markClean]);

  return { saving, saveError, handleSave };
}
