// poiesis/src/services/codegen/graph-compiler.ts
// Compile code flow graph (nodes + wires) → OmniView fn body source string.
// Output includes // @pid:xxx annotations for round-trip stability.

import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { emitOps } from "./code-op-emitter";

interface GraphNode {
  element: CodeblockElementData;
  /** Successors connected via flow-out port (general control flow) */
  successors: string[];
  /**
   * Branch-specific successors keyed by branch port ID (e.g. "br0", "br1", "br2").
   * Used by condition nodes to route each branch to its own successor chain.
   */
  branchSuccessors: Map<string, string[]>;
}

/**
 * Build adjacency map: element ID → GraphNode with successors.
 *
 * Wire routing:
 * - `fromPort === "flow-out"` → stored in node.successors (post-condition / general)
 * - `fromPort.startsWith("br")` → stored in node.branchSuccessors by branch ID
 *   (e.g. "br0" → if, "br1" → elif, "br2" → else)
 */
function buildAdjacency(
  elements: CodeblockElementData[],
  wires: WireData[],
): Map<string, GraphNode> {
  const map = new Map<string, GraphNode>();
  for (const el of elements) {
    map.set(el.id, {
      element: el,
      successors: [],
      branchSuccessors: new Map(),
    });
  }
  for (const w of wires) {
    const node = map.get(w.fromId);
    if (!node) continue;

    if (w.fromPort === "flow-out") {
      node.successors.push(w.toId);
    } else if (w.fromPort.startsWith("br")) {
      const branchId = w.fromPort;
      const existing = node.branchSuccessors.get(branchId) || [];
      existing.push(w.toId);
      node.branchSuccessors.set(branchId, existing);
    }
    // Ignore data-flow wires (data-in / data-out) — they are not control flow
  }
  return map;
}

/** Resolve a stable PID from an element — prefers annotated pid, falls back to element id. */
function resolvePid(el: CodeblockElementData): string {
  return (el as { pid?: string }).pid || el.id;
}

/** Emit the // @pid:xxx annotation line for a node. */
function emitPidComment(pid: string): string {
  return `// @pid:${pid}\n`;
}

/**
 * Emit a single code flow node as OmniView source text.
 *
 * @param node    The current graph node to emit.
 * @param adj     Full adjacency map for recursive lookups.
 * @param indent  Current indentation level (number of 2-space units).
 * @param visited Set of already-visited element IDs (prevents infinite loops).
 * @returns       Emitted source text for this node and its successors.
 */
function emitNode(
  node: GraphNode,
  adj: Map<string, GraphNode>,
  indent: number,
  visited: Set<string>,
): string {
  if (visited.has(node.element.id)) return "";
  visited.add(node.element.id);

  const pad = "  ".repeat(indent);
  const pid = resolvePid(node.element);
  const tag = node.element.tag;

  switch (tag) {
    case "fn": {
      const name = (node.element.props?.name as string) || "unnamed";
      const params = (node.element.props?.params as string) || "";
      const returnType = (node.element.props?.returnType as string) || "";
      const ret = returnType ? ` -> ${returnType}` : "";
      let out = emitPidComment(pid);
      out += `fn ${name}(${params})${ret} {\n`;

      // Emit successors (the fn body)
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent + 1, visited);
      }

      // Also check branch successors for fn body nodes wired with branch ports
      for (const [, branchIds] of node.branchSuccessors) {
        for (const succId of branchIds) {
          const succ = adj.get(succId);
          if (succ) out += emitNode(succ, adj, indent + 1, visited);
        }
      }

      out += `}`;
      return out;
    }

    case "code-block": {
      // Use structured operations if available, fall back to raw source
      const operations = (node.element as any).operations as import("../../core/types/code-op").CodeOp[] | undefined;
      let out: string;

      if (operations && operations.length > 0) {
        out = emitPidComment(pid);
        out += emitOps(operations);
      } else {
        // Legacy: raw source text
        const source = node.element.source || "";
        out = emitPidComment(pid);
        const lines = source.split("\n");
        for (const line of lines) {
          out += `${pad}${line}\n`;
        }
      }

      // Emit successor (control flow continues)
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent, visited);
      }
      return out;
    }

    case "condition": {
      const branches = (node.element.props?.branches as Array<{
        id: string;
        type: "if" | "elif" | "else";
        condition: string;
      }>) || [];

      if (branches.length === 0) return "";

      let out = "";
      for (let i = 0; i < branches.length; i++) {
        const br = branches[i];

        // Determine the branch port ID: "br0", "br1", "br2", …
        // Use the branch's explicit id if available, otherwise compute from index.
        const brPortId = `br${i}`;

        const keyword =
          br.type === "if" ? "if"
          : br.type === "elif" ? "else if"
          : "else";
        const condStr = br.type !== "else" ? ` ${br.condition}` : "";

        // Emit @pid annotation only on the first branch line
        if (i === 0) {
          out += emitPidComment(pid);
        }
        out += `${pad}${keyword}${condStr} {\n`;

        // Emit only the successors wired to this specific branch port
        const branchSuccIds = node.branchSuccessors.get(brPortId) || [];
        for (const succId of branchSuccIds) {
          const succ = adj.get(succId);
          if (succ) {
            out += emitNode(succ, adj, indent + 1, visited);
          }
        }

        out += `${pad}}\n`;
      }

      // Emit flow-out successors as post-condition code
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) {
          out += emitNode(succ, adj, indent, visited);
        }
      }

      return out;
    }

    case "return": {
      const expr = node.element.source || "";
      let out = emitPidComment(pid);
      out += `${pad}return${expr ? " " + expr : ""};\n`;
      // return is terminal — no successors emitted
      return out;
    }

    default:
      return "";
  }
}

/**
 * Compile a code flow graph to a complete fn definition string.
 *
 * @param elements  All code flow elements (fn, code-block, condition, return)
 * @param wires     All wires between elements (control-flow and data-flow)
 * @param fnPid     The PID (or element ID) of the fn entry node
 * @returns         Complete fn definition string with // @pid:xxx annotations
 */
export function compileCodeFlowGraph(
  elements: CodeblockElementData[],
  wires: WireData[],
  fnPid: string,
): string {
  const adj = buildAdjacency(elements, wires);
  const fnNode = adj.get(fnPid);
  if (!fnNode) return "";
  return emitNode(fnNode, adj, 0, new Set());
}
