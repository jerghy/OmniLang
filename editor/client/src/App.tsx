import React, { useEffect, useCallback, useState, useRef, useMemo } from "react";
import { useEditorStore } from "./store/editor";
import { useUndoStore } from "./store/undo";
import { api } from "./api/client";
import { OmniDriver } from "./driver";
import { FileExplorer } from "./panels/FileExplorer";
import { BlockPalette } from "./panels/BlockPalette";
import { PropertyPanel } from "./panels/PropertyPanel";
import { ComponentPalette } from "./panels/ComponentPalette";
import { ConnectorPanel } from "./panels/ConnectorPanel";
import { TokenPanel, useTokenStore } from "./panels/TokenPanel";
import { StatePanel } from "./panels/StatePanel";
import { FunctionLibrary } from "./panels/FunctionLibrary";
import { ActivityBar, PanelContainer, type PanelId } from "./panels/ActivityBar";
import { FlowCanvas } from "./canvas/FlowCanvas";
import { LayoutView } from "./canvas/LayoutView";
import { WireOverlay } from "./canvas/WireOverlay";
import { StateHUD } from "./canvas/StateHUD";
import { PerfBadge } from "./canvas/PerfBadge";
import { DebugPanel } from "./canvas/DebugPanel";
import { MonacoEditor, type MonacoEditorHandle } from "./editor/MonacoEditor";
import { CodeEditorPopup } from "./editor/CodeEditorPopup";
import { ConflictPanel } from "./panels/ConflictPanel";
import { ReadOnlyBanner } from "./panels/ReadOnlyBanner";
import type { ConflictInfo } from "./panels/ConflictPanel";
import { AnimationImport } from "./panels/AnimationImport";
import { SearchModal } from "./search/SearchModal";
import { collectFocusOrder, focusOverlayStyles } from "./a11y/FocusOrder";
import type { SearchResult } from "./search/SearchIndex";
import type { CommandContext } from "./commands/types";
import { SetElementsAndWires } from "./commands/SetElementsAndWires";

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
  const [conflictInfo, setConflictInfo] = useState<ConflictInfo | null>(null);
  const [showAnimation, setShowAnimation] = useState(false);
  const [activePanel, setActivePanel] = useState<PanelId>(null);
  const codeEditorRef = useRef<MonacoEditorHandle>(null);
  const [layoutTrigger, setLayoutTrigger] = useState(0);

  // Save/Load state
  const [loadedCanvasPath, setLoadedCanvasPath] = useState<string | null>(null);
  const [saveDialog, setSaveDialog] = useState<{ visible: boolean; defaultName: string }>({ visible: false, defaultName: "" });
  const [loadDialog, setLoadDialog] = useState<{ visible: boolean; files: string[] }>({ visible: false, files: [] });

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
  const undoExec = useUndoStore((s) => s.execute);
  const undoUndo = useUndoStore((s) => s.undo);
  const undoRedo = useUndoStore((s) => s.redo);
  const setHandWritten = useEditorStore((s) => s.setHandWritten);
  const isHandWritten = useEditorStore((s) => s.isHandWritten);
  const readOnlyMode = useEditorStore((s) => s.readOnlyMode);
  const handWrittenWarnings = useEditorStore((s) => s.handWrittenWarnings);

  // ── Init ────────────────────────────────────

  useEffect(() => {
    (async () => {
      const proj = await api.getProject();
      if (proj.dir) setProjectDir(proj.dir);
      const list = await api.listDir(".");
      if (list.entries) loadFiles(list.entries);
      if (proj.mainFile) {
        const src = await api.readFile(proj.mainFile);
        if (src.content) {
          openFile(proj.mainFile, src.content);
          // M6: Detect hand-written files
          if (proj.mainFile.endsWith('.omni')) {
            const hw = await api.detectHandWritten(src.content);
            if (hw.isHandWritten) {
              setHandWritten(true, hw.warnings);
            }
          }
        }
      }
    })();
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
    const content = useEditorStore.getState().activeFileContent;
    if (!content) return;
    setStatus({ text: "编译中...", isError: false });

    const result = await api.compile(content, target, undefined, runStatus.running);
    const errs = result.errors || [];
    setCompileErrors(errs);

    // Parse view tree for canvas — clear old first
    useEditorStore.setState({ elements: [], selectedElementId: null });
    try {
      const [vRes, cbRes] = await Promise.all([
        fetch("/api/parse-view", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ source: content }) }),
        fetch("/api/parse-codeblocks", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ source: content }) }),
      ]);
      const vData = await vRes.json();
      const cbData = await cbRes.json();

      const viewElements = vData.success && vData.nodes ? flattenTree(vData.nodes, null) : [];
      const codeElements: import("./store/editor").ElementData[] = [];
      const seenIds = new Set<string>();

      if (cbData.success && cbData.blocks) {
        const portDefs: Record<string, Array<{ name: string; direction: "in" | "out"; type: string }>> = {
          store:    [{ name: "items", direction: "out", type: "any" }],
          signal:   [{ name: "value", direction: "out", type: "i32" }, { name: "set", direction: "in", type: "i32" }],
          computed: [{ name: "value", direction: "out", type: "i32" }, { name: "deps", direction: "in", type: "any" }],
          action:   [{ name: "call", direction: "in", type: "any" }],
        };
        for (const b of cbData.blocks) {
          const parentStore = b.parentStore || "";
          const id = b.id;
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
        // Add missing store containers (if children exist but store wasn't in blocks)
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

      // Build fnToNodeMap for execution highlighting (same logic as handleRun)
      const newFnToNodeMap2: Record<string, string> = {};
      const compMatch2 = content.match(/component\s+(\w+)\s*\{/);
      const componentName2 = compMatch2 ? compMatch2[1] : "App";
      if (cbData.success && cbData.blocks) {
        for (const block of cbData.blocks) {
          if (block.blockType === "action") {
            const compName = block.parentStore || componentName2;
            newFnToNodeMap2[`${compName}.${block.name}`] = block.id;
          }
        }
      }
      for (const el of viewElements) {
        if (el.tag === "onPress") newFnToNodeMap2[`${componentName2}.handlePress`] = el.id;
        else if (el.tag === "onChange") newFnToNodeMap2[`${componentName2}.handleChange`] = el.id;
      }
      setFnToNodeMap(newFnToNodeMap2);

      useEditorStore.setState({ elements: [...viewElements, ...codeElements] });
    } catch { /* canvas not critical */ }

    const hasErrors = errs.some((e: {severity:string}) => e.severity === "error");
    const warns = errs.filter((e: {severity:string}) => e.severity === "warning").length;
    if (hasErrors) setStatus({ text: "❌ 编译失败", isError: true });
    else {
      setStatus({ text: `✅ 已编译 — ${new Date().toLocaleTimeString()}${warns ? ` (${warns} 条警告)` : ""}`, isError: false });
      // M6: Auto-save .omni.json after successful compilation
      if (!result.errors?.some((e: {severity:string}) => e.severity === "error")) {
        autoSaveOmniJson(content);
      }
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
      const viewElements = vData.success && vData.nodes ? flattenTree(vData.nodes, null) : [];
      // (code element parsing same as handleCompile)
      const codeElements: import("./store/editor").ElementData[] = [];
      const seenIds = new Set<string>();
      if (cbData.success && cbData.blocks) {
        const portDefs: Record<string, Array<{ name: string; direction: "in" | "out"; type: string }>> = {
          store:    [{ name: "items", direction: "out", type: "any" }],
          signal:   [{ name: "value", direction: "out", type: "i32" }, { name: "set", direction: "in", type: "i32" }],
          computed: [{ name: "value", direction: "out", type: "i32" }, { name: "deps", direction: "in", type: "any" }],
          action:   [{ name: "call", direction: "in", type: "any" }],
        };
        for (const b of cbData.blocks) {
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
          if (block.blockType === "action") {
            const compName = block.parentStore || componentName;
            newFnToNodeMap[`${compName}.${block.name}`] = block.id;
          }
        }
      }
      for (const el of viewElements) {
        if (el.tag === "onPress") newFnToNodeMap[`${componentName}.handlePress`] = el.id;
        else if (el.tag === "onChange") newFnToNodeMap[`${componentName}.handleChange`] = el.id;
      }
      setFnToNodeMap(newFnToNodeMap);

      useEditorStore.setState({ elements: [...viewElements, ...codeElements] });
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

  // ── M6: .omni.json auto-save ──────────────────

  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

  const autoSaveOmniJson = useCallback((source: string) => {
    // Debounce: skip rapid successive calls
    if (autoSaveTimer) return;
    autoSaveTimer = setTimeout(async () => {
      autoSaveTimer = null;
      try {
        const store = useEditorStore.getState();
        // Only save if not in read-only mode (hand-written files)
        if (store.readOnlyMode) return;

        const activeFile = store.activeFilePath;
        if (!activeFile || !activeFile.endsWith(".omni")) return;

        // Determine .omni.json path alongside the .omni file
        const omniJsonPath = activeFile.replace(/\.omni$/, ".omni.json");

        // Export current state to .omni.json
        const existingElements = store.elements.map(el => ({
          id: el.id, nodeType: el.nodeType, tag: el.tag,
          props: el.props, children: el.children, parentId: el.parentId,
          ports: el.ports || [], isFolded: el.isFolded || false,
          typeAnnotation: el.typeAnnotation || "", storeName: el.storeName || "",
        }));
        const existingWires = store.wires.map(w => ({
          id: w.id, fromId: w.fromId, fromPort: w.fromPort,
          toId: w.toId, toPort: w.toPort,
          isCollapsed: w.isCollapsed || false, collapseGroupId: w.collapseGroupId || "",
        }));

        const exportResp = await api.exportOmniJson(source, activeFile, target);
        if (exportResp.success && exportResp.document) {
          // Merge in the current elements/wires from the store
          const doc = exportResp.document as Record<string, unknown>;
          doc.elements = existingElements;
          doc.wires = existingWires;
          await api.saveOmniJson(omniJsonPath, doc);
        }
      } catch {
        // Auto-save is best-effort; don't disrupt user
      }
    }, 1000); // 1s debounce
  }, [target]);

  // ── Keyboard ────────────────────────────────

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); handleCompile(); }
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) { e.preventDefault(); undoUndo(ctx); }
      if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.key === "z" && e.shiftKey))) { e.preventDefault(); undoRedo(ctx); }
      if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); const p = useEditorStore.getState().activeFilePath; const c = useEditorStore.getState().activeFileContent; if (p && c) { api.writeFile(p, c); autoSaveOmniJson(c); setStatus({text:"已保存",isError:false}); /* If running, recompile + write for HMR */ if (runStatus.running) { api.compile(c, target, undefined, true).then(result => { if (result.success && result.files) api.writeRunFiles(result.files).catch(()=>{}); }).catch(()=>{}); } } }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "F") { e.preventDefault(); setShowSearch(true); }
      // Activity Bar panel shortcuts
      if ((e.ctrlKey || e.metaKey) && e.key === "1") { e.preventDefault(); setActivePanel(p => p === "files" ? null : "files"); }
      if ((e.ctrlKey || e.metaKey) && e.key === "2") { e.preventDefault(); setActivePanel(p => p === "state" ? null : "state"); }
      if ((e.ctrlKey || e.metaKey) && e.key === "3") { e.preventDefault(); setActivePanel(p => p === "functions" ? null : "functions"); }
      if (e.key === "Escape") { setShowHUD(false); setShowSearch(false); setShowConflict(false); setActivePanel(null); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleCompile, undoUndo, undoRedo, target, runStatus.running]);

  // ── Cleanup: stop Vite on page close ───────────

  useEffect(() => {
    const onUnload = () => {
      api.stopRun().catch(() => {});
      api.disconnectDebug();
    };
    window.addEventListener("beforeunload", onUnload);
    return () => window.removeEventListener("beforeunload", onUnload);
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
  const prevFilePath = useRef<string | null>(null);
  useEffect(() => {
    const currentPath = useEditorStore.getState().activeFilePath;
    const currentContent = useEditorStore.getState().activeFileContent;
    if (currentPath && currentContent && currentPath !== prevFilePath.current) {
      prevFilePath.current = currentPath;
      // Use a short debounce to avoid double-compile on initial load
      const timer = setTimeout(() => {
        if (currentPath.endsWith('.omni')) {
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
  };

  const compileErrors = useEditorStore((s) => s.compileErrors);
  const selectedId = useEditorStore((s) => s.selectedElementId);

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

  // ── Feature 1: Save/Load canvas state ───────────

  const handleSaveCanvas = useCallback(async (customPath?: string | React.MouseEvent) => {
    const store = useEditorStore.getState();
    const activeFile = store.activeFilePath;

    // If customPath is a string (from dialog), use it; otherwise check loadedCanvasPath
    const explicitPath = typeof customPath === "string" ? customPath : undefined;

    // If already loaded from a file, save directly without dialog
    let canvasPath = explicitPath || loadedCanvasPath;
    if (!canvasPath) {
      // Show save dialog
      const defaultName = activeFile
        ? activeFile.replace(/\.omni$/, ".canvas.json")
        : "canvas.json";
      setSaveDialog({ visible: true, defaultName });
      return;
    }

    const data = JSON.stringify(
      { elements: store.elements, wires: store.wires },
      null,
      2,
    );

    setStatus({ text: `保存到 ${canvasPath} ...`, isError: false });
    try {
      const result = await api.writeFile(canvasPath, data);
      if (result.ok) {
        setStatus({ text: `✅ 已保存: ${canvasPath}`, isError: false });
        setLoadedCanvasPath(canvasPath);
      } else {
        setStatus({ text: `❌ 保存失败: ${result.error || "未知错误"}`, isError: true });
      }
    } catch (e) {
      setStatus({ text: `❌ 保存失败: ${String(e)}`, isError: true });
    }
  }, [loadedCanvasPath]);

  const handleLoadCanvas = useCallback(async (filePath?: string | React.MouseEvent) => {
    // If a string is passed (from dialog click), use it directly
    const explicitPath = typeof filePath === "string" ? filePath : undefined;

    // If a specific file is given, load it directly
    if (explicitPath) {
      setStatus({ text: `加载 ${explicitPath} ...`, isError: false });
      try {
        const result = await api.readFile(explicitPath);
        if (!result.content) {
          setStatus({ text: `❌ 文件不存在: ${explicitPath}`, isError: true });
          return;
        }
        const data = JSON.parse(result.content);
        if (!data.elements || !data.wires) {
          setStatus({ text: "画布文件格式无效", isError: true });
          return;
        }
        const cmd = new SetElementsAndWires(data.elements, data.wires, `📂 加载: ${explicitPath}`);
        undoExec(cmd, ctx);
        setLoadedCanvasPath(explicitPath);
        setStatus({ text: `✅ 已加载: ${explicitPath}`, isError: false });
      } catch (e) {
        setStatus({
          text: `❌ 加载失败: ${e instanceof SyntaxError ? "文件格式错误" : String(e)}`,
          isError: true,
        });
      }
      setLoadDialog({ visible: false, files: [] });
      return;
    }

    // Scan for .omni.json files and show dialog (scans root + one level of subdirs)
    setStatus({ text: "扫描画布文件...", isError: false });
    try {
      const listing = await api.listDir(".");
      const entries = listing?.entries || [];
      // Collect .omni.json files from root
      let jsonFiles = entries
        .filter((e: { name: string }) => e.name.endsWith(".omni.json"))
        .map((e: { name: string }) => e.name);
      // Also scan one level of subdirectories for .omni.json files
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
        } catch { /* skip inaccessible subdirs */ }
      }

      if (jsonFiles.length === 0) {
        setStatus({ text: "❌ 未找到可加载的画布文件", isError: true });
        return;
      }
      setLoadDialog({ visible: true, files: jsonFiles });
    } catch {
      setStatus({ text: "❌ 扫描文件失败", isError: true });
    }
  }, [undoExec, ctx]);

  // ═════════════════════════════════════════════
  // Render
  // ═════════════════════════════════════════════

  return (
    <div style={S.app}>
      {/* M6: Read-only banner for hand-written files */}
      {readOnlyMode && (
        <ReadOnlyBanner
          warnings={handWrittenWarnings}
          source={activeFileContent || ''}
          onConverted={() => setHandWritten(false, [])}
        />
      )}

      {/* Mode toggle — top-left corner */}
      <div style={{ position: "absolute", top: 4, left: 4, zIndex: 100, display: "flex", gap: 2, pointerEvents: "none" }}>
        <button onClick={() => setMode("design")} style={{...S.modeBtn(mode === "design"), pointerEvents: "auto"}}>🎨 设计</button>
        <button onClick={() => setMode("code")} style={{...S.modeBtn(mode === "code"), pointerEvents: "auto"}}>💻 代码</button>
      </div>

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
            <BlockPalette />
          </PanelContainer>
          <main style={S.main}>
            <div style={S.toolbar}>
              <span style={{ fontWeight: 600, marginLeft: 110 }}>OmniLang 编辑器 — 代码</span>
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
            <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
              <div style={S.codeArea}>
                <FlowCanvas
                  ctx={ctx}
                  showViews={false}
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
              <span style={{ color: "#9ca3af" }}>{activeFilePath || "未打开"} {compileErrors.length > 0 && `— ${compileErrors.length} 条错误`}</span>
            </div>
          </main>
        </>
      ) : (
        /* ══════ 设计模式 ══════ */
        <>
          <aside style={{...S.sidebar, display: "flex", flexDirection: "column" }}>
            <div style={{ flex: showTokens ? "0 0 auto" : 1, overflow: showTokens ? "hidden" : "auto", maxHeight: showTokens ? "50%" : "100%" }}>
              <ComponentPalette />
            </div>
            {showTokens && (
              <div style={{ flex: 1, overflow: "auto", borderTop: "1px solid #e5e7eb" }}>
                <TokenPanel />
              </div>
            )}
            <div style={{ borderTop: "1px solid #e5e7eb" }}>
              {showAnimation ? (
                <AnimationImport
                  onImportLottie={(data, fileName) => {
                    const code = generateAnimationCode("lottie", fileName, data);
                    if (codeEditorRef.current) {
                      codeEditorRef.current.insertSnippet(code);
                      setShowAnimation(false);
                    }
                  }}
                  onImportRive={(data, fileName) => {
                    const code = generateAnimationCode("rive", fileName, {});
                    if (codeEditorRef.current) {
                      codeEditorRef.current.insertSnippet(code);
                      setShowAnimation(false);
                    }
                  }}
                  onInsertCode={(code) => {
                    codeEditorRef.current?.insertSnippet(code);
                    setShowAnimation(false);
                  }}
                />
              ) : (
                <button
                  onClick={() => setShowAnimation(true)}
                  style={{
                    width: "100%", padding: "6px 8px", fontSize: 11, fontWeight: 600,
                    background: "#FAFAFA", border: "none", cursor: "pointer",
                    color: "#6B7280", display: "flex", alignItems: "center", gap: 6,
                  }}
                >
                  🎬 导入动画…
                </button>
              )}
            </div>
          </aside>
          <main style={S.main}>
            <div style={S.toolbar}>
              <span style={{ fontWeight: 600, marginLeft: 110 }}>OmniLang 编辑器 — 设计</span>
              {projectDir && <span style={{ color: "#9ca3af", fontSize: 11 }}>{projectDir.split(/[/\\]/).slice(-2).join("/")}</span>}
              <span style={{ flex: 1 }} />
              <span style={{ fontSize: 10, color: "#9CA3AF" }}>视口:</span>
              {(["desktop","tablet","mobile"] as const).map(bp => (
                <button key={bp} onClick={() => setBreakpoint(bp)}
                  style={{ padding: "2px 8px", fontSize: 10, border: "1px solid", borderRadius: 3, cursor: "pointer",
                    background: breakpoint === bp ? "#4F46E5" : "#fff",
                    color: breakpoint === bp ? "#fff" : "#6B7280",
                    borderColor: breakpoint === bp ? "#4F46E5" : "#d1d5db" }}>
                  {{desktop:"🖥️ 桌面",tablet:"📱 平板",mobile:"📱 手机"}[bp]}
                </button>
              ))}
              <button onClick={() => setShowTokens(!showTokens)}
                style={{...btnStyle, background: showTokens ? "#EEF2FF" : "#fff", borderColor: showTokens ? "#4F46E5" : "#d1d5db" }}>
                {showTokens ? "🎨 令牌" : "🎨 令牌"}
              </button>
              <button onClick={() => setWireMode(!wireMode)}
                style={{...btnStyle, background: wireMode ? "#EEF2FF" : "#fff", borderColor: wireMode ? "#4F46E5" : "#d1d5db" }}>
                {wireMode ? "🔗 连线" : "🔗 连线"}
              </button>
              <button onClick={() => setShowHUD(!showHUD)}
                style={{...btnStyle, background: showHUD ? "#EEF2FF" : "#fff", borderColor: showHUD ? "#4F46E5" : "#d1d5db", fontSize: 11 }}>
                {showHUD ? "🔍 状态" : "🔍 状态"}
              </button>
              {/* M7: Component state preview */}
              <select
                value={previewState}
                onChange={(e) => setPreviewState(e.target.value)}
                style={{ padding: "3px 6px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4, background: "#fff" }}
                title="Component state preview"
              >
                <option value="normal">👁 Normal</option>
                <option value="hover">🖱 Hover</option>
                <option value="active">👆 Active</option>
                <option value="focus">🎯 Focus</option>
                <option value="disabled">🚫 Disabled</option>
                <option value="loading">⏳ Loading</option>
              </select>
              <button onClick={() => setShowFocusOrder(!showFocusOrder)}
                style={{...btnStyle, background: showFocusOrder ? "#EEF2FF" : "#fff", borderColor: showFocusOrder ? "#4F46E5" : "#d1d5db", fontSize: 11 }}>
                {showFocusOrder ? "↹ 焦点" : "↹ 焦点"}
              </button>
              <button onClick={handleDeploy}
                style={{...btnStyle, background: "#FFF7ED", borderColor: "#F97316", color: "#EA580C" }}>
                🚀 发布
              </button>
              <button onClick={handleSaveCanvas}
                style={{...btnStyle, background: "#fff"}}>
                💾 保存画布
              </button>
              <button onClick={handleLoadCanvas}
                style={{...btnStyle, background: "#fff"}}>
                📂 加载画布
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
            <div style={{ display: "flex", justifyContent: "center", background: "#f0f0f0", flex: 1, overflow: "auto" }}>
              <div style={{
                width: breakpoint === "desktop" ? "100%" : breakpoint === "tablet" ? 768 : 375,
                maxWidth: "100%",
                background: "#fff",
                boxShadow: "0 0 20px rgba(0,0,0,0.1)",
                position: "relative",
                transition: "width 0.3s ease",
                display: "flex", flexDirection: "column",
              }}>
              <LayoutView ctx={ctx} hideCodeBlocks />
              {wireMode && <WireOverlay />}
              {/* State HUD (M5.2) */}
              {selectedId && <StateHUD elementId={selectedId} visible={showHUD} onClose={() => setShowHUD(false)} />}
              {/* Focus order overlay (M5.4) */}
              {showFocusOrder && <FocusOrderOverlay />}
            </div>
            </div>
            <div style={S.statusBar}>
              <span style={{ color: status.isError ? "#DC2626" : status.text.startsWith("⚠") ? "#D97706" : "#6B7280" }}>{status.text}</span>
              <span style={{ color: "#9ca3af" }}>{activeFilePath || "未打开"}</span>
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
      {conflictInfo && (
        <ConflictPanel
          conflict={conflictInfo}
          onResolve={(choice) => {
            if (choice === "merge") setMode("code");
            setShowConflict(false);
            setConflictInfo(null);
          }}
          onDismiss={() => { setShowConflict(false); setConflictInfo(null); }}
        />
      )}

      {/* ── Save Dialog ────────────────────────── */}
      {saveDialog.visible && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)", zIndex: 10000,
          display: "flex", alignItems: "center", justifyContent: "center",
        }} onClick={() => setSaveDialog({ visible: false, defaultName: "" })}>
          <div style={{
            background: "#fff", borderRadius: 12, padding: "24px 28px", minWidth: 380,
            boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          }} onClick={e => e.stopPropagation()}>
            <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 16 }}>
              💾 保存画布状态
            </div>
            <input
              id="save-filename-input"
              defaultValue={saveDialog.defaultName}
              placeholder="输入文件名..."
              style={{
                width: "100%", padding: "8px 12px", fontSize: 14, border: "1px solid #d1d5db",
                borderRadius: 6, boxSizing: "border-box", marginBottom: 16,
              }}
              onKeyDown={e => {
                if (e.key === "Enter") {
                  const input = (e.target as HTMLInputElement).value.trim();
                  if (input) {
                    setSaveDialog({ visible: false, defaultName: "" });
                    handleSaveCanvas(input);
                  }
                }
                if (e.key === "Escape") setSaveDialog({ visible: false, defaultName: "" });
              }}
              autoFocus
            />
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button onClick={() => setSaveDialog({ visible: false, defaultName: "" })}
                style={{ padding: "6px 16px", fontSize: 13, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer" }}>
                取消
              </button>
              <button onClick={() => {
                const input = (document.getElementById("save-filename-input") as HTMLInputElement)?.value.trim();
                if (input) {
                  setSaveDialog({ visible: false, defaultName: "" });
                  handleSaveCanvas(input);
                }
              }}
                style={{ padding: "6px 16px", fontSize: 13, border: "none", borderRadius: 6, background: "#4F46E5", color: "#fff", cursor: "pointer" }}>
                保存
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Load Dialog ────────────────────────── */}
      {loadDialog.visible && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)", zIndex: 10000,
          display: "flex", alignItems: "center", justifyContent: "center",
        }} onClick={() => setLoadDialog({ visible: false, files: [] })}>
          <div style={{
            background: "#fff", borderRadius: 12, padding: "24px 28px", minWidth: 420, maxHeight: "70vh",
            boxShadow: "0 8px 32px rgba(0,0,0,0.2)", display: "flex", flexDirection: "column",
          }} onClick={e => e.stopPropagation()}>
            <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 12 }}>
              📂 加载画布状态
            </div>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 12 }}>
              选择要恢复的画布快照文件
            </div>
            <div style={{ flex: 1, overflow: "auto", marginBottom: 12 }}>
              {loadDialog.files.map((f, i) => (
                <div key={f}
                  onClick={() => handleLoadCanvas(f)}
                  style={{
                    padding: "10px 14px", cursor: "pointer", borderRadius: 6,
                    display: "flex", alignItems: "center", gap: 10,
                    transition: "background 0.15s", borderBottom: i < loadDialog.files.length - 1 ? "1px solid #f3f4f6" : "none",
                  }}
                  onMouseEnter={e => (e.currentTarget.style.background = "#F5F3FF")}
                  onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
                >
                  <span style={{ fontSize: 20 }}>📄</span>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 14, fontWeight: 500, color: "#374151" }}>{f}</div>
                  </div>
                  <span style={{ fontSize: 11, color: "#9ca3af" }}>点击加载</span>
                </div>
              ))}
            </div>
            <button onClick={() => setLoadDialog({ visible: false, files: [] })}
              style={{ padding: "6px 16px", fontSize: 13, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer", alignSelf: "flex-end" }}>
              取消
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════

const btnStyle: React.CSSProperties = { padding: "4px 12px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 4, background: "#fff", cursor: "pointer" };

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

// ── M5.5 Animation code generator ─────────────

function generateAnimationCode(type: "lottie" | "rive", fileName: string, _data: Record<string, unknown>): string {
  const baseName = fileName.replace(/\.(json|riv)$/i, "");
  const compName = baseName.replace(/[-_\s]+/g, " ").split(" ").map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase()).join("") + "Animation";

  if (type === "lottie") {
    return `// @editor: managed
component ${compName} {
    signal src: String = "/animations/${fileName}";
    signal playing: bool = true;
    signal loop: bool = true;
    signal speed: double = 1.0;

    view {
        <LottiePlayer src={src} playing={playing} loop={loop} speed={speed} />
    }
}`;
  }
  return `// @editor: managed
component ${compName} {
    signal src: String = "/animations/${fileName}";
    signal artboard: String = "default";
    signal stateMachine: String = "default";

    view {
        <RivePlayer src={src} artboard={artboard} stateMachine={stateMachine} />
    }
}`;
}
