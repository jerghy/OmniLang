import { lazy, Suspense, useState } from "react";
import { ArchitectureToolbar } from "../components/architecture/ArchitectureToolbar";
import { ArchitecturePreview } from "../components/architecture/ArchitecturePreview";

const ArchitectureGraph = lazy(() => import("../components/architecture/ArchitectureGraph").then((m) => ({ default: m.ArchitectureGraph })));

export function ArchitecturePage() {
  const [viewMode, setViewMode] = useState<"preview" | "graph">("graph");

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        background: "var(--ol-gray-50)",
      }}
    >
      <ArchitectureToolbar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
      />
      <div style={{ flex: 1, position: "relative" }}>
        {viewMode === "preview" ? <ArchitecturePreview /> : (
          <Suspense fallback={<div style={{ padding: 40, color: "var(--ol-text-secondary)", fontSize: 14 }}>Loading graph...</div>}>
            <ArchitectureGraph />
          </Suspense>
        )}
      </div>
    </div>
  );
}
