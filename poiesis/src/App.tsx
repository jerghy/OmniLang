import { usePoiesisStore } from "./store";
import { ArchitecturePage } from "./pages/ArchitecturePage";
import { DesignPage } from "./pages/DesignPage";
import { WelcomePage } from "./pages/WelcomePage";
import { StatusBar } from "./components/shell/StatusBar";
import { api } from "./commands";
import { DEFAULT_PROJECT_NAME } from "./constants";
import { parseOmniSource } from "./services/codegen";
import type { ElementData, WireData } from "./core/types";
import { detectTabType } from "./core/types/design-page";
import { useEffect, useRef } from "react";

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
    } catch { /* dir may not exist */ }
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
  const parsed: ParsedFile[] = [];
  for (const filePath of omniPaths) {
    try {
      const res = await api.project.readFile(filePath);
      if (!res.content) continue;
      const parseResult = parseOmniSource(res.content);
      if (!parseResult.designElements) continue;
      const componentName = filePath.split("/").pop()!.replace(/\.omni$/, "");
      const rootOrder = parseResult.designElements
        .filter((e: ElementData) => !e.parentId)
        .map((e: ElementData) => e.id);
      const positionedElements = parseResult.designElements.map((el, idx) => ({
        ...el,
        position: el.position ?? { x: 100 + idx * 50, y: 100 + idx * 50 },
      }));
      parsed.push({
        filePath, componentName,
        content: res.content,
        elements: positionedElements,
        wires: parseResult.designWires || [],
        rootOrder,
      });
    } catch (e) {
      console.warn("[App] Failed to parse .omni file:", filePath, e);
    }
  }

  // Batch-apply parsed results to store.
  // Strategy: populate tabData for ALL tabs, set canvas to last-parsed file's elements,
  // then switchTab to main — which correctly saves current elements to the last tab
  // and restores main's elements from tabData.
  if (parsed.length > 0) {
    const store = usePoiesisStore.getState();
    const stateUpdate: Record<string, unknown> = {};
    const newTabData = { ...store.tabData };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const newComponentDefs: Record<string, any> = { ...store.componentDefs };

    // Last parsed file = current active tab (last opened via openTab)
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

    // Canvas shows last tab's elements (current active tab before switchTab)
    stateUpdate.elements = lastParsed.elements;
    stateUpdate.wires = lastParsed.wires;
    stateUpdate.rootOrder = lastParsed.rootOrder;

    usePoiesisStore.setState({
      ...stateUpdate,
      tabData: newTabData,
      componentDefs: newComponentDefs,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);

    // Switch to main tab (unless it's already active)
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
  const openTab = usePoiesisStore((s) => s.openTab);
  const compileErrors = usePoiesisStore((s) => s.compileErrors);

  const statusText =
    compileErrors.length > 0
      ? `${compileErrors.length} 个错误`
      : "就绪";

  // ── On mount: if projectDir is set, load .omni files into canvas ──
  const loadedRef = useRef(false);
  useEffect(() => {
    if (!projectDir || loadedRef.current) return;
    loadedRef.current = true;
    loadOmniFiles().catch((e) => console.warn("[App] Failed to load .omni:", e));
  }, [projectDir]);

  // ── Open project: set dir (useEffect handles loading) ──
  const handleProjectOpened = async (path: string, name: string) => {
    resetEditorState();
    setProjectName(name);
    loadedRef.current = false; // allow re-loading
    setProjectDir(path);
  };

  // ── No project open → show welcome page ──
  if (!projectDir) {
    return (
      <div style={{ height: "100vh", overflow: "hidden" }}>
        <WelcomePage onProjectOpened={handleProjectOpened} />
      </div>
    );
  }

  // ── Project is open → show editor ──
  return (
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
        {currentPage === "architecture" ? <ArchitecturePage /> : <DesignPage />}
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

      {/* Status bar at bottom */}
      <StatusBar
        status={{ text: statusText, isError: compileErrors.length > 0 }}
        isDirty={isDirty}
        projectName={projectName || DEFAULT_PROJECT_NAME}
        projectDir={projectDir}
        compileErrors={compileErrors}
      />
    </div>
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
