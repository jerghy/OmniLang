// services/project/project-service.ts
// Project save logic — extracted 1:1 from App.tsx saveProject + saveFileEntry.

import { generateOmniSource } from "../codegen";
import { api } from "../../commands";
import { usePoiesisStore } from "../../store";
import { useCodeGraphStore } from "../../services/codegraph/store";
import { getComponentDefinitions } from "../../store";
import type { ElementData, WireData, CodeTab } from "../../core/types";
import { saveComponentDefinitionsToServer } from "./component-service";
import { createLogger } from "../../lib/utils/logger";

const log = createLogger("project");

export interface FileSaveEntry {
  filePath: string;
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
}

/**
 * Save a single file entry: generate .omni source from elements/wires and write
 * to disk.  Returns `true` on success, `false` on failure.
 */
export async function saveFileEntry(
  entry: FileSaveEntry,
  codeTabs: { tabs: unknown[] },
  statusMessage?: (text: string, isError: boolean) => void,
): Promise<boolean> {
  const store = usePoiesisStore.getState();
  // Derive component name from the matching open tab, or fall back to "App" for the main canvas.
  const tab = store.designPage.openTabs.find((t) => t.filePath === entry.filePath);
  const componentName = tab?.componentName ?? "App";

  const genResult = generateOmniSource({
    designElements: entry.elements,
    designWires: entry.wires,
    codeTabs: codeTabs.tabs as CodeTab[],
    componentName,
    rootOrder: entry.rootOrder,
    forSave: true,
  });

  let content: string;
  if (genResult.value) {
    content = genResult.value;
  } else {
    content = store.activeFileContent || "";
    if (!content) return false;
  }

  const writeRes = await api.project.writeFile(entry.filePath, content);
  if (!writeRes.ok) {
    statusMessage?.(
      `⚠ ${entry.filePath} 写入失败: ${writeRes.error || "未知错误"}`,
      false,
    );
    return false;
  }
  return true;
}

/**
 * Save all project state — active canvas, component canvases, tab states,
 * component definitions, and editor TOML state.
 */
export async function saveProject(
  customPath?: string,
  statusMessage?: (text: string, isError: boolean) => void,
): Promise<void> {
  const store = usePoiesisStore.getState();
  const filePath = customPath || store.activeFilePath;
  const projectDir = store.projectDir;

  if (!filePath) {
    statusMessage?.("请先打开或创建一个 .omni 文件", true);
    return;
  }

  statusMessage?.(`保存到 ${filePath} ...`, false);

  const cg = useCodeGraphStore.getState() as unknown as import("../../services/codegraph/store").CodeGraphStoreState;
  const codeTabs = cg.toSerializable();

  // ── Collect all file states that need saving ──

  const entries: FileSaveEntry[] = [];
  const seenPaths = new Set<string>();

  // 1. Active state: current elements/wires belong to the active file
  if (store.activeFilePath && !seenPaths.has(store.activeFilePath)) {
    entries.push({
      filePath: store.activeFilePath,
      elements: store.elements,
      wires: store.wires,
      rootOrder: store.rootOrder,
    });
    seenPaths.add(store.activeFilePath);
  }

  // 2. Other open tabs' data from tabData (component/file tabs)
  const activeTabId = store.designPage.activeTabId;
  for (const tab of store.designPage.openTabs) {
    if (tab.filePath && !seenPaths.has(tab.filePath)) {
      // Skip active tab — already saved above from live state
      if (tab.id === activeTabId) continue;
      const tabState = store.tabData[tab.id];
      if (tabState) {
        entries.push({
          filePath: tab.filePath,
          elements: tabState.elements || [],
          wires: tabState.wires || [],
          rootOrder: tabState.rootOrder || [],
        });
        seenPaths.add(tab.filePath);
      }
    }
  }

  // ── Save each file ──

  let savedCount = 0;
  let activeContent = "";
  const activeTab = activeTabId
    ? store.designPage.openTabs.find((t) => t.id === activeTabId)
    : null;
  for (const entry of entries) {
    const ok = await saveFileEntry(entry, codeTabs, statusMessage);
    if (ok) savedCount++;
    // Track active file's content so we can update in-memory activeFileContent
    if (entry.filePath === filePath) {
      const genResult = generateOmniSource({
        designElements: entry.elements,
        designWires: entry.wires,
        codeTabs: codeTabs.tabs as CodeTab[],
        componentName: activeTab?.componentName ?? "App",
        rootOrder: entry.rootOrder,
        forSave: true,
      });
      if (genResult.value) activeContent = genResult.value;
    }
  }
  if (activeContent) store.setActiveFileContent(activeContent);

  // 2. Save component definitions (as omni DSL to src/components/*.omni)
  const defs = getComponentDefinitions();
  saveComponentDefinitionsToServer(defs).catch((e) => log.error("Failed to save component definitions:", e));

  await api.annotation.saveEditorToml({
    active_file: filePath,
    active_mode: "design",
    open_files:
      store.designPage.openTabs.length > 0
        ? store.designPage.openTabs.map(t => t.filePath)
        : filePath
          ? [filePath]
          : [],
    left_open: true,
    right_width: 320,
    viewports: {},
  });

  store.markClean();
  usePoiesisStore.setState({
    projectName: filePath.replace(/^.*[\\/]/, "").replace(/\.omni$/, ""),
  });
  const fullPath = projectDir
    ? `${projectDir}/${filePath}`
    : filePath;
  statusMessage?.(`✅ 已保存 ${savedCount} 个文件: ${fullPath}`, false);
}

