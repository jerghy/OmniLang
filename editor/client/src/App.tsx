import React, { useEffect, useCallback, useState, useRef, useMemo } from "react";
import { useEditorStore } from "./store/editor";
import { useUndoStore } from "./store/undo";
import { api } from "./api/client";
import { OmniDriver } from "./driver";
import { FileExplorer } from "./panels/FileExplorer";
import { BlockPalette } from "./panels/BlockPalette";
import { PropertyPanel } from "./panels/PropertyPanel";
import { CodePropertyPanel } from "./panels/CodePropertyPanel";
import { ComponentPalette } from "./panels/ComponentPalette";
import { ConnectorPanel } from "./panels/ConnectorPanel";
import { TokenPanel, useTokenStore } from "./panels/TokenPanel";
import { StatePanel } from "./panels/StatePanel";
import { FunctionLibrary } from "./panels/FunctionLibrary";
import { ActivityBar, PanelContainer, DESIGN_PANELS, type PanelId } from "./panels/ActivityBar";
import { CanvasPanel } from "./panels/CanvasPanel";
import { FlowCanvas } from "./canvas/FlowCanvas";
import { LayoutView } from "./canvas/LayoutView";
import { CanvasTabBar } from "./canvas/CanvasTabBar";
import { CodeTabBar } from "./canvas/CodeTabBar";
import { useCodeGraphStore } from "./codegraph/store";
import { createMainTab } from "./codegraph/types";
import { migrateOldCodeGraph } from "./codegraph/migration";
import { useComponentRegistry } from "./store/componentRegistry";
import { WireOverlay } from "./canvas/WireOverlay";
import { StateHUD } from "./canvas/StateHUD";
import { PerfBadge } from "./canvas/PerfBadge";
import { DebugPanel } from "./canvas/DebugPanel";
import { MonacoEditor, type MonacoEditorHandle } from "./editor/MonacoEditor";
import { CodeEditorPopup } from "./editor/CodeEditorPopup";
import { SearchModal } from "./search/SearchModal";
import { collectFocusOrder, focusOverlayStyles } from "./a11y/FocusOrder";
import type { SearchResult } from "./search/SearchIndex";
import type { CommandContext } from "./commands/types";
import { copySelection, pasteFromClipboard, cutSelection, duplicateSelection } from "./commands/CopyPaste";
import { showToast } from "./notifications/toast";
import { parseOmniSource, generateOmniSource, generateOmniFiles, validate } from "./omni-gen";

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const S = {
  app: { display: "flex", height: "100vh", fontFamily: "system-ui, sans-serif" } as React.CSSProperties,
  sidebar: { width: 220, minWidth: 220, flexShrink: 0, borderRight: "1px solid #e5e7eb", overflow: "auto", background: "#f9fafb", display: "flex", flexDirection: "column" } as React.CSSProperties,
  main: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column", position: "relative", zIndex: 1 } as React.CSSProperties,
  toolbar: { height: 36, minHeight: 36, borderBottom: "1px solid #e5e7eb", padding: "0 12px", display: "flex", alignItems: "center", gap: 8, fontSize: 12, position: "relative", zIndex: 5 } as React.CSSProperties,
  canvasArea: { flex: 1, overflow: "auto", position: "relative" } as React.CSSProperties,
  codeArea: { flex: 1, overflow: "hidden", display: "flex" } as React.CSSProperties,
  inspector: { width: 280, minWidth: 280, flexShrink: 0, borderLeft: "1px solid #e5e7eb", display: "flex", flexDirection: "column", background: "#f9fafb" } as React.CSSProperties,
  statusBar: { height: 24, minHeight: 24, borderTop: "1px solid #e5e7eb", padding: "0 12px", display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 11, background: "#f9fafb" } as React.CSSProperties,
  modeBtn: (active: boolean) => ({ padding: "3px 14px", fontSize: 12, border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600, background: active ? "#4F46E5" : "#F3F4F6", color: active ? "#fff" : "#374151" } as React.CSSProperties),
};

// ═══════════════════════════════════════════════
// App
// ═══════════════════════════════════════════════

export default function App() {
  const [mode, setMode] = useState<"design" | "code">("design");
  const [wireMode, setWireMode] = useState(false);
  const [codePopup, setCodePopup] = useState<{ visible: boolean; blockId: string | null; title: string; source: string; blockElement: import("./store/editor").ElementData | null }>({ visible: false, blockId: null, title: "", source: "", blockElement: null });
  const [highlightedWireId, setHighlightedWireId] = useState<string | null>(null);
  const [target, setTarget] = useState("react");
  const [status, setStatus] = useState<{ text: string; isError: boolean }>({ text: "就绪", isError: false });
  const [showSnippets, setShowSnippets] = useState(true);
  const [showConnector, setShowConnector] = useState(false);
  const [breakpoint, setBreakpoint] = useState<"desktop" | "tablet" | "mobile">("desktop");
  const [showTokens, setShowTokens] = useState(false);
  const [showHUD, setShowHUD] = useState(false);
  const [previewState, setPreviewState] = useState<string>("normal");  // M7: component state preview
  const [showFocusOrder, setShowFocusOrder] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [showConflict, setShowConflict] = useState(false);
  const [activePanel, setActivePanel] = useState<PanelId>(null);
  const codeEditorRef = useRef<MonacoEditorHandle>(null);
  const [layoutTrigger, setLayoutTrigger] = useState(0);

  // Save/Load state (v2: project-based, no separate load dialog)

  // Run / Vite DevServer state (Phase 1 Runtime Bridge)
  const [runStatus, setRunStatus] = useState<{ running: boolean; url: string | null }>({ running: false, url: null });
  const [runLoading, setRunLoading] = useState(false);

  // Debug state (Phase 2 Runtime Bridge)
  const [debugConnected, setDebugConnected] = useState(false);
  const [debugStateChanges, setDebugStateChanges] = useState<Array<{
    signalId: string; name: string; value: unknown; timestamp: number;
  }>>([]);
  const [debugBreakpoints, setDebugBreakpoints] = useState<Set<string>>(new Set());
  const [activeNodeIds, setActiveNodeIds] = useState<Set<string>>(new Set());
  const [activeEdgeIds, setActiveEdgeIds] = useState<Set<string>>(new Set());
  const [debugPaused, setDebugPaused] = useState(false);
  /** Maps semantic trace IDs ("Component.fnName") → canvas element IDs. */
  const [fnToNodeMap, setFnToNodeMap] = useState<Record<string, string>>({});
  // Ref to avoid stale closure in handleDebugMessage
  const fnToNodeMapRef = useRef(fnToNodeMap);
  fnToNodeMapRef.current = fnToNodeMap;

  const projectDir = useEditorStore((s) => s.projectDir);
  const activeFilePath = useEditorStore((s) => s.activeFilePath);
  const activeFileContent = useEditorStore((s) => s.activeFileContent);
  const setProjectDir = useEditorStore((s) => s.setProjectDir);
  const loadFiles = useEditorStore((s) => s.loadFiles);
  const openFile = useEditorStore((s) => s.openFile);
  const setActiveFileContent = useEditorStore((s) => s.setActiveFileContent);
  const setElementSource = useEditorStore((s) => s.setElementSource);
  const setCompileErrors = useEditorStore((s) => s.setCompileErrors);
  const activeDocId = useEditorStore((s) => s.activeDocId);
  const undoExec = useUndoStore((s) => s.execute);
  const undoUndo = useUndoStore((s) => s.undo);
  const undoRedo = useUndoStore((s) => s.redo);

  const projectName = useEditorStore((s) => s.projectName);
  const projectPath = useEditorStore((s) => s.projectPath);
  const isDirty = useEditorStore((s) => s.isDirty);

  // ── Init ────────────────────────────────────

  useEffect(() => {
    (async () => {
      const proj = await api.getProject();
      if (proj.dir) setProjectDir(proj.dir);
      const list = await api.listDir(".");
      if (list.entries) loadFiles(list.entries);
      // Start with a new unnamed project
      useEditorStore.getState().newProject();
    })();
  }, []);

  // Auto-save component definitions when registry changes
  useEffect(() => {
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    const unsub = useComponentRegistry.subscribe((state) => {
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveTimer = null;
        if (Object.keys(state.definitions).length > 0) {
          api.saveComponentDefinitions(state.definitions as Record<string, unknown>).catch(() => {
            // Best-effort auto-save; failures are non-critical
          });
        }
      }, 2000);
    });
    return () => {
      unsub();
      if (saveTimer) clearTimeout(saveTimer);
    };
  }, []);

  // ── OmniDriver initialization (AI automation API) ──
  useEffect(() => {
    const omni = new OmniDriver();
    (window as any).__OMNI__ = omni;
    (window as any).__OMNI_MODE__ = mode;
    (window as any).__OMNI_TARGET__ = target;

    // Listen for mode changes triggered by OmniDriver.switchMode()
    const onModeChange = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.mode) setMode(detail.mode);
    };
    window.addEventListener("omni-mode-change", onModeChange);
    return () => window.removeEventListener("omni-mode-change", onModeChange);
  }, []);

  // Keep the mode global in sync
  useEffect(() => {
    (window as any).__OMNI_MODE__ = mode;
  }, [mode]);

  // Keep the target global in sync
  useEffect(() => {
    (window as any).__OMNI_TARGET__ = target;
  }, [target]);

  useEffect(() => {
    api.connectWatch(
      (data) => {
        setCompileErrors(data.errors || []);
        const hasErrors = data.errors?.some((e: {severity:string}) => e.severity === "error");
        const warns = data.errors?.filter((e: {severity:string}) => e.severity === "warning").length || 0;
        if (hasErrors) setStatus({ text: "❌ 编译失败", isError: true });
        else if (warns) setStatus({ text: `⚠ 已编译 — ${new Date().toLocaleTimeString()} (${warns} 条警告)`, isError: false });
        else setStatus({ text: `✅ 已编译 — ${new Date().toLocaleTimeString()}`, isError: false });
      },
      (msg, isErr) => setStatus({ text: msg, isError: isErr })
    );
    return () => api.disconnect();
  }, []);

  // ── Compile ─────────────────────────────────

  const handleCompile = useCallback(async () => {
    // ── Generate and validate OmniLang source via omni-gen ──
    const store = useEditorStore.getState();
    const cg = useCodeGraphStore.getState();
    const codeTabs = cg.toSerializable().tabs;
    const compMatch = (store.activeFileContent || "").match(/component\s+(\w+)\s*\{/);
    const componentName = compMatch ? compMatch[1] : "App";

    const genResult = generateOmniSource({
      designElements: store.elements,
      designWires: store.wires,
      codeTabs,
      componentName,
    });

    let content = store.activeFileContent;
    if (genResult.value) {
      const validationResult = validate(genResult.value);
      if (!validationResult.valid) {
        setCompileErrors(validationResult.errors);
        setStatus({ text: "❌ 代码验证失败", isError: true });
        return;
      }
      // Use generated source for compilation and update editor content
      content = genResult.value;
      store.setSourceCode(content);
    }
    if (!content) return;
    setStatus({ text: "编译中...", isError: false });

    const result = await api.compile(content, target, undefined, runStatus.running);
    const errs = result.errors || [];
    setCompileErrors(errs);

    // Parse view tree for code canvas (populate main tab)
    useCodeGraphStore.getState().switchTab("main");
    useCodeGraphStore.getState().setElementsAndWires([], []);
    try {
      const [vRes, cbRes] = await Promise.all([
        fetch("/api/parse-view", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ source: content }) }),
        fetch("/api/parse-codeblocks", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ source: content }) }),
      ]);
      const vData = await vRes.json();
      const cbData = await cbRes.json();

      const codeElements: import("./store/editor").ElementData[] = [];
      const seenIds = new Set<string>();

      if (cbData.success && cbData.blocks) {
        const portDefs: Record<string, Array<{ name: string; direction: "in" | "out"; type: string }>> = {
          store:    [{ name: "items", direction: "out", type: "any" }],
          signal:   [{ name: "value", direction: "out", type: "i32" }, { name: "set", direction: "in", type: "i32" }],
          computed: [{ name: "value", direction: "out", type: "i32" }, { name: "deps", direction: "in", type: "any" }],
        };
        for (const b of cbData.blocks) {
          const parentStore = b.parentStore || "";
          const id = b.id;
          // Skip action/event blocks — events are bound in design mode
          if (b.blockType === "action" || b.blockType === "onPress" || b.blockType === "onChange") continue;
          if (seenIds.has(id)) continue;
          seenIds.add(id);
          const el: import("./store/editor").ElementData = {
            id,
            nodeType: "codeblock",
            tag: b.blockType,
            props: { name: b.name, parentStore, typeAnnotation: b.typeAnnotation },
            children: [],
            parentId: parentStore ? `cb-store-${parentStore}` : null,
            ports: portDefs[b.blockType] || [],
            isFolded: false,
          };
          codeElements.push(el);
        }
        // Add missing store containers
        const existingStoreIds = new Set(codeElements.map((e) => e.id));
        for (const el of codeElements) {
          const pid = el.parentId;
          if (pid && pid.startsWith("cb-store-") && !existingStoreIds.has(pid)) {
            existingStoreIds.add(pid);
            const storeName = pid.replace("cb-store-", "");
            codeElements.push({
              id: pid,
              nodeType: "codeblock",
              tag: "store",
              props: { name: storeName },
              children: [el.id],
              parentId: null,
              ports: portDefs.store,
              isFolded: false,
            });
          }
        }
      }

      // Build fnToNodeMap for execution highlighting
      const newFnToNodeMap2: Record<string, string> = {};
      const compMatch2 = content.match(/component\s+(\w+)\s*\{/);
      const componentName2 = compMatch2 ? compMatch2[1] : "App";
      if (cbData.success && cbData.blocks) {
        for (const block of cbData.blocks) {
          if (block.blockType === "fn") {
            const compName = block.parentStore || componentName2;
            newFnToNodeMap2[`${compName}.${block.name}`] = block.id;
          }
        }
      }
      setFnToNodeMap(newFnToNodeMap2);

      useCodeGraphStore.getState().setElementsAndWires(codeElements, []);
      useEditorStore.getState().setSourceCode(content);
    } catch { /* canvas not critical */ }

    const hasErrors = errs.some((e: {severity:string}) => e.severity === "error");
    const warns = errs.filter((e: {severity:string}) => e.severity === "warning").length;
    if (hasErrors) setStatus({ text: "❌ 编译失败", isError: true });
    else {
      setStatus({ text: `✅ 已编译 — ${new Date().toLocaleTimeString()}${warns ? ` (${warns} 条警告)` : ""}`, isError: false });
      // Phase 1 Runtime Bridge: write compiled files to running Vite server
      if (runStatus.running && result.files) {
        api.writeRunFiles(result.files).catch(() => {});
      }
    }
  }, [target, runStatus.running]);

  // ── Run / Vite DevServer (Phase 1 Runtime Bridge) ──

  const handleRun = useCallback(async () => {
    const content = useEditorStore.getState().activeFileContent;
    if (!content) { setStatus({ text: "请先打开文件", isError: true }); return; }

    setRunLoading(true);
    setStatus({ text: "编译并启动运行...", isError: false });

    // 1. Compile (with debug mode enabled for runtime tracking)
    const result = await api.compile(content, target, undefined, true);
    const errs = result.errors || [];
    const hasErrors = errs.some((e: {severity:string}) => e.severity === "error");
    if (hasErrors) {
      setCompileErrors(errs);
      setStatus({ text: "❌ 编译失败 — 无法启动", isError: true });
      setRunLoading(false);
      return;
    }

    // 2. Parse view tree & code blocks for reactive canvas
    setCompileErrors([]);
    try {
      const [vRes, cbRes] = await Promise.all([
        fetch("/api/parse-view", { method: "POST", headers: {"Content-Type":"application/json"}, body: JSON.stringify({source: content}) }),
        fetch("/api/parse-codeblocks", { method: "POST", headers: {"Content-Type":"application/json"}, body: JSON.stringify({source: content}) }),
      ]);
      const vData = await vRes.json();
      const cbData = await cbRes.json();
      // (code element parsing same as handleCompile — no event/action nodes)
      const codeElements: import("./store/editor").ElementData[] = [];
      const seenIds = new Set<string>();
      if (cbData.success && cbData.blocks) {
        const portDefs: Record<string, Array<{ name: string; direction: "in" | "out"; type: string }>> = {
          store:    [{ name: "items", direction: "out", type: "any" }],
          signal:   [{ name: "value", direction: "out", type: "i32" }, { name: "set", direction: "in", type: "i32" }],
          computed: [{ name: "value", direction: "out", type: "i32" }, { name: "deps", direction: "in", type: "any" }],
        };
        for (const b of cbData.blocks) {
          // Skip action/event blocks — events are bound in design mode
          if (b.blockType === "action" || b.blockType === "onPress" || b.blockType === "onChange") continue;
          const parentStore = b.parentStore || "";
          const id = b.id;
          if (seenIds.has(id)) continue;
          seenIds.add(id);
          codeElements.push({
            id, nodeType: "codeblock", tag: b.blockType,
            props: { name: b.name, parentStore, typeAnnotation: b.typeAnnotation },
            children: [], parentId: parentStore ? `cb-store-${parentStore}` : null,
            ports: portDefs[b.blockType] || [], isFolded: false,
          });
        }
      }
      // Build fnToNodeMap: semantic ID → canvas element ID for execution highlighting
      const newFnToNodeMap: Record<string, string> = {};
      const compMatch = content.match(/component\s+(\w+)\s*\{/);
      const componentName = compMatch ? compMatch[1] : "App";
      if (cbData.success && cbData.blocks) {
        for (const block of cbData.blocks) {
          if (block.blockType === "fn") {
            const compName = block.parentStore || componentName;
            newFnToNodeMap[`${compName}.${block.name}`] = block.id;
          }
        }
      }
      setFnToNodeMap(newFnToNodeMap);

      useCodeGraphStore.getState().setElementsAndWires(codeElements, []);
      useEditorStore.getState().setSourceCode(content);
    } catch { /* non-critical */ }

    // 3. Start Vite dev server with compiled files
    setStatus({ text: "启动开发服务器...", isError: false });
    const runResult = await api.startRun(result.files || {}, true);

    if (runResult.success && runResult.url) {
      setRunStatus({ running: true, url: runResult.url });
      setStatus({ text: `▶ 运行中 — ${runResult.url}`, isError: false });
      window.open(runResult.url, "_blank");
    } else {
      setStatus({ text: `❌ 启动失败: ${runResult.error || "未知错误"}`, isError: true });
    }
    setRunLoading(false);
  }, [target]);

  const handleStop = useCallback(async () => {
    setStatus({ text: "停止开发服务器...", isError: false });
    await api.stopRun();
    setRunStatus({ running: false, url: null });
    setStatus({ text: "⏹ 已停止", isError: false });
    // Clean up debug state
    setDebugConnected(false);
    setDebugStateChanges([]);
    setActiveNodeIds(new Set());
    setActiveEdgeIds(new Set());
    setDebugPaused(false);
    setFnToNodeMap({});
  }, []);

  // ── v2: Project save/load ────────────────────

  const saveProject = useCallback(async (customPath?: string) => {
    const store = useEditorStore.getState();
    let projectPath = customPath || store.projectPath;

    if (!projectPath) {
      // Prompt for filename
      const name = prompt("保存项目为:", `${store.projectName}.omni.json`);
      if (!name) return;
      projectPath = name.endsWith(".omni.json") ? name : `${name}.omni.json`;
    }

    setStatus({ text: `保存项目到 ${projectPath} ...`, isError: false });
    try {
      const now = new Date().toISOString();
      const doc: Record<string, unknown> = {
        schemaVersion: "2.0.0",
        meta: {
          projectName: store.projectName,
          omniVersion: "0.6.0",
          createdAt: now,
          updatedAt: now,
        },
        sourceCode: store.sourceCode,
        design: {
          elements: store.elements,
          wires: store.wires,
        },
        code: {
          elements: useCodeGraphStore.getState().getActiveGraph().elements,
          wires: useCodeGraphStore.getState().getActiveGraph().wires,
        },
        codeTabs: useCodeGraphStore.getState().toSerializable(),
        components: (() => {
          const defs = useComponentRegistry.getState().definitions;
          return Object.values(defs).map((d: any) => ({
            name: d.name,
            source: d.source || "embedded",
            ...(d.source === "embedded" || !d.source ? {
              definition: {
                id: d.id,
                name: d.name,
                category: d.category,
                description: d.description,
                props: d.props,
                events: d.events,
                styleProperties: d.styleProperties,
                children: d.children,
                elements: d.elements,
                wires: d.wires,
                createdAt: d.createdAt,
                updatedAt: d.updatedAt,
                storage: d.storage,
              },
            } : {}),
          }));
        })(),
        canvasLayout: { positions: {} },
        tabState: {
          openTabs: store.openTabs,
          activeTab: store.activeDocId || "main",
        },
        config: store.config,
      };

      await api.saveOmniJson(projectPath, doc);
      // Also save component definitions for external references
      const externalDefs: Record<string, unknown> = {};
      const { definitions } = useComponentRegistry.getState();
      for (const [name, def] of Object.entries(definitions)) {
        if ((def as any).source && (def as any).source !== "embedded") {
          externalDefs[name] = def;
        }
      }
      if (Object.keys(externalDefs).length > 0) {
        api.saveComponentDefinitions(externalDefs).catch(() => {});
      }

      store.markClean();
      const name = projectPath.replace(/^.*[\\/]/, "").replace(".omni.json", "");
      useEditorStore.setState({ projectPath, projectName: name });
      const fullPath = projectDir ? `${projectDir}/${projectPath}` : projectPath;

      // ── Generate OmniLang source files via omni-gen ──
      const _cg = useCodeGraphStore.getState();
      const _genFilesResult = generateOmniFiles({
        designElements: useEditorStore.getState().elements,
        designWires: useEditorStore.getState().wires,
        codeTabs: _cg.toSerializable().tabs,
        componentName: projectName || "App",
      });

      if (_genFilesResult.value) {
        // Validate all files first, then write
        const _validFiles: Record<string, string> = {};
        let _allValid = true;
        for (const [_filePath, _source] of Object.entries(_genFilesResult.value)) {
          const _vResult = validate(_source);
          if (_vResult.valid) {
            _validFiles[_filePath] = _source;
          } else {
            _allValid = false;
            setStatus({ text: `❌ 验证失败: ${_filePath}`, isError: true });
            break;
          }
        }
        if (_allValid && Object.keys(_validFiles).length > 0) {
          Promise.all(
            Object.entries(_validFiles).map(([fp, src]) => api.writeFile(fp, src))
          ).catch((_e) => {
            setStatus({ text: "❌ 生成文件写入失败", isError: true });
          });
        }
      } else if (_genFilesResult.errors.length > 0) {
        // Non-fatal generation errors — warn but don't block save
        const _fatal = _genFilesResult.errors.some(e => e.severity === "fatal");
        if (_fatal) {
          setStatus({ text: "❌ 代码文件生成失败", isError: true });
        }
      }

      setStatus({ text: `✅ 已保存: ${fullPath}`, isError: false });
    } catch (e) {
      setStatus({ text: `❌ 保存失败: ${String(e)}`, isError: true });
    }
  }, []);

  // ── Open project ─────────────────────────────

  const [openDialog, setOpenDialog] = useState<{ visible: boolean; files: string[] }>({ visible: false, files: [] });

  const handleOpenProject = useCallback(async () => {
    setStatus({ text: "扫描项目文件...", isError: false });
    try {
      const listing = await api.listDir(".");
      const entries = listing?.entries || [];
      let jsonFiles = entries
        .filter((e: { name: string }) => e.name.endsWith(".omni.json"))
        .map((e: { name: string }) => e.name);
      // Scan one level of subdirs
      const subdirs = entries.filter((e: { type: string }) => e.type === "directory");
      for (const dir of subdirs) {
        try {
          const subListing = await api.listDir(dir.name);
          const subEntries = subListing?.entries || [];
          for (const se of subEntries) {
            if (se.name.endsWith(".omni.json")) {
              jsonFiles.push(`${dir.name}/${se.name}`);
            }
          }
        } catch { /* skip */ }
      }
      if (jsonFiles.length === 0) {
        setStatus({ text: "未找到项目文件 (.omni.json)", isError: true });
        return;
      }
      setOpenDialog({ visible: true, files: jsonFiles });
    } catch { setStatus({ text: "扫描失败", isError: true }); }
  }, []);

  const openProjectFile = useCallback(async (filePath: string) => {
    setOpenDialog({ visible: false, files: [] });
    setStatus({ text: `加载 ${filePath}...`, isError: false });
    try {
      const result = await api.readFile(filePath);
      if (!result.content) { setStatus({ text: `❌ 文件不存在: ${filePath}`, isError: true }); return; }
      const data = JSON.parse(result.content);
      const store = useEditorStore.getState();
      store.openProject(filePath, data);

      // Restore code tabs from saved data (backward compat + new format)
      const codeTabs = data.codeTabs as Record<string, unknown> | undefined;
      if (codeTabs && Array.isArray(codeTabs.tabs)) {
        useCodeGraphStore.getState().loadFromData(
          codeTabs.tabs as any,
          codeTabs.activeTab as string | undefined,
        );
      } else {
        // Backward compat: migrate old code.elements/code.wires → new format
        const code = (data.code || {}) as Record<string, unknown>;
        const oldElements = (code.elements || []) as any[];
        const oldWires = (code.wires || []) as any[];
        const migrated = migrateOldCodeGraph(oldElements, oldWires);
        useCodeGraphStore.getState().reset();
        useCodeGraphStore.getState().setElementsAndWires(migrated.elements, migrated.wires);
        if (migrated.droppedElements > 0 || migrated.droppedWires > 0) {
          setStatus({
            text: `✅ 已打开项目: ${filePath} (已迁移 ${migrated.droppedElements} 个旧节点)`,
            isError: false,
          });
          return; // skip the normal status set below
        }
      }

      setStatus({ text: `✅ 已打开项目: ${filePath}`, isError: false });
    } catch (e) {
      setStatus({ text: `❌ 加载失败: ${e instanceof SyntaxError ? "格式错误" : String(e)}`, isError: true });
    }
  }, []);

  // ── Keyboard ────────────────────────────────

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      const { selectedElementIds } = useEditorStore.getState();

      // Compile
      if (ctrl && e.key === "Enter") { e.preventDefault(); handleCompile(); }
      // Undo / Redo
      if (ctrl && e.key === "z" && !e.shiftKey) { e.preventDefault(); undoUndo(ctx); }
      if (ctrl && (e.key === "y" || (e.key === "z" && e.shiftKey))) { e.preventDefault(); undoRedo(ctx); }
      // Save
      if (ctrl && e.key === "s") { e.preventDefault(); saveProject(); }
      // Search
      if (ctrl && e.shiftKey && e.key === "F") { e.preventDefault(); setShowSearch(true); }
      // Activity Bar panel shortcuts (mode-aware)
      if (ctrl && e.key === "1") {
        e.preventDefault();
        setActivePanel(p => {
          const target: PanelId = mode === "design" ? "components" : "files";
          return p === target ? null : target;
        });
      }
      if (ctrl && e.key === "2") {
        e.preventDefault();
        setActivePanel(p => {
          const target: PanelId = mode === "design" ? "canvas" : "state";
          return p === target ? null : target;
        });
      }
      if (ctrl && e.key === "3") { e.preventDefault(); setActivePanel(p => p === "functions" ? null : "functions"); }

      // ── NEW: Clipboard shortcuts ──
      if (ctrl && e.key === "c" && !e.shiftKey) {
        // Don't capture when user is editing text
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const data = copySelection(ctx, selectedElementIds);
        if (data) showToast(`已复制 ${data.elements.length} 个元素`, "info");
      }
      if (ctrl && e.key === "v" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        e.preventDefault();
        const pastedIds = pasteFromClipboard(ctx);
        if (pastedIds && pastedIds.length > 0) {
          // Select pasted elements
          useEditorStore.setState({ selectedElementIds: pastedIds });
          showToast(`已粘贴 ${pastedIds.length} 个元素`, "success");
        } else {
          showToast("剪贴板为空", "info");
        }
      }
      if (ctrl && e.key === "x" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const ids = cutSelection(ctx, selectedElementIds);
        if (ids) showToast(`已剪切 ${ids.length} 个元素`, "success");
      }
      if (ctrl && e.key === "d" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const dupIds = duplicateSelection(ctx, selectedElementIds);
        if (dupIds) {
          useEditorStore.setState({ selectedElementIds: dupIds });
          showToast(`已复制 ${dupIds.length} 个元素`, "success");
        }
      }
      // Select all
      if (ctrl && e.key === "a" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        e.preventDefault();
        useEditorStore.getState().selectAll();
        const allIds = useEditorStore.getState().selectedElementIds;
        showToast(`已选择 ${allIds.length} 个元素`, "info");
      }

      // ── Arrow key nudge ──
      if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const delta = e.shiftKey ? 10 : 1;
        const dx = e.key === "ArrowLeft" ? -delta : e.key === "ArrowRight" ? delta : 0;
        const dy = e.key === "ArrowUp" ? -delta : e.key === "ArrowDown" ? delta : 0;
        const store = useEditorStore.getState();
        for (const id of selectedElementIds) {
          const el = store.elements.find((x) => x.id === id);
          if (el?.position) {
            store.setElementPosition(id, {
              x: el.position.x + dx,
              y: el.position.y + dy,
            });
          }
        }
      }

      // Escape: close popups + clear selection
      if (e.key === "Escape") {
        setShowHUD(false); setShowSearch(false); setShowConflict(false); setActivePanel(null);
        useEditorStore.getState().clearSelection();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleCompile, undoUndo, undoRedo, target, runStatus.running]);

  // ── Cleanup: stop Vite on page close + unsaved changes prompt ──

  useEffect(() => {
    const onUnload = (e: BeforeUnloadEvent) => {
      api.stopRun().catch(() => {});
      api.disconnectDebug();
      if (useEditorStore.getState().isDirty) {
        e.preventDefault();
        e.returnValue = "您有未保存的更改。";
      }
    };
    window.addEventListener("beforeunload", onUnload);
    return () => {
      window.removeEventListener("beforeunload", onUnload);
    };
  }, []);

  // ── Debug WebSocket connection (Phase 2) ────────

  const handleDebugMessage = useCallback((msg: Record<string, unknown>) => {
    const msgType = msg.type as string;
    switch (msgType) {
      case "state-change":
        setDebugStateChanges((prev) => {
          const next = [...prev, {
            signalId: msg.signalId as string || "",
            name: msg.name as string || "",
            value: msg.value,
            timestamp: msg.timestamp as number || Date.now(),
          }];
          return next.length > 200 ? next.slice(-200) : next;
        });
        break;
      case "execution-trace": {
        const semanticId = msg.nodeId as string;
        const edgeId = msg.edgeId as string | undefined;
        // Translate semantic ID ("Component.fnName") → canvas element ID
        const nodeId = fnToNodeMapRef.current[semanticId] || semanticId;
        if (nodeId) {
          setActiveNodeIds((prev) => {
            const next = new Set(prev);
            next.add(nodeId);
            // Auto-clear after 1.5s
            setTimeout(() => {
              setActiveNodeIds((p) => {
                const updated = new Set(p);
                updated.delete(nodeId);
                return updated;
              });
            }, 1500);
            return next;
          });
        }
        if (edgeId) {
          setActiveEdgeIds((prev) => {
            const next = new Set(prev);
            next.add(edgeId);
            setTimeout(() => {
              setActiveEdgeIds((p) => {
                const updated = new Set(p);
                updated.delete(edgeId);
                return updated;
              });
            }, 1500);
            return next;
          });
        }
        break;
      }
      case "runtime-ready":
        setDebugConnected(true);
        setDebugStateChanges([]);
        break;
      case "paused":
        setDebugPaused(true);
        break;
      case "resumed":
        setDebugPaused(false);
        break;
      case "runtime-disconnected":
        setDebugConnected(false);
        setActiveNodeIds(new Set());
        setActiveEdgeIds(new Set());
        setDebugPaused(false);
        setFnToNodeMap({});
        break;
    }
  }, []);

  // Auto-compile when opening a file (populate canvas)
  // Also parse OmniLang source via omni-gen to populate design elements/code graph
  const prevFilePath = useRef<string | null>(null);
  useEffect(() => {
    const currentPath = useEditorStore.getState().activeFilePath;
    const currentContent = useEditorStore.getState().activeFileContent;
    if (currentPath && currentContent && currentPath !== prevFilePath.current) {
      prevFilePath.current = currentPath;
      // Use a short debounce to avoid double-compile on initial load
      const timer = setTimeout(() => {
        if (currentPath.endsWith('.omni')) {
          // ── Parse `.omni` source to populate design elements and code tabs ──
          const parseResult = parseOmniSource(currentContent);
          if (parseResult.designElements) {
            useEditorStore.setState({
              elements: parseResult.designElements,
              wires: parseResult.designWires,
            });
            useCodeGraphStore.getState().loadFromData(parseResult.codeTabs);
          }
          handleCompile();
        }
      }, 150);
      return () => clearTimeout(timer);
    }
  }, [useEditorStore((s) => s.activeFilePath), useEditorStore((s) => s.activeFileContent)]);

  // Connect debug WebSocket when Run is active
  useEffect(() => {
    if (runStatus.running) {
      api.connectDebugWs(handleDebugMessage, setDebugConnected);
      return () => {
        api.disconnectDebug();
        setDebugConnected(false);
      };
    }
  }, [runStatus.running, handleDebugMessage]);

  // Debug helpers
  const handleClearDebug = useCallback(() => {
    setDebugStateChanges([]);
  }, []);

  const handleToggleBreakpoint = useCallback((signalId: string) => {
    setDebugBreakpoints((prev) => {
      const next = new Set(prev);
      if (next.has(signalId)) {
        next.delete(signalId);
        api.sendDebugCommand({ type: "remove-breakpoint", signalId });
      } else {
        next.add(signalId);
        api.sendDebugCommand({ type: "set-breakpoint", signalId });
      }
      return next;
    });
  }, []);

  const handleStep = useCallback(() => {
    api.sendDebugCommand({ type: "step" });
  }, []);

  const handleResume = useCallback(() => {
    api.sendDebugCommand({ type: "resume" });
  }, []);

  const ctx: CommandContext = {
    getElement: (id) => useEditorStore.getState().elements.find(e => e.id === id),
    get elements() { return useEditorStore.getState().elements; },
    get wires() { return useEditorStore.getState().wires; },
    addElement: (el) => useEditorStore.getState().addElement(el),
    removeElement: (id) => useEditorStore.getState().removeElement(id),
    setProp: (id, p, v) => useEditorStore.getState().setProp(id, p, v),
    getProp: (id, p) => useEditorStore.getState().getProp(id, p) ?? null,
    addWire: (w) => useEditorStore.getState().addWire(w),
    removeWire: (id) => useEditorStore.getState().removeWire(id),
    setElementsAndWires: (elements, wires) =>
      useEditorStore.getState().setElementsAndWires(elements, wires),
    reorderElement: (elementId, targetSiblingId, position) =>
      useEditorStore.getState().reorderElement(elementId, targetSiblingId, position),
    moveElement: (elementId, newParentId, targetSiblingId, position) =>
      useEditorStore.getState().moveElement(elementId, newParentId, targetSiblingId, position),
    getRootOrder: () => useEditorStore.getState().rootOrder,
  };

  const compileErrors = useEditorStore((s) => s.compileErrors);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const selectedId = selectedIds.length > 0 ? selectedIds[0] : null;

  // ── Search context (M5.6) ────────────────────
  const tokensState = useTokenStore((s) => s.tokens);
  const filesState = useEditorStore((s) => s.files);
  const wiresState = useEditorStore((s) => s.wires);
  const searchContext = useMemo(() => ({
    files: filesState,
    wires: wiresState,
    tokens: Object.entries(tokensState.colors || {}).map(([name, value]) => ({
      name, value, category: "colors",
    })),
  }), [filesState, wiresState, tokensState]);

  // ── Search result handler ────────────────────
  const handleSearchSelect = useCallback((result: SearchResult) => {
    switch (result.action.type) {
      case "open-file":
        api.readFile(result.action.payload).then((r) => {
          if (r.content) useEditorStore.getState().openFile(result.action.payload, r.content);
        });
        break;
      case "insert-snippet":
        codeEditorRef.current?.insertSnippet(result.action.payload);
        setMode("code");
        break;
      case "select-element":
        // Find element by tag and select it
        const el = useEditorStore.getState().elements.find(e => e.tag === result.action.payload);
        if (el) useEditorStore.getState().selectElement(el.id);
        setMode("design");
        break;
      case "run-command":
        if (result.action.payload === "compile") handleCompile();
        else if (result.action.payload === "toggle-wire") setWireMode(!wireMode);
        else if (result.action.payload === "toggle-snippets") setShowSnippets(!showSnippets);
        else if (result.action.payload === "toggle-tokens") setShowTokens(!showTokens);
        else if (result.action.payload === "focus-order") setShowFocusOrder(!showFocusOrder);
        break;
    }
  }, [wireMode, showSnippets, showTokens, showFocusOrder, handleCompile]);

  // ── Deploy handler (M5.5) ────────────────────
  const handleDeploy = useCallback(async () => {
    const dir = useEditorStore.getState().projectDir;
    if (!dir) { setStatus({ text: "请先打开项目", isError: true }); return; }
    setStatus({ text: "发布中...", isError: false });
    try {
      const res = await fetch("/api/deploy", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ projectDir: dir, platform: "vercel" }),
      });
      const data = await res.json();
      if (data.success) {
        setStatus({ text: `✅ 已发布: ${data.url}`, isError: false });
      } else {
        setStatus({ text: `❌ 发布失败: ${data.error}`, isError: true });
      }
    } catch (e) {
      setStatus({ text: `❌ 发布失败: ${String(e)}`, isError: true });
    }
  }, []);

  // ── Feature: Load project ──────────────────────


  // ═════════════════════════════════════════════
  // Render
  // ═════════════════════════════════════════════

  return (
    <div style={S.app}>
      {mode === "code" ? (
        /* ══════ 代码模式 ══════ */
        <>
          <ActivityBar activePanel={activePanel} onToggle={(id) => setActivePanel(p => p === id ? null : id)} />
          <PanelContainer visible={activePanel === "files"}>
            <FileExplorer />
          </PanelContainer>
          <PanelContainer visible={activePanel === "state"}>
            <StatePanel ctx={ctx} />
          </PanelContainer>
          <PanelContainer visible={activePanel === "functions"}>
            <FunctionLibrary
              ctx={ctx}
              onFnDoubleClick={(fnId, fnData) => {
                const el = useEditorStore.getState().elements.find(e => e.id === fnId);
                setCodePopup({
                  visible: true,
                  blockId: fnId,
                  title: `📦 ${fnData.name} (${fnData.params}) → ${fnData.returnType}`,
                  source: fnData.source,
                  blockElement: el || null,
                });
              }}
            />
            <BlockPalette ctx={ctx} />
          </PanelContainer>
          <main style={S.main}>
            <div style={S.toolbar}>
              <button onMouseDown={() => setMode("design")} onClick={() => setMode("design")} style={S.modeBtn(mode === "design")}>🎨 设计</button>
              <button onMouseDown={() => setMode("code")} onClick={() => setMode("code")} style={S.modeBtn(mode === "code")}>💻 代码</button>
              <span style={{ width: 8 }} />
              <button onClick={() => undoUndo(ctx)} style={{...btnStyle, padding:"2px 6px"}} title="撤销 (Ctrl+Z)">↩</button>
              <button onClick={() => undoRedo(ctx)} style={{...btnStyle, padding:"2px 6px"}} title="重做 (Ctrl+Shift+Z)">↪</button>
              <button onClick={() => saveProject()} style={{...btnStyle, padding:"2px 6px", color: isDirty ? "#DC2626" : "#6B7280", fontWeight: isDirty ? 700 : 400}} title="保存项目 (Ctrl+S)">💾</button>
              <select
                value=""
                onChange={(e) => {
                  const v = e.target.value;
                  if (v) {
                    const O = (window as any).__OMNI__;
                    if (O) O.snippet(v);
                  }
                  e.target.value = "";
                }}
                style={{...btnStyle, fontSize: 11, padding: "2px 6px"}}
                title="快速模板"
              >
                <option value="" disabled>📋 模板</option>
                <option value="login-form">登录表单</option>
                <option value="data-card">数据卡片</option>
                <option value="crud-table">CRUD 表格</option>
              </select>
              {projectDir && <span style={{ color: "#9ca3af", fontSize: 11 }}>{projectDir.split(/[/\\]/).slice(-2).join("/")}</span>}
              <span style={{ flex: 1 }} />
              <span style={{ fontSize: 10, color: "#9ca3af" }}>目标:</span>
              <select value={target} onChange={e => setTarget(e.target.value)} style={{ padding: "2px 6px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4 }}>
                {["react","omniview","vue","dioxus","flutter","python","javascript","rust","dart"].map(t => <option key={t}>{t}</option>)}
              </select>
              <button onClick={() => setShowConnector(!showConnector)}
                style={{...btnStyle, background: showConnector ? "#EEF2FF" : "#fff", borderColor: showConnector ? "#4F46E5" : "#d1d5db" }}>
                {showConnector ? "🔌 连接器" : "🔌 连接器"}
              </button>
              <button onClick={() => setLayoutTrigger(v => v + 1)}
                style={{...btnStyle, background: "#fff"}}>
                📐 自动布局
              </button>
              {runStatus.running ? (
                <>
                  <a href={runStatus.url || "#"} target="_blank" rel="noreferrer"
                    style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", textDecoration: "none" }}>
                    🔗 {runStatus.url?.replace("http://", "") || "运行"}
                  </a>
                  <button onClick={handleStop}
                    style={{...btnStyle, background: "#FEF2F2", borderColor: "#FCA5A5", color: "#DC2626" }}>
                    ⏹ 停止
                  </button>
                </>
              ) : (
                <button onClick={handleRun} disabled={runLoading}
                  style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", fontWeight: 600 }}>
                  {runLoading ? "⏳ 启动中..." : "▶ 运行"}
                </button>
              )}
              <button onClick={handleCompile} style={btnStyle}>编译 (Ctrl+Enter)</button>
            </div>
            <CodeTabBar />
            <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
              <div style={S.codeArea}>
                <FlowCanvas
                  highlightedWireId={highlightedWireId}
                  layoutTrigger={layoutTrigger}
                  activeNodeIds={activeNodeIds}
                  activeEdgeIds={activeEdgeIds}
                  onNodeDoubleClick={(id, el) => {
                    const blockSource = el.source || "";
                    setCodePopup({
                      visible: true,
                      blockId: id,
                      title: `${(el.props as Record<string,unknown>)?.name || "Code"} (${el.tag})`,
                      source: blockSource,
                      blockElement: el,
                    });
                  }}
                />
                {/* Debug Panel (Phase 2 Runtime Bridge) */}
                {runStatus.running && (
                  <DebugPanel
                    connected={debugConnected}
                    stateChanges={debugStateChanges}
                    onClear={handleClearDebug}
                    onToggleBreakpoint={handleToggleBreakpoint}
                    breakpoints={debugBreakpoints}
                    paused={debugPaused}
                    onStep={handleStep}
                    onResume={handleResume}
                  />
                )}
              </div>
              {/* Code Property Panel (right sidebar for node inspection) */}
              <aside style={{ width: 260, minWidth: 220, flexShrink: 0, borderLeft: "1px solid #e5e7eb", background: "#f9fafb", overflow: "auto" }}>
                <div style={{ padding: "4px 8px", fontSize: 10, fontWeight: 600, color: "#9CA3AF", borderBottom: "1px solid #e5e7eb" }}>
                  节点属性
                </div>
                <CodePropertyPanel />
              </aside>
              {showConnector && (
                <aside style={{ width: 280, minWidth: 240, flexShrink: 0, borderLeft: "1px solid #e5e7eb", background: "#f9fafb", overflow: "hidden" }}>
                  <ConnectorPanel
                    onInsertCode={(code) => codeEditorRef.current?.insertSnippet(code)}
                  />
                </aside>
              )}
            </div>
            <div style={S.statusBar}>
              <span style={{ color: status.isError ? "#DC2626" : status.text.startsWith("⚠") ? "#D97706" : "#6B7280" }}>{status.text}</span>
              <span style={{ color: "#9ca3af" }}>{projectName}{isDirty ? " ●" : ""} {projectDir ? `| 📁 ${projectDir}` : ""} {compileErrors.length > 0 && `— ${compileErrors.length} 条错误`}</span>
            </div>
          </main>
        </>
      ) : (
        /* ══════ 设计模式 ══════ */
        <>
          <ActivityBar panels={DESIGN_PANELS} activePanel={activePanel} onToggle={(id) => setActivePanel(p => p === id ? null : id)} />
          <PanelContainer visible={activePanel === "components" || activePanel === "canvas"}>
            {activePanel === "components" && <ComponentPalette />}
            {activePanel === "canvas" && <CanvasPanel />}
            {showTokens && activePanel !== null && (
              <div style={{ borderTop: "1px solid #e5e7eb" }}>
                <TokenPanel />
              </div>
            )}
          </PanelContainer>
          <main style={S.main}>
            <div style={S.toolbar}>
              {/* Mode toggle */}
              <button onMouseDown={() => setMode("design")} onClick={() => setMode("design")} style={S.modeBtn(mode === "design")}>🎨 设计</button>
              <button onMouseDown={() => setMode("code")} onClick={() => setMode("code")} style={S.modeBtn(mode === "code")}>💻 代码</button>
              <span style={{ width: 8 }} />
              {/* Undo/Redo */}
              <button onClick={() => undoUndo(ctx)} style={iconBtn} title="撤销 (Ctrl+Z)">↩</button>
              <button onClick={() => undoRedo(ctx)} style={iconBtn} title="重做 (Ctrl+Shift+Z)">↪</button>
              {/* Save */}
              <button onClick={() => saveProject()} style={{...iconBtn, color: isDirty ? "#DC2626" : "#6B7280", fontWeight: isDirty ? 700 : 400}} title="保存项目 (Ctrl+S)">💾</button>
              <button onClick={() => handleOpenProject()} style={iconBtn} title="打开项目">📂</button>
              <span style={{ flex: 1 }} />
              {/* Viewport */}
              <span style={{ fontSize: 10, color: "#9CA3AF" }}>视口:</span>
              {(["desktop","tablet","mobile"] as const).map(bp => (
                <button key={bp} onClick={() => setBreakpoint(bp)}
                  style={{ padding: "2px 6px", fontSize: 10, border: "1px solid", borderRadius: 3, cursor: "pointer",
                    background: breakpoint === bp ? "#4F46E5" : "#fff",
                    color: breakpoint === bp ? "#fff" : "#6B7280",
                    borderColor: breakpoint === bp ? "#4F46E5" : "#d1d5db" }}>
                  {{desktop:"🖥",tablet:"📱",mobile:"📱"}[bp]}
                </button>
              ))}
              {/* State preview */}
              <select value={previewState} onChange={(e) => setPreviewState(e.target.value)}
                style={{ padding: "2px 4px", fontSize: 10, border: "1px solid #d1d5db", borderRadius: 3, background: "#fff" }}
                title="Component state preview">
                <option value="normal">👁</option>
                <option value="hover">🖱</option>
                <option value="active">👆</option>
                <option value="focus">🎯</option>
                <option value="disabled">🚫</option>
                <option value="loading">⏳</option>
              </select>
              <span style={{ width: 8 }} />
              {/* Toggle buttons (compact icon-only) */}
              <button onClick={() => setShowTokens(!showTokens)}
                style={{...iconBtn, background: showTokens ? "#EEF2FF" : "transparent", borderColor: showTokens ? "#4F46E5" : "transparent" }}
                title="令牌面板">🎨</button>
              <button onClick={() => setWireMode(!wireMode)}
                style={{...iconBtn, background: wireMode ? "#EEF2FF" : "transparent", borderColor: wireMode ? "#4F46E5" : "transparent" }}
                title="连线模式">🔗</button>
              <button onClick={() => setShowHUD(!showHUD)}
                style={{...iconBtn, background: showHUD ? "#EEF2FF" : "transparent", borderColor: showHUD ? "#4F46E5" : "transparent" }}
                title="状态 HUD">🔍</button>
              <button onClick={() => setShowFocusOrder(!showFocusOrder)}
                style={{...iconBtn, background: showFocusOrder ? "#EEF2FF" : "transparent", borderColor: showFocusOrder ? "#4F46E5" : "transparent" }}
                title="焦点顺序">↹</button>
              <span style={{ width: 8 }} />
              {/* Run/Stop/Compile */}
              {runStatus.running ? (
                <>
                  <a href={runStatus.url || "#"} target="_blank" rel="noreferrer"
                    style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", textDecoration: "none" }}>
                    🔗 {runStatus.url?.replace("http://", "") || "运行"}
                  </a>
                  <button onClick={handleStop}
                    style={{...btnStyle, background: "#FEF2F2", borderColor: "#FCA5A5", color: "#DC2626" }}>
                    ⏹ 停止
                  </button>
                </>
              ) : (
                <button onClick={handleRun} disabled={runLoading}
                  style={{...btnStyle, background: "#ECFDF5", borderColor: "#10B981", color: "#059669", fontWeight: 600 }}>
                  {runLoading ? "⏳" : "▶"} 运行
                </button>
              )}
              <button onClick={handleCompile} style={btnStyle}>编译</button>
            </div>
            <CanvasTabBar />
            <div style={{ display: "flex", justifyContent: "center", background: "#f0f0f0", flex: 1, overflow: "auto" }}>
              <div style={{
                width: breakpoint === "desktop" ? "100%" : breakpoint === "tablet" ? 768 : 375,
                maxWidth: "100%", background: "#fff",
                boxShadow: "0 0 20px rgba(0,0,0,0.1)",
                position: "relative", transition: "width 0.3s ease",
                display: "flex", flexDirection: "column",
              }}>
                <LayoutView ctx={ctx} hideCodeBlocks />
                {wireMode && <WireOverlay />}
                {selectedId && <StateHUD elementId={selectedId} visible={showHUD} onClose={() => setShowHUD(false)} />}
                {showFocusOrder && <FocusOrderOverlay />}
              </div>
            </div>
            <div style={S.statusBar}>
              <span style={{ color: status.isError ? "#DC2626" : status.text.startsWith("⚠") ? "#D97706" : "#6B7280" }}>{status.text}</span>
              <span style={{ color: "#9ca3af" }}>{projectName}{isDirty ? " ● 未保存" : ""} {projectDir ? `| 📁 ${projectDir}` : ""}</span>
              <span style={{ flex: 1 }} />
              <PerfBadge />
            </div>
          </main>
          <aside style={S.inspector}>
            <PropertyPanel
              ctx={ctx}
              onNavigateToCode={(wireId) => {
                setMode("code");
                setHighlightedWireId(wireId);
              }}
            />
          </aside>
        </>
      )}

      {/* ── Code editor popup — edits block-specific source ── */}
      <CodeEditorPopup
        visible={codePopup.visible}
        blockId={codePopup.blockId}
        title={codePopup.title}
        source={codePopup.source}
        blockElement={codePopup.blockElement}
        onClose={() => setCodePopup({ visible: false, blockId: null, title: "", source: "", blockElement: null })}
        onSave={(newSource) => {
          if (codePopup.blockId) {
            setElementSource(codePopup.blockId, newSource);
            // Auto-extract fn name from source (for function library entries)
            const fnMatch = newSource.match(/fn\s+(\w+)\s*\(/);
            if (fnMatch) {
              useEditorStore.getState().setProp(codePopup.blockId, "name", fnMatch[1]);
            }
          }
          setCodePopup({ visible: false, blockId: null, title: "", source: "", blockElement: null });
        }}
      />

      {/* ── Open Project Dialog ────────────────── */}
      {openDialog.visible && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)", zIndex: 10000, display: "flex", alignItems: "center", justifyContent: "center" }}
          onClick={() => setOpenDialog({ visible: false, files: [] })}>
          <div style={{ background: "#fff", borderRadius: 12, padding: "24px 28px", minWidth: 420, maxHeight: "70vh", boxShadow: "0 8px 32px rgba(0,0,0,0.2)", display: "flex", flexDirection: "column" }}
            onClick={e => e.stopPropagation()}>
            <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 12 }}>📂 打开项目</div>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 12 }}>选择 .omni.json 项目文件</div>
            <div style={{ flex: 1, overflow: "auto", marginBottom: 12 }}>
              {openDialog.files.map((f, i) => (
                <div key={f} onClick={() => openProjectFile(f)}
                  style={{ padding: "10px 14px", cursor: "pointer", borderRadius: 6, display: "flex", alignItems: "center", gap: 10,
                    borderBottom: i < openDialog.files.length - 1 ? "1px solid #f3f4f6" : "none",
                  }}
                  onMouseEnter={e => (e.currentTarget.style.background = "#F5F3FF")}
                  onMouseLeave={e => (e.currentTarget.style.background = "transparent")}>
                  <span style={{ fontSize: 20 }}>📄</span>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 14, fontWeight: 500, color: "#374151" }}>{f}</div>
                  </div>
                  <span style={{ fontSize: 11, color: "#9ca3af" }}>点击打开</span>
                </div>
              ))}
            </div>
            <button onClick={() => setOpenDialog({ visible: false, files: [] })}
              style={{ padding: "6px 16px", fontSize: 13, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer", alignSelf: "flex-end" }}>取消</button>
          </div>
        </div>
      )}

      {/* ── Top-level modals (M5) ── */}
      <SearchModal
        visible={showSearch}
        onClose={() => setShowSearch(false)}
        onSelect={handleSearchSelect}
        context={{
          files: useEditorStore.getState().files,
          fileContents: new Map([[useEditorStore.getState().activeFilePath || "", useEditorStore.getState().activeFileContent || ""]]),
          wires: useEditorStore.getState().wires,
          tokens: Object.entries(useTokenStore.getState().tokens.colors).map(([name, value]) => ({ name, value, category: "colors" })),
        }}
      />
    </div>
  );
}

// ═══════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════

const btnStyle: React.CSSProperties = { padding: "4px 12px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 4, background: "#fff", cursor: "pointer" };
const iconBtn: React.CSSProperties = { padding: "2px 6px", fontSize: 13, border: "1px solid transparent", borderRadius: 4, background: "transparent", cursor: "pointer", lineHeight: 1 };

interface TreeNode { id: string; tag: string; props?: Record<string, unknown>; children?: TreeNode[]; testId?: string | null; }

function flattenTree(nodes: TreeNode[], parentId: string | null): import("./store/editor").ElementData[] {
  const result: import("./store/editor").ElementData[] = [];
  for (const n of nodes) {
    const el: import("./store/editor").ElementData = { id: n.id, nodeType: "view", tag: n.tag, props: n.props || {}, children: [], parentId };
    if (n.children?.length) { el.children = n.children.map(c => c.id); result.push(...flattenTree(n.children, n.id)); }
    result.push(el);
  }
  return result;
}

// ── M5.4 Focus Order Overlay ──────────────────

function FocusOrderOverlay() {
  const elements = useEditorStore((s) => s.elements);
  const [entries, setEntries] = useState<ReturnType<typeof collectFocusOrder>>([]);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Event-driven: update on element changes + container resize
    const update = () => setEntries(collectFocusOrder(elements));
    update();

    const ro = new ResizeObserver(update);
    const container = containerRef.current?.parentElement;
    if (container) ro.observe(container);

    return () => ro.disconnect();
  }, [elements]);

  return (
    <div ref={containerRef} style={{ position: "absolute", top: 0, left: 0, zIndex: 50, pointerEvents: "none" }}>
      {entries.map((entry) => {
        const styles = focusOverlayStyles(entry);
        return (
          <div key={entry.id} style={styles.container}>
            <div style={styles.box}>
              <span style={styles.label}>Tab {entry.index}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}

