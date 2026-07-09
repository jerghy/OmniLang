// components/design/OmniViewElement.tsx
// Standalone React.memo component for rendering a single view element.
// Children lookups use O(1) Map.get() — no closure-per-child or Array.find() overhead.

import React from "react";
import { resolveTag } from "./render-utils";
import type { ElementData, ViewElementData } from "../../core/types/element";

export interface OmniViewElementProps {
  element: ViewElementData;
  expandedElements: ReadonlyMap<string, ElementData>;
  customChildIds: ReadonlySet<string>;
  selectedIds: ReadonlySet<string>;
  inlineStyleMap: ReadonlyMap<string, Record<string, string>>;
  /** WASM-resolved class names per element id. When available, className is used. */
  classMap?: ReadonlyMap<string, string[]> | null;
  onSelect: (id: string) => void;
  onDoubleClick: (id: string) => void;
}

export const OmniViewElement = React.memo(function OmniViewElement({
  element,
  expandedElements,
  customChildIds,
  selectedIds,
  inlineStyleMap,
  classMap,
  onSelect,
  onDoubleClick,
}: OmniViewElementProps) {
  const isInsideCustom = customChildIds.has(element.id);
  const isSelected = !isInsideCustom && selectedIds.has(element.id);

  // WASM class names take priority over inline styles for design classes
  const wasmClasses = classMap?.get(element.id);
  const className = wasmClasses ? wasmClasses.join(" ") : undefined;

  const inlineStyles = inlineStyleMap.get(element.id) ?? {};

  const style: React.CSSProperties = {
    outline: isSelected ? "2px solid #7C3AED" : "none",
    outlineOffset: "-2px",
    cursor: isInsideCustom ? "default" : "pointer",
    // When WASM classes are active, skip inline styles (they are from design classes)
    ...(className ? {} : inlineStyles),
  };

  const tagName = resolveTag(element.tag);

  const handleClick = isInsideCustom
    ? undefined
    : (e: React.MouseEvent) => {
        e.stopPropagation();
        onSelect(element.id);
      };

  const handleDoubleClick = isInsideCustom
    ? undefined
    : (e: React.MouseEvent) => {
        e.stopPropagation();
        onDoubleClick(element.id);
      };

  // Resolve children: O(1) Map.get() per child, no Array.find() fallback
  const childElements = (element.children || [])
    .map((cid) => expandedElements.get(cid))
    .filter((e): e is ViewElementData => e !== undefined && "children" in e && "tag" in e)
    .map((childEl) => (
      <OmniViewElement
        key={childEl.id}
        element={childEl}
        expandedElements={expandedElements}
        customChildIds={customChildIds}
        selectedIds={selectedIds}
        inlineStyleMap={inlineStyleMap}
        classMap={classMap}
        onSelect={onSelect}
        onDoubleClick={onDoubleClick}
      />
    ));

  return React.createElement(
    tagName,
    {
      key: element.id,
      "data-pid": element.id,
      className,
      style,
      onClick: handleClick,
      onDoubleClick: handleDoubleClick,
    },
    element.props?.label != null ? String(element.props.label) : null,
    ...childElements,
  );
});
