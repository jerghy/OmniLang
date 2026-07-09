// poiesis/src/components/canvas/InfiniteCanvas.tsx
import React, { useCallback, useRef, useState } from "react";
import { useCanvasStore } from "./canvas-store";
import { CanvasGrid } from "./CanvasGrid";
import { Artboard } from "./Artboard";
import { ZoomControls } from "./ZoomControls";

export interface InfiniteCanvasProps {
  children: React.ReactNode;
}

export function InfiniteCanvas({ children }: InfiniteCanvasProps) {
  const zoom = useCanvasStore((s) => s.zoom);
  const panX = useCanvasStore((s) => s.panX);
  const panY = useCanvasStore((s) => s.panY);
  const setPan = useCanvasStore((s) => s.setPan);
  const zoomAtPoint = useCanvasStore((s) => s.zoomAtPoint);

  const isPanning = useRef(false);
  const lastPointer = useRef({ x: 0, y: 0 });
  const [spaceHeld, setSpaceHeld] = useState(false);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const rect = e.currentTarget.getBoundingClientRect();
        const newZoom = zoom * (1 - e.deltaY * 0.001);
        zoomAtPoint(newZoom, e.clientX - rect.left, e.clientY - rect.top);
      }
      // Plain scroll: let it bubble to ArtboardContent (overflow:auto)
    },
    [zoom, zoomAtPoint],
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      // Space+drag or middle-button drag = pan
      if (spaceHeld || e.button === 1) {
        e.preventDefault();
        isPanning.current = true;
        lastPointer.current = { x: e.clientX, y: e.clientY };
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      }
    },
    [],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isPanning.current) return;
      const dx = e.clientX - lastPointer.current.x;
      const dy = e.clientY - lastPointer.current.y;
      lastPointer.current = { x: e.clientX, y: e.clientY };
      setPan(panX + dx, panY + dy);
    },
    [panX, panY, setPan],
  );

  const handlePointerUp = useCallback(() => {
    isPanning.current = false;
  }, []);

  // Keyboard: space for pan mode
  React.useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === "Space" && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) {
        e.preventDefault();
        setSpaceHeld(true);
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "Space") setSpaceHeld(false);
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, []);

  return (
    <div
      style={{
        flex: 1,
        position: "relative",
        overflow: "hidden",
        cursor: spaceHeld ? "grab" : "default",
        background: "#f8fafc",
      }}
      onWheel={handleWheel}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      <CanvasGrid />

      {/* Transformed canvas layer */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          transform: `scale(${zoom}) translate(${panX / zoom}px, ${panY / zoom}px)`,
          transformOrigin: "0 0",
        }}
      >
        <Artboard>
          {children}
        </Artboard>
      </div>

      <ZoomControls />
    </div>
  );
}
