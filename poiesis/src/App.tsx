import { lazy, Suspense, useEffect, useRef } from "react";
import { usePoiesisStore } from "./store";
import { WelcomePage } from "./pages/WelcomePage";
import { StatusBar } from "./components/shell/StatusBar";
import { api } from "./commands";
import { DEFAULT_PROJECT_NAME } from "./constants";
import { parseOmniSource } from "./services/codegen";
import type { ElementData, WireData } from "./core/types";
import { detectTabType } from "./core/types/design-page";
import { ErrorBoundary } from "./components/error/ErrorBoundary";
import { createLogger } from "./lib/utils/logger";
import "./styles/tokens.css";

const DesignPage = lazy(() => import("./pages/DesignPage").then((m) => ({ default: m.DesignPage })));
const ArchitecturePage = lazy(() => import("./pages/ArchitecturePage").then((m) => ({ default: m.ArchitecturePage })));

const log = createLogger("App");

/** Load .omni files from disk: open tabs, populate canvas/main, store component defs. */
async function loadOmniFiles(): Promise<void> {
  const scanDirs = [".", "src", "src/components"];
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
    const componentName = filePath.split("/").pop()!.replace(/\.omni$/, "");
    usePoiesisStore.getState().openTab(filePath, componentName, detectTabType(filePath));

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
  }

  // Batch-load files in parallel (BATCH_SIZE=5) to avoid too many concurrent I/O ops
  const BATCH_SIZE = 5;
  const parsed: ParsedFile[] = [];
  for (let i = 0; i < omniPaths.length; i += BATCH_SIZE) {
    const batch = omniPaths.slice(i, i + BATCH_SIZE);
    const results = await Promise.all(batch.map(async (filePath) => {
      try {
        const res = await api.project.readFile(filePath);
        if (!res.content) return null;
        const parseResult = parseOmniSource(res.content);
        if (!parseResult.designElements) return null;
        const componentName = filePath.split("/").pop()!.replace(/\.omni$/, "");
        const rootOrder = parseResult.designElements
          .filter((e: ElementData) => !e.parentId)
          .map((e: ElementData) => e.id);
        const positionedElements = parseResult.designElements.map((el, idx) => ({
          ...el,
          position: el.position ?? { x: 100 + idx * 50, y: 100 + idx * 50 },
        }));
        return {
          filePath, componentName,
          content: res.content,
          elements: positionedElements,
          wires: parseResult.designWires || [],
          rootOrder,
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

    const lastParsed = parsed[parsed.length - 1];

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
        newComponentDefs[p.componentName] = {
          ...newComponentDefs[p.componentName],
          elements: p.elements,
          wires: p.wires,
          rootOrder: p.rootOrder,
        };
      }
    }

    stateUpdate.elements = lastParsed.elements;
    stateUpdate.wires = lastParsed.wires;
    stateUpdate.rootOrder = lastParsed.rootOrder;

    usePoiesisStore.setState({
      ...stateUpdate,
      tabData: newTabData,
      componentDefs: newComponentDefs,
    });

    if (mainPath && mainPath !== lastParsed.filePath) {
      const mainTabs = usePoiesisStore.getState().designPage.openTabs
        .filter((t) => t.filePath === mainPath);
      if (mainTabs.length > 0) {
        usePoiesisStore.getState().switchTab(mainTabs[0].id);
      }
    }
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

  // ── On mount: if projectDir is set, load .omni files into canvas ──
  const loadedRef = useRef(false);
  useEffect(() => {
    if (!projectDir || loadedRef.current) return;
    loadedRef.current = true;
    loadOmniFiles().catch((e) => log.warn("Failed to load .omni:", e));
  }, [projectDir]);

  // ── Open project: set dir (useEffect handles loading) ──
  const handleProjectOpened = async (path: string, name: string) => {
    resetEditorState();
    setProjectName(name);
    loadedRef.current = false;
    setProjectDir(path);
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
              {currentPage === "architecture" ? <ArchitecturePage /> : <DesignPage />}
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
