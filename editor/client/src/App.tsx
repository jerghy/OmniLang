import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import { useEditorStore } from "./store/editor";
import { useUndoStore } from "./store/undo";
import { useTokenStore } from "./ui/panels/TokenPanel";
import { S } from "./ui/shell/theme";
import { CodeEditorPopup } from "./ui/editor/CodeEditorPopup";
import { SearchModal } from "./ui/shared/SearchModal";
import { ProjectWelcome } from "./ui/panels/ProjectWelcome";
import { saveProject as saveProjectToServer } from "./services/project/project-service";
import { api } from "./services/api";
import { createCommandContext } from "./core/commands/createCommandContext";
import type { SearchResult } from "./services/parser/search-index";
import type { PanelId } from "./ui/panels/ActivityBar";

import { useProjectInit } from "./features/project-explorer/useProjectInit";
import { useDesignKeyboard } from "./features/design-mode/useDesignKeyboard";
import { useCodeKeyboard } from "./features/code-mode/useCodeKeyboard";
import { useCompile } from "./features/compiler/useCompile";
import { useRun } from "./features/compiler/useRun";
import { useAutoCompile } from "./features/compiler/useAutoCompile";
import { useDebugSession } from "./features/debug/useDebugSession";
import { DesignModeLayout } from "./features/design-mode/DesignModeLayout";
import { CodeModeLayout } from "./features/code-mode/CodeModeLayout";

export default function App() {
  // ── Core mode / welcome ──
  const [mode, setMode] = useState<"design" | "code">("design");
  const [showWelcome, setShowWelcome] = useState(true);

  // ── Local UI state ──
  const [status, setStatus] = useState<{ text: string; isError: boolean }>({ text: "就绪", isError: false });
  const [target, setTarget] = useState("react");
  const [showSearch, setShowSearch] = useState(false);
  const [showConflict, setShowConflict] = useState(false);
  const [activePanel, setActivePanel] = useState<PanelId>(null);
  const [wireMode, setWireMode] = useState(false);
  const [showSnippets, setShowSnippets] = useState(true);
  const [showConnector, setShowConnector] = useState(false);
  const [breakpoint, setBreakpoint] = useState<"desktop" | "tablet" | "mobile">("desktop");
  const [showTokens, setShowTokens] = useState(false);
  const [showHUD, setShowHUD] = useState(false);
  const [showFocusOrder, setShowFocusOrder] = useState(false);
  const [previewState, setPreviewState] = useState("normal");
  const [layoutTrigger, setLayoutTrigger] = useState(0);
  const [highlightedWireId, setHighlightedWireId] = useState<string | null>(null);
  const [codePopup, setCodePopup] = useState<{
    visible: boolean;
    blockId: string | null;
    title: string;
    source: string;
    blockElement: import("./store/editor").ElementData | null;
  }>({ visible: false, blockId: null, title: "", source: "", blockElement: null });
  const [openDialog, setOpenDialog] = useState<{ visible: boolean; files: string[] }>({ visible: false, files: [] });
  const codeEditorRef = useRef<{ insertSnippet: (code: string) => void }>(null);

  // ── Store values ──
  const projectDir = useEditorStore((s) => s.projectDir);
  const isDirty = useEditorStore((s) => s.isDirty);
  const projectName = useEditorStore((s) => s.projectName);
  const compileErrors = useEditorStore((s) => s.compileErrors);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const selectedId = selectedIds.length > 0 ? selectedIds[0] : null;
  const setCompileErrors = useEditorStore((s) => s.setCompileErrors);
  const setElementSource = useEditorStore((s) => s.setElementSource);

  // ── Undo / Redo ──
  const undoExec = useUndoStore((s) => s.execute);
  const undoUndo = useUndoStore((s) => s.undo);
  const undoRedo = useUndoStore((s) => s.redo);

  // ── Command context ──
  const ctx = useMemo(() => createCommandContext(), []);

  // ── Feature hooks ──
  const { handleProjectReady } = useProjectInit(setShowWelcome);
  const { runStatus, runLoading, fnToNodeMap, debugState, handleRun, handleStop,
    handleClearDebug, handleToggleBreakpoint, handleStep, handleResume,
    setFnToNodeMap, setDebugConnected, setDebugStateChanges,
    setActiveNodeIds, setActiveEdgeIds, setDebugPaused } = useRun(target, setStatus);
  const { handleCompile } = useCompile(ctx, target, runStatus, setFnToNodeMap, setStatus);
  useDesignKeyboard(ctx, {
    mode,
    setShowSearch: (v: boolean) => setShowSearch(v),
    setActivePanel: (fn: (prev: string | null) => string | null) => setActivePanel(fn as any),
    setShowHUD: (v: boolean) => setShowHUD(v),
    setShowConflict: (v: boolean) => setShowConflict(v),
  });
  useCodeKeyboard({
    ctx, handleCompile,
    saveProject: () => saveProjectToServer(undefined, (text, isErr) => setStatus({ text, isError: isErr })),
    undoUndo, undoRedo,
    searchProject: () => setShowSearch(true),
  });
  useDebugSession(runStatus.running,
    setDebugConnected as (v: boolean) => void,
    setDebugStateChanges as any,
    setActiveNodeIds as any,
    setActiveEdgeIds as any,
    setDebugPaused as (v: boolean) => void,
    setFnToNodeMap, fnToNodeMap);
  useAutoCompile(handleCompile, setCompileErrors as (errors: unknown[]) => void, setStatus);

  // ── Handlers ──

  const handleSearchSelect = useCallback((result: SearchResult) => {
    switch (result.action.type) {
      case "open-file":
        api.project.readFile(result.action.payload).then((r) => {
          if (r.content) useEditorStore.getState().openFile(result.action.payload, r.content);
        });
        break;
      case "insert-snippet":
        codeEditorRef.current?.insertSnippet(result.action.payload);
        setMode("code");
        break;
      case "select-element": {
        const el = useEditorStore.getState().elements.find((e) => e.tag === result.action.payload);
        if (el) useEditorStore.getState().selectElement(el.id);
        setMode("design");
        break;
      }
      case "run-command":
        if (result.action.payload === "compile") handleCompile();
        else if (result.action.payload === "toggle-wire") setWireMode((p) => !p);
        else if (result.action.payload === "toggle-snippets") setShowSnippets((p) => !p);
        else if (result.action.payload === "toggle-tokens") setShowTokens((p) => !p);
        else if (result.action.payload === "focus-order") setShowFocusOrder((p) => !p);
        break;
    }
  }, [handleCompile]);

  const handleOpenProject = useCallback(async () => {
    setStatus({ text: "扫描 .omni 文件...", isError: false });
    try {
      const omniFiles: string[] = [];
      const scanned = new Set<string>();
      const scanDir = async (dirPath: string, depth: number): Promise<void> => {
        if (depth > 3 || scanned.has(dirPath)) return;
        scanned.add(dirPath);
        try {
          const listing = await api.project.listDir(dirPath);
          for (const e of listing?.entries || []) {
            if (e.name.endsWith(".omni")) omniFiles.push(dirPath === "." ? e.name : `${dirPath}/${e.name}`);
            if (depth < 3 && e.type === "directory") {
              await scanDir(dirPath === "." ? e.name : `${dirPath}/${e.name}`, depth + 1);
            }
          }
        } catch { /* skip */ }
      };
      await scanDir(".", 1);
      if (omniFiles.length === 0) { setStatus({ text: "未找到 .omni 文件", isError: true }); return; }
      setOpenDialog({ visible: true, files: omniFiles });
    } catch { setStatus({ text: "扫描失败", isError: true }); }
  }, []);

  const openProjectFile = useCallback(async (filePath: string) => {
    setOpenDialog({ visible: false, files: [] });
    try {
      const result = await api.annotation.annotatedLoad(filePath);
      if (!result.success || !result.source) {
        setStatus({ text: `❌ 加载失败: ${result.error}`, isError: true });
        return;
      }
      const store = useEditorStore.getState();
      store.newProject();
      useEditorStore.setState({
        activeFileContent: result.source, activeFilePath: filePath,
        projectPath: projectDir || filePath,
        projectName: filePath.replace(/^.*[\\/]/, "").replace(/\.omni$/, ""),
      });
      const { parseOmniSource } = await import("./services/codegen");
      const parseResult = parseOmniSource(result.source);
      if (parseResult.designElements) {
        const rootOrder = parseResult.designElements.filter((e) => !e.parentId).map((e) => e.id);
        useEditorStore.setState({ elements: parseResult.designElements, wires: parseResult.designWires || [], rootOrder });
      }
      setStatus({ text: `✅ 已打开: ${filePath}`, isError: false });
    } catch (e) { setStatus({ text: `❌ 加载失败: ${String(e)}`, isError: true }); }
  }, [projectDir]);

  const handleNavigateToCode = useCallback((wireId: string) => {
    setMode("code");
    setHighlightedWireId(wireId);
  }, []);

  // ── OmniDriver init ──
  useEffect(() => {
    import("./driver").then(({ OmniDriver }) => {
      const omni = new OmniDriver();
      (window as any).__OMNI__ = omni;
    }).catch(() => {});
    (window as any).__OMNI_MODE__ = mode;
    (window as any).__OMNI_TARGET__ = target;
    const onModeChange = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.mode) setMode(detail.mode);
    };
    window.addEventListener("omni-mode-change", onModeChange);
    return () => window.removeEventListener("omni-mode-change", onModeChange);
  }, []);

  // ── Watch WS connection ──
  useEffect(() => {
    if (!projectDir) return;
    api.connectWatch(() => {}, (msg, isErr) => setStatus({ text: String(msg), isError: Boolean(isErr) }));
    return () => api.disconnect();
  }, [projectDir]);

  // ── Render ──
  if (showWelcome) {
    return <ProjectWelcome onProjectReady={handleProjectReady} />;
  }

  const toolbarProps = {
    mode, isDirty, projectDir, runStatus, runLoading, status, compileErrors, projectName,
    onModeChange: setMode, onUndo: () => undoUndo(ctx), onRedo: () => undoRedo(ctx),
    onSave: () => saveProjectToServer(undefined, (text, isErr) => setStatus({ text, isError: isErr })),
    onRun: handleRun, onStop: handleStop, onCompile: handleCompile,
  };

  return (
    <div style={S.app}>
      {mode === "code" ? (
        <CodeModeLayout
          ctx={ctx} mode={mode} toolbarProps={toolbarProps}
          activePanel={activePanel} showConnector={showConnector}
          target={target} debugState={debugState} fnToNodeMap={fnToNodeMap}
          highlightedWireId={highlightedWireId} layoutTrigger={layoutTrigger}
          codeEditorRef={codeEditorRef}
          onActivePanelChange={setActivePanel} onTargetChange={setTarget}
          onToggleConnector={() => setShowConnector((p) => !p)}
          onAutoLayout={() => setLayoutTrigger((v) => v + 1)}
          onNodeDoubleClick={(id, el) => {
            const blockSource = el.source || "";
            setCodePopup({
              visible: true, blockId: id,
              title: `${(el.props as Record<string, unknown>)?.name || "Code"} (${el.tag})`,
              source: blockSource, blockElement: el,
            });
          }}
          onClearDebug={handleClearDebug} onToggleBreakpoint={handleToggleBreakpoint}
          onStep={handleStep} onResume={handleResume}
          onFnDoubleClick={(fnId, fnData) => {
            const el = useEditorStore.getState().elements.find((e) => e.id === fnId);
            setCodePopup({
              visible: true, blockId: fnId,
              title: `📦 ${fnData.name} (${fnData.params}) → ${fnData.returnType}`,
              source: fnData.source as string, blockElement: el || null,
            });
          }}
        />
      ) : (
        <DesignModeLayout
          ctx={ctx} mode={mode} toolbarProps={toolbarProps}
          breakpoint={breakpoint} previewState={previewState}
          wireMode={wireMode} showTokens={showTokens} showHUD={showHUD}
          showFocusOrder={showFocusOrder} showSnippets={showSnippets}
          showConnector={showConnector} activePanel={activePanel}
          selectedId={selectedId}
          onBreakpointChange={setBreakpoint}
          onPreviewStateChange={setPreviewState}
          onToggleTokens={() => setShowTokens((p) => !p)}
          onToggleWireMode={() => setWireMode((p) => !p)}
          onToggleHUD={() => setShowHUD((p) => !p)}
          onToggleFocusOrder={() => setShowFocusOrder((p) => !p)}
          onOpenProject={handleOpenProject}
          onActivePanelChange={setActivePanel}
          handleNavigateToCode={handleNavigateToCode}
        />
      )}

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
            const fnMatch = newSource.match(/fn\s+(\w+)\s*\(/);
            if (fnMatch) useEditorStore.getState().setProp(codePopup.blockId, "name", fnMatch[1]);
          }
          setCodePopup({ visible: false, blockId: null, title: "", source: "", blockElement: null });
        }}
      />

      {openDialog.visible && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)", zIndex: 10000, display: "flex", alignItems: "center", justifyContent: "center" }}
          onClick={() => setOpenDialog({ visible: false, files: [] })}>
          <div style={{ background: "#fff", borderRadius: 12, padding: "24px 28px", minWidth: 420, maxHeight: "70vh", boxShadow: "0 8px 32px rgba(0,0,0,0.2)", display: "flex", flexDirection: "column" }}
            onClick={(e) => e.stopPropagation()}>
            <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 12 }}>📂 打开 .omni 文件</div>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 12 }}>选择一个 .omni 源文件打开</div>
            <div style={{ flex: 1, overflow: "auto", marginBottom: 12 }}>
              {openDialog.files.map((f, i) => (
                <div key={f} onClick={() => openProjectFile(f)}
                  style={{ padding: "10px 14px", cursor: "pointer", borderRadius: 6, display: "flex", alignItems: "center", gap: 10,
                    borderBottom: i < openDialog.files.length - 1 ? "1px solid #f3f4f6" : "none",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "#F5F3FF")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}>
                  <span style={{ fontSize: 20 }}>📄</span>
                  <div style={{ flex: 1 }}><div style={{ fontSize: 14, fontWeight: 500, color: "#374151" }}>{f}</div></div>
                  <span style={{ fontSize: 11, color: "#9ca3af" }}>点击打开</span>
                </div>
              ))}
            </div>
            <button onClick={() => setOpenDialog({ visible: false, files: [] })}
              style={{ padding: "6px 16px", fontSize: 13, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer", alignSelf: "flex-end" }}>取消</button>
          </div>
        </div>
      )}

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
