// components/design/OmniViewRenderer.tsx
// React direct renderer — walks the element tree, expanding custom components
// inline so their internal elements render correctly. Elements inside custom
// components are non-interactive (cannot be selected individually).

import React, { useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { useElements, useRootOrder, useSelectedElementIds, useComponentDefs } from "../../store/hooks";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { StyleValueSegment } from "../../core/types/style";
import type { ComponentDef } from "../../core/types/project-json";
import type { DesignToken } from "../../core/types/design";

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
  if (!("tag" in el)) return styles;
  const htmlTag = resolveTag(el.tag);
  if (blockTags.has(htmlTag)) {
    styles.display = "block";
    if (!el.props?.label && (!("children" in el) || !el.children || el.children.length === 0)) {
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

/** Resolve a style value from segments, following var-ref through tokens. */
function resolveSegmentsValue(
  segments: StyleValueSegment[],
  tokens: DesignToken[],
): string | null {
  for (const seg of segments) {
    if (seg.type === "text") return seg.value;
    if (seg.type === "var-ref") {
      const token = tokens.find((t) => t.name === seg.name);
      if (token) return token.value;
    }
  }
  return null;
}

export function OmniViewRenderer() {
  const elements = useElements();
  const rootOrder = useRootOrder();
  const selectedIds = useSelectedElementIds();
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  // Subscribe to componentDefs so the renderer re-expands when a custom
  // component's definition changes (e.g. after editing and saving MyCard.omni).
  const componentDefs = useComponentDefs();
  // Subscribe to activeDesign so class/style changes reflect on canvas immediately.
  const activeDesign = usePoiesisStore((s) => s.activeDesign);

  // Build expanded element map + track IDs that are inside custom components
  const { expandedElements, customChildIds } = useMemo(() => {
    const registry: Record<string, ComponentDef> = componentDefs ?? {};
    const map = new Map<string, ElementData>(elements.map((e) => [e.id, e]));
    const innerIds = new Set<string>();

    const expanded = new Set<string>();
    let foundNew = true;
    while (foundNew) {
      foundNew = false;
      for (const [id, el] of [...map]) {
        if (!("tag" in el)) continue; // skip group elements
        const compRef: string | undefined = "componentRef" in el ? (el.componentRef || (registry[el.tag] ? el.tag : undefined)) : (registry[el.tag] ? el.tag : undefined);
        if (!compRef || expanded.has(id)) continue;
        expanded.add(id);
        const def = registry[compRef];
        const defElements = def && Array.isArray((def as unknown as Record<string, unknown>).elements)
          ? (def as unknown as Record<string, unknown>).elements as ElementData[]
          : null;
        if (!defElements || defElements.length === 0) continue;

        const idMap = new Map<string, string>();
        let idx = 0;
        for (const defEl of defElements) {
          idMap.set(defEl.id, `${id}__${idx++}`);
        }

        const newChildIds: string[] = [];
        for (const defEl of defElements) {
          if (!("children" in defEl && "tag" in defEl)) continue;
          const newId = idMap.get(defEl.id)!;
          // Mark as inside custom component (non-selectable)
          innerIds.add(newId);
          const isRoot = !defEl.parentId || !defElements.some((d) => d.id === defEl.parentId);
          const cloned: ViewElementData = {
            id: newId,
            parentId: isRoot ? id : (idMap.get(defEl.parentId || "") || defEl.parentId),
            nodeType: "view",
            tag: defEl.tag || "div",
            props: defEl.props || {},
            children: (defEl.children || []).map((cid) => idMap.get(cid) || cid),
            // Copy style (classRefs + inlineProperties) so canvas preview reflects
            // design-system class bindings and inline styles applied to the component.
            style: defEl.style ? { ...defEl.style, classRefs: [...(defEl.style.classRefs ?? [])], inlineProperties: [...(defEl.style.inlineProperties ?? [])] } : undefined,
          };
          map.set(newId, cloned);
          if (isRoot) newChildIds.push(newId);
        }

        const updated = { ...map.get(id)! };
        if ("children" in updated) {
          updated.children = [...updated.children, ...newChildIds];
        }
        map.set(id, updated);
        foundNew = true;
      }
    }

    return { expandedElements: map, customChildIds: innerIds };
  }, [elements, componentDefs]);

  const renderElement = (el: ElementData): React.ReactNode => {
    if (!("children" in el && "tag" in el)) return null;

    const expanded = expandedElements.get(el.id);
    if (expanded && "children" in expanded && "tag" in expanded) {
      el = expanded;
    }

    const isInsideCustom = customChildIds.has(el.id);
    const isSelected = !isInsideCustom && selectedIds.includes(el.id);

    const children = el.children
      .map((cid: string) => expandedElements.get(cid) || elements.find((e: ElementData) => e.id === cid))
      .filter((e): e is ElementData => e !== undefined);

    const defaultStyles = getDefaultInlineStyles(el);
    const inlineStyles: Record<string, string> = { ...defaultStyles };
    if (el.style?.inlineProperties) {
      for (const prop of el.style.inlineProperties) {
        const textSeg = prop.segments.find(isTextSegment);
        if (textSeg) inlineStyles[prop.name] = textSeg.value;
      }
    }

    // Resolve class bindings: merge base class properties into inlineStyles
    if (el.style?.classRefs && activeDesign) {
      for (const ref of el.style.classRefs) {
        // Strip namespace prefix (e.g. "AppDesign.Card" → "Card")
        const className = ref.includes(".") ? ref.split(".").slice(1).join(".") : ref;
        const classDef = activeDesign.classes.find((c) => c.name === className);
        if (!classDef) continue;
        for (const prop of classDef.properties) {
          const resolved = resolveSegmentsValue(prop.segments, activeDesign.tokens);
          if (resolved != null) {
            inlineStyles[prop.name] = resolved;
          }
        }
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
