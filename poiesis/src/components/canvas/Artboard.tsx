// poiesis/src/components/canvas/Artboard.tsx
import React from "react";
import { useCanvasStore } from "./canvas-store";
import { ArtboardHeader } from "./ArtboardHeader";
import { ArtboardContent } from "./ArtboardContent";

export interface ArtboardProps {
  children: React.ReactNode;
}

export const Artboard = React.memo(function Artboard({ children }: ArtboardProps) {
  const devicePreset = useCanvasStore((s) => s.devicePreset);

  return (
    <div
      style={{
        position: "absolute",
        // Center in canvas
        left: "50%",
        top: "50%",
        transform: "translate(-50%, -50%)",
        width: devicePreset.width || undefined,
        minHeight: devicePreset.height || undefined,
        maxHeight: devicePreset.height || undefined,
        background: "#fff",
        boxShadow: "0 0 0 1px #e2e8f0, 0 4px 24px rgba(0,0,0,0.08)",
        borderRadius: 4,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <ArtboardHeader />
      <ArtboardContent>
        {children}
      </ArtboardContent>
    </div>
  );
});
