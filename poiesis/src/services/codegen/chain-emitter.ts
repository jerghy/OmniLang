// poiesis/src/services/codegen/chain-emitter.ts
// Converts code graph elements and wires to @chain/@node/@wire annotation blocks.
// EmitChains maps each lowered function to a ChainBlock describing its subgraph.

import type { ElementData, WireData } from "../../core/types";
import type { CodeTab } from "../../core/types";
import type { ChainBlock, LoweredCode } from "./types";
import { emitPidAnnotation } from "./annotation-registry";
import type { CodeFlowGraph } from "../../core/types/project-json";
import type { CodeblockElementData } from "../../core/types/element";

// ── Node attribute definitions ────────────────────────────────────

interface AttrDef {
  required: string[];
  optional: string[];
}

const NODE_ATTRS: Record<string, AttrDef> = {
  literal: { required: ["value"], optional: [] },
  signal: { required: ["name"], optional: ["type", "init"] },
  getVariable: { required: ["name"], optional: [] },
  setVariable: { required: ["name"], optional: [] },
  add: { required: [], optional: [] },
  sub: { required: [], optional: [] },
  mul: { required: [], optional: [] },
  div: { required: [], optional: [] },
  compare: { required: ["op"], optional: [] },
  and: { required: [], optional: [] },
  or: { required: [], optional: [] },
  not: { required: [], optional: [] },
  concat: { required: [], optional: [] },
  return: { required: [], optional: [] },
  customFn: { required: ["fnName"], optional: ["returnType"] },
  store: { required: ["name"], optional: [] },
  computed: { required: ["name"], optional: ["returnType"] },
  fn: { required: ["source"], optional: [] },
  updateUIProperty: { required: ["targetId", "property"], optional: [] },
};

// ── Helpers ───────────────────────────────────────────────────────

/** Build a map from element ID to ElementData. */
function buildElMap(elements: ElementData[]): Map<string, ElementData> {
  return new Map(elements.map((e) => [e.id, e]));
}

/**
 * BFS from a starting node ID to collect all reachable nodes and wires.
 */
function collectReachable(
  fnNodeId: string,
  elMap: Map<string, ElementData>,
  allWires: WireData[],
): { nodes: ElementData[]; wires: WireData[] } {
  const visitedNodes = new Set<string>();
  const collectedWires: WireData[] = [];
  const collectedWireIds = new Set<string>();
  const queue = [fnNodeId];

  visitedNodes.add(fnNodeId);

  while (queue.length > 0) {
    const current = queue.shift()!;

    for (const w of allWires) {
      // Check if this wire connects to/from the current node
      if (w.fromId === current || w.toId === current) {
        // O(1) dedup via Set (was O(n²) with Array.some)
        if (!collectedWireIds.has(w.id)) {
          collectedWireIds.add(w.id);
          collectedWires.push(w);
        }

        // Enqueue the neighbor if not visited
        const neighbor = w.fromId === current ? w.toId : w.fromId;
        if (!visitedNodes.has(neighbor) && elMap.has(neighbor)) {
          visitedNodes.add(neighbor);
          queue.push(neighbor);
        }
      }
    }
  }

  const nodes: ElementData[] = [];
  for (const id of visitedNodes) {
    const el = elMap.get(id);
    if (el) nodes.push(el);
  }

  return { nodes, wires: collectedWires };
}

/**
 * Resolve a named attribute value from an element.
 * Checks config first (for CustomFunctionMeta attrs), then props.
 */
function resolveAttr(el: ElementData, attrName: string): unknown {
  // Must have tag to be a codeblock or view element
  if (!("tag" in el)) return undefined;
  // For customFn attributes stored in config
  if (el.tag === "customFn" && "config" in el && el.config) {
    const config = el.config as Record<string, unknown>;
    if (attrName === "fnName") return config.fnName;
    if (attrName === "returnType") return config.fnReturnType;
  }

  // For signal "type" attr, look up typeAnnotation prop
  if (el.tag === "signal" && attrName === "type") {
    return el.props.typeAnnotation ?? el.props.type;
  }

  // For signal "init" attr, look up init prop
  if (el.tag === "signal" && attrName === "init") {
    return el.props.init ?? el.props.default;
  }

  // Default: look up in props, then fallback to config for any node type
  if (attrName in el.props) return el.props[attrName];
  if ("config" in el && el.config && attrName in el.config) return (el.config as Record<string, unknown>)[attrName];
  return undefined;
}

/**
 * Format an attribute value for the @node line.
 *
 * For literal `value` attrs, strings are quoted to distinguish them
 * as actual string values. All other attribute values are bare tokens.
 */
function formatAttrValue(value: unknown, isLiteralValue: boolean): string {
  if (typeof value === "string") {
    if (isLiteralValue) {
      return `"${value}"`;
    }
    return value;
  }
  return String(value ?? "");
}

/**
 * Format a single element as an @node comment line.
 */
function formatNode(el: ElementData): string {
  if (!("tag" in el)) return "";
  const parts: string[] = ["//   @node", el.id, el.tag];

  const attrs = NODE_ATTRS[el.tag];
  if (attrs) {
    // Required attrs
    for (const attr of attrs.required) {
      const value = resolveAttr(el, attr);
      if (value !== undefined && value !== null) {
        // Only literal.value is a code value; other attrs are bare identifiers
        const isLiteralValue = el.tag === "literal" && attr === "value";
        parts.push(`${attr}=${formatAttrValue(value, isLiteralValue)}`);
      }
    }
    // Optional attrs
    for (const attr of attrs.optional) {
      const value = resolveAttr(el, attr);
      if (value !== undefined && value !== null) {
        const isLiteralValue = el.tag === "literal" && attr === "value";
        parts.push(`${attr}=${formatAttrValue(value, isLiteralValue)}`);
      }
    }
  }

  // Position
  if (el.position) {
    parts.push(`pos=${el.position.x},${el.position.y}`);
  }

  return parts.join(" ");
}

/**
 * Format a single wire as an @wire comment line.
 * fn nodes (customFn) skip port name on the from side.
 */
function formatWire(w: WireData, elMap: Map<string, ElementData>): string {
  const src = elMap.get(w.fromId);

  // fn/customFn nodes skip port name on the from side
  if (src && "tag" in src && src.tag === "customFn") {
    return `//   @wire ${w.id} ${w.fromId} -> ${w.toId}.${w.toPort}`;
  }

  return `//   @wire ${w.id} ${w.fromId}.${w.fromPort} -> ${w.toId}.${w.toPort}`;
}

/**
 * S5 validation: check that getVariable/setVariable names exist in tab variables.
 * Returns warning strings for invalid references.
 */
function validateScope(
  nodes: ElementData[],
  variables: CodeTab["variables"],
): string[] {
  const validNames = new Set(variables.map((v) => v.name));
  const warnings: string[] = [];

  for (const el of nodes) {
    if (!("tag" in el)) continue;
    if (el.tag === "getVariable" || el.tag === "setVariable") {
      const name = el.props.name as string | undefined;
      if (name && !validNames.has(name)) {
        warnings.push(
          `// warning: ${el.tag} "${name}" not found in tab variables (element ${el.id})`,
        );
      }
    }
  }

  return warnings;
}

// ── Main export ───────────────────────────────────────────────────

/**
 * Convert code graph elements+wires to @chain/@node/@wire annotation blocks.
 *
 * For each customFn in `functions`, find the matching element in `tab.graph.elements`
 * by config.fnName (or props.name). BFS from the fnNode to collect all reachable
 * nodes and wires, then format them as annotation lines.
 *
 * @returns Map keyed by function name, each value is a ChainBlock
 */
export function emitChains(
  tab: CodeTab,
  functions: LoweredCode["functions"],
): Map<string, ChainBlock> {
  const elMap = buildElMap(tab.graph.elements);
  const result = new Map<string, ChainBlock>();

  for (const fn of functions) {
    // Find matching customFn element
    const fnEl = tab.graph.elements.find(
      (el) =>
        "tag" in el && "config" in el && el.tag === "customFn" &&
        (el.config as Record<string, unknown>)?.fnName === fn.name,
    );

    if (!fnEl) continue; // No matching graph element — skip

    // BFS to collect reachable nodes and wires
    const { nodes, wires } = collectReachable(fnEl.id, elMap, tab.graph.wires);

    // Format node lines
    const nodeLines: string[] = [];
    for (const n of nodes) {
      nodeLines.push(formatNode(n));
    }

    // Format wire lines
    const wireLines: string[] = [];
    for (const w of wires) {
      wireLines.push(formatWire(w, elMap));
    }

    // S5 validation
    const warnings = validateScope(nodes, tab.variables);
    const allLines = [...warnings, ...nodeLines, ...wireLines];

    result.set(fn.name, {
      functionName: fn.name,
      nodeLines: allLines.filter((l) => l.includes("@node") || l.includes("warning")),
      wireLines: allLines.filter((l) => l.includes("@wire")),
    });
  }

  return result;
}

// ── New exports: code flow graph JSON persistence ─────────────────────

/**
 * Build CodeFlowGraph JSON structure from parsed code flow data.
 * Converts the internal parsed format (from ParseResult.codeFlowGraphs)
 * to the merge-friendly project.json persistence format.
 *
 * Called during save to populate project.json codeFlowGraphs field.
 *
 * @param parsedGraphs  Code flow graphs keyed by fn/action name, each with
 *                      elements (CodeblockElementData[]) and wires (WireData[]).
 * @returns             CodeFlowGraphs keyed by fn/action name, suitable for
 *                      ProjectJson.codeFlowGraphs.
 */
export function buildCodeFlowGraphs(
  parsedGraphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>,
): Record<string, CodeFlowGraph> {
  const graphs: Record<string, CodeFlowGraph> = {};

  for (const [fnName, { elements, wires }] of Object.entries(parsedGraphs)) {
    const graph: CodeFlowGraph = {
      pid: fnName,
      nodes: {},
      wires: {},
    };

    for (const el of elements) {
      graph.nodes[el.id] = { nodeType: el.tag };
    }

    for (const w of wires) {
      graph.wires[w.id] = {
        from: w.fromId,
        fromPort: w.fromPort,
        to: w.toId,
        toPort: w.toPort,
      };
    }

    graphs[fnName] = graph;
  }

  return graphs;
}

/**
 * Format chain block node IDs as // @pid annotation lines.
 * Replaces the old // @chain/@node/@wire comment block format.
 * Each node line in the chain produces a // @pid:<id> annotation.
 *
 * @param chain  The ChainBlock whose nodeLines are parsed for node IDs.
 * @returns      A string of // @pid:<id> lines, one per node, joined by newline.
 */
export function formatChainAsPidAnnotations(chain: ChainBlock): string {
  const lines: string[] = [];
  for (const nodeLine of chain.nodeLines) {
    const match = nodeLine.match(/@node\s+(\S+)/);
    if (match) {
      lines.push(emitPidAnnotation(match[1]));
    }
  }
  return lines.join("\n");
}
