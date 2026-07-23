// components/design/OmniViewElement.tsx
// React.memo component for rendering a single element tree node.
// Uses stable nid as React key for zero-flicker reconciliation.
//
// Custom components (identified by tag containing a hyphen, e.g. "my-card")
// act as opaque black boxes: clicking anywhere inside the component selects
// the component itself, not its internal children.

import React from "react";
import type { ElementTreeNode } from "./useWasmCompile";
import { _lastClickPos } from "./omni-view-utils";

export interface OmniViewElementProps {
  node: ElementTreeNode;
  tag: string;
  className?: string | undefined;
  isSelected: boolean;
  selectedNid?: string | null;
  onSelect: (nid: string) => void;
  onDoubleClick: (nid: string) => void;
  /** Passed from OmniViewElementTree for CSS resolution. */
  classMap?: ReadonlyMap<string, string[]>;
  /** Passed from OmniViewElementTree for tag resolution. */
  tagMap?: ReadonlyMap<string, string>;
  /**
   * When set, this element is a descendant of a custom component.
   * Click events select the ancestor component nid instead of this node's nid.
   */
  parentComponentNid?: string | null;
}

export const OmniViewElement = React.memo(function OmniViewElement({
  node,
  tag,
  className,
  isSelected,
  selectedNid,
  onSelect,
  onDoubleClick,
  classMap,
  tagMap,
  parentComponentNid,
}: OmniViewElementProps) {
  // Custom components are identified by a hyphen in the HTML tag
  // (e.g. "my-card"). Per spec, standard HTML elements never contain hyphens.
  const isCustomComponent = tag.includes("-");

  // Text nodes render as <span> but should not intercept clicks —
  // they pass selection up to their parent element.
  const isTextNode = node.tag === "#text";

  // Nid to propagate to children: if THIS element is a custom component,
  // its descendants should select this component, not themselves.
  // Nested custom components form their own boundary.
  const componentNid = isCustomComponent ? node.nid : parentComponentNid;

  // Effective nid for click selection.
  // A custom component always selects itself (even if inside another component).
  // Regular elements inside a component select their nearest component ancestor.
  const effectiveNid = isCustomComponent ? node.nid : (parentComponentNid ?? node.nid);

  // Only the effective selection target shows the outline — not every child.
  const showOutline = node.nid === effectiveNid && effectiveNid === selectedNid;

  // Merge WASM-compiled inline styles with editor chrome styles
  const inlineCss: React.CSSProperties = {};
  if (node.inlineStyle) {
    for (const [key, val] of Object.entries(node.inlineStyle)) {
      (inlineCss as any)[key] = val;
    }
  }

  const style: React.CSSProperties = {
    display: isCustomComponent ? "block" : undefined,
    outline: showOutline ? "2px solid #7C3AED" : "none",
    outlineOffset: "-2px",
    cursor: isTextNode ? undefined : "pointer",
    ...inlineCss,
  };

  // Text elements don't intercept clicks — events bubble to the parent element.
  const clickHandlerProps = isTextNode
    ? {}
    : {
        onClick: (e: React.MouseEvent) => {
          _lastClickPos = { x: e.clientX, y: e.clientY };
          e.stopPropagation();
          onSelect(effectiveNid);
        },
        onDoubleClick: (e: React.MouseEvent) => {
          e.stopPropagation();
          onDoubleClick(effectiveNid);
        },
      };

  return React.createElement(
    tag,
    {
      key: node.nid,
      "data-nid": node.nid,
      className,
      style,
      ...clickHandlerProps,
    },
    node.text,
    ...(node.children || []).map((child) => (
      <OmniViewElement
        key={child.nid}
        node={child}
        tag={child.tag === "#text" ? "span" : child.tag}
        className={(classMap?.get(child.nid) || []).join(" ")}
        isSelected={child.nid === selectedNid}
        selectedNid={selectedNid}
        classMap={classMap}
        tagMap={tagMap}
        onSelect={onSelect}
        onDoubleClick={onDoubleClick}
        parentComponentNid={componentNid}
      />
    )),
  );
});
