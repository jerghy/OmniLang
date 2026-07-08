// pages/DesignPage.tsx
// Top-level design page with tabbed editor, left panels, and Z-axis canvas.
// Drag-and-drop from palette to canvas uses @dnd-kit/core (pointer events, not native HTML5 DnD).

import { Component, useState, useEffect, useCallback, type ReactNode } from "react";
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
import { api } from "../commands";
import type { RecentProject } from "../commands/project-api";
import { DesignTabContent } from "../components/design/DesignTabContent";
import { DesignCanvas } from "../components/design/DesignCanvas";
import { CodeFlowCanvas } from "../components/design/CodeFlowCanvas";
import { BreadcrumbBar } from "../components/design/BreadcrumbBar";
import { ComponentPalette } from "../components/panels/ComponentPalette";
import { ComponentTree } from "../components/panels/ComponentTree";
import { ContextInspector } from "../components/panels/ContextInspector";
import { usePoiesisStore } from "../store";
import { useSaveProject } from "../hooks/useSaveProject";
import { useDragHandlers } from "../hooks/useDragHandlers";

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
        <div style={{ padding: 16, color: "#dc2626", fontSize: 12, background: "#fef2f2", border: "1px solid #fecaca", borderRadius: 4, margin: 4 }}>
          <strong>⚠ {this.props.name}</strong>
          <div style={{ marginTop: 4, opacity: 0.7 }}>{this.state.error.message}</div>
        </div>
      );
    }
    return this.props.children;
  }
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
  // Side panels are visible only for component tabs NOT in code-flow mode
  const showSidePanels = activeTabType === "component" && !codeFlowVisible;

  const { saving, saveError, handleSave } = useSaveProject();
  const { handleDragStart, handleDragMove, handleDragEnd, activeComponent, activeTreeId, treeDropState } = useDragHandlers();

  // ── Open Project panel ──
  const [showProjectPanel, setShowProjectPanel] = useState(false);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);

  useEffect(() => {
    if (showProjectPanel) {
      api.project.getRecentProjects().then(setRecentProjects).catch(() => {});
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
      /* ignore */
    }
  }, [onProjectOpened]);

  const handleOpenRecent = useCallback(async (proj: RecentProject) => {
    try {
      await api.project.setProjectDir(proj.path);
      await api.project.addRecentProject(proj.name, proj.path);
      setShowProjectPanel(false);
      onProjectOpened?.(proj.path, proj.name);
    } catch (e) {
      /* ignore */
    }
  }, [onProjectOpened]);

  const handleRemoveRecent = useCallback(async (e: React.MouseEvent, path: string) => {
    e.stopPropagation();
    await api.project.removeRecentProject(path);
    const list = await api.project.getRecentProjects();
    setRecentProjects(list);
  }, []);

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
          gridTemplateColumns: showSidePanels ? "280px 1fr 300px" : "1fr",
          gridTemplateRows: "36px 32px 1fr 24px",
          height: "100vh",
          background: "#f8fafc",
        }}
      >
        {/* Tab bar spans full width */}
        <div style={{ gridRow: 1, gridColumn: "1 / -1" }}>
          <DesignTabBar />
        </div>

        {/* Toolbar row — undo / save / CSS styles */}
        <div
          style={{
            gridRow: 2,
            gridColumn: "1 / -1",
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "0 12px",
            background: "#fff",
            borderBottom: "1px solid #e2e8f0",
          }}
        >
          <button
            onClick={undo}
            title="撤销 (Ctrl+Z)"
            style={{
              padding: "2px 8px",
              fontSize: 13,
              border: "1px solid transparent",
              borderRadius: 4,
              background: "transparent",
              cursor: "pointer",
              lineHeight: 1,
            }}
          >
            ↩
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            title="保存项目 (Ctrl+S)"
            style={{
              padding: "2px 8px",
              fontSize: 13,
              border: "1px solid transparent",
              borderRadius: 4,
              background: "transparent",
              cursor: saving ? "wait" : "pointer",
              lineHeight: 1,
              color: isDirty ? "#DC2626" : "#6B7280",
              fontWeight: isDirty ? 700 : 400,
              opacity: saving ? 0.6 : 1,
            }}
          >
            {saving ? "⏳" : "💾"}
          </button>
          <button
            onClick={() => setShowProjectPanel(true)}
            title="打开项目"
            style={{
              padding: "2px 8px",
              fontSize: 13,
              border: "1px solid transparent",
              borderRadius: 4,
              background: "transparent",
              cursor: "pointer",
              lineHeight: 1,
              color: "#6B7280",
            }}
          >
            📂
          </button>
          {saveError && (
            <span
              title={saveError}
              style={{
                marginLeft: 8,
                fontSize: 12,
                color: "#dc2626",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
                maxWidth: 300,
              }}
            >
              ⚠ {saveError}
            </span>
          )}
        </div>

        {/* Left panel: ComponentPalette + ComponentTree (component tabs, hidden in code-flow mode) */}
        {showSidePanels && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 1,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
            background: "#fff",
            borderRight: "1px solid #e2e8f0",
          }}
        >
          <div style={{ flex: "0 0 auto", maxHeight: "50%", overflow: "auto" }}>
            <PanelErrorBoundary name="组件面板">
              <ComponentPalette />
            </PanelErrorBoundary>
          </div>
          <div style={{ flex: 1, borderTop: "1px solid #e2e8f0", overflow: "auto" }}>
            <PanelErrorBoundary name="组件树">
              <ComponentTree treeDropState={treeDropState} />
            </PanelErrorBoundary>
          </div>
        </div>
        )}

        {/* Center: DesignCanvas for components, CodeFlowCanvas for store/code */}
        <div
          style={{
            gridRow: 3,
            gridColumn: showSidePanels ? 2 : "1 / -1",
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
            background: "#f1f5f9",
          }}
        >
          {activeTabType === "design" ? (
            <DesignTabContent />
          ) : activeTabType === "component" ? (
            <DesignCanvas />
          ) : (
            <CodeFlowCanvas />
          )}
        </div>

        {/* Right panel: ContextInspector (component tabs, hidden in code-flow mode) */}
        {showSidePanels && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 3,
            overflow: "hidden",
            display: "flex",
            background: "#fff",
            borderLeft: "1px solid #e2e8f0",
          }}
        >
          <ContextInspector />
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
          <div
            style={{
              padding: "6px 14px",
              background: "#4F46E5",
              color: "#fff",
              borderRadius: 6,
              fontSize: 12,
              fontWeight: 600,
              boxShadow: "0 4px 12px rgba(79, 70, 229, 0.35)",
              cursor: "grabbing",
            }}
          >
            {activeComponent}
          </div>
        ) : null}
      </DragOverlay>

      {/* ── Open Project Panel (modal overlay) ── */}
      {showProjectPanel && (
        <div
          style={{
            position: "fixed", inset: 0, zIndex: 200,
            background: "rgba(0,0,0,0.3)",
            display: "flex", alignItems: "center", justifyContent: "center",
          }}
          onClick={() => setShowProjectPanel(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              background: "#fff",
              borderRadius: 16,
              padding: 28,
              width: 420,
              maxWidth: "90vw",
              boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
            }}
          >
            <div style={{ fontSize: 18, fontWeight: 700, color: "#1f2937", marginBottom: 20 }}>
              打开项目
            </div>

            {/* Open folder button */}
            <button
              onClick={handleOpenProject}
              style={{
                display: "flex", alignItems: "center", gap: 8,
                width: "100%", padding: "16px 20px",
                background: "#fff", border: "1px solid #e5e7eb",
                borderRadius: 12, cursor: "pointer",
                fontFamily: "inherit", textAlign: "left",
                fontSize: 14, fontWeight: 600, color: "#1f2937",
              }}
            >
              <span style={{ fontSize: 24 }}>📂</span>
              <span>打开项目</span>
            </button>

            {/* Recent projects */}
            {recentProjects.length > 0 && (
              <div style={{ marginTop: 20 }}>
                <div style={{
                  fontSize: 12, fontWeight: 600, color: "#9ca3af",
                  textTransform: "uppercase", letterSpacing: 1, marginBottom: 8,
                }}>
                  最近打开
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {recentProjects.map((proj) => (
                    <button
                      key={proj.path}
                      onClick={() => handleOpenRecent(proj)}
                      style={{
                        display: "flex", alignItems: "center", gap: 10,
                        width: "100%", padding: "10px 12px",
                        background: "#fff", border: "1px solid #f3f4f6",
                        borderRadius: 8, cursor: "pointer",
                        fontFamily: "inherit", textAlign: "left",
                        fontSize: 13, color: "#1f2937", fontWeight: 600,
                      }}
                    >
                      <span style={{ fontSize: 18, flexShrink: 0 }}>📁</span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ fontWeight: 600, color: "#1f2937", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {proj.name}
                        </div>
                        <div style={{ fontSize: 11, color: "#9ca3af", marginTop: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {proj.path}
                        </div>
                      </div>
                      <span
                        onClick={(e) => handleRemoveRecent(e, proj.path)}
                        style={{
                          fontSize: 16, color: "#d1d5db", padding: "2px 6px",
                          borderRadius: 4, flexShrink: 0, cursor: "pointer",
                        }}
                        title="移除"
                      >
                        ×
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Close button */}
            <button
              onClick={() => setShowProjectPanel(false)}
              style={{
                marginTop: 16, width: "100%", padding: "8px 0",
                border: "none", background: "transparent",
                cursor: "pointer", fontSize: 12, color: "#9ca3af",
                fontFamily: "inherit",
              }}
            >
              取消
            </button>
          </div>
        </div>
      )}
    </DndContext>
  );
}

/** Drag overlay preview for tree-node drags. */
function TreeDragPreview({ elementId }: { elementId: string }) {
  const el = usePoiesisStore((s) => s.elements.find((e) => e.id === elementId));
  if (!el) return null;
  return (
    <div
      style={{
        padding: "3px 10px",
        background: "#4F46E5",
        color: "#fff",
        borderRadius: 4,
        fontSize: 11,
        fontWeight: 500,
        boxShadow: "0 2px 8px rgba(79, 70, 229, 0.3)",
        cursor: "grabbing",
      }}
    >
      {"tag" in el ? el.tag : ""}
      {"props" in el && el.props?.label ? ` "${String(el.props.label).slice(0, 20)}"` : ""}
    </div>
  );
}
