// components/design/OmniViewElementTree.tsx
// Renders the WASM-produced element tree using React.createElement.
// CSS is injected via <style> tags managed by useCssScopePool (Task 4).

import { useMemo } from "react";
import { OmniViewElement } from "./OmniViewElement";
import type { OmniViewOutput } from "./useWasmCompile";

export interface OmniViewElementTreeProps {
  output: OmniViewOutput;
  selectedNid: string | null;
  onSelectNid: (nid: string) => void;
  onDoubleClickNid: (nid: string) => void;
}

export function OmniViewElementTree({
  output,
  selectedNid,
  onSelectNid,
  onDoubleClickNid,
}: OmniViewElementTreeProps) {
  // serde_wasm_bindgen serializes HashMap as JS Map, not plain object
  const classMap = useMemo(
    () => output.element_classes instanceof Map
      ? output.element_classes as Map<string, string[]>
      : new Map(Object.entries(output.element_classes)),
    [output.element_classes],
  );

  const tagMap = useMemo(
    () => output.element_tags instanceof Map
      ? output.element_tags as Map<string, string>
      : new Map(Object.entries(output.element_tags)),
    [output.element_tags],
  );

  if (output.element_tree.length === 0) {
    return (
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "center",
        height: "100%", color: "#94a3b8", fontSize: 13,
      }}>
        空画布 — 从组件面板拖入元素
      </div>
    );
  }

  return (
    <>
      {output.element_tree.map((node) => (
        <OmniViewElement
          key={node.nid}
          node={node}
          tag={tagMap.get(node.nid) || node.tag}
          className={(classMap.get(node.nid) || []).join(" ")}
          isSelected={node.nid === selectedNid}
          selectedNid={selectedNid}
          classMap={classMap}
          onSelect={onSelectNid}
          onDoubleClick={onDoubleClickNid}
        />
      ))}
    </>
  );
}
