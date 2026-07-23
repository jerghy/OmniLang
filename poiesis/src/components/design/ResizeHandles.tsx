// components/design/ResizeHandles.tsx
// Eight resize handles (corners + edges) for the interactive ClassPreview.

import { useRef, useEffect } from "react";
import styles from "./ResizeHandles.module.css";

type HandleDir = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

const ALL_HANDLES: HandleDir[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

const HANDLE_SIZE = 8;
const HANDLE_HIT = 12;

interface DragState {
  startX: number;
  startY: number;
  startW: number;
  startH: number;
}

function computeDims(
  dir: HandleDir,
  totalDx: number,
  totalDy: number,
  startW: number,
  startH: number,
): { w: number; h: number } {
  let w = startW;
  let h = startH;
  switch (dir) {
    case "e": case "ne": case "se":
      w = Math.max(4, startW + totalDx);
      break;
    case "w": case "nw": case "sw":
      w = Math.max(4, startW - totalDx);
      break;
  }
  switch (dir) {
    case "s": case "se": case "sw":
      h = Math.max(4, startH + totalDy);
      break;
    case "n": case "nw": case "ne":
      h = Math.max(4, startH - totalDy);
      break;
  }
  return { w, h };
}

const HANDLE_STYLE_MAP: Record<HandleDir, { pos: React.CSSProperties; cursor: string }> = {
  nw: { pos: { top: -HANDLE_HIT / 2, left: -HANDLE_HIT / 2 }, cursor: "nwse-resize" },
  n:  { pos: { top: -HANDLE_HIT / 2, left: "50%", marginLeft: -HANDLE_HIT / 2 }, cursor: "ns-resize" },
  ne: { pos: { top: -HANDLE_HIT / 2, right: -HANDLE_HIT / 2 }, cursor: "nesw-resize" },
  e:  { pos: { top: "50%", right: -HANDLE_HIT / 2, marginTop: -HANDLE_HIT / 2 }, cursor: "ew-resize" },
  se: { pos: { bottom: -HANDLE_HIT / 2, right: -HANDLE_HIT / 2 }, cursor: "nwse-resize" },
  s:  { pos: { bottom: -HANDLE_HIT / 2, left: "50%", marginLeft: -HANDLE_HIT / 2 }, cursor: "ns-resize" },
  sw: { pos: { bottom: -HANDLE_HIT / 2, left: -HANDLE_HIT / 2 }, cursor: "nesw-resize" },
  w:  { pos: { top: "50%", left: -HANDLE_HIT / 2, marginTop: -HANDLE_HIT / 2 }, cursor: "ew-resize" },
};

interface ResizeHandlesProps {
  previewElRef: React.RefObject<HTMLElement | null>;
  labelRef: React.RefObject<HTMLDivElement | null>;
  onResizeRef: React.MutableRefObject<((w: number, h: number) => void) | undefined>;
  onResizeEndRef: React.MutableRefObject<((w: number, h: number) => void) | undefined>;
  onDisplayDimsChange: (w: number, h: number) => void;
}

export function ResizeHandles({
  previewElRef,
  labelRef,
  onResizeRef,
  onResizeEndRef,
  onDisplayDimsChange,
}: ResizeHandlesProps) {
  const dragRef = useRef<DragState | null>(null);

  // Clean up event listeners on unmount
  useEffect(() => {
    return () => {
      dragRef.current = null;
    };
  }, []);

  const handleMouseDown = (dir: HandleDir) => (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const el = previewElRef.current;
    if (!el) return;

    const rect = el.getBoundingClientRect();
    const startW = rect.width;
    const startH = rect.height;

    dragRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      startW,
      startH,
    };

    const onMove = (me: MouseEvent) => {
      const d = dragRef.current;
      if (!d) return;
      const totalDx = me.clientX - d.startX;
      const totalDy = me.clientY - d.startY;
      const { w, h } = computeDims(dir, totalDx, totalDy, d.startW, d.startH);

      el.style.width = `${w}px`;
      el.style.height = `${h}px`;

      if (labelRef.current) {
        labelRef.current.textContent = `${w} × ${h}`;
      }

      onResizeRef.current?.(w, h);
    };

    const onUp = () => {
      const d = dragRef.current;
      dragRef.current = null;
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);

      if (d && el) {
        const finalRect = el.getBoundingClientRect();
        const finalW = Math.round(finalRect.width);
        const finalH = Math.round(finalRect.height);
        onDisplayDimsChange(finalW, finalH);
        onResizeEndRef.current?.(finalW, finalH);
      }
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };

  return (
    <>
      {ALL_HANDLES.map((dir) => {
        const { pos, cursor } = HANDLE_STYLE_MAP[dir];
        return (
          <div
            key={dir}
            role="separator"
            tabIndex={0}
            aria-label={`Resize ${dir}`}
            className={styles.handleOuter}
            style={{
              width: HANDLE_HIT,
              height: HANDLE_HIT,
              cursor,
              ...pos,
            }}
            onMouseDown={handleMouseDown(dir)}
          >
            <div className={styles.handleInner} />
          </div>
        );
      })}
      {/* Outline */}
      <div className={styles.outline} />
    </>
  );
}
