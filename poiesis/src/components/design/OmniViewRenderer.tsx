// components/design/OmniViewRenderer.tsx
// React direct renderer for component preview (replaces iframe).
// Walks the element tree from the store and renders real DOM elements.

import React from "react";
import { usePoiesisStore } from "../../store";
import type { ElementData } from "../../core/types/element";
import type { StyleValueSegment } from "../../core/types/style";

/** Check if a segment is a plain text value segment. */
function isTextSegment(
  s: StyleValueSegment
): s is StyleValueSegment & { type: "text"; value: string } {
  return s.type === "text";
}

/** Map OmniLang tags to HTML tag names — static, never changes. */
const tagMap: Record<string, string> = {
  text: "span",
  button: "button",
  input: "input",
  img: "img",
  a: "a",
  p: "p",
  h1: "h1",
  h2: "h2",
  h3: "h3",
  section: "section",
  article: "article",
  header: "header",
  footer: "footer",
  nav: "nav",
  main: "main",
  ul: "ul",
  ol: "ol",
  li: "li",
  label: "label",
  textarea: "textarea",
  select: "select",
};

function resolveTag(tag: string): string {
  return tagMap[tag] || tag || "div";
}

export function OmniViewRenderer() {
  const elements = usePoiesisStore((s) => s.elements);
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);

  const renderElement = (el: ElementData): React.ReactNode => {
    const isSelected = selectedIds.includes(el.id);
    const children = el.children
      .map((cid) => elements.find((e) => e.id === cid))
      .filter((e): e is ElementData => e !== undefined);

    // Resolve inline styles from store
    const inlineStyles: Record<string, string> = {};
    if (el.style?.inlineProperties) {
      for (const prop of el.style.inlineProperties) {
        const textSeg = prop.segments.find(isTextSegment);
        if (textSeg) {
          inlineStyles[prop.name] = textSeg.value;
        }
      }
    }

    const style: React.CSSProperties = {
      outline: isSelected ? "2px solid #7C3AED" : "none",
      outlineOffset: "-2px",
      cursor: "pointer",
      ...inlineStyles,
    };

    const tagName = resolveTag(el.tag);

    const handleClick = (e: React.MouseEvent) => {
      e.stopPropagation();
      selectElement(el.id);
    };

    const handleDoubleClick = (e: React.MouseEvent) => {
      e.stopPropagation();
      toggleCodeFlow(el.id);
    };

    return React.createElement(
      tagName,
      {
        key: el.id,
        "data-pid": el.id,
        style,
        onClick: handleClick,
        onDoubleClick: handleDoubleClick,
      },
      el.props?.label != null ? String(el.props.label) : null,
      ...children.map(renderElement)
    );
  };

  const rootElements = elements.filter((e) => !e.parentId);

  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        overflow: "auto",
        padding: 8,
        boxSizing: "border-box",
      }}
    >
      {rootElements.length === 0 ? (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            color: "#94a3b8",
            fontSize: 13,
          }}
        >
          空画布 — 从组件面板拖入元素
        </div>
      ) : (
        rootElements.map(renderElement)
      )}
    </div>
  );
}
