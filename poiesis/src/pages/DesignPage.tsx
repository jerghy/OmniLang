// pages/DesignPage.tsx
// Top-level design page with tabbed editor, left panels, and Z-axis canvas.
// Drag-and-drop from palette to canvas uses @dnd-kit/core (pointer events, not native HTML5 DnD).

import { useState, useEffect, useRef, useCallback, useMemo } from "react";
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
import { LogicLayer } from "../components/canvas/LogicLayer";
import { SourceNodeLayer } from "../components/canvas/SourceNodeLayer";
import { BreathingPoint } from "../components/canvas/BreathingPoint";
import { OmniViewRenderer } from "../components/design/OmniViewRenderer";
import { CodeFlowCanvas } from "../components/design/CodeFlowCanvas";
import { StorePage } from "./StorePage";
import { CodeDrawer } from "../components/design/CodeDrawer";
import { StoreDrawer } from "../components/design/StoreDrawer";
import { BottomBar } from "../components/design/BottomBar";
import { PaletteDrawer } from "../components/design/PaletteDrawer";
import { PanelErrorBoundary } from "../components/design/PanelErrorBoundary";
import { generateOmniFiles, generateOmniFragment } from "../services/codegen/pipeline";
import { emitDesignSource } from "../services/codegen/design-emitter";
import { emitStoreDefinition } from "../services/codegen/source-emitter";
import { ComponentTree } from "../components/panels/ComponentTree";
import { ContextPanel } from "../components/design/ContextPanel";
import { LibMembersPanel } from "../components/code/LibMembersPanel";
import { DesignToolbar } from "../components/design/DesignToolbar";
import { StorePanelToggleButton } from "../components/design/StorePanelToggleButton";
import { usePoiesisStore } from "../store";
import { useSaveProject } from "../hooks/useSaveProject";
import { useDragHandlers } from "../hooks/useDragHandlers";
import styles from "./DesignPage.module.css";

// 面板错误边界已提取至 components/design/PanelErrorBoundary.tsx（共享，M2 T6 复审）

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
  const redo = usePoiesisStore((s) => s.redo);
  const isDirty = usePoiesisStore((s) => s.isDirty);
  const activeTabType = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return "component" as const;
    return s.designPage.openTabs.find((t) => t.id === activeId)?.tabType ?? "component";
  });
  const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const activeTab = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return null;
    return s.designPage.openTabs.find((t) => t.id === activeId) ?? null;
  });
  // Side panels are visible only for component tabs NOT in store drawer mode
  const showDesignPanels = activeTabType === "component" && !storePanelVisible;
  // 逻辑层叠加可见：组件 tab 醒态 或 Shift 多选 ≥2（画布叠加，旧 codeFlowVisible 双态模式已删）
  const logicLayerVisible = usePoiesisStore((s) => s.designPage.awake || s.selectedElementIds.length >= 2);

  // ── F1 物化：组件 tab 进入逻辑层时合并当前组件的 fn 图 ──
  // 物化是幂等的（pid 去重、已存在不覆盖），可安全地在每次逻辑层可见时触发。
  const materializeFnGraphs = usePoiesisStore((s) => s.materializeFnGraphs);
  const activeTabComponentName = activeTab?.componentName;
  useEffect(() => {
    if (logicLayerVisible && activeTabType === "component" && activeTabComponentName) {
      materializeFnGraphs(activeTabComponentName);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 物化幂等，依赖仅触发时机
  }, [logicLayerVisible, activeTabType, activeTabComponentName]);

  // ── 物化：code tab 进入时合并当前 lib 文件的 fn 图（幂等，对标组件 F1 物化）──
  const materializeLibGraphs = usePoiesisStore((s) => s.materializeLibGraphs);
  useEffect(() => {
    if (activeTabType === "code" && activeTab?.filePath) {
      materializeLibGraphs(activeTab.filePath);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 物化幂等，依赖仅触发时机
  }, [activeTabType, activeTab?.filePath]);

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
  const [paletteOpen, setPaletteOpen] = useState(false);
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
          gridTemplateColumns: showDesignPanels || activeTabType === "code" ? "280px 1fr" : "1fr",
          gridTemplateRows: "36px 32px 1fr 28px",
          height: "100vh",
          background: "var(--ol-gray-50)",
        }}
      >
        {/* Tab bar spans full width */}
        <div className={styles.tabBarArea}>
          <DesignTabBar />
        </div>

        {/* Toolbar row — undo / redo / save / CSS styles */}
        <DesignToolbar
          style={{ gridColumn: "1 / -1" }}
          isDirty={isDirty}
          saving={saving}
          onUndo={undo}
          onRedo={redo}
          onSave={handleSave}
          onOpenProject={() => setShowProjectPanel(true)}
          showCode={showCode}
          onToggleCode={() => setShowCode((v) => !v)}
          saveError={saveError}
        >
          <DebugWindowButton />
          <StorePanelToggleButton />
        </DesignToolbar>

        {/* Left panel: ComponentTree only（组件 tab，code 模式隐藏；[组件库] 已移至底栏弹出面板 PaletteDrawer） */}
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
          <div style={{ flex: 1, overflow: "auto" }}>
            <PanelErrorBoundary name="组件树">
              <ComponentTree treeDropState={treeDropState} />
            </PanelErrorBoundary>
          </div>
        </div>
        )}

        {/* Left panel: LibMembersPanel（code tab 常驻，无按钮开关——用户已确认） */}
        {activeTabType === "code" && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 1,
            overflow: "hidden",
            background: "var(--ol-surface)",
            borderRight: "1px solid var(--ol-border)",
          }}
        >
          <LibMembersPanel />
        </div>
        )}

        {/* Center: DesignCanvas for components, CodeFlowCanvas for store/code */}
        <div
          style={{
            gridRow: 3,
            gridColumn: showDesignPanels ? 2 : activeTabType === "code" ? 2 : "1 / -1",
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
            background: "var(--ol-gray-100)",
          }}
        >
          {activeTabType === "design" ? (
            <DesignTabContent />
          ) : activeTabType === "component" ? (
            /* Z-axis 双层叠加：设计层（DOM 画布）常显，逻辑层（醒态 fn 图）浮于其上 */
            /* M2 T8 实机发现：zAxisRef 容器须为 flex 容器（InfiniteCanvas 根节点 flex:1
                只在 flex 容器内生效），否则高度塌陷 → Artboard 被推出视口（y≈-426） */
            <div
              ref={zAxisRef}
              style={{
                flex: 1,
                position: "relative",
                overflow: "hidden",
                display: "flex",
                flexDirection: "column",
              }}
            >
              <InfiniteCanvas>
                <OmniViewRenderer />
              </InfiniteCanvas>
              <LogicLayer visible={logicLayerVisible}>
                <CodeFlowCanvas />
              </LogicLayer>
              {/* 数据区（M2 T4）：源节点卡片层——醒态 + [数据节点] 开关门控，右上角自动排布可拖动 */}
              <SourceNodeLayer />
              {/* 呼吸点（T3）：角落小脉搏整页睡/醒开关，绝对定位叠加 */}
              <BreathingPoint />
              {/* 语境面板（T6）：浮动主/副面板 + 浮标，叠加在画布层之上 */}
              <ContextPanel />
            </div>
          ) : activeTabType === "store" && activeTab ? (
            <StorePage storeName={activeTab.componentName} tabId={activeTab.id} />
          ) : (
            <CodeFlowCanvas />
          )}
        </div>

        {/* Store Drawer — overlays center area when visible */}
        {storePanelVisible && activeTabType === "component" && (
          <div style={{
            gridRow: 3,
            gridColumn: showDesignPanels ? 2 : "1 / -1",
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

        {/* [组件库] 弹出面板（M2 T6）：底栏开关，DesignPage 受控 paletteOpen */}
        {paletteOpen && <PaletteDrawer onClose={() => setPaletteOpen(false)} />}

        {/* 底栏（M2）：左 = 面包屑；右 = [组件库] 弹出面板开关 + [数据节点] 显隐开关 */}
        <div style={{ gridRow: 4, gridColumn: "1 / -1" }}>
          <BottomBar onPaletteToggle={() => setPaletteOpen(!paletteOpen)} />
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
