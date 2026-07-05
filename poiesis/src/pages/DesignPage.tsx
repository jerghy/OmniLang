// pages/DesignPage.tsx
// Top-level design page with tabbed editor, left panels, and Z-axis canvas.
// Drag-and-drop from palette to canvas uses @dnd-kit/core (pointer events, not native HTML5 DnD).

import { useMemo, useState, useCallback, useRef, Component, type ReactNode } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  closestCenter,
  type DragStartEvent,
  type DragMoveEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import { DesignTabBar } from "../components/design/DesignTabBar";
import { DesignCanvas } from "../components/design/DesignCanvas";
import { CodeFlowCanvas } from "../components/design/CodeFlowCanvas";
import { BreadcrumbBar } from "../components/design/BreadcrumbBar";
import { ComponentPalette } from "../components/panels/ComponentPalette";
import { ComponentTree } from "../components/panels/ComponentTree";
import { ContextInspector } from "../components/panels/ContextInspector";
import { createCommandContext } from "../core/commands/createCommandContext";
import { usePoiesisStore } from "../store";
import { api } from "../commands";
import { generateOmniFiles } from "../services/codegen/pipeline";
import { uid } from "../core/utils/id";
import { getDefaultProps } from "../core/constants/defaults";
import type { ElementData } from "../core/types/element";
import type { CodeTab } from "../core/types/codegraph";
import { MoveElement } from "../core/commands/MoveElement";
import { ReorderElement } from "../core/commands/ReorderElement";
import type { TreeDropState } from "../components/panels/TreeNodeRenderer";

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
  const ctx = useMemo(() => createCommandContext(), []);
  const addElement = usePoiesisStore((s) => s.addElement);
  const undo = usePoiesisStore((s) => s.undo);
  const isDirty = usePoiesisStore((s) => s.isDirty);
  const markClean = usePoiesisStore((s) => s.markClean);
  const activeTabType = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return "component" as const;
    return s.designPage.openTabs.find((t) => t.id === activeId)?.tabType ?? "component";
  });
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  // Side panels are visible only for component tabs NOT in code-flow mode
  const showSidePanels = activeTabType === "component" && !codeFlowVisible;
  const [saving, setSaving] = useState(false);
  const [activeComponent, setActiveComponent] = useState<string | null>(null);

  // ── @dnd-kit sensors (unified context handles both palette and tree drags) ──
  const pointerSensor = useSensor(PointerSensor, {
    activationConstraint: { distance: 5 },
  });
  const keyboardSensor = useSensor(KeyboardSensor);
  const sensors = useSensors(pointerSensor, keyboardSensor);

  // ── Tree drag state (handled in unified DndContext) ──
  const [treeDropState, setTreeDropState] = useState<TreeDropState | null>(null);
  const treeDropRef = useRef<TreeDropState | null>(null);
  const [activeTreeId, setActiveTreeId] = useState<string | null>(null);

  // ── Command executor ──
  const exec = useCallback(
    (cmd: import("../core/commands/types").Command) => cmd.execute(ctx),
    [ctx],
  );

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const store = usePoiesisStore.getState();
      const activeTabId = store.designPage.activeTabId;
      const activeTab = activeTabId
        ? store.designPage.openTabs.find((t) => t.id === activeTabId)
        : null;

      // ── 1. Build codeTabs from current elements+wires ──
      const codeTabs: CodeTab[] = [
        {
          id: "main",
          type: "logic",
          label: "Main",
          icon: "📄",
          graph: { elements: store.elements, wires: store.wires },
          variables: [],
        },
      ];

      // ── 2. Generate .omni source via codegen pipeline ──
      const componentName = activeTab?.componentName ?? "main";
      const genResult = generateOmniFiles({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName,
        rootOrder: store.rootOrder,
        forSave: true,
        filePath: activeTab?.filePath,
      });

      if (genResult.value) {
        // ── 3. Write each generated .omni file to disk ──
        for (const [filePath, source] of Object.entries(genResult.value)) {
          await api.project.writeFile(filePath, source);
        }
      } else if (genResult.errors.some((e) => e.severity === "fatal")) {
        console.error("[save] codegen errors:", genResult.errors);
        return;
      }

      // ── 4. Save project.json (editor metadata) ──
      const result = await api.annotation.saveProject();
      if (result.ok) {
        markClean();
      }

      // ── 5. Update custom component def with designed elements ──
      // This ensures expandCustomComponents() can inline the component's
      // content when it's used as a custom component instance on another canvas.
      if (componentName !== "main") {
        const currentDef = store.componentDefs[componentName] || {};
        // setComponentDef expects ComponentDef but stores arbitrary shapes;
        // elements/wires/rootOrder are needed for expandCustomComponents()
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (store.setComponentDef as any)(componentName, {
          ...currentDef,
          elements: store.elements,
          wires: store.wires,
          rootOrder: store.rootOrder,
        });
      }
    } catch (e) {
      console.error("[save] failed:", e);
    } finally {
      setSaving(false);
    }
  }, [markClean]);

  // ── Unified drag start (palette + tree) ──
  const handleDragStart = (event: DragStartEvent) => {
    const data = event.active.data.current;
    if (data?.type === "tree-node") {
      setActiveTreeId(data.elementId as string);
      return;
    }
    const tag = data?.tag as string | undefined;
    setActiveComponent(tag ?? null);
  };

  // ── Unified drag move (tree reorder position calculation) ──
  const handleDragMove = (event: DragMoveEvent) => {
    const data = event.active.data.current;
    if (data?.type !== "tree-node") return;

    const over = event.over;
    if (!over || typeof over.id !== "string" || !over.id.startsWith("tree-")) {
      setTreeDropState(null);
      treeDropRef.current = null;
      return;
    }

    const targetId = over.id.replace("tree-", "");
    const activeRect = event.active.rect.current.translated;
    const overRect = over.rect;

    if (!activeRect || !overRect) {
      setTreeDropState(null);
      treeDropRef.current = null;
      return;
    }

    // top 25% → before, bottom 25% → after, middle 50% → inside
    const dragCenterY = activeRect.top + activeRect.height / 2;
    const overTop = overRect.top;
    const overHeight = overRect.height;

    let position: "before" | "after" | "inside";
    if (dragCenterY < overTop + overHeight * 0.25) {
      position = "before";
    } else if (dragCenterY > overTop + overHeight * 0.75) {
      position = "after";
    } else {
      position = "inside";
    }

    if (targetId === data.elementId) {
      setTreeDropState(null);
      treeDropRef.current = null;
      return;
    }

    setTreeDropState({ targetId, position });
    treeDropRef.current = { targetId, position };
  };

  // ── Unified drag end (palette drop + tree reorder/nest) ──
  const handleDragEnd = (event: DragEndEvent) => {
    const data = event.active.data.current;

    // ── Tree-node drop ──
    if (data?.type === "tree-node") {
      setActiveTreeId(null);
      const draggedId = data.elementId as string | undefined;
      const dropInfo = treeDropRef.current;
      setTreeDropState(null);
      treeDropRef.current = null;

      // Resolve target from dropInfo (pointer drag) or event.over (keyboard / no-move drags)
      let targetId: string | undefined;
      let position: "before" | "after" | "inside" = "after";
      if (dropInfo && dropInfo.targetId !== draggedId) {
        targetId = dropInfo.targetId;
        position = dropInfo.position;
      } else if (event.over && typeof event.over.id === "string" && event.over.id.startsWith("tree-")) {
        targetId = event.over.id.replace("tree-", "");
      }

      if (!draggedId || !targetId || draggedId === targetId) return;

      if (position === "inside") {
        exec(new MoveElement(draggedId, targetId, null, "after"));
      } else {
        const draggedEl = ctx.getElement(draggedId);
        const targetEl = ctx.getElement(targetId);

        if (draggedEl && targetEl && draggedEl.parentId === targetEl.parentId) {
          exec(new ReorderElement(draggedId, targetId, position));
        } else if (targetEl) {
          exec(new MoveElement(draggedId, targetEl.parentId ?? (null as unknown as string), targetId, position));
        }
      }
      return;
    }

    // ── Palette drop on canvas ──
    setActiveComponent(null);

    if (event.over?.id !== "design-canvas") return;

    const tag = data?.tag as string | undefined;
    if (!tag) return;

    const id = uid(tag.toLowerCase());
    const props = getDefaultProps(tag);

    // Only PascalCase tags are custom component references; built-in HTML
    // tags (div, p, button, etc.) should not carry a componentRef.
    const isPascal = /^[A-Z]/.test(tag);

    const newEl: ElementData = {
      id,
      nodeType: "view",
      tag,
      props,
      children: [],
      parentId: null,
      ...(isPascal ? { componentRef: tag } : {}),
    };

    addElement(newEl);
  };

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
            title="CSS 样式设计"
            style={{
              padding: "2px 10px",
              fontSize: 12,
              border: "1px solid #d1d5db",
              borderRadius: 4,
              background: "#fff",
              cursor: "pointer",
              color: "#6366F1",
              fontWeight: 600,
            }}
          >
            🎨 CSS 样式
          </button>
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
            <ComponentTree ctx={ctx} treeDropState={treeDropState} />
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
            background: "#f1f5f9",
          }}
        >
          {activeTabType === "component" ? (
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
      {el.tag}
      {el.props?.label ? ` "${String(el.props.label).slice(0, 20)}"` : ""}
    </div>
  );
}
