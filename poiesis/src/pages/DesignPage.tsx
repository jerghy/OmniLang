// pages/DesignPage.tsx
// Top-level design page with tabbed editor, left panels, and Z-axis canvas.
// Drag-and-drop from palette to canvas uses @dnd-kit/core (pointer events, not native HTML5 DnD).

import { Component, useState, useEffect, useRef, useLayoutEffect, useCallback, useMemo, type ReactNode } from "react";
import { Logger } from "../lib/utils/logger";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  closestCenter,
} from "@dnd-kit/core";
import { DesignTabBar } from "../components/design/DesignTabBar";
import { DebugWindowButton } from "../features/debug/DebugWindowButton";
import { api } from "../commands";
import type { RecentProject } from "../commands/project-api";
import type { StoreDefinition } from "../core/types/store";
import { DesignTabContent } from "../components/design/DesignTabContent";
import { InfiniteCanvas } from "../components/canvas/InfiniteCanvas";
import { OmniViewRenderer } from "../components/design/OmniViewRenderer";
import { CodeFlowCanvas } from "../components/design/CodeFlowCanvas";
import { StorePage } from "./StorePage";
import { CodeDrawer } from "../components/design/CodeDrawer";
import { StoreDrawer } from "../components/design/StoreDrawer";
import { BreadcrumbBar } from "../components/design/BreadcrumbBar";
import { generateOmniFiles, generateOmniFragment } from "../services/codegen/pipeline";
import { emitDesignSource } from "../services/codegen/design-emitter";
import { emitStoreDefinition } from "../services/codegen/source-emitter";
import { ComponentPalette } from "../components/panels/ComponentPalette";
import { ComponentTree } from "../components/panels/ComponentTree";
import { ContextInspector } from "../components/panels/ContextInspector";
import { ComponentMembersPanel } from "../components/design/ComponentMembersPanel";
import { DesignToolbar } from "../components/design/DesignToolbar";
import { usePoiesisStore } from "../store";
import { useSaveProject } from "../hooks/useSaveProject";
import { useDragHandlers } from "../hooks/useDragHandlers";
import styles from "./DesignPage.module.css";

// Lightweight error boundary to prevent one panel crash from blanking the entire page
class PanelErrorBoundary extends Component<{ children: ReactNode; name: string }, { error: Error | null }> {
  constructor(props: { children: ReactNode; name: string }) {
    super(props);
    this.state = { error: null };
  }
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 16, color: "var(--ol-color-danger-hover)", fontSize: 12, background: "var(--ol-color-danger-subtle)", border: "1px solid var(--ol-color-danger-muted)", borderRadius: 4, margin: 4 }}>
          <strong>⚠ {this.props.name}</strong>
          <div style={{ marginTop: 4, opacity: 0.7 }}>{this.state.error.message}</div>
        </div>
      );
    }
    return this.props.children;
  }
}

function buildCodeSource(showCode: boolean): string {
  if (!showCode) return "";
  const state = usePoiesisStore.getState();
  const activeTab = state.designPage.openTabs.find((t) => t.id === state.designPage.activeTabId);
  if (!activeTab) return "";
  if (activeTab.tabType === "design") {
    return state.activeDesign ? emitDesignSource(state.activeDesign) : "";
  }
  if (activeTab.tabType === "store") {
    const storeDef = state.storeDefs[activeTab.componentName];
    return storeDef ? emitStoreDefinition(storeDef) : "// No store definition found";
  }
  const componentName = activeTab.componentName || "main";
  const prefix = `${componentName}.`;
  const inlineStores: Array<unknown> = [];
  for (const [key, def] of Object.entries(state.inlineStoreDefs)) {
    if (key.startsWith(prefix)) inlineStores.push(def);
  }
  const codeTabs = [{ id: "code", type: "logic" as const, label: "Preview", icon: "📄",
    graph: { elements: state.elements, wires: state.wires }, variables: [] }];
  const result = componentName === "main"
    ? generateOmniFiles({
        designElements: state.elements, designWires: state.wires, codeTabs,
        componentName: "main", rootOrder: state.rootOrder,
        filePath: activeTab.filePath || "src/main.omni",
        designName: state.activeDesign?.name, inlineStores: inlineStores as StoreDefinition[],
      })
    : generateOmniFragment({
        componentName, designElements: state.elements, designWires: state.wires,
        codeTabs, rootOrder: state.rootOrder, fromDefinition: true, inlineStores: inlineStores as StoreDefinition[],
      });
  const fileKey = Object.keys(result.value || {}).find((k) => k.includes(componentName));
  const files = result.value as Record<string, string> | undefined;
  return (fileKey && files?.[fileKey]) || "// Failed to generate source";
}

function useProjectPanel(onProjectOpened?: (path: string, name: string) => void) {
  const [showProjectPanel, setShowProjectPanel] = useState(false);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);

  useEffect(() => {
    if (showProjectPanel) {
      api.project.getRecentProjects().then(setRecentProjects).catch((e) => { new Logger({ prefix: "DesignPage" }).warn("Failed to load recent projects", e); });
    }
  }, [showProjectPanel]);

  const handleOpenProject = useCallback(async () => {
    try {
      const folder = await api.project.openFolderDialog();
      if (!folder) return;
      const name = folder.replace(/\\/g, "/").split("/").pop() || folder;
      await api.project.setProjectDir(folder);
      await api.project.addRecentProject(name, folder);
      setShowProjectPanel(false);
      onProjectOpened?.(folder, name);
    } catch (e) {
      new Logger({ prefix: "DesignPage" }).error("Failed to create project", e);
    }
  }, [onProjectOpened]);

  const handleOpenRecent = useCallback(async (proj: RecentProject) => {
    try {
      await api.project.setProjectDir(proj.path);
      await api.project.addRecentProject(proj.name, proj.path);
      setShowProjectPanel(false);
      onProjectOpened?.(proj.path, proj.name);
    } catch (e) {
      new Logger({ prefix: "DesignPage" }).error("Failed to open recent project", e);
    }
  }, [onProjectOpened]);

  const handleRemoveRecent = useCallback(async (e: React.MouseEvent, path: string) => {
    e.stopPropagation();
    await api.project.removeRecentProject(path);
    const list = await api.project.getRecentProjects();
    setRecentProjects(list);
  }, []);

  return { showProjectPanel, setShowProjectPanel, recentProjects, handleOpenProject, handleOpenRecent, handleRemoveRecent };
}

export function DesignPage({ onProjectOpened }: { onProjectOpened?: (path: string, name: string) => void }) {
  const undo = usePoiesisStore((s) => s.undo);
  const isDirty = usePoiesisStore((s) => s.isDirty);
  const activeTabType = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return "component" as const;
    return s.designPage.openTabs.find((t) => t.id === activeId)?.tabType ?? "component";
  });
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  const codeFlowFocusPoint = usePoiesisStore((s) => s.designPage.codeFlowFocusPoint);
  const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const activeTab = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return null;
    return s.designPage.openTabs.find((t) => t.id === activeId) ?? null;
  });
  // Side panels are visible only for component tabs NOT in code-flow mode AND not in store drawer mode
  const showCodeFlowPanel = codeFlowVisible && activeTabType === "component";
  const showDesignPanels = activeTabType === "component" && !codeFlowVisible && !storePanelVisible;

  // Ref for z-axis container to convert client coords to percentages
  const zAxisRef = useRef<HTMLDivElement>(null);

  const { saving, saveError, handleSave } = useSaveProject();
  const { handleDragStart, handleDragMove, handleDragEnd, activeComponent, activeTreeId, treeDropState } = useDragHandlers();

  // ── Code drawer ──
  // Subscribe to all state that should trigger a code refresh:
  //   elements / wires / rootOrder  → canvas edits
  //   activeDesign / storeDefs      → design-system / store edits
  //   activeTabId / openTabs        → tab switching
  //   componentDefs                 → custom component creation
  const [showCode, setShowCode] = useState(false);
  const codeElements = usePoiesisStore((s) => s.elements);
  const codeWires = usePoiesisStore((s) => s.wires);
  const codeRootOrder = usePoiesisStore((s) => s.rootOrder);
  const codeActiveDesign = usePoiesisStore((s) => s.activeDesign);
  const codeStoreDefs = usePoiesisStore((s) => s.storeDefs);
  const codeActiveTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const codeOpenTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const codeComponentDefs = usePoiesisStore((s) => s.componentDefs);

  const codeInlineStores = usePoiesisStore((s) => s.inlineStoreDefs);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const codeSource = useMemo(() => buildCodeSource(showCode), [showCode, codeElements, codeWires, codeRootOrder, codeActiveDesign, codeStoreDefs, codeActiveTabId, codeOpenTabs, codeComponentDefs, codeInlineStores]);

  const codeFileName = useMemo(() => {
    if (!showCode) return "";
    const state = usePoiesisStore.getState();
    const activeTab = state.designPage.openTabs.find((t) => t.id === state.designPage.activeTabId);
    return activeTab?.filePath || "source.omni";
  }, [showCode, codeActiveTabId, codeOpenTabs]);

  // ── Open Project panel ──
  const { showProjectPanel, setShowProjectPanel, recentProjects, handleOpenProject, handleOpenRecent, handleRemoveRecent } = useProjectPanel(onProjectOpened);

  // ── @dnd-kit sensors (unified context handles both palette and tree drags) ──
  const pointerSensor = useSensor(PointerSensor, {
    activationConstraint: { distance: 5 },
  });
  const keyboardSensor = useSensor(KeyboardSensor);
  const sensors = useSensors(pointerSensor, keyboardSensor);

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragStart={handleDragStart} onDragMove={handleDragMove} onDragEnd={handleDragEnd}>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: showDesignPanels
            ? "280px 1fr 300px"
            : showCodeFlowPanel
              ? "280px 1fr 240px"
              : "1fr",
          gridTemplateRows: "36px 32px 1fr 24px",
          height: "100vh",
          background: "var(--ol-gray-50)",
        }}
      >
        {/* Tab bar spans full width */}
        <div className={styles.tabBarArea}>
          <DesignTabBar />
        </div>

        {/* Toolbar row — undo / save / CSS styles */}
        <DesignToolbar
          isDirty={isDirty}
          saving={saving}
          onUndo={undo}
          onSave={handleSave}
          onOpenProject={() => setShowProjectPanel(true)}
          showCode={showCode}
          onToggleCode={() => setShowCode((v) => !v)}
          saveError={saveError}
        >
          <DebugWindowButton />
        </DesignToolbar>

        {/* Left panel: ComponentPalette + ComponentTree (component tabs, hidden in code-flow mode) */}
        {showDesignPanels && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 1,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
            background: "var(--ol-surface)",
            borderRight: "1px solid var(--ol-border)",
          }}
        >
          <div style={{ flex: "0 0 auto", maxHeight: "50%", overflow: "auto" }}>
            <PanelErrorBoundary name="组件面板">
              <ComponentPalette />
            </PanelErrorBoundary>
          </div>
          <div style={{ flex: 1, borderTop: "1px solid var(--ol-border)", overflow: "auto" }}>
            <PanelErrorBoundary name="组件树">
              <ComponentTree treeDropState={treeDropState} />
            </PanelErrorBoundary>
          </div>
        </div>
        )}

        {/* Left panel: ComponentMembersPanel (code-flow mode for component tabs) */}
        {showCodeFlowPanel && activeTab && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 1,
            overflow: "hidden",
            background: "var(--ol-surface)",
            borderRight: "1px solid var(--ol-border)",
          }}
        >
          <ComponentMembersPanel componentName={activeTab.componentName} />
        </div>
        )}

        {/* Center: DesignCanvas for components, CodeFlowCanvas for store/code */}
        <div
          style={{
            gridRow: 3,
            gridColumn: showDesignPanels ? 2 : showCodeFlowPanel ? 2 : "1 / -1",
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
            background: "var(--ol-gray-100)",
          }}
        >
          {activeTabType === "design" ? (
            <DesignTabContent />
          ) : activeTabType === "component" ? (
            /* Z-axis dual-layer: OmniView top layer shrinks to reveal CodeFlowCanvas below */
            <div ref={zAxisRef} style={{ flex: 1, position: "relative", overflow: "hidden" }}>
              {/* Top layer: OmniViewRenderer with shrink animation */}
              <TopLayer
                codeFlowVisible={codeFlowVisible}
                focusPoint={codeFlowFocusPoint}
                containerRef={zAxisRef}
              >
                <InfiniteCanvas>
                  <OmniViewRenderer />
                </InfiniteCanvas>
              </TopLayer>

              {/* Bottom layer: CodeFlowCanvas (revealed when codeFlowVisible) */}
              {codeFlowVisible && (
                <div
                  onDoubleClick={(e) => {
                    const target = e.target as HTMLElement;
                    if (target.closest(".react-flow__node, .react-flow__controls")) return;
                    toggleCodeFlow(null);
                  }}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    zIndex: 0,
                  }}
                >
                  <CodeFlowCanvas />
                </div>
              )}
            </div>
          ) : activeTabType === "store" && activeTab ? (
            <StorePage storeName={activeTab.componentName} tabId={activeTab.id} />
          ) : (
            <CodeFlowCanvas />
          )}
        </div>

        {/* Right panel: ContextInspector (component tabs, hidden in code-flow mode) */}
        {showDesignPanels && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 3,
            overflow: "hidden",
            display: "flex",
            background: "var(--ol-surface)",
            borderLeft: "1px solid var(--ol-border)",
          }}
        >
          <ContextInspector />
        </div>
        )}

        {/* Store Drawer — overlays center area when visible */}
        {storePanelVisible && activeTabType === "component" && (
          <div style={{
            gridRow: 3,
            gridColumn: showDesignPanels ? 2 : showCodeFlowPanel ? 2 : "1 / -1",
            overflow: "hidden",
            position: "relative",
            zIndex: 10,
          }}>
            <StoreDrawer />
          </div>
        )}

        {/* Code drawer — fixed at viewport bottom, overlays all content */}
        {showCode && (
          <div style={{
            position: "fixed", bottom: 0, left: 0, right: 0, zIndex: 100,
          }}>
            <CodeDrawer source={codeSource} fileName={codeFileName} onClose={() => setShowCode(false)} />
          </div>
        )}

        {/* Breadcrumb bar spans full width */}
        <div style={{ gridRow: 4, gridColumn: "1 / -1" }}>
          <BreadcrumbBar />
        </div>
      </div>

      {/* Drag overlay: shows what's being dragged (palette or tree) */}
      <DragOverlay dropAnimation={null}>
        {activeTreeId ? (
          <TreeDragPreview elementId={activeTreeId} />
        ) : activeComponent ? (
          <div className={styles.dragOverlay}>
            {activeComponent}
          </div>
        ) : null}
      </DragOverlay>

      {/* ── Open Project Panel (modal overlay) ── */}
      {showProjectPanel && (
        <ProjectPanelModal
          recentProjects={recentProjects}
          onOpenProject={handleOpenProject}
          onOpenRecent={handleOpenRecent}
          onRemoveRecent={handleRemoveRecent}
          onClose={() => setShowProjectPanel(false)}
        />
      )}
    </DndContext>
  );
}

/** Project panel modal — extracted to keep DesignPage under 300 lines. */
function ProjectPanelModal({
  recentProjects, onOpenProject, onOpenRecent, onRemoveRecent, onClose,
}: {
  recentProjects: RecentProject[];
  onOpenProject: () => void;
  onOpenRecent: (proj: RecentProject) => void;
  onRemoveRecent: (e: React.MouseEvent, path: string) => void;
  onClose: () => void;
}) {
  return (
    <div className={styles.modalOverlay} onClick={onClose}>
      <div onClick={(e) => e.stopPropagation()} className={styles.modalContent}>
        <div className={styles.modalTitle}>打开项目</div>
        <button type="button" onClick={onOpenProject} className={styles.openFolderBtn}>
          <span className={styles.openFolderIcon}>📂</span>
          <span>打开项目</span>
        </button>
        {recentProjects.length > 0 && (
          <div className={styles.recentSection}>
            <div className={styles.recentLabel}>最近打开</div>
            <div className={styles.recentList}>
              {recentProjects.map((proj) => (
                <button type="button"
                  key={proj.path}
                  onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'remove-recent') { onRemoveRecent(e as unknown as React.MouseEvent, proj.path); } else { onOpenRecent(proj); } }}
                  className={styles.recentItemBtn}
                >
                  <span className={styles.recentItemIcon}>📁</span>
                  <div className={styles.recentItemInfo}>
                    <div className={styles.recentItemName}>{proj.name}</div>
                    <div className={styles.recentItemPath}>{proj.path}</div>
                  </div>
                  <span data-action="remove-recent" className={styles.recentItemRemove} title="移除">×</span>
                </button>
              ))}
            </div>
          </div>
        )}
        <button type="button" onClick={onClose} className={styles.closePanelBtn}>取消</button>
      </div>
    </div>
  );
}

/** Drag overlay preview for tree-node drags. */
function TreeDragPreview({ elementId }: { elementId: string }) {
  const el = usePoiesisStore((s) => s.elements.find((e) => e.id === elementId));
  if (!el) return null;
  return (
    <div className={styles.treeDragPreview}>
      {"tag" in el ? el.tag : ""}
      {"props" in el && el.props?.label ? ` "${String(el.props.label).slice(0, 20)}"` : ""}
    </div>
  );
}

/** Z-axis top layer with shrink animation centered on the double-click focus point. */
function TopLayer({
  codeFlowVisible,
  focusPoint,
  containerRef,
  children,
}: {
  codeFlowVisible: boolean;
  focusPoint: { x: number; y: number } | null;
  containerRef: React.RefObject<HTMLDivElement | null>;
  children: React.ReactNode;
}) {
  // Measure the container AFTER layout (side panels hidden) to get correct pixel coords.
  const [origin, setOrigin] = useState<string>("50% 50%");

  useLayoutEffect(() => {
    if (!codeFlowVisible || !focusPoint) return;
    const container = containerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    // Convert viewport-relative client coords to element-relative pixel coords
    const ox = focusPoint.x - rect.left;
    const oy = focusPoint.y - rect.top;
    setOrigin(`${ox}px ${oy}px`);
  }, [codeFlowVisible, focusPoint, containerRef]);

  return (
    <div
      className={styles.topLayer}
      style={{
        display: codeFlowVisible ? "none" : "flex",
        transform: codeFlowVisible ? "scale(0.4)" : "scale(1)",
        transformOrigin: origin,
        borderRadius: codeFlowVisible ? 8 : 0,
        boxShadow: codeFlowVisible ? "0 4px 16px rgba(0,0,0,0.15)" : "none",
        pointerEvents: codeFlowVisible ? "none" : "auto",
      }}
    >
      {children}
    </div>
  );
}
