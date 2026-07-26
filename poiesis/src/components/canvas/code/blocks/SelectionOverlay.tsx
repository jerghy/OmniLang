// poiesis/src/components/canvas/code/blocks/SelectionOverlay.tsx
// Task 4: Selection overlay that renders a visual cursor on the selected target
// (block, slot, or gap) using absolute positioning over the Blockly workspace.

import React, { useEffect, useRef } from "react";
import type { SelectionTarget } from "./BlockModel";

interface SelectionOverlayProps {
  target: SelectionTarget | null;
  /** Map from target key to DOM element for positioning. */
  getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null;
}

const BLINK_KEYFRAMES = `
@keyframes sel-gap-blink { 50% { opacity: 0; } }
`;

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

    const domEl = getElement(target);
    if (!domEl) {
      el.style.display = "none";
      return;
    }

    const rect = domEl.getBoundingClientRect();
    el.style.display = "block";
    el.style.left = `${rect.x}px`;
    el.style.top = `${rect.y}px`;
    el.style.width = `${rect.width}px`;
    el.style.height = `${rect.height}px`;

    switch (target.type) {
      case "block":
        el.style.outline = "2px solid #cc6b4a";
        el.style.outlineOffset = "2px";
        el.style.border = "none";
        el.style.borderRadius = "4px";
        el.style.animation = "none";
        break;
      case "slot":
        el.style.outline = "none";
        el.style.border = "2px dashed #cc6b4a";
        el.style.borderRadius = "4px";
        el.style.animation = "none";
        break;
      case "gap":
        el.style.outline = "none";
        el.style.border = "none";
        el.style.borderTop = "2px solid #cc6b4a";
        el.style.animation = "sel-gap-blink 1s step-end infinite";
        break;
    }
  }, [target, getElement]);

  return (
    <>
      <style>{BLINK_KEYFRAMES}</style>
      <div
        ref={overlayRef}
        style={{
          position: "absolute",
          pointerEvents: "none",
          zIndex: 1000,
          transition: "all 150ms ease",
        }}
      />
    </>
  );
};
