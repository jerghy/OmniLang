// poiesis/src/components/canvas/code/blocks/SelectionOverlay.tsx
// Task 4: Selection overlay that renders a visual cursor on the selected target
// (block, slot, or gap) using absolute positioning over the Blockly workspace.

import React, { useEffect, useRef } from "react";
import type { SelectionTarget } from "./BlockModel";

interface SelectionOverlayProps {
  target: SelectionTarget | null;
  /** Maps a SelectionTarget to a DOM element for positioning. */
  getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null;
}

const BLINK_KEYFRAMES = `
@keyframes sel-gap-blink { 50% { opacity: 0; } }
`;

function getTargetRect(
  target: SelectionTarget,
  getElement: (t: SelectionTarget) => HTMLElement | SVGElement | null,
): DOMRect | null {
  const el = getElement(target);
  if (!el) return null;

  // SVG <g> elements have no intrinsic bounding box — use the first
  // visible child rect instead.
  if (el instanceof SVGGElement && el.children.length > 0) {
    try {
      const bbox = (el as SVGGElement).getBBox();
      const ctm = (el as SVGGElement).getScreenCTM();
      if (ctm && bbox.width > 0 && bbox.height > 0) {
        const p = (el as SVGGElement).ownerSVGElement!.createSVGPoint();
        p.x = bbox.x; p.y = bbox.y;
        const screenP = p.matrixTransform(ctm);
        return new DOMRect(screenP.x, screenP.y, bbox.width * ctm.a, bbox.height * ctm.d);
      }
    } catch {
      // fall through to getBoundingClientRect
    }
  }

  const rect = el.getBoundingClientRect();
  if (rect.width > 0 && rect.height > 0) return rect;
  return null;
}

export const SelectionOverlay: React.FC<SelectionOverlayProps> = ({
  target,
  getElement,
}) => {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = overlayRef.current;
    if (!el) return;

    if (!target) {
      el.style.display = "none";
      return;
    }

    const rect = getTargetRect(target, getElement);
    if (!rect || rect.width === 0 || rect.height === 0) {
      el.style.display = "none";
      return;
    }

    el.style.display = "block";
    el.style.left = `${rect.x}px`;
    el.style.top = `${rect.y}px`;
    el.style.width = `${rect.width}px`;
    el.style.height = `${rect.height}px`;

    switch (target.type) {
      case "block":
        el.style.outline = "3px solid #cc6b4a";
        el.style.outlineOffset = "3px";
        el.style.border = "none";
        el.style.borderRadius = "6px";
        el.style.background = "rgba(204, 107, 74, 0.1)";
        el.style.boxShadow = "0 0 0 6px rgba(204, 107, 74, 0.15)";
        el.style.animation = "none";
        break;
      case "slot":
        el.style.outline = "none";
        el.style.border = "3px dashed #cc6b4a";
        el.style.borderRadius = "4px";
        el.style.background = "rgba(204, 107, 74, 0.08)";
        el.style.boxShadow = "none";
        el.style.animation = "none";
        break;
      case "gap":
        el.style.outline = "none";
        el.style.border = "none";
        el.style.background = "rgba(204, 107, 74, 0.18)";
        el.style.borderRadius = "4px";
        el.style.boxShadow = "none";
        el.style.animation = "sel-gap-blink 1.2s step-end infinite";
        break;
    }
  }, [target, getElement]);

  return (
    <>
      <style>{BLINK_KEYFRAMES}</style>
      <div
        ref={overlayRef}
        style={{
          position: "fixed",
          pointerEvents: "none",
          zIndex: 1000,
          transition: "left 150ms ease, top 150ms ease, width 150ms ease, height 150ms ease",
        }}
      />
    </>
  );
};
