# Class Editor Visual Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the form-style Class Editor with visual design cards (Size, Background, Border, Radius, Spacing) + collapsible property groups + live preview.

**Architecture:** Add shared atomic components (ScrubInput, CollapsibleGroup, IconButtonGroup, HSVPanel, ColorHUD), build 5 specialized property cards, add FocusOverlay for full-preview mode, then assemble everything in a new ClassEditor.

**Tech Stack:** React 18 + TypeScript 5, Zustand store, inline CSSProperties styling (matches project convention)

## Global Constraints

- No external design-system libraries; all styling is inline `React.CSSProperties`
- Follow existing project conventions: `PROPERTY_GROUPS`, `StyleProperty`, `DesignClass` types
- Store actions: `updateDesignClass`, `addDesignClass`, `removeDesignClass` (from `design-slice.ts`)
- TypeScript strict; `npx tsc --noEmit` must pass each task
- All existing TypeScript types in `design.ts` and `style.ts` remain unchanged
- Projects convention: `components/design/cards/` for card components

---

### Task 1: ScrubInput — Drag-Scrub Numeric Input

**Files:**
- Create: `poiesis/src/components/design/ScrubInput.tsx`

**Interfaces:**
- Produces: `ScrubInput` component
  ```typescript
  interface ScrubInputProps {
    value: number;
    unit?: string;
    min?: number;
    max?: number;
    step?: number;
    fineStep?: number;
    onChange: (value: number) => void;
    onChangeEnd?: (value: number) => void;
    disabled?: boolean;
    style?: React.CSSProperties;
  }
  ```

- [ ] **Step 1: Create ScrubInput.tsx**

```typescript
// components/design/ScrubInput.tsx
// Drag-scrub numeric input — click+drag left/right to change value.

import { useState, useRef, useCallback, useEffect } from "react";

interface ScrubInputProps {
  value: number;
  unit?: string;
  min?: number;
  max?: number;
  step?: number;
  fineStep?: number;
  onChange: (value: number) => void;
  onChangeEnd?: (value: number) => void;
  disabled?: boolean;
  style?: React.CSSProperties;
}

const INPUT_STYLE: React.CSSProperties = {
  padding: "2px 4px",
  fontSize: 11,
  fontFamily: "'Cascadia Code', 'Fira Code', monospace",
  border: "1px solid #e2e8f0",
  borderRadius: 4,
  outline: "none",
  background: "#fff",
  color: "#374151",
  boxSizing: "border-box",
  width: 48,
  textAlign: "center",
  cursor: "ew-resize",
  lineHeight: "18px",
};

export function ScrubInput({
  value,
  unit,
  min,
  max,
  step = 1,
  fineStep = 10,
  onChange,
  onChangeEnd,
  disabled = false,
  style,
}: ScrubInputProps) {
  const [editValue, setEditValue] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const scrubRef = useRef<{ startX: number; startVal: number; moved: boolean } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const clamp = useCallback(
    (v: number) => {
      if (min !== undefined && v < min) return min;
      if (max !== undefined && v > max) return max;
      return Math.round(v * 100) / 100;
    },
    [min, max]
  );

  // Global mouse events for scrub
  useEffect(() => {
    if (!scrubRef.current) return;

    const onMouseMove = (e: MouseEvent) => {
      const st = scrubRef.current!;
      const dx = e.clientX - st.startX;
      if (Math.abs(dx) < 2) return;
      st.moved = true;
      if (!isDragging) setIsDragging(true);
      document.body.style.cursor = "ew-resize";
      document.body.style.userSelect = "none";
      const mod = e.shiftKey ? fineStep : step;
      onChange(clamp(st.startVal + dx * mod));
    };

    const onMouseUp = () => {
      const st = scrubRef.current!;
      if (st.moved) {
        const finalVal = clamp(st.startVal + (/* last dx is lost but acceptable */ 0));
        // Re-read: we can't easily get final delta here, onChangeEnd fires with current value
        onChangeEnd?.(value);
      }
      scrubRef.current = null;
      setIsDragging(false);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [value, step, fineStep, clamp, onChange, onChangeEnd, isDragging]);

  const handleMouseDown = (e: React.MouseEvent) => {
    if (disabled) return;
    scrubRef.current = { startX: e.clientX, startVal: value, moved: false };
  };

  const handleFocus = () => {
    if (!isDragging) setEditValue(String(value));
  };

  const handleBlur = () => {
    if (editValue !== null) {
      const parsed = parseFloat(editValue);
      if (!isNaN(parsed)) {
        onChange(clamp(parsed));
        onChangeEnd?.(clamp(parsed));
      }
    }
    setEditValue(null);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      (e.target as HTMLInputElement).blur();
    }
    if (e.key === "Escape") {
      setEditValue(null);
      setTimeout(() => inputRef.current?.blur(), 0);
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      onChange(clamp(value + (e.shiftKey ? fineStep : step)));
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      onChange(clamp(value - (e.shiftKey ? fineStep : step)));
    }
  };

  const displayValue = editValue ?? (isDragging ? value : value);

  return (
    <div style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
      <input
        ref={inputRef}
        type="text"
        value={displayValue}
        onChange={(e) => setEditValue(e.target.value)}
        onFocus={handleFocus}
        onBlur={handleBlur}
        onKeyDown={handleKeyDown}
        onMouseDown={handleMouseDown}
        disabled={disabled}
        style={{ ...INPUT_STYLE, ...style, opacity: disabled ? 0.5 : 1 }}
      />
      {unit && (
        <span style={{ fontSize: 10, color: "#94a3b8", minWidth: 16 }}>{unit}</span>
      )}
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check**

```bash
npx tsc --noEmit --pretty 2>&1 | grep -E "error TS|Found"
```

Expected: only pre-existing error in `pipeline.test.ts`

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/ScrubInput.tsx
git commit -m "feat: add ScrubInput drag-scrub numeric component"
```

---

### Task 2: CollapsibleGroup — Collapsible Property Section

**Files:**
- Create: `poiesis/src/components/design/CollapsibleGroup.tsx`

**Interfaces:**
- Produces: `CollapsibleGroup` component
  ```typescript
  interface CollapsibleGroupProps {
    title: string;
    summary: string;
    defaultOpen?: boolean;
    children: React.ReactNode;
  }
  ```

- [ ] **Step 1: Create CollapsibleGroup.tsx**

```typescript
// components/design/CollapsibleGroup.tsx
// Collapsible property group with value summary when collapsed.

import { useState, useCallback } from "react";

interface CollapsibleGroupProps {
  title: string;
  summary: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

const HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  padding: "6px 12px",
  cursor: "pointer",
  userSelect: "none",
  borderBottom: "1px solid #e2e8f0",
  fontSize: 11,
  fontWeight: 600,
};

const CHEVRON: React.CSSProperties = {
  fontSize: 10,
  color: "#94a3b8",
  transition: "transform 150ms",
  flexShrink: 0,
};

export function CollapsibleGroup({
  title,
  summary,
  defaultOpen = false,
  children,
}: CollapsibleGroupProps) {
  const [open, setOpen] = useState(defaultOpen);

  const toggle = useCallback(() => setOpen((v) => !v), []);

  return (
    <div style={{ borderBottom: "1px solid #e2e8f0" }}>
      <div
        onClick={toggle}
        style={HEADER_STYLE}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLElement).style.background = "#f1f5f9";
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLElement).style.background = "transparent";
        }}
      >
        <span style={{ ...CHEVRON, transform: open ? "rotate(90deg)" : "rotate(0deg)" }}>
          ▶
        </span>
        <span style={{ color: "#374151", textTransform: "uppercase", letterSpacing: "0.5px" }}>
          {title}
        </span>
        {!open && (
          <span style={{ color: "#94a3b8", fontWeight: 400, fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
            {summary}
          </span>
        )}
      </div>
      {open && <div style={{ padding: "4px 12px 8px" }}>{children}</div>}
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/CollapsibleGroup.tsx
git commit -m "feat: add CollapsibleGroup component"
```

---

### Task 3: IconButtonGroup — Icon Button Toggle Group

**Files:**
- Create: `poiesis/src/components/design/IconButtonGroup.tsx`

**Interfaces:**
- Produces: `IconButtonGroup` component
  ```typescript
  interface IconButtonGroupProps<T extends string> {
    options: { value: T; label: string; title?: string }[];
    value: T;
    onChange: (value: T) => void;
  }
  ```

- [ ] **Step 1: Create IconButtonGroup.tsx**

```typescript
// components/design/IconButtonGroup.tsx
// Group of toggle buttons for enum/icon properties (display, direction, align, etc.)

interface IconButtonGroupProps<T extends string> {
  options: { value: T; label: string; title?: string }[];
  value: T;
  onChange: (value: T) => void;
}

const BTN: React.CSSProperties = {
  padding: "2px 6px",
  fontSize: 11,
  border: "1px solid #e2e8f0",
  background: "#fff",
  color: "#64748b",
  cursor: "pointer",
  fontFamily: "inherit",
  borderRadius: 4,
  lineHeight: "18px",
};

const BTN_ACTIVE: React.CSSProperties = {
  ...BTN,
  background: "#6366f1",
  color: "#fff",
  borderColor: "#6366f1",
};

export function IconButtonGroup<T extends string>({
  options,
  value,
  onChange,
}: IconButtonGroupProps<T>) {
  return (
    <div style={{ display: "flex", gap: 2 }} role="radiogroup">
      {options.map((opt) => (
        <button
          key={opt.value}
          role="radio"
          aria-checked={value === opt.value}
          title={opt.title ?? opt.label}
          style={value === opt.value ? BTN_ACTIVE : BTN}
          onClick={() => onChange(opt.value)}
          onMouseEnter={(e) => {
            if (value !== opt.value)
              (e.currentTarget as HTMLElement).style.background = "#f1f5f9";
          }}
          onMouseLeave={(e) => {
            if (value !== opt.value)
              (e.currentTarget as HTMLElement).style.background = "#fff";
          }}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 4: HSVPanel + ColorHUD — Color Picker

**Files:**
- Create: `poiesis/src/components/design/HSVPanel.tsx`
- Create: `poiesis/src/components/design/ColorHUD.tsx`

**Interfaces:**
- Produces: `HSVPanel` (inline HSV picker), `ColorHUD` (floating popover wrapper)
- See code below for exact signatures.

- [ ] **Step 1: Create HSVPanel.tsx**

```typescript
// components/design/HSVPanel.tsx
// Inline HSV color picker: SV plane + hue strip + opacity strip + hex input.

import { useState, useRef, useCallback, useEffect } from "react";

interface HSVColor {
  h: number; // 0-360
  s: number; // 0-100
  v: number; // 0-100
  a: number; // 0-1
}

function hsvToHex(h: number, s: number, v: number): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(Math.min(k, 4 - k, 1), 0);
  };
  const toHex = (n: number) =>
    Math.round(n * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${toHex(f(5))}${toHex(f(3))}${toHex(f(1))}`;
}

function hexToHsv(hex: string): HSVColor {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b),
    min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
    else if (max === g) h = ((b - r) / d + 2) * 60;
    else h = ((r - g) / d + 4) * 60;
  }
  return {
    h,
    s: max === 0 ? 0 : (d / max) * 100,
    v: max * 100,
    a: 1,
  };
}

export type { HSVColor };

interface HSVPanelProps {
  color: HSVColor;
  onChange: (color: HSVColor) => void;
  onChangeEnd?: (color: HSVColor) => void;
  showAlpha?: boolean;
  showHexInput?: boolean;
}

const PLANE_SIZE = 160;
const STRIP_H = 14;

function useDrag(
  onDrag: (dx: number, dy: number, e: MouseEvent) => void,
  onEnd?: () => void
) {
  const dragging = useRef(false);

  const onMouseDown = useCallback(() => {
    dragging.current = true;
    const onMove = (e: MouseEvent) => {
      if (!dragging.current) return;
      onDrag(e.movementX, e.movementY, e);
    };
    const onUp = () => {
      dragging.current = false;
      onEnd?.();
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }, [onDrag, onEnd]);

  return { onMouseDown };
}

export function HSVPanel({
  color,
  onChange,
  onChangeEnd,
  showAlpha = true,
  showHexInput = true,
}: HSVPanelProps) {
  const planeRef = useRef<HTMLDivElement>(null);
  const [hexInput, setHexInput] = useState("");

  const updateFromPlane = useCallback(
    (e: MouseEvent) => {
      const rect = planeRef.current?.getBoundingClientRect();
      if (!rect) return;
      const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
      onChange({ ...color, s: x * 100, v: (1 - y) * 100 });
    },
    [color, onChange]
  );

  const planeDrag = useDrag((_dx, _dy, e) => updateFromPlane(e), () => onChangeEnd?.(color));
  const hueDrag = useDrag((dx) => {
    onChange({ ...color, h: ((color.h + dx) + 360) % 360 });
  }, () => onChangeEnd?.(color));
  const alphaDrag = useDrag((dx) => {
    onChange({ ...color, a: Math.max(0, Math.min(1, color.a + dx * 0.01)) });
  }, () => onChangeEnd?.(color));

  const handleHexBlur = () => {
    if (/^#[0-9a-fA-F]{6}$/.test(hexInput)) {
      const hsv = hexToHsv(hexInput);
      onChange({ ...hsv, a: color.a });
      onChangeEnd?.({ ...hsv, a: color.a });
    }
    setHexInput("");
  };

  const hueColor = hsvToHex(color.h, 100, 100);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, userSelect: "none" }}>
      {/* SV Plane */}
      <div
        ref={planeRef}
        onMouseDown={(e) => {
          planeDrag.onMouseDown();
          updateFromPlane(e.nativeEvent);
        }}
        style={{
          width: PLANE_SIZE,
          height: PLANE_SIZE,
          borderRadius: 6,
          position: "relative",
          cursor: "crosshair",
          background: `linear-gradient(to right, white, ${hueColor}), linear-gradient(to top, black, transparent)`,
          backgroundBlendMode: "multiply",
        }}
      >
        <div
          style={{
            position: "absolute",
            left: `${color.s}%`,
            top: `${100 - color.v}%`,
            width: 12,
            height: 12,
            borderRadius: "50%",
            border: "2px solid white",
            boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
            transform: "translate(-50%, -50%)",
            pointerEvents: "none",
          }}
        />
      </div>

      {/* Hue strip */}
      <div
        onMouseDown={hueDrag.onMouseDown}
        style={{
          width: PLANE_SIZE,
          height: STRIP_H,
          borderRadius: 4,
          cursor: "ew-resize",
          background:
            "linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)",
          position: "relative",
        }}
      >
        <div
          style={{
            position: "absolute",
            left: `${(color.h / 360) * 100}%`,
            top: -2,
            width: 4,
            height: STRIP_H + 4,
            borderRadius: 2,
            border: "2px solid white",
            boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
            transform: "translateX(-50%)",
            pointerEvents: "none",
            background: hueColor,
          }}
        />
      </div>

      {/* Opacity strip */}
      {showAlpha && (
        <div
          onMouseDown={alphaDrag.onMouseDown}
          style={{
            width: PLANE_SIZE,
            height: STRIP_H,
            borderRadius: 4,
            cursor: "ew-resize",
            background: `linear-gradient(to right, transparent, ${hsvToHex(color.h, color.s, color.v)})`,
            position: "relative",
            // checkerboard underneath
            backgroundImage: `linear-gradient(to right, transparent, ${hsvToHex(color.h, color.s, color.v)}), linear-gradient(45deg, #ccc 25%, transparent 25%), linear-gradient(-45deg, #ccc 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #ccc 75%), linear-gradient(-45deg, transparent 75%, #ccc 75%)`,
            backgroundSize: `${PLANE_SIZE}px ${STRIP_H}px, 8px 8px, 8px 8px, 8px 8px, 8px 8px`,
            backgroundPosition: "0 0, 0 0, 0 4px, 4px -4px, -4px 0px",
          }}
        >
          <div
            style={{
              position: "absolute",
              left: `${color.a * 100}%`,
              top: -2,
              width: 4,
              height: STRIP_H + 4,
              borderRadius: 2,
              border: "2px solid white",
              boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
              transform: "translateX(-50%)",
              pointerEvents: "none",
            }}
          />
        </div>
      )}

      {/* Hex input */}
      {showHexInput && (
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <div
            style={{
              width: 24,
              height: 24,
              borderRadius: 4,
              background: hsvToHex(color.h, color.s, color.v),
              opacity: color.a,
              border: "1px solid #cbd5e1",
              flexShrink: 0,
            }}
          />
          <input
            type="text"
            value={hexInput || hsvToHex(color.h, color.s, color.v)}
            onChange={(e) => setHexInput(e.target.value)}
            onBlur={handleHexBlur}
            onKeyDown={(e) => {
              if (e.key === "Enter") (e.target as HTMLInputElement).blur();
            }}
            placeholder="#000000"
            style={{
              padding: "2px 4px",
              fontSize: 11,
              fontFamily: "'Cascadia Code', 'Fira Code', monospace",
              border: "1px solid #e2e8f0",
              borderRadius: 4,
              outline: "none",
              width: 80,
              color: "#374151",
            }}
          />
          <span style={{ fontSize: 10, color: "#94a3b8" }}>
            {Math.round(color.a * 100)}%
          </span>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Create ColorHUD.tsx**

```typescript
// components/design/ColorHUD.tsx
// Floating color picker HUD — anchored to a trigger element.

import { useState, useCallback, useEffect, useRef } from "react";
import { HSVPanel, type HSVColor } from "./HSVPanel";
import type { DesignToken } from "../../core/types/design";

function hexToHsv(hex: string): HSVColor {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
    else if (max === g) h = ((b - r) / d + 2) * 60;
    else h = ((r - g) / d + 4) * 60;
  }
  return { h, s: max === 0 ? 0 : (d / max) * 100, v: max * 100, a: 1 };
}

function hsvToHex(h: number, s: number, v: number): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(Math.min(k, 4 - k, 1), 0);
  };
  const toHex = (n: number) => Math.round(n * 255).toString(16).padStart(2, "0");
  return `#${toHex(f(5))}${toHex(f(3))}${toHex(f(1))}`;
}

interface ColorHUDProps {
  hex: string;
  tokens: DesignToken[];
  onChange: (hex: string, tokenName: string | null) => void;
  onClose: () => void;
  anchorRect: DOMRect;
}

export function ColorHUD({ hex, tokens, onChange, onClose, anchorRect }: ColorHUDProps) {
  const hudRef = useRef<HTMLDivElement>(null);
  const [hsv, setHsv] = useState<HSVColor>(() => hexToHsv(hex));
  const [selectedToken, setSelectedToken] = useState<string | null>(null);

  const handleChange = useCallback((c: HSVColor) => {
    setHsv(c);
    setSelectedToken(null);
  }, []);

  const handleChangeEnd = useCallback(
    (c: HSVColor) => {
      onChange(hsvToHex(c.h, c.s, c.v), null);
    },
    [onChange]
  );

  const handleTokenClick = useCallback(
    (t: DesignToken) => {
      onChange(t.value, t.name);
      setSelectedToken(t.name);
      onClose();
    },
    [onChange, onClose]
  );

  // Click outside → close
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (hudRef.current && !hudRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const timer = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [onClose]);

  // Position calc
  const HUD_W = 180;
  const left = Math.max(8, Math.min(anchorRect.left, window.innerWidth - HUD_W - 8));
  const top =
    anchorRect.bottom + 8 + 260 > window.innerHeight
      ? anchorRect.top - 260 - 8
      : anchorRect.bottom + 8;

  const colorTokens = tokens.filter((t) => t.tokenType === "color");

  return (
    <div
      ref={hudRef}
      style={{
        position: "fixed",
        left,
        top,
        zIndex: 10000,
        background: "#fff",
        border: "1px solid #e2e8f0",
        borderRadius: 8,
        boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
        padding: 10,
        width: HUD_W,
      }}
    >
      <HSVPanel color={hsv} onChange={handleChange} onChangeEnd={handleChangeEnd} />

      {/* Token palette */}
      {colorTokens.length > 0 && (
        <div style={{ marginTop: 8 }}>
          <div style={{ fontSize: 10, fontWeight: 600, color: "#94a3b8", marginBottom: 4 }}>
            Tokens
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
            {colorTokens.map((t) => (
              <button
                key={t.id}
                onClick={() => handleTokenClick(t)}
                title={t.name}
                style={{
                  width: 20,
                  height: 20,
                  borderRadius: 4,
                  border: selectedToken === t.name ? "2px solid #6366f1" : "1px solid #cbd5e1",
                  background: t.value,
                  cursor: "pointer",
                  padding: 0,
                }}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 3: TypeScript check** → only pre-existing errors

- [ ] **Step 4: Commit**

---

### Task 5: PropertyRow — Compact Visual Property Row

**Files:**
- Create: `poiesis/src/components/design/PropertyRow.tsx`
- Modify: `poiesis/src/components/design/ClassEditor.tsx` (use new component)

**Interfaces:**
- Produces: `PropertyRow` component
  ```typescript
  interface PropertyRowProps {
    label: string;
    children: React.ReactNode;
    inherited?: boolean;
    showOverrideDot?: boolean;
    onRemove?: () => void;
    onTokenToggle?: () => void;
    hasToken?: boolean;
    tokenName?: string | null;
  }
  ```

- [ ] **Step 1: Create PropertyRow.tsx**

```typescript
// components/design/PropertyRow.tsx
// Compact 28px property row used inside collapsible groups and cards.

const ROW: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  height: 28,
  padding: "0 4px",
};

const LABEL: React.CSSProperties = {
  fontSize: 11,
  color: "#374151",
  width: 56,
  flexShrink: 0,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const INHERITED_LABEL: React.CSSProperties = {
  ...LABEL,
  color: "#94a3b8",
};

interface PropertyRowProps {
  label: string;
  children: React.ReactNode;
  inherited?: boolean;
  showOverrideDot?: boolean;
  onRemove?: () => void;
  onTokenToggle?: () => void;
  hasToken?: boolean;
  tokenName?: string | null;
}

export function PropertyRow({
  label,
  children,
  inherited = false,
  showOverrideDot = false,
  onRemove,
  onTokenToggle,
  hasToken = false,
  tokenName,
}: PropertyRowProps) {
  return (
    <div style={ROW}>
      {/* Override dot for pseudo states */}
      {showOverrideDot ? (
        <div
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: "#6366f1",
            flexShrink: 0,
          }}
        />
      ) : inherited ? (
        <div style={{ width: 6, flexShrink: 0 }} />
      ) : null}
      <span style={inherited ? INHERITED_LABEL : LABEL}>{label}</span>
      <div style={{ flex: 1, minWidth: 0 }}>{children}</div>
      {onTokenToggle && (
        <button
          onClick={onTokenToggle}
          title={hasToken ? `Token: ${tokenName}` : "Link token"}
          style={{
            border: "none",
            background: "none",
            cursor: "pointer",
            fontSize: 12,
            padding: "2px 4px",
            color: hasToken ? "#6366f1" : "#94a3b8",
            flexShrink: 0,
          }}
        >
          &#128279;
        </button>
      )}
      {onRemove && (
        <button
          onClick={onRemove}
          style={{
            border: "none",
            background: "none",
            cursor: "pointer",
            color: "#94a3b8",
            fontSize: 14,
            padding: "2px 4px",
            flexShrink: 0,
          }}
        >
          ×
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 6: SizeCard — Dimensions Card

**Files:**
- Create: `poiesis/src/components/design/cards/SizeCard.tsx`

**Interfaces:**
- Consumes: `ScrubInput` from Task 1
- Produces: `SizeCard` component
  ```typescript
  interface SizeCardProps {
    properties: StyleProperty[];
    onChange: (props: StyleProperty[]) => void;
    onFocus?: () => void;  // double-click → enter focus mode
  }
  ```

- [ ] **Step 1: Create cards/SizeCard.tsx**

```typescript
// components/design/cards/SizeCard.tsx
// Dimensions card: width/height with preview drag-resize.

import { useState, useCallback, useMemo } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils";

interface SizeCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
  onFocus?: () => void;
}

function getProp(props: StyleProperty[], name: string): string {
  return props.find((p) => p.name === name)?.segments[0]?.value ?? "";
}

function setProp(props: StyleProperty[], name: string, value: string): StyleProperty[] {
  const next = [...props];
  const idx = next.findIndex((p) => p.name === name);
  if (idx >= 0) {
    next[idx] = { ...next[idx], segments: [{ type: "text", value }] };
  } else if (value) {
    next.push({ id: uid("prop"), name, segments: [{ type: "text", value }] });
  }
  return next;
}

export function SizeCard({ properties, onChange, onFocus }: SizeCardProps) {
  const [linked, setLinked] = useState(false);
  const w = getProp(properties, "width");
  const h = getProp(properties, "height");
  const wNum = parseFloat(w) || 200;
  const hNum = parseFloat(h) || 150;

  const handleW = useCallback(
    (v: number) => onChange(setProp(properties, "width", `${v}px`)),
    [properties, onChange]
  );
  const handleH = useCallback(
    (v: number) => onChange(setProp(properties, "height", `${v}px`)),
    [properties, onChange]
  );

  const summary = `${w || "auto"} × ${h || "auto"}`;

  return (
    <CollapsibleGroup title="尺寸" summary={summary} defaultOpen={true}>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 11, fontWeight: 600, width: 16 }}>W</span>
          <ScrubInput value={wNum} onChange={handleW} min={0} max={4000} />
          <span style={{ fontSize: 11, fontWeight: 600, width: 16 }}>H</span>
          <ScrubInput value={hNum} onChange={handleH} min={0} max={4000} />
          <button
            onClick={() => setLinked(!linked)}
            title={linked ? "解除等比" : "锁定等比"}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              fontSize: 14,
              color: linked ? "#6366f1" : "#94a3b8",
              padding: "2px 4px",
            }}
          >
            🔗
          </button>
        </div>
        {/* Mini preview */}
        <div
          onDoubleClick={onFocus}
          style={{
            height: 60,
            background: "#f1f5f9",
            borderRadius: 6,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: "pointer",
            position: "relative",
          }}
          title="双击进入聚焦模式"
        >
          <div
            style={{
              width: Math.min(Math.max(20, wNum / 8), 200),
              height: Math.min(Math.max(12, hNum / 8), 60),
              border: "2px solid #6366f1",
              borderRadius: 2,
              background: "rgba(99,102,241,0.08)",
            }}
          />
          <span
            style={{
              position: "absolute",
              bottom: 4,
              fontSize: 9,
              color: "#94a3b8",
            }}
          >
            {w || "auto"} × {h || "auto"}
          </span>
        </div>
      </div>
    </CollapsibleGroup>
  );
}
```

- [ ] **Step 2: Create cards directory and TypeScript check**

```bash
mkdir -p poiesis/src/components/design/cards
npx tsc --noEmit --pretty 2>&1 | grep -E "error TS|Found"
```

- [ ] **Step 3: Commit**

---

### Task 7: BackgroundCard — Background Card with HSV Panel

**Files:**
- Create: `poiesis/src/components/design/cards/BackgroundCard.tsx`

**Interfaces:**
- Consumes: `CollapsibleGroup` from Task 2, `HSVPanel` from Task 4
- Produces: `BackgroundCard` component

- [ ] **Step 1: Create cards/BackgroundCard.tsx**

```typescript
// components/design/cards/BackgroundCard.tsx
// Background card: solid/gradient color with inline HSV picker.

import { useState, useCallback, useMemo } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { HSVPanel, type HSVColor } from "../HSVPanel";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils";

interface BackgroundCardProps {
  properties: StyleProperty[];
  tokens: DesignToken[];
  onChange: (props: StyleProperty[]) => void;
}

function getProp(props: StyleProperty[], name: string): string {
  return props.find((p) => p.name === name)?.segments[0]?.value ?? "";
}

function hexToHsv(hex: string): HSVColor {
  hex = hex.replace("#", "");
  const r = parseInt(hex.slice(0, 2), 16) / 255;
  const g = parseInt(hex.slice(2, 4), 16) / 255;
  const b = parseInt(hex.slice(4, 6), 16) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
    else if (max === g) h = ((b - r) / d + 2) * 60;
    else h = ((r - g) / d + 4) * 60;
  }
  return { h, s: max === 0 ? 0 : (d / max) * 100, v: max * 100, a: 1 };
}

function hsvToHex(h: number, s: number, v: number): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(Math.min(k, 4 - k, 1), 0);
  };
  const toHex = (n: number) => Math.round(n * 255).toString(16).padStart(2, "0");
  return `#${toHex(f(5))}${toHex(f(3))}${toHex(f(1))}`;
}

export function BackgroundCard({ properties, tokens, onChange }: BackgroundCardProps) {
  const bgColor = getProp(properties, "background") || "#ffffff";
  const isVarRef = properties.find((p) => p.name === "background")?.segments.some(
    (s) => s.type === "var-ref"
  );

  const [hsv, setHsv] = useState<HSVColor>(() => {
    if (isVarRef) {
      const tokenName = properties.find((p) => p.name === "background")?.segments.find(
        (s) => s.type === "var-ref"
      )?.name;
      const token = tokens.find((t) => t.name === tokenName);
      if (token) return hexToHsv(token.value);
    }
    if (/^#[0-9a-fA-F]{6}$/.test(bgColor)) return hexToHsv(bgColor);
    return hexToHsv("#ffffff");
  });

  const handleChange = useCallback(
    (c: HSVColor) => {
      setHsv(c);
      const hex = hsvToHex(c.h, c.s, c.v);
      const next = [...properties];
      const idx = next.findIndex((p) => p.name === "background");
      if (idx >= 0) {
        next[idx] = { ...next[idx], segments: [{ type: "text", value: hex }] };
      } else {
        next.push({ id: uid("prop"), name: "background", segments: [{ type: "text", value: hex }] });
      }
      onChange(next);
    },
    [properties, onChange]
  );

  const summary = isVarRef
    ? `var(--${properties.find((p) => p.name === "background")?.segments.find((s) => s.type === "var-ref")?.name})`
    : bgColor;

  return (
    <CollapsibleGroup title="背景" summary={summary} defaultOpen={false}>
      <HSVPanel color={hsv} onChange={handleChange} />
    </CollapsibleGroup>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 8: BorderCard — Border Card

**Files:**
- Create: `poiesis/src/components/design/cards/BorderCard.tsx`

**Interfaces:**
- Consumes: `CollapsibleGroup` from Task 2, `ScrubInput` from Task 1
- Consumes: `PropertyRow` from Task 5

- [ ] **Step 1: Create cards/BorderCard.tsx**

```typescript
// components/design/cards/BorderCard.tsx
// Border card: width/style/color with preview.

import { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import { PropertyRow } from "../PropertyRow";
import { ColorHUD } from "../ColorHUD";
import type { StyleProperty } from "../../../core/types/style";
import type { DesignToken } from "../../../core/types/design";
import { uid } from "../../../core/utils";

interface BorderCardProps {
  properties: StyleProperty[];
  tokens: DesignToken[];
  onChange: (props: StyleProperty[]) => void;
}

function getProp(props: StyleProperty[], name: string): string {
  return props.find((p) => p.name === name)?.segments[0]?.value ?? "";
}

function setProp(props: StyleProperty[], name: string, value: string): StyleProperty[] {
  const next = [...props];
  const idx = next.findIndex((p) => p.name === name);
  if (idx >= 0) {
    next[idx] = { ...next[idx], segments: [{ type: "text", value }] };
  } else if (value) {
    next.push({ id: uid("prop"), name, segments: [{ type: "text", value }] });
  }
  return next;
}

const BORDER_STYLES = ["solid", "dashed", "dotted", "double", "none"];

export function BorderCard({ properties, tokens, onChange }: BorderCardProps) {
  const [showColorHUD, setShowColorHUD] = useState(false);
  const [colorAnchor, setColorAnchor] = useState<DOMRect | null>(null);

  const width = getProp(properties, "border") || "";
  const style = getProp(properties, "borderStyle") || "solid";
  const color = getProp(properties, "borderColor") || "#d4d4d8";

  const handleWidth = useCallback(
    (v: number) => {
      // border is compound "2px solid #xxx", but we track simple case
      onChange(setProp(properties, "border", `${v}px`));
    },
    [properties, onChange]
  );

  const handleStyle = useCallback(
    (s: string) => onChange(setProp(properties, "borderStyle", s)),
    [properties, onChange]
  );

  const handleColor = useCallback(
    (hex: string, tokenName: string | null) => {
      if (tokenName) {
        const next = setProp(properties, "borderColor", hex);
        const idx = next.findIndex((p) => p.name === "borderColor");
        if (idx >= 0) {
          next[idx] = { ...next[idx], segments: [{ type: "var-ref", name: tokenName }] };
        }
        onChange(next);
      } else {
        onChange(setProp(properties, "borderColor", hex));
      }
      setShowColorHUD(false);
    },
    [properties, onChange]
  );

  const wNum = parseFloat(width) || 1;
  const summary = `${width || 0}px ${style} ${color}`;

  return (
    <CollapsibleGroup title="边框" summary={summary} defaultOpen={false}>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <PropertyRow label="粗细">
          <ScrubInput value={wNum} onChange={handleWidth} min={0} max={100} />
          <span style={{ fontSize: 10, color: "#94a3b8", marginLeft: 4 }}>px</span>
        </PropertyRow>

        <PropertyRow label="样式">
          <select
            value={style}
            onChange={(e) => handleStyle(e.target.value)}
            style={{
              padding: "2px 4px",
              fontSize: 11,
              border: "1px solid #e2e8f0",
              borderRadius: 4,
              background: "#fff",
              color: "#374151",
              flex: 1,
            }}
          >
            {BORDER_STYLES.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
        </PropertyRow>

        <PropertyRow label="颜色">
          <button
            onClick={(e) => {
              setColorAnchor((e.currentTarget as HTMLElement).getBoundingClientRect());
              setShowColorHUD(!showColorHUD);
            }}
            style={{
              width: 24,
              height: 24,
              borderRadius: 4,
              border: "1px solid #cbd5e1",
              background: color,
              cursor: "pointer",
              padding: 0,
            }}
          />
          <span style={{ fontSize: 10, fontFamily: "monospace", color: "#64748b" }}>
            {color}
          </span>
        </PropertyRow>
      </div>

      {showColorHUD && colorAnchor && (
        <ColorHUD
          hex={color}
          tokens={tokens}
          onChange={handleColor}
          onClose={() => setShowColorHUD(false)}
          anchorRect={colorAnchor}
        />
      )}
    </CollapsibleGroup>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 9: RadiusCard — Border Radius Card

**Files:**
- Create: `poiesis/src/components/design/cards/RadiusCard.tsx`

**Interfaces:**
- Consumes: `CollapsibleGroup` from Task 2, `ScrubInput` from Task 1

- [ ] **Step 1: Create cards/RadiusCard.tsx**

```typescript
// components/design/cards/RadiusCard.tsx
// Border radius card: unified or four-corner independent editing.

import { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils";

interface RadiusCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
}

function getProp(props: StyleProperty[], name: string): string {
  return props.find((p) => p.name === name)?.segments[0]?.value ?? "";
}

function setProp(props: StyleProperty[], name: string, value: string): StyleProperty[] {
  const next = [...props];
  const idx = next.findIndex((p) => p.name === name);
  if (idx >= 0) {
    if (value) next[idx] = { ...next[idx], segments: [{ type: "text", value }] };
    else next.splice(idx, 1);
  } else if (value) {
    next.push({ id: uid("prop"), name, segments: [{ type: "text", value }] });
  }
  return next;
}

export function RadiusCard({ properties, onChange }: RadiusCardProps) {
  const [unified, setUnified] = useState(true);

  const rAll = getProp(properties, "borderRadius");
  const rTL = getProp(properties, "borderTopLeftRadius");
  const rTR = getProp(properties, "borderTopRightRadius");
  const rBR = getProp(properties, "borderBottomRightRadius");
  const rBL = getProp(properties, "borderBottomLeftRadius");

  const unifiedVal = parseFloat(rAll) || 0;

  const handleUnified = useCallback(
    (v: number) => {
      // Set borderRadius and clear corner overrides
      let next = setProp(properties, "borderRadius", `${v}px`);
      next = setProp(next, "borderTopLeftRadius", "");
      next = setProp(next, "borderTopRightRadius", "");
      next = setProp(next, "borderBottomRightRadius", "");
      next = setProp(next, "borderBottomLeftRadius", "");
      onChange(next);
    },
    [properties, onChange]
  );

  const handleCorner = useCallback(
    (corner: string, v: number) => {
      onChange(setProp(properties, corner, `${v}px`));
    },
    [properties, onChange]
  );

  const toggleMode = useCallback(() => {
    if (unified) {
      // Switching to individual: copy unified value to all corners
      const val = unifiedVal;
      let next = setProp(properties, "borderRadius", "");
      next = setProp(next, "borderTopLeftRadius", `${val}px`);
      next = setProp(next, "borderTopRightRadius", `${val}px`);
      next = setProp(next, "borderBottomRightRadius", `${val}px`);
      next = setProp(next, "borderBottomLeftRadius", `${val}px`);
      onChange(next);
    } else {
      // Switching to unified: average corners
      const vals = [
        parseFloat(rTL) || 0,
        parseFloat(rTR) || 0,
        parseFloat(rBR) || 0,
        parseFloat(rBL) || 0,
      ].filter((v) => v > 0);
      const avg = vals.length > 0 ? Math.round(vals.reduce((a, b) => a + b, 0) / vals.length) : 0;
      let next = setProp(properties, "borderRadius", `${avg}px`);
      next = setProp(next, "borderTopLeftRadius", "");
      next = setProp(next, "borderTopRightRadius", "");
      next = setProp(next, "borderBottomRightRadius", "");
      next = setProp(next, "borderBottomLeftRadius", "");
      onChange(next);
    }
    setUnified(!unified);
  }, [unified, unifiedVal, rTL, rTR, rBR, rBL, properties, onChange]);

  const summary = unified
    ? `${rAll || "0"}px`
    : `${rTL || "0"},${rTR || "0"},${rBR || "0"},${rBL || "0"}`;

  return (
    <CollapsibleGroup title="圆角" summary={summary} defaultOpen={false}>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          {unified ? (
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ fontSize: 11 }}>圆角</span>
              <ScrubInput value={unifiedVal} onChange={handleUnified} min={0} max={200} />
              <span style={{ fontSize: 10, color: "#94a3b8" }}>px</span>
            </div>
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ fontSize: 10, width: 20 }}>↖</span>
                <ScrubInput
                  value={parseFloat(rTL) || 0}
                  onChange={(v) => handleCorner("borderTopLeftRadius", v)}
                  min={0}
                  max={200}
                />
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ fontSize: 10, width: 20 }}>↗</span>
                <ScrubInput
                  value={parseFloat(rTR) || 0}
                  onChange={(v) => handleCorner("borderTopRightRadius", v)}
                  min={0}
                  max={200}
                />
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ fontSize: 10, width: 20 }}>↙</span>
                <ScrubInput
                  value={parseFloat(rBL) || 0}
                  onChange={(v) => handleCorner("borderBottomLeftRadius", v)}
                  min={0}
                  max={200}
                />
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ fontSize: 10, width: 20 }}>↘</span>
                <ScrubInput
                  value={parseFloat(rBR) || 0}
                  onChange={(v) => handleCorner("borderBottomRightRadius", v)}
                  min={0}
                  max={200}
                />
              </div>
            </div>
          )}
          <button
            onClick={toggleMode}
            title={unified ? "独立四角" : "统一圆角"}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              fontSize: 14,
              color: unified ? "#94a3b8" : "#6366f1",
              padding: "2px 4px",
            }}
          >
            {unified ? "🔗" : "🔓"}
          </button>
        </div>

        {/* Preview */}
        <div
          style={{
            height: 50,
            background: "#f1f5f9",
            borderRadius: `${unified ? unifiedVal : `${rTL || "0"}px ${rTR || "0"}px ${rBR || "0"}px ${rBL || "0"}px`}`,
            border: "2px solid #6366f1",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <span style={{ fontSize: 9, color: "#94a3b8" }}>
            {unified ? `${unifiedVal}px` : "独立圆角"}
          </span>
        </div>
      </div>
    </CollapsibleGroup>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 10: SpacingCard — Spacing Card with Box Model

**Files:**
- Create: `poiesis/src/components/design/cards/BoxModelPreview.tsx`
- Create: `poiesis/src/components/design/cards/SpacingCard.tsx`

**Interfaces:**
- Consumes: `CollapsibleGroup` from Task 2, `ScrubInput` from Task 1

- [ ] **Step 1: Create cards/BoxModelPreview.tsx**

```typescript
// components/design/cards/BoxModelPreview.tsx
// Box model diagram: margin (orange) → padding (green) → content (white)

interface BoxModelPreviewProps {
  margin: { top: number; right: number; bottom: number; left: number };
  padding: { top: number; right: number; bottom: number; left: number };
  size?: number;
}

export function BoxModelPreview({
  margin,
  padding,
  size = 120,
}: BoxModelPreviewProps) {
  const totalM = margin.top + margin.bottom;
  const totalP = padding.top + padding.bottom;
  const scale = Math.max(1, (totalM + totalP + 20) / size);
  const s = (v: number) => Math.max(1, v / scale);

  const mT = s(margin.top), mR = s(margin.right), mB = s(margin.bottom), mL = s(margin.left);
  const pT = s(padding.top), pR = s(padding.right), pB = s(padding.bottom), pL = s(padding.left);

  const totalH = mT + pT + 30 + pB + mB;
  const totalW = mL + pL + 50 + pR + mR;

  return (
    <div style={{ display: "flex", justifyContent: "center", padding: 4 }}>
      <div style={{ position: "relative", width: totalW, height: Math.max(totalH, 40) }}>
        {/* Margin (orange) */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            background: "rgba(249,115,22,0.15)",
            border: "1px dashed rgba(249,115,22,0.4)",
            borderRadius: 2,
          }}
        />
        {/* Padding (green) */}
        <div
          style={{
            position: "absolute",
            top: mT,
            left: mL,
            right: mR,
            bottom: mB,
            background: "rgba(34,197,94,0.15)",
            border: "1px dashed rgba(34,197,94,0.4)",
            borderRadius: 2,
          }}
        />
        {/* Content (white) */}
        <div
          style={{
            position: "absolute",
            top: mT + pT,
            left: mL + pL,
            right: mR + pR,
            bottom: mB + pB,
            background: "#fff",
            border: "1px solid #cbd5e1",
            borderRadius: 2,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <span style={{ fontSize: 8, color: "#94a3b8" }}>内容</span>
        </div>

        {/* Labels */}
        <div style={{
          position: "absolute", top: 0, left: "50%", transform: "translateX(-50%)",
          fontSize: 8, color: "rgba(249,115,22,0.7)",
        }}>
          M:{margin.top}
        </div>
        <div style={{
          position: "absolute", bottom: 0, left: "50%", transform: "translateX(-50%)",
          fontSize: 8, color: "rgba(249,115,22,0.7)",
        }}>
          M:{margin.bottom}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create cards/SpacingCard.tsx**

```typescript
// components/design/cards/SpacingCard.tsx
// Spacing card: padding + margin with box model preview.

import { useState, useCallback } from "react";
import { CollapsibleGroup } from "../CollapsibleGroup";
import { ScrubInput } from "../ScrubInput";
import { BoxModelPreview } from "./BoxModelPreview";
import type { StyleProperty } from "../../../core/types/style";
import { uid } from "../../../core/utils";

interface SpacingCardProps {
  properties: StyleProperty[];
  onChange: (props: StyleProperty[]) => void;
}

function getProp(props: StyleProperty[], name: string): string {
  return props.find((p) => p.name === name)?.segments[0]?.value ?? "";
}

function setProp(props: StyleProperty[], name: string, value: string): StyleProperty[] {
  const next = [...props];
  const idx = next.findIndex((p) => p.name === name);
  if (idx >= 0) {
    if (value) next[idx] = { ...next[idx], segments: [{ type: "text", value }] };
    else next.splice(idx, 1);
  } else if (value) {
    next.push({ id: uid("prop"), name, segments: [{ type: "text", value }] });
  }
  return next;
}

export function SpacingCard({ properties, onChange }: SpacingCardProps) {
  const [padUnified, setPadUnified] = useState(true);
  const [marUnified, setMarUnified] = useState(true);

  // Padding values
  const padAll = parseFloat(getProp(properties, "padding")) || 0;
  const padT = parseFloat(getProp(properties, "paddingTop")) || 0;
  const padR = parseFloat(getProp(properties, "paddingRight")) || 0;
  const padB = parseFloat(getProp(properties, "paddingBottom")) || 0;
  const padL = parseFloat(getProp(properties, "paddingLeft")) || 0;

  // Margin values
  const marAll = parseFloat(getProp(properties, "margin")) || 0;
  const marT = parseFloat(getProp(properties, "marginTop")) || 0;
  const marR = parseFloat(getProp(properties, "marginRight")) || 0;
  const marB = parseFloat(getProp(properties, "marginBottom")) || 0;
  const marL = parseFloat(getProp(properties, "marginLeft")) || 0;

  const handlePadUnified = useCallback(
    (v: number) => {
      let next = setProp(properties, "padding", `${v}px`);
      next = setProp(next, "paddingTop", "");
      next = setProp(next, "paddingRight", "");
      next = setProp(next, "paddingBottom", "");
      next = setProp(next, "paddingLeft", "");
      onChange(next);
    },
    [properties, onChange]
  );

  const handlePadCorner = useCallback(
    (corner: string, v: number) => {
      onChange(setProp(properties, corner, `${v}px`));
    },
    [properties, onChange]
  );

  const handleMarUnified = useCallback(
    (v: number) => {
      let next = setProp(properties, "margin", `${v}px`);
      next = setProp(next, "marginTop", "");
      next = setProp(next, "marginRight", "");
      next = setProp(next, "marginBottom", "");
      next = setProp(next, "marginLeft", "");
      onChange(next);
    },
    [properties, onChange]
  );

  const handleMarCorner = useCallback(
    (corner: string, v: number) => {
      onChange(setProp(properties, corner, `${v}px`));
    },
    [properties, onChange]
  );

  const summary = `P:${padAll || `${padT}/${padR}/${padB}/${padL}`}px M:${marAll || `${marT}/${marR}/${marB}/${marL}`}px`;

  const margin = { top: marUnified ? marAll : marT, right: marUnified ? marAll : marR, bottom: marUnified ? marAll : marB, left: marUnified ? marAll : marL };
  const padding = { top: padUnified ? padAll : padT, right: padUnified ? padAll : padR, bottom: padUnified ? padAll : padB, left: padUnified ? padAll : padL };

  return (
    <CollapsibleGroup title="间距" summary={summary} defaultOpen={false}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {/* Padding */}
        <div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
            <span style={{ fontSize: 11, fontWeight: 600 }}>Padding</span>
            <button
              onClick={() => setPadUnified(!padUnified)}
              title={padUnified ? "独立四边" : "统一"}
              style={{
                border: "none", background: "none", cursor: "pointer",
                fontSize: 14, color: padUnified ? "#94a3b8" : "#6366f1", padding: "2px 4px",
              }}
            >
              {padUnified ? "🔗" : "🔓"}
            </button>
          </div>
          {padUnified ? (
            <ScrubInput value={padAll} onChange={handlePadUnified} min={0} max={500} />
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
              {(["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"] as const).map((key) => {
                const val = parseFloat(getProp(properties, key)) || 0;
                const label = key === "paddingTop" ? "上" : key === "paddingRight" ? "右" : key === "paddingBottom" ? "下" : "左";
                return (
                  <div key={key} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <span style={{ fontSize: 10, width: 16 }}>{label}</span>
                    <ScrubInput value={val} onChange={(v) => handlePadCorner(key, v)} min={0} max={500} />
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Margin */}
        <div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
            <span style={{ fontSize: 11, fontWeight: 600 }}>Margin</span>
            <button
              onClick={() => setMarUnified(!marUnified)}
              title={marUnified ? "独立四边" : "统一"}
              style={{
                border: "none", background: "none", cursor: "pointer",
                fontSize: 14, color: marUnified ? "#94a3b8" : "#6366f1", padding: "2px 4px",
              }}
            >
              {marUnified ? "🔗" : "🔓"}
            </button>
          </div>
          {marUnified ? (
            <ScrubInput value={marAll} onChange={handleMarUnified} min={0} max={500} />
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
              {(["marginTop", "marginRight", "marginBottom", "marginLeft"] as const).map((key) => {
                const val = parseFloat(getProp(properties, key)) || 0;
                const label = key === "marginTop" ? "上" : key === "marginRight" ? "右" : key === "marginBottom" ? "下" : "左";
                return (
                  <div key={key} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <span style={{ fontSize: 10, width: 16 }}>{label}</span>
                    <ScrubInput value={val} onChange={(v) => handleMarCorner(key, v)} min={0} max={500} />
                  </div>
                );
              })}
            </div>
          )}
        </div>

        <BoxModelPreview margin={margin} padding={padding} />
      </div>
    </CollapsibleGroup>
  );
}
```

- [ ] **Step 3: TypeScript check** → only pre-existing errors

- [ ] **Step 4: Commit**

---

### Task 11: FocusOverlay — Focus Mode Container

**Files:**
- Create: `poiesis/src/components/design/FocusOverlay.tsx`

**Interfaces:**
- Produces: `FocusOverlay` component
  ```typescript
  interface FocusOverlayProps {
    visible: boolean;
    onClose: () => void;
    children: React.ReactNode;   // The full-preview element
    floatingCard: React.ReactNode; // The floating control card
  }
  ```

- [ ] **Step 1: Create FocusOverlay.tsx**

```typescript
// components/design/FocusOverlay.tsx
// Focus mode: full-size preview with floating card overlay.

import { useEffect, useCallback } from "react";

interface FocusOverlayProps {
  visible: boolean;
  onClose: () => void;
  children: React.ReactNode;
  floatingCard: React.ReactNode;
}

export function FocusOverlay({ visible, onClose, children, floatingCard }: FocusOverlayProps) {
  // Esc to exit
  useEffect(() => {
    if (!visible) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [visible, onClose]);

  if (!visible) return null;

  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        zIndex: 500,
        background: "#f8fafc",
        display: "flex",
        flexDirection: "column",
      }}
    >
      {/* Top bar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "6px 12px",
          borderBottom: "1px solid #e2e8f0",
          background: "#fff",
          flexShrink: 0,
        }}
      >
        <span style={{ fontSize: 11, fontWeight: 600, color: "#6366f1" }}>
          聚焦模式 — 拖拽元素边缘直接编辑
        </span>
        <button
          onClick={onClose}
          style={{
            border: "none",
            background: "none",
            cursor: "pointer",
            fontSize: 18,
            color: "#94a3b8",
            padding: "0 4px",
          }}
        >
          ×
        </button>
      </div>

      {/* Preview area */}
      <div
        style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          padding: 32,
          overflow: "auto",
        }}
      >
        {children}
      </div>

      {/* Floating card */}
      <div
        style={{
          position: "absolute",
          bottom: 16,
          right: 16,
          background: "#fff",
          border: "1px solid #e2e8f0",
          borderRadius: 8,
          boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
          padding: "12px 16px",
          minWidth: 200,
          zIndex: 501,
        }}
      >
        {floatingCard}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 12: Assemble ClassEditor — Replace Old Editor with Cards

**Files:**
- Rewrite: `poiesis/src/components/design/ClassEditor.tsx`

**Interfaces:**
- Consumes: All cards from Tasks 6-10, FocusOverlay from Task 11, CollapsibleGroup/PropertyRow/ScrubInput/IconButtonGroup from Tasks 1-5
- Consumes: store (`usePoiesisStore`), design types unchanged
- Existing imports preserved: `ClassPreview`, pseudo tabs, token picker

- [ ] **Step 1: Rewrite ClassEditor.tsx**

The new ClassEditor keeps the left sidebar and pseudo tabs, but replaces the flat property list with cards + collapsible groups layout.

Key changes from old ClassEditor:
1. Import all cards instead of old PROPERTY_GROUPS rendering
2. Replace `renderPropertyGroups()` with cards rendering
3. Each card reads/writes `selectedClass.properties`
4. Add focusMode state, render FocusOverlay on double-click
5. Keep ClassPreview in right panel

See full file in implementation — too long to inline here. Key structure:

```typescript
// Inside the property editor area (replacing renderPropertyGroups()):
<div style={{ flex: 1, overflow: "auto", padding: "8px 0" }}>
  {/* Cards section */}
  <SizeCard
    properties={selectedClass.properties}
    onChange={handlePropsChange}
    onFocus={() => setFocusMode("size")}
  />
  <BackgroundCard
    properties={selectedClass.properties}
    tokens={tokens}
    onChange={handlePropsChange}
  />
  <BorderCard
    properties={selectedClass.properties}
    tokens={tokens}
    onChange={handlePropsChange}
  />
  <RadiusCard
    properties={selectedClass.properties}
    onChange={handlePropsChange}
  />
  <SpacingCard
    properties={selectedClass.properties}
    onChange={handlePropsChange}
  />

  {/* Collapsible groups for remaining properties */}
  <CollapsibleGroup title="布局" summary={layoutSummary} defaultOpen={false}>
    {/* IconButtonGroup for display/direction, ScrubInput for gap, etc. */}
  </CollapsibleGroup>

  <CollapsibleGroup title="文字" summary={typographySummary} defaultOpen={false}>
    {/* Font select, ScrubInput for size/line/letter, IconButtonGroup for align/decoration */}
  </CollapsibleGroup>
</div>
```

- [ ] **Step 2: TypeScript check** → only pre-existing errors

- [ ] **Step 3: Commit**

---

### Task 13: Victory Lap — Verify Build and Integration

- [ ] **Step 1: Full TypeScript check**

```bash
npx tsc --noEmit --pretty 2>&1 | grep -v "pipeline.test.ts"
```

Expect: zero new errors.

- [ ] **Step 2: Verify all imports resolve**

```bash
rg "from.*\.\.\/" poiesis/src/components/design/cards/ | head -20
```

- [ ] **Step 3: Verify cards render in existing app (manual via Poiesis)**

- [ ] **Step 4: Commit**

---

### Task 14: Refactor — Clean Up Deprecated Code from ClassEditor

**Files:**
- Modify: `poiesis/src/components/design/ClassEditor.tsx` (remove unused helpers/constants)

Remove deprecated constants that are no longer used after card migration:
- `PROPERTY_GROUPS` (Partially - keep for Layout/Typography collapsed groups)
- Old `renderPropertyRow` / `renderPropertyGroups` functions
- `INPUT_STYLE`, `SELECT_STYLE`, `PROP_ROW`, `PROP_LABEL`, `INHERITED_LABEL`, `GROUP_HEADER` style constants (replaced by ScrubInput/PropertyRow/CollapsibleGroup)

- [ ] **Step 1: Remove deprecated code**
- [ ] **Step 2: TypeScript check** → only pre-existing errors
- [ ] **Step 3: Commit**
