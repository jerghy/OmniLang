// poiesis/src/components/design/HSVPanel.tsx
// Standalone HSV color picker panel with SV plane, hue strip, opacity strip, and hex input.

import React, { useCallback, useEffect, useRef, useState } from "react";

// ── Types ──────────────────────────────────────────────────────
export type HSVColor = {
  h: number; // 0–360
  s: number; // 0–100
  v: number; // 0–100
  a: number; // 0–1
};

// ── Color conversion helpers ───────────────────────────────────
export function hsvToHex(h: number, s: number, v: number): string {
  const ss = Math.max(0, Math.min(100, s)) / 100;
  const vv = Math.max(0, Math.min(100, v)) / 100;
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return vv - vv * ss * Math.max(Math.min(k, 4 - k, 1), 0);
  };
  const r = Math.round(f(5) * 255);
  const g = Math.round(f(3) * 255);
  const b = Math.round(f(1) * 255);
  return "#" + [r, g, b].map((c) => c.toString(16).padStart(2, "0")).join("");
}

export function hexToHsv(hex: string): { h: number; s: number; v: number } {
  const clean = hex.replace("#", "");
  let r = 0;
  let g = 0;
  let b = 0;
  if (clean.length === 3) {
    r = parseInt(clean[0] + clean[0], 16);
    g = parseInt(clean[1] + clean[1], 16);
    b = parseInt(clean[2] + clean[2], 16);
  } else if (clean.length === 6) {
    r = parseInt(clean.slice(0, 2), 16);
    g = parseInt(clean.slice(2, 4), 16);
    b = parseInt(clean.slice(4, 6), 16);
  }
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) * 60;
    else if (max === gn) h = ((bn - rn) / d + 2) * 60;
    else h = ((rn - gn) / d + 4) * 60;
  }
  const s = max === 0 ? 0 : (d / max) * 100;
  const v = max * 100;
  return { h: Math.round(h), s: Math.round(s), v: Math.round(v) };
}

// ── Drag hook ──────────────────────────────────────────────────
function useDrag(
  onMove: (x: number, y: number, e: MouseEvent) => void,
  onEnd?: (e: MouseEvent) => void
): (e: React.MouseEvent) => void {
  const onMoveRef = useRef(onMove);
  const onEndRef = useRef(onEnd);
  useEffect(() => { onMoveRef.current = onMove; });
  useEffect(() => { onEndRef.current = onEnd; });

  return useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const rect = e.currentTarget.getBoundingClientRect();

    const handleMove = (me: MouseEvent) => {
      const x = Math.max(0, Math.min(rect.width, me.clientX - rect.left));
      const y = Math.max(0, Math.min(rect.height, me.clientY - rect.top));
      onMoveRef.current(x, y, me);
    };

    const handleUp = (me: MouseEvent) => {
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
      onEndRef.current?.(me);
    };

    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
  }, []);
}

// ── HSVPanel ───────────────────────────────────────────────────
export interface HSVPanelProps {
  color: HSVColor;
  onChange: (c: HSVColor) => void;
  onChangeEnd?: (c: HSVColor) => void;
  showAlpha?: boolean;
  showHexInput?: boolean;
}

const SV_SIZE = 160;
const STRIP_HEIGHT = 14;

export function HSVPanel({
  color,
  onChange,
  onChangeEnd,
  showAlpha,
  showHexInput,
}: HSVPanelProps) {
  const [hexInput, setHexInput] = useState(() =>
    hsvToHex(color.h, color.s, color.v)
  );

  // Keep hex input in sync with external color changes.
  useEffect(() => {
    setHexInput(hsvToHex(color.h, color.s, color.v));
  }, [color.h, color.s, color.v]);

  // ── SV Plane ──────────────────────────────────────────────────
  const crosshairX = (color.s / 100) * SV_SIZE;
  const crosshairY = (1 - color.v / 100) * SV_SIZE;

  const handleSvMove = useCallback(
    (_x: number, _y: number) => {
      const s = Math.round((_x / SV_SIZE) * 100);
      const v = Math.round((1 - _y / SV_SIZE) * 100);
      onChange({
        h: color.h,
        s: Math.max(0, Math.min(100, s)),
        v: Math.max(0, Math.min(100, v)),
        a: color.a,
      });
    },
    [onChange, color.h, color.a]
  );

  const handleSvEnd = useCallback(() => {
    onChangeEnd?.({
      h: color.h,
      s: color.s,
      v: color.v,
      a: color.a,
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onChangeEnd, color]);

  const onSvMouseDown = useDrag(handleSvMove, handleSvEnd);

  // ── Hue Strip ─────────────────────────────────────────────────
  const hueX = (color.h / 360) * SV_SIZE;

  const handleHueMove = useCallback(
    (_x: number) => {
      let h = Math.round((_x / SV_SIZE) * 360);
      if (h >= 360) h = 0;
      onChange({ ...color, h });
    },
    [onChange, color]
  );

  const handleHueEnd = useCallback(() => {
    onChangeEnd?.({ ...color });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onChangeEnd, color]);

  const onHueMouseDown = useDrag(handleHueMove, handleHueEnd);

  // ── Opacity Strip ────────────────────────────────────────────
  const alphaX = color.a * SV_SIZE;

  const handleAlphaMove = useCallback(
    (_x: number) => {
      const a = Math.round((_x / SV_SIZE) * 100) / 100;
      onChange({ ...color, a: Math.max(0, Math.min(1, a)) });
    },
    [onChange, color]
  );

  const handleAlphaEnd = useCallback(() => {
    onChangeEnd?.({ ...color });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onChangeEnd, color]);

  const onAlphaMouseDown = useDrag(handleAlphaMove, handleAlphaEnd);

  // ── Hex Input ─────────────────────────────────────────────────
  const handleHexSubmit = useCallback(() => {
    const clean = hexInput.replace("#", "");
    if (/^[0-9a-fA-F]{6}$/.test(clean)) {
      const hsv = hexToHsv(clean);
      onChange({ h: hsv.h, s: hsv.s, v: hsv.v, a: color.a });
    } else {
      setHexInput(hsvToHex(color.h, color.s, color.v));
    }
  }, [hexInput, color.a, onChange, color.h, color.s, color.v]);

  const currentHex = hsvToHex(color.h, color.s, color.v);

  // ── Render ────────────────────────────────────────────────────
  return (
    <div
      style={
        {
          display: "flex",
          flexDirection: "column",
          gap: 8,
          width: SV_SIZE,
          fontFamily: "monospace",
          fontSize: 12,
          userSelect: "none",
        } as React.CSSProperties
      }
    >
      {/* SV Plane */}
      <div
        role="button"
        tabIndex={0}
        aria-label="Color saturation and brightness"
        onMouseDown={onSvMouseDown}
        style={
          {
            width: SV_SIZE,
            height: SV_SIZE,
            backgroundColor: `hsl(${color.h}, 100%, 50%)`,
            backgroundImage:
              "linear-gradient(to right, #fff, transparent), linear-gradient(to bottom, transparent, #000)",
            backgroundBlendMode: "multiply",
            borderRadius: 4,
            position: "relative",
            cursor: "crosshair",
          } as React.CSSProperties
        }
      >
        <div
          style={
            {
              position: "absolute",
              left: crosshairX - 6,
              top: crosshairY - 6,
              width: 12,
              height: 12,
              borderRadius: "50%",
              border: "2px solid #fff",
              boxShadow: "0 0 0 1px rgba(0,0,0,0.3), inset 0 0 0 1px rgba(0,0,0,0.3)",
              pointerEvents: "none",
              boxSizing: "border-box",
            } as React.CSSProperties
          }
        />
      </div>

      {/* Hue Strip */}
      <div
        role="button"
        tabIndex={0}
        aria-label="Hue"
        onMouseDown={onHueMouseDown}
        style={
          {
            width: SV_SIZE,
            height: STRIP_HEIGHT,
            background:
              "linear-gradient(to right, #f00 0%, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00 100%)",
            borderRadius: 4,
            position: "relative",
            cursor: "pointer",
          } as React.CSSProperties
        }
      >
        <div
          style={
            {
              position: "absolute",
              left: hueX - 5,
              top: -1,
              width: 10,
              height: STRIP_HEIGHT + 2,
              border: "2px solid #fff",
              borderRadius: 3,
              boxShadow: "0 0 0 1px rgba(0,0,0,0.3)",
              pointerEvents: "none",
              boxSizing: "border-box",
            } as React.CSSProperties
          }
        />
      </div>

      {/* Opacity Strip */}
      {showAlpha && (
        <div
          role="button"
          tabIndex={0}
          aria-label="Opacity"
          onMouseDown={onAlphaMouseDown}
          style={
            {
              width: SV_SIZE,
              height: STRIP_HEIGHT,
              borderRadius: 4,
              position: "relative",
              cursor: "pointer",
              backgroundImage: [
                `linear-gradient(to right, transparent, ${currentHex})`,
                "repeating-conic-gradient(#ccc 0% 25%, #fff 0% 50%)",
              ].join(","),
              backgroundSize: "100% 100%, 10px 10px",
              backgroundPosition: "0 0, 0 0",
            } as React.CSSProperties
          }
        >
          <div
            style={
              {
                position: "absolute",
                left: alphaX - 5,
                top: -1,
                width: 10,
                height: STRIP_HEIGHT + 2,
                border: "2px solid #fff",
                borderRadius: 3,
                boxShadow: "0 0 0 1px rgba(0,0,0,0.3)",
                pointerEvents: "none",
                boxSizing: "border-box",
              } as React.CSSProperties
            }
          />
        </div>
      )}

      {/* Hex Input */}
      {showHexInput && (
        <div
          style={
            {
              display: "flex",
              alignItems: "center",
              gap: 6,
            } as React.CSSProperties
          }
        >
          <div
            style={
              {
                width: 24,
                height: 24,
                borderRadius: 4,
                backgroundColor: currentHex,
                boxShadow: "inset 0 1px 2px rgba(0,0,0,0.2)",
                flexShrink: 0,
              } as React.CSSProperties
            }
          />
          <input
            aria-label="Hex color"
            value={hexInput}
            onChange={(e) => setHexInput(e.target.value)}
            onBlur={handleHexSubmit}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleHexSubmit();
            }}
            style={
              {
                width: 72,
                padding: "2px 4px",
                border: "1px solid #ccc",
                borderRadius: 3,
                fontFamily: "monospace",
                fontSize: 12,
                outline: "2px solid transparent",
              } as React.CSSProperties
            }
          />
          <span style={{ color: "#666" } as React.CSSProperties}>
            {Math.round(color.a * 100)}%
          </span>
        </div>
      )}
    </div>
  );
}
