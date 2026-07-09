// components/design/ClassPreview.tsx
// Live preview of a class's CSS properties on a selectable HTML element.
// In focus mode (interactive=true), supports drag-to-resize via edge/corner handles.

import React, { useState, useMemo, useRef, useCallback, useEffect } from "react";
import type { StyleProperty } from "../../core/types/style";
import type { PseudoState, DesignToken } from "../../core/types/design";

// ── Helpers ──

function segmentsToString(segments: StyleProperty["segments"]): string {
  return segments.map((s) => (s.type === "text" ? s.value : "")).join("");
}

function resolvePropertyValue(p: StyleProperty, tokens: DesignToken[]): string {
  const varRef = p.segments.find((s) => s.type === "var-ref");
  if (varRef) {
    const token = tokens.find((t) => t.name === varRef.name);
    if (token) return token.value;
    return `var(--${varRef.name})`;
  }
  if (p.segments.some((s) => s.type === "signal")) return "";
  return segmentsToString(p.segments);
}

function propertiesToInlineStyle(props: StyleProperty[], tokens: DesignToken[]): React.CSSProperties {
  const style: React.CSSProperties = {};
  for (const p of props) {
    const value = resolvePropertyValue(p, tokens);
    if (value) {
      (style as Record<string, string>)[p.name] = value;
    }
  }
  return style;
}

// ── Preview element options ──

const PREVIEW_ELEMENTS = [
  { value: "div", label: "<div>", content: "Div content" },
  { value: "p", label: "<p>", content: "这是一段文字，展示了当前 class 的样式效果。" },
  { value: "span", label: "<span>", content: "Inline text" },
  { value: "h1", label: "<h1>", content: "标题文字" },
  { value: "h2", label: "<h2>", content: "标题文字" },
  { value: "h3", label: "<h3>", content: "标题文字" },
  { value: "button", label: "<button>", content: "按钮", isButton: true },
  { value: "input", label: "<input>", isInput: true, extra: { placeholder: "输入文字..." } },
  { value: "img", label: "<img>", isImg: true },
  { value: "a", label: "<a>", content: "点此链接", isLink: true },
  { value: "section", label: "<section>", content: "Section content" },
  { value: "article", label: "<article>", content: "Article content" },
] as const;

// ── Resize types ──

type HandleDir = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

const HANDLE_SIZE = 8;
const HANDLE_HIT = 12;

interface DragState {
  startX: number;
  startY: number;
  startW: number;
  startH: number;
}

// ── Props ──

interface ClassPreviewProps {
  baseProperties: StyleProperty[];
  pseudoStates: PseudoState[];
  className: string;
  tokens: DesignToken[];
  interactive?: boolean;
  onResize?: (width: number, height: number) => void;
  onResizeEnd?: (width: number, height: number) => void;
  width?: number;
  height?: number;
}

// ── Directional math ──

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

// ── Handle position/cursor map ──

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

// ── Main component ──

export const ClassPreview = React.memo(function ClassPreview({
  baseProperties,
  pseudoStates,
  tokens,
  interactive = false,
  onResize,
  onResizeEnd,
  width: propWidth,
  height: propHeight,
}: ClassPreviewProps) {
  const [elementIdx, setElementIdx] = useState(0);
  const [simulatedPseudo, setSimulatedPseudo] = useState<string | null>(null);

  // Displayed dimensions (updated only on drag end or prop change)
  const [displayW, setDisplayW] = useState(propWidth ?? 0);
  const [displayH, setDisplayH] = useState(propHeight ?? 0);

  // Refs for direct DOM manipulation during drag
  const previewElRef = useRef<HTMLElement | null>(null);
  const labelRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<DragState | null>(null);
  const onResizeRef = useRef(onResize);
  const onResizeEndRef = useRef(onResizeEnd);
  onResizeRef.current = onResize;
  onResizeEndRef.current = onResizeEnd;

  // Sync display dimensions when props change (external update)
  useEffect(() => {
    setDisplayW(propWidth ?? 0);
    setDisplayH(propHeight ?? 0);
  }, [propWidth, propHeight]);

  const inlineStyle = useMemo(() => {
    const base = propertiesToInlineStyle(baseProperties, tokens);
    if (simulatedPseudo) {
      const ps = pseudoStates.find((p) => p.pseudo === simulatedPseudo);
      if (ps) Object.assign(base, propertiesToInlineStyle(ps.properties, tokens));
    }
    if (!base.transition) base.transition = "all 150ms ease";
    if (interactive) {
      if (displayW > 0) (base as Record<string, string>).width = `${displayW}px`;
      if (displayH > 0) (base as Record<string, string>).height = `${displayH}px`;
    }
    return base;
  }, [baseProperties, pseudoStates, simulatedPseudo, tokens, interactive, displayW, displayH]);

  const hasHover = pseudoStates.some((p) => p.pseudo === "hover");
  const hasFocus = pseudoStates.some((p) => p.pseudo === "focus");
  const hasActive = pseudoStates.some((p) => p.pseudo === "active");

  const simProps: Record<string, unknown> = { style: inlineStyle };
  if (hasHover) {
    simProps.onMouseEnter = () => setSimulatedPseudo("hover");
    simProps.onMouseLeave = () => setSimulatedPseudo(null);
  }
  if (hasFocus) {
    simProps.onFocus = () => setSimulatedPseudo("focus");
    simProps.onBlur = () => setSimulatedPseudo(null);
  }
  if (hasActive) {
    simProps.onMouseDown = () => setSimulatedPseudo("active");
    simProps.onMouseUp = () => setSimulatedPseudo(null);
  }

  // ── Resize: mouse-down on a handle ──

  const handleMouseDown = useCallback(
    (dir: HandleDir) => (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const el = previewElRef.current;
      if (!el) return;

      // Read current rendered dimensions from the DOM (bypass React state)
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

        // Direct DOM write — zero React overhead
        el.style.width = `${w}px`;
        el.style.height = `${h}px`;

        // Update label if present
        if (labelRef.current) {
          labelRef.current.textContent = `${w} × ${h}`;
        }

        // Notify store (throttled by rAF)
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
          setDisplayW(finalW);
          setDisplayH(finalH);
          onResizeEndRef.current?.(finalW, finalH);
        }
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [],
  );

  // ── Preview element ──

  const renderPreview = () => {
    const value = PREVIEW_ELEMENTS[elementIdx];
    const refCallback = (el: HTMLElement | null) => { previewElRef.current = el; };

    switch (value.value) {
      case "input":
        return <input ref={refCallback as React.Ref<HTMLInputElement>} {...(simProps as React.InputHTMLAttributes<HTMLInputElement>)} placeholder={value.extra?.placeholder ?? ""} readOnly />;
      case "img":
        return (
          <img
            ref={refCallback as React.Ref<HTMLImageElement>}
            {...(simProps as React.ImgHTMLAttributes<HTMLImageElement>)}
            src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='80'%3E%3Crect width='120' height='80' fill='%23e2e8f0'/%3E%3Ctext x='60' y='45' font-size='12' text-anchor='middle' fill='%2394a3b8'%3EImage%3C/text%3E%3C/svg%3E"
            alt="preview"
          />
        );
      case "a":
        return <a ref={refCallback as React.Ref<HTMLAnchorElement>} {...(simProps as React.AnchorHTMLAttributes<HTMLAnchorElement>)} href="#">{value.content}</a>;
      case "button":
        return <button ref={refCallback as React.Ref<HTMLButtonElement>} {...(simProps as React.ButtonHTMLAttributes<HTMLButtonElement>)} type="button">{value.content}</button>;
      case "p":
        return <p ref={refCallback as React.Ref<HTMLParagraphElement>} {...(simProps as React.HTMLAttributes<HTMLParagraphElement>)}>{value.content}</p>;
      case "h1":
      case "h2":
      case "h3":
        return React.createElement(value.value, { ref: refCallback, ...simProps as React.HTMLAttributes<HTMLHeadingElement> }, value.content);
      case "span":
        return <span ref={refCallback as React.Ref<HTMLSpanElement>} {...(simProps as React.HTMLAttributes<HTMLSpanElement>)}>{value.content}</span>;
      case "section":
      case "article":
        return React.createElement(value.value, { ref: refCallback, ...simProps as React.HTMLAttributes<HTMLElement> }, value.content);
      default:
        return <div ref={refCallback as React.Ref<HTMLDivElement>} {...(simProps as React.HTMLAttributes<HTMLDivElement>)}>{value.content}</div>;
    }
  };

  const ALL_HANDLES: HandleDir[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

  const previewContent = (
    <div style={{ display: "inline-flex", flexDirection: "column", maxWidth: "100%" }}>
      {renderPreview()}
    </div>
  );

  const showLabel = interactive && (displayW > 0 || displayH > 0);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        width: 260,
        flexShrink: 0,
        borderLeft: "1px solid var(--dp-border, #e8e5f2)",
        background: "var(--dp-surface, #fff)",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "8px 12px",
          fontSize: 10,
          fontWeight: 600,
          color: "var(--dp-text-secondary, #6b6893)",
          textTransform: "uppercase",
          letterSpacing: "0.5px",
          borderBottom: "1px solid var(--dp-border, #e8e5f2)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexShrink: 0,
          background: "var(--dp-bg, #f7f6fa)",
        }}
      >
        Preview
        <select
          value={elementIdx}
          onChange={(e) => setElementIdx(Number(e.target.value))}
          style={{
            padding: "2px 6px",
            fontSize: 10,
            fontFamily: "var(--dp-font-mono, monospace)",
            border: "1px solid var(--dp-border, #e8e5f2)",
            borderRadius: "var(--dp-radius-sm, 6px)",
            outline: "none",
            background: "var(--dp-surface, #fff)",
            color: "var(--dp-text, #1e1b4b)",
            cursor: "pointer",
            fontWeight: 500,
            textTransform: "none",
            letterSpacing: "normal",
          }}
        >
          {PREVIEW_ELEMENTS.map((e, i) => (
            <option key={e.value} value={i}>{e.label}</option>
          ))}
        </select>
      </div>

      {/* Preview area */}
      <div
        style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          padding: interactive ? 32 : 24,
          overflow: "auto",
          minHeight: 120,
          background: `
            linear-gradient(45deg, #f0eef7 25%, transparent 25%),
            linear-gradient(-45deg, #f0eef7 25%, transparent 25%),
            linear-gradient(45deg, transparent 75%, #f0eef7 75%),
            linear-gradient(-45deg, transparent 75%, #f0eef7 75%)
          `,
          backgroundSize: "12px 12px",
          backgroundPosition: "0 0, 0 6px, 6px -6px, -6px 0px",
        }}
      >
        {interactive ? (
          <div style={{ position: "relative", display: "inline-block" }}>
            {/* Dimension label */}
            {showLabel && (
              <div
                ref={labelRef}
                style={{
                  position: "absolute",
                  top: -20,
                  left: "50%",
                  transform: "translateX(-50%)",
                  fontSize: 10,
                  fontFamily: "var(--dp-font-mono, monospace)",
                  color: "var(--dp-accent, #6366f1)",
                  background: "rgba(255,255,255,0.9)",
                  padding: "1px 6px",
                  borderRadius: 3,
                  whiteSpace: "nowrap",
                  pointerEvents: "none",
                }}
              >
                {displayW > 0 ? displayW : "auto"} × {displayH > 0 ? displayH : "auto"}
              </div>
            )}
            {previewContent}
            {/* Resize handles */}
            {ALL_HANDLES.map((dir) => {
              const { pos, cursor } = HANDLE_STYLE_MAP[dir];
              return (
                <div
                  key={dir}
                  style={{
                    position: "absolute",
                    width: HANDLE_HIT,
                    height: HANDLE_HIT,
                    zIndex: 10,
                    cursor,
                    ...pos,
                  }}
                  onMouseDown={handleMouseDown(dir)}
                >
                  <div
                    style={{
                      position: "absolute",
                      top: "50%",
                      left: "50%",
                      transform: "translate(-50%, -50%)",
                      width: HANDLE_SIZE,
                      height: HANDLE_SIZE,
                      borderRadius: 2,
                      background: "#fff",
                      border: "2px solid var(--dp-accent, #6366f1)",
                      boxShadow: "0 1px 3px rgba(0,0,0,0.15)",
                      pointerEvents: "none",
                    }}
                  />
                </div>
              );
            })}
            {/* Outline */}
            <div
              style={{
                position: "absolute",
                inset: -1,
                border: "1px dashed var(--dp-accent, #6366f1)",
                borderRadius: 2,
                pointerEvents: "none",
              }}
            />
          </div>
        ) : (
          previewContent
        )}
      </div>

      {/* Pseudo state indicator */}
      {simulatedPseudo && (
        <div
          style={{
            textAlign: "center",
            fontSize: 10,
            color: "var(--dp-accent, #6366f1)",
            fontWeight: 600,
            padding: "4px 0",
            background: "var(--dp-accent-subtle, #eef0ff)",
            borderTop: "1px solid var(--dp-border, #e8e5f2)",
          }}
        >
          :{simulatedPseudo}
        </div>
      )}
    </div>
  );
});
