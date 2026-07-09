// poiesis/src/components/canvas/ArtboardContent.tsx
import React, { useRef, useEffect } from "react";
import { useCanvasStore } from "./canvas-store";

export interface ArtboardContentProps {
  children: React.ReactNode;
}

export const ArtboardContent = React.memo(function ArtboardContent({
  children,
}: ArtboardContentProps) {
  const setArtboardScroll = useCanvasStore((s) => s.setArtboardScroll);
  const scrollRef = useRef<HTMLDivElement>(null);
  const styleContainerRef = useRef<HTMLDivElement>(null);

  // Listen to scroll events on the artboard content
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const handler = () => {
      setArtboardScroll(el.scrollTop, el.scrollLeft);
    };
    el.addEventListener("scroll", handler, { passive: true });
    return () => el.removeEventListener("scroll", handler);
  }, [setArtboardScroll]);

  return (
    <div
      ref={scrollRef}
      style={{
        flex: 1,
        overflow: "auto",
        position: "relative",
      }}
    >
      {/* CSS scope pool container — managed by useCssScopePool */}
      <div ref={styleContainerRef} data-css-scopes="" />
      {children}
    </div>
  );
});
