// poiesis/src/components/canvas/ArtboardHeader.tsx
import React from "react";
import { useCanvasStore, DEVICE_PRESETS } from "./canvas-store";

export const ArtboardHeader = React.memo(function ArtboardHeader() {
  const devicePreset = useCanvasStore((s) => s.devicePreset);
  const setDevicePreset = useCanvasStore((s) => s.setDevicePreset);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "4px 12px",
        background: "#f1f5f9",
        borderBottom: "1px solid #e2e8f0",
        fontSize: 12,
        color: "#475569",
        flexShrink: 0,
      }}
    >
      <select
        aria-label="Device preset"
        value={devicePreset.name}
        onChange={(e) => {
          const preset = DEVICE_PRESETS.find((p) => p.name === e.target.value);
          if (preset) setDevicePreset(preset);
        }}
        style={{
          border: "1px solid #e2e8f0",
          borderRadius: 4,
          padding: "2px 6px",
          fontSize: 12,
          fontFamily: "inherit",
          background: "#fff",
        }}
      >
        {DEVICE_PRESETS.map((p) => (
          <option key={p.name} value={p.name}>{p.name}</option>
        ))}
      </select>
      <span>
        {devicePreset.width} &times; {devicePreset.height || "auto"}
      </span>
    </div>
  );
});
