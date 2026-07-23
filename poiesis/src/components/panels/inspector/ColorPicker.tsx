// components/panels/inspector/ColorPicker.tsx
// HUD color picker with saturation/value plane, hue strip, and hex input.

import { useState, useRef, useEffect, useCallback } from "react";
import styles from "./ColorPicker.module.css";

// ── Color conversion utilities (exported for use by TokenEditor) ──

function hslToHex(h: number, s: number, l: number): string {
  s /= 100;
  l /= 100;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
    return Math.round(255 * color)
      .toString(16)
      .padStart(2, "0");
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const clean = hex.replace("#", "");
  let r = 0;
  let g = 0;
  let b = 0;
  if (clean.length === 3) {
    r = parseInt(clean[0] + clean[0], 16);
    g = parseInt(clean[1] + clean[1], 16);
    b = parseInt(clean[2] + clean[2], 16);
  } else if (clean.length === 6) {
    r = parseInt(clean.substring(0, 2), 16);
    g = parseInt(clean.substring(2, 4), 16);
    b = parseInt(clean.substring(4, 6), 16);
  }
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
        break;
      case g:
        h = ((b - r) / d + 2) / 6;
        break;
      case b:
        h = ((r - g) / d + 4) / 6;
        break;
    }
  }
  return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) };
}

function hexToHsv(hex: string): { h: number; s: number; v: number } {
  const clean = hex.replace("#", "");
  let r = 0;
  let g = 0;
  let b = 0;
  if (clean.length === 3) {
    r = parseInt(clean[0] + clean[0], 16);
    g = parseInt(clean[1] + clean[1], 16);
    b = parseInt(clean[2] + clean[2], 16);
  } else if (clean.length === 6) {
    r = parseInt(clean.substring(0, 2), 16);
    g = parseInt(clean.substring(2, 4), 16);
    b = parseInt(clean.substring(4, 6), 16);
  }
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  const s = max === 0 ? 0 : d / max;
  const v = max;
  if (max !== min) {
    switch (max) {
      case r:
        h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
        break;
      case g:
        h = ((b - r) / d + 2) / 6;
        break;
      case b:
        h = ((r - g) / d + 4) / 6;
        break;
    }
  }
  return { h: Math.round(h * 360), s: Math.round(s * 100), v: Math.round(v * 100) };
}

function hsvToHex(h: number, s: number, v: number): string {
  const { s: ss, l } = hsvToHsl(h, s, v);
  return hslToHex(h, ss, l);
}

function hsvToHsl(h: number, s: number, v: number): { h: number; s: number; l: number } {
  const ss = s / 100;
  const vv = v / 100;
  const l = vv * (1 - ss / 2);
  const d = Math.min(l, 1 - l);
  const sl = d === 0 ? 0 : (vv - l) / d;
  return { h, s: Math.round(sl * 100), l: Math.round(l * 100) };
}

function isValidHex(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

// ── Component ──

interface ColorPickerProps {
  color: string;
  onChange: (color: string) => void;
  onClose: () => void;
  anchorRect: DOMRect;
}

export function ColorPicker({ color: initialColor, onChange, onClose, anchorRect }: ColorPickerProps) {
  const validColor = isValidHex(initialColor) ? initialColor : "#6366f1";
  const { h: initH, s: initS, v: initV } = hexToHsv(validColor);

  // Mutable refs so mousemove handler never closes over stale state
  const stateRef = useRef({ hue: initH, s: initS, v: initV });
  // Display state to trigger re-render
  const [displayHue, setDisplayHue] = useState(initH);
  const [displaySv, setDisplaySv] = useState({ s: initS, v: initV });
  const [hexInput, setHexInput] = useState(validColor);

  const svElRef = useRef<HTMLDivElement>(null);
  const hueElRef = useRef<HTMLDivElement>(null);
  const pickerElRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<"sv" | "hue" | null>(null);
  const onChangeRef = useRef(onChange);
  const onCloseRef = useRef(onClose);
  useEffect(() => { onChangeRef.current = onChange; });
  useEffect(() => { onCloseRef.current = onClose; });

  const emitColor = useCallback((h: number, s: number, v: number) => {
    const hex = hsvToHex(h, s, v);
    setHexInput(hex);
    onChangeRef.current(hex);
  }, []);

  // Global mouse handlers for dragging on SV plane and hue strip
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (dragRef.current === "sv" && svElRef.current) {
        const rect = svElRef.current.getBoundingClientRect();
        const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
        const y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
        const newS = Math.round(x * 100);
        const newV = Math.round((1 - y) * 100);
        stateRef.current = { ...stateRef.current, s: newS, v: newV };
        setDisplaySv({ s: newS, v: newV });
        emitColor(stateRef.current.hue, newS, newV);
      }
      if (dragRef.current === "hue" && hueElRef.current) {
        const rect = hueElRef.current.getBoundingClientRect();
        const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
        const newH = Math.round(x * 360);
        stateRef.current = { ...stateRef.current, hue: newH };
        setDisplayHue(newH);
        emitColor(newH, stateRef.current.s, stateRef.current.v);
      }
    };
    const handleMouseUp = () => {
      dragRef.current = null;
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [emitColor]);

  // Outside click to close
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (pickerElRef.current && !pickerElRef.current.contains(e.target as Node)) {
        onCloseRef.current();
      }
    };
    // Defer to avoid the mousedown that opened the picker from immediately closing it
    const timer = setTimeout(() => {
      document.addEventListener("mousedown", handleClick);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handleClick);
    };
  }, []);

  const handleHexChange = useCallback((value: string) => {
    setHexInput(value);
    if (isValidHex(value)) {
      const { h, s, v } = hexToHsv(value);
      stateRef.current = { hue: h, s, v };
      setDisplayHue(h);
      setDisplaySv({ s, v });
      onChange(value);
    }
  }, [onChange]);

  const handleHexBlur = useCallback(() => {
    if (!isValidHex(hexInput)) {
      // Reset to last valid color
      const { h, s, v } = hexToHsv(validColor);
      setDisplayHue(h);
      setDisplaySv({ s, v });
      setHexInput(validColor);
    }
  }, [hexInput, validColor]);

  // Compute marker positions
  const currentColor = hsvToHex(displayHue, displaySv.s, displaySv.v);
  const svX = `${displaySv.s}%`;
  const svY = `${100 - displaySv.v}%`;
  const hueMarkerX = `${(displayHue / 360) * 100}%`;

  // Position: below the anchor, clamped to viewport
  const PICKER_WIDTH = 220;
  const PADDING = 12;
  const contentWidth = PICKER_WIDTH - PADDING * 2;
  const left = Math.min(anchorRect.left, window.innerWidth - PICKER_WIDTH - 8);

  return (
    <div
      ref={pickerElRef}
      className={styles.picker}
      style={{
        left,
        top: anchorRect.bottom + 4,
        padding: PADDING,
        width: PICKER_WIDTH,
      }}
    >
      {/* Saturation / Value plane */}
      <div
        ref={svElRef}
        role="button"
        tabIndex={0}
        aria-label="Color saturation and brightness"
        onMouseDown={(e) => {
          dragRef.current = "sv";
          // Also apply the click position immediately
          const rect = svElRef.current!.getBoundingClientRect();
          const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
          const y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
          const newS = Math.round(x * 100);
          const newV = Math.round((1 - y) * 100);
          stateRef.current = { ...stateRef.current, s: newS, v: newV };
          setDisplaySv({ s: newS, v: newV });
          emitColor(stateRef.current.hue, newS, newV);
        }}
        style={{
          width: contentWidth,
          height: 150,
          borderRadius: 8,
          position: "relative",
          cursor: "crosshair",
          background: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${displayHue}, 100%, 50%))`,
          marginBottom: 8,
        }}
      >
        <div
          className={styles.svMarker}
          style={{
            left: svX,
            top: svY,
          }}
        />
      </div>

      {/* Hue strip (horizontal) */}
      <div
        ref={hueElRef}
        role="button"
        tabIndex={0}
        aria-label="Hue"
        onMouseDown={(e) => {
          dragRef.current = "hue";
          const rect = hueElRef.current!.getBoundingClientRect();
          const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
          const newH = Math.round(x * 360);
          stateRef.current = { ...stateRef.current, hue: newH };
          setDisplayHue(newH);
          emitColor(newH, stateRef.current.s, stateRef.current.v);
        }}
        style={{
          width: contentWidth,
          height: 14,
          borderRadius: 7,
          position: "relative",
          cursor: "crosshair",
          background:
            "linear-gradient(to right, #f00 0%, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00 100%)",
          marginBottom: 8,
        }}
      >
        <div
          className={styles.hueMarker}
          style={{
            left: hueMarkerX,
          }}
        />
      </div>

      {/* Preview + hex input */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <div
          style={{
            width: 28,
            height: 28,
            borderRadius: "50%",
            background: currentColor,
            border: "2px solid #334155",
            flexShrink: 0,
          }}
        />
        <input
          aria-label="Hex color"
          value={hexInput}
          onChange={(e) => handleHexChange(e.target.value)}
          onBlur={handleHexBlur}
          onKeyDown={(e) => {
            if (e.key === "Enter") onCloseRef.current();
            if (e.key === "Escape") onCloseRef.current();
          }}
          className={styles.hexInput}
          placeholder="#000000"
          spellCheck={false}
        />
      </div>
    </div>
  );
}
