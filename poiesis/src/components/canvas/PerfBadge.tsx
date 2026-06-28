/** Tiny FPS/element counter badge for the toolbar. */
import React from "react";
import { usePerfMetrics } from "./PerfMonitor";

export function PerfBadge(): React.ReactElement {
  const m = usePerfMetrics();

  if (m.fps === 0) return <span style={{ fontSize: 10, color: "#9CA3AF" }}>— fps</span>;

  const fpsColor = m.meetsGate ? "#10B981" : m.fps >= 20 ? "#F59E0B" : "#EF4444";

  return (
    <span
      title={`Avg frame: ${m.avgFrameTime.toFixed(1)}ms, Max: ${m.maxFrameTime.toFixed(1)}ms, Elements: ${m.elementCount}, Wires: ${m.wireCount}`}
      style={{ fontSize: 10, color: fpsColor, fontFamily: "monospace" }}
    >
      {m.fps}fps {m.elementCount}el
    </span>
  );
}
