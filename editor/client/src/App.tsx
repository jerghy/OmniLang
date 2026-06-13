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

    const result = await api.compile(content, target);
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
    }
  }, [target]);

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
      if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); const p = useEditorStore.getState().activeFilePath; const c = useEditorStore.getState().activeFileContent; if (p && c) { api.writeFile(p, c); autoSaveOmniJson(c); setStatus({text:"已保存",isError:false}); } }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "F") { e.preventDefault(); setShowSearch(true); }
      // Activity Bar panel shortcuts
      if ((e.ctrlKey || e.metaKey) && e.key === "1") { e.preventDefault(); setActivePanel(p => p === "files" ? null : "files"); }
      if ((e.ctrlKey || e.metaKey) && e.key === "2") { e.preventDefault(); setActivePanel(p => p === "state" ? null : "state"); }
      if ((e.ctrlKey || e.metaKey) && e.key === "3") { e.preventDefault(); setActivePanel(p => p === "functions" ? null : "functions"); }
      if (e.key === "Escape") { setShowHUD(false); setShowSearch(false); setShowConflict(false); setActivePanel(null); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleCompile, undoUndo, undoRedo]);

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
                {["react","vue","dioxus","flutter","python","javascript","rust","dart"].map(t => <option key={t}>{t}</option>)}
              </select>
              <button onClick={() => setShowConnector(!showConnector)}
                style={{...btnStyle, background: showConnector ? "#EEF2FF" : "#fff", borderColor: showConnector ? "#4F46E5" : "#d1d5db" }}>
                {showConnector ? "🔌 连接器" : "🔌 连接器"}
              </button>
              <button onClick={handleCompile} style={btnStyle}>编译 (Ctrl+Enter)</button>
            </div>
            <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
              <div style={S.codeArea}>
                <FlowCanvas
                  ctx={ctx}
                  showViews={false}
                  highlightedWireId={highlightedWireId}
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
