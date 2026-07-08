// components/design/ClassPreview.tsx
// Live preview of a class's CSS properties on a selectable HTML element.

import React, { useState, useMemo } from "react";
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

// ── Props ──

interface ClassPreviewProps {
  baseProperties: StyleProperty[];
  pseudoStates: PseudoState[];
  className: string;
  tokens: DesignToken[];
}

// ── Component ──

export function ClassPreview({ baseProperties, pseudoStates, className, tokens }: ClassPreviewProps) {
  const [elementIdx, setElementIdx] = useState(0);
  const [simulatedPseudo, setSimulatedPseudo] = useState<string | null>(null);

  const inlineStyle = useMemo(() => {
    const base = propertiesToInlineStyle(baseProperties, tokens);
    if (simulatedPseudo) {
      const ps = pseudoStates.find((p) => p.pseudo === simulatedPseudo);
      if (ps) {
        Object.assign(base, propertiesToInlineStyle(ps.properties, tokens));
      }
    }
    if (!base.transition) {
      base.transition = "all 150ms ease";
    }
    return base;
  }, [baseProperties, pseudoStates, simulatedPseudo, tokens]);

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

  const renderPreview = () => {
    const value = PREVIEW_ELEMENTS[elementIdx];
    switch (value.value) {
      case "input":
        return <input {...(simProps as React.InputHTMLAttributes<HTMLInputElement>)} placeholder={value.extra?.placeholder ?? ""} readOnly />;
      case "img":
        return (
          <img
            {...(simProps as React.ImgHTMLAttributes<HTMLImageElement>)}
            src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='80'%3E%3Crect width='120' height='80' fill='%23e2e8f0'/%3E%3Ctext x='60' y='45' font-size='12' text-anchor='middle' fill='%2394a3b8'%3EImage%3C/text%3E%3C/svg%3E"
            alt="preview"
          />
        );
      case "a":
        return <a {...(simProps as React.AnchorHTMLAttributes<HTMLAnchorElement>)} href="#">{value.content}</a>;
      case "button":
        return <button {...(simProps as React.ButtonHTMLAttributes<HTMLButtonElement>)} type="button">{value.content}</button>;
      case "p":
        return <p {...(simProps as React.HTMLAttributes<HTMLParagraphElement>)}>{value.content}</p>;
      case "h1":
      case "h2":
      case "h3":
        return React.createElement(value.value, simProps as React.HTMLAttributes<HTMLHeadingElement>, value.content);
      case "span":
        return <span {...(simProps as React.HTMLAttributes<HTMLSpanElement>)}>{value.content}</span>;
      case "section":
      case "article":
        return React.createElement(value.value, simProps as React.HTMLAttributes<HTMLElement>, value.content);
      default:
        return <div {...(simProps as React.HTMLAttributes<HTMLDivElement>)}>{value.content}</div>;
    }
  };

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
          padding: 24,
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
        <div style={{ display: "inline-flex", flexDirection: "column", maxWidth: "100%" }}>
          {renderPreview()}
        </div>
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
}
