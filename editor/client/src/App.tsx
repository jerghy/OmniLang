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
import { ComponentTree } from "./panels/ComponentTree";
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
import { useComponentRegistry } from "./store/componentRegistry";
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
import { parseOmniSource, generateOmniSource, emitCustomComponents, validate } from "./omni-gen";
import type { ElementData, WireData } from "./core/types";
import type { ComponentDefinition } from "./store/componentRegistry";
import { ProjectWelcome } from "./panels/ProjectWelcome";
import { compilerCache } from "./omni-compiler-cache";

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
// Custom Component Save/Load Helpers
// ═══════════════════════════════════════════════

/**
 * Convert registry definitions to omni DSL and save to server.
 * Each component's elements/wires are emitted as proper .omni source
 * with metadata stored as // @component / @prop / @event / @style comments (no more .meta.json).
 */
async function saveComponentDefinitionsToServer(defs: Record<string, ComponentDefinition>): Promise<void> {
  const components: Array<{ name: string; elements: ElementData[]; meta?: Record<string, unknown> }> = [];

  for (const [name, def] of Object.entries(defs)) {
    components.push({
      name,
      elements: def.elements || [],
      meta: {
        id: def.id,
        props: def.props,
        events: def.events,
        styleProperties: def.styleProperties,
        category: def.category,
        description: def.description,
        children: def.children,
      },
    });
  }

  const result = emitCustomComponents(components);
  if (!result.value) return;

  const payload: Record<string, { omniSource: string }> = {};
  for (const [name] of Object.entries(defs)) {
    const omniSource = result.value[`src/components/${name}.omni`] || "";
    if (omniSource) payload[name] = { omniSource };
  }

  await api.saveComponentDefinitions(payload as unknown as Record<string, unknown>);
}

/**
 * Load custom component definitions from server.
 * Parses .omni source back to elements/wires; metadata is extracted from
 * // @component / @prop / @event / @style comments in the source (no more .meta.json).
 */
async function loadComponentDefinitionsFromServer(): Promise<Record<string, unknown>> {
  const result = await api.loadComponentDefinitions();
  const merged: Record<string, unknown> = {};

  for (const [name, raw] of Object.entries(result.definitions)) {
    const data = raw as Record<string, unknown>;
    const source = data.source as string || "";
    if (!source) continue;

    const parsed = parseOmniSource(source);
    const meta = parsed.componentMeta || {};

    // Elements/wires come from parsed view tree
    const elements = parsed.designElements && parsed.designElements.length > 0
      ? parsed.designElements
      : [];
    const wires = parsed.designWires || [];

    // Props schema from comment metadata (// @prop), fallback to DSL declarations
    const propSchema: Record<string, unknown> = (meta.propSchema as Record<string, unknown>) || {};
    const dslProps = parsed.props || [];
    for (const p of dslProps) {
      if (!propSchema[p.name]) {
        propSchema[p.name] = { type: p.type, default: p.default, editor: "text" };
      }
    }

    // Events schema from comment metadata (// @event), fallback to DSL declarations
    const eventSchema: Record<string, unknown> = (meta.eventSchema as Record<string, unknown>) || {};
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
  const [showWelcome, setShowWelcome] = useState(true);
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
      // Show welcome screen instead of auto-creating project
      setShowWelcome(true);
    })();
  }, []);

  const handleProjectReady = useCallback((dir: string, name: string) => {
    setProjectDir(dir);
    // Reset editor state first, then set project identity
    useEditorStore.getState().newProject();
    useEditorStore.setState({ projectName: name, projectPath: dir });

    // Load main.omni via annotatedLoad (extracts annotations from .omni comments)
    (async () => {
      try {
        const loadResult = await api.annotatedLoad("src/main.omni");
        if (loadResult.success && loadResult.source) {
          useEditorStore.setState({
            activeFileContent: loadResult.source,
            activeFilePath: "src/main.omni",
          });

          // Parse .omni source into design elements
          const parseResult = parseOmniSource(loadResult.source);
          if (parseResult.designElements) {
            const rootOrder = parseResult.designElements
              .filter((e: ElementData) => !e.parentId)
              .map((e: ElementData) => e.id);
            useEditorStore.setState({
              elements: parseResult.designElements,
              wires: parseResult.designWires || [],
              rootOrder,
            });
            if (parseResult.codeTabs?.length) {
              useCodeGraphStore.getState().loadFromData(parseResult.codeTabs);
            }
          }

          // Restore view markers (folded state, render mode)
          if (loadResult.annotations?.viewMarkers) {
            const markers = loadResult.annotations.viewMarkers;
            useEditorStore.setState((prev) => ({
              elements: prev.elements.map((el) => {
                const marker = markers.find((m: any) => m.elementId === el.id);
                if (marker) {
                  return {
                    ...el,
                    isFolded: marker.isFolded ?? el.isFolded,
                    renderMode: marker.renderMode || el.renderMode,
                  };
                }
                return el;
              }),
            }));
          }

          // Restore code graph chains if available
          if (loadResult.annotations?.chains?.length) {
            const chains = loadResult.annotations.chains;
            const codeElements: any[] = [];
            const codeWires: any[] = [];
            for (const chain of chains) {
              for (const node of chain.nodes || []) {
                codeElements.push({
                  id: node.id,
                  tag: node.tag,
                  props: node.attrs || {},
                  position: { x: node.posX, y: node.posY },
                  nodeType: "codeblock",
                  children: [],
                  parentId: null,
                });
              }
              for (const wire of chain.wires || []) {
                codeWires.push({
                  id: wire.id,
                  fromId: wire.fromId,
                  fromPort: wire.fromPort,
                  toId: wire.toId,
                  toPort: wire.toPort,
                });
              }
            }
            if (codeElements.length > 0) {
              useCodeGraphStore.getState().setElementsAndWires(codeElements, codeWires);
            }
          }
        } else {
          console.warn('[App] annotatedLoad failed:', loadResult.error);
          // Fallback: try plain read
          const res = await api.readFile("src/main.omni");
          if (res.content) {
            useEditorStore.setState({ activeFileContent: res.content, activeFilePath: "src/main.omni" });
            const parseResult = parseOmniSource(res.content);
            if (parseResult.designElements) {
              const rootOrder = parseResult.designElements
                .filter((e: ElementData) => !e.parentId)
                .map((e: ElementData) => e.id);
              useEditorStore.setState({ elements: parseResult.designElements, wires: parseResult.designWires || [], rootOrder });
            }
          }
        }
      } catch (e) {
        console.warn('[App] Failed to load main.omni:', e);
      }
    })();

    // Load custom component definitions from src/components/*.omni
    (async () => {
      try {
        const merged = await loadComponentDefinitionsFromServer();
        if (Object.keys(merged).length > 0) {
          const { useComponentRegistry } = await import("./store/componentRegistry");
          useComponentRegistry.setState({ definitions: merged } as any);
          console.log(`[App] Loaded ${Object.keys(merged).length} custom component definitions`);
        }
      } catch (e) {
        console.warn('[App] Failed to load component definitions:', e);
      }
    })();

    // Load editor UI state from .omni-editor.toml
    (async () => {
      try {
        const tomlResult = await api.loadEditorToml();
        if (tomlResult.state) {
          const s = tomlResult.state;
          if (s.open_files?.length > 0) {
            useEditorStore.setState({ openTabs: s.open_files });
          }
        }
      } catch { /* non-critical */ }
    })();

    setShowWelcome(false);
    setStatus({ text: `📂 已打开 ${name}`, isError: false });
  }, [setProjectDir]);

  // ── Initial compile for canvas (client-side, via compilerCache) ──
  // Handled by LayoutView's effect chain: wait for wasmReady → recompile.
  // App.tsx no longer triggers compilation to avoid WASM-not-ready races.

  // ── Store subscription: recompile on element/wire changes ──
  // NOTE: LayoutView handles its own compile subscription with WASM readiness
  // This App-level subscription is intentionally omitted to avoid race conditions
  // with WASM initialization. LayoutView's subscription waits for wasmReady.

  // Auto-save component definitions (as omni DSL) when registry changes
  useEffect(() => {
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    const unsub = useComponentRegistry.subscribe((state) => {
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveTimer = null;
        // Always save, even when empty — server cleanup deletes orphaned .omni files
        saveComponentDefinitionsToServer(state.definitions).catch(() => {
          // Best-effort auto-save; failures are non-critical
        });
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

  // Connect watch WS when a project is active (starts after welcome flow)
  useEffect(() => {
    if (!projectDir) return;
    api.connectWatch(
      (msg, isErr) => setStatus({ text: msg, isError: isErr })
    );
    return () => api.disconnect();
  }, [projectDir]);

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
      rootOrder: store.rootOrder,
    });

    let content = store.activeFileContent;
    if (genResult.value) {
      const validationResult = validate(genResult.value);
      if (!validationResult.valid) {
        // Fallback to original content when generated source fails validation
        setCompileErrors(validationResult.errors);
        setStatus({ text: "⚠️ 生成的代码验证失败，使用原始内容编译", isError: false });
        content = store.activeFileContent;
      } else {
        // Use generated source for compilation and update editor content
        content = genResult.value;
        store.setSourceCode(content);
      }
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
      // Update canvas preview via client-side WASM compile
      const s = useEditorStore.getState();
      if (s.elements.length > 0) {
        const cg = useCodeGraphStore.getState();
        const ct = cg.toSerializable().tabs;
        const out = compilerCache.compileProject({
          designElements: s.elements, designWires: s.wires, codeTabs: ct, componentName,
          rootOrder: s.rootOrder,
        }, true);
        if (out) window.dispatchEvent(new CustomEvent("omni-srcdoc-update", { detail: { srcdoc: out.srcdoc } }));
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

  // ── Save: 注解直写 .omni + 编辑器状态 → .omni-editor.toml ──

  /** Save a single file entry: generate .omni source from elements/wires and write to disk. */
  const saveFileEntry = useCallback(async (
    entry: { filePath: string; elements: ElementData[]; wires: WireData[]; rootOrder: string[] },
    codeTabs: any,
    store: ReturnType<typeof useEditorStore.getState>,
  ): Promise<boolean> => {
    const compMatch = (store.activeFileContent || store.sourceCode || "").match(/component\s+(\w+)\s*\{/);
    const componentName = compMatch ? compMatch[1] : "App";

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

    const writeRes = await api.writeFile(entry.filePath, content);
    if (!writeRes.ok) {
      setStatus({ text: `⚠ ${entry.filePath} 写入失败: ${writeRes.error || "未知错误"}`, isError: false });
      return false;
    }
    return true;
  }, [setStatus]);

  const saveProject = useCallback(async (customPath?: string) => {
    const store = useEditorStore.getState();
    const filePath = customPath || store.activeFilePath;

    if (!filePath) {
      setStatus({ text: "请先打开或创建一个 .omni 文件", isError: true });
      return;
    }

    setStatus({ text: `保存到 ${filePath} ...`, isError: false });
    try {
      const cg = useCodeGraphStore.getState();
      const codeTabs = cg.toSerializable();

      // ── Collect all file states that need saving ──

      interface FileSaveEntry {
        filePath: string;
        elements: ElementData[];
        wires: WireData[];
        rootOrder: string[];
      }

      const entries: FileSaveEntry[] = [];
      const seenPaths = new Set<string>();

      // 1. Active state: main canvas or active file tab
      if (store.activeDocId === null || store.activeDocId === "main") {
        // Main canvas: use activeFilePath (the .omni file that was opened)
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
            rootOrder: store.docBackup.elements.filter((e) => !e.parentId).map((e) => e.id),
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
              elements: state.elements,
              wires: state.wires,
              rootOrder: state.elements.filter((e) => !e.parentId).map((e) => e.id),
            });
            seenPaths.add(path);
          }
        }
      }

      // ── Save each file ──

      let savedCount = 0;
      let activeContent = "";
      for (const entry of entries) {
        const ok = await saveFileEntry(entry, codeTabs, store);
        if (ok) savedCount++;
        // Track active file's content so we can update in-memory sourceCode
        if (entry.filePath === filePath) {
          const genResult = generateOmniSource({
            designElements: entry.elements,
            designWires: entry.wires,
            codeTabs: codeTabs.tabs,
            componentName: (store.activeFileContent || store.sourceCode || "").match(/component\s+(\w+)\s*\{/)?.[1] || "App",
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
      await api.saveEditorToml({
        active_file: filePath,
        active_mode: "design",
        open_files: store.openTabs.length > 0
          ? store.openTabs
          : (filePath ? [filePath] : []),
        left_open: true,
        right_width: 320,
        viewports: {},
      });

      store.markClean();
      useEditorStore.setState({ projectName: filePath.replace(/^.*[\\/]/, "").replace(/\.omni$/, "") });
      const fullPath = projectDir ? `${projectDir}/${filePath}` : filePath;
      setStatus({ text: `✅ 已保存 ${savedCount} 个文件: ${fullPath}`, isError: false });
    } catch (e) {
      setStatus({ text: `❌ 保存失败: ${String(e)}`, isError: true });
    }
  }, [projectDir, saveFileEntry]);

  // ── Open .omni file dialog ────────────────────

  const [openDialog, setOpenDialog] = useState<{ visible: boolean; files: string[] }>({ visible: false, files: [] });

  const handleOpenProject = useCallback(async () => {
    setStatus({ text: "扫描 .omni 文件...", isError: false });
    try {
      const omniFiles: string[] = [];
      const scanned = new Set<string>();

      const scanDir = async (dirPath: string, depth: number): Promise<void> => {
        if (depth > 3 || scanned.has(dirPath)) return;
        scanned.add(dirPath);
        try {
          const listing = await api.listDir(dirPath);
          const entries = listing?.entries || [];
          for (const e of entries) {
            if (e.name.endsWith(".omni")) {
              const fullPath = dirPath === "." ? e.name : `${dirPath}/${e.name}`;
              omniFiles.push(fullPath);
            }
          }
          if (depth < 3) {
            for (const d of entries.filter((x: { type: string }) => x.type === "directory")) {
              const subPath = dirPath === "." ? d.name : `${dirPath}/${d.name}`;
              await scanDir(subPath, depth + 1);
            }
          }
        } catch { /* skip */ }
      };

      await scanDir(".", 1);

      if (omniFiles.length === 0) {
        setStatus({ text: "未找到 .omni 文件", isError: true });
        return;
      }
      setOpenDialog({ visible: true, files: omniFiles });
    } catch { setStatus({ text: "扫描失败", isError: true }); }
  }, []);

  const openProjectFile = useCallback(async (filePath: string) => {
    setOpenDialog({ visible: false, files: [] });
    setStatus({ text: `加载 ${filePath}...`, isError: false });
    try {
      // Use annotatedLoad: reads .omni source + extract annotations from comments
      const result = await api.annotatedLoad(filePath);
      if (!result.success || !result.source) {
        setStatus({ text: `❌ 加载失败: ${result.error}`, isError: true });
        return;
      }

      const store = useEditorStore.getState();
      store.newProject();
      useEditorStore.setState({
        activeFileContent: result.source,
        activeFilePath: filePath,
        projectPath: projectDir || filePath,
        projectName: filePath.replace(/^.*[\\/]/, "").replace(/\.omni$/, ""),
      });

      // Parse .omni source into design elements
      const parseResult = parseOmniSource(result.source);
      if (parseResult.designElements) {
        const rootOrder = parseResult.designElements
          .filter((e: ElementData) => !e.parentId)
          .map((e: ElementData) => e.id);
        useEditorStore.setState({
          elements: parseResult.designElements,
          wires: parseResult.designWires || [],
          rootOrder,
        });
      }

      // Restore code graph from annotation chains
      if (result.annotations?.chains?.length) {
        const chains = result.annotations.chains;
        const codeElements: any[] = [];
        const codeWires: any[] = [];
        for (const chain of chains) {
          for (const node of chain.nodes || []) {
            codeElements.push({
              id: node.id, tag: node.tag, props: node.attrs || {},
              position: { x: node.posX, y: node.posY },
              nodeType: "codeblock", children: [], parentId: null,
            });
          }
          for (const wire of chain.wires || []) {
            codeWires.push({
              id: wire.id, fromId: wire.fromId, fromPort: wire.fromPort,
              toId: wire.toId, toPort: wire.toPort,
            });
          }
        }
        if (codeElements.length > 0) {
          useCodeGraphStore.getState().setElementsAndWires(codeElements, codeWires);
        }
      }

      // Load custom component definitions from src/components/*.omni
      loadComponentDefinitionsFromServer().then((merged) => {
        if (Object.keys(merged).length > 0) {
          import("./store/componentRegistry").then((m) => {
            const existing = m.useComponentRegistry.getState().definitions;
            m.useComponentRegistry.setState({
              definitions: { ...existing, ...merged },
            });
          }).catch(() => {});
        }
      }).catch(() => {});

      setStatus({ text: `✅ 已打开: ${filePath}`, isError: false });
    } catch (e) {
      setStatus({ text: `❌ 加载失败: ${String(e)}`, isError: true });
    }
  }, [projectDir]); // end openProjectFile

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

  // ── Cleanup: stop Vite on page close ──
  // NOTE: beforeunload 确认对话框已移除（测试时影响刷新流程），需要时可加回

  useEffect(() => {
    return () => {
      api.stopRun().catch(() => {});
      api.disconnectDebug();
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
          // ── Validate before parse (spec: validate first, if not valid → report error, do not enter Parser) ──
          const validation = validate(currentContent);
          if (!validation.valid) {
            setCompileErrors(validation.errors);
            setStatus({ text: `❌ 验证失败: ${validation.errors.length} 个错误`, isError: true });
            return;
          }
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

  if (showWelcome) {
    return <ProjectWelcome onProjectReady={handleProjectReady} />;
  }

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
          <PanelContainer visible={activePanel === "components" || activePanel === "tree" || activePanel === "canvas"}>
            {activePanel === "components" && <ComponentPalette />}
            {activePanel === "tree" && <ComponentTree ctx={ctx} />}
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
            <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 12 }}>📂 打开 .omni 文件</div>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 12 }}>选择一个 .omni 源文件打开</div>
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

