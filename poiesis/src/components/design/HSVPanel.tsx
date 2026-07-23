// poiesis/src/components/design/HSVPanel.tsx
// Standalone HSV color picker panel with SV plane, hue strip, opacity strip, and hex input.

import React, { useCallback, useEffect, useRef, useState } from "react";
import { hsvToHex, hexToHsv } from "./hsv-utils";
import type { HSVColor } from "./hsv-utils";
import styles from "./HSVPanel.module.css";

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
    <div className={styles.hsvPanel} style={{ width: SV_SIZE }}>
      {/* SV Plane */}
      <div
        role="button"
        tabIndex={0}
        aria-label="Color saturation and brightness"
        onMouseDown={onSvMouseDown}
        className={styles.svPlane}
        style={{
          width: SV_SIZE,
          height: SV_SIZE,
          backgroundColor: `hsl(${color.h}, 100%, 50%)`,
          backgroundImage:
            "linear-gradient(to right, #fff, transparent), linear-gradient(to bottom, transparent, #000)",
          backgroundBlendMode: "multiply",
        }}
      >
        <div
          className={styles.crosshair}
          style={{ left: crosshairX - 6, top: crosshairY - 6 }}
        />
      </div>

      {/* Hue Strip */}
      <div
        role="button"
        tabIndex={0}
        aria-label="Hue"
        onMouseDown={onHueMouseDown}
        className={styles.hueStrip}
        style={{
          width: SV_SIZE,
          height: STRIP_HEIGHT,
          background:
            "linear-gradient(to right, #f00 0%, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00 100%)",
        }}
      >
        <div
          className={styles.hueSlider}
          style={{ left: hueX - 5, height: STRIP_HEIGHT + 2 }}
        />
      </div>

      {/* Opacity Strip */}
      {showAlpha && (
        <div
          role="button"
          tabIndex={0}
          aria-label="Opacity"
          onMouseDown={onAlphaMouseDown}
          className={styles.alphaStrip}
          style={{
            width: SV_SIZE,
            height: STRIP_HEIGHT,
            backgroundImage: [
              `linear-gradient(to right, transparent, ${currentHex})`,
              "repeating-conic-gradient(#ccc 0% 25%, #fff 0% 50%)",
            ].join(","),
          }}
        >
          <div
            className={styles.alphaSlider}
            style={{ left: alphaX - 5, height: STRIP_HEIGHT + 2 }}
          />
        </div>
      )}

      {/* Hex Input */}
      {showHexInput && (
        <div className={styles.hexRow}>
          <div
            className={styles.colorSwatch}
            style={{ backgroundColor: currentHex }}
          />
          <input
            aria-label="Hex color"
            value={hexInput}
            onChange={(e) => setHexInput(e.target.value)}
            onBlur={handleHexSubmit}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleHexSubmit();
            }}
            className={styles.hexInput}
          />
          <span className={styles.opacityLabel}>
            {Math.round(color.a * 100)}%
          </span>
        </div>
      )}
    </div>
  );
}
