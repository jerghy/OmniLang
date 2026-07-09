// components/design/OmniViewElement.tsx
// React.memo component for rendering a single element tree node.
// Uses stable nid as React key for zero-flicker reconciliation.

import React from "react";
import type { ElementTreeNode } from "./useWasmCompile";

export interface OmniViewElementProps {
  node: ElementTreeNode;
  tag: string;
  className?: string | undefined;
  isSelected: boolean;
  onSelect: (nid: string) => void;
  onDoubleClick: (nid: string) => void;
  /** Passed from OmniViewElementTree for CSS resolution (unused here; handled by Task 4). */
  classMap?: ReadonlyMap<string, string[]>;
  /** Passed from OmniViewElementTree for tag resolution. */
  tagMap?: ReadonlyMap<string, string>;
}

export const OmniViewElement = React.memo(function OmniViewElement({
  node,
  tag,
  isSelected,
  onSelect,
  onDoubleClick,
}: OmniViewElementProps) {
  const style: React.CSSProperties = {
    outline: isSelected ? "2px solid #7C3AED" : "none",
    outlineOffset: "-2px",
    cursor: "pointer",
  };

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onSelect(node.nid);
  };

  const handleDoubleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onDoubleClick(node.nid);
  };

  return React.createElement(
    tag,
    {
      key: node.nid,
      "data-nid": node.nid,
      className: node.tag === "#text" ? undefined : undefined, // className set by parent via CSS
      style,
      onClick: handleClick,
      onDoubleClick: handleDoubleClick,
    },
    node.text,
    ...node.children.map((child) => (
      <OmniViewElement
        key={child.nid}
        node={child}
        tag={child.tag === "#text" ? "span" : child.tag}
        isSelected={false}
        onSelect={onSelect}
        onDoubleClick={onDoubleClick}
      />
    )),
  );
});
