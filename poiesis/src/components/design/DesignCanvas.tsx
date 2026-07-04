// components/design/DesignCanvas.tsx
// Z-axis dual-layer container with animated transition between OmniView and CodeFlow layers.

import { usePoiesisStore } from "../../store";
import { OmniViewRenderer } from "./OmniViewRenderer";
import { CodeFlowCanvas } from "./CodeFlowCanvas";

export function DesignCanvas() {
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);

  return (
    <div
      style={{
        flex: 1,
        position: "relative",
        overflow: "hidden",
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
