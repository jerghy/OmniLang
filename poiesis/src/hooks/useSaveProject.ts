// hooks/useSaveProject.ts
// Hook that provides the save-project logic: generates .omni files and writes them to disk.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../store";
import { api } from "../commands";
import { generateOmniFiles } from "../services/codegen/pipeline";
import { emitStoreDefinition } from "../services/codegen/source-emitter";
import { validateSources } from "../services/monaco/diagnostics";
import type { CodeTab } from "../core/types/codegraph";
import type { StoreDefinition } from "../core/types/store";
import type { CodeblockElementData } from "../core/types/element";
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

/**
 * Collect all known project .omni sources for cross-file WASM validation.
 * Reads existing files from disk, then overlays generated content on top.
 */
async function collectProjectSources(
  generatedFiles: Record<string, string>,
  extraSources?: Array<{ path: string; content: string }>,
): Promise<Record<string, string>> {
  const { usePoiesisStore } = await import("../store");
  const store = usePoiesisStore.getState();
  const sources: Record<string, string> = {};

  // Gather known .omni file paths from all available sources
  const knownPaths = new Set<string>();

  // 1. Open tabs
  for (const tab of store.designPage.openTabs) {
    if (tab.filePath) knownPaths.add(tab.filePath);
  }

  // 2. Component defs (parsed from .omni, have known file paths)
  for (const def of Object.values(store.componentDefs)) {
    if (def.filePath) knownPaths.add(def.filePath);
  }

  // 3. Generated files
  for (const path of Object.keys(generatedFiles)) {
    knownPaths.add(path);
  }

  // 4. Common paths that might not be in tabs/defs yet
  knownPaths.add("src/design.omni");
  knownPaths.add("src/main.omni");

  // Read existing files from disk
  const fileResults = await Promise.all([...knownPaths].map(path =>
    api.project.readFile(path).then(res => ({ path, res })).catch(() => ({ path, res: null }))
  ));
  for (const { path, res } of fileResults) {
    if (res?.content && !res.error) {
      sources[path] = res.content;
    }
  }

  // Overlay generated files (they take precedence over on-disk content)
  for (const [path, content] of Object.entries(generatedFiles)) {
    sources[path] = content;
  }

  // Overlay extra sources (e.g., freshly generated design source)
  if (extraSources) {
    for (const { path, content } of extraSources) {
      sources[path] = content;
    }
  }

  return sources;
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
      let designSource: string | null = null;
      let designFilePath: string | null = null;
      if (hasDesignContent) {
        const { emitDesignSource } = await import("../services/codegen/design-emitter");
        designSource = emitDesignSource(store.activeDesign);
        designFilePath = activeTab?.tabType === "design" && activeTab?.filePath
          ? activeTab.filePath
          : "src/design.omni";

        // Validate through WASM analyse before writing
        const designErrors = await validateSources({ [designFilePath]: designSource });
        const designCriticals = designErrors.filter((e) => e.severity === "error");
        if (designCriticals.length > 0) {
          setSaveError(formatDiagnostics(designCriticals));
          setSaving(false);
          return;
        }

        await api.project.writeFile(designFilePath, designSource);
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
          // Sync the working code flow (the selected action's canvas) into the
          // action def. StorePage only writes elements/wires back on action
          // switch, so edits made without switching would otherwise be lost.
          const tabData = store.tabData[activeTab.id];
          const selectedActionPid = tabData?.selectedActionPid;
          if (selectedActionPid) {
            const codeblockElements = store.elements.filter(
              (e): e is CodeblockElementData => e.nodeType === "codeblock",
            );
            const action = storeDef.actions.find((a) => a.pid === selectedActionPid);
            if (action && (codeblockElements.length > 0 || store.wires.length > 0)) {
              store.updateStoreAction(activeTab.componentName, selectedActionPid, {
                elements: codeblockElements,
                wires: store.wires,
              });
              storeDef.actions = storeDef.actions.map((a) =>
                a.pid === selectedActionPid ? { ...a, elements: codeblockElements, wires: store.wires } : a,
              );
            }
          }
          source = emitStoreDefinition(storeDef);
        } else {
          // New store with no members yet — write minimal stub
          source = `store ${activeTab.componentName} {\n}\n`;
        }

        // Validate through WASM analyse before writing (include all project sources for imports)
        const storeSources = await collectProjectSources({ [activeTab.filePath]: source });
        const storeErrors = await validateSources(storeSources);
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
      const inlineStores: Array<unknown> = [];
      for (const [key, def] of Object.entries(store.inlineStoreDefs)) {
        if (key.startsWith(`${componentName}.`)) {
          inlineStores.push(def);
        }
      }

      const genResult = generateOmniFiles({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName,
        rootOrder: store.rootOrder,
        // forSave removed — pipeline always emits clean code
        filePath: activeTab?.filePath,
        inlineStores: inlineStores as StoreDefinition[], // NEW
        designName: store.activeDesign?.name && (store.activeDesign.tokens?.length || store.activeDesign.classes?.length || store.activeDesign.variants?.length) ? store.activeDesign.name : undefined,
      });

      if (genResult.value) {
        // ── 3. Validate generated sources through WASM analyse ──
        // Collect all project .omni files so the WASM can resolve cross-file imports
        const extraSources: Array<{ path: string; content: string }> = [];
        if (designSource && designFilePath) {
          extraSources.push({ path: designFilePath, content: designSource });
        }
        const allSources = await collectProjectSources(genResult.value, extraSources);
        const analyseErrors = await validateSources(allSources);
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
      // Preserve panel-managed props/signals/computeds/events/fns that were
      // edited through ComponentMembersPanel (or parsed from .omni on load).
      if (componentName !== "main") {
        const currentDef = store.componentDefs[componentName] || {};
        store.setComponentDef(componentName, {
          ...currentDef,
          name: componentName,
          filePath: `src/components/${componentName}.omni`,
          signals: currentDef.signals || [],
          props: currentDef.props || [],
          computeds: currentDef.computeds || [],
          events: currentDef.events || [],
          fns: currentDef.fns || [],
          elements: store.elements,
          wires: store.wires,
          rootOrder: store.rootOrder,
        });
      } else {
        // Also ensure main component's panel data is preserved
        const currentDef = store.componentDefs["main"] || {};
        store.setComponentDef("main", {
          ...currentDef,
          name: "main",
          filePath: "src/main.omni",
          signals: currentDef.signals || [],
          props: currentDef.props || [],
          computeds: currentDef.computeds || [],
          events: currentDef.events || [],
          fns: currentDef.fns || [],
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
