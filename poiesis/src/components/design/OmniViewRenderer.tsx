// components/design/OmniViewRenderer.tsx
// React direct renderer — walks the element tree, expanding custom components
// inline so their internal elements render correctly. Elements inside custom
// components are non-interactive (cannot be selected individually).

import React, { useMemo } from "react";
import { usePoiesisStore } from "../../store";
import type { ElementData } from "../../core/types/element";
import type { StyleValueSegment } from "../../core/types/style";

function isTextSegment(
  s: StyleValueSegment
): s is StyleValueSegment & { type: "text"; value: string } {
  return s.type === "text";
}

const tagMap: Record<string, string> = {
  container: "div", flex: "div", grid: "div", card: "div",
  text: "span", heading: "h1", button: "button", input: "input",
  textarea: "textarea", select: "select", checkbox: "input",
  list: "ul", listitem: "li", image: "img", spinner: "div",
  progress: "progress", alert: "div", link: "a", form: "form",
  div: "div", span: "span", img: "img", a: "a", p: "p",
  h1: "h1", h2: "h2", h3: "h3", h4: "h4", h5: "h5", h6: "h6",
  section: "section", article: "article", header: "header",
  footer: "footer", nav: "nav", main: "main",
  ul: "ul", ol: "ol", li: "li", label: "label", hr: "hr",
};

const blockTags = new Set([
  "div", "section", "article", "header", "footer", "nav", "main",
  "ul", "ol", "li", "form", "h1", "h2", "h3", "h4", "h5", "h6", "p",
]);

function resolveTag(tag: string): string {
  return tagMap[tag.toLowerCase()] || tagMap[tag] || "div";
}

function getDefaultInlineStyles(el: ElementData): Record<string, string> {
  const styles: Record<string, string> = {};
  const htmlTag = resolveTag(el.tag);
  if (blockTags.has(htmlTag)) {
    styles.display = "block";
    if (!el.props?.label && (!el.children || el.children.length === 0)) {
      styles.minHeight = "32px";
    }
  }
  if (htmlTag === "span" && !el.props?.label) {
    styles.display = "inline-block";
    styles.minWidth = "32px";
    styles.minHeight = "20px";
  }
  return styles;
}

export function OmniViewRenderer() {
  const elements = usePoiesisStore((s) => s.elements);
  const rootOrder = usePoiesisStore((s) => s.rootOrder);
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  // Subscribe to componentDefs so the renderer re-expands when a custom
  // component's definition changes (e.g. after editing and saving MyCard.omni).
  const componentDefs = usePoiesisStore((s) => s.componentDefs);

  // Build expanded element map + track IDs that are inside custom components
  const { expandedElements, customChildIds } = useMemo(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const registry = componentDefs as Record<string, any> || {};
    const map = new Map<string, ElementData>(elements.map((e) => [e.id, e]));
    const innerIds = new Set<string>();

    const expanded = new Set<string>();
    let foundNew = true;
    while (foundNew) {
      foundNew = false;
      for (const [id, el] of [...map]) {
        const compRef: string | undefined = el.componentRef || (registry[el.tag] ? el.tag : undefined);
        if (!compRef || expanded.has(id)) continue;
        expanded.add(id);
        const def = registry[compRef];
        const defElements = def && Array.isArray((def as Record<string, unknown>).elements)
          ? (def as Record<string, unknown>).elements as ElementData[]
          : null;
        if (!defElements || defElements.length === 0) continue;

        const idMap = new Map<string, string>();
        let idx = 0;
        for (const defEl of defElements) {
          idMap.set(defEl.id, `${id}__${idx++}`);
        }

        const newChildIds: string[] = [];
        for (const defEl of defElements) {
          const newId = idMap.get(defEl.id)!;
          // Mark as inside custom component (non-selectable)
          innerIds.add(newId);
          const isRoot = !defEl.parentId || !defElements.some((d) => d.id === defEl.parentId);
          const cloned: ElementData = {
            ...defEl,
            id: newId,
            nodeType: "view",
            tag: defEl.tag || "div",
            props: defEl.props || {},
            parentId: isRoot ? id : (idMap.get(defEl.parentId || "") || defEl.parentId),
            children: (defEl.children || []).map((cid) => idMap.get(cid) || cid),
          };
          map.set(newId, cloned);
          if (isRoot) newChildIds.push(newId);
        }

        const updated = { ...map.get(id)! };
        updated.children = [...updated.children, ...newChildIds];
        map.set(id, updated);
        foundNew = true;
      }
    }

    return { expandedElements: map, customChildIds: innerIds };
  }, [elements, componentDefs]);

  const renderElement = (el: ElementData): React.ReactNode => {
    const expanded = expandedElements.get(el.id);
    if (expanded && expanded.children !== el.children) {
      el = expanded;
    }

    const isInsideCustom = customChildIds.has(el.id);
    const isSelected = !isInsideCustom && selectedIds.includes(el.id);

    const children = el.children
      .map((cid) => expandedElements.get(cid) || elements.find((e) => e.id === cid))
      .filter((e): e is ElementData => e !== undefined);

    const defaultStyles = getDefaultInlineStyles(el);
    const inlineStyles: Record<string, string> = { ...defaultStyles };
    if (el.style?.inlineProperties) {
      for (const prop of el.style.inlineProperties) {
        const textSeg = prop.segments.find(isTextSegment);
        if (textSeg) inlineStyles[prop.name] = textSeg.value;
      }
    }

    const style: React.CSSProperties = {
      outline: isSelected ? "2px solid #7C3AED" : "none",
      outlineOffset: "-2px",
      cursor: isInsideCustom ? "default" : "pointer",
      ...inlineStyles,
    };

    const tagName = resolveTag(el.tag);

    const handleClick = isInsideCustom ? undefined : (e: React.MouseEvent) => {
      e.stopPropagation();
      selectElement(el.id);
    };

    const handleDoubleClick = isInsideCustom ? undefined : (e: React.MouseEvent) => {
      e.stopPropagation();
      // Toggle: if already in code-flow mode, exit; otherwise enter for this element
      toggleCodeFlow(codeFlowVisible ? null : el.id);
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

  const rootElements = rootOrder
    .map((id) => elements.find((e) => e.id === id))
    .filter((e): e is ElementData => e !== undefined);

  return (
    <div
      data-renderer="omniview"
      style={{
        width: "100%", height: "100%", overflow: "auto",
        padding: 8, boxSizing: "border-box",
      }}
    >
      {rootElements.length === 0 ? (
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "center",
          height: "100%", color: "#94a3b8", fontSize: 13,
        }}>
          空画布 — 从组件面板拖入元素
        </div>
      ) : (
        rootElements.map(renderElement)
      )}
    </div>
  );
}
