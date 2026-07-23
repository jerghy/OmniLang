// poiesis/src/components/canvas/ArtboardHeader.tsx
import React from "react";
import { useCanvasStore, DEVICE_PRESETS } from "./canvas-store";
import styles from "./ArtboardHeader.module.css";

export const ArtboardHeader = React.memo(function ArtboardHeader() {
  const devicePreset = useCanvasStore((s) => s.devicePreset);
  const setDevicePreset = useCanvasStore((s) => s.setDevicePreset);

  return (
    <div
      className={styles.header}
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
