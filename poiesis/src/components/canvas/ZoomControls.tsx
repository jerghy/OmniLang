// poiesis/src/components/canvas/ZoomControls.tsx
import React, { useCallback } from "react";
import { useCanvasStore } from "./canvas-store";
import styles from "./ZoomControls.module.css";

export const ZoomControls = React.memo(function ZoomControls() {
  const zoom = useCanvasStore((s) => s.zoom);
  const setZoom = useCanvasStore((s) => s.setZoom);
  const fitToView = useCanvasStore((s) => s.fitToView);
  const resetView = useCanvasStore((s) => s.resetView);

  const handleFit = useCallback(() => {
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    fitToView(vw - 600, vh - 120); // account for panels
  }, [fitToView]);

  return (
    <div className={styles.container}>
      <button type="button"
        onClick={() => setZoom(zoom - 0.1)}
        style={btnStyle}
        title="Zoom out"
      >&minus;</button>
      <button
        type="button"
        tabIndex={0}
        className={styles.zoomLabel}
        onClick={resetView}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); resetView(); } }}
        title="Reset zoom"
      >
        {Math.round(zoom * 100)}%
      </button>
      <button type="button"
        onClick={() => setZoom(zoom + 0.1)}
        style={btnStyle}
        title="Zoom in"
      >+</button>
      <button type="button"
        onClick={handleFit}
        style={{ ...btnStyle, marginLeft: 4 }}
        title="Fit to view"
      >
        &#x229E;
      </button>
    </div>
  );
});

const btnStyle: React.CSSProperties = {
  width: 24,
  height: 24,
  border: "none",
  background: "transparent",
  cursor: "pointer",
  borderRadius: 4,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontSize: 14,
  color: "var(--ol-text-secondary)",
};
