// poiesis/src/components/design/ColorHUD.tsx
// Floating popover color picker anchored to a trigger element.

import React, { useCallback, useEffect, useRef, useState } from "react";
import type { DesignToken } from "../../core/types/design";
import { HSVPanel, hsvToHex, hexToHsv } from "./HSVPanel";
import type { HSVColor } from "./HSVPanel";

// ── ColorHUD ───────────────────────────────────────────────────
export interface ColorHUDProps {
  hex: string;
  tokens: DesignToken[];
  onChange: (hex: string, tokenName: string | null) => void;
  onClose: () => void;
  anchorRect: DOMRect;
}

const PANEL_WIDTH = 168; // SV_SIZE (160) + padding on each side
const PANEL_HEIGHT_EST = 240;

export function ColorHUD({
  hex,
  tokens,
  onChange,
  onClose,
  anchorRect,
}: ColorHUDProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // ── Click-outside detection ───────────────────────────────────
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (
        panelRef.current &&
        !panelRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [onClose]);

  // ── Position ──────────────────────────────────────────────────
  let top = anchorRect.bottom + 4;
  let left = anchorRect.left;

  // Clamp to viewport bounds
  if (left + PANEL_WIDTH > window.innerWidth - 8) {
    left = window.innerWidth - PANEL_WIDTH - 8;
  }
  if (left < 8) left = 8;
  if (top + PANEL_HEIGHT_EST > window.innerHeight - 8) {
    top = anchorRect.top - PANEL_HEIGHT_EST - 4;
  }
  if (top < 8) top = 8;

  // ── Color state ───────────────────────────────────────────────
  const initialHsv = hexToHsv(hex);
  const [panelColor, setPanelColor] = useState<HSVColor>(() => ({
    ...initialHsv,
    a: 1,
  }));

  // Sync external hex changes.
  useEffect(() => {
    const hsv = hexToHsv(hex);
    setPanelColor((prev) => {
      if (prev.h === hsv.h && prev.s === hsv.s && prev.v === hsv.v)
        return prev;
      return { ...hsv, a: prev.a };
    });
  }, [hex]);

  // ── Handlers ──────────────────────────────────────────────────
  const handlePanelChange = useCallback(
    (c: HSVColor) => {
      setPanelColor(c);
      onChange(hsvToHex(c.h, c.s, c.v), null);
    },
    [onChange]
  );

  const handlePanelEnd = useCallback(
    (c: HSVColor) => {
      onChange(hsvToHex(c.h, c.s, c.v), null);
    },
    [onChange]
  );

  const colorTokens = tokens.filter((t) => t.tokenType === "color");

  const handleTokenClick = useCallback(
    (token: DesignToken) => {
      onChange(token.value, token.name);
    },
    [onChange]
  );

  // ── Render ────────────────────────────────────────────────────
  return (
    <div
      ref={panelRef}
      style={
        {
          position: "fixed",
          top,
          left,
          zIndex: 10000,
          background: "#fff",
          borderRadius: 8,
          boxShadow: "0 4px 16px rgba(0,0,0,0.2)",
          padding: 10,
          display: "flex",
          flexDirection: "column",
          gap: 8,
        } as React.CSSProperties
      }
    >
      <HSVPanel
        color={panelColor}
        onChange={handlePanelChange}
        onChangeEnd={handlePanelEnd}
        showAlpha
        showHexInput
      />

      {colorTokens.length > 0 && (
        <div
          style={
            {
              display: "flex",
              flexWrap: "wrap",
              gap: 4,
              maxWidth: PANEL_WIDTH,
            } as React.CSSProperties
          }
        >
          {colorTokens.map((token) => {
            const isSelected =
              hex.toUpperCase() === token.value.toUpperCase();
            return (
              <div
                key={token.id}
                onClick={() => handleTokenClick(token)}
                title={`${token.name} (${token.value})`}
                style={
                  {
                    width: 20,
                    height: 20,
                    borderRadius: 3,
                    backgroundColor: token.value,
                    cursor: "pointer",
                    border: isSelected
                      ? "2px solid #fff"
                      : ("1px solid rgba(0,0,0,0.15)" as string),
                    boxShadow: isSelected
                      ? "0 0 0 2px #666"
                      : "none",
                    boxSizing: "border-box",
                  } as React.CSSProperties
                }
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
