// poiesis/src/components/canvas/CanvasGrid.tsx
import React from "react";

export const CanvasGrid = React.memo(function CanvasGrid() {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        backgroundImage: `
          radial-gradient(circle, var(--ol-border-hover) 1px, transparent 1px)
        `,
        backgroundSize: "20px 20px",
        pointerEvents: "none",
        zIndex: 0,
      }}
    />
  );
});
