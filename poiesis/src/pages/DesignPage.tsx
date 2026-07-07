// pages/DesignPage.tsx
// Top-level design page with tabbed editor, left panels, and Z-axis canvas.
// Drag-and-drop from palette to canvas uses @dnd-kit/core (pointer events, not native HTML5 DnD).

import { Component, type ReactNode } from "react";
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

export function DesignPage() {
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
