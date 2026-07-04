// pages/DesignPage.tsx
// Top-level design page with tabbed editor, left panels, and Z-axis canvas.

import { useMemo, Component, type ReactNode } from "react";
import { DesignTabBar } from "../components/design/DesignTabBar";
import { DesignCanvas } from "../components/design/DesignCanvas";
import { BreadcrumbBar } from "../components/design/BreadcrumbBar";
import { ComponentPalette } from "../components/panels/ComponentPalette";
import { ComponentTree } from "../components/panels/ComponentTree";
import { ContextInspector } from "../components/panels/ContextInspector";
import { createCommandContext } from "../core/commands/createCommandContext";

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

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "280px 1fr 300px",
        gridTemplateRows: "36px 1fr 24px",
        height: "100vh",
        background: "#f8fafc",
      }}
    >
      {/* Tab bar spans full width */}
      <div style={{ gridRow: 1, gridColumn: "1 / -1" }}>
        <DesignTabBar />
      </div>

      {/* Left panel: ComponentPalette + ComponentTree */}
      <div
        style={{
          gridRow: 2,
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
          <ComponentTree ctx={ctx} />
        </div>
      </div>

      {/* Center: DesignCanvas (Z-axis dual-layer) */}
      <div
        style={{
          gridRow: 2,
          gridColumn: 2,
          overflow: "hidden",
          display: "flex",
          background: "#f1f5f9",
        }}
      >
        <DesignCanvas />
      </div>

      {/* Right panel: ContextInspector */}
      <div
        style={{
          gridRow: 2,
          gridColumn: 3,
          overflow: "hidden",
          display: "flex",
          background: "#fff",
          borderLeft: "1px solid #e2e8f0",
        }}
      >
        <ContextInspector />
      </div>

      {/* Breadcrumb bar spans full width */}
      <div style={{ gridRow: 3, gridColumn: "1 / -1" }}>
        <BreadcrumbBar />
      </div>
    </div>
  );
}
