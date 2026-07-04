import { useState } from "react";
import { ArchitectureGraph } from "../components/architecture/ArchitectureGraph";
import { ArchitectureToolbar } from "../components/architecture/ArchitectureToolbar";
import { ArchitecturePreview } from "../components/architecture/ArchitecturePreview";

export function ArchitecturePage() {
  const [viewMode, setViewMode] = useState<"preview" | "graph">("graph");

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        background: "#f8fafc",
      }}
    >
      <ArchitectureToolbar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
      />
      <div style={{ flex: 1, position: "relative" }}>
        {viewMode === "preview" ? <ArchitecturePreview /> : <ArchitectureGraph />}
      </div>
    </div>
  );
}
