import { lazy, Suspense, useEffect, useRef } from "react";
import { usePoiesisStore } from "./store";
import { WelcomePage } from "./pages/WelcomePage";
import { StatusBar } from "./components/shell/StatusBar";
import { api } from "./commands";
import { rustCompiler } from "./commands/rustCompiler";
import { DEFAULT_PROJECT_NAME } from "./constants";
import { parseOmniSource } from "./services/codegen";
import { parseStyleValue } from "./services/codegen/design-value-parser";
import type { ElementData, WireData } from "./core/types";
import { detectTabType } from "./core/types/design-page";
import type { DesignDefinition, TokenType } from "./core/types/design";
import { tokenTypeToCategory } from "./core/types/design";
import { uid } from "./core/utils/id";
import { ErrorBoundary } from "./components/error/ErrorBoundary";
import { createLogger } from "./lib/utils/logger";
import "./styles/tokens.css";
import "./styles/legacy-bridge.css";

const DesignPage = lazy(() => import("./pages/DesignPage").then((m) => ({ default: m.DesignPage })));
const ArchitecturePage = lazy(() => import("./pages/ArchitecturePage").then((m) => ({ default: m.ArchitecturePage })));

const log = createLogger("App");

/** Load .omni files from disk: open tabs, populate canvas/main, store component defs. */
async function loadOmniFiles(): Promise<void> {
  const scanDirs = [".", "src", "src/components", "src/stores", "src/lib"];
  const omniPaths: string[] = [];

  for (const dir of scanDirs) {
    try {
      const result = await api.project.listDir(dir);
      if (!result.entries) continue;
      for (const entry of result.entries) {
        if (entry.ext === "omni") omniPaths.push(entry.path);
      }
    } catch (e) { log.warn("Failed to read project directory:", e); }
  }

  let mainPath: string | null = null;
  for (const filePath of omniPaths) {
    if (!mainPath && (filePath === "src/main.omni" || filePath === "main.omni")) {
      mainPath = filePath;
    }
  }
  // If no main.omni found, use the first .omni file
  if (!mainPath && omniPaths.length > 0) mainPath = omniPaths[0];

  // Parse all .omni files, collecting results before applying to store.
  interface ParsedFile {
    filePath: string;
    componentName: string;
    content: string;
    elements: ElementData[];
    wires: WireData[];
    rootOrder: string[];
    storeDef?: import("./core/types/store").StoreDefinition;
    inlineStoreDefs?: Record<string, import("./core/types/store").StoreDefinition>;
  }

  // ── Phase 1: Parse all files and load storeDefs BEFORE opening tabs ──
  // This prevents StorePage's useEffect from creating an empty default store
  // that overwrites the parsed store definition.
  const BATCH_SIZE = 5;
  const parsed: ParsedFile[] = [];
  for (let i = 0; i < omniPaths.length; i += BATCH_SIZE) {
    const batch = omniPaths.slice(i, i + BATCH_SIZE);
    const results = await Promise.all(batch.map(async (filePath) => {
      try {
        const res = await api.project.readFile(filePath);
        if (!res.content) return null;
        const parseResult = parseOmniSource(res.content);
        // Allow files with store definitions even if they have no view elements
        if (!parseResult.designElements && !parseResult.storeDef) return null;
        const componentName = filePath.split("/").pop()!.replace(/\.omni$/, "");
        const positionedElements = (parseResult.designElements ?? []).map((el, idx) => {
          const position = el.nodeType !== "group" ? el.position : undefined;
          return {
            ...el,
            position: position ?? { x: 100 + idx * 50, y: 100 + idx * 50 },
          };
        });
        const rootOrder = positionedElements
          .filter((e: ElementData) => !e.parentId)
          .map((e: ElementData) => e.id);
        return {
          filePath, componentName,
          content: res.content,
          elements: positionedElements,
          wires: parseResult.designWires || [],
          rootOrder,
          storeDef: parseResult.storeDef || undefined,
          inlineStoreDefs: parseResult.inlineStoreDefs || undefined,
        } as ParsedFile;
      } catch (e) {
        log.warn("Failed to parse .omni file:", filePath, e);
        return null;
      }
    }));
    for (const r of results) {
      if (r) parsed.push(r);
    }
  }

  // ── Load storeDefs now, before opening tabs ──
  for (const p of parsed) {
    if (p.storeDef) {
      usePoiesisStore.getState().loadStoreDef(p.storeDef);
      usePoiesisStore.getState().registerStorePids(p.storeDef);
    }
    if (p.inlineStoreDefs) {
      for (const def of Object.values(p.inlineStoreDefs)) {
        // Re-key: parser uses PascalCase from source (e.g. "Main.TestStore"),
        // but tabs use componentName from filename (e.g. "main" from main.omni)
        const dotIdx = def.name.indexOf(".");
        const storeName = dotIdx >= 0 ? def.name.slice(dotIdx + 1) : def.name;
        const rekeyed = { ...def, name: `${p.componentName}.${storeName}` };
        usePoiesisStore.getState().loadInlineStoreDef(rekeyed);
      }
    }
  }

  // ── Phase 2: Open tabs (storeDefs already loaded, so StorePage won't create empty defaults) ──
  for (const filePath of omniPaths) {
    const componentName = filePath.split("/").pop()!.replace(/\.omni$/, "");
    usePoiesisStore.getState().openTab(filePath, componentName, detectTabType(filePath));
  }

  // ── Load design.omni into the design store ──
  // Design data is separate from elements/wires — it provides tokens, classes,
  // and variants that are bound to elements via classRefs in the canvas renderer.
  {
    const designPath = omniPaths.find(
      (p) => p === "src/design.omni" || p === "design.omni",
    );
    if (designPath) {
      try {
        const designRes = await api.project.readFile(designPath);
        if (designRes.content) {
          const designResult = parseOmniSource(designRes.content);
          if (
            (designResult.designTokens?.length ?? 0) > 0 ||
            (designResult.designClasses?.length ?? 0) > 0 ||
            (designResult.designVariants?.length ?? 0) > 0
          ) {
            const design: DesignDefinition = {
              name: designResult.designName || "AppDesign",
              tokens: (designResult.designTokens ?? []).map((t) => ({
                id: t.id,
                name: t.name,
                category: tokenTypeToCategory(t.tokenType),
                tokenType: t.tokenType as TokenType,
                value: t.value,
              })),
              classes: (designResult.designClasses ?? []).map((c) => ({
                id: c.id,
                name: c.name,
                properties: (c.properties ?? []).map((p) => ({
                  id: uid("sp"),
                  name: p.name,
                  segments: parseStyleValue(p.value),
                })),
                pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
                  id: uid("ps"),
                  pseudo: ps.pseudo,
                  properties: ps.properties.map((p) => ({
                    id: uid("sp"),
                    name: p.name,
                    segments: parseStyleValue(p.value),
                  })),
                })),
              })),
              variants: (designResult.designVariants ?? []).map((v) => ({
                id: v.id,
                name: v.name,
                dimensions: (v.dimensions ?? []).map((d) => ({
                  name: d.name,
                  values: d.values.map((dv) => dv.name),
                })),
                defaults: v.defaults ?? {},
                baseProperties: (v.baseProperties ?? []).map((p) => ({
                  id: uid("sp"),
                  name: p.name,
                  segments: parseStyleValue(p.value),
                })),
                branches: (v.branches ?? []).map((b) => {
                  const dot = b.name.indexOf(".");
                  const dim = dot >= 0 ? b.name.slice(0, dot) : "";
                  const val = dot >= 0 ? b.name.slice(dot + 1) : b.name;
                  const props = b.properties.map((p) => ({
                    id: uid("sp"),
                    name: p.name,
                    segments: parseStyleValue(p.value),
                  }));
                  return { id: uid("vb"), dimension: dim, value: val, properties: props };
                }),
              })),
            };
            usePoiesisStore.getState().loadDesign(design);
          }
        }
      } catch (e) {
        log.warn("Failed to load design.omni:", e);
      }
    }
  }

  // Batch-apply parsed results to store.
  if (parsed.length > 0) {
    const store = usePoiesisStore.getState();
    const stateUpdate: Partial<{
      activeFileContent: string;
      activeFilePath: string | null;
      elements: import("./core/types/element").ElementData[];
      wires: import("./core/types/wire").WireData[];
      rootOrder: string[];
    }> = {};
    const newTabData = { ...store.tabData };
    const newComponentDefs: Record<string, import("./core/types/project-json").ComponentDef> = { ...store.componentDefs };

    // Find the main file's parsed data — canvas elements should come from main.omni,
    // NOT the last file in scan order (which could be a component definition).
    const mainParsed = parsed.find((p) => p.filePath === mainPath) || parsed[parsed.length - 1];

    for (const p of parsed) {
      const tab = store.designPage.openTabs.find((t) => t.filePath === p.filePath);
      if (tab) {
        newTabData[tab.id] = {
          elements: p.elements,
          wires: p.wires,
          rootOrder: p.rootOrder,
          selectedElementIds: [] as string[],
        };
      }

      if (p.filePath === mainPath) {
        stateUpdate.activeFileContent = p.content;
        stateUpdate.activeFilePath = p.filePath;
      } else {
        const existing = newComponentDefs[p.componentName];
        newComponentDefs[p.componentName] = {
          name: p.componentName,
          filePath: p.filePath,
          signals: existing?.signals ?? [],
          props: existing?.props ?? [],
          computeds: existing?.computeds ?? [],
          events: existing?.events ?? [],
          fns: existing?.fns ?? [],
          elements: p.elements,
          wires: p.wires,
          rootOrder: p.rootOrder,
        };
      }
    }

    stateUpdate.elements = mainParsed.elements;
    stateUpdate.wires = mainParsed.wires;
    stateUpdate.rootOrder = mainParsed.rootOrder;

    // Find the main.omni tab ID — the active tab must match the main canvas
    // to avoid tabData corruption when switching tabs later.
    const mainTabId = store.designPage.openTabs.find(
      (t) => t.filePath === mainPath,
    )?.id;

    usePoiesisStore.setState({
      ...stateUpdate,
      tabData: newTabData,
      componentDefs: newComponentDefs,
      designPage: {
        ...store.designPage,
        // Ensure the active tab matches the loaded main.omni canvas.
        // Do NOT call switchTab separately — it saves current state to the
        // old tab's slot, corrupting freshly-loaded tabData.
        ...(mainTabId ? { activeTabId: mainTabId } : {}),
      },
    });

  }
}

export default function App() {
  const currentPage = usePoiesisStore((s) => s.currentPage);
  const navigateTo = usePoiesisStore((s) => s.navigateTo);
  const isDirty = usePoiesisStore((s) => s.isDirty);
  const projectName = usePoiesisStore((s) => s.projectName);
  const projectDir = usePoiesisStore((s) => s.projectDir);
  const setProjectDir = usePoiesisStore((s) => s.setProjectDir);
  const setProjectName = usePoiesisStore((s) => s.setProjectName);
  const resetEditorState = usePoiesisStore((s) => s.resetEditorState);
  const compileErrors = usePoiesisStore((s) => s.compileErrors);

  const statusText =
    compileErrors.length > 0
      ? `${compileErrors.length} 个错误`
      : "就绪";

  // ── Global unhandled promise rejection handler ──
  useEffect(() => {
    const handler = (event: PromiseRejectionEvent) => {
      log.error("Unhandled promise rejection:", event.reason);
    };
    window.addEventListener("unhandledrejection", handler);
    return () => window.removeEventListener("unhandledrejection", handler);
  }, []);

  // ── WASM compiler early init ──
  useEffect(() => {
    rustCompiler.init();
  }, []);

  // ── Theme initialization and system preference listener ──
  const setTheme = usePoiesisStore((s) => s.setTheme);
  useEffect(() => {
    // Sync store with the data-theme already set by the anti-FOUC script
    const stored = localStorage.getItem("poiesis-theme") as "light" | "dark" | "system" | null;
    const theme = stored || "system";
    const resolved = theme === "system"
      ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
      : theme;
    document.documentElement.setAttribute("data-theme", resolved);
    setTheme(theme);

    // Listen for system theme changes
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      const current = usePoiesisStore.getState().theme;
      if (current === "system") {
        const next = e.matches ? "dark" : "light";
        document.documentElement.setAttribute("data-theme", next);
        usePoiesisStore.setState({ resolvedTheme: next } as any);
      }
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [setTheme]);

  // ── On mount: if projectDir is set, load .omni files into canvas ──
  const loadedRef = useRef(false);
  useEffect(() => {
    if (!projectDir || loadedRef.current) return;
    loadedRef.current = true;
    loadOmniFiles().catch((e) => log.warn("Failed to load .omni:", e));
  }, [projectDir]);

  // ── Open project: set dir and load .omni files ──
  const handleProjectOpened = async (path: string, name: string) => {
    resetEditorState();
    setProjectName(name);
    loadedRef.current = true; // prevent useEffect from also calling loadOmniFiles
    setProjectDir(path);
    // Always reload .omni files, even if same project is re-selected
    await loadOmniFiles();
  };

  return (
    <ErrorBoundary>
      {!projectDir ? (
        <div style={{ height: "100vh", overflow: "hidden" }}>
          <WelcomePage onProjectOpened={handleProjectOpened} />
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            height: "100vh",
            overflow: "hidden",
            position: "relative",
          }}
        >
          {/* Page content */}
          <div style={{ flex: 1, overflow: "hidden" }}>
            <Suspense fallback={<div style={{ padding: 40, color: "#6b7280", fontSize: 14 }}>Loading...</div>}>
              {currentPage === "architecture" ? <ArchitecturePage /> : <DesignPage onProjectOpened={handleProjectOpened} />}
            </Suspense>
          </div>

          {/* Page-switch toggle — top-right floating */}
          <div
            style={{
              position: "absolute",
              top: 8,
              right: 12,
              zIndex: 100,
              display: "flex",
              gap: 0,
              background: "rgba(255,255,255,0.92)",
              backdropFilter: "blur(8px)",
              borderRadius: 8,
              padding: 3,
              boxShadow: "0 1px 4px rgba(0,0,0,0.12), 0 0 0 1px rgba(0,0,0,0.06)",
            }}
          >
            <PageToggleButton
              active={currentPage === "design"}
              onClick={() => navigateTo("design")}
            >
              📐 设计
            </PageToggleButton>
            <PageToggleButton
              active={currentPage === "architecture"}
              onClick={() => navigateTo("architecture")}
            >
              🔗 架构
            </PageToggleButton>
          </div>

          <StatusBar
            status={{ text: statusText, isError: compileErrors.length > 0 }}
            isDirty={isDirty}
            projectName={projectName || DEFAULT_PROJECT_NAME}
            projectDir={projectDir}
            compileErrors={compileErrors}
          />
        </div>
      )}
    </ErrorBoundary>
  );
}

function PageToggleButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "5px 14px",
        fontSize: 12,
        fontWeight: active ? 600 : 400,
        color: active ? "#6d28d9" : "#6b7280",
        background: active ? "#f5f3ff" : "transparent",
        border: "none",
        borderRadius: 6,
        cursor: "pointer",
        transition: "all 0.15s",
        whiteSpace: "nowrap",
      }}
    >
      {children}
    </button>
  );
}
