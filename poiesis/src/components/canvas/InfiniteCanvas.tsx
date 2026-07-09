// poiesis/src/components/canvas/InfiniteCanvas.tsx
import React, { useCallback, useRef, useState, useEffect } from "react";
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

  const wrapperRef = useRef<HTMLDivElement>(null);
  const isPanning = useRef(false);
  const lastPointer = useRef({ x: 0, y: 0 });
  const [spaceHeld, setSpaceHeld] = useState(false);

  // Refs to keep native event handlers fresh without stale closures
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;
  const panRef = useRef({ x: panX, y: panY });
  panRef.current = { x: panX, y: panY };
  const zoomAtPointRef = useRef(zoomAtPoint);
  zoomAtPointRef.current = zoomAtPoint;
  const setPanRef = useRef(setPan);
  setPanRef.current = setPan;

  // ── Native wheel handler — { passive: false } is REQUIRED for WebView2 ──
  // React's synthetic onWheel may be treated as passive by the browser,
  // which means preventDefault() is a no-op and the whole window scrolls.
  useEffect(() => {
    const el = wrapperRef.current;
    if (!el) return;

    const onWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        // Ctrl+wheel → zoom at cursor
        e.preventDefault();
        const rect = el.getBoundingClientRect();
        const newZoom = zoomRef.current * (1 - e.deltaY * 0.001);
        zoomAtPointRef.current(newZoom, e.clientX - rect.left, e.clientY - rect.top);
        return;
      }

      // No Ctrl: plain scroll inside artboard → scroll content;
      // plain scroll on canvas background → pan canvas.
      const target = e.target as HTMLElement;
      const isInArtboard = target.closest("[data-artboard-scroll]");
      if (!isInArtboard) {
        e.preventDefault();
        setPanRef.current(
          panRef.current.x - e.deltaX,
          panRef.current.y - e.deltaY,
        );
      }
    };

    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  // ── Pointer: Ctrl+drag | space+drag | middle-button → pan ──
  // Ctrl+drag lets the user pan even when the artboard fills the viewport
  // (no canvas background to grab). No modifier = normal artboard interaction.
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (e.ctrlKey || e.metaKey || spaceHeld || e.button === 1) {
        e.preventDefault();
        isPanning.current = true;
        lastPointer.current = { x: e.clientX, y: e.clientY };
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      }
    },
    [spaceHeld],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isPanning.current) return;
      const dx = e.clientX - lastPointer.current.x;
      const dy = e.clientY - lastPointer.current.y;
      lastPointer.current = { x: e.clientX, y: e.clientY };
      setPan(panRef.current.x + dx, panRef.current.y + dy);
    },
    [setPan],
  );

  const handlePointerUp = useCallback(() => {
    isPanning.current = false;
  }, []);

  // ── Keyboard: Space / Ctrl / Meta for pan-mode cursor ──
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (
        e.code === "Space" &&
        !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)
      ) {
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
      ref={wrapperRef}
      style={{
        flex: 1,
        position: "relative",
        overflow: "hidden",
        cursor: spaceHeld ? "grab" : isPanning.current ? "grabbing" : "default",
        background: "#f8fafc",
      }}
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
        <Artboard>{children}</Artboard>
      </div>

      <ZoomControls />
    </div>
  );
}
