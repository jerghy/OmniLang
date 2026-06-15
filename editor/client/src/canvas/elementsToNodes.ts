/** Pure function: convert ElementData[] to React Flow Node[].
 *  Extracted from FlowCanvas.tsx for testability and readability. */

import type { Node } from "@xyflow/react";
import type { ElementData } from "../store/editor";
import { getNodeConfig } from "./nodeRegistry";
import { getAllPorts } from "./connectionValidators";
import type { BaseNodeData } from "./nodes/BaseNode";

export interface ElementsToNodesOptions {
  elements: ElementData[];
  wires: Array<{ fromId: string; toId: string }>;
  selectedElementIds: string[];
  /** Active node IDs for runtime execution highlighting. */
  activeNodeIds?: Set<string>;
}

export function elementsToNodes(opts: ElementsToNodesOptions): Node[] {
  const { elements, wires, selectedElementIds, activeNodeIds } = opts;

  const nodes: Node[] = [];
  const type = "codeblock"; // all nodes are codeblock type now (no viewEvent)

  for (const el of elements) {
    if (el.nodeType !== "codeblock") continue; // skip non-codeblock elements
    if (el.tag === "fn") continue; // fn nodes without ports are function definitions — not rendered

    const config = getNodeConfig(el.tag);
    const ports = getAllPorts(el);
    const name = String(el.props.name || el.props.label || el.tag);
    const pos = el.position || {
      x: 50,
      y: 50 + nodes.length * 120,
    };

    const nodeData: BaseNodeData = {
      elementId: el.id,
      tag: el.tag,
      label: name,
      typeAnnotation: el.typeAnnotation || (el.props.typeAnnotation as string) || undefined,
      outputValue: el.outputValue,
      ports,
      isFolded: el.isFolded,
    };

    // Runtime execution highlighting
    const isExecuting = activeNodeIds?.has(el.id);
    const style: React.CSSProperties = {};
    if (isExecuting) {
      style.border = "2px solid #10B981";
      style.boxShadow = "0 0 12px rgba(16, 185, 129, 0.6)";
      style.transition = "box-shadow 0.3s ease, border 0.3s ease";
    }

    nodes.push({
      id: el.id,
      type,
      position: pos,
      data: nodeData as unknown as Record<string, unknown>,
      selected: selectedElementIds.includes(el.id),
      style: Object.keys(style).length > 0 ? style : undefined,
    });
  }

  return nodes;
}
