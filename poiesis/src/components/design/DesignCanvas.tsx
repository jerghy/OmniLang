// components/design/DesignCanvas.tsx
// Z-axis dual-layer container with animated transition between OmniView and CodeFlow layers.
// Drag-and-drop from the palette is handled via @dnd-kit/core — the canvas wrapper is a
// droppable target that accepts palette draggables and delegates creation to DesignPage's
// DndContext onDragEnd handler.

import { useDroppable } from "@dnd-kit/core";
import { usePoiesisStore } from "../../store";
import { OmniViewRenderer } from "./OmniViewRenderer";
import { CodeFlowCanvas } from "./CodeFlowCanvas";

export function DesignCanvas() {
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);

  const { setNodeRef, isOver } = useDroppable({
    id: "design-canvas",
  });

  return (
    <div
      ref={setNodeRef}
      style={{
        flex: 1,
        position: "relative",
        overflow: "hidden",
        outline: isOver ? "2px dashed #4F46E5" : "none",
        outlineOffset: "-4px",
      }}
    >
      {/* Upper layer: OmniViewRenderer (component preview) */}
      <div
        style={{
          position: "absolute",
          transition: "all 300ms ease",
          top: 0,
          left: 0,
          right: codeFlowVisible ? "60%" : 0,
          bottom: codeFlowVisible ? "60%" : 0,
          zIndex: 1,
          borderRadius: codeFlowVisible ? 8 : 0,
          boxShadow: codeFlowVisible
            ? "0 4px 16px rgba(0,0,0,0.15)"
            : "none",
          overflow: "auto",
          background: "#fff",
        }}
      >
        <OmniViewRenderer />
      </div>

      {/* Lower layer: CodeFlowCanvas (code node flow, Phase 5) */}
      {codeFlowVisible && (
        <div
          onDoubleClick={() => toggleCodeFlow(null)}
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
  );
}
