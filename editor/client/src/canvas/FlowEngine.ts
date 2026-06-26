import type { ElementData, WireData } from "../core/types";

export interface ExecutionStep {
  elementId: string;
  tag: string;
  /** Data inputs gathered from incoming data edges */
  inputs: Record<string, unknown>;
  /** Flow order index */
  order: number;
}

export interface ExecutionResult {
  steps: ExecutionStep[];
  /** Data outputs produced (elementId → port → value) */
  outputs: Map<string, Map<string, unknown>>;
  /** Nodes that are unreachable from any event source */
  unreachable: string[];
}

/**
 * Tags that serve as execution roots (event sources).
 */
const EVENT_TAGS = new Set(["onPress", "onChange", "onHover", "onMount"]);

/**
 * Execute the flow graph for a given set of elements and wires.
 * Returns ordered execution steps with gathered data inputs.
 *
 * Algorithm:
 * 1. Find all event nodes (execution roots)
 * 2. For each event node, BFS through flow-out edges to build execution DAG
 * 3. Data inputs are gathered from incoming data wires at each step
 */
export function executeFlow(
  elements: ElementData[],
  wires: WireData[],
): ExecutionResult {
  const elMap = new Map(elements.map(e => [e.id, e]));
  const outputs = new Map<string, Map<string, unknown>>();
  const steps: ExecutionStep[] = [];
  const visited = new Set<string>();

  // Find all event nodes (execution roots)
  const eventNodes = elements.filter(
    e => e.nodeCategory === "event" || EVENT_TAGS.has(e.tag),
  );

  if (eventNodes.length === 0) {
    return {
      steps: [],
      outputs,
      unreachable: elements.filter(e => e.nodeType === "codeblock").map(e => e.id),
    };
  }

  // For each event node, trace flow graph via BFS
  for (const eventNode of eventNodes) {
    const queue: string[] = [eventNode.id];
    const localVisited = new Set<string>();
    let order = 0;

    while (queue.length > 0) {
      const currentId = queue.shift()!;
      if (localVisited.has(currentId)) continue;
      localVisited.add(currentId);
      visited.add(currentId);

      const current = elMap.get(currentId);
      if (!current) continue;

      // Gather data inputs from incoming data wires
      const inputs: Record<string, unknown> = {};
      const incomingDataWires = wires.filter(
        w => w.toId === currentId,
      );
      for (const w of incomingDataWires) {
        const sourceOutputs = outputs.get(w.fromId);
        if (sourceOutputs?.has(w.fromPort)) {
          inputs[w.toPort] = sourceOutputs.get(w.fromPort);
        }
      }

      steps.push({
        elementId: current.id,
        tag: current.tag,
        inputs,
        order: order++,
      });

      // Follow flow-out edges to next nodes
      const flowOutWires = wires.filter(
        w => w.fromId === currentId,
      );
      for (const w of flowOutWires) {
        const toEl = elMap.get(w.toId);
        if (!toEl) continue;
        const toPort = toEl.ports?.find(p => p.name === w.toPort);
        const fromPort = current.ports?.find(p => p.name === w.fromPort);
        // Only follow if this is a flow connection
        if (fromPort?.portType === "flow" || toPort?.portType === "flow") {
          queue.push(w.toId);
        }
      }
    }
  }

  // Find unreachable code blocks
  const unreachable = elements
    .filter(e => e.nodeType === "codeblock" && !visited.has(e.id))
    .map(e => e.id);

  return { steps, outputs, unreachable };
}

/**
 * Validate the flow graph: detect cycles and orphan flow inputs.
 */
export function validateFlow(
  elements: ElementData[],
  wires: WireData[],
): { valid: boolean; cycles: string[][]; orphanFlowIns: string[] } {
  // Build flow adjacency list
  const flowAdj = new Map<string, string[]>();
  for (const w of wires) {
    const fromEl = elements.find(e => e.id === w.fromId);
    const toEl = elements.find(e => e.id === w.toId);
    const fromPort = fromEl?.ports?.find(p => p.name === w.fromPort);
    const toPort = toEl?.ports?.find(p => p.name === w.toPort);
    if (fromPort?.portType === "flow" || toPort?.portType === "flow") {
      const list = flowAdj.get(w.fromId) || [];
      list.push(w.toId);
      flowAdj.set(w.fromId, list);
    }
  }

  // DFS cycle detection
  const cycles: string[][] = [];
  const WHITE = 0, GRAY = 1, BLACK = 2;
  const color = new Map<string, number>();
  for (const el of elements) color.set(el.id, WHITE);

  function dfs(nodeId: string, path: string[]): boolean {
    const c = color.get(nodeId);
    if (c === GRAY) {
      cycles.push([...path, nodeId]);
      return true; // found a cycle, but continue checking
    }
    if (c === BLACK) return false;
    color.set(nodeId, GRAY);
    path.push(nodeId);
    const neighbors = flowAdj.get(nodeId) || [];
    for (const n of neighbors) {
      dfs(n, path);
    }
    path.pop();
    color.set(nodeId, BLACK);
    return false;
  }

  for (const el of elements) {
    if (color.get(el.id) === WHITE) dfs(el.id, []);
  }

  // Find action/event nodes with unconnected flow-in ports
  const orphanFlowIns = elements
    .filter(e => {
      const flowInPorts = (e.ports || []).filter(
        p => p.direction === "in" && p.portType === "flow",
      );
      if (flowInPorts.length === 0) return false;
      return !wires.some(w =>
        w.toId === e.id && flowInPorts.some(p => p.name === w.toPort),
      );
    })
    .map(e => e.id);

  return {
    valid: cycles.length === 0,
    cycles,
    orphanFlowIns,
  };
}
