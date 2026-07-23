// poiesis/src/components/canvas/Artboard.tsx
import React from "react";
import { useCanvasStore } from "./canvas-store";
import { ArtboardHeader } from "./ArtboardHeader";
import { ArtboardContent } from "./ArtboardContent";
import styles from "./Artboard.module.css";

export interface ArtboardProps {
  children: React.ReactNode;
}

export const Artboard = React.memo(function Artboard({ children }: ArtboardProps) {
  const devicePreset = useCanvasStore((s) => s.devicePreset);

  return (
    <div
      className={styles.artboard}
      style={{
        width: devicePreset.width || undefined,
        minHeight: devicePreset.height || undefined,
        maxHeight: devicePreset.height || undefined,
      }}
    >
      <ArtboardHeader />
      <ArtboardContent>
        {children}
      </ArtboardContent>
    </div>
  );
});
