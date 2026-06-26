// services/project/project-service.ts
// Project save logic — extracted 1:1 from App.tsx saveProject + saveFileEntry.

import { generateOmniSource } from "../codegen";
import { api } from "../api";
import { extractComponentName } from "../../core/utils";
import { useEditorStore } from "../../store/editor";
import { useComponentRegistry } from "../../store/componentRegistry";
import { useCodeGraphStore } from "../../codegraph/store";
import type { ElementData, WireData } from "../../core/types";

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
  const store = useEditorStore.getState();
  const componentName = extractComponentName(
    store.activeFileContent || store.sourceCode || "",
  );

  const genResult = generateOmniSource({
    designElements: entry.elements,
    designWires: entry.wires,
    codeTabs: codeTabs.tabs,
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
  const store = useEditorStore.getState();
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
        codeTabs: codeTabs.tabs,
        componentName: extractComponentName(
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
  saveComponentDefinitionsToServer(definitions).catch(() => {});

  // 3. Save editor UI state to .omni-editor.toml
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
  useEditorStore.setState({
    projectName: filePath.replace(/^.*[\\/]/, "").replace(/\.omni$/, ""),
  });
  const fullPath = projectDir
    ? `${projectDir}/${filePath}`
    : filePath;
  statusMessage?.(`✅ 已保存 ${savedCount} 个文件: ${fullPath}`, false);
}

/**
 * Convert registry definitions to omni DSL and save to server.
 * Each component's elements/wires are emitted as proper .omni source
 * with metadata stored as // @component / @prop / @event / @style comments
 * (no more .meta.json).
 */
export async function saveComponentDefinitionsToServer(
  defs: Record<string, { id?: string; category?: string; description?: string; props?: unknown; events?: unknown; styleProperties?: string[]; children?: boolean; createdAt?: string; elements?: ElementData[]; wires?: WireData[] }>,
): Promise<void> {
  const payload: Record<string, { omniSource: string }> = {};

  for (const [name, def] of Object.entries(defs)) {
    const result = generateOmniSource({
      designElements: def.elements || [],
      designWires: def.wires || [],
      codeTabs: [],
      componentName: name,
      rootOrder: (def.elements || [])
        .filter((e: { parentId?: string | null }) => !e.parentId)
        .map((e: { id: string }) => e.id),
      forSave: true,
      componentMeta: {
        id: def.id,
        category: def.category,
        description: def.description,
        props: def.props as Record<string, unknown>,
        events: def.events as Record<string, unknown>,
        styleProperties: def.styleProperties,
        children: def.children,
        createdAt: def.createdAt,
      },
    });

    if (result.value) {
      payload[name] = { omniSource: result.value };
    }
  }

  if (Object.keys(payload).length > 0) {
    await api.annotation.saveComponentDefinitions(
      payload as unknown as Record<string, unknown>,
    );
  }
}

/**
 * Load custom component definitions from server.
 * Parses .omni source back to elements/wires; metadata is extracted from
 * // @component / @prop / @event / @style comments in the source.
 */
export async function loadComponentDefinitionsFromServer(): Promise<
  Record<string, unknown>
> {
  const { parseOmniSource } = await import("../codegen");
  const result = await api.annotation.loadComponentDefinitions();
  const merged: Record<string, unknown> = {};

  for (const [name, raw] of Object.entries(result.definitions)) {
    const data = raw as Record<string, unknown>;
    const source = (data.source as string) || "";
    if (!source) continue;

    const parsed = parseOmniSource(source);
    const meta = parsed.componentMeta || {};

    // Elements/wires come from parsed view tree
    const elements =
      parsed.designElements && parsed.designElements.length > 0
        ? parsed.designElements
        : [];
    const wires = parsed.designWires || [];

    // Props schema from comment metadata (// @prop), fallback to DSL declarations
    const propSchema: Record<string, unknown> =
      (meta.propSchema as Record<string, unknown>) || {};
    const dslProps = parsed.props || [];
    for (const p of dslProps) {
      if (!propSchema[p.name]) {
        propSchema[p.name] = {
          type: p.type,
          default: p.default,
          editor: "text",
        };
      }
    }

    // Events schema from comment metadata (// @event), fallback to DSL declarations
    const eventSchema: Record<string, unknown> =
      (meta.eventSchema as Record<string, unknown>) || {};
    const dslEvents = parsed.events || [];
    for (const e of dslEvents) {
      if (!eventSchema[e.name]) {
        eventSchema[e.name] = { id: e.name, description: "", payload: null };
      }
    }

    merged[name] = {
      id: (meta.id as string) || name,
      name,
      category: (meta.category as string) || "Custom",
      description: (meta.description as string) || "",
      props: propSchema,
      events: eventSchema,
      styleProperties: (meta.styleProperties as string[]) || [],
      children: (meta.children as boolean) ?? true,
      elements,
      wires,
      createdAt: (meta.createdAt as string) || new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      storage: "file",
    };
  }

  return merged;
}
