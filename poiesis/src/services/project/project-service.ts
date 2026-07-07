// @ts-nocheck — Legacy service, needs full migration to unified store API
// services/project/project-service.ts
// Project save logic — extracted 1:1 from App.tsx saveProject + saveFileEntry.

import { generateOmniSource } from "../codegen";
import { api } from "../../commands";
import { extractComponentName } from "../../core/utils";
import { usePoiesisStore } from "../../store";
import { useCodeGraphStore } from "../../services/codegraph/store";
import { useComponentRegistry } from "../../store/componentRegistry";
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
  // Main canvas always uses "App" as registry key; file tabs use their actual component name.
  const isMainCanvas = store.activeDocId === null || store.activeDocId === "main";
  const componentName = isMainCanvas
    ? "App"
    : extractComponentName(store.activeFileContent || store.sourceCode || "");

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
    content = store.sourceCode || store.activeFileContent || "";
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

  const cg = useCodeGraphStore.getState();
  const codeTabs = cg.toSerializable();

  // ── Collect all file states that need saving ──

  const entries: FileSaveEntry[] = [];
  const seenPaths = new Set<string>();

  // 1. Active state: main canvas or active file tab
  if (store.activeDocId === null || store.activeDocId === "main") {
    const mainPath = filePath;
    if (mainPath && !seenPaths.has(mainPath)) {
      entries.push({
        filePath: mainPath,
        elements: store.elements,
        wires: store.wires,
        rootOrder: store.rootOrder,
      });
      seenPaths.add(mainPath);
    }
  } else if (store.activeDocId.startsWith("file:")) {
    const path = store.activeDocId.replace("file:", "");
    if (!seenPaths.has(path)) {
      entries.push({
        filePath: path,
        elements: store.elements,
        wires: store.wires,
        rootOrder: store.rootOrder,
      });
      seenPaths.add(path);
    }
  }

  // 2. Component canvas: main canvas state is in docBackup
  if (store.activeDocId?.startsWith("comp:") && store.docBackup) {
    const mainPath = filePath;
    if (mainPath && !seenPaths.has(mainPath)) {
      entries.push({
        filePath: mainPath,
        elements: store.docBackup.elements,
        wires: store.docBackup.wires,
        rootOrder: store.docBackup.elements
          .filter((e: { parentId: string | null }) => !e.parentId)
          .map((e: { id: string }) => e.id),
      });
      seenPaths.add(mainPath);
    }
  }

  // 3. Other file-based tabs from tabStates
  for (const [docId, state] of Object.entries(store.tabStates)) {
    if (docId.startsWith("file:")) {
      const path = docId.replace("file:", "");
      if (!seenPaths.has(path)) {
        entries.push({
          filePath: path,
          elements: (state as { elements: ElementData[] }).elements,
          wires: (state as { wires: WireData[] }).wires,
          rootOrder: (state as { elements: ElementData[] }).elements
            .filter((e: { parentId: string | null }) => !e.parentId)
            .map((e: { id: string }) => e.id),
        });
        seenPaths.add(path);
      }
    }
  }

  // ── Save each file ──

  let savedCount = 0;
  let activeContent = "";
  for (const entry of entries) {
    const ok = await saveFileEntry(entry, codeTabs, statusMessage);
    if (ok) savedCount++;
    // Track active file's content so we can update in-memory sourceCode
    if (entry.filePath === filePath) {
      const genResult = generateOmniSource({
        designElements: entry.elements,
        designWires: entry.wires,
        codeTabs: codeTabs.tabs as CodeTab[],
        componentName: (store.activeDocId === null || store.activeDocId === "main")
          ? "App"
          : extractComponentName(
              store.activeFileContent || store.sourceCode || "",
            ),
        rootOrder: entry.rootOrder,
        forSave: true,
      });
      if (genResult.value) activeContent = genResult.value;
    }
  }
  if (activeContent) store.setSourceCode(activeContent);

  // 2. Save component definitions (as omni DSL to src/components/*.omni)
  const { definitions } = useComponentRegistry.getState();
  saveComponentDefinitionsToServer(definitions).catch((e) => log.error("Failed to save component definitions:", e));

  await api.annotation.saveEditorToml({
    active_file: filePath,
    active_mode: "design",
    open_files:
      store.openTabs.length > 0
        ? store.openTabs
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

